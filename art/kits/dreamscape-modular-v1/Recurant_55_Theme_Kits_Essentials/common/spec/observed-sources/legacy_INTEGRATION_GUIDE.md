# Ruins level kit integration guide

Build a small playable ruin from the supplied visual reference: shaded corridor → open junction → bridge or shallow-water route → statue chamber. The kit supplies visual assets, authored blockout meshes, a proposed layout, and integration contracts. **It is not a complete game or an exact reconstruction of the pictured architecture.** All grid dimensions and hidden layout choices are new design proposals.

## Start here

1. Read this guide and `CLAUDE_PROMPT.md`.
2. Load `module_catalog.json` for mesh dimensions, pivots, material bindings, UV and collision conventions.
3. Load `level_layout.json` for explicit instances and the movement graph. Paths are relative to the kit root, not this `spec` directory.
4. View `topdown_plan.svg` and `module_dimensions.svg`. These are deterministic vector diagrams, not AI-inferred measurements.
5. Inspect `meshes/mesh_manifest.json` and `meshes/validation_report.json` for exported geometry facts. Use the mesh export manifest if an approximate description conflicts with its exact bounds.

The assembled layout contains 114 grid cells, 95 walkable cells, 266 mesh instances and 149 bidirectional navigation edges. The dry and water routes were both checked by graph search. These checks do **not** prove runtime controller, collision, asset loading, or rendering behavior.

## Ready assembled imports

`meshes/assembled_textured.glb` is a preassembled visual scene with the runtime textures embedded. `meshes/assembled_blockout.glb` is the same proposed layout with placeholder materials. They contain the declared 266 instances and are convenient for an immediate visual import. Use **one assembled scene or the individual modules plus layout**, not both at once. The JSON still supplies logical navigation and collider proposals; importing an assembled scene does not implement those systems.

The assembled previews are cutaway inspection renders; the corridor ceiling is hidden for visibility in those renders, not omitted from the authored layout. Independent GLB validation checked the assembled transforms against the final layout. This is asset validation, not an engine/game test.

## What the reference establishes

Keep warm weathered sandstone, square pillars with heavy capitals, dark sheltered passages, broken paving, moss and vines, cyan-green water, strong warm sunlight, and an imposing seated statue silhouette. The bridge and chamber should be legible at a 320 × 240 internal viewport. Use broad silhouettes and clear light/dark groupings rather than fine surface noise.

The source is a montage of perspective views, a texture palette, and modular-piece concepts. It is not a measured floor plan, photogrammetry dataset, texture atlas, or set of matching orthographic projections. The proposed bridge is a low slab causeway, not a recovered version of the pictured arch bridge. The statue mesh is deliberately an abstract seated blockout, not a finished sculpt or historical reconstruction.

## Coordinate and placement contract

- Meters; right-handed coordinates; +Y up, +X right, local forward −Z.
- Square grid spacing 2 m. Cell `(x,z)` center is `(2x+1, surfaceY, 2z+1)`.
- North is −Z. Positive Y rotation takes local +Z toward world +X.
- `rotation_degrees_xyz` applies X, then Y, then Z. The layout only needs Y rotations and the ceiling's X rotation.
- Do not auto-center imported meshes. Do not apply a second axis conversion or normalize their scale.
- Floor/bridge pivots sit on the top walking surface. Walls, columns, doorway, rail, plinth and statue use base-center pivots.
- **Stairs are the exception:** pivot at the center of their LOW edge; local bounds X ±1, Y 0…0.6, Z −2…0. They ascend toward −Z. Preserve this asymmetric pivot.
- Default scale is `(1,1,1)`, except supplied columns use `(1,1.025,1)` so their tops reach 4.1 m rather than z-fighting with 4 m wall tops. East/west-facing boundary walls additionally use Y scale 0.9995, a 2 mm top offset to prevent coplanar corner faces. Preserve all explicit scales. Shared mesh/material handles are preferable to duplicating mesh data per instance.

### Proposed spaces

| Space | Footprint or clearance | Treatment |
|---|---:|---|
| Corridor | 4 m wide × 12 m long | Roof underside Y = 4 m |
| Junction | 10 × 8 m | Open to sky |
| Water court | 10 × 16 m | Bed Y = −0.6 m, water Y = −0.3 m |
| Bridge | 4 × 10 m | Walking top Y = 0 m, low parapets |
| Statue chamber | 12 × 14 m | Open sky, 4 × 4 m blocked plinth |
| Doorway | 3 × 3 m clear opening | Outer bounds 4 × 4 × 1 m |

Dry floor and bridge top are Y = 0. Water is 0.3 m above its separate bed. Two three-step stair modules each rise 0.6 m over 2 m. They connect the junction and chamber to the water route. This is a shallow wading path, not swimming. The northern pool beyond the bridge is scenic and non-traversable; only the southern pool belongs to the walkable alternative. All 95 declared walkable cells are reachable from the start.

