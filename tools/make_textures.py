#!/usr/bin/env python3
"""Generate the environment textures in the palette of the environment reference.

    python3 tools/make_textures.py [--preview sheet.png]

Colour ramps are sampled from the texture-palette strip of
art/environment-reference.png; the patterns (block courses, moss, cracks,
carvings, diamond floor tiles, vines, ferns, torches) are generated, so
every tile is seamless and sharp at the game's resolution.

Each texture is generated together with a height field, from which come:

- `<name>_n.png`: a tangent-space normal map (OpenGL convention: +Y points
  to the top of the image), so lights rake across bevels, joints and cracks;
- `<name>_orm.png`: occlusion (R, cavities from the height field), roughness
  (G: matte sandstone, worn tiles, wet moss) and metallic (B, always 0), the
  glTF packing Bevy reads for `occlusion_texture` and
  `metallic_roughness_texture`.

Output goes to crates/zrpg/assets/env/, where the game embeds it.
Needs Pillow and NumPy. Deterministic: same reference, same textures.
"""
import argparse
import os

import numpy as np
from PIL import Image

REFERENCE = "art/environment-reference.png"
OUT_DIR = "crates/zrpg/assets/env"
SIZE = 64
# Height-to-normal steepness: higher = deeper-looking relief.
RELIEF = 3.0

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


class Tex:
    """A generated texture: colour, height (0 = deepest, 1 = proud),
    roughness (0 = glossy, 1 = matte) and, for cutouts, alpha."""

    def __init__(self, albedo, height, rough, alpha=None):
        self.albedo, self.height, self.rough, self.alpha = albedo, height, rough, alpha


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


# --- noise, layout and relief --------------------------------------------------

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


def chunky_noise(rng, cluster=2, size=SIZE):
    """Coarse grain: random values in 2x2-texel clusters, like the
    reference's sandstone."""
    n = size // cluster
    return np.kron(rng.random((n, n)), np.ones((cluster, cluster)))


def courses(rng, n_courses, min_w, max_w, size=SIZE):
    """Staggered block courses that tile horizontally. Returns per-pixel
    block ids and the mortar mask."""
    h = size // n_courses
    ids = np.zeros((size, size), int)
    mortar = np.zeros((size, size), bool)
    next_id = 0
    for c in range(n_courses):
        widths = []
        while sum(widths) < size:
            widths.append(int(rng.integers(min_w, max_w + 1)))
        widths[-1] -= sum(widths) - size
        if widths[-1] < min_w * 0.6 and len(widths) > 1:
            widths[-2] += widths.pop()
        x = int(rng.integers(0, size))
        for w in widths:
            cols = (np.arange(x, x + w)) % size
            rows = slice(c * h, (c + 1) * h)
            ids[rows, cols] = next_id
            mortar[c * h + h - 1, cols] = True
            mortar[rows, cols[0]] = True
            next_id += 1
            x += w
    return ids, mortar


def distance_to(mask, max_dist=8):
    """Wrapped city-block distance from every pixel to the nearest True."""
    d = np.where(mask, 0.0, float(max_dist))
    for _ in range(max_dist):
        for axis in (0, 1):
            for shift in (1, -1):
                d = np.minimum(d, np.roll(d, shift, axis=axis) + 1)
    return d


def bevel(dist, width):
    """0 at an edge, rising smoothly to 1 at `width` pixels in."""
    t = np.clip(dist / width, 0.0, 1.0)
    return t * t * (3 - 2 * t)


def blur(a, radius=2):
    out = a.copy()
    for axis in (0, 1):
        acc = out.copy()
        for s in range(1, radius + 1):
            acc += np.roll(out, s, axis=axis) + np.roll(out, -s, axis=axis)
        out = acc / (2 * radius + 1)
    return out


