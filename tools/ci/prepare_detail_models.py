"""Discover immutable model metadata and stage verified weights on hosted CI only."""
import hashlib,json,os,pathlib,shutil,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve()
REV="ad2df557eca5645f588766101cf3bc3682455c3e"
REPO="xandergos/terrain-diffusion-30m-onnx"
folder=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-detail-models"/REV
folder.mkdir(parents=True,exist_ok=True)
with urllib.request.urlopen("https://huggingface.co/api/models/"+REPO+"/revision/"+REV+"?blobs=true",timeout=90) as response:metadata=json.load(response)
assert metadata["sha"]==REV
names=["coarse_model.onnx","base_model.onnx","decoder_model.onnx","world_pipeline_config.json"]
siblings={item["rfilename"]:item for item in metadata["siblings"]}
assets={}
for name in names:
    entry=siblings[name];lfs=entry.get("lfs")
    assets[name]={"size":entry.get("size",lfs.get("size") if lfs else 0),"sha256":lfs.get("sha256") if lfs else None,"git_blob":entry.get("blobId"),"url":"https://huggingface.co/"+REPO+"/resolve/"+REV+"/"+name}
assert assets["coarse_model.onnx"]["sha256"]=="d6ca15b21b2e35d5e594a9ac7a4249a2376590c0ad2b5b49a1e6e2d033450008"
def download(name):
    asset=assets[name];target=folder/name
    def valid():
        if not target.exists() or target.stat().st_size!=asset["size"]:return False
        digest=hashlib.sha256() if asset["sha256"] else hashlib.sha1(("blob "+str(asset["size"])+"\0").encode())
        with target.open("rb") as source:
            while data:=source.read(1024*1024):digest.update(data)
        return digest.hexdigest()==(asset["sha256"] or asset["git_blob"])
    if valid():return
    assert 0<asset["size"]<3*1024**3 and shutil.disk_usage(folder).free>asset["size"]+1024**3
    temporary=target.with_suffix(".part")
    print("Downloading "+name+" ("+str(asset["size"])+" bytes)",flush=True)
    count=0;next_progress=128*1024**2
    with urllib.request.urlopen(asset["url"],timeout=180) as response,temporary.open("wb") as output:
        while data:=response.read(1024*1024):
            count+=len(data);assert count<=asset["size"]
            output.write(data)
            if count>=next_progress:
                print(name+": "+str(count)+"/"+str(asset["size"]),flush=True);next_progress+=128*1024**2
    assert count==asset["size"]
    temporary.replace(target)
    if not valid():target.unlink();raise AssertionError("Model checksum mismatch")
download("world_pipeline_config.json")
config=json.loads((folder/"world_pipeline_config.json").read_text())
(ROOT/"detail-model-assets.json").write_text(json.dumps({"revision":REV,"assets":assets,"config":config},indent=2))
for name in names[:3]:download(name)
print("All three immutable model files verified in hosted temporary storage.",flush=True)
