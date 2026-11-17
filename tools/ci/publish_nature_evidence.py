"""Retain real native captures with immutable source/run provenance on a QA branch."""
import base64,hashlib,json,os,pathlib,struct,subprocess,time,urllib.error,urllib.request
ROOT=pathlib.Path.cwd().resolve()
assert os.environ.get('GITHUB_ACTIONS')=='true' and ROOT==pathlib.Path(os.environ['GITHUB_WORKSPACE']).resolve()
SOURCE=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
assert SOURCE==os.environ['GITHUB_SHA']
BRANCH='codex/worldcraft-nature-evidence'
API='https://api.github.com/repos/'+os.environ['GITHUB_REPOSITORY']
HEADERS={'Authorization':'Bearer '+os.environ['GH_TOKEN'],'Accept':'application/vnd.github+json'}
def api(path,data=None,method=None):
    request=urllib.request.Request(API+path,headers=HEADERS,data=None if data is None else json.dumps(data).encode(),method=method)
    with urllib.request.urlopen(request,timeout=40) as response:return json.load(response)
try:api('/git/ref/heads/'+BRANCH)
except urllib.error.HTTPError as error:
    if error.code!=404:raise
    api('/git/refs',{'ref':'refs/heads/'+BRANCH,'sha':SOURCE},'POST')
entries=[];images=[]
for path in sorted(ROOT.glob('worldcraft-nature-*.png')):
    data=path.read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n' and 12000<len(data)<8*1024*1024
    width,height=struct.unpack('>II',data[16:24]);assert (width,height)==(960,640)
    blob=api('/git/blobs',{'encoding':'base64','content':base64.b64encode(data).decode()},'POST')
    relative='docs/media/nature/'+SOURCE+'/'+path.name
    entries.append({'path':relative,'mode':'100644','type':'blob','sha':blob['sha']})
    images.append({'path':relative,'width':width,'height':height,'sha256':hashlib.sha256(data).hexdigest()})
assert len(images)>=9
report={'source':SOURCE,'run':os.environ['GITHUB_RUN_ID'],'status':'captured','native_log_gate_passed':True,
    'images':images,'fixture':json.loads((ROOT/'nature-render-audit.json').read_text()),
    'materials':json.loads((ROOT/'crates/worldcraft/embedded/manifest.json').read_text()),
    'scope':'Hosted software Vulkan; image appearance still requires review, not gaming performance certification'}
entries.append({'path':'docs/evidence/nature-'+SOURCE+'.json','mode':'100644','type':'blob','content':json.dumps(report,indent=2)+'\n'})
for attempt in range(3):
    head=api('/git/ref/heads/'+BRANCH)['object']['sha'];old=api('/git/commits/'+head)
    tree=api('/git/trees',{'base_tree':old['tree']['sha'],'tree':entries},'POST')
    commit=api('/git/commits',{'message':'test: retain native nature captures '+SOURCE[:12],'tree':tree['sha'],'parents':[head]},'POST')
    try:
        api('/git/refs/heads/'+BRANCH,{'sha':commit['sha'],'force':False},'PATCH');break
    except urllib.error.HTTPError as error:
        if error.code not in (409,422) or attempt==2:raise
        time.sleep(2)
print('Retained',len(images),'native images for source',SOURCE)
