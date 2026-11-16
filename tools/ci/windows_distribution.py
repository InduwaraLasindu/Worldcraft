"""Hosted Windows validation with bounded retries for packaging-only fixes."""
import base64,datetime,json,os,pathlib,subprocess,time,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve(),"Wrong hosted workspace"
BRANCH="codex/worldcraft-foundation"
QA="codex/worldcraft-visual-qa"
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
TOKEN=os.environ["GH_TOKEN"]
LOG=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-distribution.log"
def git(*args):return subprocess.check_output(["git",*args],text=True).strip()
def status(state,stage,code=None):
    payload={"status":state,"stage":stage,"exit_code":code,"source":git("rev-parse","HEAD"),
    "run":os.environ["GITHUB_RUN_ID"],"updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "log_tail":LOG.read_text(errors="replace")[-16000:] if LOG.exists() else ""}
    url=API+"/contents/qa/windows-distribution-status.json"
    headers={"Authorization":"Bearer "+TOKEN,"Accept":"application/vnd.github+json"}
    for attempt in range(3):
        try:
            try:
                with urllib.request.urlopen(urllib.request.Request(url+"?ref="+QA,headers=headers),timeout=30) as response:sha=json.load(response)["sha"]
            except Exception:sha=None
            body={"message":"test: record hosted Windows distribution status","branch":QA,"content":base64.b64encode(json.dumps(payload,indent=2).encode()).decode()}
            if sha:body["sha"]=sha
            with urllib.request.urlopen(urllib.request.Request(url,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=30) as response:response.read()
            return
        except Exception:
            if attempt==2:print("Could not publish distribution status",flush=True)
            time.sleep(2)
def run(script,stage):
    with LOG.open("a",encoding="utf-8") as output:
        output.write("\n== "+stage+" ==\n");output.flush()
        process=subprocess.Popen(["pwsh","-NoProfile","-File",script],stdout=output,stderr=subprocess.STDOUT)
        deadline=time.monotonic()+45*60
        while process.poll() is None:
            status("running",stage)
            if time.monotonic()>deadline:
                subprocess.run(["taskkill","/PID",str(process.pid),"/T","/F"],check=False)
                return 124
            time.sleep(15)
        return process.returncode
LOG.write_text("Hosted distribution source "+git("rev-parse","HEAD")+"\n",encoding="utf-8")
stage_index=0
stages=[("tools/test-windows-render.ps1","native Windows render"),("tools/publish-windows.ps1","portable / installer / safe uninstall / draft build")]
for attempt in range(4):
    code=0
    while stage_index<len(stages):
        script,stage=stages[stage_index]
        code=run(script,stage)
        if code:break
        stage_index+=1
    if not code:
        status("passed","Windows distribution complete",0)
        print(LOG.read_text(errors="replace")[-12000:]);raise SystemExit(0)
    status("failed_waiting_update",stages[stage_index][1],code)
    print("A Windows validation stage failed; allowing a bounded packaging/script fix.",flush=True)
    previous=git("rev-parse","HEAD")
    until=time.monotonic()+8*60
    changed=False
    while time.monotonic()<until:
        subprocess.run(["git","fetch","origin",BRANCH],check=True,stdout=subprocess.DEVNULL)
        latest=git("rev-parse","origin/"+BRANCH)
        if latest!=previous and subprocess.run(["git","diff","--quiet",previous,latest,"--","tools"]).returncode:
            # Never reuse an executable for changed product code or dependencies.
            if subprocess.run(["git","diff","--quiet",previous,latest,"--","crates","Cargo.toml","Cargo.lock"]).returncode:
                status("failed","product source changed; a new build is required",code);raise SystemExit(code)
            # This checkout is disposable, inside the verified hosted workspace.
            subprocess.run(["git","reset","--hard",latest],check=True)
            changed=True;break
        time.sleep(15)
    if not changed:
        status("failed","no script update before retry limit",code);raise SystemExit(code)
status("failed","distribution retry limit",code);raise SystemExit(code)
