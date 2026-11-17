"""Validate procedural-route evidence. These checks are not a realism score."""
import json
from pathlib import Path
import numpy as np
from PIL import Image

report = json.loads(Path('biome-render-audit.json').read_text())
assert report['status'] == 'passed' and report['generator_version'] == 3
assert report['authored_instances'] == 0
expected = {'woodland', 'birch', 'savanna', 'tropical', 'alpine', 'meadow'}
assert {row['label'] for row in report['sites']} == expected
assert len({row['seed'] for row in report['sites']}) >= 5
for row in report['sites']:
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
