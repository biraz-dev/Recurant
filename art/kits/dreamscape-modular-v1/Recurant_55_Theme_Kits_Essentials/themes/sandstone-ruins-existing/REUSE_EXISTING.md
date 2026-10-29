# Existing Sandstone Ruins — preserve and reuse

This is the preserved legacy catalogue entry, **not a newly generated or rebundled sandstone kit**. The original assets already exist in the project repository and the current entrance uses their converted binary. Do not overwrite the original authoring kit, composition or identity.

## Practical reuse
From the project repository root, read `ASSET_HANDOFF.md` and `ASSET_INVENTORY.json`, verify the tracked archive hashes, then run the repository-provided extraction helper when authorized:

```sh
python3 tools/extract_asset_kits.py
```

The tracked source archives are `art/kits/archives/sandstone-ruins-v1.volume-001.zip` through `sandstone-ruins-v1.volume-018.zip`. They restore the existing `art/kits/sandstone-ruins-v1/` authoring tree. Use its eleven original modules plus original layout **or** its assembled scene, not both. The helper belongs to the repository and was not executed by this asset handoff.

The original asset handoff is pinned at commit `f10af6ddcb3cf3d437adf2c42f77f8244bda1924`; current runtime/contract inspection was at `b9fdd98f37bcb419bcf2951103529ff571e5ea21` on 2026-10-29. Recheck current HEAD before integration. Exact original archive/file hashes remain authoritative in the repository's `ASSET_INVENTORY.json`.

Already-converted runtime assets are `crates/zrpg/assets/ruins/level.bin` and adjacent PNGs, with core map `crates/zrpg_core/src/ruins.map`; converter `tools/make_ruins.py`. The current original entrance is the preferred ready-to-run sandstone route.

## Differences from new kits
Original sandstone walls are 2 × 4 × 0.4 m per the source catalog, doorway module 4 m wide with 3 × 3 m opening, and original stairs have a low-edge pivot ascending −Z. Preserve per-instance scales and anti-z-fighting offsets. New kits use their separate contract; never silently reinterpret the old kit through new dimensions.

Legacy mapping for deliberate new recipes: `M_Sandstone`→wall, `M_Floor`→ground, `M_Carved`→structure, `M_Rubble`→detail. `M_Water` remains separate optional fluid. Original carved motif sheet is not a globally tiling texture; original statue is a blockout proxy, not a finished sculpt.

Two original text documents and their verified provenance are preserved in `common/spec/observed-sources/`. Volume 018 was checked at 186,960 bytes, SHA-256 `289626626f3099e932419d5ef984984500bbcf7ce55556c295781a29359a56d7`. This does not mean the other seventeen archives or original GLBs/textures are copied into this new package.
