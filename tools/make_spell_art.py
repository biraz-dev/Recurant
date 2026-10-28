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


def main():
    SPELLS.mkdir(parents=True, exist_ok=True)
    ITEMS.mkdir(parents=True, exist_ok=True)
    out = {
        SPELLS / "spark.png": spark_icon(),
        SPELLS / "chain.png": chain_icon(),
        SPELLS / "bolt.png": bolt_icon(),
        SPELLS / "sparks.png": spark_particles(),
        SPELLS / "lightning.png": lightning_strip(),
        ITEMS / "health_potion.png": health_potion(),
    }
    for path, img in out.items():
        img.save(path, optimize=True)
        print(path.relative_to(ROOT), img.size)


if __name__ == "__main__":
    main()
