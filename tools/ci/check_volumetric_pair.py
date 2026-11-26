"""Check matched native captures and guard against a visually inert mist toggle."""
import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image


def measure(on, off, label):
    def capture(folder):
        report = json.loads((folder / "biome-render-audit.json").read_text())
        row = next(s for s in report["sites"] if s["label"] == label)
        pixels = np.asarray(Image.open(folder / row["image"]).convert("RGB"), dtype=np.int16)
        return report["source"], row, pixels

    source, lit, a = capture(on)
    other, clear, b = capture(off)
    assert source == other and lit["volumetrics"] and not clear["volumetrics"]
    for key in ("seed", "camera", "hour", "pitch", "yaw", "surface_texture_side", "resident_chunks"):
        assert lit[key] == clear[key], key
    assert a.shape == b.shape and lit["queued_jobs"] == clear["queued_jobs"] == 0
    diff = np.abs(a-b).mean(axis=2)
    h = diff.shape[0]
    sky_ridge = float(diff[:h//2].mean())
    foreground = float(diff[h*4//5:].mean())
    changed = float((diff > 12).mean())
    if label == "estuary":
        assert sky_ridge > 14 and sky_ridge > foreground*1.3 and changed > .20, (sky_ridge, foreground, changed)
    return dict(source=source, site=label, seed=lit["seed"], hour=lit["hour"], camera=lit["camera"],
                texture_side=lit["surface_texture_side"], mean_rgb_delta=float(diff.mean()),
                upper_half_delta=sky_ridge, foreground_delta=foreground, fraction_over_12=changed)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("on",type=Path)
    parser.add_argument("off",type=Path)
    parser.add_argument("--label",default="estuary")
    args=parser.parse_args()
    print(json.dumps(measure(args.on,args.off,args.label),indent=2))
