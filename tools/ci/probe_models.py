"""Inspect pinned public model provenance on the hosted runner; no weight downloads."""
import base64,json,os,pathlib,subprocess,urllib.request,datetime
repo="xandergos/terrain-diffusion-30m-onnx"
revision="ad2df557eca5645f588766101cf3bc3682455c3e"
def get(url):
    with urllib.request.urlopen(urllib.request.Request(url,headers={"User-Agent":"Worldcraft-model-audit"}),timeout=60) as r:return r.read()
result={"repository":repo,"revision":revision,"errors":[]}
try:
    metadata=json.loads(get("https://huggingface.co/api/models/"+repo+"/revision/"+revision+"?blobs=true"))
    result["metadata"]=metadata
    small={}
    for item in metadata.get("siblings",[]):
        name=item["rfilename"]
        if name.lower() in ["readme.md","license","license.txt","license.md","world_pipeline_config.json"]:
            try:small[name]=get("https://huggingface.co/"+repo+"/resolve/"+revision+"/"+name).decode()
            except Exception as e:result["errors"].append(name+": "+str(e))
    result["small_files"]=small
except Exception as e:result["errors"].append(str(e))
try:
    p=subprocess.run(["vulkaninfo"],capture_output=True,text=True,timeout=60)
    result["vulkan"]=[line.strip() for line in p.stdout.splitlines() if any(k in line for k in ["deviceName","deviceType","driverInfo","rayQuery","VK_KHR_acceleration_structure","VK_KHR_ray_tracing_pipeline","VK_KHR_ray_query"])]
except Exception as e:result["vulkan_error"]=str(e)
result["updated"]=datetime.datetime.now(datetime.timezone.utc).isoformat()
api="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/qa/terrain-model-probe.json"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
def request(url,data=None):
    req=urllib.request.Request(url,data=None if data is None else json.dumps(data).encode(),headers=headers,method="GET" if data is None else "PUT")
    with urllib.request.urlopen(req,timeout=60) as r:return json.load(r)
body={"message":"test: audit learned model provenance and hosted Vulkan capabilities",
    "content":base64.b64encode(json.dumps(result,indent=2).encode()).decode(),"branch":"codex/worldcraft-visual-qa"}
try:body["sha"]=request(api+"?ref=codex/worldcraft-visual-qa")["sha"]
except Exception:pass
request(api,body)
print("Published pinned model and device audit.")
