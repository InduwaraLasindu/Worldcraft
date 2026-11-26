"""Bake a periodic weather/billow field into a mipmapped 64³ RG8 texture."""
from itertools import product

import numpy as np

SIDE = 64


def value_noise(grid, cells, rng):
    lattice = rng.random((cells,) * 3, dtype=np.float32)
    q = grid * cells
    cell = np.floor(q).astype(np.int32)
    f = q - cell
    w = f * f * (3 - 2 * f)
    values = np.zeros(grid.shape[1:], dtype=np.float32)
    for corner in product((0, 1), repeat=3):
        offset = np.array(corner).reshape(3, 1, 1, 1)
        weight = np.where(offset, w, 1 - w).prod(axis=0)
        values += weight * lattice[tuple((cell + offset) % cells)]
    return values


def billows(grid, cells, rng):
    points = 0.12 + 0.76 * rng.random((cells, cells, cells, 3), dtype=np.float32)
    q = grid * cells
    cell = np.floor(q).astype(np.int32)
    f = q - cell
    distance_sq = np.full(grid.shape[1:], np.inf, dtype=np.float32)
    for neighbor in product((-1, 0, 1), repeat=3):
        offset = np.array(neighbor).reshape(3, 1, 1, 1)
        feature = points[tuple((cell + offset) % cells)].transpose(3, 0, 1, 2)
        delta = offset + feature - f
        np.minimum(distance_sq, (delta * delta).sum(axis=0), out=distance_sq)
    return np.clip(1 - np.sqrt(distance_sq), 0, 1)


def bake(path):
    rng = np.random.default_rng(0xC10D)
    grid = (np.indices((SIDE,) * 3, dtype=np.float32) + 0.5) / SIDE
    weather = sum(weight * value_noise(grid, cells, rng)
                  for cells, weight in [(4, 0.57), (8, 0.29), (16, 0.14)])
    # Sheared cell boundaries give banks a windward face and smaller
    # turbulent scallops. Bake these octaves into the existing density channel.
    flow = value_noise(grid, 4, rng) - 0.5
    warped = (grid + np.stack((flow * 0.049, -flow * 0.032, flow * 0.078))) % 1.0
    puffs = (0.59 * billows(warped, 6, rng) + 0.30 * billows(warped, 13, rng)
             + 0.11 * billows(warped, 24, rng))
    level = np.rint(255 * np.stack((weather, puffs), axis=-1)).astype(np.uint8)
    with path.open("wb") as out:
        while True:
            out.write(level.tobytes())
            n = len(level) // 2
            if not n:
                break
            boxes = level.reshape(n, 2, n, 2, n, 2, 2)
            level = ((boxes.sum(axis=(1, 3, 5), dtype=np.uint16) + 4) // 8).astype(np.uint8)
