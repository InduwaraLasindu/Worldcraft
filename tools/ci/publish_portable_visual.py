"""Record the packaged executable's wider Windows capture for visual review."""
import base64,datetime,hashlib,json,os,pathlib,sys,time,urllib.request,urllib.error
QA="codex/worldcraft-visual-qa"
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
temporary=pathlib.Path(os.environ["RUNNER_TEMP"])
def write(path,data):
    url=API+path
    for attempt in range(3):
        try:
            try:
                with urllib.request.urlopen(urllib.request.Request(url+"?ref="+QA,headers=headers),timeout=30) as response:sha=json.load(response)["sha"]
            except urllib.error.HTTPError as error:
                if error.code!=404:raise
                sha=None
            body={"message":"test: record packaged Windows visual audit","branch":QA,"content":base64.b64encode(data).decode()}
            if sha:body["sha"]=sha
            with urllib.request.urlopen(urllib.request.Request(url,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=30) as response:response.read()
            return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
state=sys.argv[1]
logs=[]
for name in ["worldcraft-portable-visual-output.log","worldcraft-portable-visual-errors.log"]:
    path=temporary/name
    if path.exists():logs.append(path.read_text(errors="replace")[-6000:])
report={"status":state,"source":os.environ["WORLDCRAFT_VISUAL_SOURCE"],"run":os.environ["GITHUB_RUN_ID"],
"tag":os.environ["WORLDCRAFT_VISUAL_TAG"],"archive_sha256":os.environ["WORLDCRAFT_VISUAL_ZIP_SHA"],
"updated":datetime.datetime.now(datetime.timezone.utc).isoformat(),"scope":"Packaged binary, offline learned world, software Direct3D 12 at 640x360. Requires visual review; not a hardware performance benchmark.","visual_review_passed":False,"log_tail":"\n".join(logs)}
if state=="passed_image_checks":
    capture=pathlib.Path(os.environ["WORLDCRAFT_VISUAL_CAPTURE"]).read_bytes()
    report["image_sha256"]=hashlib.sha256(capture).hexdigest()
    report["pixels"]=json.loads((temporary/"worldcraft-portable-visual-pixels.json").read_text(encoding="utf-8-sig"))
    write("docs/media/windows-portable-learned.base64.txt",base64.b64encode(capture)+b"\n")
write("qa/windows-portable-visual.json",json.dumps(report,indent=2).encode())
print("Packaged Windows visual audit: "+state,flush=True)
