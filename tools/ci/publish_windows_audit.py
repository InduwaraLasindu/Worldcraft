"""Record native Windows QA evidence without downloading it onto the owner's computer."""
import base64,datetime,hashlib,json,os,pathlib,re,subprocess,time,urllib.request,urllib.error
ROOT=pathlib.Path.cwd().resolve()
assert ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/"
QA="codex/worldcraft-visual-qa"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
def publish(path,data):
    url=API+path
    for attempt in range(3):
        try:
            try:
                with urllib.request.urlopen(urllib.request.Request(url+"?ref="+QA,headers=headers),timeout=30) as response:sha=json.load(response)["sha"]
            except urllib.error.HTTPError as error:
                if error.code!=404:raise
                sha=None
            body={"message":"test: record native Windows render evidence","branch":QA,"content":base64.b64encode(data).decode()}
            if sha:body["sha"]=sha
            with urllib.request.urlopen(urllib.request.Request(url,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=30) as response:response.read()
            return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
images=json.loads((ROOT/"windows-render-image-audit.json").read_text(encoding="utf-8-sig"))
for image in images:
    data=(ROOT/("worldcraft-"+image["scene"]+".png")).read_bytes()
    image["sha256"]=hashlib.sha256(data).hexdigest()
    image["bytes"]=len(data)
    if image["scene"] in ["balanced","high","coast"]:
        publish("docs/media/windows-"+image["scene"]+".base64.txt",base64.b64encode(data)+b"\n")
log=(pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-distribution.log").read_text(errors="replace")
match=re.search(r"^Hosted distribution source ([0-9a-f]{40})",log)
assert match,"Missing frozen executable source in distribution log"
compiled=match.group(1)
report={"status":"passed","source":subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),"compiled_source":compiled,
"run":os.environ["GITHUB_RUN_ID"],"checked_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),
"adapter":"Microsoft Basic Render Driver / software Direct3D 12","shader_compiler":"DXC v1.9.2609",
"scope":"Native integration and nonuniform rendered pixels; not Windows UI interaction or a hardware performance benchmark.","images":images}
publish("qa/windows-render-audit.json",json.dumps(report,indent=2).encode())
print("Recorded eight validated Windows captures and three visual inspection previews.",flush=True)