## Mesh and collision use

Eleven GLB meshes and OBJ/MTL alternatives are supplied. They have UV0 and simple exported materials. They are authored low-poly bases, not just sketches. The game still needs to load them, create collision and movement behavior, bind textures, and supply lighting.

Use GLB as the initial import route; keep OBJ as a portable editing/export fallback. Treat GLB and OBJ as alternatives, not two assets to spawn simultaneously. A ceiling uses the floor mesh rotated 180 degrees around X at Y = 4 m; its slab then extends upward.

Use the catalog's simple collider descriptions instead of triangle-mesh collision everywhere. **A doorway needs three boxes around its opening, not one solid bounds box.** Stairs use a convex ramp for smooth movement. Water planes have no collider. The bed beneath water provides the surface. Plinth cells are blocked. Rubble in this layout is visual dressing with no collision; if moved into a path, update both collision and navigation.

Use `navigation.edges` as the authoritative cardinal movement graph; missing edges are forbidden. Do not allow diagonal corner cutting or an implicit jump between water and bridge. Parapets separate bridge and pool; stairs only allow east-west traversal. The two stair side restrictions are logical grid constraints rather than a promise of fully enclosed physics corridors. A free-movement controller would require an additional collision pass and possibly extra rails.

Start at cell `[2,17]`, facing north; proposed eye height 1.5 m, capsule radius 0.28 m. The proposed noncombat goal is cell `[13,8]`, facing the statue. These are scaffolding, not implemented gameplay. For a grid controller, animate between cells and interpolate stair height rather than teleporting vertically. A water speed multiplier of 0.7 is optional tuning, not an asset dependency.

## Materials and texture projection

The material contract is intentionally small:

| Exported slot | Basecolor candidate |
|---|---|
| `M_Sandstone` | `art/runtime/256/materials/sandstone_blocks_basecolor.png` |
| `M_Floor` | `art/runtime/256/materials/worn_floor_basecolor.png` |
| `M_Carved` | `art/runtime/256/materials/sandstone_blocks_basecolor.png` initially |
| `M_Rubble` | `art/runtime/256/materials/rubble_stone_basecolor.png` |
| `M_Water` | `art/runtime/256/materials/water_basecolor.png` |

**Important carved-trim exception:** `art/materials/carved_trim_basecolor.png` is a motif sheet, not a whole-image tile. The supplied generic planar UVs must not repeat that sheet across every carved face. Start `M_Carved` with the sandstone fallback above. For a later detail pass, select sheet regions with explicit per-face UVs, or use `art/runtime/256/trim/carved_meander_strip.png` and `carved_diamond_strip.png` with purpose-authored strip UVs (U-repeat candidate, V-clamp). The strips are not a replacement for re-UVing.

Runtime 256 × 256 derivatives are supplied; full-size source images remain in `art/materials/` and `art/foliage/`. `mossy_stone_basecolor.png` and `compact_dirt_basecolor.png` in the runtime materials folder are optional variants. Do not create a unique material per instance; reuse a few shared variants. The initial exported materials may be placeholder colors rather than embedded generated images. Bind by material name and verify what your loader actually imported.

UV0 uses planar projection per face with a nominal repeat every 2 world meters. A 2 m face spans one texture repeat; a 4 m wall rises through two repeats. Intentional UV seams occur between faces. There is no promised lightmap UV1, unwrap for a detailed hero statue, normal map, roughness map, displacement map or PBR calibration. These are stylized basecolor-style images, not measured physical albedo.

Start with basecolor only, metallic 0, high roughness, and a simple lighting setup. Avoid multiplying strong generated shading by harsh dynamic lighting. Use repeat addressing on tile candidates, and test a visible 3 × 3 repeat before accepting seams. Generated tile candidates are **not automatically certified seamless**. Keep originals untouched; use the supplied 256 px runtime copies as a starting point, and make any further reductions only after a repeat and 320 × 240 readability review. Nearest-neighbor is a suggested retro starting point, not a requirement; compare against a less shimmery filtered option during movement.

Foliage candidates are `art/runtime/256/foliage/fern_cluster_rgba.png` and `art/runtime/256/foliage/hanging_vines_rgba.png`. Their card meshes are **not** part of the eleven-module kit. Claude can add one or two crossed quads for ferns and vertical cards for hanging vines, with UV 0…1, clamp addressing, alpha mask, no collision, and a cutoff tuned after alpha inspection. Keep silhouettes away from navigation centers. Alpha edges and halos still need engine QA; avoid extensive overlapping transparency.

## How to use the projections

`module_dimensions.svg` is the dimensional guide. `topdown_plan.svg` is the layout guide. The AI-generated `art/guides/modular_orthographic_style_reference.png` is only an appearance reference: do not trace its measurements, assume matching silhouettes, or use it as a texture atlas.

