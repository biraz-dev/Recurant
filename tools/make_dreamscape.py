#!/usr/bin/env python3
"""Build a playable level from one Dreamscape theme kit, for the game's
embedded formats.

Reads the extracted kit (run tools/extract_dreamscape_assets.py first):

    art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/
        common/geometry/geometry-manifest.json   module bounds, pivots, guides
        common/geometry/meshes/<module>.glb      the shared module meshes
        themes/<theme>/theme-manifest.json       the theme's material bindings

and writes, for a theme with a layout below:

    crates/zrpg/assets/dream/<theme>/level.bin   modules + instances + lights
    crates/zrpg/assets/dream/<theme>/*.png       wall/ground/structure/detail
    crates/zrpg_core/src/dream/<theme>.map       walkable cells + edge masks

The kit's own showcase is a 3 x 5 cell cutaway gallery, not a level; this
lays out a small playable floor from the same modules instead (the kit's
"module + placement recipe" route, never its assembled GLB as well), and
follows its contract (common/spec/COLLISION_AND_IMPORT.md):

- metres, +Y up, +X east, -Z north, 2 m cells; the game scales by 0.5 at
  load, exactly once. Pivots and authored sizes are kept as they are.
- Walls' detailed face is -Z: a south wall is unrotated, north 180, west
  -90, east +90 (degrees about Y).
- Materials bind by the four role names, never by order. Its showcase
  applies no role overrides to these modules; any module whose material
  names aren't exactly the four roles is refused rather than guessed.
- Walkability comes from the layout, not from triangles: every open
  neighbour pair is an allowed, two-way edge; masks are derived from that.
  Prop cells are blocked exhibits, open to the eye.
- Checks before writing: every module's GLB has no node transforms; the
  1.2 m lane with 2.4 m headroom between every pair of walkable cells
  misses every placed module's collision guide (compound door guides, so
  doorways stay open); every walkable cell is reachable from the start.

Standard library only, like tools/make_ruins.py (whose GLB reader it uses).
"""
from __future__ import annotations

import json
import math
import shutil
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_ruins import accessor, read_glb  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials"
GEO = KIT / "common/geometry"

ROLES = ["wall", "ground", "structure", "detail"]
DIRS = {(0, -1): 1, (1, 0): 2, (0, 1): 4, (-1, 0): 8}  # N E S W bits
# The doorway lane, from the contract: 1.2 m wide, 2.4 m headroom.
LANE_WIDTH = 1.2
LANE_HEADROOM = 2.4

# A layout, per theme. Map characters: '#' solid rock; '.' floor; 'S' the
# start (facing north); '>' the way down; any other letter a blocked
# exhibit cell holding that prop. Doors are wall openings between two open
# cells; portals are freestanding arches across a lane. Glows are lights,
# at a prop letter's cells. Cells are (x, z), z growing southward.
LAYOUTS = {
    "mindglass-sanctum": dict(
        title="Mindglass Sanctum",
        rows=[
            "###############",
            "#####o.a.o#####",
            "#####..>..#####",
            "#####x...x#####",
            "#####.....#####",
            "#######.#######",
            "##p.........p##",
            "##..q.....q..##",
            "##c.........c##",
            "##..q.....q..##",
            "##u.........A##",
            "#######.#######",
            "#######.#######",
            "#######.#######",
            "#######S#######",
            "###############",
        ],
        floor="floor_inset_2m",
        ceiling="ceiling_coffer_2m",
        wall="masonry_inset_panel",
        door="masonry_doorway_pointed",
        portal="crystal_pointed_arch",
        # letter: (module, yaw in degrees)
        props={
            "o": ("crystal_obelisk", 0),
            "a": ("astral_ring_focus", 0),
            "x": ("crystal_cluster", 30),
            "p": ("ceremonial_pedestal", 15),
            "q": ("column_square", 0),
            "c": ("crystal_shard_tall", -15),
            "u": ("urn", -15),
            "A": ("altar_block", 90),
        },
        doors=[((7, 4), (7, 5)), ((7, 5), (7, 6)), ((7, 10), (7, 11))],
        portals=[((7, 12), (7, 13)), ((7, 2), (7, 1))],
        # letter: (height m, linear rgb, lumens): soft crystal light.
        glows={
            "o": (2.6, (0.62, 0.38, 1.0), 5500.0),
            "a": (1.2, (0.55, 0.8, 1.0), 7000.0),
            "c": (2.2, (0.7, 0.5, 1.0), 4000.0),
            "p": (1.6, (1.0, 0.78, 0.5), 3200.0),
        },
    ),
}


