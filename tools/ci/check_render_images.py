"""Check native output and reconstruction ownership, never assign a realism score."""
import hashlib
import json
from pathlib import Path
import numpy as np
from PIL import Image

report = json.loads(Path('renderer-settings-audit.json').read_text())
labels = ['fxaa', 'smaa', 'taa', 'cinematic', 'lens', 'scaled', 'dlss', 'dlaa', 'fsr']
assert report['status'] == 'passed'
assert [row['label'] for row in report['cases']] == labels
images = []
for row in report['cases']:
    path = Path('worldcraft-render-' + row['label'] + '.png')
    with Image.open(path) as original:
        assert original.width >= 320 and original.height >= 240
        pixels = np.array(original.convert('RGB'), dtype=np.float32)
        height, width = pixels.shape[:2]
        scene = pixels[height//5:height*4//5, width//12:width*11//12]
        assert float(scene.std()) > 3.0, 'Empty/uniform scene: ' + row['label']
        assert float(scene.max() - scene.min()) > 30.0, 'Insufficient scene range'
    assert row['queued_jobs'] == 0 and row['resident_chunks'] == 25
    assert row['fallback_aa_components'] == (1 if row['active'] == 'Native' else 0)
    images.append({'label': row['label'], 'width': width, 'height': height,
                   'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                   'scene_std': float(scene.std())})
assert images[0]['sha256'] != images[1]['sha256'], 'FXAA and SMAA captures are identical'
assert images[1]['sha256'] != images[2]['sha256'], 'SMAA and TAA captures are identical'
assert report['cases'][5]['output'][0] < report['cases'][0]['output'][0]
result = {'status': 'passed', 'scope': 'Native pixels, AA ownership and scaled output; no gaming-GPU or realism score',
          'images': images, 'vendor_skips': [r['label'] for r in report['cases'] if r['vendor_execution_skipped']]}
Path('renderer-settings-image-audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
