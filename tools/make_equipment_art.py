#!/usr/bin/env python3
"""Draw the equipment screen's art, in the manner of an early-90s dungeon
crawler's paper doll: placeholder pixel art, meant to be repainted. The game
embeds these files as they are, so any can be replaced with a hand-made
version of the same size.

Writes, under crates/zrpg/assets/equipment/:

    backdrop.png          320x240 stone screen: a lit alcove for the figure
                          (left) and a dark panel for the stats (right)
    figure.png            84x168 silhouette that stands in the alcove
    slot.png              20x20 recessed square that holds one piece
    empty/<slot>.png      16x16 faint outline shown in an empty slot:
                          helmet, necklace, armor, right_hand, left_hand,
                          gloves, belt, ring
    gear/<gear>.png       16x16 icon for each piece: sword, torch

Requires Pillow.
"""
from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "crates/zrpg/assets/equipment"

W, H = 320, 240
# Where the alcove and the panel sit; the game lays its slots out to match.
ALCOVE = (8, 8, 196, 232)
PANEL = (202, 8, 312, 232)

STONE = (74, 66, 58)
MORTAR = (36, 31, 27)
BRONZE = (176, 128, 64)
BRONZE_DARK = (88, 60, 30)
BRONZE_LIGHT = (232, 190, 120)
GHOST = (196, 182, 150, 120)


def clamp(c: float) -> int:
    return max(0, min(255, int(c)))


def shade(rgb, k: float):
    return tuple(clamp(c * k) for c in rgb)


def bevel(img: Image.Image, box, light, dark, inset: bool) -> None:
    """A one-pixel bevel: light top-left and dark bottom-right (swapped
    when inset)."""
    x0, y0, x1, y1 = box
    d = ImageDraw.Draw(img)
    a, b = (dark, light) if inset else (light, dark)
    d.line([(x0, y1), (x0, y0), (x1, y0)], fill=a)
    d.line([(x1, y0 + 1), (x1, y1), (x0 + 1, y1)], fill=b)


def backdrop() -> Image.Image:
    rng = random.Random(3)
    img = Image.new("RGB", (W, H))
    px = img.load()
    # Stone blocks, 16 x 8, every other row offset by half.
    tones = {}
    for y in range(H):
        row = y // 8
        for x in range(W):
            bx = (x + (8 if row % 2 else 0)) // 16
            key = (bx, row)
            if key not in tones:
                tones[key] = 0.8 + 0.35 * rng.random()
            mortar = y % 8 == 7 or (x + (8 if row % 2 else 0)) % 16 == 15
            if mortar:
                px[x, y] = MORTAR
            else:
                # Each block lit a little from its top-left edge.
                edge = 1.12 if y % 8 == 0 or (x + (8 if row % 2 else 0)) % 16 == 0 else 1.0
                grain = 0.92 + 0.16 * rng.random()
                px[x, y] = shade(STONE, tones[key] * edge * grain)

    # The alcove: a deep niche, lit from above where the figure stands.
    x0, y0, x1, y1 = ALCOVE
    cx, cy = (x0 + x1) / 2, y0 + 20
    for y in range(y0 + 1, y1):
        for x in range(x0 + 1, x1):
            d = math.hypot((x - cx) / 110, (y - cy) / 190)
            light = max(0.0, 1.0 - d)
            base = (22, 20, 24)
            glow = (70, 62, 52)
            n = 0.95 + 0.1 * rng.random()
            px[x, y] = tuple(clamp((b + (g - b) * light) * n) for b, g in zip(base, glow))
    # A worn floor the figure stands on.
    for y in range(196, y1):
        for x in range(x0 + 1, x1):
            k = 0.85 + 0.3 * rng.random()
            px[x, y] = shade((48, 42, 38), k * (1.0 - (y - 196) / 80))
    bevel(img, ALCOVE, (24, 20, 18), (140, 124, 104), inset=True)

    # The panel: a dark, faintly warm slab.
    x0, y0, x1, y1 = PANEL
    for y in range(y0 + 1, y1):
        for x in range(x0 + 1, x1):
            n = 0.9 + 0.2 * rng.random()
            px[x, y] = shade((30, 25, 22), n)
    bevel(img, PANEL, (24, 20, 18), (140, 124, 104), inset=True)

    # A bronze frame around the whole screen.
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, W - 1, H - 1], outline=BRONZE_DARK)
    d.rectangle([1, 1, W - 2, H - 2], outline=BRONZE)
    d.rectangle([2, 2, W - 3, H - 3], outline=BRONZE_DARK)
    for cx, cy in [(1, 1), (W - 2, 1), (1, H - 2), (W - 2, H - 2)]:
        d.rectangle([cx - 1, cy - 1, cx + 1, cy + 1], fill=BRONZE_LIGHT)
    return img


