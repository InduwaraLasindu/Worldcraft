"""Validate procedural-route evidence. These checks are not a realism score."""
import json
from pathlib import Path
import numpy as np
from PIL import Image

report = json.loads(Path('biome-render-audit.json').read_text())
assert report['status'] == 'passed' and report['generator_version'] == 3
assert report['authored_instances'] == 0
expected = {'woodland', 'birch', 'savanna', 'tropical', 'alpine', 'meadow', 'estuary'}
assert {row['label'] for row in report['sites']} == expected
assert len({row['seed'] for row in report['sites']}) >= 5
for row in report['sites']:
    assert row['procedural_surfaces'] is True
    if row['label'] == 'estuary':
        lo, hi = row['estuary_depth_range_m']
        assert lo < 0 < 1 < hi
    assert row['queued_jobs'] == 0 and row['resident_chunks'] == 25
    assert sum(row['center_species_counts']) <= 4352
    assert row['cloud_draw_commands'] >= 5 and row['far_triangles'] > 0
    with Image.open(row['image']) as image:
        assert image.width >= 640 and image.height >= 360
        pixels = np.asarray(image.convert('RGB'), dtype=np.float32)
    luma = pixels @ np.array([.2126, .7152, .0722])
    assert float(luma.std()) > 10, f"{row['label']}: empty or flat native view"
    assert float(np.percentile(luma, 95)-np.percentile(luma, 5)) > 30
    print(f"{row['label']}: native image, settled residency, bounded population and cloud commands verified")

# Switch the live material setting at one fixed viewpoint. The sky, plant
# placements and lighting stay fixed for both captures.
with Image.open('worldcraft-biome-savanna.png') as original, Image.open('worldcraft-material-savanna-scanned.png') as scanned:
    before = np.asarray(original.convert('RGB'), dtype=np.int16)
    after = np.asarray(scanned.convert('RGB'), dtype=np.int16)
assert before.shape == after.shape
delta = np.abs(before - after).max(axis=2)
ground = delta[delta.shape[0] * 2 // 3:]
assert (ground > 4).mean() > 0.08, "Material control did not change the visible ground"
print(f"Switching surface sets changed {(ground > 4).mean():.1%} of the lower third at the fixed savanna camera")
