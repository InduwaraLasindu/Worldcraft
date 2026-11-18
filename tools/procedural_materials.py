"""Original periodic ground, stone and turf. Inputs are a seed and a resolution."""
import numpy as np


def noise(side, cells, rng):
    """Sample smooth value noise on a torus, including filtered tile borders."""
    lattice = rng.uniform(-1, 1, (cells, cells)).astype(np.float32)
    t = (np.arange(side, dtype=np.float32) + 0.5) * cells / side
    i = t.astype(int)
    j = (i + 1) % cells
    f = t - i
    w = f * f * (3 - 2 * f)
    lower = (1 - w) * lattice[:, i] + w * lattice[:, j]
    return (1 - w[:, None]) * lower[i] + w[:, None] * lower[j]


def paint(color, height, rng, count, shape, size_range, aspect_range, pigment):
    """Rasterize curved grass, veined leaf litter or rounded gravel across seams."""
    side = len(height)
    for _ in range(count):
        cx, cy = rng.uniform(0, side, 2)
        radius = rng.uniform(*size_range) * (side / 512)
        width = radius * rng.uniform(*aspect_range)
        angle = rng.uniform(0, 2 * np.pi)
        dx, dy = np.cos(angle), np.sin(angle)
        curvature = rng.uniform(-0.7, 0.7)
        extent = int(np.ceil(radius * 1.35 + width + 1))
        x = np.arange(int(cx) - extent, int(cx) + extent + 1)
        y = np.arange(int(cy) - extent, int(cy) + extent + 1)
        xx, yy = x[None, :] + 0.5 - cx, y[:, None] + 0.5 - cy
        along = (xx * dx + yy * dy) / radius
        across = (-xx * dy + yy * dx) / width - curvature * along * along
        if shape == "gravel":
            relief = np.maximum(0, 1 - along * along - across * across)
            profile = np.sqrt(relief)
        else:
            profile = np.maximum(0, 1 - along * along)
            if shape == "leaf":
                profile *= 0.92 + 0.08 * np.cos(along * 24)
            relief = np.maximum(0, profile - abs(across))
        mask = np.clip(relief * width * 1.9, 0, 1)
        where = np.ix_(y % side, x % side)
        dye = np.asarray(pigment) * rng.uniform(0.62, 1.4)
        if shape == "leaf":
            vein = np.maximum(0, 1 - abs(across) * 11) * profile
            shade = 0.72 + 0.27 * relief + 0.17 * vein
        else:
            shade = 0.73 + 0.38 * relief
        color[where] += mask[..., None] * (dye * shade[..., None] - color[where])
        height[where] += mask * (relief * min(width, 3.0) + 0.4)


def stone(side, rng, grain, relief):
    y, x = (np.indices((side, side), dtype=np.float32) + 0.5) / side
    warp = noise(side, 5, rng)
    # First and second nearest sites give branching boundaries. The lattice
    # and contact distances both wrap, so fractures continue across tile edges.
    cells = 6
    u, v = x * cells + warp * 0.52, y * cells + noise(side, 5, rng) * 0.44
    iu, iv = np.floor(u).astype(int), np.floor(v).astype(int)
    centers = rng.uniform(0.14, 0.86, (cells, cells, 2))
    first = np.full_like(u, np.inf)
    second = first.copy()
    for oz in (-1, 0, 1):
        for ox in (-1, 0, 1):
            ax, az = iu + ox, iv + oz
            jitter = centers[az % cells, ax % cells]
            d = np.hypot(ax + jitter[..., 0] - u, az + jitter[..., 1] - v)
            second = np.minimum(second, np.maximum(first, d))
            first = np.minimum(first, d)
    crack = np.exp(-np.square((second - first) / 0.036))
    bands = np.cos((x + 2 * y + warp * 0.10) * 2 * np.pi * 4)
    mineral = np.maximum(0, 1 - abs(bands) * 3.5) ** 2
    light = 0.94 + 0.22 * relief + 0.28 * grain + 0.15 * bands - 0.61 * crack
    color = light[..., None] * np.array([0.21, 0.195, 0.17]) + mineral[..., None] * [0.062, 0.055, 0.046]
    height = 4.0 * relief + 1.7 * grain + bands * 1.5 - crack * 3.4 + mineral * 1.4
    return color, height, np.clip(0.81 + 0.09 * grain + 0.12 * crack - 0.13 * mineral, 0.45, 1)


def layers(side):
    """Yield linear albedo, OpenGL normal and perceptual roughness for each layer."""
    for layer in ("forest", "rock", "turf"):
        rng = np.random.Generator(np.random.PCG64({"forest": 4139, "rock": 6203, "turf": 20261}[layer]))
        relief = noise(side, 8, rng) + 0.48 * noise(side, 32, rng)
        grain = 0.63 * noise(side, 117, rng) + 0.37 * rng.uniform(-1, 1, (side, side))
        if layer == "rock":
            color, height, rough = stone(side, rng, grain, relief)
        else:
            forest = layer == "forest"
            base = np.array([0.108, 0.075, 0.044] if forest else [0.16, 0.195, 0.088])
            color = (1.0 + 0.35 * relief + 0.46 * grain)[..., None] * base
            height = relief * 2.2 + grain * 1.5
            paint(color, height, rng, 650, "gravel", (1.1, 3.8), (0.48, 0.94), [0.16, 0.143, 0.108])
            if forest:
                paint(color, height, rng, 560, "leaf", (3.7, 10), (0.22, 0.53), [0.235, 0.13, 0.054])
                paint(color, height, rng, 510, "blade", (4, 14), (0.045, 0.11), [0.19, 0.135, 0.074])
            else:
                paint(color, height, rng, 2300, "blade", (3.6, 12), (0.075, 0.21), [0.138, 0.214, 0.076])
                paint(color, height, rng, 1000, "blade", (3, 9), (0.055, 0.16), [0.28, 0.252, 0.12])
            rough = np.clip(0.90 + 0.045 * grain - 0.017 * height, 0.66, 0.99)
        dx = 0.5 * (np.roll(height, -1, 1) - np.roll(height, 1, 1))
        dy = 0.5 * (np.roll(height, -1, 0) - np.roll(height, 1, 0))
        normal = np.stack((-dx, dy, np.ones_like(height)), axis=2)
        yield color.astype(np.float32), normal.astype(np.float32), rough[..., None].astype(np.float32)
