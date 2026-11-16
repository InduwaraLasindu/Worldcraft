"""Run the native download audit with Cargo's own artifact/dependency resolution."""
import base64,json,os,pathlib,subprocess,traceback,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert os.environ.get("GITHUB_ACTIONS")=="true" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
temporary=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-native-download-audit";temporary.mkdir(exist_ok=True)
log=temporary/"audit.log";code=1
example=ROOT/"crates/worldcraft-ml/examples/native_download_probe.rs"
created=False
try:
    assert not example.exists(),"Refusing to overwrite a project example"
    example.parent.mkdir(exist_ok=True)
    example.write_bytes((ROOT/"tools/ci/native_download_probe.rs").read_bytes());created=True
    with log.open("w") as output:
        code=subprocess.run(["cargo","run","--profile","ci","-p","worldcraft-ml","--example","native_download_probe","--locked","--",str(temporary/"models")],stdout=output,stderr=subprocess.STDOUT,timeout=600).returncode
except Exception:
    with log.open("a") as output:output.write(traceback.format_exc())
finally:
    if created:example.unlink()
report={"status":"passed" if not code else "failed","source":os.environ["GITHUB_SHA"],"run":os.environ["GITHUB_RUN_ID"],"log_tail":log.read_text(errors="replace")[-16000:]}
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
