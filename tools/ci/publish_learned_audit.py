import base64, json, os, pathlib, urllib.request
api="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]
branch="codex/worldcraft-visual-qa"
def call(path,data=None):
    headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
    req=urllib.request.Request(api+path,headers=headers,data=None if data is None else json.dumps(data).encode())
    with urllib.request.urlopen(req,timeout=90) as r:return json.load(r)
osname=os.environ["AUDIT_OS"].lower()
for local,remote in [("learned-audit.log","qa/learned-"+osname+".log"),("learned-audit.worldcraft.json","qa/learned-"+osname+".worldcraft.json")]:
    file=pathlib.Path(local)
    if not file.exists():continue
    for attempt in range(3):
        try:
            try:previous=call("/contents/"+remote+"?ref="+branch)["sha"]
            except Exception:previous=None
            body={"message":"test: record native learned terrain evidence","branch":branch,"content":base64.b64encode(file.read_bytes()).decode()}
            if previous:body["sha"]=previous
            request=urllib.request.Request(api+"/contents/"+remote,headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"},data=json.dumps(body).encode(),method="PUT")
            with urllib.request.urlopen(request,timeout=90) as r:print(remote,r.status)
            break
        except Exception:
            if attempt==2:raise
