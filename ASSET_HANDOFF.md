# Recurant asset handoff

**The complete kits are independent ZIP volumes, not pre-integrated runtime assets.** This branch adds isolated art packages, previews, inventories and an extraction helper. It does not change game code, replace existing runtime assets, or implement gameplay.

## First: verify and extract

From the repository root, with Python 3.9 or later:

```sh
python3 tools/extract_asset_kits.py --verify-only
python3 tools/extract_asset_kits.py
```

The standard-library-only helper checks every archive and file SHA-256, rejects unsafe paths/symlinks, and extracts into `art/kits/<kit-id>/`. It keeps existing identical files and refuses to overwrite different files. To extract one kit, pass its ID, for example `python3 tools/extract_asset_kits.py ember-skeleton-v1`.

Every numbered volume for a kit is required; each is an ordinary independently valid ZIP, not a split binary segment. You may instead verify each archive SHA-256 in `ASSET_INVENTORY.json` and unzip all volumes for a kit into the repository root: archive paths already start with `art/kits/<kit-id>/`. Keep each directory structure intact. Do not import the ZIP file itself into the engine.

## Then give Claude these entry points

| Kit | Documentation after extraction | Main runtime candidates within kit |
|---|---|---|
| Sandstone ruins | `art/kits/sandstone-ruins-v1/spec/INTEGRATION_GUIDE.md` and `spec/CLAUDE_PROMPT.md` | `meshes/assembled_textured.glb` **or** individual modules plus `spec/level_layout.json`; `art/runtime/256/` |
| Smoldering undead | `art/kits/undead-four-direction-v1/art/extracted/README.md` and `art/extracted/manifest.json` | `art/extracted/atlases128/` **or** `art/extracted/frames128/` |
| Ember skeleton | `art/kits/ember-skeleton-v1/docs/CLAUDE_INTEGRATION.md` and `frames/manifest.json` | `frames/atlases128/` **or** `frames/frames128/` |
| Demon knight | `art/kits/demon-knight-v1/README.md` and root `manifest.json` | `atlases128/` **or** `frames128/` |
| Skeleton knight | `art/kits/skeleton-knight-v1/README.md` and root `manifest.json` | `atlases128/` **or** `frames128/` |
| Skeleton warrior | `art/kits/skeleton-warrior-v1/README.md` and root `manifest.json` | `atlases128/` **or** `frames128/` |
| Mummy | `art/kits/mummy-v1/README.md` and `extracted/manifest.json` | `extracted/atlases128/` **or** `extracted/frames128/` |
| Wraith | `art/kits/wraith-v1/README.md` and `extracted/manifest.json` | `extracted/atlases128/` **or** `extracted/frames128/` |
| Inner demon | `art/kits/inner-demon-v1/README.md` and `extracted/manifest.json` | `extracted/atlases128/` **or** `extracted/frames128/` |
| Royal mummy (separate character) | `art/kits/royal-mummy-v1/README.md` and `extracted/manifest.json` | `extracted/atlases128/` **or** `extracted/frames128/` |
| First-person sword v02 | `art/kits/first-person-sword-v02/README.md` and `sword_v02_animation.json` | `frames/` **or** `sword_v02_atlas_4x3.png`; exact 320 × 240 screen-overlay cells |

Start by saying: **“Read ASSET_HANDOFF.md, verify and extract the asset kits with the supplied helper, then inspect the relevant kit's integration guide and manifest before adapting the game. Do not claim runtime compatibility until it is tested.”**

Paths within each kit remain relative to its original kit root or the manifest's documented base. Do not spawn both an assembled ruins scene and its individual layout instances. For characters, use aligned transparent PNG frames/atlases, not blind grid slices of source sheets or opaque preview GIFs.

## Preview without unpacking

Start with [the original eight-character overview](art/kits/previews/recurant-bestiary-preview.png). The newer royal mummy and sword v02 have separate contact sheets and GIFs under their named preview directories. Browse `art/kits/previews/`: the ruins assembled preview and geometry contact sheet, plus character all-frame contacts and four-view walk/hover/attack GIFs. These duplicate selected files inside the archives for easy inspection; they are not additional poses or runtime assets.

## Scope and verification

Eleven kits total: sandstone ruins, nine creatures, and the first-person sword v02.

- Ruins: eleven mesh modules, GLB and OBJ/MTL alternatives, two assembled scenes, textures, layout, diagrams, authoring scripts and asset QA reports. The layout has 266 instances; its statue is a blockout proxy.
- Nine creatures (864 total poses): each has four directions × four actions (walk/hover plus three attacks) × six frames (96 poses), aligned 128 px/256 px frames and atlases, native cutouts, previews, original source sheets, references, prompts, manifests and extraction checks.
- Sword v02: twelve transparent RGBA **320 × 240** overlay frames, atlas, timing/registration manifest, source design reference, previews and rebuild script. Draw whole frames at viewport origin `(0,0)` without automatic trimming/centering. Large swing offscreen exits and the almost-empty lowered reset are intentional; impact timing is proposed, not implemented damage.
- Royal mummy is a separate character (reference 141); it does not replace the earlier mummy (reference 137). Sword v01 runtime assets are not included; only the copied idle design reference is retained within v02 sources.
- All nine creature kits retain original/rejected source variants for provenance; use only the selected, aligned runtime frames declared by each manifest. Wraith locomotion is hover rather than walk; animation names and hit windows are kit-specific.
- No intended kit source/runtime file is omitted from its volumes. Transfer helpers, caches, the recursive sword ZIP duplicate and the duplicated ruins reference filename are excluded; the reference itself is preserved as `reference/source_concept.png`.
- `ASSET_INVENTORY.json` lists archive hashes and every included kit file with its repository-relative path, byte count, SHA-256 and Git blob SHA. Local extraction was checked against these inventories.
- Included reports establish **asset-level** checks only. These are prototype animations and visual/blockout assets, not production-verified runtime integration.
- No native, browser/Wasm, handheld, physics/controller, combat, performance or N64-runtime QA is claimed. Timing/hit frames and performance budgets are proposals, not measured gameplay.
- Preserve the original source artwork and review each kit's provenance and limitations before shipping. Provenance is not a rights-clearance guarantee.

Asset-authoring/extraction scripts remain optional rebuild aids inside the archives. No merge, deployment or game-code integration is included in this handoff.
