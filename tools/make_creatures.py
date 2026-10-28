#!/usr/bin/env python3
"""Pack the creature kits' sprite atlases for the game.

Reads each kit's animation manifest (run tools/extract_asset_kits.py first)
and its four 128 px direction atlases, and writes:

    crates/zrpg/assets/creatures/<name>.png   one sheet per creature
    crates/zrpg/src/sheets.rs                 the clip table (generated)

A sheet is 6 columns (frames) x 16 rows of exact 128 x 128 cells: the four
views (front, back, left, right), each with its four actions in the kit's
atlas row order (locomotion first, then the three attacks). Cells are copied
byte for byte from the kit's atlases; nothing is redrawn, mirrored or
re-fitted, and the shared ground pivot (64, 116) is kept.

Requires Pillow.
"""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
KITS = ROOT / "art/kits"
OUT = ROOT / "crates/zrpg/assets/creatures"
RUST = ROOT / "crates/zrpg/src/sheets.rs"

VIEWS = ["front", "back", "left", "right"]
CELL = 128
FRAMES = 6
PIVOT = [64, 116]

# name -> (manifest, EnemyKind variant, standing height in world units).
# A wall is 1.0 (2 m); the zombie stands 0.95.
CREATURES = {
    "undead": ("undead-four-direction-v1/art/extracted/manifest.json", "Undead", 0.95),
    "skeleton_warrior": ("skeleton-warrior-v1/manifest.json", "SkeletonWarrior", 0.92),
    "mummy": ("mummy-v1/extracted/manifest.json", "Mummy", 0.95),
    "ember_skeleton": ("ember-skeleton-v1/frames/manifest.json", "EmberSkeleton", 0.92),
    # Hovering: the pivot marks its lowest tendrils, not a foot.
    "wraith": ("wraith-v1/extracted/manifest.json", "Wraith", 1.05),
    "skeleton_knight": ("skeleton-knight-v1/manifest.json", "SkeletonKnight", 0.98),
    "inner_demon": ("inner-demon-v1/extracted/manifest.json", "InnerDemon", 0.95),
    "demon_knight": ("demon-knight-v1/manifest.json", "DemonKnight", 1.12),
}


def load(name, manifest_path):
    path = KITS / manifest_path
    m = json.loads(path.read_text())
    base = path.parent
    assert m["runtime_size"] == [CELL, CELL], name
    frames = m["frames"]

    # Actions in atlas row order, taken from where their frames sit.
    rows = {}
    for f in frames.values():
        assert f["pivot128"] == PIVOT, f"{name}: unexpected pivot"
        rows.setdefault(f["action"], f["rect128"][1] // CELL)
    actions = sorted(rows, key=rows.get)
    assert len(actions) == 4, f"{name}: expected four actions"

    sheet = Image.new("RGBA", (CELL * FRAMES, CELL * len(VIEWS) * len(actions)))
    atlases = {}
    for v, view in enumerate(VIEWS):
        for a, action in enumerate(actions):
            anim = m["animations"][f"{view}_{action}"]
            assert len(anim["frames"]) == FRAMES, f"{name}: {view}_{action}"
            for i, key in enumerate(anim["frames"]):
                f = frames[key]
                assert f["view"] == view and f["action"] == action
                atlas = atlases.setdefault(f["atlas128"], Image.open(base / f["atlas128"]).convert("RGBA"))
                x, y, w, h = f["rect128"]
                assert (w, h) == (CELL, CELL)
                cell = atlas.crop((x, y, x + w, y + h))
                # The kit says its atlas tiles match the standalone frames.
                alone = Image.open(base / f["runtime"]).convert("RGBA")
                assert cell.tobytes() == alone.tobytes(), f"{name}: {key} atlas mismatch"
                sheet.paste(cell, (i * CELL, (v * len(actions) + a) * CELL))

    clips = []
    for action in actions:
        # Timing must agree across views to share one clip per action.
        anims = [m["animations"][f"{view}_{action}"] for view in VIEWS]
        fps = {a["fps"] for a in anims}
        loop = {a["loop"] for a in anims}
        hits = {tuple(a["suggested_hit_frames_zero_based"]) for a in anims}
        assert len(fps) == len(loop) == len(hits) == 1, f"{name}: {action} differs by view"
        hit = hits.pop()
        assert len(hit) <= 1
        clips.append((action, fps.pop(), loop.pop(), hit[0] if hit else None))
    assert clips[0][2] and not any(c[2] for c in clips[1:]), f"{name}: locomotion first"

    # Standing body height: front walk frame 0, from the kit's alpha bounds
    # (256 px masters) halved to the 128 px runtime cells.
    _, top, _, bottom = frames[m["animations"][f"front_{actions[0]}"]["frames"][0]]["alpha_bounds256"]
    body_px = (bottom - top) / 2
    return sheet, clips, body_px


def rust_clip(action, fps, hit):
    hit = "None" if hit is None else f"Some({hit})"
    return f'Clip {{ name: "{action}", fps: {fps:.1f}, hit: {hit} }}'


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    consts = []
    arms = []
    for name, (manifest, kind, height) in CREATURES.items():
        sheet, clips, body_px = load(name, manifest)
        sheet.save(OUT / f"{name}.png", optimize=True)
        const = name.upper()
        locomotion, attacks = clips[0], clips[1:]
        consts.append(
            f"""/// From `art/kits/{manifest}`.
pub const {const}: Sheet = Sheet {{
    png: include_bytes!("../assets/creatures/{name}.png"),
    height: {height},
    body_px: {body_px:.1f},
    locomotion: {rust_clip(locomotion[0], locomotion[1], None)},
    attacks: [
{"".join(f"        {rust_clip(a, f, h)},{chr(10)}" for a, f, _, h in attacks)}    ],
}};
"""
        )
        arms.append(f"        EnemyKind::{kind} => Some(&{const}),")
        print(f"{name}: {[c[0] for c in clips]}, body {body_px:.0f} px")

    RUST.write_text(
        f"""//! Creature sprite sheets. Generated by `tools/make_creatures.py` from the
//! creature kits' manifests in `art/kits`; do not edit.

use zrpg_core::EnemyKind;

use crate::enemies::{{Clip, Sheet}};

{chr(10).join(consts)}
/// The animated sheet for a kind, if it has one (the zombie is a still).
pub fn sheet(kind: EnemyKind) -> Option<&'static Sheet> {{
    match kind {{
{chr(10).join(arms)}
        _ => None,
    }}
}}
"""
    )


if __name__ == "__main__":
    main()
