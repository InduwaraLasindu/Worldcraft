"""Retain unedited native PNGs from a successful exact-source Linux job."""
import base64
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import urllib.request

ROOT = Path(sys.argv[1])
SOURCE = os.environ['WORLDCRAFT_CAPTURE_SOURCE']
RUN = os.environ['WORLDCRAFT_CAPTURE_RUN']
BRANCH = 'codex/worldcraft-render-evidence'
API = 'https://api.github.com/repos/' + os.environ['GITHUB_REPOSITORY']
HEADERS = {'Authorization': 'Bearer ' + os.environ['GH_TOKEN'], 'Accept': 'application/vnd.github+json'}


def api(path, data=None, method=None):
    request = urllib.request.Request(API + path, headers=HEADERS,
        data=None if data is None else json.dumps(data).encode(), method=method)
    with urllib.request.urlopen(request, timeout=40) as response:
        return json.load(response)


jobs = api('/actions/runs/' + RUN + '/jobs')['jobs']
assert any(j['name'] == 'verify (ubuntu-latest)' and j['conclusion'] == 'success' for j in jobs)
renderer = json.loads((ROOT / 'renderer-settings-audit.json').read_text())
pixels = json.loads((ROOT / 'renderer-settings-image-audit.json').read_text())
assert renderer['source'] == SOURCE and renderer['status'] == pixels['status'] == 'passed'
paths = sorted(ROOT.glob('worldcraft-render-*.png')) + sorted(ROOT.glob('worldcraft-biome-*.png')) + sorted(ROOT.glob('worldcraft-nature-*.png'))
assert len(paths) == 27
entries = []
images = []
for path in paths:
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n' and 12000 < len(data) < 8 * 1024 * 1024
    width, height = struct.unpack('>II', data[16:24])
    assert (width, height) == (960, 640)
    blob = api('/git/blobs', {'encoding': 'base64', 'content': base64.b64encode(data).decode()}, 'POST')
    relative = 'docs/media/render/' + SOURCE + '/' + path.name
    entries.append({'path': relative, 'mode': '100644', 'type': 'blob', 'sha': blob['sha']})
    images.append({'path': relative, 'width': width, 'height': height,
        'sha256': hashlib.sha256(data).hexdigest(), 'blob': blob['sha']})
report = {'source': SOURCE, 'run': RUN, 'status': 'native_linux_job_passed',
    'scope': 'Unedited software Vulkan captures; unsupported vendor dispatches are skips, not gaming GPU certification',
    'visual_review': 'pending', 'renderer': renderer, 'pixels': pixels,
    'nature': json.loads((ROOT / 'nature-render-audit.json').read_text()),
    'biomes': json.loads((ROOT / 'biome-render-audit.json').read_text()), 'images': images}
entries.append({'path': 'docs/evidence/render-' + SOURCE + '.json', 'mode': '100644',
    'type': 'blob', 'content': json.dumps(report, indent=2) + '\n'})
gallery = '# Recent native WorldCraft screenshots\n\n'
gallery += 'Source: ' + SOURCE + '. [Passing Linux run](https://github.com/' + os.environ['GITHUB_REPOSITORY'] + '/actions/runs/' + RUN + ').\n\n'
gallery += 'Unedited 960 × 640 software Vulkan captures. The subsequent API-selection compatibility patch does not change the Vulkan shaders. These images are real application output; they do not establish AAA quality or vendor GPU execution. The 80% realism target remains unmet.\n\n'
for name, label in [('worldcraft-render-taa.png', 'Native TAA woodland'), ('worldcraft-biome-savanna.png', 'Untouched savanna'), ('worldcraft-biome-alpine.png', 'Untouched alpine'), ('worldcraft-render-lens.png', 'Native depth-of-field case')]:
    gallery += '## ' + label + '\n\n![' + label + '](media/render/' + SOURCE + '/' + name + ')\n\n'
entries.append({'path': 'docs/RENDER_GALLERY.md', 'mode': '100644', 'type': 'blob', 'content': gallery})
head = api('/git/ref/heads/' + BRANCH)['object']['sha']
old = api('/git/commits/' + head)
tree = api('/git/trees', {'base_tree': old['tree']['sha'], 'tree': entries}, 'POST')
commit = api('/git/commits', {'message': 'test: retain unedited renderer captures ' + SOURCE[:12],
    'tree': tree['sha'], 'parents': [head]}, 'POST')
api('/git/refs/heads/' + BRANCH, {'sha': commit['sha'], 'force': False}, 'PATCH')
print('Retained', len(images), 'original native screenshots for', SOURCE)
