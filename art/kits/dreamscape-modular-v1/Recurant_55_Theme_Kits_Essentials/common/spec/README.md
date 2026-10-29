# Modular asset contract — Recurant

**Status:** reusable asset-generation and Claude integration contract, not an implemented engine feature. The themes are visual inspirations, not copies of named source worlds. Do not reproduce source-world logos, exact layouts, recognizable characters or signature props.

## Read first

- `asset_contract.json`: normative **new-kit proposal**, plus separate observed facts.
- `geometry_manifest.schema.json`: structure shared geometry manifests should satisfy.
- `theme_manifest.schema.json`: lightweight proposed theme assembly manifest; an adapter may normalize material-worker manifests into this form.
- `COLLISION_AND_IMPORT.md`: actionable integration checklist and sandstone reuse recipe.
- `observed-sources/`: unchanged legacy sandstone documents and provenance. These describe the **old kit**, not the new four-slot kits.

The inspected branch was `ccr-0aac5f81-70amxp`, HEAD `b9fdd98f37bcb419bcf2951103529ff571e5ea21`, on 2026-10-29. Recheck HEAD before integration. No repository files were changed and no repository scripts or game builds were run for this contract.

## The new kit in one minute

| Item | Authoring convention | Game conversion |
|---|---|---|
| Units / axes | metres; right-handed; +Y up, +X east, −Z north | multiply positions and dimensions by **0.5 once** |
| Grid | 2 m square; cell `(x,z)` centre `(2x+1, y, 2z+1)` | 1 game unit per cell |
| Main wall | 2 m long, 4 m tall, 0.2 m thick; long axis X | tall ruins profile: 2 game units high |
| Optional short wall | 2 m tall, explicitly named | dungeon profile: 1 game unit high |
| Floor | 2 × 2 m; top Y=0, bottom Y=−0.2 | centre-top pivot |
| Wall / prop pivot | base centre, origin `(0,0,0)` | no auto-centering |
| Door | 2 m module; ≥1.4 m wide × 2.4 m high clear opening | visual proposal, not a controller measurement |
| Travel lane | central 1.2 m width, ≥2.4 m headroom | keep every allowed cardinal route clear |
| UVs | `wall`, `ground`: 1 repeat / 2 m; `structure`, `detail`: per-component 0–1 panels | repeat tiles; clamp panels |
| Material names | **`wall`, `ground`, `structure`, `detail`** | explicit theme binding, never positional guessing |
| Geometry | indexed triangles, normals, UV0; identity mesh-root transforms | bake transform into vertices before export |
| Collision | authoring box guides + explicit map occupancy/edges | core grid logic remains authoritative |

Floor connectors: N `(0,0,−1)`, E `(1,0,0)`, S `(0,0,1)`, W `(−1,0,0)`. Wall end connectors: `(−1,0,0)` and `(1,0,0)`. Wall edge placement at grid cell `(x,z)`: north anchor `(2x+1,0,2z)`, east `(2x+2,0,2z+1)`, south `(2x+1,0,2z+2)`, west `(2x,0,2z+1)`. East/west walls need 90° yaw. Author positive yaw using right-handed +Y rotation: local +Z turns toward +X. Decoration facing into the room must be checked separately.

**Do not shrink a 4 m tall mesh to fit the 2 m dungeon ceiling.** Choose a purpose-authored short-wall profile or update the level's ceiling/eye-height policy explicitly. A 2.4 m clear doorway does not fit beneath a 2 m ceiling.

## What was actually observed

The current game's cell is 1.0 on X/Z; ordinary dungeon walls/ceilings are height 1.0. The sandstone entrance is a separate tall profile: original 2 m cells imported at 0.5 scale and 4 m walls become height 2.0. Its eye is 1.5 authoring metres / 0.75 game units; the dungeon eye is 0.5 game units. The camera uses 60° vertical field of view, near clip 0.05 and a 0.25-game-unit backward offset from cell centre. These are observed renderer settings, not new asset requirements. See [level.rs](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg/src/level.rs), [view.rs](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg/src/view.rs), and [atmosphere.rs](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg/src/atmosphere.rs).

