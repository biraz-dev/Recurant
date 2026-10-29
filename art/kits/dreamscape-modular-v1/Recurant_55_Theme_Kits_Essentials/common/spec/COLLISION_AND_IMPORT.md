# Claude integration notes

## Preserve the game's authority boundary

The current `zrpg_core::Map` decides walkability and cardinal movement. Tiles are Wall, Floor, StairsDown; edge masks can forbid passage between two otherwise walkable cells. N/E/S/W bits are **1/2/4/8**. `Map::block` makes restrictions bidirectional. `can_step` checks destination walkability and the source edge mask. Mesh bounds and decorative collision guides do not implement that logic. See [map.rs](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/crates/zrpg_core/src/map.rs).

For every assembled example, define cells `[x,z]`, their surface height in metres, walkable/blocked state, and explicit bidirectional adjacent allowed edges. Missing edges between declared walkable neighbours are forbidden. Derive symmetric blocked masks for the current core. No diagonal movement, automatic jumps, climbing, swimming, falling, damage or interactions are supplied by these art assets. Hazards are visual unless explicitly integrated in core rules later.

Keep the central 1.2 m-wide travel corridor clear on every allowed edge, with at least 2.4 m headroom in the tall profile. Include camera backward offset when checking all four look directions. Bounds are not collision: door/arch guides must leave their opening clear; never use one full bounds box. Rails close edges, not entire cells. Fluid surfaces have no walking collider; bed/deck is separate. Platforms at different heights require logical route/height declarations. Scenic props stay in blocked cells or wall margins. Decoration must not produce invisible barriers, and visible barriers must not allow unmarked passage.

Current line-of-sight uses cell walkability and does not account for all edge-blocking geometry. Do not promise that rails/doors also occlude spells, sight or attacks; inspect the relevant core system before assigning gameplay semantics.

## Import sequence

1. Read current HEAD and local project instructions; preserve the user’s current code. Inspect this contract, actual geometry manifests and the chosen theme's material/assembly manifest.
2. Choose one route: individual modules + placement recipe, **or** an assembled GLB. Never spawn both. Do not import source sheets/contact sheets as a world atlas.
3. Prototype one theme in isolation. Use flat colours first. Preserve authored pivots, bake module node transforms, apply 0.5 metres-to-game scale exactly once. Never normalize mesh size or auto-centre.
4. Add a small offline adapter modeled on the existing ruins converter only if appropriate. The legacy converter's hardcoded five material names cannot consume the new four roles unchanged. Bind `wall`, `ground`, `structure`, `detail` by name. Keep the existing embedded-assets approach unless deliberately changed.
5. Build walkability/edge masks from explicit layout metadata, not triangles. Confirm symmetric edges and full reachability from the start; mark scenic cells blocked. Keep path-affecting changes in `zrpg_core`, rendering in `zrpg`.
6. Bind external runtime PNGs by manifest role. `wall`/`ground` repeat; `structure`/`detail` clamp using per-component 0–1 panel UVs. Inspect whether the particular GLB has preview textures or placeholder factors. Add normal/ORM only when present and tangents are generated/verified.
7. Use appropriate tall/short ceiling and eye-height profiles. Verify doorway openings, all rotations, ceiling face normals, near-plane/back-offset clipping, stairs, fluid/bridge separation and prop margins.
8. Inspect static native-resolution frames and movement; compare nearest magnification with mipmapped minification. Retain the depth-grade material where intended. Do not accidentally turn every material unlit or every emissive prop into a point light.
9. Run repository tests and release/native + browser/Wasm smoke tests when authorized. Exercise start/restart, seed determinism, stairs, enemy routing, all four movement directions and level changes. Profile actual handheld targets separately.
10. Report asset checks separately from runtime validation. Do not claim compatibility merely because Blender or a generic GLB viewer opened the files.

## Existing sandstone theme: reuse, do not replace

The repository already contains the complete original `sandstone-ruins-v1` across 18 independent ZIP volumes, with all file/archive hashes in `ASSET_INVENTORY.json`; [ASSET_HANDOFF.md](https://github.com/biraz-dev/Recurant/blob/b9fdd98f37bcb419bcf2951103529ff571e5ea21/ASSET_HANDOFF.md) explains extraction. Its runtime entrance is already integrated at inspected HEAD. Preserve that original composition and asset identity.

The existing ready-to-run route is the committed `crates/zrpg/assets/ruins/level.bin`, its PNGs, `crates/zrpg_core/src/ruins.map`, `ruins.rs`, and `tools/make_ruins.py`. Do not replace these with the new library as part of a visual-kit handoff. The rebuild/editing route is to verify/extract the original archives using the repository's helper, then use its **eleven modules + original layout**, not both these and its assembled scene. No repository helper was run while preparing this contract.

Legacy role mapping for a new recipe, if desired: `M_Sandstone`→wall, `M_Floor`→ground, `M_Carved`→structure, `M_Rubble`→detail. `M_Water` remains a separately bound fluid variant, not a silent fifth slot on every new module. Original carved motif sheet is not globally tileable: keep the sandstone fallback unless purpose-authored trim UVs are supplied. Preserve original stairs' low-edge pivots and original per-instance scales (including small anti-z-fighting adjustments). The original statue is explicitly a blockout proxy, not a finished sculpt.

The two preserved original text documents in `observed-sources` were extracted read-only from the small final archive volume and checked against the repository inventory. They provide exact old-kit dimensions and instructions; they do **not** mean all original sandstone meshes/textures have been duplicated into this new package. Clearly state that dependency when delivering a new standalone sandstone recipe.

## Acceptance gates

- **Asset gate:** valid GLBs/PNGs, finite bounds, normals, UVs, triangle/index limits, baked transforms, no missing declared paths, exact role names, path-safe relative manifest references, SHA-256 inventory, 320×240 and repeat previews.
- **Assembly gate:** one selected instance route, grid alignment, open door voids, lane clearance, start/goal reachability, symmetric edge restrictions, no unwanted coplanar overlaps, no undeclared gameplay.
- **Runtime gate:** current import adapter, core map agreement, materials, lighting, moving-camera image quality, native/browser loading, measured target-device performance.

Only mark gates passed when their evidence exists. Asset generation does not itself pass the runtime gate.
