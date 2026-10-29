#!/usr/bin/env python3
"""Build the dream floors: patchworks of rooms from contrasting Dreamscape
kits, joined by doorways.

Reads the extracted kit (run tools/extract_dreamscape_assets.py first):

    art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/
        common/geometry/geometry-manifest.json   module bounds, pivots, guides
        common/geometry/meshes/<module>.glb      the shared module meshes
        assembly-src/theme_recipes.py            each theme's floor, wall,
                                                 portal and props (read as
                                                 data, never run)
        themes/<theme>/theme-manifest.json       its four material bindings

and writes:

    crates/zrpg/assets/dream/patchwork/modules.bin   every module used, once
                                                     per material remap
    crates/zrpg/assets/dream/patchwork/floor_NN.bin  one floor: instances,
                                                     lights, each cell's theme
    crates/zrpg/assets/dream/themes/<theme>/*.png    four role textures (the
                                                     kit's 128 px variants)
    crates/zrpg/src/dream_pool.rs                    generated: the pool's
                                                     themes, looks and files
    crates/zrpg_core/src/dream/patchwork/floor_NN.map  walkable cells
    crates/zrpg_core/src/dream/patchwork.rs          generated: the maps

How a floor dreams:

1. Rooms from different kits, through ordinary doors: a chain of rooms, each
   from a theme unlike its neighbours, joined by short corridors with each
   room's own portal at its door.
2. Thresholds: every cell records its theme, so the game can blend haze and
   light as you cross from one to the next.
3. Strays: now and then one exhibit in a room is from another theme
   entirely, with its own materials.
4. A room that repeats: on longer floors, a room you passed comes back
   later, the same, but with one thing changed.
5. A different dream each run: FLOORS floors are built, all validated; the
   game picks among them by the run's seed.

The kit's contract (common/spec/COLLISION_AND_IMPORT.md) is kept as in
tools/make_dreamscape.py: metres, 2 m cells, 0.5 to game once; pivots and
sizes as authored (exhibits only ever shrunk to fit their pad); roles bound
by name; walkability from the layout, never triangles. Material overrides
follow the kit's own assembler: a bark, leaf, stone or water module takes
the role whose surface describes it best, and its UVs are rebound for the
new role (metric planar /2 for wall and ground, each face island 0-1 for
structure and detail). Every floor is checked before anything is written:
the 1.2 m x 2.4 m lane is clear of every collision guide on every walkable
edge (doorways open), edges are symmetric, every walkable cell is reachable
from the start, sentries stand on walkable cells.

Standard library only.
"""
from __future__ import annotations

import ast
import json
import math
import random
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_ruins import accessor, read_glb  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
KIT = ROOT / "art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials"
GEO = KIT / "common/geometry"
ASSETS = ROOT / "crates/zrpg/assets/dream"
CORE = ROOT / "crates/zrpg_core/src/dream"

ROLES = ["wall", "ground", "structure", "detail"]
DIRS = {(0, -1): 1, (1, 0): 2, (0, 1): 4, (-1, 0): 8}  # N E S W bits
LANE_WIDTH = 1.2
LANE_HEADROOM = 2.4
# Room walls meet the coffered ceiling's underside here; shorter walls get a
# plain wall of the theme's own surface behind them.
CEILING_UNDERSIDE = 3.84
# The most a wall may stand into its cell (half the cell less half the lane).
WALL_FRONT_LIMIT = 0.4
# Exhibits fit a pad this wide and stay under the ceiling.
PAD = 1.8
PROP_HEIGHT_LIMIT = 3.6

FLOORS = 12
SEED = 0x5EED_D2EA
# A floor drawing more triangles than this is drawn again: modest hardware
# first (the ruins are 39k, the sanctum 14k).
TRIANGLE_BUDGET = 45_000


class OverBudget(Exception):
    pass

# The pool: contrasting kits whose walls, doors and floors fit the grid.
POOL = [
    "mindglass-sanctum",
    "tatami-house",
    "sunflower-sanctuary",
    "lost-toy-world",
    "clockwork",
    "enchanted-forest",
    "embroidered-world",
    "aquatic-living-laboratory",
    "alabaster-temple",
    "golden-plains",
    "tavern-inn",
    "crypt-catacombs",
]
# The toy world's own arch is a small prop with no passable opening; its
# showcase uses the timber gate, and so does this.
PORTAL_SWAP = {"toy_block_arch": "timber_post_beam_gate"}
CEILING = "ceiling_coffer_2m"
BACKER = "masonry_wall_2x4"

