"""Pinned build inputs and redistributables; no Python/SDK is shipped with the app."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile

from bootstrap_ort import fetch

PROJECT = Path(__file__).resolve().parents[2]
SOURCES = json.loads((PROJECT / 'tools/upscaler_sources.json').read_text())


def safe_path(path):
    value = PurePosixPath(path)
    if value.is_absolute() or '..' in value.parts or '\\' in path or ':' in path or path in ('', '.'):
        raise ValueError('Invalid SDK input path')
    return value


def runtime_files(platform):
    for item in SOURCES['nvidia']['files']:
        if item['platform'] == platform and '/rel/' in item['path']:
            yield 'dlss-sdk', item
    if platform == 'win-x64':
        for item in SOURCES['amd']['files']:
            yield 'fsr-sdk', item


def bootstrap(root, platform):
    if platform not in ('win-x64', 'linux-x64'):
        raise ValueError('Unsupported native SDK target')
    root.mkdir(parents=True, exist_ok=True)
    jobs = []
    for item in SOURCES['nvidia']['files']:
        if item['platform'] not in ('all', platform):
            continue
        path = safe_path(item['path'])
        destination = root / 'dlss-sdk' / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        url = ('https://raw.githubusercontent.com/NVIDIA/DLSS/'
               + SOURCES['nvidia']['commit'] + '/' + path.as_posix())
        jobs.append((url, destination, item['sha256'], item['size']))
    if platform == 'win-x64':
        for item in SOURCES['amd']['files']:
            path = safe_path(item['path'])
            destination = root / 'fsr-sdk' / path.name
            destination.parent.mkdir(parents=True, exist_ok=True)
            url = ('https://raw.githubusercontent.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK/'
                   + SOURCES['amd']['commit'] + '/' + path.as_posix())
            jobs.append((url, destination, item['sha256'], item['size']))
    with ThreadPoolExecutor(max_workers=4) as pool:
        for _ in pool.map(lambda args: fetch(*args), jobs):
            pass
    vulkan = SOURCES['vulkan_headers']
    archive = fetch(vulkan['url'], root / 'vulkan-headers.tar.gz', vulkan['sha256'], vulkan['size'])
    with tempfile.TemporaryDirectory(prefix='vulkan-headers-', dir=root) as temporary:
        with tarfile.open(archive) as package:
            # 'data' excludes links escaping this directory and special files.
            package.extractall(temporary, filter='data')
        include = next(Path(temporary).glob('*/include'))
        shutil.copytree(include, root / 'vulkan-sdk/include', dirs_exist_ok=True)
    return {
        'DLSS_SDK': str((root / 'dlss-sdk').resolve()),
        'VULKAN_SDK': str((root / 'vulkan-sdk').resolve()),
        'WORLDCRAFT_UPSCALER_ROOT': str(root.resolve()),
        'WORLDCRAFT_CHECK_FSR_ABI': '1',
    }


def copy_runtime(root, platform, destination):
    import hashlib
    destination.mkdir(parents=True, exist_ok=True)
    for folder, item in runtime_files(platform):
        path = safe_path(item['path'])
        source = root / folder / (path if folder == 'dlss-sdk' else path.name)
        with source.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if source.stat().st_size != item['size'] or digest != item['sha256']:
            raise ValueError('Refusing unverified upscaler redistributable')
        shutil.copy2(source, destination / path.name)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path)
    parser.add_argument('--copy-to', type=Path)
    args = parser.parse_args()
    root = args.root or Path(os.environ.get('RUNNER_TEMP', PROJECT / '.dev')) / 'worldcraft-upscalers'
    platform = 'win-x64' if os.name == 'nt' else 'linux-x64'
    if args.copy_to:
        copy_runtime(root, platform, args.copy_to)
        print(json.dumps({'status': 'verified-runtime-copied', 'platform': platform}))
        return
    environment = bootstrap(root, platform)
    if os.environ.get('GITHUB_ENV'):
        with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as output:
            for name, value in environment.items():
                output.write(name + '=' + value + '\n')
    print(json.dumps({'status': 'verified', 'platform': platform,
                      'nvidia': SOURCES['nvidia']['version'],
                      'amd': SOURCES['amd']['version'], 'environment': environment}))


if __name__ == '__main__':
    main()