def normal_map(height, relief=RELIEF):
    """Tangent-space normals, OpenGL convention: x right, y up the image."""
    dx = (np.roll(height, -1, axis=1) - np.roll(height, 1, axis=1)) * 0.5
    drow = (np.roll(height, -1, axis=0) - np.roll(height, 1, axis=0)) * 0.5
    n = np.dstack([-dx * relief, drow * relief, np.ones_like(height)])
    return n / np.linalg.norm(n, axis=2, keepdims=True)


def occlusion(height):
    """Cavities (pixels lower than their surroundings) get less ambient light."""
    cavity = np.clip(blur(height, 2) - height, 0.0, None)
    return np.clip(1.0 - cavity * 2.2, 0.4, 1.0)


# --- environment textures --------------------------------------------------------

def sandstone_wall(r, seed, n_courses=4):
    rng = np.random.default_rng(seed)
    ids, mortar = courses(rng, n_courses, 24, 42)
    dist = distance_to(mortar)
    edge = bevel(dist, 2.5)
    tone = rng.normal(0.5, 0.13, ids.max() + 1)[ids]
    grain = chunky_noise(rng) - 0.5
    t = (
        tone
        + (tile_noise(rng, 16, 2) - 0.5) * 0.3
        + (tile_noise(rng, 2, 2) - 0.5) * 0.25
        + grain * 0.22
        + (edge - 1) * 0.12  # a hint of baked shading at the joints
    )
    pits = rng.random((SIZE, SIZE)) < 0.035
    t = np.where(pits, t - 0.25, t)
    albedo = shade(r["sandstone"], t)
    albedo[mortar] = shade(r["stone"], 0.05 + rng.random(mortar.sum()) * 0.15)

    height = 0.2 + 0.8 * edge + grain * 0.1 + (tile_noise(rng, 8, 2) - 0.5) * 0.08
    height = np.where(pits, height - 0.3, height)
    height[mortar] = 0.0
    rough = 0.86 + tile_noise(rng, 8, 2) * 0.1
    rough[mortar] = 0.95
    return Tex(albedo, height, rough), (ids, mortar)


def mossy_wall(r, seed):
    tex, (ids, mortar) = sandstone_wall(r, seed)
    rng = np.random.default_rng(seed + 100)
    y = np.arange(SIZE)[:, None] / SIZE
    ledge = np.roll(mortar, 1, axis=0) & ~mortar  # top rows of blocks catch moss
    m = tile_noise(rng, 6, 3) + y * 0.2 + mortar * 0.1 + ledge * 0.25
    moss = shade(r["mossy"], tile_noise(rng, 16, 2) * 1.2 - 0.05)
    dither = (np.indices((SIZE, SIZE)).sum(axis=0) % 2) == 0
    grow = (m > 0.8) | ((m > 0.72) & dither)
    tex.albedo[grow] = moss[grow]
    tex.height[grow] = np.maximum(tex.height[grow], 0.55) + tile_noise(rng, 16, 1)[grow] * 0.25
    tex.rough[grow] = 0.5  # damp moss catches a soft sheen
    return tex


