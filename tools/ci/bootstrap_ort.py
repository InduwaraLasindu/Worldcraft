"""Fetch the pinned native runtime on hosted CI; never installs on the owner PC."""
import hashlib, json, os, pathlib, shutil, tarfile, urllib.request, zipfile
name="win-x64" if os.name=="nt" else "linux-x64"
suffix="zip" if os.name=="nt" else "tgz"
expected={"win-x64":"aaa1d04fcf86bd31e84b90bc0cbde80e632bd0720dcc5d8ba05c4fe45c09d127",
"linux-x64":"bb169b7e374731730d2ae406fb39de7be4b315f413a9132efe896261c31bb503"}[name]
root=pathlib.Path(os.environ["RUNNER_TEMP"])/"worldcraft-ort"
root.mkdir(exist_ok=True)
archive=root/("runtime."+suffix)
url=f"https://github.com/microsoft/onnxruntime/releases/download/v1.32.0/onnxruntime-{name}-1.32.0.{suffix}"
if not archive.exists():
    with urllib.request.urlopen(url,timeout=90) as response,archive.open("wb") as output:shutil.copyfileobj(response,output)
assert hashlib.sha256(archive.read_bytes()).hexdigest()==expected,"Native runtime checksum mismatch"
destination=root/"extracted"
destination.mkdir(exist_ok=True)
if suffix=="tgz":
    with tarfile.open(archive) as package:package.extractall(destination,filter="data")
else:
    with zipfile.ZipFile(archive) as package:
        for item in package.infolist():
            target=(destination/item.filename).resolve()
            assert target.is_relative_to(destination.resolve()),"Invalid runtime archive path"
        package.extractall(destination)
library=next(destination.rglob("onnxruntime.dll" if os.name=="nt" else "libonnxruntime.so"))
with open(os.environ["GITHUB_ENV"],"a") as output:output.write("WORLDCRAFT_ORT_DYLIB="+str(library)+"\n")
print(json.dumps({"runtime":"ONNX Runtime 1.32.0","archive_sha256":expected,"library":str(library),"hosted_only":True}))
