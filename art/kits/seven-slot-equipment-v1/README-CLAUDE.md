# Recurant equipment screen — v0.1 preview

## Intent

A **320 × 240** equipment-only screen with a central front-facing humanoid silhouette and **exactly seven** equipment areas:

1. Head
2. Body
3. Left hand
4. Right hand
5. Ring 1
6. Ring 2
7. Amulet

Restrained charcoal stone, aged-bronze frames and a neutral stone-grey silhouette. The early dark-fantasy RPG feel is intentional; this is original generated art, not extracted game art. No inventory grid, statistics, portrait, buttons or additional gameplay systems were added.

**Approved for this public art handoff; not integrated or tested in-engine.**

## Start here

- `previews/equipment-320x240.png` — exact native-size baked visual reference.
- `previews/equipment-preview-4x.png` — 1280 × 960 nearest-neighbor review copy; do not use as the game texture.
- `layout.json` — measured native-pixel slot rectangles, 16 × 16 icon placements and label rectangles.
- `assets/panel-320x240.png` — clean panel background: no text, slots or figure.
- `assets/silhouette-54x124.png` — clean transparent figure, 54 × 124 pixels.
- `assets/slot-*.png` — seven cropped empty frames at their actual preview sizes, with opaque black interiors.
- `assets/label-*.png` and `assets/title-equipment.png` — exact opaque lettering crops for visual reference; runtime text is preferable.
- `assets/slot-20x20-compat.png` — optional nearest-resized 20 × 20 frame study for the existing renderer, not used by this preview.
- `source/` — original imagegen outputs and generation prompt record.
- `QA.json` — validation results and known limitations.

## Suggested integration approach

1. Keep a logical render surface of **320 × 240**; use integer display scaling and **nearest-neighbor**, never bilinear filtering.
2. For a quick approval mockup, draw the baked native preview as a single background and overlay item icons inside the seven `icon_rect` values in `layout.json`. Do not also draw another set of slot frames/labels.
3. For a flexible UI, draw the clean panel, then the transparent silhouette at `(133,75)`, then the seven cropped frames at their JSON coordinates. Draw title and labels with the engine's bitmap font. The label crops have opaque background and are not ideal for dynamic text/localization.
4. Center existing **16 × 16** item icons in the provided icon rectangles. The frame interior has room for them; do not scale icons to fill the whole frame.
5. Keep item data, hover/selection state and input logic separate from this art. No hover/focus/equipped/disabled state is supplied. Confirm those after visual approval rather than assuming these empty frames are a complete interactive UI.
6. Bind each ring to a distinct ID. `left_hand` and `right_hand` in this kit describe left/right **UI position**; explicitly map them to the game's chosen anatomical/main-hand/off-hand convention.

## Important repository mismatch

Read-only repository review reported the current default revision as `b9fdd98f37bcb419bcf2951103529ff571e5ea21`:

- `crates/zrpg/src/equipment.rs` reportedly uses a 320 × 240 equipment view, 20 × 20 slots with 16 × 16 icons, figure origin `(60,34)`, and a separate stats panel around `(207,13)` width `100` (verify current code before modifying).
- Existing assets are under `crates/zrpg/assets/equipment/` (`backdrop.png`, `figure.png`, `slot.png`, `empty/`, `gear/`).
- The current placeholder reportedly exposes **eleven** equipment slots, including gloves, belt, bow and quiver in addition to the seven requested here.

**This concept intentionally follows the user's seven-slot request and uses the entire screen without a stats panel. It is not a drop-in repaint of the existing eleven-slot layout.** Before integration, reconcile the UI/data mapping with the requested simplified design; do not silently hide or delete the extra equipment data or alter gameplay. Avoid overwriting existing assets merely because filenames look similar. No such changes are authorized or performed by this kit.

## Asset fidelity / limitations

The art was produced with the built-in image generator, then cropped/resized using Pillow. No code-drawn substitute UI art was used. The generated source is larger than the target and was normalized to 320 × 240; the 4× preview is a true nearest-neighbor expansion of that target. It is not hand-authored indexed-palette pixel art.

The clean panel and transparent figure were generated as separate matching edits. They are usable modular assets but **not pixel-identical reconstructions** of the baked preview. Cropped slot and label images include a little opaque local background; use the baked preview if exact pixel reproduction matters. The transparent silhouette's alpha was verified; other assets are intentionally opaque. The isolated silhouette used area/BOX downsampling to suppress colored generator edge speckles and has some partial-alpha edge pixels; review against the final game background. Its 4× inspection copy still uses nearest-neighbor enlargement.

This is an art-direction preview and starter asset kit, **not a tested production-ready UI**. Code mapping, readable in-engine text, interaction states and game testing remain.

## Rebuild from bundled sources

With Python and Pillow installed, run `python3 build_kit.py` from this folder. The helper reads only bundled `source/` files. It recreates the approved PNGs with nearest-neighbor normalization/cropping, except the clean silhouette uses its full alpha bounding box and BOX downsampling to 54 × 124; its inspection preview is then enlarged with nearest-neighbor. It does not fetch or generate new artwork.

The corrected helper was run in an isolated copy and every rebuilt/source PNG SHA-256 matched the approved originals. The approved artwork was not modified. See `REBUILD_VERIFICATION.json`. PNG byte reproducibility was observed with the recorded Pillow version; other encoders may produce different PNG byte streams even when pixels agree.
