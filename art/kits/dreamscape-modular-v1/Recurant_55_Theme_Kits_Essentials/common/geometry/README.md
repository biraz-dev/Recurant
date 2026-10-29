# Recurant — modular low-poly geometry building blocks

**150 reusable modules, 10 families, 6 assembled demonstration scenes.** This is a practical, editable starter production library—not 54 bespoke reconstructions of the concept paintings, and not an integrated game level.

All geometry is newly authored parametric art. No meshes were downloaded, extracted from games, or reconstructed by passing concept images off as models. The shared families intentionally support multiple themes through different combinations and material bindings.

## Start here

- `previews/all_150_meshes_contact.png` — every delivered module, rendered from its actual GLB.
- `previews/contact_<family>.png` — larger labeled family boards.
- `previews/assembly_contact.png` — six real multi-module combinations.
- `geometry-manifest.json` — bounds, pivots, material roles, connectors, metrics, hashes, collision guidance.
- `asset-index.csv` — compact sortable inventory.
- `meshes/*.glb` — individual canonical render meshes.
- `Recurant_modular_geometry_library.blend` — editable Blender catalog; each asset is marked for the Asset Browser.
- `assemblies/*.glb` and matching `.json` — flattened, identity-transform example scenes and exact placement recipes.
- `collision-guides/` — separate box-guide JSON and optional preview GLBs, **never automatic gameplay collision**.
- `CLAUDE_GEOMETRY_HANDOFF.md` — integration and art-support notes.
- `../spec/` — verified game contract and explicit new-kit proposals.

## Inventory

| Family | Modules | Included forms |
|---|---:|---|
| Masonry | 19 | Full/half walls, ashlar relief, corners, broken crest, round/pointed doorways, hinged door leaf, gate, four columns, buttress, parapets, panels and brackets |
| Surfaces | 17 | Five floors, flat/coffer/vault ceilings, two stair rises, ramp, landing, stone/timber/metal bridges, iron/timber rails |
| Timber | 19 | Plaster/plank/lattice/paper walls, portal, posts, beam, gable and roof halves, inn furniture, tatami mat, shelf, barrel and crate |
| Organic | 25 | Rocks, cave wall/arch, stalagmite/stalactite, modular roots, trunk/buttress forms, branches, foliage, fungus, tissue ribs, pod and bone |
| Crystal | 9 | Three prism sizes, cluster, wall ribs, pointed portal, mineral floor, pedestal and orbit-ring focus |
| Machinery | 21 | Panel/rib/bulkhead walls, frame, pipes and fittings, valve, toothed gears, tank, console, anvil, forge, crucible, coils, vent and ladder |
| Textile | 8 | Closed pleated curtain, banner, rug, canopy, cushion, cube/wedge/arch toy blocks |
| Props | 9 | Urn, bottle, lantern, sarcophagus, altar, water placeholder, rubble, sunflower and book row |
| Landscape | 13 | Log, acacia, palm, two grass forms, cattails, two corals, cliff, shore bank, standing stone, seed heads and cactus |
| Infrastructure | 10 | Wide arch, open window, shrine niche, channel/crossing, culvert, rope bridge, raised walkway, fountain base and armory rack |

There are **149 unique raw mesh payloads**: `landing_2m` intentionally aliases the plain floor geometry. Size variants and common construction functions are also intentional; “150 modules” must not be described as 150 independently bespoke sculpted designs. The combined triangle count is **19,303**, before instancing.

## Authoring contract

- Metres, **Y up**, +X east, **−Z north / detailed front face**.
- 2 m placement grid; normal wall is 2 m wide × 4 m high × 0.2 m thick.
- Uniform import scale **0.5** converts the proposed authoring grid to the existing one-unit game cell.
- Floor pivot is center-top; standard floor extends Y=−0.2…0. Walls use edge-midpoint at floor level. Props generally use center-base.
- Stairs and ramp use a **low-edge center pivot** and ascend toward **−Z**. Their manifests record both landing connectors.
- Door leaf uses a real hinge-base pivot; place the hinge at doorway-local X=−0.68.
- Ceilings are authored above a room-floor pivot. Read their real bounds: coffer clearance is below 4 m; barrel vault spring is 4 m with a higher crown.
- Ornament, cave surfaces, roots and scenery can overhang the nominal grid bay. Snap to connectors, not arbitrary mesh bounds.
- All exported positions are baked into vertex buffers. Canonical individual GLB nodes have identity transforms. They use indexed triangles, float POSITION/NORMAL/TEXCOORD_0, and uint16 indices.

The Blender source is a **display catalog** with objects spaced apart for inspection. Its object placements are not level-layout coordinates. Local mesh data and the canonical GLBs retain the intended asset pivots. Blender itself displays Z-up; the files convert correctly to the canonical Y-up interchange coordinates.

## Materials and UVs

The only exact semantic slots are `wall`, `ground`, `structure`, `detail`; each module uses at most four. Default GLBs embed neutral solid display materials, **not final theme textures**. Theme workers provide diffuse PNGs separately, and theme showcase GLBs may embed them.

UV0 is real and finite. Wall/ground faces use metric planar projection at **2 m per UV repeat**, corresponding nominally to 128 px/m with a 256 px tile. Structure/detail faces use local 0–1 planar mapping so a nonrepeating panel can fit a face. This is a practical first-pass mapping, not a packed lightmap or hand-painted unique atlas. Curved/faceted forms have deliberate face seams. Long thin detail faces may need an artist-authored trim layout for final close-up polish.

Use each theme's material manifest to decide REPEAT versus CLAMP. Do not invent normal/roughness/metallic maps from the diffuse art, or claim water/translucent crystal/emissive effects are already implemented.

## Rebuild

The mesh generator requires Python 3 and NumPy; validation additionally uses `jsonschema`. Rendering uses Blender 4.3+; board composition uses Pillow. These were already installed in the production workspace; no external art or package download was used.

```bash
python src/generate_geometry.py
python src/validate_deliverables.py
blender -b -t 4 --python src/render_blender.py
python src/make_contact_sheets.py
```

Run from any directory; output paths resolve relative to the scripts. Regeneration overwrites **this geometry library's generated outputs only**. It does not edit theme textures or game code. Adjust primitive parameters in `src/generate_geometry.py` to create meaningful silhouette variants; keep stable IDs for existing assets.

## Verified and deliberately not claimed

The delivered files pass independent binary-read checks for structure, finite attributes, nondegenerate triangles, normal/winding consistency, positive total signed volume, valid uint16 indices, manifest bounds/metrics/hashes, and identity nodes. The nine declared standing-doorway apertures pass triangle–box clearance tests with a 1 micrometre inset tolerance. All 150 GLBs import and render in Blender.

These checks do **not** establish runtime navigation, optimal draw calls, a unique UV atlas, manifold boolean union, skinning, animation, VFX, LODs, polished hero-asset detail, or final texture seam quality. Meshes are built from overlapping low-poly solids, then same-facing coincident surfaces are clipped to avoid z-fighting. This is not a boolean watertight union. Separate collision guides are design aids; the current game uses grid walkability and edge masks. See `qa/` for machine-readable evidence and limitations.

## Optional source and regeneratable views
In this compact Essentials edition, source masters and the editable .blend are in the separately hashed Source Art bundle. Restore that bundle first for source-level rebuilds. Native previews/contact summaries are included; large per-theme overviews and historical material repeat PNGs are not bundled and can be regenerated with the supplied render helpers.
