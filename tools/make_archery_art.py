#!/usr/bin/env python3
"""Draw the archery art: placeholder pixel art, meant to be repainted at the
same sizes. (The arrow in flight is built from simple shapes in code.)

Writes:

    crates/zrpg/assets/equipment/gear/bow.png       16x16 hunting bow (also
                                                    the HUD's bow slot)
    crates/zrpg/assets/equipment/gear/quiver.png    16x16 quiver
    crates/zrpg/assets/equipment/empty/bow.png      16x16 faint outlines for
    crates/zrpg/assets/equipment/empty/quiver.png   the empty slots
    crates/zrpg/assets/items/arrows.png             12x12 arrows lying on
                                                    the floor

Requires Pillow. Shares its helpers with make_equipment_art.py.
"""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_equipment_art import OUT, ROOT, ghost, pattern  # noqa: E402

ITEMS = ROOT / "crates/zrpg/assets/items"

WOOD = (150, 98, 52)
WOOD_LIGHT = (206, 150, 86)
WOOD_DARK = (90, 56, 28)
STRING = (226, 220, 200)
IRON = (170, 176, 190)
IRON_DARK = (90, 94, 108)
FLETCH = (196, 60, 48)
FLETCH_LIGHT = (236, 120, 96)
LEATHER = (120, 72, 40)
LEATHER_LIGHT = (170, 110, 62)
LEATHER_DARK = (70, 40, 22)


def bow():
    return pattern(
        [
            "....ww..........",
            "...wWd..........",
            "..wW.ds.........",
            "..wd..s.........",
            ".wW....s........",
            ".wd....s........",
            ".wd.....s.......",
            ".hh.....s.......",
            ".hh.....s.......",
            ".wd.....s.......",
            ".wd....s........",
            ".wW....s........",
            "..wd..s.........",
            "..wW.ds.........",
            "...wWd..........",
            "....ww..........",
        ],
        {"w": WOOD, "W": WOOD_LIGHT, "d": WOOD_DARK, "s": STRING, "h": LEATHER_DARK},
    )


def quiver():
    return pattern(
        [
            "......f..f......",
            ".....fFifFi.....",
            "......f.ifi.....",
            "......|..|......",
            ".....LLLLLL.....",
            ".....lLLLLd.....",
            ".....lLLLLd.....",
            ".....lLsLLd.....",
            ".....lLsLLd.....",
            ".....lLLLLd.....",
            ".....lLLLLd.....",
            ".....lLLsLd.....",
            ".....lLLsLd.....",
            ".....lLLLLd.....",
            "......dddd......",
            "................",
        ],
        {"f": FLETCH, "F": FLETCH_LIGHT, "i": IRON, "|": WOOD, "L": LEATHER_LIGHT,
         "l": LEATHER, "d": LEATHER_DARK, "s": (220, 200, 140)},
    )


def g_bow(d, s):
    d.arc([2 * s, 1 * s, 14 * s, 15 * s], 110, 250, fill=255, width=s)
    d.line([5 * s, 2 * s, 5 * s, 14 * s], fill=255, width=s)


def g_quiver(d, s):
    d.rectangle([5 * s, 4 * s, 11 * s, 15 * s], outline=255, width=s)
    for x in (6, 9):
        d.line([x * s, 1 * s, x * s, 4 * s], fill=255, width=s)


def arrows():
    """Three arrows lying crossed, heads to the right."""
    return pattern(
        [
            "............",
            "ff.......ki.",
            ".ffwwwwwwwk.",
            "ff.......ki.",
            "............",
            "ff......ki..",
            ".ffwwwwwwk..",
            "ff......ki..",
            "............",
            ".ff.......ki",
            "..ffwwwwwwwk",
            ".ff.......ki",
        ],
        {"f": FLETCH, "w": WOOD_LIGHT, "k": IRON_DARK, "i": IRON},
    )


def main():
    out = {
        OUT / "gear/bow.png": bow(),
        OUT / "gear/quiver.png": quiver(),
        OUT / "empty/bow.png": ghost(g_bow),
        OUT / "empty/quiver.png": ghost(g_quiver),
        ITEMS / "arrows.png": arrows(),
    }
    for path, img in out.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        img.save(path, optimize=True)
        print(path.relative_to(ROOT), img.size)


if __name__ == "__main__":
    main()
