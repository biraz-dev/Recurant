# Optional original source art — lossless deduplicated bundle

Download and extract every Source Art ZIP volume into the same directory. This bundle contains every original generated material/concept master and retained source-art revision selected for the final handoff, plus the editable shared .blend. Exact duplicate masters are stored once bySHA-256; `SOURCE_ALIAS_MAP.json` lists each original relative path and its verified content blob. No source pixels were changed and no original master was deleted from authoring storage.

To verify without copying:
```sh
python3 restore_source_art.py /path/to/Recurant_55_Theme_Kits_Essentials --verify-only
```
To restore all original source paths:
```sh
python3 restore_source_art.py /path/to/Recurant_55_Theme_Kits_Essentials
```
The helper rejects unsafe paths, missing/hash-mismatched blobs and differing existing destination files; it never silently overwrites an edited source. Matching files are reused. `--only-theme onebit-spiritual-landscape` restores just that theme's sources. Original paired atlases deliberately restore into more than one theme path when both themes used the same generated master.

Source is optional for viewing/importing the supplied GLBs/runtime PNGs, but required before rederiving textures or using the one-bit source derivation script. Concept source studies may include earlier drafts; Essentials' native reference is the selected art-direction reference. Source masters are not runtime textures or screenshots of the game. One-bit raw generator masters may contain gray; the documented binary conversion creates the exact two-color delivery assets.

Large derived QA repetition images, duplicate QA candidates and per-theme800px overview renders are deliberately not distributed here. Their current evidence summaries and regeneration scripts are in Essentials. This bundle preserves all selected original source artwork, not every transient debug output.
