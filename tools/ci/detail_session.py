"""Bounded hosted build/ML workspace; no caches or owner-computer execution."""
import base64,datetime,json,os,pathlib,signal,subprocess,time,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
BRANCH="codex/worldcraft-diffusion-detail";QA="codex/worldcraft-visual-qa"
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
TOKEN=os.environ["GH_TOKEN"];START=time.monotonic()
LOG=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-detail.log"
LOG.write_text("Hosted detailed Terrain Diffusion iteration\n")
def git(*args):return subprocess.check_output(["git",*args],text=True).strip()
def api(path,data=None,method=None):
    headers={"Authorization":"Bearer "+TOKEN,"Accept":"application/vnd.github+json"}
    request=urllib.request.Request(API+path,headers=headers,data=None if data is None else json.dumps(data).encode(),method=method)
    with urllib.request.urlopen(request,timeout=40) as response:return json.load(response)
def report(state,stage,code=None):
    payload={"status":state,"stage":stage,"source":git("rev-parse","HEAD"),"run":os.environ["GITHUB_RUN_ID"],"updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),"exit_code":code,"log_tail":LOG.read_text(errors="replace")[-24000:]}
    for name,key in [("detail-model-assets.json","manifest"),("detail-probe.json","probe"),("detail-audit.json","audit")]:
        file=ROOT/name
        if file.exists():
            try:payload[key]=json.loads(file.read_text())
            except Exception:pass
    url="/contents/qa/detail-session-status.json"
    for attempt in range(3):
        try:
            try:sha=api(url+"?ref="+QA)["sha"]
            except Exception:sha=None
            body={"message":"test: record hosted detailed-generation status","branch":QA,"content":base64.b64encode(json.dumps(payload,indent=2).encode()).decode()}
            if sha:body["sha"]=sha
            api(url,body,"PUT");return
        except Exception:
            if attempt==2:print("Could not publish status",flush=True)
            time.sleep(2)
def run(command,stage,timeout=1800):
    with LOG.open("a") as output:
        output.write("\n== "+stage+" ==\n");output.flush()
        process=subprocess.Popen(command,stdout=output,stderr=subprocess.STDOUT,start_new_session=True)
        deadline=min(START+104*60,time.monotonic()+timeout)
        while process.poll() is None:
            report("running",stage)
            if time.monotonic()>deadline:
                os.killpg(process.pid,signal.SIGKILL);process.wait();return 124
            time.sleep(15)
        return process.returncode
code=run(["python3","tools/ci/prepare_detail_models.py"],"immutable model preparation",1200)
if code:report("failed","model preparation",code);raise SystemExit(code)
processed=None;last_change=time.monotonic();warmed=False
while time.monotonic()-START<105*60:
    subprocess.run(["git","fetch","origin",BRANCH],check=True,stdout=subprocess.DEVNULL)
    target=git("rev-parse","origin/"+BRANCH)
    if target!=git("rev-parse","HEAD"):subprocess.run(["git","reset","--hard",target],check=True)
    control=json.loads((ROOT/".worldcraft-detail-session.json").read_text())
    if not control.get("enabled"):report("stopped","session closed");break
    if not control.get("ready") or target==processed:
        if time.monotonic()-last_change>20*60:report("stopped","idle limit");break
        time.sleep(10);continue
    last_change=time.monotonic()
    LOG.write_text("Detailed generation source "+target+"\n")
    code=run(["cargo","fmt","--all"],"canonical formatting",90)
    if not code and subprocess.run(["git","diff","--quiet","--","crates"]).returncode:
        subprocess.run(["git","config","user.name","Worldcraft CI"],check=True);subprocess.run(["git","config","user.email","worldcraft-ci@users.noreply.github.com"],check=True)
        subprocess.run(["git","add","crates"],check=True);subprocess.run(["git","commit","-m","chore: format detailed-generation iteration [detail-iterate]"],check=True)
        if subprocess.run(["git","push","origin","HEAD:"+BRANCH]).returncode:report("superseded","new source arrived");processed=target;continue
    source=git("rev-parse","HEAD")
    phase=control.get("phase","probe")
    models=str(pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-detail-models"/"ad2df557eca5645f588766101cf3bc3682455c3e")
    commands=[(["cargo","test","-p","worldcraft-core","-p","worldcraft-ml","--locked"],"core and ML regression tests",300)]
    if phase=="probe":commands.append((["cargo","run","--profile","ci","-p","worldcraft-ml","--bin","detail-probe","--",models,os.environ["WORLDCRAFT_ORT_DYLIB"]],"native base/decoder contracts",1200))
    else:commands.append((["cargo","run","--profile","ci","-p","worldcraft-ml","--bin","detail-audit","--",models,os.environ["WORLDCRAFT_ORT_DYLIB"]],"actual detailed region inference",2400))
    if not warmed or phase=="render":
        commands.extend([(["cargo","build","--profile","ci","-p","worldcraft","--locked"],"native desktop build",1800),(["cargo","test","--profile","ci","-p","worldcraft","--locked"],"desktop regression tests",300)])
    if phase=="render":commands.append((["timeout","720s","xvfb-run","-a","./target/ci/worldcraft","--smoke-test"],"native graphics regression",750))
    for command,stage,timeout in commands:
        if code:break
        code=run(command,stage,timeout)
    if not code:warmed=True
    report("passed" if not code else "failed","iteration complete",code)
    processed=git("rev-parse","HEAD")
else:report("stopped","session time limit")
