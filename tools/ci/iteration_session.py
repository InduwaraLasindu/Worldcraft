"""Bounded hosted CI workspace, never a product runtime dependency."""
import base64, datetime, hashlib, json, os, pathlib, re, subprocess, time, urllib.request
ROOT=pathlib.Path.cwd()
REPO=os.environ["GITHUB_REPOSITORY"]
BRANCH="codex/worldcraft-foundation"
QA="codex/worldcraft-visual-qa"
TOKEN=os.environ["GH_TOKEN"]
API="https://api.github.com/repos/"+REPO
LOG=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-iteration.log"
START=time.monotonic()
def api(path,data=None,method=None):
    headers={"Authorization":"Bearer "+TOKEN,"Accept":"application/vnd.github+json","X-GitHub-Api-Version":"2022-11-28"}
    body=None if data is None else json.dumps(data).encode()
    request=urllib.request.Request(API+path,data=body,headers=headers,method=method)
    with urllib.request.urlopen(request,timeout=90) as r:return json.load(r)
def git(*args):return subprocess.check_output(["git",*args],text=True).strip()
def report(source,status,stage,code=None):
    payload={"source":source,"status":status,"stage":stage,"exit_code":code,
        "updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "run":os.environ["GITHUB_RUN_ID"],"log_tail":LOG.read_text(errors="replace")[-48000:] if LOG.exists() else ""}
    data=base64.b64encode(json.dumps(payload,indent=2).encode()).decode()
    for attempt in range(3):
        try:
            try:old=api("/contents/qa/session-status.json?ref="+QA);sha=old["sha"]
            except Exception:sha=None
            body={"message":"test: record hosted iteration status","content":data,"branch":QA}
            if sha:body["sha"]=sha
            api("/contents/qa/session-status.json",body,"PUT");return
        except Exception as e:
            if attempt==2:print("Status publication failed:",type(e).__name__,flush=True)
            time.sleep(2)
def run(command,source,stage,timeout=1500):
    report(source,"running",stage)
    with LOG.open("a") as out:
        out.write("\n== "+stage+" ==\n");out.flush()
        try:p=subprocess.run(command,stdout=out,stderr=subprocess.STDOUT,timeout=timeout)
        except subprocess.TimeoutExpired:return 124
    return p.returncode
def captures(source):
    allowed={"smoke","balanced","high","home","alpine","coast","rebased","map","atmosphere","vista","learned"}
    entries=[]
    for file in sorted(ROOT.glob("worldcraft-*.png")):
        name=file.stem.removeprefix("worldcraft-")
        if name not in allowed:continue
        content=file.read_bytes()
        if len(content)>8*1024*1024:continue
        encoded=base64.b64encode(content).decode()
        blob=api("/git/blobs",{"content":encoded,"encoding":"base64"},"POST")
        entries.extend([{"path":"docs/media/cloud-"+name+".png","mode":"100644","type":"blob","sha":blob["sha"]},
            {"path":"docs/media/cloud-"+name+".base64.txt","mode":"100644","type":"blob","content":encoded}])
    entries.append({"path":"qa/capture-source.json","mode":"100644","type":"blob","content":json.dumps({"source":source,"run":os.environ["GITHUB_RUN_ID"]})})
    for attempt in range(3):
        try:
            ref=api("/git/ref/heads/"+QA);head=ref["object"]["sha"]
            commit=api("/git/commits/"+head)
            tree=api("/git/trees",{"base_tree":commit["tree"]["sha"],"tree":entries},"POST")
            new=api("/git/commits",{"message":"test: record audited hosted native captures","tree":tree["sha"],"parents":[head]},"POST")
            api("/git/refs/heads/"+QA,{"sha":new["sha"],"force":False},"PATCH");return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
processed=None
last_change=time.monotonic()
while time.monotonic()-START<105*60:
    try:
        subprocess.run(["git","fetch","origin",BRANCH],check=True,stdout=subprocess.DEVNULL)
        target=git("rev-parse","origin/"+BRANCH)
        if target!=git("rev-parse","HEAD"):
            subprocess.run(["git","reset","--hard",target],check=True,stdout=subprocess.DEVNULL)
        control=json.loads((ROOT/".worldcraft-session.json").read_text())
        if not control.get("enabled",False):
            report(processed or target,"stopped","session closed");break
        if not control.get("ready",False) or target==processed:
            if time.monotonic()-last_change>20*60:
                report(processed or target,"stopped","idle limit");break
            time.sleep(10);continue
        last_change=time.monotonic()
        LOG.write_text("Source "+target+"\n")
        message=git("log","-1","--pretty=%B")
        source=target
        code=run(["cargo","metadata","--locked","--no-deps","--format-version","1"],source,"dependency validation",300)
        if code:code=run(["cargo","generate-lockfile"],source,"lockfile resolution",300)
        if not code:code=run(["cargo","fmt","--all"],source,"canonical formatting",90)
        if not code and subprocess.run(["git","diff","--quiet"]).returncode:
            subprocess.run(["git","config","user.name","Worldcraft CI"],check=True)
            subprocess.run(["git","config","user.email","worldcraft-ci@users.noreply.github.com"],check=True)
            subprocess.run(["git","add","Cargo.lock","crates"],check=True)
            subprocess.run(["git","commit","-m","chore: format and lock hosted iteration [iterate]"],check=True)
            if subprocess.run(["git","push","origin","HEAD:"+BRANCH]).returncode:
                # Another source update won the lease; never overwrite it.
                report(source,"superseded","new source arrived");processed=target;continue
            source=git("rev-parse","HEAD")
        stages=[
            (["cargo","test","-p","worldcraft-core","--locked"],"core regression tests"),
            (["cargo","check","--profile","ci","-p","worldcraft","--all-targets","--locked"],"desktop API check"),
        ]
        if "[render]" in message or "[audit]" in message:
            stages.extend([
                (["cargo","clippy","-p","worldcraft-core","--all-targets","--locked"],"core audit"),
                (["cargo","build","--profile","ci","-p","worldcraft","--locked"],"native executable"),
                (["cargo","test","--profile","ci","-p","worldcraft","--locked"],"desktop regression tests"),
                (["timeout","600s","xvfb-run","-a","./target/ci/worldcraft","--smoke-test"],"native render audit"),
            ])
        for command,stage in stages:
            if code:break
            code=run(command,source,stage)
        if not code and ("[render]" in message or "[audit]" in message):
            text=LOG.read_text(errors="replace")
            if re.search(r"error validating shader|ShaderValidationError|panicked at",text,re.I):code=1
            if not code:captures(source)
        report(source,"passed" if code==0 else "failed","iteration complete",code)
        processed=source
    except Exception as e:
        print("Iteration error:",type(e).__name__,str(e)[:300],flush=True)
        report(processed or "unknown","failed","session coordinator")
        time.sleep(10)
else:
    report(processed or "unknown","stopped","session time limit")