Use the source's corridor/junction/bridge/chamber panels as composition targets. Do not project the whole montage onto world geometry, use it as a cubemap, or treat it as a true panorama. The layout JSON includes proposed diagnostic camera views so four similar scene checks can be rendered from actual 3D geometry. No calibrated cameras or texture-projection reconstruction are supplied.

## Implementation order for Claude

1. Inspect the existing Rust/Bevy project and its actual dependency versions before changing code. Keep its controller and asset conventions unless they conflict with this contract.
2. Load the module catalog and instantiate the JSON layout in flat colors. Show a debug grid, cell IDs, and allowed edges. Walk both supplied validation routes.
3. Test collision and the three openings. Check stair pivots, roof orientation, statue facing, pool heights and forbidden water/bridge transitions.
4. Bind the five core materials. Verify UV repeat scale, face orientation and texture paths. Add optional moss, dirt and modest foliage only after the base view reads clearly.
5. Light with restrained cool ambient plus one warm directional source. Prefer cheap baked-looking contrast; enable costly shadows only if measurements permit. A torch can be an emissive visual accent without a point light.
6. Render internally at 320 × 240 and scale to the actual display. Keep enemy billboard integration separate; enemy sprites, combat, AI, audio and UI are not included in this environment kit.
7. Profile a release/native build and the actual browser/Wasm build on target Linux/Android handheld hardware. Document measured results and remaining issues rather than claiming broad compatibility from an editor preview.

## Performance targets and later N64 work

These are proposed engineering budgets, **not measured results, hardware limits or guarantees**:

- Initial handheld/browser target: stable 30 fps at 320 × 240 internal resolution; 60 fps is a stretch goal after profiling.
- Aim for no more than 20,000 visible triangles and roughly 40 visible draw submissions after batching/culling, with a small reused material set. Actual scene complexity varies by camera.
- The current full layout totals **39,336 triangles before culling**. The eleven unique mesh assets total 1,754 triangles. Instance count is not draw-call count; do not confuse the two.
- Prefer opaque water, alpha-mask foliage, no refraction, no per-pixel normal mapping, and no dynamic shadow dependency for the first pass.
- The supplied 256 px runtime copies are a starting point; 128 px alternatives are a proposed experiment. Validate detail and memory on the real device; the larger source art is not the runtime budget.
- If visibility is expensive, group by corridor/junction/court/chamber sectors, cull concealed sectors, batch static opaque pieces, and replace hidden faces or excessive bevels in a lower-detail export.

A later real N64 port is a separate engineering/export project. The supplied GLB files and Bevy/Wasm application are **not a drop-in N64 build**. Retain the layout semantics and visual language, but rebuild the renderer, asset conversion and runtime for the chosen toolchain. As a first exploratory art budget—not a compatibility claim—try 3,000–5,000 visible triangles, palette-limited 32–64 px texture derivatives, very few material switches, simplified floor/wall boxes or quads, and baked/vertex-style lighting. Actual memory, texture formats, display-list organization, audio and performance must be established against that toolchain and real hardware or a validated emulator. No N64-ready export is supplied.

## Acceptance checklist

- [ ] All eleven mesh IDs resolve, without duplicate scene spawning or a second coordinate conversion.
- [ ] All core textures bind by exact filename and material ID; no black/magenta missing maps.
- [ ] The source montage and the AI style sheet are not mistakenly used as runtime textures.
- [ ] Floor tops align at Y = 0; ceiling underside is at 4 m; stair high edges meet dry land.
- [ ] Doorways remain open with compound collision; statue faces the chamber approach.
- [ ] Both explicit routes reach the goal; plinth and parapet crossings remain blocked.
- [ ] Texture repeats, UV seams, foliage alpha and water depth are checked at game resolution.
- [ ] Camera height and cell transitions do not clip columns or cause abrupt stair jumps.
- [ ] Native and browser runs are tested on representative hardware; budgets are reported as measured or unverified.

## Provenance and limits

The source is the user-supplied `reference/source_concept.png`. New images are AI-generated visual derivatives based on that direction; the supplied geometry, layout and SVG diagrams are newly authored approximations. No third-party asset-library meshes or textures were intentionally copied into this kit. This provenance statement is **not a copyright clearance or a claim that the reference is public domain**. The user should confirm they have the necessary rights to the source and intended distribution.

Missing by design: a final sculpted statue; authored foliage meshes; characters and enemy sprite sheets; animation rigs; combat/AI; sound/music; UI; complete engine code; tested controller/physics integration; certified seamless/PBR texture maps; calibrated reconstruction; and a validated N64 port. Optional assembled scene previews, if present elsewhere in the package, are visual/import aids and not proof that the game is implemented.
