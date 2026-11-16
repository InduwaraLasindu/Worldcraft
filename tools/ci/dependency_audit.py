"""Read-only public dependency advisory check; no installs or private source uploads."""
import concurrent.futures,datetime,hashlib,json,os,pathlib,time,tomllib,urllib.request
def request(url,data=None):
    body=None if data is None else json.dumps(data).encode()
    headers={"Content-Type":"application/json","User-Agent":"Worldcraft-dependency-audit"}
    for attempt in range(3):
        try:
            with urllib.request.urlopen(urllib.request.Request(url,headers=headers,data=body),timeout=60) as r:return json.load(r)
        except Exception:
            if attempt==2:raise
            time.sleep(2)
lock=pathlib.Path("Cargo.lock").read_bytes()
packages=[p for p in tomllib.loads(lock.decode())["package"] if p.get("source","").startswith("registry+")]
queries=[{"package":{"ecosystem":"crates.io","name":p["name"]},"version":p["version"]} for p in packages]
findings={}
pending=list(enumerate(queries))
while pending:
    batch=request("https://api.osv.dev/v1/querybatch",{"queries":[q for _,q in pending]})
    assert len(batch["results"])==len(pending)
    next_page=[]
    for (index,query),result in zip(pending,batch["results"]):
        for vuln in result.get("vulns",[]):findings.setdefault(vuln["id"],set()).add(index)
        if result.get("next_page_token"):next_page.append((index,{**query,"page_token":result["next_page_token"]}))
    pending=next_page
def details(item):
    identifier,indexes=item
    record=request("https://api.osv.dev/v1/vulns/"+identifier)
    return {"advisory":record,"locked_packages":[{"name":packages[i]["name"],"version":packages[i]["version"]} for i in sorted(indexes)]}
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:records=list(pool.map(details,sorted(findings.items())))
def informational(item):
    record=item["advisory"]
    if record.get("database_specific",{}).get("informational"):return True
    names={p["name"] for p in item["locked_packages"]}
    affected=[a for a in record.get("affected",[]) if a.get("package",{}).get("name") in names]
    return bool(affected) and all(a.get("database_specific",{}).get("informational") for a in affected)
active=[r for r in records if not r["advisory"].get("withdrawn") and not informational(r)]
notices=[r for r in records if informational(r)]
report={"source":os.environ["GITHUB_SHA"],"cargo_lock_sha256":hashlib.sha256(lock).hexdigest(),
"checked_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"registry_packages_checked":len(packages),
"status":("passed_with_maintenance_notices" if notices else "passed") if not active else "review_required","active_vulnerability_advisories":len(active),"maintenance_notices":len(notices),"advisories":records,
"scope":"Locked registry packages, including optional/other-platform entries; not a native-library or binary security certification."}
output=json.dumps(report,indent=2).encode()
pathlib.Path("dependency-audit.json").write_bytes(output)
api="https://api.github.com/repos/"+os.environ["GITHUB_REPOSITORY"]+"/contents/qa/dependency-audit.json"
headers={"Authorization":"Bearer "+os.environ["GH_TOKEN"],"Accept":"application/vnd.github+json"}
for attempt in range(3):
    try:
        try:
            with urllib.request.urlopen(urllib.request.Request(api+"?ref=codex/worldcraft-visual-qa",headers=headers),timeout=60) as r:sha=json.load(r)["sha"]
        except Exception:sha=None
        body={"message":"test: record locked dependency advisory evidence","branch":"codex/worldcraft-visual-qa","content":__import__("base64").b64encode(output).decode()}
        if sha:body["sha"]=sha
        with urllib.request.urlopen(urllib.request.Request(api,headers=headers,data=json.dumps(body).encode(),method="PUT"),timeout=60) as r:print("Published dependency report",r.status)
        break
    except Exception:
        if attempt==2:raise
        time.sleep(2)
print(json.dumps({"status":report["status"],"packages":len(packages),"advisory_ids":[r["advisory"]["id"] for r in records],"active_noninformational":len(active)}))
