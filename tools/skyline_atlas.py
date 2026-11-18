"""Bake five original, alpha-tested tree profiles for the distant canopy."""
import math
import random

import numpy as np
from PIL import Image, ImageDraw

TILE = 128
SIDE = TILE * 4


def profile(family, variant):
    rng = random.Random(714015 + family * 83 + variant)
    n = TILE * 3
    im = Image.new("RGBA", (n, n))
    draw = ImageDraw.Draw(im)

    def point(x, y):
        return (round(x * n), round((1 - y) * n))

    def limb(a, b, width, color):
        draw.line((point(*a), point(*b)), fill=(*color, 255), width=max(1, round(width * n)))

    greens = [(73, 99, 44), (54, 80, 51), (91, 114, 50), (88, 100, 49), (83, 111, 56)]
    bark = (138, 134, 109) if family == 2 else (70, 59, 42)
    base = 0.49 + rng.uniform(-0.035, 0.035)
    bend = rng.uniform(-0.065, 0.065)

    def trunk(y):
        return base + bend * y * y

    def spray(x, y, w, h, light):
        # Each silhouette has open space between the smaller sprays. Bright tips
        # and a shaded interior survive the last few texture mips.
        for _ in range(19):
            a = rng.uniform(0, math.tau)
            radius = math.sqrt(rng.random())
            px, py = x + math.cos(a) * radius * w, y + math.sin(a) * radius * h
            rx = w * rng.uniform(0.12, 0.30)
            ry = h * rng.uniform(0.18, 0.41)
            pigment = light * rng.uniform(0.83, 1.13) * (1 + (py - y) / h * 0.11)
            color = tuple(min(255, round(c * pigment)) for c in greens[family])
            draw.ellipse((point(px - rx, py + ry), point(px + rx, py - ry)), fill=(*color, 255))

    for k in range(9):
        y = 0.03 + 0.086 * k
        limb((trunk(y), y), (trunk(y + 0.086), y + 0.086), 0.025 * (1 - y), bark)
    if family == 1:
        for i in range(16):
            y = 0.23 + i * 0.046
            w = (1.03 - y) ** 0.73 * 0.46
            for side in [-1, 1]:
                reach = w * rng.uniform(0.7, 1)
                for j in range(5):
                    t = (j + 0.4) / 5
                    x = trunk(y) + side * reach * t
                    py = y + 0.032 * (1 - t) - 0.055 * t + 0.022 * t * t
                    spray(x, py, 0.022 + 0.072 * (1 - y), 0.020 + 0.019 * (1 - t),
                          0.75 + 0.25 * t + 0.12 * y)
    elif family == 4:
        for i in range(15):
            side = (-1, 1)[i % 2]
            reach = rng.uniform(0.14, 0.40)
            rise = rng.uniform(0.09, 0.19)
            fall = rng.uniform(0.16, 0.33)
            prev = (trunk(0.77), 0.77)
            for j in range(1, 23):
                t = j / 22
                at = (trunk(0.77) + side * reach * t, 0.77 + rise * 4 * t * (1-t) - fall*t*t)
                limb(prev, at, 0.006, greens[family])
                limb(at, (at[0] + side * 0.022, at[1] - 0.065 * math.sin(t * math.pi)), 0.008,
                     tuple(round(c * rng.uniform(0.87, 1.12)) for c in greens[family]))
                prev = at
    else:
        # Project successive branching whorls in depth order so twigs appear
        # both in front of and behind sunlit clumps.
        branches = []
        for k in range(29):
            az = k * 2.399963 + variant
            level = k / 29
            attach = 0.23 + 0.51 * level
            if family == 3:
                attach = 0.27 + level * 0.21
                end_y = 0.76 + level * 0.11 + rng.uniform(-0.07, 0.03)
                reach = 0.41 * (1 - 0.56 * level)
            else:
                end_y = 0.43 + 0.45 * level + rng.uniform(-0.045, 0.04)
                reach = (0.36 if family == 2 else 0.42) * (1 - 0.68 * level**2)
            x = trunk(attach) + math.cos(az) * reach * rng.uniform(0.83, 1.0)
            branches.append((math.sin(az), x, end_y, attach))
        for depth, x, y, root in sorted(branches):
            mid = (trunk(root) * 0.57 + x * 0.43, root * 0.42 + y * 0.58)
            limb((trunk(root), root), mid, 0.013, bark)
            limb(mid, (x, y), 0.007, bark)
            for j in range(4):
                cx, cy = x + rng.uniform(-0.034, 0.034), y + rng.uniform(-0.027, 0.027)
                spray(cx, cy, rng.uniform(0.043, 0.077), rng.uniform(0.029, 0.055),
                      0.91 + depth * 0.14 + (cy - 0.5) * 0.29)
    return im.resize((TILE, TILE), Image.Resampling.LANCZOS)


def bake(path):
    atlas = Image.new("RGBA", (SIDE, SIDE))
    for family in range(5):
        for variant in range(3):
            tile = family * 3 + variant
            atlas.paste(profile(family, variant), (tile % 4 * TILE, tile // 4 * TILE))
    # Filter each tile separately so a conifer never acquires a neighboring
    # species at the coarsest mip. Pillow filters RGBA in premultiplied space.
    levels = []
    for mip in range(8):
        side = TILE >> mip
        out = Image.new("RGBA", (side * 4, side * 4))
        for t in range(15):
            x, y = t % 4, t // 4
            src = atlas.crop((x*TILE, y*TILE, (x+1)*TILE, (y+1)*TILE))
            px = np.asarray(src.resize((side, side), Image.Resampling.BOX)).copy()
            if mip and side > 1:
                a = px[:, :, 3].astype(float)
                target = (np.asarray(src)[:, :, 3] >= 128).mean()
                cut = np.quantile(a, 1 - target)
                if cut > 0:
                    px[:, :, 3] = np.rint(np.clip(a * min(3.0, 128 / cut), 0, 255)).astype(np.uint8)
            out.paste(Image.fromarray(px), (x*side, y*side))
        levels.append(out.tobytes())
    path.write_bytes(b"".join(levels))
