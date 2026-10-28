#!/usr/bin/env python3
"""Copy the first-person sword overlay (art/kits/first-person-sword-v02) into
the game, after checking it against its manifest.

Run tools/extract_asset_kits.py first. Writes crates/zrpg/assets/sword.png:
the kit's 4 x 3 atlas of 320 x 240 screen overlays, unchanged. The game's
swing timing lives in hud.rs; the kit's timeline is a proposal (see there).

Requires Pillow.
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "art/kits/first-person-sword-v02"
OUT = ROOT / "crates/zrpg/assets/sword.png"

SCREEN = (320, 240)
COLUMNS, ROWS = 4, 3


def main():
    m = json.loads((KIT / "sword_v02_animation.json").read_text())
    assert tuple(m["viewport_px"]) == SCREEN, "overlays must match the 320 x 240 screen"
    atlas_info = m["atlas"]
    assert (atlas_info["columns"], atlas_info["rows"]) == (COLUMNS, ROWS)
    assert atlas_info["order"] == "row_major" and atlas_info["padding_px"] == 0
    assert m["idle"]["frame_index"] == 0

    atlas = Image.open(KIT / atlas_info["file"]).convert("RGBA")
    assert atlas.size == (SCREEN[0] * COLUMNS, SCREEN[1] * ROWS)
    frames = sorted((KIT / "frames").glob("sword_v02_*.png"))
    assert len(frames) == COLUMNS * ROWS
    for i, path in enumerate(frames):
        x, y = (i % COLUMNS) * SCREEN[0], (i // COLUMNS) * SCREEN[1]
        cell = atlas.crop((x, y, x + SCREEN[0], y + SCREEN[1]))
        alone = Image.open(path).convert("RGBA")
        assert cell.tobytes() == alone.tobytes(), f"atlas cell {i} differs from {path.name}"

    order = [step["frame_index"] for step in m["attack"]["timeline"]]
    assert order == list(range(1, COLUMNS * ROWS)), "attack plays frames 1..11 in order"
    impact = next(e for e in m["attack"]["events"] if e["name"] == "melee_impact")
    shutil.copyfile(KIT / atlas_info["file"], OUT)
    print(f"sword: {len(frames)} frames, kit impact on frame {impact['frame_index']}")


if __name__ == "__main__":
    main()
