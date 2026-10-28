#!/usr/bin/env python3
"""Draw the first spell and item art: placeholder pixel art, meant to be
repainted. The game embeds these files as they are, so any of them can be
replaced with a hand-made version of the same size.

Writes:

    crates/zrpg/assets/spells/spark.png       16x16 HUD icon
    crates/zrpg/assets/spells/chain.png       16x16 HUD icon
    crates/zrpg/assets/spells/bolt.png        16x16 HUD icon
    crates/zrpg/assets/spells/sparks.png      4 frames of 8x8 spark particles
    crates/zrpg/assets/spells/lightning.png   64x16 strip for arcs and bolts:
                                              the core runs along the middle,
                                              glow fades to the edges; it
                                              repeats left to right
    crates/zrpg/assets/spells/blade.png       16x16 HUD icons for the
    crates/zrpg/assets/spells/hammer.png      psionic blade, the magic hammer
    crates/zrpg/assets/spells/barrage.png     and the spark barrage
    crates/zrpg/assets/sword_psionic.png      the sword sheet (assets/sword.png)
                                              with its steel turned to violet
                                              light and haloed; the hand is
                                              left as drawn
    crates/zrpg/assets/items/health_potion.png 12x12 icon

Requires Pillow.
"""
from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SPELLS = ROOT / "crates/zrpg/assets/spells"
ITEMS = ROOT / "crates/zrpg/assets/items"

WHITE = (255, 255, 255)
PALE = (210, 240, 255)
CYAN = (110, 200, 255)
BLUE = (50, 110, 230)
GOLD = (255, 220, 120)
DEEP = (20, 40, 110)


def pattern(rows: list[str], palette: dict[str, tuple[int, int, int]]) -> Image.Image:
    """An image from a character map: '.' is transparent."""
    h, w = len(rows), len(rows[0])
    img = Image.new("RGBA", (w, h))
    for y, row in enumerate(rows):
        assert len(row) == w, row
        for x, c in enumerate(row):
            if c != ".":
                img.putpixel((x, y), (*palette[c], 255))
    return img


def icon_frame(img: Image.Image) -> Image.Image:
    """A dark rounded backing, so icons read on any background."""
    base = Image.new("RGBA", img.size)
    for y in range(16):
        for x in range(16):
            corner = (x in (0, 15)) and (y in (0, 15))
            if not corner:
                base.putpixel((x, y), (*DEEP, 200))
    base.alpha_composite(img)
    return base


def spark_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                ".......w........",
                ".......w....p...",
                "...p...c...p....",
                "....c..c..c.....",
                ".....cwWwc......",
                ".wwccWWWWWccww..",
                ".....cwWwc......",
                "....c..c..c.....",
                "...p...c...p....",
                ".......w........",
                ".......w....g...",
                "..g........g.g..",
                ".g.g........g...",
                "..g.............",
                "................",
            ],
            {"w": WHITE, "W": WHITE, "c": CYAN, "p": PALE, "g": GOLD},
        )
    )


def chain_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                ".oo.............",
                "oOOo....w.......",
                "oOOo...wc.......",
                ".ooc..wc..w.....",
                "....c.c..wc..oo.",
                ".....wc.wc..oOOo",
                "......cwc...oOOo",
                ".......c.....oc.",
                "...........cw...",
                ".....oo...wc....",
                "....oOOo.wc.....",
                "....oOOocw......",
                ".....oo.........",
                "................",
                "................",
            ],
            {"w": WHITE, "c": CYAN, "o": BLUE, "O": PALE},
        )
    )


def bolt_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                ".........cwwc...",
                "........cwWwc...",
                ".......cwWwc....",
                "......cwWwc.....",
                ".....cwWWwwwc...",
                "....cwwwWWwc....",
                "......cwWwc.....",
                ".....cwWwc......",
                "....cwWwc.......",
                "...cwWWwwc......",
                "..cwwwWwc.......",
                "....cwwc........",
                "...cwc..........",
                "..cc............",
                "................",
            ],
            {"w": PALE, "W": WHITE, "c": CYAN},
        )
    )


def spark_particles() -> Image.Image:
    """Four frames of a spark flaring and fading, left to right."""
    frames = [
        [
            "........",
            "...w....",
            "...w....",
            ".wwWww..",
            "...w....",
            "...w....",
            "........",
            "........",
        ],
        [
            ".c...c..",
            "..c.c...",
            "...w....",
            "ccwWwcc.",
            "...w....",
            "..c.c...",
            ".c...c..",
            "........",
        ],
        [
            "p.....p.",
            "........",
            "..c.c...",
            "...p....",
            "..c.c...",
            "........",
            "p.....p.",
            "........",
        ],
        [
            "........",
            ".p...p..",
            "........",
            "........",
            "........",
            ".p...p..",
            "........",
            "........",
        ],
    ]
    sheet = Image.new("RGBA", (32, 8))
    for i, rows in enumerate(frames):
        sheet.paste(pattern(rows, {"w": PALE, "W": WHITE, "c": CYAN, "p": BLUE}), (i * 8, 0))
    return sheet


def lightning_strip() -> Image.Image:
    """A glowing strip: a solid white-hot core along the middle, a blue glow
    fading to the top and bottom, with a little flicker along its length."""
    w, h = 64, 16
    img = Image.new("RGBA", (w, h))
    rng = random.Random(7)
    flicker = [0.8 + 0.2 * rng.random() for _ in range(w)]
    glow_rgb = (90, 160, 255)
    for x in range(w):
        for y in range(h):
            d = abs(y + 0.5 - h / 2) / (h / 2)  # 0 middle, 1 edge
            core = 1.0 if d < 0.2 else max(0.0, 1.0 - (d - 0.2) * 6.0)
            glow = max(0.0, 1.0 - d) ** 1.5
            a = min(1.0, core + 0.85 * glow) * (1.0 if core > 0.9 else flicker[x])
            rgb = tuple(int(g + (255 - g) * core) for g in glow_rgb)
            img.putpixel((x, y), (*rgb, int(255 * a)))
    return img


