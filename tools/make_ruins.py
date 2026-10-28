#!/usr/bin/env python3
"""Convert the sandstone ruins kit into the game's embedded formats.

Reads the extracted kit (run tools/extract_asset_kits.py first):

    art/kits/sandstone-ruins-v1/meshes/ruins_*.glb   the eleven modules
    art/kits/sandstone-ruins-v1/spec/level_layout.json

and writes:

    crates/zrpg/assets/ruins/level.bin   module meshes + instance transforms
    crates/zrpg/assets/ruins/*.png       the runtime 256 px basecolor maps
    crates/zrpg_core/src/ruins.map       walkable cells + forbidden edges

The kit's integration guide asks for the modules + layout route *or* the
assembled scene, never both. This uses modules + layout; the assembled scene
is only read to check that every transform here matches it.

Coordinates stay in the kit's metres; the game scales by 0.5 at load so one
2 m kit cell is one game cell. The kit's cell (x, z) is the core's
GridPos(x, y), and north is -Z in both.
"""
from __future__ import annotations

import json
import math
import shutil
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "art/kits/sandstone-ruins-v1"
OUT = ROOT / "crates/zrpg/assets/ruins"
MAP_OUT = ROOT / "crates/zrpg_core/src/ruins.map"

# Material slots, in the order the game binds them (see ruins.rs).
MATERIALS = ["M_Sandstone", "M_Floor", "M_Carved", "M_Rubble", "M_Water"]
# The guide's basecolor candidates. M_Carved uses the sandstone fallback:
# the carved sheet is a motif sheet, not a tile, and needs authored UVs.
TEXTURES = {
    "sandstone.png": "sandstone_blocks_basecolor.png",
    "floor.png": "worn_floor_basecolor.png",
    "rubble.png": "rubble_stone_basecolor.png",
    "water.png": "water_basecolor.png",
}

# Cell kinds -> map characters. Everything not walkable is solid ('#').
CELL_CHARS = {
    "dry_floor": ".",
    "bridge_deck": "=",
    "shallow_water": "~",
    "stairs": "/",
}
# The proposed goal, facing the statue, becomes the way down.
DESCENT = "inspect_statue"
DIRS = {(0, -1): 1, (1, 0): 2, (0, 1): 4, (-1, 0): 8}  # N E S W bits


def read_glb(path: Path):
    data = path.read_bytes()
    magic, _, _ = struct.unpack_from("<4sII", data, 0)
    assert magic == b"glTF", path
    json_len, json_type = struct.unpack_from("<I4s", data, 12)
    assert json_type == b"JSON"
    gltf = json.loads(data[20 : 20 + json_len])
    off = 20 + json_len
    bin_chunk = b""
    if off < len(data):
        bin_len, bin_type = struct.unpack_from("<I4s", data, off)
        assert bin_type == b"BIN\x00"
        bin_chunk = data[off + 8 : off + 8 + bin_len]
    return gltf, bin_chunk


def accessor(gltf, binary, index):
    acc = gltf["accessors"][index]
    view = gltf["bufferViews"][acc["bufferView"]]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[acc["type"]]
    fmt = {5126: "f", 5123: "H", 5125: "I"}[acc["componentType"]]
    size = struct.calcsize(fmt)
    stride = view.get("byteStride", size * width)
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    out = []
    for i in range(acc["count"]):
        out.append(struct.unpack_from(f"<{width}{fmt}", binary, start + i * stride))
    return out


def rot_x(a):
    c, s = math.cos(a), math.sin(a)
    return [[1, 0, 0], [0, c, -s], [0, s, c]]


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def rot_z(a):
    c, s = math.cos(a), math.sin(a)
    return [[c, -s, 0], [s, c, 0], [0, 0, 1]]


def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def instance_matrix(inst):
    """Linear part (rows) of an instance: rotate X, then Y, then Z, after scale."""
    rx, ry, rz = (math.radians(v) for v in inst["rotation_degrees_xyz"])
    r = matmul(rot_z(rz), matmul(rot_y(ry), rot_x(rx)))
    sx, sy, sz = inst["scale_xyz"]
    return [[r[i][0] * sx, r[i][1] * sy, r[i][2] * sz] for i in range(3)]


def quat_matrix(q):
    x, y, z, w = q
    return [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]


def check_against_assembled(instances):
    """Every instance transform must match the kit's own assembled scene."""
    gltf, _ = read_glb(KIT / "meshes/assembled_textured.glb")
    nodes = {n["name"]: n for n in gltf["nodes"]}
    assert len(nodes) == len(instances), "instance count differs from assembled scene"
    for inst in instances:
        node = nodes[inst["instance_id"]]
        r = quat_matrix(node.get("rotation", [0, 0, 0, 1]))
        s = node.get("scale", [1, 1, 1])
        expect = [[r[i][j] * s[j] for j in range(3)] for i in range(3)]
        mine = instance_matrix(inst)
        err = max(abs(mine[i][j] - expect[i][j]) for i in range(3) for j in range(3))
        moved = max(abs(a - b) for a, b in zip(inst["position_m"], node.get("translation", [0, 0, 0])))
        assert err < 1e-4 and moved < 1e-4, f"{inst['instance_id']}: transform differs"


