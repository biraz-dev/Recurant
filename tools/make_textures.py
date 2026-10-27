#!/usr/bin/env python3
"""Generate the environment textures in the palette of the environment reference.

    python3 tools/make_textures.py [--preview sheet.png]

Colour ramps are sampled from the texture-palette strip of
art/environment-reference.png; the patterns (block courses, moss, cracks,
carvings, diamond floor tiles, vines, ferns) are generated, so every tile is
seamless and sharp at the game's resolution. Output goes to
crates/zrpg/assets/env/, where the game embeds it.

Needs Pillow and NumPy. Deterministic: same reference, same textures.
"""
import argparse
import os

import numpy as np
from PIL import Image

REFERENCE = "art/environment-reference.png"
OUT_DIR = "crates/zrpg/assets/env"
SIZE = 64

# x-ranges of the swatches in the reference's "TEXTURE PALETTE" strip.
SWATCH_ROWS = (885, 965)
SWATCHES = {
    "sandstone": (14, 91),
    "stone": (97, 169),
    "cracked": (175, 247),
    "mossy": (253, 324),
    "dirt": (331, 403),
    "plants": (413, 481),
    "vines": (488, 560),
    "carved": (650, 725),
    "floor": (897, 971),
}


# --- palette ----------------------------------------------------------------