def cracked_wall(r, seed):
    rng = np.random.default_rng(seed)
    tex, (ids, mortar) = sandstone_wall(r, seed)
    # Some blocks have weathered to grey stone.
    grey = rng.random(ids.max() + 1) < 0.35
    stone = shade(r["cracked"], tile_noise(rng, 16, 2) * 0.6 + 0.25)
    sel = grey[ids] & ~mortar
    tex.albedo[sel] = stone[sel]
    tex.rough[sel] = 0.95
    for _ in range(3):
        x, yy = float(rng.integers(0, SIZE)), float(rng.integers(0, SIZE // 2))
        for _ in range(int(rng.integers(18, 40))):
            xi, yi = int(x) % SIZE, int(yy)
            if yi >= SIZE:
                break
            tex.albedo[yi, xi] = r["cracked"][0] * 0.55
            tex.height[yi, xi] = 0.0
            nx = (xi + 1) % SIZE
            tex.albedo[yi, nx] *= 0.85
            tex.height[yi, nx] = min(tex.height[yi, nx], 0.45)
            yy += 1
            x += rng.choice([-1, 0, 0, 1])
    return tex


def carved_wall(r, seed):
    tex, _ = sandstone_wall(r, seed)
    rng = np.random.default_rng(seed + 7)
    ramp = r["carved"]
    yy, xx = np.mgrid[0:SIZE, 0:SIZE] + 0.5
    x0, x1, y0, y1 = 12, 52, 10, 54
    inside = (xx > x0) & (xx < x1) & (yy > y0) & (yy < y1)
    grain = (tile_noise(rng, 16, 2) - 0.5) * 0.15
    # The panel: a recessed field inside a raised, bevelled frame.
    border = np.minimum.reduce([xx - x0, x1 - xx, yy - y0, y1 - yy])
    frame = inside & (border < 4)
    tex.albedo[inside] = shade(ramp, 0.5 + grain)[inside]
    tex.albedo[frame] = shade(ramp, 0.66 + grain)[frame]
    tex.height[inside] = 0.5 + grain[inside]
    tex.height[frame] = (0.5 + 0.45 * np.sin(np.clip(border / 4, 0, 1) * np.pi))[frame]
    # A ring and a four-pointed star cut into the field.
    cx, cy = 32, 32
    d = np.hypot(xx - cx, yy - cy)
    ring = (d > 10) & (d < 14)
    star = (np.abs(xx - cx) + np.abs(yy - cy) * 2.2 < 9) | (
        np.abs(yy - cy) + np.abs(xx - cx) * 2.2 < 9
    )
    cut = ring | star
    tex.albedo[cut] = shade(ramp, 0.42 + grain)[cut]
    tex.height[cut] = 0.22
    tex.rough[inside] = 0.72  # dressed stone, smoother than the blocks
    return tex


def floor_tiles(r, seed, mossy=False):
    """Diamond flagstones, as in the reference's FLOOR swatch."""
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:SIZE, 0:SIZE]
    p = 32
    a, b = (xx + yy) % p, (xx - yy) % p
    seam = (a == 0) | (b == 0)
    tile_id = ((xx + yy) // p) * 7 + ((xx - yy) % SIZE) // p
    edge = bevel(distance_to(seam), 3.0)
    tone = rng.normal(0.52, 0.1, tile_id.max() + 1)[tile_id]
    grain = chunky_noise(rng) - 0.5
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3 + grain * 0.12 + (edge - 1) * 0.1
    albedo = shade(r["floor"], t)
    albedo[seam] = shade(r["dirt"], 0.1 + rng.random(seam.sum()) * 0.2)
    height = 0.15 + 0.85 * edge + grain * 0.06
    height[seam] = 0.0
    # Worn smooth by feet: the shiniest surface in the dungeon.
    rough = 0.55 + tile_noise(rng, 8, 2) * 0.15
    rough[seam] = 0.95
    m = tile_noise(rng, 4, 3)
    moss = shade(r["mossy"], tile_noise(rng, 16, 2))
    grow = seam & (m > (0.45 if mossy else 0.6))
    if mossy:
        grow |= (m > 0.72) & ~seam
    albedo[grow] = moss[grow]
    height[grow] = np.maximum(height[grow], 0.3)
    rough[grow] = 0.5
    return Tex(albedo, height, rough)


def ceiling(r, seed):
    rng = np.random.default_rng(seed)
    ids, mortar = courses(rng, 2, 26, 44)
    edge = bevel(distance_to(mortar), 3.0)
    tone = rng.normal(0.38, 0.08, ids.max() + 1)[ids]
    grain = chunky_noise(rng) - 0.5
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3 + grain * 0.15 + (edge - 1) * 0.1
    albedo = (shade(r["sandstone"], t) * 0.55 + shade(r["stone"], t) * 0.45) * 0.82
    albedo[mortar] = r["stone"][0] * 0.8
    height = 0.2 + 0.8 * edge + grain * 0.08
    height[mortar] = 0.0
    return Tex(albedo, height, np.full((SIZE, SIZE), 0.9))


def trim(r, seed):
    """Dressed stone for baseboards and crown molding: smoother and paler
    than the walls, with a joint every half cell."""
    rng = np.random.default_rng(seed)
    x = np.arange(SIZE)[None, :] + np.zeros((SIZE, 1))
    joint = (x % 32 == 0)
    edge = bevel(distance_to(joint), 2.0)
    grain = chunky_noise(rng) - 0.5
    t = 0.62 + (tile_noise(rng, 8, 2) - 0.5) * 0.2 + grain * 0.08 + (edge - 1) * 0.1
    albedo = shade(r["sandstone"], t)
    albedo[joint] = r["stone"][1]
    height = 0.3 + 0.7 * edge + grain * 0.04
    height[joint] = 0.0
    return Tex(albedo, height, np.full((SIZE, SIZE), 0.7))


def stairs(r, seed):
    """Four steps of 16 px each: a rounded nosing at the front edge (row 15)
    and a crease at the back where the next riser meets the tread."""
    rng = np.random.default_rng(seed)
    ids, mortar = courses(rng, 4, 20, 30)
    tone = rng.normal(0.55, 0.08, ids.max() + 1)[ids]
    row = np.arange(SIZE)[:, None] % 16
    grain = chunky_noise(rng) - 0.5
    t = tone + (tile_noise(rng, 16, 2) - 0.5) * 0.3 + grain * 0.12
    t = np.where(row == 15, t + 0.15, t)
    t = np.where(row == 0, t - 0.15, t)
    albedo = shade(r["sandstone"], t)
    joint = mortar & (row != 15)
    albedo[joint] = r["stone"][1]
    profile = np.select([row == 0, row == 1, row >= 14], [0.35, 0.7, 0.8], 0.9)
    height = profile + grain * 0.06 + np.zeros((SIZE, SIZE))
    height[joint] = 0.2
    return Tex(albedo, height, np.full((SIZE, SIZE), 0.85))


def arch(r, seed):
    """The wall the stairs lead under: a dark arched passage."""
    tex, _ = sandstone_wall(r, seed)
    yy, xx = np.mgrid[0:SIZE, 0:SIZE] + 0.5
    cx, spring, radius = 32.0, 30.0, 20.0
    d = np.hypot(xx - cx, yy - spring)
    inside = (np.abs(xx - cx) < radius) & ((yy > spring) | (d < radius))
    ring = ~inside & (np.abs(xx - cx) < radius + 6) & ((yy > spring) | (d < radius + 6))
    depth = (1 - np.abs(xx - cx) / radius) * 0.6 + yy / SIZE * 0.4
    tex.albedo[inside] = (r["stone"][0] * 0.45 * (1 - depth[..., None] * 0.9))[inside]
    tex.height[inside] = 0.0
    tex.rough[inside] = 1.0
    angle = np.arctan2(yy - spring, xx - cx)
    joint = ring & (yy <= spring) & (d < radius + 5.5) & (np.abs(np.sin(angle * 4.5)) < 0.16)
    across = np.minimum(np.abs(np.maximum(d, np.abs(xx - cx)) - radius), radius + 6 - d)
    rt = 0.7 + (radius + 6 - np.maximum(d, np.abs(xx - cx))) * 0.03
    tex.albedo[ring] = shade(r["sandstone"], rt)[ring]
    tex.height[ring] = (0.35 + 0.65 * bevel(across, 2.0))[ring]
    tex.albedo[joint] = r["stone"][0]
    tex.height[joint] = 0.05
    return tex


def vines(r, seed):
    """Vines hanging from the top edge (cutout)."""
    rng = np.random.default_rng(seed)
    albedo = np.zeros((SIZE, SIZE, 3))
    height = np.zeros((SIZE, SIZE))
    alpha = np.zeros((SIZE, SIZE), bool)
    leaf, stem = r["plants"], r["vines"]

    def put(x, y, colour, h):
        if 0 <= y < SIZE:
            albedo[y, x % SIZE] = colour
            height[y, x % SIZE] = h
            alpha[y, x % SIZE] = True

    # A ragged band of foliage along the top.
    band = (tile_noise(rng, 8, 2)[0] * 9 + 2).astype(int)
    for x in range(SIZE):
        for y in range(band[x]):
            put(x, y, shade(leaf, np.array(0.25 + rng.random() * 0.6)), 0.5 + rng.random() * 0.5)
    for i in range(int(rng.integers(10, 13))):
        x0 = int((i + rng.random() * 0.8) * SIZE / 12)
        length = int(rng.integers(20, 58))
        phase = rng.random() * 6.28
        for y in range(length):
            x = x0 + int(round(1.4 * np.sin(y / 7 + phase)))
            put(x, y, stem[1] if y % 3 else stem[2], 0.5)
            if y % 3 == 1:
                side = 1 if (y // 3) % 2 else -1
                tone = 0.45 + rng.random() * 0.45
                for dx, dy in ((side, 0), (2 * side, 0), (side, 1), (2 * side, -1)):
                    put(x + dx, y + dy, shade(leaf, np.array(tone)), 0.8)
                put(x + 2 * side, y, shade(leaf, np.array(min(1.0, tone + 0.3))), 1.0)
    return Tex(albedo, height, np.full((SIZE, SIZE), 0.55), alpha)


def fern(r, seed, size=48):
    """A fern for the foot of a wall (cutout)."""
    rng = np.random.default_rng(seed)
    albedo = np.zeros((size, size, 3))
    height = np.zeros((size, size))
    alpha = np.zeros((size, size), bool)
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
                        albedo[yy, xx] = shade(leaf, np.array(tone + (0.25 if dx < 0 else 0)))
                        height[yy, xx] = 1.0 - abs(dx) * 0.15
                        alpha[yy, xx] = True
    return Tex(albedo, height, np.full((size, size), 0.55), alpha)


def sconce(r, seed, w=16, h=24):
    """An iron wall bracket holding a wooden torch (cutout). The flame is
    a separate, glowing sprite drawn above it."""
    albedo = np.zeros((h, w, 3))
    height = np.zeros((h, w))
    alpha = np.zeros((h, w), bool)
    iron = np.array([[26, 24, 26], [48, 45, 46], [78, 74, 72]], float)
    wood = shade(r["dirt"], np.linspace(0.2, 0.9, 6))
    cx = w // 2
    # Torch shaft: from the top of the cup down to near the bottom.
    for y in range(4, h - 4):
        for x in (cx - 1, cx):
            albedo[y, x] = wood[min(5, (y + x) % 6)]
            height[y, x] = 0.7 if x == cx - 1 else 0.5
            alpha[y, x] = True
    # Cup around the top of the shaft, collar, and the wall plate.
    for y, half in ((3, 3), (4, 3), (5, 2), (12, 2), (h - 4, 2), (h - 3, 3), (h - 2, 3)):
        for x in range(cx - half, cx + half):
            albedo[y, x] = iron[2 if y in (3, h - 4) else 1]
            height[y, x] = 0.9 if y in (3, h - 4) else 0.6
            alpha[y, x] = True
    return Tex(albedo, height, np.full((h, w), 0.45), alpha)


def flame(w=16, h=24):
    """A torch flame: white-hot core, orange body, red tips (cutout,
    drawn unlit and glowing)."""
    yy, xx = np.mgrid[0:h, 0:w] + 0.5
    cx = w / 2
    # Teardrop: widest near the bottom, pointed at the top.
    u = 1 - yy / h  # 0 at the bottom, 1 at the top
    half = (w * 0.42) * np.sin(np.clip(u, 0, 1) * np.pi) ** 0.8 * (1 - u * 0.35)
    r = np.abs(xx - cx) / np.maximum(half, 1e-3)
    alpha = (r < 1) & (u > 0.06)
    heat = np.clip((1 - r) * (1.2 - u), 0, 1)
    colour = np.where(
        heat[..., None] > 0.7,
        [255, 246, 200],
        np.where(heat[..., None] > 0.4, [255, 170, 60], [215, 70, 30]),
    ).astype(float)
    return Tex(colour, np.full((h, w), 0.5), np.full((h, w), 1.0), alpha)


# --- output --------------------------------------------------------------------

def to_u8(a):
    return np.clip(a, 0, 255).astype(np.uint8)


def save(name, tex, maps=True):
    base = os.path.join(OUT_DIR, name)
    if tex.alpha is not None:
        rgba = np.dstack([to_u8(tex.albedo), np.where(tex.alpha, 255, 0).astype(np.uint8)])
        Image.fromarray(rgba, "RGBA").save(base + ".png", optimize=True)
    else:
        Image.fromarray(to_u8(tex.albedo), "RGB").save(base + ".png", optimize=True)
    if not maps:
        return
    n = normal_map(tex.height)
    Image.fromarray(to_u8((n * 0.5 + 0.5) * 255), "RGB").save(base + "_n.png", optimize=True)
    orm = np.dstack([occlusion(tex.height), tex.rough, np.zeros_like(tex.rough)])
    Image.fromarray(to_u8(orm * 255), "RGB").save(base + "_orm.png", optimize=True)


def lit_preview(tex):
    """The texture lit from the upper left, to check the normal maps by eye."""
    n = normal_map(tex.height)
    light = np.array([-0.55, 0.55, 0.63])
    light /= np.linalg.norm(light)
    lambert = np.clip((n * light).sum(axis=2), 0, 1)
    img = tex.albedo * (0.25 + 0.95 * lambert[..., None])
    if tex.alpha is not None:
        img[~tex.alpha] = [40, 30, 22]
    return img


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--preview", help="also write a preview sheet: colour | normals | roughness | lit")
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
        "trim": trim(r, 81),
        "vines": vines(r, 51),
        "fern": fern(r, 61),
        "sconce": sconce(r, 71),
        "flame": flame(),
    }
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, tex in textures.items():
        save(name, tex, maps=name != "flame")  # fire is drawn unlit
    print(f"wrote {len(textures)} textures (+ normal and ORM maps) to {OUT_DIR}")

    if args.preview:
        rows = []
        for tex in textures.values():
            h, w = tex.height.shape
            pad = np.zeros((SIZE, SIZE, 3)) + [40, 30, 22]

            def fit(img):
                out = pad.copy()
                out[SIZE - h :, :w] = img
                return out

            albedo = tex.albedo.copy()
            if tex.alpha is not None:
                albedo[~tex.alpha] = [40, 30, 22]
            panels = [
                albedo,
                (normal_map(tex.height) * 0.5 + 0.5) * 255,
                np.repeat(tex.rough[..., None], 3, axis=2) * 255,
                lit_preview(tex),
            ]
            rows.append(np.concatenate([fit(p) for p in panels], axis=1))
        half = (len(rows) + 1) // 2
        left = np.concatenate(rows[:half], axis=0)
        right = np.concatenate(rows[half:] + [np.zeros_like(rows[0])] * (2 * half - len(rows)), axis=0)
        sheet = np.concatenate([left, np.zeros((left.shape[0], 8, 3)), right], axis=1)
        Image.fromarray(to_u8(sheet)).resize(
            (sheet.shape[1] * 2, sheet.shape[0] * 2), Image.Resampling.NEAREST
        ).save(args.preview)


if __name__ == "__main__":
    main()
