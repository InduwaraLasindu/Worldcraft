"""Reject exposed Eco boundaries and an opaque fade that hides the landscape.

These fixed-camera native images exercise the normal learned example from above
and at ground level. They are integration checks, not reference-quality scoring.
"""
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image


def load(path):
    pixels = np.asarray(Image.open(path).convert("RGB"), dtype=float)
    height, width, _ = pixels.shape
    assert width >= 640 and height >= 360, "Eco fixture is too small to inspect"
    return pixels @ np.array([0.2126, 0.7152, 0.0722])


def main():
    aerial_path = Path("worldcraft-eco.png")
    ground_path = Path("worldcraft-eco-ground.png")
    aerial, ground = load(aerial_path), load(ground_path)
    height, width = aerial.shape
    # Exclude the toolbar/status UI. A flat sea/sky boundary in the old fixture
    # produced a 45-level row jump; radial haze erased the foreground to std < 2.
    rows = aerial[int(height * .25):int(height * .65),
                  int(width * .1):int(width * .9)].mean(axis=1)
    foreground = aerial[int(height * .70):int(height * .94),
                        int(width * .1):int(width * .9)]
    height, width = ground.shape
    scene = ground[int(height * .18):int(height * .94),
                   int(width * .1):int(width * .9)]
    row_step = float(np.abs(np.diff(rows)).max())
    foreground_std = float(foreground.std())
    ground_range = float(np.quantile(scene, .95) - np.quantile(scene, .05))
    assert row_step < 8, f"Eco exposes a hard horizon ({row_step:.2f})"
    assert foreground_std >= 4, f"Eco haze erased the aerial foreground ({foreground_std:.2f})"
    assert ground_range >= 45, f"Eco haze erased nearby trees ({ground_range:.2f})"
    report = {
        "status": "passed",
        "scope": "Fixed native Eco cameras; not gaming FPS or realism certification",
        "aerial_max_row_luma_step": row_step,
        "aerial_foreground_luma_std": foreground_std,
        "ground_scene_luma_range": ground_range,
        "images": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                   for p in [aerial_path, ground_path]},
    }
    Path("eco-horizon-audit.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