def sample_ramps(path):
    """Five tones per swatch, dark to light, plus a deeper shadow and a
    brighter highlight so bevels can catch the light."""
    im = np.asarray(Image.open(path).convert("RGB")).astype(float)
    y0, y1 = SWATCH_ROWS
    ramps = {}
    for name, (x0, x1) in SWATCHES.items():
        px = im[y0:y1, x0 + 3 : x1 - 3].reshape(-1, 3)
        if name == "vines":
            px = px[px.max(axis=1) > 50]  # skip the dark gaps between strands
        order = np.argsort(px @ [0.299, 0.587, 0.114])
        tones = []
        for q in (0.05, 0.25, 0.5, 0.75, 0.95):
            i = int(q * (len(order) - 1))
            k = max(1, len(order) // 40)
            tones.append(px[order[max(0, i - k) : i + k + 1]].mean(axis=0))
        tones = [tones[0] * 0.62] + tones + [np.minimum(tones[-1] * 1.22, 255)]
        ramps[name] = np.array(tones)
    return ramps


def shade(ramp, t):
    """Map t in [0, 1] (array) onto a colour ramp."""
    t = np.clip(t, 0.0, 1.0) * (len(ramp) - 1)
    i = np.minimum(t.astype(int), len(ramp) - 2)
    f = (t - i)[..., None]
    return ramp[i] * (1 - f) + ramp[i + 1] * f


# --- noise and layout --------------------------------------------------------

def tile_noise(rng, cells, octaves=3, size=SIZE):
    """Seamless value noise in [0, 1]."""
    total = np.zeros((size, size))
    amp, norm = 1.0, 0.0
    for o in range(octaves):
        c = cells * 2**o
        grid = rng.random((c, c))
        pos = np.arange(size) * c / size
        i0 = pos.astype(int)
        f = pos - i0
        f = f * f * (3 - 2 * f)
        i1 = (i0 + 1) % c
        top = grid[i0][:, i0] * (1 - f) + grid[i0][:, i1] * f
        bot = grid[i1][:, i0] * (1 - f) + grid[i1][:, i1] * f
        total += (top * (1 - f)[:, None] + bot * f[:, None]) * amp
        norm += amp
        amp *= 0.5
    return total / norm


def courses(rng, n_courses, min_w, max_w, size=SIZE):
    """Staggered block courses that tile horizontally. Returns per-pixel
    block ids and masks for mortar, lit edges (top/left) and shaded edges
    (bottom/right)."""
    h = size // n_courses
    ids = np.zeros((size, size), int)
    mortar = np.zeros((size, size), bool)
    lit = np.zeros((size, size), bool)
    dark = np.zeros((size, size), bool)
    next_id = 0
    for c in range(n_courses):
        widths = []
        while sum(widths) < size:
            widths.append(int(rng.integers(min_w, max_w + 1)))
        widths[-1] -= sum(widths) - size
        if widths[-1] < min_w * 0.6 and len(widths) > 1:
            widths[-2] += widths.pop()
        offset = int(rng.integers(0, size))
        x = offset
        for w in widths:
            cols = (np.arange(x, x + w)) % size
            rows = slice(c * h, (c + 1) * h)
            ids[rows, cols] = next_id
            mortar[c * h + h - 1, cols] = True
            mortar[rows, cols[0]] = True
            lit[c * h, cols] = True
            lit[rows, cols[1 % w]] = True
            dark[c * h + h - 2, cols] = True
            dark[rows, cols[-1]] = True
            next_id += 1
            x += w
    lit &= ~mortar
    dark &= ~mortar & ~lit
    return ids, mortar, lit, dark


def chunky_noise(rng, cluster=2, size=SIZE):
    """Coarse grain: random values in 2x2-texel clusters, like the
    reference's sandstone."""
    n = size // cluster
    return np.kron(rng.random((n, n)), np.ones((cluster, cluster)))


def rgb(img):
    return np.clip(img, 0, 255).astype(np.uint8)


# --- textures -----------------------------------------------------------------

def sandstone_wall(r, seed, n_courses=4):
    rng = np.random.default_rng(seed)
    ids, mortar, lit, dark = courses(rng, n_courses, 24, 42)
    tone = rng.normal(0.5, 0.13, ids.max() + 1)[ids]
    t = (
        tone
        + (tile_noise(rng, 16, 2) - 0.5) * 0.3
        + (tile_noise(rng, 2, 2) - 0.5) * 0.25
        + (chunky_noise(rng) - 0.5) * 0.22
    )
    t = np.where(lit, t + 0.25, t)
    t = np.where(dark, t - 0.3, t)
    pits = rng.random((SIZE, SIZE)) < 0.035
    t = np.where(pits, t - 0.3, t)
    img = shade(r["sandstone"], t)
    img[mortar] = shade(r["stone"], 0.05 + rng.random(mortar.sum()) * 0.15)
    return img, (ids, mortar, lit)


def mossy_wall(r, seed):
    img, (ids, mortar, lit) = sandstone_wall(r, seed)
    rng = np.random.default_rng(seed + 100)
    y = np.arange(SIZE)[:, None] / SIZE
    ledge = lit & (np.roll(mortar, 1, axis=0))  # top rows of blocks catch moss
    m = tile_noise(rng, 6, 3) + y * 0.2 + mortar * 0.1 + ledge * 0.25
    moss = shade(r["mossy"], tile_noise(rng, 16, 2) * 1.2 - 0.05)
    dither = (np.indices((SIZE, SIZE)).sum(axis=0) % 2) == 0
    full = m > 0.8
    edge = (m > 0.72) & ~full & dither
    img[full | edge] = moss[full | edge]
    return img


def cracked_wall(r, seed):
    rng = np.random.default_rng(seed)
    img, (ids, mortar, lit) = sandstone_wall(r, seed)
    # Some blocks have weathered to grey stone.
    grey = rng.random(ids.max() + 1) < 0.35
    t = tile_noise(rng, 16, 2) * 0.6 + 0.25 + lit * 0.2
    stone = shade(r["cracked"], t)
    sel = grey[ids] & ~mortar
    img[sel] = stone[sel]
    for _ in range(3):
        x, yy = float(rng.integers(0, SIZE)), float(rng.integers(0, SIZE // 2))
        for _ in range(int(rng.integers(18, 40))):
            xi, yi = int(x) % SIZE, int(yy)
            if yi >= SIZE:
                break
            img[yi, xi] = r["cracked"][0] * 0.55
            img[yi, (xi + 1) % SIZE] = img[yi, (xi + 1) % SIZE] * 0.82
            yy += 1
            x += rng.choice([-1, 0, 0, 1])
    return img


def carved_wall(r, seed):
    img, _ = sandstone_wall(r, seed)
    rng = np.random.default_rng(seed + 7)
    ramp = r["carved"]
    yy, xx = np.mgrid[0:SIZE, 0:SIZE] + 0.5
    x0, x1, y0, y1 = 12, 52, 10, 54
    inside = (xx > x0) & (xx < x1) & (yy > y0) & (yy < y1)
    grain = (tile_noise(rng, 16, 2) - 0.5) * 0.15
    img[inside] = shade(ramp, 0.42 + grain)[inside]
    # Bevelled frame: lit on top/left, shaded on bottom/right.
    frame = inside & ~((xx > x0 + 3) & (xx < x1 - 3) & (yy > y0 + 3) & (yy < y1 - 3))
    lit = frame & ((xx < x0 + 3) | (yy < y0 + 3))
    img[frame] = shade(ramp, 0.62 + grain)[frame]
    img[lit] = shade(ramp, 0.86 + grain)[lit]
    # A ring and a four-pointed star, carved: shadow above-left, light below-right.
    cx, cy = 32, 32
    d = np.hypot(xx - cx, yy - cy)
    ring = (d > 10) & (d < 14)
    star = (np.abs(xx - cx) + np.abs(yy - cy) * 2.2 < 9) | (np.abs(yy - cy) + np.abs(xx - cx) * 2.2 < 9)
    cut = ring | star
    light = ((xx - cx) + (yy - cy)) / 20
    img[cut] = shade(ramp, 0.55 + light * 0.35 + grain)[cut]
    edge = cut & ~np.roll(cut, (1, 1), axis=(0, 1))
    img[edge] = shade(ramp, 0.15 + grain)[edge]
    return img


def floor_tiles(r, seed, mossy=False):
    """Diamond flagstones, as in the reference's FLOOR swatch."""
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:SIZE, 0:SIZE]
    p = 32
    a, b = (xx + yy) % p, (xx - yy) % p
    seam = (a == 0) | (b == 0)
    tile_id = ((xx + yy) // p) * 7 + ((xx - yy) % SIZE) // p
    tone = rng.normal(0.52, 0.1, tile_id.max() + 1)[tile_id]
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3
    t = np.where((a == 1) | (b == 1), t + 0.2, t)
    t = np.where((a == p - 1) | (b == p - 1), t - 0.18, t)
    img = shade(r["floor"], t)
    img[seam] = shade(r["dirt"], 0.1 + rng.random(seam.sum()) * 0.2)
    m = tile_noise(rng, 4, 3)
    moss = shade(r["mossy"], tile_noise(rng, 16, 2))
    grow = seam & (m > (0.45 if mossy else 0.6))
    if mossy:
        grow |= (m > 0.72) & ~seam
    img[grow] = moss[grow]
    return img


def ceiling(r, seed):
    rng = np.random.default_rng(seed)
    ids, mortar, lit, dark = courses(rng, 2, 26, 44)
    tone = rng.normal(0.38, 0.08, ids.max() + 1)[ids]
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3
    t = np.where(lit, t + 0.15, t)
    t = np.where(dark, t - 0.15, t)
    img = (shade(r["sandstone"], t) * 0.55 + shade(r["stone"], t) * 0.45) * 0.82
    img[mortar] = r["stone"][0] * 0.8
    return img


def stairs(r, seed):
    """Four steps of 16 px: lit nosing at the front, shadow at the back."""
    rng = np.random.default_rng(seed)
    ids, mortar, lit, dark = courses(rng, 4, 20, 30)
    tone = rng.normal(0.55, 0.08, ids.max() + 1)[ids]
    y = np.arange(SIZE)[:, None] % 16
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3
    t = np.where(y == 15, t + 0.35, t)
    t = np.where(y == 0, t - 0.3, t)
    img = shade(r["sandstone"], t)
    img[mortar & (y != 15)] = r["stone"][1]
    return img


def arch(r, seed):
    """The wall the stairs lead under: a dark arched passage."""
    img, _ = sandstone_wall(r, seed)
    yy, xx = np.mgrid[0:SIZE, 0:SIZE] + 0.5
    cx, spring, radius = 32.0, 30.0, 20.0
    d = np.hypot(xx - cx, yy - spring)
    inside = (np.abs(xx - cx) < radius) & ((yy > spring) | (d < radius))
    ring = ~inside & (np.abs(xx - cx) < radius + 6) & ((yy > spring) | (d < radius + 6))
    depth = (1 - np.abs(xx - cx) / radius) * 0.6 + yy / SIZE * 0.4
    dark_stone = r["stone"][0] * 0.45
    img[inside] = (dark_stone * (1 - depth[..., None] * 0.9))[inside]
    angle = np.arctan2(yy - spring, xx - cx)
    joint = ring & (yy <= spring) & (d < radius + 5.5) & (np.abs(np.sin(angle * 4.5)) < 0.16)
    rt = 0.7 + (radius + 6 - np.maximum(d, np.abs(xx - cx))) * 0.03
    img[ring] = shade(r["sandstone"], rt)[ring]
    img[joint] = r["stone"][0]
    return img


def vines(r, seed):
    """Vines hanging from the top edge (RGBA decal)."""
    rng = np.random.default_rng(seed)
    out = np.zeros((SIZE, SIZE, 4))
    leaf, stem = r["plants"], r["vines"]

    def put(x, y, colour):
        if 0 <= y < SIZE:
            out[y, x % SIZE, :3] = colour
            out[y, x % SIZE, 3] = 255

    # A ragged band of foliage along the top.
    band = (tile_noise(rng, 8, 2)[0] * 9 + 2).astype(int)
    for x in range(SIZE):
        for y in range(band[x]):
            put(x, y, shade(leaf, np.array(0.25 + rng.random() * 0.6)))
    for i in range(int(rng.integers(10, 13))):
        x0 = int((i + rng.random() * 0.8) * SIZE / 12)
        length = int(rng.integers(20, 58))
        phase = rng.random() * 6.28
        for y in range(length):
            x = x0 + int(round(1.4 * np.sin(y / 7 + phase)))
            put(x, y, stem[1] if y % 3 else stem[2])
            if y % 3 == 1:
                side = 1 if (y // 3) % 2 else -1
                tone = 0.45 + rng.random() * 0.45
                for dx, dy in ((side, 0), (2 * side, 0), (side, 1), (2 * side, -1)):
                    put(x + dx, y + dy, shade(leaf, np.array(tone)))
                put(x + 2 * side, y, shade(leaf, np.array(min(1.0, tone + 0.3))))
    return out


def fern(r, seed, size=48):
    """A fern for the foot of a wall (RGBA billboard)."""
    rng = np.random.default_rng(seed)
    out = np.zeros((size, size, 4))
    leaf = r["plants"]
    base = np.array([size / 2, size - 1.0])
    for k in range(9):
        a = -1.25 + 2.5 * k / 8 + rng.normal(0, 0.08)
        length = size * (0.42 + 0.18 * (1 - abs(a) / 1.3)) * (0.9 + rng.random() * 0.2)
        steps = int(length)
        for s in range(steps):
            u = s / steps
            # Rise along the angle, then droop at the tip.
            p = base + np.array([np.sin(a), -np.cos(a)]) * s + np.array([0, (u**2) * length * 0.55])
            tone = 0.3 + 0.55 * (1 - u) * 0.5 + rng.random() * 0.25
            x, y = int(p[0]), int(p[1])
            span = int(round((1 - u) * 4)) if s % 2 == 0 else 0
            for dx in range(-span, span + 1):
                for xx, yy in ((x + dx, y), (x, y + dx // 2)):
                    if 0 <= xx < size and 0 <= yy < size:
                        out[yy, xx, :3] = shade(leaf, np.array(tone + (0.25 if dx < 0 else 0)))
                        out[yy, xx, 3] = 255
    return out


def save(path, img):
    if img.shape[-1] == 4:
        Image.fromarray(rgb(img), "RGBA").save(path, optimize=True)
    else:
        Image.fromarray(rgb(img), "RGB").save(path, optimize=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--preview", help="also write a 4x preview sheet here")
    args = ap.parse_args()

    r = sample_ramps(REFERENCE)
    textures = {
        "wall": sandstone_wall(r, 11)[0],
        "wall_mossy": mossy_wall(r, 12),
        "wall_cracked": cracked_wall(r, 13),
        "wall_carved": carved_wall(r, 14),
        "floor": floor_tiles(r, 21),
        "floor_mossy": floor_tiles(r, 22, mossy=True),
        "ceiling": ceiling(r, 31),
        "stairs": stairs(r, 41),
        "arch": arch(r, 11),
        "vines": vines(r, 51),
        "fern": fern(r, 61),
    }
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, img in textures.items():
        save(os.path.join(OUT_DIR, f"{name}.png"), img)
    print(f"wrote {len(textures)} textures to {OUT_DIR}")

    if args.preview:
        tiles = []
        for name, img in textures.items():
            if img.shape[-1] == 4:
                bg = np.zeros(img.shape[:2] + (3,)) + [40, 30, 22]
                a = img[..., 3:] / 255
                img = img[..., :3] * a + bg * (1 - a)
            if img.shape[0] != SIZE:
                pad = np.zeros((SIZE, SIZE, 3)) + [40, 30, 22]
                pad[SIZE - img.shape[0] :, : img.shape[1]] = img
                img = pad
            tiles.append(np.concatenate([img, img], axis=1))  # side by side: shows seams
        sheet = np.concatenate([np.concatenate(tiles[i : i + 3], axis=1) for i in range(0, 9, 3)], axis=0)
        extra = np.concatenate(tiles[9:] + [np.zeros_like(tiles[0])], axis=1)
        sheet = np.concatenate([sheet, extra], axis=0)
        Image.fromarray(rgb(sheet)).resize((sheet.shape[1] * 3, sheet.shape[0] * 3), Image.Resampling.NEAREST).save(args.preview)


if __name__ == "__main__":
    main()
