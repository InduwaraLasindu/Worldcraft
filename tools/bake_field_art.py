"""Rebuild Worldcraft's original photographic-style texture pack from checked-in sheets."""
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter, ImageOps

from ultra_materials import write_map

ART = Path(__file__).resolve().parents[1] / "crates/worldcraft/art"
SIDE = 512


def tile(sheet, column, row, columns):
    w, h = sheet.size
    return sheet.crop((
        round(column * w / columns) + 3, round(row * h / 2) + 3,
        round((column + 1) * w / columns) - 3, round((row + 1) * h / 2) - 3,
    ))


def surface(image, strength, rough):
    # Put the photograph's interior across the repeat seam with a soft skirt.
    rgb = np.asarray(image.resize((SIDE, SIDE), Image.Resampling.LANCZOS), dtype=np.float32) / 255
    axis = np.arange(SIDE) + 0.5
    border = np.minimum(axis, SIDE - axis)
    blend = 1 - np.clip((border - 2) / 62, 0, 1)
    blend = blend * blend * (3 - 2 * blend)
    rgb = rgb * (1 - blend[None, :, None]) + np.roll(rgb, SIDE // 2, axis=1) * blend[None, :, None]
    rgb = rgb * (1 - blend[:, None, None]) + np.roll(rgb, SIDE // 2, axis=0) * blend[:, None, None]
    luma = np.dot(rgb, [0.25, 0.64, 0.11])
    base = Image.fromarray(np.rint(luma * 255).astype(np.uint8))
    blurred = np.asarray(base.filter(ImageFilter.GaussianBlur(10.5)), dtype=np.float32) / 255
    breakup = np.asarray(base.filter(ImageFilter.GaussianBlur(1.3)), dtype=np.float32) / 255
    height = 0.5 + np.clip((breakup - blurred) * 2.8 + (luma - breakup) * 0.46, -0.49, 0.49)
    du = np.roll(height, 1, axis=1) - np.roll(height, -1, axis=1)
    dv = np.roll(height, 1, axis=0) - np.roll(height, -1, axis=0)
    gain = strength / max(0.012, np.sqrt(np.mean(du * du + dv * dv)))
    n = np.stack([du * gain, dv * gain, np.ones_like(height)], axis=-1)
    n /= np.linalg.norm(n, axis=-1, keepdims=True)
    roughness = np.clip(rough + (height - 0.5) * 0.17 + (luma - 0.5) * 0.08, 0.35, 0.98)

    def rgba(channels):
        return Image.fromarray(np.rint(np.clip(channels, 0, 1) * 255).astype(np.uint8))

    return (
        rgba(np.dstack([rgb, np.ones_like(height)])),
        rgba(np.dstack([n[..., :2] * 0.5 + 0.5, height, roughness])),
        rgba(np.dstack([n * 0.5 + 0.5, np.ones_like(height)])),
    )


def cutout(image):
    rgb = np.asarray(image, dtype=np.float32) / 255
    r, g, b = rgb.transpose(2, 0, 1)
    magenta = np.minimum(r, b) - g
    alpha = np.clip((0.35 - magenta) / 0.27, 0, 1)
    # Despill magenta in antialiased edges while keeping ochre stems intact.
    b = np.minimum(b, np.maximum(g * 0.94, r * 0.70))
    r = np.where(alpha < 0.98, np.minimum(r, np.maximum(g * 1.47, b * 1.35)), r)
    result = Image.fromarray(np.rint(np.dstack([r, g, b, alpha]) * 255).astype(np.uint8))
    return result.crop(result.getchannel("A").point(lambda a: int(a > 150) * 255).getbbox())


def sprite(image, size, width, height):
    scale = min(width / image.width, height / image.height)
    image = image.resize((round(image.width * scale), round(image.height * scale)), Image.Resampling.LANCZOS)
    out = Image.new("RGBA", size)
    out.paste(image, ((size[0] - image.width) // 2, size[1] - image.height - 3))
    return out


def grass_atlas(tuft):
    atlas = Image.new("RGBA", (SIDE, SIDE))
    for habitat in range(4):
        blades = ImageOps.mirror(tuft) if habitat & 1 else tuft
        blades = sprite(blades, (196, 256), 187, [245, 241, 224, 250][habitat])
        a = np.asarray(blades).copy()
        # The procedural habitat tints provide green/straw hues at run time.
        value = np.dot(a[..., :3].astype(np.float32), [0.21, 0.72, 0.07]) / 255
        a[..., :3] = np.clip(174 + 102 * value[..., None], 0, 255).astype(np.uint8)
        a[..., 3] = np.asarray(blades.getchannel("A").filter(ImageFilter.MaxFilter(3)))
        blades = Image.fromarray(a).transpose(Image.Transpose.FLIP_TOP_BOTTOM)
        atlas.paste(blades, ((habitat & 1) * 196, (habitat >> 1) * 256))
    atlas.paste((255, 255, 255, 255), (410, 0, SIDE, SIDE))
    return atlas


def main():
    terrain = Image.open(ART / "source/terrain.png").convert("RGB")
    species = Image.open(ART / "source/vegetation.png").convert("RGB")
    woods = Image.open(ART / "source/bark-and-fronds.png").convert("RGB")
    families = [
        ("litter", 0.43, 0.87), ("granite", 0.45, 0.78),
        ("meadow", 0.30, 0.87), ("sand", 0.18, 0.91),
        ("snow", 0.21, 0.92), ("gravel", 0.44, 0.84),
        ("basalt", 0.37, 0.87), ("soil", 0.34, 0.81),
    ]
    for i, (name, strength, rough) in enumerate(families):
        color, data, _ = surface(tile(terrain, i % 4, i // 4, 4), strength, rough)
        write_map(ART / f"{name}-color.wct", color)
        write_map(ART / f"{name}-surface.wct", data)

    for name, sheet, x, y in [
        ("oak-spray", species, 0, 0), ("spruce-spray", species, 1, 0),
        ("birch-spray", species, 1, 1), ("palm-frond", woods, 0, 1),
        ("acacia-spray", woods, 1, 1),
    ]:
        keyed = cutout(tile(sheet, x, y, 2))
        if name in {"oak-spray", "spruce-spray", "birch-spray"}:
            keyed = keyed.rotate(90, expand=True)
        texture = sprite(keyed, (SIDE, SIDE), 492, 482)
        write_map(ART / f"{name}.wct", texture, cutout=True)
    write_map(ART / "grass-tufts.wct", grass_atlas(cutout(tile(species, 0, 1, 2))))

    for column, name in enumerate(["oak-bark", "birch-bark"]):
        color, _, normal = surface(tile(woods, column, 0, 2), 0.45, 0.90)
        write_map(ART / f"{name}.wct", color)
        write_map(ART / f"{name}-normal.wct", normal)


if __name__ == "__main__":
    main()