def figure() -> Image.Image:
    """A standing figure, arms a little out, drawn big and shrunk to pixels,
    then outlined and rim-lit from the upper left."""
    fw, fh = 84, 168
    s = 4
    big = Image.new("L", (fw * s, fh * s), 0)
    d = ImageDraw.Draw(big)
    c = fw * s / 2

    def poly(points):
        d.polygon([(x * s, y * s) for x, y in points], fill=255)

    def ellipse(x0, y0, x1, y1):
        d.ellipse([x0 * s, y0 * s, x1 * s, y1 * s], fill=255)

    m = fw / 2
    ellipse(m - 9, 4, m + 9, 26)  # head
    poly([(m - 5, 22), (m + 5, 22), (m + 6, 32), (m - 6, 32)])  # neck
    # Shoulders and chest, narrowing to the waist.
    poly([(m - 24, 34), (m + 24, 34), (m + 21, 44), (m + 15, 84), (m - 15, 84), (m - 21, 44)])
    ellipse(m - 27, 31, m - 15, 45)  # shoulders
    ellipse(m + 15, 31, m + 27, 45)
    # Arms: upper arm out, forearm down to a hand by the hip.
    poly([(m - 26, 38), (m - 18, 40), (m - 25, 80), (m - 33, 78)])
    poly([(m - 33, 77), (m - 25, 79), (m - 30, 112), (m - 37, 110)])
    ellipse(m - 39, 106, m - 28, 120)
    poly([(m + 26, 38), (m + 18, 40), (m + 25, 80), (m + 33, 78)])
    poly([(m + 33, 77), (m + 25, 79), (m + 30, 112), (m + 37, 110)])
    ellipse(m + 28, 106, m + 39, 120)
    # Hips and legs, a little apart.
    poly([(m - 16, 82), (m + 16, 82), (m + 18, 98), (m + 3, 99), (m, 97), (m - 3, 99), (m - 18, 98)])
    poly([(m - 18, 96), (m - 3, 98), (m - 6, 160), (m - 16, 160)])
    poly([(m + 18, 96), (m + 3, 98), (m + 6, 160), (m + 16, 160)])
    ellipse(m - 19, 155, m - 3, 166)  # feet
    ellipse(m + 3, 155, m + 19, 166)
    del c

    mask = big.resize((fw, fh), Image.BOX).point(lambda v: 255 if v > 110 else 0)
    mp = mask.load()
    img = Image.new("RGBA", (fw, fh))
    px = img.load()
    body = (58, 64, 80)
    rim = (150, 160, 180)
    outline = (12, 12, 18)

    def solid(x, y):
        return 0 <= x < fw and 0 <= y < fh and mp[x, y] > 0

    for y in range(fh):
        for x in range(fw):
            if solid(x, y):
                edge = not all(solid(x + dx, y + dy) for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1)])
                lit = not solid(x - 1, y) or not solid(x, y - 1)
                # Darker towards the feet, like light from above.
                k = 1.15 - 0.45 * (y / fh)
                if edge and lit:
                    px[x, y] = (*rim, 255)
                elif edge:
                    px[x, y] = (*outline, 255)
                else:
                    px[x, y] = (*shade(body, k), 255)
    # A faint outline just outside the body, so it lifts off the stone.
    out = img.copy()
    op = out.load()
    for y in range(fh):
        for x in range(fw):
            if not solid(x, y) and any(solid(x + dx, y + dy) for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1)]):
                op[x, y] = (*outline, 160)
    return out


def slot_frame() -> Image.Image:
    img = Image.new("RGBA", (20, 20), (14, 12, 12, 235))
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, 19, 19], outline=(*BRONZE_DARK, 255))
    bevel(img, (1, 1, 18, 18), (*BRONZE_LIGHT, 255), (*BRONZE_DARK, 255), inset=False)
    bevel(img, (2, 2, 17, 17), (6, 5, 5, 255), (70, 60, 50, 255), inset=True)
    return img


def ghost(draw_fn) -> Image.Image:
    """A faint outline icon: drawn big in strokes, shrunk to 16 x 16."""
    s = 4
    big = Image.new("L", (16 * s, 16 * s), 0)
    draw_fn(ImageDraw.Draw(big), s)
    small = big.resize((16, 16), Image.BOX).point(lambda v: 255 if v > 70 else 0)
    img = Image.new("RGBA", (16, 16))
    img.paste(Image.new("RGBA", (16, 16), GHOST), (0, 0), small)
    return img


def g_helmet(d, s):
    d.chord([3 * s, 2 * s, 13 * s, 14 * s], 180, 360, outline=255, width=s)
    d.line([3 * s, 8 * s, 3 * s, 13 * s], fill=255, width=s)
    d.line([13 * s - 1, 8 * s, 13 * s - 1, 13 * s], fill=255, width=s)
    d.line([5 * s, 9 * s, 11 * s, 9 * s], fill=255, width=s)
    d.line([8 * s, 9 * s, 8 * s, 13 * s], fill=255, width=s)


