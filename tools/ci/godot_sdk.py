"""Download the pinned official Godot editor and one export template."""
import hashlib
import os
import pathlib
import stat
import sys
import tempfile
import urllib.request
import zipfile


tag = "4.7.2-stable"
base = f"https://github.com/godotengine/godot-builds/releases/download/{tag}/Godot_v{tag}"
root = pathlib.Path(".godot-sdk")
root.mkdir(exist_ok=True)
editor, template = sys.argv[1:]
checksums = {
    "linux.x86_64.zip": "9aa00f7a605200940bce3027a567b782f49bd8e940dd06ae9e987bd65aee1b1467edd56ed84fcdcbdd44354bf613bdbb4e5d2913e925850368e150c59ed54c65",
    "win64.exe.zip": "83decd58fdf67b9d657958a1ae6bf1929c20785315a81effe245874cdc57acb709bf868e00778a96984338c1b29dafdb453c6847747694621c6ecf5da2259993",
    "export_templates.tpz": "ca4d71c4d7b81dfc15d1a98baa07534aa95b03fdda78a0075b06672e1648d2e5f40980c9adc28d23e1b92e732ee7bf3461997aa804af74ec2fcd7a93ccb84079",
}


def fetch(suffix, destination):
    urllib.request.urlretrieve(f"{base}_{suffix}", destination)
    with destination.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha512").hexdigest()
    if actual != checksums[suffix]:
        raise ValueError(f"Godot SDK checksum mismatch for {suffix}")


if sys.platform == "win32":
    template_root = pathlib.Path(os.environ["APPDATA"]) / "Godot/export_templates"
else:
    template_root = pathlib.Path(os.environ.get("XDG_DATA_HOME", pathlib.Path.home() / ".local/share")) / "godot/export_templates"
template_root /= tag.replace("-", ".")
template_root.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory() as tmp:
    archive = pathlib.Path(tmp) / "editor.zip"
    fetch(f"{editor}.zip", archive)
    with zipfile.ZipFile(archive) as package:
        member = next(name for name in package.namelist()
                      if pathlib.PurePosixPath(name).name == f"Godot_v{tag}_{editor}")
        target = root / pathlib.PurePosixPath(member).name
        target.write_bytes(package.read(member))
        target.chmod(target.stat().st_mode | stat.S_IXUSR)
    archive = pathlib.Path(tmp) / "templates.zip"
    fetch("export_templates.tpz", archive)
    with zipfile.ZipFile(archive) as package:
        target = template_root / template
        target.write_bytes(package.read(f"templates/{template}"))
        target.chmod(target.stat().st_mode | stat.S_IXUSR)