def health_potion() -> Image.Image:
    return pattern(
        [
            "....cc......",
            "....kk......",
            "...gkkg.....",
            "...g..g.....",
            "..g....g....",
            ".g.rrrr.g...",
            ".grRrrrrg...",
            ".grRrrrrg...",
            ".grrrrrrg...",
            "..grrrrg....",
            "...gggg.....",
            "............",
        ],
        {"c": (150, 100, 60), "k": (190, 140, 90), "g": (200, 220, 230), "r": (200, 30, 40), "R": (255, 120, 120)},
    )


VIOLET_DEEP = (70, 30, 140)
VIOLET = (160, 90, 255)
LAVENDER = (225, 205, 255)
SILVER = (200, 205, 220)
SILVER_DARK = (110, 115, 135)
HAFT = (130, 84, 46)


def blade_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                ".............LW.",
                "............LWL.",
                "...........LWV..",
                "..........LWV...",
                ".........LWV....",
                "........LWV.....",
                ".......LWV......",
                "..v...LWV.......",
                "..vv.LWV........",
                "...vVWV.........",
                "....vV..........",
                "...ddvV.........",
                "..dd..vv........",
                ".dd.............",
                "................",
            ],
            {"W": WHITE, "L": LAVENDER, "V": VIOLET, "v": VIOLET_DEEP, "d": (90, 60, 40)},
        )
    )


def hammer_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                "..c.............",
                ".c.sSSSSSs......",
                "...sWSSSSs.c....",
                "...sSSSSSsc.....",
                "...kkkhkkk......",
                "......h.....c...",
                "......h.........",
                ".....hh.........",
                ".....h..........",
                ".....h....c.....",
                "....hh..........",
                "....h...........",
                "....h...........",
                "...hh...........",
                "................",
            ],
            {"s": SILVER, "S": (230, 232, 240), "W": WHITE, "k": SILVER_DARK, "h": HAFT, "c": CYAN},
        )
    )


def barrage_icon() -> Image.Image:
    return icon_frame(
        pattern(
            [
                "................",
                "................",
                "............p...",
                "..........p.....",
                "................",
                "...w...c...w..c.",
                "..wWw.cWc.wWw...",
                "...w...c...w....",
                "................",
                ".c...w....c.....",
                "....wWw..cWc....",
                ".....w....c.....",
                "........p.......",
                "................",
                "..p.............",
                "................",
            ],
            {"w": PALE, "W": WHITE, "c": CYAN, "p": BLUE},
        )
    )


def psionic_sword() -> Image.Image:
    """The sword sheet with its steel made of violet light: grey, unsaturated
    pixels (blade, guard, pommel) are recoloured by brightness along a
    violet ramp and made a little translucent, and a soft halo is grown
    around them. Dark outlines and the warm hand are left alone."""
    src = Image.open(ROOT / "crates/zrpg/assets/sword.png").convert("RGBA")
    w, h = src.size
    px = src.load()
    out = src.copy()
    op = out.load()
    steel = [[False] * w for _ in range(h)]
    ramp = [VIOLET_DEEP, VIOLET, LAVENDER, WHITE]
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a == 0:
                continue
            hi, lo = max(r, g, b), min(r, g, b)
            sat = 0 if hi == 0 else (hi - lo) / hi
            value = hi / 255
            if sat < 0.2 and value >= 0.22:
                steel[y][x] = True
                t = min(0.999, max(0.0, (value - 0.22) / 0.78)) * (len(ramp) - 1)
                i, f = int(t), t - int(t)
                c = tuple(int(ramp[i][k] + (ramp[i + 1][k] - ramp[i][k]) * f) for k in range(3))
                op[x, y] = (*c, 215 if value < 0.85 else 255)
    # The halo: transparent pixels near the steel glow faintly.
    for y in range(h):
        for x in range(w):
            if px[x, y][3] != 0:
                continue
            near = 0
            for dy in range(-3, 4):
                for dx in range(-3, 4):
                    yy, xx = y + dy, x + dx
                    if 0 <= yy < h and 0 <= xx < w and steel[yy][xx]:
                        d = max(abs(dx), abs(dy))
                        near = max(near, 4 - d)
            if near:
                op[x, y] = (*VIOLET, 28 * near)
    return out


def main():
    SPELLS.mkdir(parents=True, exist_ok=True)
    ITEMS.mkdir(parents=True, exist_ok=True)
    out = {
        SPELLS / "spark.png": spark_icon(),
        SPELLS / "chain.png": chain_icon(),
        SPELLS / "bolt.png": bolt_icon(),
        SPELLS / "sparks.png": spark_particles(),
        SPELLS / "lightning.png": lightning_strip(),
        SPELLS / "blade.png": blade_icon(),
        SPELLS / "hammer.png": hammer_icon(),
        SPELLS / "barrage.png": barrage_icon(),
        ROOT / "crates/zrpg/assets/sword_psionic.png": psionic_sword(),
        ITEMS / "health_potion.png": health_potion(),
    }
    for path, img in out.items():
        img.save(path, optimize=True)
        print(path.relative_to(ROOT), img.size)


if __name__ == "__main__":
    main()