def main():
    layout = json.loads((KIT / "spec/level_layout.json").read_text())
    grid = layout["grid"]
    width, height = grid["width_cells"], grid["height_cells"]
    instances = layout["instances"]
    check_against_assembled(instances)

    # --- Navigation map for the core -------------------------------------
    rows = [["#"] * width for _ in range(height)]
    walkable = set()
    for cell in layout["cells"]:
        x, z = cell["cell"]
        if cell["walkable"]:
            rows[z][x] = CELL_CHARS[cell["kind"]]
            walkable.add((x, z))
    start = tuple(layout["player_assumptions"]["start_cell"])
    goal = next(o for o in layout["objectives"] if o["id"] == DESCENT)
    goal_cell = tuple(goal["cell"])
    assert start in walkable and goal_cell in walkable
    assert layout["player_assumptions"]["start_facing"] == "north"
    rows[start[1]][start[0]] = "S"
    rows[goal_cell[1]][goal_cell[0]] = ">"

    edges = set()
    for e in layout["navigation"]["edges"]:
        assert e["bidirectional"]
        a, b = tuple(e["a"]), tuple(e["b"])
        assert a in walkable and b in walkable and abs(a[0] - b[0]) + abs(a[1] - b[1]) == 1
        edges.add((a, b))
        edges.add((b, a))
    # Adjacent walkable cells without an edge (parapets, stair sides) are
    # blocked. Missing edges are forbidden, per the layout's contract.
    masks = [[0] * width for _ in range(height)]
    for x, z in walkable:
        for (dx, dz), bit in DIRS.items():
            n = (x + dx, z + dz)
            if n in walkable and ((x, z), n) not in edges:
                masks[z][x] |= bit
    # The game reaches every walkable cell from the start, like the kit says.
    seen, todo = {start}, [start]
    while todo:
        p = todo.pop()
        for q in [b for a, b in edges if a == p]:
            if q not in seen:
                seen.add(q)
                todo.append(q)
    assert seen == walkable, "unreachable cells"

    lines = [
        "# The sandstone ruins: the run's first floor. Generated by",
        "# tools/make_ruins.py from art/kits/sandstone-ruins-v1; do not edit.",
        "#",
        "# Map: '#' solid, '.' floor, '=' bridge, '~' shallow water,",
        "# '/' stairs out of the water, 'S' start (facing north), '>' way down.",
        "# Then one hex digit per cell: edges that are blocked even though both",
        "# cells are walkable (parapets, stair sides). Bits: 1 N, 2 E, 4 S, 8 W.",
        f"{width} {height}",
    ]
    lines += ["".join(r) for r in rows]
    lines += ["".join(format(m, "x") for m in r) for r in masks]
    MAP_OUT.write_text("\n".join(lines) + "\n")

    # --- Meshes and instances for the renderer --------------------------
    module_ids = sorted({i["module_id"] for i in instances})
    out = bytearray(b"ZRL1")
    out += struct.pack("<I", len(module_ids))
    triangles = {}
    for module_id in module_ids:
        gltf, binary = read_glb(KIT / f"meshes/{module_id}.glb")
        prims = [p for m in gltf["meshes"] for p in m["primitives"]]
        out += struct.pack("<I", len(prims))
        tris = 0
        for p in prims:
            assert p.get("mode", 4) == 4
            material = MATERIALS.index(gltf["materials"][p["material"]]["name"])
            pos = accessor(gltf, binary, p["attributes"]["POSITION"])
            nrm = accessor(gltf, binary, p["attributes"]["NORMAL"])
            uv = accessor(gltf, binary, p["attributes"]["TEXCOORD_0"])
            idx = [i[0] for i in accessor(gltf, binary, p["indices"])]
            assert max(idx) < 65536
            out += struct.pack("<II", material, len(pos))
            for vs in (pos, nrm, uv):
                for v in vs:
                    out += struct.pack(f"<{len(v)}f", *v)
            out += struct.pack("<I", len(idx))
            out += struct.pack(f"<{len(idx)}H", *idx)
            if len(idx) % 2:
                out += b"\0\0"
            tris += len(idx) // 3
        triangles[module_id] = tris

    # The descent replaces the floor slab at the goal cell.
    def skipped(inst):
        return inst["role"] == "walk_surface" and tuple(inst.get("cell", ())) == goal_cell

    kept = [i for i in instances if not skipped(i)]
    assert len(kept) == len(instances) - 1
    out += struct.pack("<I", len(kept))
    for inst in kept:
        m = instance_matrix(inst)
        cols = [m[r][c] for c in range(3) for r in range(3)]
        out += struct.pack("<I12f", module_ids.index(inst["module_id"]), *cols, *inst["position_m"])

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "level.bin").write_bytes(out)
    for name, source in TEXTURES.items():
        shutil.copyfile(KIT / "art/runtime/256/materials" / source, OUT / name)

    total = sum(triangles[i["module_id"]] for i in kept)
    print(f"{len(module_ids)} modules, {len(kept)} instances, {total} triangles")
    print(f"{len(walkable)} walkable cells, {len(edges) // 2} edges; level.bin {len(out)} bytes")


if __name__ == "__main__":
    main()
