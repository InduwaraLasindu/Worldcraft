"""Prepare checksum-pinned official DXC only inside a disposable hosted Windows workspace."""
import hashlib,json,os,pathlib,shutil,stat,urllib.request,zipfile
ROOT=pathlib.Path.cwd().resolve()
assert os.name=="nt" and ROOT==pathlib.Path(os.environ["GITHUB_WORKSPACE"]).resolve(),"Hosted Windows only"
VERSION="v1.9.2609"
URL="https://github.com/microsoft/DirectXShaderCompiler/releases/download/"+VERSION+"/dxc_2026_09_29.zip"
SHA="ad31b1fc8443175d204f77a611fdb3ef2ec42759bdc2f1167368de24a4a7e7f1"
work=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-dxc"
work.mkdir(parents=True,exist_ok=True)
archive=work/"dxc.zip"
if not archive.exists() or hashlib.sha256(archive.read_bytes()).hexdigest()!=SHA:
    with urllib.request.urlopen(urllib.request.Request(URL,headers={"User-Agent":"Worldcraft-hosted-verification"}),timeout=90) as response,archive.open("wb") as output:
        shutil.copyfileobj(response,output)
assert hashlib.sha256(archive.read_bytes()).hexdigest()==SHA,"DXC archive checksum mismatch"
extracted=work/"extracted";extracted.mkdir(exist_ok=True)
with zipfile.ZipFile(archive) as bundle:
    assert sum(entry.file_size for entry in bundle.infolist())<512*1024*1024
    for entry in bundle.infolist():
        target=(extracted/entry.filename).resolve()
        assert target.is_relative_to(extracted.resolve()),"Unsafe DXC archive path"
        assert not stat.S_ISLNK(entry.external_attr>>16),"Unexpected archive symlink"
    bundle.extractall(extracted)
for name in ["dxcompiler.dll","dxil.dll"]:
    source=extracted/"bin"/"x64"/name
    assert source.is_file(),"DXC runtime component missing: "+name
    shutil.copy2(source,ROOT/name)
notices=ROOT/"dxc-notices";notices.mkdir(exist_ok=True)
legal=[path for path in extracted.glob("*") if path.is_file() and ("license" in path.name.lower() or "notice" in path.name.lower())]
assert legal,"Official DXC notices are missing"
for path in legal:shutil.copy2(path,notices/path.name)
(notices/"PROVENANCE.json").write_text(json.dumps({"version":VERSION,"url":URL,"archive_sha256":SHA},indent=2))
print("Verified official DXC "+VERSION+" and copied its native libraries and notices.",flush=True)
