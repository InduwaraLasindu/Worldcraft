"""Frozen alpha-2 Windows validation on a standard hosted runner, with no paid cache."""
import base64,datetime,hashlib,json,os,pathlib,subprocess,time,urllib.request,urllib.error
ROOT=pathlib.Path.cwd().resolve()
assert os.name=="nt" and os.environ.get("GITHUB_ACTIONS")=="true" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
BRANCH="codex/worldcraft-diffusion-detail";QA="codex/worldcraft-visual-qa"
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
HEADERS={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
START=time.monotonic()
LOG=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-distribution.log"
def git(*args):return subprocess.check_output(["git",*args],text=True).strip()
COMPILED=git("rev-parse","HEAD")
LOG.write_text("Hosted distribution source "+COMPILED+"\n",encoding="utf-8")
def api(path,data=None,method=None):
    request=urllib.request.Request(API+path,headers=HEADERS,data=None if data is None else json.dumps(data).encode(),method=method)
    with urllib.request.urlopen(request,timeout=90) as response:return json.load(response)
def publish(path,data):
    for attempt in range(3):
        try:
            try:previous=api("/contents/"+path+"?ref="+QA)["sha"]
            except urllib.error.HTTPError as error:
                if error.code!=404:raise
                previous=None
            body={"message":"test: record detailed Windows release evidence","branch":QA,"content":base64.b64encode(data).decode()}
            if previous:body["sha"]=previous
            api("/contents/"+path,body,"PUT");return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
def status(state,stage,code=None,extra=None):
    report={"status":state,"stage":stage,"source":COMPILED,"tool_source":git("rev-parse","HEAD"),"run":os.environ["GITHUB_RUN_ID"],"updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),"exit_code":code,"log_tail":LOG.read_text(errors="replace")[-22000:]}
    for name,key in [("detail-audit.json","audit"),("detail-render-audit.json","render"),("detail-model-assets.json","models")]:
        file=ROOT/name
        if file.exists():
            try:report[key]=json.loads(file.read_text(encoding="utf-8-sig"))
            except Exception:pass
    if extra:report.update(extra)
    publish("qa/detailed-windows-status.json",json.dumps(report,indent=2).encode())
def run(command,stage,timeout=1800):
    with LOG.open("a",encoding="utf-8") as output:
        output.write("\n== "+stage+" ==\n");output.flush()
        process=subprocess.Popen(command,stdout=output,stderr=subprocess.STDOUT)
        deadline=min(START+80*60,time.monotonic()+timeout)
        while process.poll() is None:
            status("running",stage)
            if time.monotonic()>deadline:
                subprocess.run(["taskkill","/PID",str(process.pid),"/T","/F"],check=False);process.wait();return 124
            time.sleep(15)
        return process.returncode
def checked(command,stage,timeout=1800):
    code=run(command,stage,timeout)
    if code:status("failed",stage,code);raise SystemExit(code)
checked(["rustup","update","stable","--no-self-update"],"stable Rust",300)
checked(["cargo","fmt","--all","--","--check"],"canonical formatting",90)
checked(["pwsh","-NoProfile","-File","tools/validate-package-scripts.ps1"],"packaging script syntax",120)
checked(["cargo","test","-p","worldcraft-core","-p","worldcraft-ml","--locked"],"core and ML regression tests",900)
checked(["cargo","clippy","-p","worldcraft-core","-p","worldcraft-ml","--all-targets","--locked"],"core and ML static audit",900)
checked(["cargo","build","--release","-p","worldcraft","--locked"],"optimized native Windows build",2400)
checked(["cargo","test","--release","-p","worldcraft","--locked"],"desktop regression tests",600)
# Warm the compiler while the coding session finishes its reviewed source.
until=min(START+65*60,time.monotonic()+20*60)
while True:
    subprocess.run(["git","fetch","origin",BRANCH],check=True,stdout=subprocess.DEVNULL)
    latest=git("rev-parse","origin/"+BRANCH)
    control=json.loads(subprocess.check_output(["git","show",latest+":.worldcraft-detail-session.json"],text=True))
    if control.get("release_ready"):
        if latest!=COMPILED:
            subprocess.run(["git","reset","--hard",latest],check=True)
            COMPILED=latest
            previous=LOG.read_text(errors="replace").splitlines()
            previous[0]="Hosted distribution source "+COMPILED
            LOG.write_text("\n".join(previous)+"\n",encoding="utf-8")
            checked(["cargo","fmt","--all","--","--check"],"frozen source formatting",90)
            checked(["cargo","test","-p","worldcraft-core","-p","worldcraft-ml","--locked"],"frozen core and ML tests",900)
            checked(["cargo","clippy","-p","worldcraft-core","-p","worldcraft-ml","--all-targets","--locked"],"frozen static audit",900)
            checked(["cargo","build","--release","-p","worldcraft","--locked"],"frozen optimized Windows executable",2400)
            checked(["cargo","test","--release","-p","worldcraft","--locked"],"frozen desktop tests",600)
        break
    if time.monotonic()>until:status("stopped","source freeze was not ready");raise SystemExit(1)
    status("waiting_for_source","compiler warm; waiting for reviewed source")
    time.sleep(15)
checked(["python","tools/ci/bootstrap_ort.py"],"immutable ONNX Runtime",300)
runtime=next((pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-ort"/"extracted").rglob("onnxruntime.dll"))
os.environ["WORLDCRAFT_ORT_DYLIB"]=str(runtime)
checked(["python","tools/ci/prepare_detail_models.py"],"immutable detailed model set",1200)
models=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-detail-models"/"ad2df557eca5645f588766101cf3bc3682455c3e"
checked(["cargo","run","--release","-p","worldcraft-ml","--bin","audit","--",str(runtime),str(models.parent),"coarse-windows.worldcraft.json"],"actual native fast-model regression",600)
checked(["cargo","run","--release","-p","worldcraft-ml","--bin","detail-audit","--",str(models),str(runtime)],"actual native detailed generation",1200)
checked(["python","tools/ci/bootstrap_dxc.py"],"immutable modern shader compiler",300)
# Packaging/render tools may be repaired in place, but changed product code requires a new job.
stages=[(["pwsh","-NoProfile","-File","tools/test-windows-render.ps1"],"software Direct3D 12 state and shader regression",1200),(["pwsh","-NoProfile","-File","tools/package-windows.ps1"],"portable, installer and safe uninstall",900),(["pwsh","-NoProfile","-File","tools/test-detailed-package.ps1"],"packaged detailed scene at 640x360",900)]
for command,stage,timeout in stages:
    for attempt in range(3):
        code=run(command,stage,timeout)
        if not code:break
        status("failed_waiting_update",stage,code)
        previous=git("rev-parse","HEAD");until=time.monotonic()+5*60;changed=False
        while time.monotonic()<until:
            subprocess.run(["git","fetch","origin",BRANCH],check=True,stdout=subprocess.DEVNULL)
            latest=git("rev-parse","origin/"+BRANCH)
            if latest!=previous and subprocess.run(["git","diff","--quiet",previous,latest,"--","tools"]).returncode:
                assert subprocess.run(["git","diff","--quiet",COMPILED,latest,"--","crates","Cargo.toml","Cargo.lock","examples"]).returncode==0,"Product source changed; build again"
                subprocess.run(["git","reset","--hard",latest],check=True);changed=True;break
            time.sleep(15)
        if not changed:status("failed",stage,code);raise SystemExit(code)
    else:status("failed","script retry limit",code);raise SystemExit(code)
image=(ROOT/"worldcraft-detailed-windows.png").read_bytes()
image_sha=hashlib.sha256(image).hexdigest()
publish("docs/media/detailed-windows.base64.txt",base64.b64encode(image)+b"\n")
audit=json.loads((ROOT/"detail-audit.json").read_text())
assert audit["status"]=="passed"
tag="v0.2.0-alpha.2-build."+os.environ["GITHUB_RUN_NUMBER"]
assets=[ROOT/"dist/Worldcraft-Windows-x64-development.zip",ROOT/"dist/Worldcraft-Setup-0.2.0-alpha.2.exe",ROOT/"dist/SHA256SUMS.txt"]
for file in assets:assert file.is_file() and file.stat().st_size>0
hashes={file.name:hashlib.sha256(file.read_bytes()).hexdigest() for file in assets}
notes=ROOT/"detail-release-notes.md"
notes.write_text("""Worldcraft 0.2.0-alpha.2 is an experimental native Windows x64 application.

New in this milestone: optional native three-model Terrain Diffusion generation, a saved 320 x 320 central detail field, smooth coarse joins, per-stage progress/cancellation, and a precomputed offline detailed example. The fast 22.5 MB model remains available; new detailed regions use an optional 2.28 GB model set. Models load sequentially on the CPU. No Python or Java runtime is required.

Default detailed sample spacing is 30 m over 9.57 km; this is a finite regional bake with Worldcraft conditioning. Infinite learned streaming, upstream conditioning parity and GPU inference are unfinished.

Existing features include classical terrain/thermal relaxation, bounded LOD, procedural vegetation, walking/flying, seven editing brushes, undo/redo, persistence/recovery, maps, atmosphere/High volumetric mist/SSAO, capability-gated experimental ray-query lighting and GLB/OBJ/16-bit PNG terrain exports.

Hosted audits cover core/ML/desktop regressions, real repeated/different-seed native inference, joins, edit/undo/exact save/reopen, software Vulkan rendering, software Direct3D 12 state checks, a reviewed 640x360 scene from the actual portable package, runtime installation and safe uninstall. Software checks do not establish gaming-GPU performance or broad Windows UI/hardware certification.

This remains an unsigned engineering alpha. The full master specification is unfinished, including connected hydrology/hydraulic erosion, caves, procedural graphs, asset importing, wind/weather, advanced ecosystems and cinematic tools.

Compiled source: """+COMPILED+"\nWindows audit: https://github.com/"+os.environ["GITHUB_REPOSITORY"]+"/actions/runs/"+os.environ["GITHUB_RUN_ID"]+"\n",encoding="utf-8")
checked(["gh","release","create",tag,*map(str,assets),"--repo",os.environ["GITHUB_REPOSITORY"],"--target",COMPILED,"--draft","--prerelease","--title","Worldcraft 0.2.0-alpha.2 — Windows x64","--notes-file",str(notes)],"verified draft release",300)
candidates=[r for r in api("/releases?per_page=100") if r["tag_name"]==tag and r["target_commitish"]==COMPILED]
assert len(candidates)==1,"Expected one uploaded draft candidate"
candidate=candidates[0]
extra={"tag":tag,"release_id":candidate["id"],"assets":hashes,"image_sha256":image_sha,"visual_review_passed":False}
status("awaiting_visual_review","packaged scene and alpha publication",0,extra)
# The coding agent reviews actual captures and records exact source/image hashes.
until=min(START+82*60,time.monotonic()+12*60)
while time.monotonic()<until:
    try:
        entry=api("/contents/.worldcraft-detail-release-review.json?ref="+BRANCH)
        review=json.loads(base64.b64decode(entry["content"]))
    except urllib.error.HTTPError as error:
        if error.code!=404:raise
        review={}
    if review.get("approved") and review.get("source")==COMPILED and review.get("run")==os.environ["GITHUB_RUN_ID"] and review.get("tag")==tag and review.get("windows_image_sha256")==image_sha:
        linux_entry=api("/contents/qa/detailed-linux.json?ref="+QA)
        linux=json.loads(base64.b64decode(linux_entry["content"]))
        assert linux["status"]=="passed" and review.get("linux_source")==linux["source"] and review.get("linux_image_sha256")==linux["image_sha256"]
        subprocess.run(["git","fetch","origin",BRANCH],check=True)
        assert subprocess.run(["git","diff","--quiet",linux["source"],COMPILED,"--","crates","Cargo.toml","Cargo.lock","examples"]).returncode==0,"Linux audited a different product"
        assert review.get("export_validation_passed") is True
        current=api("/releases/"+str(candidate["id"]))
        remote={a["name"]:a for a in current["assets"]}
        for name,digest in hashes.items():assert remote[name]["digest"]=="sha256:"+digest
        published=api("/releases/"+str(candidate["id"]),{"draft":False,"prerelease":True,"make_latest":"false"},"PATCH")
        assert not published["draft"] and published["prerelease"]
        extra["visual_review_passed"]=True;extra["release_url"]=published["html_url"]
        status("passed","alpha 2 published",0,extra)
        print("Published "+published["html_url"],flush=True);raise SystemExit(0)
    time.sleep(15)
status("draft_ready","review window ended; verified draft retained",0,extra)
print("Verified alpha draft retained for review.",flush=True)
