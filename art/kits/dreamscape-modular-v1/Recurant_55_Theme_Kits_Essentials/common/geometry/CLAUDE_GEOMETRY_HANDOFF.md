# Claude handoff — geometry and integration boundaries

## Objective

Use these reusable low-poly building blocks, theme diffuse materials and assembly recipes to realize Recurant's environments. Preserve the originality of the forms. The concept images are inspiration, not blueprints to trace and not assets to copy from other games.

This handoff intentionally delivers actual meshes rather than only textures. It does **not** authorize modifying game code in this art-production task. The future implementation owner should review and adapt the integration steps below.

## Read before importing

1. `../spec/asset_contract.json` and `../spec/COLLISION_AND_IMPORT.md`.
2. `geometry-manifest.json` and `asset-index.csv`.
3. The selected theme's material manifest and kit/layout recipe.
4. `qa/deliverable-validation.json` and the corresponding real mesh/render previews.

The recorded repository snapshot uses custom `level.bin` conversion, not generic runtime GLB loading. The existing converter recognizes legacy names such as `M_Sandstone`, `M_Floor`, `M_Carved`, `M_Rubble`, `M_Water`, and ignores GLB node transforms and embedded images. The new names `wall`, `ground`, `structure`, `detail` therefore require an **explicit semantic-material adapter** and the existing PNG packing/conversion route. Do not assume dropping a textured GLB into the repo makes a playable theme.

## Safe implementation sequence

1. Pick a small pilot theme and a 3×5-cell layout. Keep the center travel lane at least 1.2 authored metres clear. Treat dense scenery cells as blocked.
2. Load the canonical GLB primitive buffers. Do not multiply by the visual Blender catalog positions. Bake any desired instance rotation/translation before feeding a transform-ignoring converter.
3. Apply the single uniform 0.5 authoring-to-game scale to positions, dimensions and clearance checks. Normals remain normalized. Do not apply this factor twice.
4. Map the four exact material roles to the chosen theme's four diffuse files. Honor wrap hints. Verify actual UV orientation, repeat seams and culling from eye height; do not rely only on an orthographic screenshot.
5. Adapt the existing grid walkability/edge-mask logic. **Never** merge the `collision-guides` GLBs into the visible render mesh. Never substitute a full arch bounding box for its two jambs: it would block the aperture.
6. Verify an orthogonal hall, corner, doorway, floor seam, ceiling, stairs and bridge in the real engine. Use existing camera/FOV values from the contract. Test normals on both sides of a closed wall and do not enable double-sided rendering as a blanket winding fix.
7. Only after pilot acceptance, batch the remaining themes through the same checked adapter. Keep per-theme material/layout differences explicit rather than cloning one room with a palette swap.

## Placement specifics that prevent subtle failures

- Detailed/front wall face is **−Z**. For a west boundary, rotate Y=−90° to face east into the room. For an east boundary, rotate Y=+90°. South/back wall uses zero rotation; north wall uses 180°.
- Normal wall endpoints are local X=−1/+1, Y=0, Z=0. Corner connectors and scenery footprints are different; use manifest connectors.
- Standard floor top is Y=0. The top of a raised rug/mat is slightly positive; place it as dressing, not as the sole walkability definition.
- Flat ceiling underside is Y=4; coffer beams reach Y=3.84. A vaulted ceiling extends above its spring line.
- `stairs_rise1_run2` and `stairs_rise2_run4` start at [0,0,0] and end at [0,rise,−run]. They use 0.25 m risers and 0.5 m treads. Use a movement ramp, not tread-by-tread physics, if the engine supports elevation.
- `ramp_rise1_run2` has the same low-edge / −Z convention.
- `door_leaf_plank_1p36` is hinge-pivoted. Its closed mesh spans X=0…1.36; doorway placement is [−0.68,0,0]. Runtime rotation is about its Y axis.
- `culvert_vault_2m` is intentionally low. Its 1.5 m spring height is **not** a standard standing doorway.
- `stalactite` points downward from its ceiling-attachment pivot. Do not floor-place it.
- `water_surface_2m` is opaque display geometry without movement or water behavior. Its guide list is empty deliberately.
- `colossal_trunk_quarter`, despite the short historical asset ID, is a full large scenery trunk with flared roots. It is not a quadrant wedge and must not be repeated four times around an origin.
- Pipes and gears are closed display solids, not hollow fluid geometry or mechanically rigged simulation parts.
- Library counts include size variants and one explicit duplicate-purpose floor/landing alias. Theme kits reuse these assets; do not call them all bespoke geometry per theme.

## Artist extension points

- Geometry parameters are explicit in `src/generate_geometry.py`: wall dimensions, profile points, radii, polygon counts, stem shapes, crystal growth, gear teeth, stair rise/run and prop proportions.
- A `Mesh` stores positions, flat normals, UVs and role assignment for each face. `clean_coplanar_faces()` subtracts duplicate same-facing coplanar coverage, interpolating UVs and preferring later authored trim; it is not a boolean volume union. `Mesh.transform()` bakes placements; `write_glb()` writes a compact identity-node GLB without external dependencies.
- Add meaningful silhouette variations, not only random scale/color variations. Preserve 2 m connection spans where intended.
- Keep outward winding. Thin leaves/cloth are closed solids already; retain thickness rather than relying on double-sided alpha cards.
- Use an artist-authored trim atlas for repeated narrow mouldings or pipes if the chosen texture panels stretch visibly. The supplied UVs are a usable baseline, not final unique paint UVs.
- Do not decimate already tiny meshes blindly. Repeated props benefit more from instancing and culling; final engine budgets remain to be measured.

## Acceptance evidence

`src/validate_deliverables.py` reads the actual files and checks 150 individual render GLBs, 6 scene GLBs and 138 separate collision-guide GLBs. It also checks all declared doorway apertures. `render_blender.py` imports the actual GLBs before rendering and saving the editable catalog. Contact sheets are therefore evidence of real geometry—not generated concept images.

Further runtime, shader, texture seam, animation, LOD and gameplay testing remains an implementation responsibility. No runtime integration is claimed by this art library.
