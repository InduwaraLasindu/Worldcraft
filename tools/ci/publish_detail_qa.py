"""Record real hosted detailed-world evidence and one reproducible baked example."""
import base64,datetime,hashlib,json,os,pathlib,shutil,subprocess,time,urllib.request,urllib.error
ROOT=pathlib.Path.cwd().resolve()
assert os.environ.get("GITHUB_ACTIONS")=="true" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
QA="codex/worldcraft-visual-qa";BRANCH="codex/worldcraft-diffusion-detail"
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
HEADERS={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
def git(*a):return subprocess.check_output(["git",*a],text=True).strip()
def call(path,data=None,method=None):
    request=urllib.request.Request(API+path,headers=HEADERS,data=None if data is None else json.dumps(data).encode(),method=method)
    with urllib.request.urlopen(request,timeout=90) as response:return json.load(response)
def publish(path,data):
    for attempt in range(3):
        try:
            try:previous=call("/contents/"+path+"?ref="+QA)["sha"]
            except urllib.error.HTTPError as error:
                if error.code!=404:raise
                previous=None
            body={"message":"test: record detailed Terrain Diffusion evidence","branch":QA,"content":base64.b64encode(data).decode()}
            if previous:body["sha"]=previous
            call("/contents/"+path,body,"PUT");return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
source=git("rev-parse","HEAD")
subprocess.run(["cargo","run","--profile","ci","-p","worldcraft-core","--bin","export-audit","--","detail-example.worldcraft"],check=True)
validator=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-validator"
subprocess.run(["npm","install","--prefix",str(validator),"--ignore-scripts","--no-audit","--no-fund","gltf-validator@2.0.0-dev.3.10"],check=True)
environment=dict(os.environ);environment["NODE_PATH"]=str(validator/"node_modules");environment["GITHUB_SHA"]=source
subprocess.run(["node","tools/ci/validate_glb.cjs"],check=True,env=environment)
export=json.loads((ROOT/"export-validation.json").read_text())
heightmap=json.loads((ROOT/"export-heightmap-audit.json").read_text())
assert export["issues"]["numErrors"]==0 and export["issues"]["numWarnings"]==0 and heightmap["status"]=="passed"
publish("qa/detailed-export.json",json.dumps({"status":"passed","source":source,"run":os.environ["GITHUB_RUN_ID"],"gltf":export,"heightmap":heightmap},indent=2).encode())
report={"status":"passed","source":source,"run":os.environ["GITHUB_RUN_ID"],"updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),"audit":json.loads((ROOT/"detail-audit.json").read_text()),"render":json.loads((ROOT/"detail-render-audit.json").read_text()),"visual_review_passed":False}
image=(ROOT/"worldcraft-detailed.png").read_bytes()
assert image[:8]==b"\x89PNG\r\n\x1a\n" and len(image)>12000
report["image_sha256"]=hashlib.sha256(image).hexdigest()
publish("docs/media/detailed-linux.base64.txt",base64.b64encode(image)+b"\n")
publish("qa/detailed-linux.json",json.dumps(report,indent=2).encode())
# Preserve generated samples without modifying the compiled checkout or other source files.
fixture=ROOT/"examples/detailed-highlands.worldcraft.json"
if not fixture.exists():
    try:call("/contents/examples/detailed-highlands.worldcraft.json?ref="+BRANCH)
    except urllib.error.HTTPError as error:
        if error.code!=404:raise
        call("/contents/examples/detailed-highlands.worldcraft.json",{"message":"test: preserve audited native detailed terrain example [detail-iterate]","branch":BRANCH,"content":base64.b64encode((ROOT/"detail-example.worldcraft").read_bytes()).decode()},"PUT")
print("Detailed inference, native render and export evidence recorded.")
