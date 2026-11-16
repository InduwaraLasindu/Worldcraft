"""Build only the native ML library and exercise real download cancellation/retry."""
import base64,json,os,pathlib,subprocess,sys,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert os.environ.get("GITHUB_ACTIONS")=="true" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
temporary=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-native-download-audit";temporary.mkdir(exist_ok=True)
log=temporary/"audit.log"
commands=[["cargo","build","--profile","ci","-p","worldcraft-ml","--lib","--locked"]]
code=0
with log.open("w") as output:
    for command in commands:
        code=subprocess.run(command,stdout=output,stderr=subprocess.STDOUT).returncode
        if code:break
    if not code:
        libraries=list((ROOT/"target/ci/deps").glob("libworldcraft_ml-*.rlib"))
        assert len(libraries)==1,libraries
        binary=temporary/"download-probe"
        code=subprocess.run(["rustc","--edition=2021","--crate-name","worldcraft_download_probe","tools/ci/native_download_probe.rs","-L","dependency=target/ci/deps","--extern","worldcraft_ml="+str(libraries[0]),"-o",str(binary)],stdout=output,stderr=subprocess.STDOUT).returncode
        if not code:code=subprocess.run([str(binary),str(temporary/"models")],stdout=output,stderr=subprocess.STDOUT,timeout=420).returncode
report={"status":"passed" if not code else "failed","source":os.environ["GITHUB_SHA"],"run":os.environ["GITHUB_RUN_ID"],"log_tail":log.read_text(errors="replace")[-12000:]}
if (ROOT/"native-download-audit.json").exists():report["audit"]=json.loads((ROOT/"native-download-audit.json").read_text())
api="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/qa/native-download-audit.json"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
try:
    with urllib.request.urlopen(urllib.request.Request(api+"?ref=codex/worldcraft-visual-qa",headers=headers),timeout=60) as response:sha=json.load(response)["sha"]
except Exception:sha=None
body={"message":"test: record real native download cancellation and retry","branch":"codex/worldcraft-visual-qa","content":base64.b64encode(json.dumps(report,indent=2).encode()).decode()}
if sha:body["sha"]=sha
with urllib.request.urlopen(urllib.request.Request(api,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=60) as response:response.read()
print(report["log_tail"]);raise SystemExit(code)
