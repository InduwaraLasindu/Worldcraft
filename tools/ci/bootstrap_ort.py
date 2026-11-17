"""Prepare pinned native libraries on hosted CI. No Python runtime is shipped."""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import tarfile
import urllib.error
import urllib.request
import zipfile

VERSION = "1.32.0"
PRIMARY = {
    "win-x64": "aaa1d04fcf86bd31e84b90bc0cbde80e632bd0720dcc5d8ba05c4fe45c09d127",
    "linux-x64": "bb169b7e374731730d2ae406fb39de7be4b315f413a9132efe896261c31bb503",
}
# Microsoft-published NuGet package, same runtime version and C ABI. Its SHA-512
# and size were independently checked against the official NuGet catalog entry.
NUGET_URL = ("https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/"
             "1.32.0/microsoft.ml.onnxruntime.1.32.0.nupkg")
NUGET_SHA256 = "a9150bc2992cbbb6722070da9910e1ffa989a8bfc67bdd5bb8e745ca755641db"
NUGET_SIZE = 184391411


def fetch(url, destination, expected, size=None):
    def valid(path):
        if not path.is_file() or (size is not None and path.stat().st_size != size):
            return False
        with path.open('rb') as stream:
            return hashlib.file_digest(stream, 'sha256').hexdigest() == expected
    if valid(destination):
        return destination
    part = destination.with_suffix(destination.suffix + '.part')
    try:
        request = urllib.request.Request(url, headers={'User-Agent': 'Worldcraft-hosted-ci/0.2'})
        with urllib.request.urlopen(request, timeout=90) as response, part.open('wb') as output:
            while block := response.read(1024 * 1024):
                output.write(block)
        if not valid(part):
            raise ValueError('Native runtime checksum or size mismatch')
        part.replace(destination)
    finally:
        part.unlink(missing_ok=True)
    return destination


def extract_nuget(archive, destination, platform):
    bundle = destination / f'onnxruntime-{platform}-{VERSION}'
    library_folder = bundle / 'lib'
    library_folder.mkdir(parents=True, exist_ok=True)
    prefix = f'runtimes/{platform}/native/'
    with zipfile.ZipFile(archive) as package:
        for item in package.infolist():
            path = PurePosixPath(item.filename)
            if path.is_absolute() or '..' in path.parts or '\\' in item.filename:
                raise ValueError('Invalid runtime archive path')
        native = [item for item in package.infolist() if item.filename.startswith(prefix)
                  and not item.is_dir() and (item.filename.endswith('.dll') or '.so' in PurePosixPath(item.filename).name)]
        if not native:
            raise ValueError('Runtime package has no native libraries for ' + platform)
        for item in native:
            name = item.filename.removeprefix(prefix)
            if len(PurePosixPath(name).parts) != 1:
                raise ValueError('Unexpected nested native library')
            (library_folder / name).write_bytes(package.read(item))
        for name in ['LICENSE', 'ThirdPartyNotices.txt']:
            (bundle / name).write_bytes(package.read(name))
    (bundle / 'VERSION_NUMBER').write_text(VERSION + '\n')
    return library_folder / ('onnxruntime.dll' if platform == 'win-x64' else 'libonnxruntime.so')


def bootstrap(root, platform):
    if platform not in PRIMARY:
        raise ValueError('Unsupported hosted runtime platform')
    root.mkdir(parents=True, exist_ok=True)
    suffix = 'zip' if platform == 'win-x64' else 'tgz'
    destination = root / 'extracted'
    destination.mkdir(exist_ok=True)
    url = (f'https://github.com/microsoft/onnxruntime/releases/download/v{VERSION}/'
           f'onnxruntime-{platform}-{VERSION}.{suffix}')
    try:
        archive = fetch(url, root / ('runtime.' + suffix), PRIMARY[platform])
    except urllib.error.URLError as error:
        # Keep TLS/hash checks. Only an unavailable primary source selects this
        # independently pinned official package; corrupt bytes remain a hard error.
        print('Primary runtime unavailable; selecting pinned Microsoft NuGet package:', error)
        archive = fetch(NUGET_URL, root / 'runtime.nupkg', NUGET_SHA256, NUGET_SIZE)
        library = extract_nuget(archive, destination, platform)
        return library, 'Microsoft NuGet', NUGET_SHA256
    if suffix == 'tgz':
        with tarfile.open(archive) as package:
            package.extractall(destination, filter='data')
    else:
        with zipfile.ZipFile(archive) as package:
            for item in package.infolist():
                target = (destination / item.filename).resolve()
                if not target.is_relative_to(destination.resolve()):
                    raise ValueError('Invalid runtime archive path')
            package.extractall(destination)
    library = next(destination.rglob('onnxruntime.dll' if platform == 'win-x64' else 'libonnxruntime.so'))
    return library, 'GitHub release', PRIMARY[platform]


def main():
    platform = 'win-x64' if os.name == 'nt' else 'linux-x64'
    library, source, digest = bootstrap(Path(os.environ['RUNNER_TEMP']) / 'worldcraft-ort', platform)
    if not library.is_file():
        raise ValueError('Prepared native runtime is missing')
    with open(os.environ['GITHUB_ENV'], 'a') as output:
        output.write('WORLDCRAFT_ORT_DYLIB=' + str(library) + '\n')
    print(json.dumps({'runtime': 'ONNX Runtime ' + VERSION, 'source': source,
                      'archive_sha256': digest, 'library': str(library), 'hosted_only': True}))


if __name__ == '__main__':
    main()
