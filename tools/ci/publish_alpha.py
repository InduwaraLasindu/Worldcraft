"""Publish an explicitly experimental alpha only after its frozen product passed hosted validation."""
import base64,json,os,pathlib,subprocess,urllib.request
REPO=os.environ["GITHUB_REPOSITORY"]
API="https://api.github.com/repos/"+REPO
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json","X-GitHub-Api-Version":"2022-11-28"}
def get(path):
    with urllib.request.urlopen(urllib.request.Request(API+path,headers=headers),timeout=30) as response:return json.load(response)
def qa(path):
    entry=get("/contents/"+path+"?ref=codex/worldcraft-visual-qa")
    return json.loads(base64.b64decode(entry["content"]))
run=os.environ["WORLDCRAFT_VERIFIED_RUN"]
result=get("/actions/runs/"+run)
assert result["status"]=="completed" and result["conclusion"]=="success","Verification is not successful"
assert result["event"]=="push" and result["head_branch"]=="codex/worldcraft-foundation"
jobs=get("/actions/runs/"+run+"/jobs")["jobs"]
required={"prepare","core (ubuntu-latest)","core (windows-latest)","desktop (ubuntu-latest)","desktop (windows-latest)"}
assert required.issubset({job["name"] for job in jobs if job["conclusion"]=="success"}),"A required platform check did not pass"
distribution=qa("qa/windows-distribution-status.json")
native=qa("qa/windows-render-audit.json")
portable=qa("qa/windows-portable-visual.json")
assert distribution["status"]=="passed" and distribution["run"]==run
assert native["status"]=="passed" and native["run"]==run and len(native["images"])==8
assert portable["status"]=="passed_image_checks" and portable["visual_review_passed"] and portable["source"]==distribution["source"]
assert get("/actions/runs/"+portable["run"])["conclusion"]=="success","Packaged visual audit did not pass"
# Publication code/docs may differ; no product or dependency changes may bypass the tested binary.
subprocess.run(["git","diff","--exit-code",native["compiled_source"],"HEAD","--","crates","Cargo.toml","Cargo.lock"],check=True)
candidates=[release for release in get("/releases?per_page=30") if release["tag_name"].startswith("v0.2.0-alpha.1-build.") and release["target_commitish"]==distribution["source"]]
assert len(candidates)==1,"Expected exactly one matching verified draft"
release=candidates[0]
assets={asset["name"]:asset for asset in release["assets"]}
assert {"Worldcraft-Windows-x64-development.zip","Worldcraft-Setup-0.2.0-alpha.1.exe","SHA256SUMS.txt"}.issubset(assets)
for name,asset in assets.items():
    assert asset["size"]>0 and asset.get("digest","").startswith("sha256:"),"An uploaded asset lacks a checksum"
assert release["prerelease"],"Candidate must remain a prerelease"
notes="""Worldcraft 0.2.0 alpha 1 is an experimental native Windows x64 application for creating, exploring and editing procedural landscapes.

Download **Worldcraft-Setup-0.2.0-alpha.1.exe** for the per-user installer, or the portable ZIP. Classical worlds and the included precomputed AI example run offline. New AI regions use an optional checksum-pinned 22.5 MB coarse model. Python and Java are not required.

Implemented: seeded terrain and biomes, bounded LOD streaming and distant terrain, walking/flying, seven editing brushes and undo/redo, world persistence/recovery, regional maps, native Terrain Diffusion coarse-region inference, atmosphere/High volumetric mist/SSAO, experimental capability-gated ray-query lighting, and terrain GLB/OBJ/16-bit PNG export.

Validation: 54 core/ML/desktop tests on each OS; actual native model inference on Linux and Windows; Linux software Vulkan UI/render/ray/learned-world audits; Windows software Direct3D 12 API/state checks with DXC plus a separately reviewed 640x360 scene from the packaged offline AI example; portable/installed runtime checks and safe uninstall; official Khronos GLB validation and decoded heightmap reconstruction.

This is **not the completed master specification or a stable production engine**. The full diffusion cascade/GPU inference, hydraulic river systems, caves, graph editing, asset importing, advanced ecosystems and cinematic tools are incomplete. Gaming-GPU performance and Windows user interaction are not broadly certified. Experimental ray queries remain capability-gated. The installer is unsigned.

Product/tool source: SOURCE_COMMIT
Compiled product source: COMPILED_COMMIT
[Verification run](https://github.com/REPOSITORY/actions/runs/VERIFIED_RUN) · [Packaged Windows scene audit](https://github.com/REPOSITORY/actions/runs/PORTABLE_RUN) · [Source and remaining scope](https://github.com/REPOSITORY/blob/SOURCE_COMMIT/MASTER_SPEC_COVERAGE.md)
"""
notes=notes.replace("SOURCE_COMMIT",distribution["source"]).replace("COMPILED_COMMIT",native["compiled_source"]).replace("REPOSITORY",REPO).replace("VERIFIED_RUN",run).replace("PORTABLE_RUN",portable["run"])
body={"draft":False,"prerelease":True,"make_latest":"false","name":"Worldcraft 0.2.0-alpha.1 — Windows x64","body":notes}
request=urllib.request.Request(API+"/releases/"+str(release["id"]),headers=headers,data=json.dumps(body).encode(),method="PATCH")
with urllib.request.urlopen(request,timeout=30) as response:published=json.load(response)
assert not published["draft"] and published["prerelease"]
print("Published verified alpha prerelease: "+published["html_url"],flush=True)
