# Dreamscape modular environment + equipment handoff

**Actual runtime/import assets and source art—not just concept images.** This new asset branch adds 54 new themed environment kits, an explicit reuse recipe for the existing sandstone ruins (**55 catalogue entries**), shared modular geometry, and the requested seven-slot equipment UI. Existing game code and earlier asset kits are preserved. No merge, deployment, importer, gameplay or UI-data change is included.

## One-command extraction

From the repository root, using Python 3.9 or later (standard library only):

```sh
# Essentials and equipment; original full-resolution masters are optional.
python3 tools/extract_dreamscape_assets.py

# Full handoff, including exact original source-art paths and editable Blender source.
python3 tools/extract_dreamscape_assets.py --with-sources
```

Add `--verify-only` to check without writing; `--list` shows available theme/group IDs. For one theme, pass its ID (shared essentials are included automatically), for example `python3 tools/extract_dreamscape_assets.py clockwork`. The optional source-art group restores all selected original source masters; for a narrowly targeted source restore, the separate source helper supports `--only-theme`.

The helper verifies every archive part, reassembled ZIP, member byte count and SHA-256 before extraction. It refuses unsafe paths, symlinks and different existing files; matching files are reused. Ordinary volumes are valid independent ZIPs. A source-art volume or large single-file volume may have clearly named `.zip.partNNN` binary segments; these are reassembled in memory and verified before use. Do not unzip an individual `.partNNN` file or omit numbered parts. Nothing is downloaded or executed from the network.

## Where to start

| Package | Browse now | After extraction |
|---|---|---|
| 55-theme catalogue | [Theme catalogue](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/THEME_CATALOGUE.md), [native-render gallery](MODULAR_THEME_GALLERY.md), [theme index](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/THEME_INDEX.json) | `art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/` |
| Integration contracts | [Claude start here](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/CLAUDE_START_HERE.md), [asset QA](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/VALIDATION_REPORT.json), [visual review](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials/QA_VISUAL_REVIEW.md) | Each `themes/<id>/` contains its manifest, material candidates, layout/collision/navigation proposals and actual `assembly/showcase.glb` |
| Optional source art | [Source bundle notes](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Source_Art/README.md), [lossless source alias map](art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Source_Art/SOURCE_ALIAS_MAP.json) | `art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Source_Art/`; `--with-sources` also restores original paths inside Essentials |
| Seven-slot equipment | [Claude UI guide](art/kits/seven-slot-equipment-v1/README-CLAUDE.md), [layout](art/kits/seven-slot-equipment-v1/layout.json), [native preview](art/kits/seven-slot-equipment-v1/previews/equipment-320x240.png) | `art/kits/seven-slot-equipment-v1/` |

The original canonical folder names are preserved under `art/kits/dreamscape-modular-v1/`; all validated file bytes and package-relative asset references are retained. Text documents/manifests and native previews are loose for browsing. Binary/runtime assets and original masters are in `_archives/` until extracted. `MODULAR_ASSET_INVENTORY.json` and `_inventories/` describe the connector-specific archive layout; use this helper rather than assuming the separate direct-download ZIP numbering in source-package notes.

**Claude prompt:** “Read MODULAR_ASSET_HANDOFF.md; run `python3 tools/extract_dreamscape_assets.py --with-sources`; inspect the existing project instructions and selected theme/UI manifests before making integration changes. Preserve current game behavior and do not claim in-engine compatibility until tested.”

## Source completeness and storage

The supplied full-resolution material/concept masters, retained source revisions, prompts, authoring support, and editable shared `.blend` are preserved. Exact duplicate original masters are stored once by SHA-256 in the optional source bundle; its alias map restores their original relative paths without changing pixels. Derived debug/repeat images and oversized duplicate overview renders are not runtime requirements; the validated distribution documents its QA scope and retains evidence summaries and regeneration support. No GLB images were stripped or runtime files rewritten by publication.

The existing sandstone is **reuse, not a newly generated 55th kit**. Its earlier assets and provenance remain untouched on this repository's default lineage. Follow `themes/sandstone-ruins-existing/REUSE_EXISTING.md`; when its original authoring tree is needed, the earlier repository helper remains `python3 tools/extract_asset_kits.py sandstone-ruins-v1`.

## Integration boundaries

- Environment checks are offline asset/geometry/material/layout checks. Gallery images are renders of actual GLBs, not finished playable levels or runtime proof. Controller/collision behavior, target-device speed, camera-motion shimmer, native/browser integration and shipping quality remain untested.
- Texture repeats are candidates, not certified seamless maps. Do not invent PBR/normal/ORM compatibility from diffuse-only artwork. Read each theme's documented pivots, scale conversion and exceptions before importing.
- The one-bit kit includes a specific two-color/unlit presentation contract; its raw generator masters can contain gray, and its contour/dither treatment is an authored render convention rather than a tested game renderer.
- Equipment remains exactly **seven** slots on a **320 × 240** layout. The current game has a different eleven-slot model; do not silently hide equipment data, change gameplay or overwrite existing UI assets. Reconcile that mapping deliberately in a separately authorized integration task.
- The corrected equipment rebuild helper uses bundled sources only; two isolated rebuilds reproduced all 26 approved PNGs byte-for-byte. Its silhouette uses BOX downsampling and keeps partial alpha; display enlargement is nearest-neighbor. This does not establish interactive UI behavior.

No rights, exclusivity or third-party clearances are invented. Preserve the included provenance/use notes and review them before release.