def g_necklace(d, s):
    d.arc([2 * s, 1 * s, 14 * s, 11 * s], 20, 160, fill=255, width=s)
    d.polygon([(8 * s, 9 * s), (10 * s, 12 * s), (8 * s, 15 * s), (6 * s, 12 * s)], outline=255, width=s)


def g_armor(d, s):
    d.polygon(
        [(3 * s, 2 * s), (6 * s, 2 * s), (8 * s, 4 * s), (10 * s, 2 * s), (13 * s, 2 * s), (14 * s, 6 * s),
         (12 * s, 7 * s), (12 * s, 14 * s), (4 * s, 14 * s), (4 * s, 7 * s), (2 * s, 6 * s)],
        outline=255, width=s,
    )


def g_hand(mirror: bool):
    def draw(d, s):
        def X(x):
            return (16 - x) * s if mirror else x * s
        d.rectangle([min(X(4), X(11)), 8 * s, max(X(4), X(11)), 14 * s], outline=255, width=s)
        for fx in (4, 6.3, 8.6, 11):
            d.line([X(fx), 3 * s, X(fx), 8 * s], fill=255, width=s)
        d.line([X(11), 10 * s, X(14), 7 * s], fill=255, width=s)
    return draw


def g_gloves(d, s):
    d.polygon([(4 * s, 3 * s), (11 * s, 3 * s), (11 * s, 9 * s), (14 * s, 7 * s), (13 * s, 11 * s),
               (11 * s, 12 * s), (11 * s, 14 * s), (4 * s, 14 * s)], outline=255, width=s)
    d.line([4 * s, 11 * s, 11 * s, 11 * s], fill=255, width=s)


def g_belt(d, s):
    d.line([1 * s, 7 * s, 15 * s, 7 * s], fill=255, width=s)
    d.line([1 * s, 10 * s, 15 * s, 10 * s], fill=255, width=s)
    d.rectangle([6 * s, 5 * s, 10 * s, 12 * s], outline=255, width=s)


def g_ring(d, s):
    d.ellipse([3 * s, 5 * s, 13 * s, 15 * s], outline=255, width=s)
    d.polygon([(8 * s, 1 * s), (10 * s, 3 * s), (8 * s, 5 * s), (6 * s, 3 * s)], outline=255, width=s)


def pattern(rows, palette) -> Image.Image:
    img = Image.new("RGBA", (len(rows[0]), len(rows)))
    for y, row in enumerate(rows):
        for x, ch in enumerate(row):
            if ch != ".":
                img.putpixel((x, y), (*palette[ch], 255))
    return img


def sword() -> Image.Image:
    return pattern(
        [
            "..............kw",
            ".............kws",
            "............kws.",
            "...........kws..",
            "..........kws...",
            ".........kws....",
            "........kws.....",
            ".......kws......",
            "..g...kws.......",
            "..gg.kws........",
            "...gGws.........",
            "....gG..........",
            "...hhgG.........",
            "..hh..gg........",
            ".pp.............",
            ".p..............",
        ],
        {"w": (236, 236, 240), "s": (150, 154, 166), "k": (70, 72, 84), "g": (150, 110, 50),
         "G": (220, 180, 90), "h": (90, 56, 34), "p": (200, 160, 80)},
    )


def torch() -> Image.Image:
    return pattern(
        [
            "......y.........",
            ".....yYy........",
            "....yYWYy.......",
            "....yYWWYo......",
            "...oyYWWYo......",
            "...oyYYYyo......",
            "....oyyyo.......",
            ".....rrr........",
            ".....bBb........",
            ".....bBb........",
            "......bb........",
            "......bb........",
            "......bb........",
            "......bb........",
            "......bb........",
            "......bb........",
        ],
        {"W": (255, 250, 220), "Y": (255, 220, 100), "y": (250, 170, 50), "o": (200, 80, 30),
         "r": (60, 40, 30), "B": (150, 100, 60), "b": (100, 64, 36)},
    )


def main():
    (OUT / "empty").mkdir(parents=True, exist_ok=True)
    (OUT / "gear").mkdir(parents=True, exist_ok=True)
    out = {
        OUT / "backdrop.png": backdrop(),
        OUT / "figure.png": figure(),
        OUT / "slot.png": slot_frame(),
        OUT / "empty/helmet.png": ghost(g_helmet),
        OUT / "empty/necklace.png": ghost(g_necklace),
        OUT / "empty/armor.png": ghost(g_armor),
        OUT / "empty/right_hand.png": ghost(g_hand(mirror=False)),
        OUT / "empty/left_hand.png": ghost(g_hand(mirror=True)),
        OUT / "empty/gloves.png": ghost(g_gloves),
        OUT / "empty/belt.png": ghost(g_belt),
        OUT / "empty/ring.png": ghost(g_ring),
        OUT / "gear/sword.png": sword(),
        OUT / "gear/torch.png": torch(),
    }
    for path, img in out.items():
        img.save(path, optimize=True)
        print(path.relative_to(ROOT), img.size)


if __name__ == "__main__":
    main()
