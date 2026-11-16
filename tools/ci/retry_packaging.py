"""Retry only hosted distribution tools; never reuse the binary for changed product/dependencies."""
import os,pathlib,subprocess,time
ROOT=pathlib.Path.cwd().resolve()
assert os.name=="nt" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
def git(*args):return subprocess.check_output(["git",*args],text=True).strip()
for attempt in range(3):
    result=subprocess.run(["pwsh","-NoProfile","-File","tools/publish-windows-once.ps1"])
    if result.returncode==0:raise SystemExit(0)
    print("PACKAGING WAITING FOR SCRIPT UPDATE; retaining the verified executable and render evidence.",flush=True)
    previous=git("rev-parse","HEAD");until=time.monotonic()+4*60
    updated=False
    while time.monotonic()<until:
        subprocess.run(["git","fetch","origin","codex/worldcraft-foundation"],check=True)
        latest=git("rev-parse","origin/codex/worldcraft-foundation")
        if latest!=previous and subprocess.run(["git","diff","--quiet",previous,latest,"--","tools"]).returncode:
            assert subprocess.run(["git","diff","--quiet",previous,latest,"--","crates","Cargo.toml","Cargo.lock"]).returncode==0,"New product source needs a new build"
            subprocess.run(["git","reset","--hard",latest],check=True)
            updated=True;break
        time.sleep(15)
    if not updated:raise SystemExit(result.returncode)
raise SystemExit("Packaging retry limit")