# Room templates: rows of cells, '.' walkable, 'p' an exhibit pad (blocked,
# open to the eye), 'e' walkable where a sentry may stand. Every template
# has odd sides so its doors sit on a middle row or column.
ROOMS = {
    "gallery": ["p...p", ".....", "..e..", ".....", "p...p"],
    "hall": ["p.....p", "..p.p..", "...e...", "..p.p..", "p.....p"],
    "garden": [".p.p.", ".....", "p.p.p", "..e..", ".p.p."],
    "chapel": ["p.p", "...", ".e.", "...", "p.p"],
    "court": ["p.....p", ".......", "..p.p..", "...e...", "..p.p..", ".......", "p.....p"],
}
CORRIDOR = 2


def rot_y(deg):
    a = math.radians(deg)
    c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def apply(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def recipes():
    """The kit's theme recipes, read as data (the file is a list of add()
    calls of literals), so nothing from the kit is executed."""
    src = (KIT / "assembly-src/theme_recipes.py").read_text()
    out = {}
    for node in ast.walk(ast.parse(src)):
        if isinstance(node, ast.Call) and getattr(node.func, "id", "") == "add":
            a = [ast.literal_eval(x) for x in node.args]
            kw = {k.arg: ast.literal_eval(k.value) for k in node.keywords}
            out[kw.get("public", a[0])] = dict(
                title=a[1], family=a[2], floor=a[3], wall=a[4], portal=a[5], props=a[6].split()
            )
    return out


def role_remap(module_id, descriptions):
    """The kit assembler's rule: bark, leaf, stone and water modules take
    the non-ground role whose surface description fits them best (leaves
    only ever the detail role)."""
    kind = None
    if any(k in module_id for k in ["tree", "trunk", "root_", "fallen_log"]):
        kind = "bark"
    elif any(k in module_id for k in ["fern", "broadleaf", "grass", "wheat", "cattail", "sunflower", "cactus"]):
        kind = "leaf"
    elif any(
        k in module_id
        for k in ["boulder", "rock_", "cave_", "crag", "cliff", "basalt", "standing_stone", "stalag"]
    ):
        kind = "stone"
    elif "water_surface" in module_id:
        kind = "water"
    keys = {
        "bark": ["bark", "timber", "wood", "root"],
        "leaf": ["leaf", "leaves", "fern", "grass", "foliage", "moss"],
        "stone": ["stone", "rock", "basalt", "mineral", "slate", "limestone", "masonry"],
        "water": ["water", "ripples", "ripple"],
    }
    mapping = {}
    if kind:
        score = {
            r: sum(k in d.lower() for k in keys[kind]) for r, d in descriptions.items() if r != "ground"
        }
        best = max(score, key=score.get)
        if score[best] > 0:
            for r in ["wall", "structure", "detail"]:
                if kind == "leaf" and r != "detail":
                    continue
                if r != best:
                    mapping[r] = best
    return mapping


def rebind_uv(prim, src, dst):
    """New UVs for a primitive whose role changed, as the kit's assembler
    does: metric planar by dominant normal / 2 for wall and ground, each
    disconnected island normalised to 0-1 for structure and detail."""
    pos, nrm, uv, idx = prim["pos"], prim["nrm"], prim["uv"], prim["idx"]
    if src == dst:
        return list(uv)
    if dst in ("wall", "ground"):
        dims = [(2, 1), (0, 2), (0, 1)]
        out = []
        for p, n in zip(pos, nrm):
            axis = max(range(3), key=lambda a: abs(n[a]))
            a, b = dims[axis]
            out.append((p[a] / 2, p[b] / 2))
        return out
    parent = list(range(len(pos)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    for t in range(0, len(idx), 3):
        a = find(idx[t])
        for b in idx[t + 1 : t + 3]:
            parent[find(b)] = a
    groups = {}
    for v in range(len(pos)):
        groups.setdefault(find(v), []).append(v)
    out = list(uv)
    for verts in groups.values():
        us = [uv[v][0] for v in verts]
        vs = [uv[v][1] for v in verts]
        lo_u, lo_v = min(us), min(vs)
        su, sv = max(us) - lo_u, max(vs) - lo_v
        for v in verts:
            out[v] = (
                (uv[v][0] - lo_u) / su if su > 1e-9 else 0.0,
                (uv[v][1] - lo_v) / sv if sv > 1e-9 else 0.0,
            )
    return out


class Library:
    """Module meshes, each once per material remap, with roles applied."""

    def __init__(self, catalog):
        self.catalog = catalog
        self.raw = {}
        self.keys = []
        self.entries = {}

    def raw_prims(self, module_id):
        if module_id not in self.raw:
            gltf, binary = read_glb(GEO / self.catalog[module_id]["files"]["glb"])
            for node in gltf["nodes"]:
                assert not any(
                    k in node for k in ("matrix", "translation", "rotation", "scale")
                ), (module_id, "unbaked node transform")
            prims = []
            for mesh in gltf["meshes"]:
                for p in mesh["primitives"]:
                    assert p.get("mode", 4) == 4
                    role = gltf["materials"][p["material"]]["name"]
                    assert role in ROLES, (module_id, role)
                    prims.append(
                        dict(
                            role=role,
                            pos=accessor(gltf, binary, p["attributes"]["POSITION"]),
                            nrm=accessor(gltf, binary, p["attributes"]["NORMAL"]),
                            uv=accessor(gltf, binary, p["attributes"]["TEXCOORD_0"]),
                            idx=[i[0] for i in accessor(gltf, binary, p["indices"])],
                        )
                    )
            self.raw[module_id] = prims
        return self.raw[module_id]

    def entry(self, module_id, remap):
        key = (module_id, tuple(sorted(remap.items())))
        if key not in self.entries:
            self.entries[key] = len(self.keys)
            self.keys.append(key)
        return self.entries[key]

    def write(self, path):
        out = bytearray(b"ZRM1")
        out += struct.pack("<I", len(self.keys))
        for module_id, remap in self.keys:
            remap = dict(remap)
            prims = self.raw_prims(module_id)
            out += struct.pack("<I", len(prims))
            for p in prims:
                role = remap.get(p["role"], p["role"])
                uv = rebind_uv(p, p["role"], role)
                assert max(p["idx"]) < 65536
                out += struct.pack("<II", ROLES.index(role), len(p["pos"]))
                for vs in (p["pos"], p["nrm"], uv):
                    for v in vs:
                        out += struct.pack(f"<{len(v)}f", *v)
                out += struct.pack("<I", len(p["idx"]))
                out += struct.pack(f"<{len(p['idx'])}H", *p["idx"])
                if len(p["idx"]) % 2:
                    out += b"\0\0"
        path.write_bytes(out)
        return len(out)


def mean_colour(png_path):
    """The average colour of a PNG (8-bit RGB or RGBA, no interlace), read
    with zlib so no imaging library is needed."""
    data = png_path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    at, idat, width, height, kind = 8, b"", 0, 0, 2
    while at < len(data):
        length, tag = struct.unpack(">I4s", data[at : at + 8])
        body = data[at + 8 : at + 8 + length]
        if tag == b"IHDR":
            width, height, depth, kind, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and kind in (2, 6) and interlace == 0, png_path
        elif tag == b"IDAT":
            idat += body
        at += 12 + length
    channels = 3 if kind == 2 else 4
    raw = zlib.decompress(idat)
    stride = width * channels
    prev = bytearray(stride)
    total = [0, 0, 0]
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = prev[x]
            c = prev[x - channels] if x >= channels else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                pred = a if pa <= pb and pa <= pc else b if pb <= pc else c
                line[x] = (line[x] + pred) & 255
        for x in range(0, stride, channels):
            for k in range(3):
                total[k] += line[x + k]
        prev = line
    n = width * height
    return [t / n / 255 for t in total]


def srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


class Theme:
    def __init__(self, theme_id, recipe, catalog):
        manifest = json.loads((KIT / f"themes/{theme_id}/theme-manifest.json").read_text())
        b = manifest["material_bindings"]
        self.id = theme_id
        self.title = recipe["title"]
        self.family = recipe["family"]
        self.floor = recipe["floor"]
        self.wall = recipe["wall"]
        self.portal = PORTAL_SWAP.get(recipe["portal"], recipe["portal"])
        self.props = recipe["props"]
        self.descriptions = {r: b[r]["description"] for r in ROLES}
        self.textures = {r: KIT / b[r]["low_memory_basecolor"] for r in ROLES}
        wb = catalog[self.wall]["bounds_m"]
        # A wall standing too far into its cell is set back so the lane
        # stays clear; one too short for the ceiling is backed.
        self.wall_setback = max(0.0, -wb["min"][2] - WALL_FRONT_LIMIT)
        self.wall_backed = wb["max"][1] < CEILING_UNDERSIDE
        self.wall_overhangs = wb["min"][0] < -1.0 - 1e-6 or wb["max"][0] > 1.0 + 1e-6
        assert catalog[self.portal]["collision"]["clear_openings"], (theme_id, self.portal)
        wall = mean_colour(self.textures["wall"])
        ground = mean_colour(self.textures["ground"])
        detail = mean_colour(self.textures["detail"])
        lin = lambda c: [srgb_to_linear(x) for x in c]  # noqa: E731
        # The haze: the walls' own colour, deep in shadow.
        self.fog = [min(1.0, x * 0.22 + 0.01) for x in wall]
        # Ambient light: the floor's colour, lifted toward white.
        g = lin(ground)
        top = max(g) or 1.0
        self.ambient = [0.55 + 0.45 * x / top for x in g]
        # Distance drains colour toward the walls' hue.
        w = lin(wall)
        mid = sum(w) / 3 or 1.0
        self.tint = [min(1.25, max(0.75, x / mid)) for x in w]
        # Exhibit light: the detail colour, brightened and saturated.
        d = lin(detail)
        top = max(d) or 1.0
        self.glow = [0.35 + 0.65 * (x / top) ** 1.5 for x in d]


def make_floor(rng, themes, number):
    """One patchwork floor: rooms chained by corridors, each room a theme
    unlike the one before."""
    templates = list(ROOMS)
    count = rng.choice([4, 5, 5, 6])
    placed = []  # (x0, z0, rows, theme, template, door cells)
    corridors = []  # list of (cells, theme_from, theme_to)
    occupied = set()

    def room_cells(x0, z0, rows):
        return {(x0 + x, z0 + z) for z in range(len(rows)) for x in range(len(rows[0]))}

    def near(cell):
        x, z = cell
        return any((x + dx, z + dz) in occupied for dx in (-1, 0, 1) for dz in (-1, 0, 1))

    def free(cells, corr):
        # A new room keeps a cell of rock from everything; its corridor may
        # touch only the room it leaves (at its first cell).
        return (
            not any(near(c) for c in cells)
            and not any(c in occupied for c in corr)
            and not any(near(c) for c in corr[1:])
        )

    theme_order = []
    for i in range(count):
        choices = [t for t in themes if not theme_order or t.id != theme_order[-1].id]
        if len(theme_order) >= 2:
            choices = [t for t in choices if t.id != theme_order[-2].id]
        theme_order.append(rng.choice(choices))
    # A room that comes back, one thing changed: on longer floors, the
    # second-to-last room repeats one passed before (not the one just
    # before it).
    repeat = None
    if count >= 5:
        repeat = (count - 2, rng.randrange(0, count - 3))
    rooms_meta = []
    for i in range(count):
        template = rng.choice(templates)
        if repeat and i == repeat[0]:
            template = rooms_meta[repeat[1]]["template"]
            theme_order[i] = theme_order[repeat[1]]
        rows = ROOMS[template]
        w, h = len(rows[0]), len(rows)
        if i == 0:
            x0, z0, entry = 0, 0, None
            cells = room_cells(x0, z0, rows)
        else:
            prev = placed[-1]
            px0, pz0, prows = prev[0], prev[1], prev[2]
            pw, ph = len(prows[0]), len(prows)
            options = ["N", "E", "W"]
            rng.shuffle(options)
            for d in options:
                if d == "N":
                    door_a = (px0 + pw // 2, pz0)
                    corr = [(door_a[0], door_a[1] - k) for k in range(1, CORRIDOR + 1)]
                    x0, z0 = door_a[0] - w // 2, corr[-1][1] - h
                    entry = (x0 + w // 2, z0 + h - 1)
                elif d == "E":
                    door_a = (px0 + pw - 1, pz0 + ph // 2)
                    corr = [(door_a[0] + k, door_a[1]) for k in range(1, CORRIDOR + 1)]
                    x0, z0 = corr[-1][0] + 1, door_a[1] - h // 2
                    entry = (x0, z0 + h // 2)
                else:
                    door_a = (px0, pz0 + ph // 2)
                    corr = [(door_a[0] - k, door_a[1]) for k in range(1, CORRIDOR + 1)]
                    x0, z0 = corr[-1][0] - w, door_a[1] - h // 2
                    entry = (x0 + w - 1, z0 + h // 2)
                cells = room_cells(x0, z0, rows)
                if free(cells, corr):
                    break
            else:
                break  # boxed in: the floor ends here
            corridors.append((corr, door_a, entry, i - 1, i))
            occupied.update(corr)
        occupied.update(cells)
        placed.append((x0, z0, rows, theme_order[i], template))
        rooms_meta.append(dict(template=template, theme=theme_order[i], x0=x0, z0=z0, entry=entry))
    if repeat and repeat[0] >= len(placed):
        repeat = None
    return placed, corridors, rooms_meta, repeat


def build_floor(number, themes, lib, catalog, rng):
    placed, corridors, meta, repeat = make_floor(rng, themes, number)
    # Normalise so the floor sits inside a one-cell border of rock.
    xs = [x for (x0, z0, rows, *_ ) in placed for x in (x0, x0 + len(rows[0]) - 1)]
    zs = [z for (x0, z0, rows, *_ ) in placed for z in (z0, z0 + len(rows) - 1)]
    for corr, *_ in corridors:
        xs += [c[0] for c in corr]
        zs += [c[1] for c in corr]
    ox, oz = 1 - min(xs), 1 - min(zs)
    width, height = max(xs) + ox + 2, max(zs) + oz + 2

    cell_theme = {}
    kind = {}  # cell -> '.', 'p', 'e'
    room_of = {}
    for r, (x0, z0, rows, theme, template) in enumerate(placed):
        for z, row in enumerate(rows):
            for x, c in enumerate(row):
                cell = (x0 + x + ox, z0 + z + oz)
                cell_theme[cell] = theme
                kind[cell] = c
                room_of[cell] = r
    doors = []
    for corr, door_a, entry, ra, rb in corridors:
        cells = [(x + ox, z + oz) for x, z in corr]
        for k, cell in enumerate(cells):
            # The first half belongs to the room behind, the rest to the
            # room ahead: the threshold is mid-corridor.
            cell_theme[cell] = placed[ra][3] if k < len(cells) // 2 else placed[rb][3]
            kind[cell] = "."
        a = (door_a[0] + ox, door_a[1] + oz)
        b = (entry[0] + ox, entry[1] + oz)
        doors.append((a, cells[0], placed[ra][3]))
        doors.append((cells[-1], b, placed[rb][3]))
        # The door cells themselves must be walkable.
        for cell in (a, b):
            if kind[cell] == "p":
                kind[cell] = "."
    open_cells = set(kind)
    walkable = {c for c, k in kind.items() if k in ".e"}
    pads = {c for c, k in kind.items() if k == "p"}
    # Keep pads off the door lanes: any pad beside a door cell, in line
    # with the door, becomes floor.
    start_room = placed[0]
    sx0, sz0, srows = start_room[0] + ox, start_room[1] + oz, start_room[2]
    start = (sx0 + len(srows[0]) // 2, sz0 + len(srows) - 1)
    if start in pads:
        pads.discard(start)
        walkable.add(start)
    last = placed[-1]
    lx0, lz0, lrows = last[0] + ox, last[1] + oz, last[2]
    goal = (lx0 + len(lrows[0]) // 2, lz0 + len(lrows) // 2)
    # The way down: the last room's middle (a pad there becomes floor).
    pads.discard(goal)
    walkable.add(goal)
    # Approach the way down from the door you came in by.
    entry = (meta[-1]["entry"][0] + ox, meta[-1]["entry"][1] + oz) if meta[-1]["entry"] else start
    dx, dz = goal[0] - entry[0], goal[1] - entry[1]
    stairs_dir = (
        "N" if abs(dz) >= abs(dx) and dz < 0 else "S" if abs(dz) >= abs(dx) else "E" if dx > 0 else "W"
    )
    sentries = sorted(c for c, k in kind.items() if k == "e" and c not in (start, goal))[:3]
    # The start room is quiet.
    sentries = [s for s in sentries if room_of.get(s) != 0]

    instances = []

    def place(module_id, pos, yaw, theme, scale=1.0, role="scenery", remap=None):
        assert module_id in catalog, module_id
        if remap is None:
            remap = role_remap(module_id, theme.descriptions)
        instances.append(
            dict(
                lib=lib.entry(module_id, remap),
                module=module_id,
                pos=list(pos),
                yaw=yaw,
                scale=scale,
                theme=theme,
                role=role,
            )
        )

    for x, z in sorted(open_cells):
        t = cell_theme[(x, z)]
        centre = (2 * x + 1, 0, 2 * z + 1)
        if (x, z) != goal:
            place(t.floor, centre, 0, t, role="floor", remap={})
        place(CEILING, centre, 0, t, role="ceiling", remap={})
    for x, z in sorted(open_cells):
        t = cell_theme[(x, z)]
        for (dx, dz), _ in DIRS.items():
            if (x + dx, z + dz) in open_cells:
                continue
            back = t.wall_setback
            if dz == 1:
                pos, yaw, out = (2 * x + 1, 0, 2 * z + 2 + back), 0, (0, 0, 1)
            elif dz == -1:
                pos, yaw, out = (2 * x + 1, 0, 2 * z - back), 180, (0, 0, -1)
            elif dx == -1:
                pos, yaw, out = (2 * x - back, 0, 2 * z + 1), -90, (-1, 0, 0)
            else:
                pos, yaw, out = (2 * x + 2 + back, 0, 2 * z + 1), 90, (1, 0, 0)
            # Beside an opening along the same wall line, a wall that
            # overhangs its 2 m bay would stand in the doorway: use the
            # theme's plain wall there instead.
            along = [(-dz, dx), (dz, -dx)]
            flanks_opening = any(
                (x + ax, z + az) in open_cells and (x + ax + dx, z + az + dz) in open_cells
                for ax, az in along
            )
            if flanks_opening and t.wall_overhangs:
                place(BACKER, pos, yaw, t, role="wall", remap={"wall": "wall"})
                continue
            place(t.wall, pos, yaw, t, role="wall")
            if t.wall_backed:
                bpos = [pos[i] + out[i] * 0.15 for i in range(3)]
                place(BACKER, bpos, yaw, t, role="wall", remap={})
    for a, b, t in doors:
        (ax, az), (bx, bz) = a, b
        if ax == bx:
            place(t.portal, (2 * ax + 1, 0, 2 * max(az, bz)), 0, t, role="door")
        else:
            place(t.portal, (2 * max(ax, bx), 0, 2 * az + 1), 90, t, role="door")

    # Exhibits. A room keeps a shuffled list of its theme's props; strays
    # come from another theme. The repeated room copies its original's
    # exhibits, but one of them is changed.
    pads_by_room = {}
    for c in sorted(pads):
        pads_by_room.setdefault(room_of[c], []).append(c)
    exhibits = {}
    strays = []
    others = lambda t: [o for o in themes if o.id != t.id]  # noqa: E731
    for r, cells in sorted(pads_by_room.items()):
        theme = placed[r][3]
        order = theme.props[:]
        rng.shuffle(order)
        chosen = [(theme, order[k % len(order)], rng.choice([0, 15, -15, 90, 180])) for k in range(len(cells))]
        if rng.random() < 0.45 and cells:
            k = rng.randrange(len(cells))
            other = rng.choice(others(theme))
            chosen[k] = (other, rng.choice(other.props), rng.choice([0, 30, -30]))
            strays.append((cells[k], theme.id, other.id, chosen[k][1]))
        exhibits[r] = list(zip(cells, chosen))
    change = None
    if repeat:
        src, dst = repeat[1], repeat[0]
        if src in exhibits and dst in exhibits and len(exhibits[src]) == len(exhibits[dst]):
            copied = [(dcell, choice) for (dcell, _), (_, choice) in zip(exhibits[dst], exhibits[src])]
            k = rng.randrange(len(copied))
            cell, (t, module_id, yaw) = copied[k]
            # One thing changed: the exhibit turned around, or swapped for
            # one from another theme.
            if rng.random() < 0.5:
                copied[k] = (cell, (t, module_id, yaw + 180))
                change = (cell, f"{module_id} turned around")
            else:
                other = rng.choice(others(t))
                new = rng.choice(other.props)
                copied[k] = (cell, (other, new, yaw))
                change = (cell, f"{module_id} became {other.id}'s {new}")
            exhibits[dst] = copied
    glows = []
    for r, items in exhibits.items():
        lit = 0
        for (x, z), (t, module_id, yaw) in items:
            b = catalog[module_id]["bounds_m"]
            footprint = max(b["size"][0], b["size"][2])
            s = min(1.0, PAD / footprint, PROP_HEIGHT_LIMIT / max(b["size"][1], 1e-3))
            centre_x = (b["min"][0] + b["max"][0]) / 2
            centre_z = (b["min"][2] + b["max"][2]) / 2
            r_ = rot_y(yaw)
            off = apply(r_, [centre_x, 0, centre_z])
            pos = (2 * x + 1 - off[0] * s, -b["min"][1] * s, 2 * z + 1 - off[2] * s)
            place(module_id, pos, yaw, t, scale=s, role="prop")
            if lit < 2 and b["size"][1] * s > 1.0:
                glows.append(((2 * x + 1, min(2.6, b["size"][1] * s + 0.4), 2 * z + 1), t.glow, 4200.0))
                lit += 1

    tris = sum(
        len(p["idx"]) // 3 for inst in instances for p in lib.raw_prims(inst["module"])
    )
    if tris > TRIANGLE_BUDGET:
        raise OverBudget(tris)

    # --- Checks ----------------------------------------------------------
    def world_boxes(inst):
        r_ = rot_y(inst["yaw"])
        s = inst["scale"]
        for box in catalog[inst["module"]]["collision"]["boxes"]:
            c = apply(r_, box["center"])
            c = [c[i] * s + inst["pos"][i] for i in range(3)]
            half = [sum(abs(r_[i][k]) * box["size"][k] for k in range(3)) * s / 2 for i in range(3)]
            yield [c[i] - half[i] for i in range(3)], [c[i] + half[i] for i in range(3)]

    edges = set()
    for x, z in walkable:
        for dx, dz in DIRS:
            n = (x + dx, z + dz)
            if n in walkable:
                edges.add(((x, z), n))
    blockers = [i for i in instances if i["role"] not in ("floor", "ceiling")]
    boxes = [(i, lo, hi) for i in blockers for lo, hi in world_boxes(i)]
    eps = 1e-4
    for a, b in edges:
        if a > b:
            continue
        (ax, az), (bx, bz) = a, b
        lo = [min(ax, bx) * 2 + 1, 0.02, min(az, bz) * 2 + 1]
        hi = [max(ax, bx) * 2 + 1, LANE_HEADROOM, max(az, bz) * 2 + 1]
        across = 0 if ax == bx else 2
        mid = (lo[across] + hi[across]) / 2
        lo[across], hi[across] = mid - LANE_WIDTH / 2, mid + LANE_WIDTH / 2
        for inst, blo, bhi in boxes:
            hit = all(bhi[i] > lo[i] + eps and blo[i] < hi[i] - eps for i in range(3))
            assert not hit, f"floor {number}: {inst['module']} at {inst['pos']} blocks {a}-{b}"
    seen, todo = {start}, [start]
    while todo:
        p = todo.pop()
        for d in DIRS:
            q = (p[0] + d[0], p[1] + d[1])
            if (p, q) in edges and q not in seen:
                seen.add(q)
                todo.append(q)
    assert seen == walkable, f"floor {number}: unreachable {sorted(walkable - seen)}"
    for s in sentries:
        assert s in walkable

    # --- The core's map ----------------------------------------------------
    rows = []
    for z in range(height):
        line = ""
        for x in range(width):
            c = (x, z)
            if c == start:
                line += "S"
            elif c == goal:
                line += ">"
            elif c in sentries:
                line += "e"
            elif c in walkable:
                line += "."
            else:
                line += "#"
        rows.append(line)
    masks = ["0" * width for _ in range(height)]
    themes_used = sorted({cell_theme[c].id for c in open_cells}, key=[t.id for t in themes].index)
    lines = [
        f"# Dream floor {number}: a patchwork of {', '.join(themes_used)}.",
        "# Generated by tools/make_patchwork.py from the Dreamscape kits; do",
        "# not edit. Map: '#' solid (rock, or an exhibit you can see but not",
        "# enter), '.' floor, 'e' floor where a sentry waits, 'S' start, '>'",
        "# way down; then hex edge masks (all open here).",
        f"{width} {height}",
    ]
    (CORE / f"patchwork/floor_{number:02d}.map").write_text("\n".join(lines + rows + masks) + "\n")
    start_facing = "N"

    # --- The renderer's floor ---------------------------------------------
    index = {t.id: k for k, t in enumerate(themes)}
    out = bytearray(b"ZRV1")
    out += struct.pack("<I", len(instances))
    for inst in instances:
        r_ = rot_y(inst["yaw"])
        s = inst["scale"]
        cols = [r_[row][col] * s for col in range(3) for row in range(3)]
        out += struct.pack("<II12f", inst["lib"], index[inst["theme"].id], *cols, *inst["pos"])
    out += struct.pack("<I", len(glows))
    for pos, rgb, lumens in glows:
        out += struct.pack("<7f", *pos, *rgb, lumens)
    out += struct.pack("<II", width, height)
    for z in range(height):
        for x in range(width):
            t = cell_theme.get((x, z))
            out += bytes([index[t.id] if t else 255])
    (ASSETS / f"patchwork/floor_{number:02d}.bin").write_bytes(out)
    return dict(
        number=number,
        size=(width, height),
        rooms=[(m["template"], m["theme"].id) for m in meta[: len(placed)]],
        instances=len(instances),
        walkable=len(walkable),
        edges=len(edges) // 2,
        glows=len(glows),
        strays=strays,
        repeat=(repeat, change),
        start=start,
        start_facing=start_facing,
        goal=goal,
        stairs_dir=stairs_dir,
        sentries=sentries,
        tris=tris,
    )


def rust_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    catalog = {a["id"]: a for a in json.loads((GEO / "geometry-manifest.json").read_text())["assets"]}
    rec = recipes()
    themes = [Theme(t, rec[t], catalog) for t in POOL]
    lib = Library(catalog)
    (ASSETS / "patchwork").mkdir(parents=True, exist_ok=True)
    (CORE / "patchwork").mkdir(parents=True, exist_ok=True)
    rng = random.Random(SEED)
    floors = []
    for n in range(FLOORS):
        while True:
            try:
                floors.append(build_floor(n, themes, lib, catalog, rng))
                break
            except OverBudget as e:
                print(f"floor {n:02d}: {e.args[0]} triangles, over budget; drawing again")
    size = lib.write(ASSETS / "patchwork/modules.bin")
    for t in themes:
        d = ASSETS / f"themes/{t.id}"
        d.mkdir(parents=True, exist_ok=True)
        for role in ROLES:
            shutil.copyfile(t.textures[role], d / f"{role}.png")

    # Generated Rust: the pool for the renderer, the maps for the core.
    f3 = lambda v: "[" + ", ".join(f"{x:.4f}" for x in v) + "]"  # noqa: E731
    src = [
        "//! The dream floors' pool and files. Generated by tools/make_patchwork.py;",
        "//! do not edit.",
        "",
        "/// One theme in the pool: its four role textures (wall, ground,",
        "/// structure, detail) and the look of its rooms, derived from them.",
        "pub struct DreamTheme {",
        "    pub id: &'static str,",
        "    pub textures: [&'static [u8]; 4],",
        "    /// Haze colour (sRGB), ambient light (linear) and the colour",
        "    /// distance drains toward. (Its exhibits' light is in each floor.)",
        "    pub fog: [f32; 3],",
        "    pub ambient: [f32; 3],",
        "    pub tint: [f32; 3],",
        "}",
        "",
        f"pub const THEMES: [DreamTheme; {len(themes)}] = [",
    ]
    for t in themes:
        tex = ", ".join(
            f'include_bytes!("../assets/dream/themes/{t.id}/{r}.png")' for r in ROLES
        )
        src += [
            "    DreamTheme {",
            f"        id: {rust_str(t.id)},",
            f"        textures: [{tex}],",
            f"        fog: {f3(t.fog)},",
            f"        ambient: {f3(t.ambient)},",
            f"        tint: {f3(t.tint)},",
            "    },",
        ]
    src += [
        "];",
        "",
        "/// Every module, once per material remap.",
        'pub const MODULES: &[u8] = include_bytes!("../assets/dream/patchwork/modules.bin");',
        "",
        "/// The floors, in the core's order (`zrpg_core::dream::patchwork`).",
        f"pub const FLOORS: [&[u8]; {len(floors)}] = [",
    ]
    src += [f'    include_bytes!("../assets/dream/patchwork/floor_{f["number"]:02d}.bin"),' for f in floors]
    src += ["];", ""]
    (ROOT / "crates/zrpg/src/dream_pool.rs").write_text("\n".join(src))

    core = [
        "//! The dream floors' maps. Generated by tools/make_patchwork.py; do not",
        "//! edit.",
        "",
        "use crate::grid::Facing;",
        "",
        "/// One floor: its map, and which way the way down is approached.",
        "pub struct Patchwork {",
        "    pub map: &'static str,",
        "    pub stairs_dir: Facing,",
        "}",
        "",
        f"pub const FLOORS: [Patchwork; {len(floors)}] = [",
    ]
    names = {"N": "North", "E": "East", "S": "South", "W": "West"}
    for f in floors:
        core += [
            "    Patchwork {",
            f'        map: include_str!("patchwork/floor_{f["number"]:02d}.map"),',
            f"        stairs_dir: Facing::{names[f['stairs_dir']]},",
            "    },",
        ]
    core += ["];", ""]
    (CORE / "patchwork.rs").write_text("\n".join(core))
    # Keep the generated Rust as `cargo fmt` would have it, if rustfmt is
    # installed (it is with any Rust toolchain).
    generated = [ROOT / "crates/zrpg/src/dream_pool.rs", CORE / "patchwork.rs"]
    if shutil.which("rustfmt"):
        subprocess.run(["rustfmt", "--edition", "2024", *map(str, generated)], check=True)

    for f in floors:
        rooms = " > ".join(f"{t}({r})" for r, t in f["rooms"])
        print(
            f"floor {f['number']:02d} {f['size'][0]}x{f['size'][1]}: {rooms}; "
            f"{f['walkable']} walkable, {f['instances']} instances, {f['tris']} triangles, {f['glows']} lights, "
            f"{len(f['sentries'])} sentries"
        )
        for cell, home, other, prop in f["strays"]:
            print(f"   stray: {other}'s {prop} in the {home} room at {cell}")
        (rep, change) = f["repeat"]
        if rep and change:
            print(f"   repeat: room {rep[0]} is room {rep[1]} again, but {change[1]}")
    print(f"modules.bin {size} bytes, {len(lib.keys)} module variants")


if __name__ == "__main__":
    main()
