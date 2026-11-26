"""Periodic beach aggregate. RGB stores unit normals, alpha stores diffuse pigment."""
import numpy as np


def bake(path, side=1024):
    rng = np.random.default_rng(71751)
    sand = rng.random((side, side), dtype=np.float32)
    height = sand * 0.24
    pigment = 0.48 + (sand - 0.5) * 0.24
    for _ in range(3400):
        x, y = rng.integers(side, size=2)
        radius = 1.3 + 8.0 * rng.random() ** 3
        width = radius * rng.uniform(0.35, 0.94)
        angle = rng.uniform(0, 2 * np.pi)
        r = int(np.ceil(radius))
        span = np.arange(-r, r + 1)
        dx, dy = np.meshgrid(span, span)
        u = (dx*np.cos(angle) + dy*np.sin(angle)) / radius
        v = (dy*np.cos(angle) - dx*np.sin(angle)) / width
        cap = np.maximum(0, 1 - u*u - v*v)
        rock = np.sqrt(cap)
        mask = np.minimum(1, cap*radius)
        where = np.ix_((y+span) % side, (x+span) % side)
        height[where] += rock * width * 0.54
        shell = rng.random() < 0.13
        color = 0.84 + 0.11*np.cos(18*u + 7*v*v) if shell else rng.uniform(0.12, 0.72)
        pigment[where] += mask * (color - pigment[where])
    nx = (np.roll(height, 1, 1) - np.roll(height, -1, 1)) * 0.5
    ny = (np.roll(height, 1, 0) - np.roll(height, -1, 0)) * 0.5
    normal = np.stack((nx, ny, np.ones_like(nx)), axis=2)
    with path.open('wb') as f:
        while True:
            normal /= np.linalg.norm(normal, axis=2, keepdims=True)
            packed = np.dstack((0.5 + normal*0.5, pigment))
            f.write(np.rint(packed.clip(0, 1)*255).astype(np.uint8).tobytes())
            n = len(pigment)//2
            if not n:
                break
            normal = normal.reshape(n, 2, n, 2, 3).mean(axis=(1, 3))
            pigment = pigment.reshape(n, 2, n, 2).mean(axis=(1, 3))
