"""Recover the verified alpha's draft lookup without rebuilding or changing its bytes."""
import base64,hashlib,json,os,pathlib,struct,subprocess,time,urllib.request,zipfile
ROOT=pathlib.Path.cwd().resolve()
assert os.environ.get("GITHUB_ACTIONS")=="true" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
API="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
HEADERS={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
QA="codex/worldcraft-visual-qa";BRANCH="codex/worldcraft-diffusion-detail"
COMPILED="2ace1eb0f86df10a8267a9a21487d25ca79557d7";RID=446967361
TAG="v0.2.0-alpha.2-build.2";BUILD="43852794308"
EXPECTED={"Worldcraft-Windows-x64-development.zip":"2ea4e06a12f082457d32d27d7954c2e94f2a3cddad1a59342ce76ecd14c0453f","Worldcraft-Setup-0.2.0-alpha.2.exe":"bbb7da7e8883dcaa32063fbe821093daa4d156b2446b50caa5c5875cc1139450","SHA256SUMS.txt":"8a3b6e5ee236399b8cf78519e2c612e72a7f5e9ab53326bb52cb37472978f491"}
def api(path,data=None,method=None):
    request=urllib.request.Request(API+path,headers=HEADERS,data=None if data is None else json.dumps(data).encode(),method=method)
    with urllib.request.urlopen(request,timeout=90) as response:return json.load(response)
def file(path,branch=QA):
    entry=api("/contents/"+path+"?ref="+branch)
    return entry,base64.b64decode(entry["content"])
def qa(path):return json.loads(file(path)[1])
def publish(path,data):
    for attempt in range(3):
        try:
            try:sha=api("/contents/"+path+"?ref="+QA)["sha"]
            except Exception:sha=None
            body={"message":"test: record verified alpha publication recovery","branch":QA,"content":base64.b64encode(data).decode()}
            if sha:body["sha"]=sha
            api("/contents/"+path,body,"PUT");return
        except Exception:
            if attempt==2:raise
            time.sleep(2)
linux=qa("qa/detailed-linux.json");exports=qa("qa/detailed-export.json");download=qa("qa/native-download-audit.json");windows=qa("qa/detailed-windows-status.json");native=qa("qa/windows-render-audit.json")
assert linux["status"]=="passed" and linux["visual_review_passed"]
assert exports["status"]=="passed" and exports["gltf"]["issues"]["numErrors"]==0 and exports["gltf"]["issues"]["numWarnings"]==0
assert download["status"]=="passed" and download["audit"]["fast_model_preserved"] and download["audit"]["partial_files_removed"]
assert api("/actions/runs/"+download["run"])["conclusion"]=="success"
assert api("/actions/runs/"+linux["run"])["conclusion"]=="success"
assert windows["source"]==COMPILED and windows["run"]==BUILD and windows["stage"]=="verified draft release"
assert windows["audit"]["status"]=="passed" and windows["render"]["status"]=="passed"
assert native["status"]=="passed" and native["run"]==BUILD and native["compiled_source"]==COMPILED and len(native["images"])==8
# Verify the failure was confined to resolving a draft by a not-yet-created tag.
request=urllib.request.Request(API+"/actions/jobs/133249371907/logs",headers=HEADERS)
with urllib.request.urlopen(request,timeout=90) as response:log=response.read().decode("utf-8",errors="replace")
assert 'candidate=api("/releases/tags/"+tag)' in log and "HTTP Error 404" in log
for source in [linux["source"],exports["source"],download["source"],os.environ["GITHUB_SHA"]]:
    subprocess.run(["git","diff","--exit-code",source,COMPILED,"--","crates","Cargo.toml","Cargo.lock","examples"],check=True)
release=api("/releases/"+str(RID))
assert release["tag_name"]==TAG and release["target_commitish"]==COMPILED and release["prerelease"]
assets={a["name"]:a for a in release["assets"]}
temporary=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-verified-publication";temporary.mkdir(exist_ok=True)
for name,digest in EXPECTED.items():
    asset=assets[name];assert asset["digest"]=="sha256:"+digest
    request=urllib.request.Request(API+"/releases/assets/"+str(asset["id"]),headers={**HEADERS,"Accept":"application/octet-stream"})
    hash=hashlib.sha256();count=0
    with urllib.request.urlopen(request,timeout=120) as response,(temporary/name).open("wb") as output:
        while data:=response.read(1024*1024):
            count+=len(data);assert count<=asset["size"];hash.update(data);output.write(data)
    assert count==asset["size"] and hash.hexdigest()==digest,"Uploaded asset bytes did not match"
    print("Verified uploaded "+name,flush=True)
sums={}
for line in (temporary/"SHA256SUMS.txt").read_text().splitlines():
    digest,name=line.split(maxsplit=1);sums[name.strip()]=digest
for name in EXPECTED:
    if name!="SHA256SUMS.txt":assert sums[name]==EXPECTED[name]
with zipfile.ZipFile(temporary/"Worldcraft-Windows-x64-development.zip") as archive:
    info=next(n for n in archive.namelist() if n.endswith("/BUILD-INFO.txt"))
    assert COMPILED in archive.read(info).decode("utf-8-sig")
    baked=next(n for n in archive.namelist() if n.endswith("/examples/detailed-highlands.worldcraft.json"))
    world=json.loads(archive.read(baked));assert world["learned"]["detail"]["side"]==320 and world["learned"]["detail"]["spacing"]==30
    binary=next(n for n in archive.namelist() if n.endswith("/Worldcraft.exe"))
    data=archive.read(binary);assert data[:2]==b"MZ"
    pe=struct.unpack_from("<I",data,0x3c)[0];assert data[pe:pe+4]==b"PE\0\0" and struct.unpack_from("<H",data,pe+4)[0]==0x8664
image_entry,encoded=file("docs/media/detailed-windows.base64.txt")
image=base64.b64decode(encoded);assert image[:8]==b"\x89PNG\r\n\x1a\n" and struct.unpack(">II",image[16:24])==(640,360)
image_sha=hashlib.sha256(image).hexdigest()
report={"status":"awaiting_visual_review","source":COMPILED,"build_run":BUILD,"publication_run":os.environ["GITHUB_RUN_ID"],"tag":TAG,"release_id":RID,"assets":EXPECTED,"windows_image_blob":image_entry["sha"],"windows_image_sha256":image_sha,"linux_source":linux["source"],"linux_image_sha256":linux["image_sha256"],"export_validation_passed":True,"native_download_validation_passed":True,"uploaded_bytes_verified":True,"draft_lookup_recovered":True}
publish("qa/detailed-publication-status.json",json.dumps(report,indent=2).encode())
until=time.monotonic()+8*60
while time.monotonic()<until:
    try:review=json.loads(file(".worldcraft-detail-release-review.json",BRANCH)[1])
    except Exception:review={}
    if review.get("approved") and review.get("source")==COMPILED and review.get("run")==BUILD and review.get("tag")==TAG and review.get("windows_image_sha256")==image_sha and review.get("linux_image_sha256")==linux["image_sha256"] and review.get("native_download_validation_passed"):
        final=api("/releases/"+str(RID),{"draft":False,"prerelease":True,"make_latest":"false"},"PATCH")
        assert not final["draft"] and final["prerelease"]
        report["status"]="passed";report["release_url"]=final["html_url"];report["visual_review_passed"]=True
        publish("qa/detailed-publication-status.json",json.dumps(report,indent=2).encode())
        print("Published verified alpha: "+final["html_url"],flush=True);break
    time.sleep(10)
else:raise SystemExit("The verified draft remains unpublished because the exact capture review was not recorded")