def rot_y(deg):
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def apply(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def load_module(module_id, catalog):
    gltf, binary = read_glb(GEO / catalog[module_id]["files"]["glb"])
    for node in gltf["nodes"]:
        # The contract: transforms are baked into the vertices.
        assert not any(k in node for k in ("matrix", "translation", "rotation", "scale")), (
            module_id,
            "unbaked node transform",
        )
    prims = []
    for mesh in gltf["meshes"]:
        for p in mesh["primitives"]:
            assert p.get("mode", 4) == 4
            role = gltf["materials"][p["material"]]["name"]
            assert role in ROLES, (module_id, role, "not one of the four roles")
            prims.append(
                dict(
                    role=ROLES.index(role),
                    pos=accessor(gltf, binary, p["attributes"]["POSITION"]),
                    nrm=accessor(gltf, binary, p["attributes"]["NORMAL"]),
                    uv=accessor(gltf, binary, p["attributes"]["TEXCOORD_0"]),
                    idx=[i[0] for i in accessor(gltf, binary, p["indices"])],
                )
            )
    return prims


def build(theme):
    spec = LAYOUTS[theme]
    manifest = json.loads((KIT / f"themes/{theme}/theme-manifest.json").read_text())
    catalog = {a["id"]: a for a in json.loads((GEO / "geometry-manifest.json").read_text())["assets"]}
    rows = spec["rows"]
    height, width = len(rows), len(rows[0])
    assert all(len(r) == width for r in rows)

    def char(x, z):
        return rows[z][x] if 0 <= x < width and 0 <= z < height else "#"

    cells = [(x, z) for z in range(height) for x in range(width)]
    open_cells = {c for c in cells if char(*c) != "#"}
    walkable = {c for c in open_cells if char(*c) in ".S>"}
    props = {c: char(*c) for c in open_cells - walkable}
    start = next(c for c in cells if char(*c) == "S")
    goal = next(c for c in cells if char(*c) == ">")

    instances = []

    def place(module_id, pos, yaw=0.0, scale=1.0, role="scenery"):
        assert module_id in catalog, module_id
        instances.append(dict(module=module_id, pos=list(pos), yaw=yaw, scale=scale, role=role))

    for x, z in sorted(open_cells):
        centre = (2 * x + 1, 0, 2 * z + 1)
        # The way down is a stair pit sunk into the floor (the game draws
        # it), so its cell has no floor slab.
        if (x, z) != goal:
            place(spec["floor"], centre, role="floor")
        place(spec["ceiling"], centre, role="ceiling")
    # Walls on every edge between an open cell and rock, facing the room.
    for x, z in sorted(open_cells):
        for (dx, dz), _ in DIRS.items():
            if (x + dx, z + dz) in open_cells:
                continue
            if dz == 1:
                place(spec["wall"], (2 * x + 1, 0, 2 * z + 2), 0, role="wall")
            elif dz == -1:
                place(spec["wall"], (2 * x + 1, 0, 2 * z), 180, role="wall")
            elif dx == -1:
                place(spec["wall"], (2 * x, 0, 2 * z + 1), -90, role="wall")
            else:
                place(spec["wall"], (2 * x + 2, 0, 2 * z + 1), 90, role="wall")

    def edge_place(module_id, a, b, role):
        (ax, az), (bx, bz) = a, b
        assert abs(ax - bx) + abs(az - bz) == 1 and a in open_cells and b in open_cells, (a, b)
        if ax == bx:  # a north-south step: the opening faces along Z
            place(module_id, (2 * ax + 1, 0, 2 * max(az, bz)), 0, role=role)
        else:
            place(module_id, (2 * max(ax, bx), 0, 2 * az + 1), 90, role=role)

    for a, b in spec["doors"]:
        edge_place(spec["door"], a, b, "door")
    for a, b in spec["portals"]:
        edge_place(spec["portal"], a, b, "portal")
    # Exhibits: each prop centred in its cell, shrunk (never grown) to fit
    # a 1.8 m pad, as the kit's showcase does.
    for (x, z), letter in sorted(props.items()):
        module_id, yaw = spec["props"][letter]
        b = catalog[module_id]["bounds_m"]
        size = max(b["size"][0], b["size"][2])
        place(module_id, (2 * x + 1, 0, 2 * z + 1), yaw, min(1.0, 1.8 / size), role="prop")

    # --- Checks the contract asks for -----------------------------------
    def world_boxes(inst):
        r = rot_y(inst["yaw"])
        s = inst["scale"]
        for box in catalog[inst["module"]]["collision"]["boxes"]:
            c = apply(r, box["center"])
            c = [c[i] * s + inst["pos"][i] for i in range(3)]
            half = [
                sum(abs(r[i][k]) * box["size"][k] for k in range(3)) * s / 2 for i in range(3)
            ]
            yield [c[i] - half[i] for i in range(3)], [c[i] + half[i] for i in range(3)]

    edges = set()
    for x, z in walkable:
        for dx, dz in DIRS:
            n = (x + dx, z + dz)
            if n in walkable:
                edges.add(((x, z), n))
    blockers = [i for i in instances if i["role"] not in ("floor", "ceiling")]
    eps = 1e-4
    for a, b in edges:
        if a > b:
            continue
        (ax, az), (bx, bz) = a, b
        lo = [min(ax, bx) * 2 + 1, 0.02, min(az, bz) * 2 + 1]
        hi = [max(ax, bx) * 2 + 1, LANE_HEADROOM, max(az, bz) * 2 + 1]
        axis = 2 if ax == bx else 0
        across = 0 if axis == 2 else 2
        mid = (lo[across] + hi[across]) / 2
        lo[across], hi[across] = mid - LANE_WIDTH / 2, mid + LANE_WIDTH / 2
        for inst in blockers:
            for blo, bhi in world_boxes(inst):
                hit = all(bhi[i] > lo[i] + eps and blo[i] < hi[i] - eps for i in range(3))
                assert not hit, f"{inst['module']} at {inst['pos']} blocks the lane {a}-{b}"
    seen, todo = {start}, [start]
    while todo:
        p = todo.pop()
        for q in [bb for aa, bb in edges if aa == p]:
            if q not in seen:
                seen.add(q)
                todo.append(q)
    assert seen == walkable, f"unreachable: {sorted(walkable - seen)}"
    masks = [[0] * width for _ in range(height)]
    for x, z in walkable:
        for (dx, dz), bit in DIRS.items():
            n = (x + dx, z + dz)
            if n in walkable and ((x, z), n) not in edges:
                masks[z][x] |= bit
    for x, z in walkable:  # symmetric, as the core's Map::block makes them
        for (dx, dz), bit in DIRS.items():
            n = (x + dx, z + dz)
            if n in walkable:
                back = DIRS[(-dx, -dz)]
                assert bool(masks[z][x] & bit) == bool(masks[n[1]][n[0]] & back)

    # --- The core's map ---------------------------------------------------
    map_rows = []
    for z in range(height):
        line = ""
        for x in range(width):
            c = char(x, z)
            line += c if c in ".S>" else "#"
        map_rows.append(line)
    map_out = ROOT / f"crates/zrpg_core/src/dream/{theme}.map"
    map_out.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        f"# {spec['title']}: a dreamscape floor. Generated by",
        f"# tools/make_dreamscape.py from the {theme} kit; do not edit.",
        "#",
        "# Map: '#' solid (rock, or an exhibit you can see but not enter),",
        "# '.' floor, 'S' start (facing north), '>' way down. Then one hex",
        "# digit per cell: edges blocked between walkable cells (1 N, 2 E,",
        "# 4 S, 8 W).",
        f"{width} {height}",
    ]
    lines += map_rows
    lines += ["".join(format(m, "x") for m in r) for r in masks]
    map_out.write_text("\n".join(lines) + "\n")

    # --- Meshes, instances and lights for the renderer ------------------
    module_ids = sorted({i["module"] for i in instances})
    out = bytearray(b"ZRL1")
    out += struct.pack("<I", len(module_ids))
    tris = {}
    for module_id in module_ids:
        prims = load_module(module_id, catalog)
        out += struct.pack("<I", len(prims))
        tris[module_id] = 0
        for p in prims:
            assert max(p["idx"]) < 65536
            out += struct.pack("<II", p["role"], len(p["pos"]))
            for vs in (p["pos"], p["nrm"], p["uv"]):
                for v in vs:
                    out += struct.pack(f"<{len(v)}f", *v)
            out += struct.pack("<I", len(p["idx"]))
            out += struct.pack(f"<{len(p['idx'])}H", *p["idx"])
            if len(p["idx"]) % 2:
                out += b"\0\0"
            tris[module_id] += len(p["idx"]) // 3
    out += struct.pack("<I", len(instances))
    for inst in instances:
        r = rot_y(inst["yaw"])
        s = inst["scale"]
        cols = [r[row][col] * s for col in range(3) for row in range(3)]
        out += struct.pack("<I12f", module_ids.index(inst["module"]), *cols, *inst["pos"])
    glows = [
        ((2 * x + 1, spec["glows"][letter][0], 2 * z + 1), *spec["glows"][letter][1:])
        for (x, z), letter in sorted(props.items())
        if letter in spec["glows"]
    ]
    out += struct.pack("<I", len(glows))
    for pos, rgb, lumens in glows:
        out += struct.pack("<7f", *pos, *rgb, lumens)

    out_dir = ROOT / f"crates/zrpg/assets/dream/{theme}"
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "level.bin").write_bytes(out)
    for role in ROLES:
        source = KIT / manifest["material_bindings"][role]["basecolor"]
        shutil.copyfile(source, out_dir / f"{role}.png")

    total = sum(tris[i["module"]] for i in instances)
    print(f"{spec['title']}: {len(module_ids)} modules, {len(instances)} instances, {total} triangles")
    print(f"{len(walkable)} walkable cells, {len(edges) // 2} edges, {len(props)} exhibits, {len(glows)} lights")
    print(f"start {start}, way down {goal}; level.bin {len(out)} bytes")


def main():
    themes = sys.argv[1:] or list(LAYOUTS)
    for theme in themes:
        if theme not in LAYOUTS:
            sys.exit(f"no layout for {theme!r} yet (have: {', '.join(LAYOUTS)})")
        build(theme)


if __name__ == "__main__":
    main()