**Legacy thickness discrepancy:** original `module_catalog.json` specifies a 0.4 m thick sandstone wall (Z bounds ±0.2), while a comment in `ruins.rs` says 0.2 m. Do not silently rewrite the original kit or derive its bounds from that comment. New kits deliberately use 0.2 m nominal wall thickness; exact exported bounds govern relief, capitals and supports. Original doorway is 4 m wide with a 3 × 3 m clear opening, not the proposed 2 m doorway. Original stairs have a low-edge pivot and ascend −Z; never auto-centre them.

Current conversion is **not** a generic runtime GLB loader. `tools/make_ruins.py` reads mesh primitives and placements into embedded `level.bin`; `ruins.rs` merges meshes by material. The old converter recognizes only `M_Sandstone`, `M_Floor`, `M_Carved`, `M_Rubble`, `M_Water`, and copies named external PNGs. New four-slot names require an adapter. It does not apply module GLB node transforms or automatically consume embedded textures. See [converter](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/tools/make_ruins.py) and [renderer](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg/src/ruins.rs).

## Materials and low-resolution discipline

Supply 256 × 256 square PNG runtime candidates for each of the four roles. 64/128 derivatives are useful optional low-memory profiles; keep full-size generated sources outside runtime directories. Basecolor is sRGB. Optional normal and ORM maps are linear: ORM R=occlusion, G=roughness, B=metallic; tangent normals use +Y/OpenGL convention. Optional maps are not implied to exist merely because the contract permits them. The existing dungeon supports these maps, but the legacy ruins use basecolor-only. See [textures.rs](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg/src/textures.rs).

Use opaque surfaces first; alpha-masked cutouts may use cutoff 0.5 and double-sided rendering when needed. Avoid alpha-blended glass/water as the default: use an opaque stylized fluid variant until sorting/performance is tested. Emissive accents should not imply one light per prop. Default nonmetal roughness 0.85 and metallic 0; change explicitly for a metal theme. Preserve readability of silhouettes, path edges, doorway voids and landmarks at **native 320 × 240**, not just enlarged contact sheets. Test seams in a 3 × 3 repeat and during camera motion. Generated tile candidates are not certified seamless by intent alone.

GLB packaging follows glTF 2.0: use indexed TRIANGLES, float positions/normals/UVs, unsigned-short indices, and uncompressed geometry. Keep ≤65,535 vertices per primitive; no Draco, meshopt, KTX2, skins, morphs, lights or required extensions in the baseline. Bake module transforms; preserve instance transforms in separate layouts. One GLB may embed PNGs for viewing, but the manifest's external material bindings remain the adapter source of truth. Format reference: [Khronos glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html).

**Proposed budgets, not measured limits:** ordinary structure ≤1,500 triangles/module, large landmark ≤5,000; ≤4 material slots/module; aim for ≤60,000 visible environment triangles and ≤24 environment material batches in a diagnostic scene. Share geometry/material handles and batch by role where appropriate. Profile native, browser and handheld targets before calling this shipping-ready. No engine, controller, target-device or performance compatibility is claimed by asset-level QA.

## Package layout

```
modular-kits/
  common/spec/                 this contract, schemas, original source notes
  common/geometry/meshes/      reusable GLBs
  common/geometry/collision-guides/  authoring-only JSON
  common/geometry/assemblies/  example arrangements (not game levels by default)
  common/geometry/previews/    generated geometry renders
  common/geometry/src/         editable authoring source and rebuild helpers
  themes/<theme-id>/materials/ runtime PNGs and binding manifests
  themes/<theme-id>/art_sources/ original generation output
  themes/<theme-id>/prompts/   generation provenance
  themes/<theme-id>/           theme manifest, layout/recipe, integration notes
  qa/                        asset checks and limitations
```

All paths are relative to an explicitly declared manifest base. Shared geometry must be included when distributing a standalone theme. Do not count material recolours as new geometry. A theme is complete only when its recipe identifies structural modules, distinctive props/silhouettes, material bindings, placement/collision intent, an inspectable assembled example, and remaining limitations. Reuse is intentional; theme identity must come from combinations, silhouettes, props and composition as well as colour.
