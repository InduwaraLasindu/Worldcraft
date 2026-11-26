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


def verify_forest(root):
    source = os.environ['GITHUB_SHA']
    biomes = json.loads((root / 'biome-render-audit.json').read_text())
    travel = json.loads((root / 'forest-travel-audit.json').read_text())
    for report in (biomes, travel):
        assert report['source'] == source and report['authored_instances'] == 0
    assert biomes['status'] == 'passed' and len(biomes['sites']) == 7
    assert all(row['procedural_surfaces'] for row in biomes['sites'])
    assert [row['distance_m'] for row in travel['captures']] == [92, 126, 130, 160, 192, 218, 226, 248]
    names = [row['image'] for row in biomes['sites'] + travel['captures']]
    for name in names + ['worldcraft-material-savanna-scanned.png']:
        assert Path(name).name == name
        with (root / name).open('rb') as image:
            header = image.read(24)
        assert header[:8] == bytes.fromhex('89504e470d0a1a0a')
        assert int.from_bytes(header[16:20], 'big') == 960 and int.from_bytes(header[20:24], 'big') == 640
    print('Seven exact-source landscapes, live material switch and eight forest-route captures verified.')


def verify_relief(windows, linux):
    source = os.environ['GITHUB_SHA']
    for root,count in ((windows,3),(linux,9)):
        route=json.loads((root/'relief-render-audit.json').read_text(encoding='utf-8-sig'))
        pixels=json.loads((root/'relief-image-audit.json').read_text(encoding='utf-8-sig'))
        for report in (route,pixels):
            assert report['source']==source and report['status']=='passed'
        assert len(route['cases'])==count and route['authored_instances']==0
        assert route['cases'][2]['side']==4096 and route['cases'][2]['pom_layers']==24
        assert pixels['rock']['fraction_over_4']>.045
        for shot in route['cases']:
            name=shot['image']
            assert name=='worldcraft-relief-'+shot['label']+'.png' and (root/name).stat().st_size>6000
        pack=json.loads((root/'crates/worldcraft/embedded/manifest.json').read_text())['relief']
        assert pack['side']==4096 and pack['mips']==13 and len(pack['layers'])==3
        assert [f['size'] for f in pack['outputs']]==[67108944,33554472]
    ray=json.loads((linux/'relief-ray-audit.json').read_text())
    assert ray['source']==source and ray['completed_ray_dispatch'] and ray['stone_proxy_textured']
    assert ray['primary_pom_layers']==24 and ray['secondary_stone_texture_side']==256
    print('Matched 4K POM pixels on two backends, procedural textures and completed textured ray queries verified.')


if __name__ == '__main__':
    if sys.argv[1] == '--relief':
        verify_relief(Path(sys.argv[2]),Path(sys.argv[3]))
    elif sys.argv[1] == '--forest':
        verify_forest(Path(sys.argv[2]))
    else:
        verify(Path(sys.argv[1]), Path(sys.argv[2]))
