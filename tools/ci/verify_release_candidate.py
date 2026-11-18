"""Release only the tested source and checksum-pinned redistributables."""
import hashlib
import json
import os
from pathlib import Path
import sys
import zipfile


def verify(windows, linux):
    source = os.environ['GITHUB_SHA']
    for root in (windows, linux):
        report = json.loads((root / 'renderer-settings-audit.json').read_text(encoding='utf-8-sig'))
        assert report['source'] == source and report['status'] == 'passed'
        assert len(report['cases']) == 9
        assert json.loads((root / 'renderer-settings-image-audit.json').read_text())['status'] == 'passed'
    sources = json.loads(Path('tools/upscaler_sources.json').read_text())
    package = windows / 'dist/Worldcraft-Windows-x64-development.zip'
    with zipfile.ZipFile(package) as archive:
        prefix = 'Worldcraft-Windows-x64-development/'
        info = archive.read(prefix + 'BUILD-INFO.txt').decode('utf-8-sig')
        assert 'Source commit: ' + source in info
        assert 'NVIDIA DLSS/DLAA feature compiled: True' in info
        assert archive.getinfo(prefix + 'Worldcraft.exe').file_size > 1_000_000
        for group in ('nvidia', 'amd'):
            for item in sources[group]['files']:
                if group == 'nvidia' and not (item['platform'] == 'win-x64' and '/rel/' in item['path']):
                    continue
                name = prefix + Path(item['path']).name
                assert archive.getinfo(name).file_size == item['size']
                with archive.open(name) as binary:
                    assert hashlib.file_digest(binary, 'sha256').hexdigest() == item['sha256']
        for notice in ('AMD-FSR-SDK-2.4.md', 'AMD-FSR-ThirdParty.md', 'NVIDIA-RTX-SDK-LICENSE.txt', 'NVIDIA-DLSS-NOTICES.txt'):
            assert archive.read(prefix + 'licenses/' + notice) == Path('licenses', notice).read_bytes()
        assert json.loads(archive.read(prefix + 'upscaler_sources.json')) == sources
    print('Exact-source renderer reports, executable, runtime hashes and notices verified.')


if __name__ == '__main__':
    verify(Path(sys.argv[1]), Path(sys.argv[2]))
