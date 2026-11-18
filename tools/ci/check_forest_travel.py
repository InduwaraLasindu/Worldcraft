"""Check the native moving-camera route and its original capture files."""
import json
from pathlib import Path

import numpy as np
from PIL import Image

report = json.loads(Path("forest-travel-audit.json").read_text())
assert report["step_m"] == 2 and report["authored_instances"] == 0
assert report["peak_resident_chunks"] == 25 and report["peak_jobs"] <= 4
assert report["crown_switch_m"] == 128 and report["tree_cutoff_m"] == 224
assert [r["distance_m"] for r in report["captures"]] == [92, 126, 130, 160, 192, 218, 226, 248]
times = np.asarray(report["moving_frames"]["intervals_ms"])
assert len(times) == report["moving_frames"]["samples"] == 78
assert np.isfinite(times).all() and (times > 0).all()
for p in (50, 95, 99):
    assert report["moving_frames"][f"p{p}_ms"] == sorted(times)[int(np.ceil(p * len(times) / 100)) - 1]
previous = None
for row in report["captures"]:
    assert row["resident_chunks"] == 25 and row["queued_jobs"] == 0
    assert abs(np.linalg.norm(np.subtract(row["eye"], report["tree"])) - row["distance_m"]) < 0.001
    with Image.open(row["image"]) as image:
        assert image.size == (960, 640)
        pixels = np.asarray(image.convert("L"), dtype=np.float32)
    assert pixels.std() > 10, "Empty or flat native travel frame"
    if previous is not None:
        assert np.abs(previous - pixels).mean() > 0.5, "Duplicate or frozen travel capture"
    previous = pixels
print("Eight original frames span both tree transitions within the streaming budgets.")
