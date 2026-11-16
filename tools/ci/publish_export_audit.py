import base64,json,os,pathlib,urllib.request
file=pathlib.Path("export-validation.json")
if not file.exists():raise SystemExit("No validator report was produced")
api="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/qa/export-validation.json"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
try:
    with urllib.request.urlopen(urllib.request.Request(api+"?ref=codex/worldcraft-visual-qa",headers=headers),timeout=60) as r:sha=json.load(r)["sha"]
except Exception:sha=None
body={"message":"test: record Khronos export validation","content":base64.b64encode(file.read_bytes()).decode(),"branch":"codex/worldcraft-visual-qa"}
if sha:body["sha"]=sha
with urllib.request.urlopen(urllib.request.Request(api,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=90) as r:print(r.status)
