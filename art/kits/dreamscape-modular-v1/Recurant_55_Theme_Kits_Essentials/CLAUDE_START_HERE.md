# Claude: integration is a deliberate next step

Do not blindly copy this folder over the repository. Read current project instructions and HEAD first. This asset task did not modify repository code.

1. Run the asset integrity check: from extracted root, `sha256sum -c SHA256SUMS.txt`. Review `VALIDATION_REPORT.json` and `QA_VISUAL_REVIEW.md`.
2. Pick one new theme, and inspect `theme-manifest.json`, `CLAUDE_README.md`, the actual GLB and native image. The normalized manifest base is package root; raw material manifests are theme-folder-relative; shared geometry files are geometry-folder-relative.
3. Decide one import route. The GLB already bakes all placement transforms and embeds four real PNGs. A module route must apply `material_role_overrides`, exact explicit instance transforms and material names. Avoid double-spawning both routes.
4. Preserve metres/+Y/grid/pivots. Convert to game scale 0.5 once. Structural modules may have documented profile/pivot exceptions; exact geometry manifest wins over naming assumptions. Do not auto-centre. **Actual ceiling exception:** ceiling_flat/coffer use a room-floor pivot with underside near Y=4 m; barrel-vault spring is also Y=4 m. Do not add a second 4 m offset merely because the proposal described an underside pivot. Exact per-module metadata overrides the proposed default.
5. Current converter recognizes only legacy sandstone names and ignores node transforms; it writes custom `level.bin`. A new explicit offline adapter is required. New roles are wall, ground, structure, detail. Embedded GLB textures are for preview; bind external PNGs deliberately.
6. Create the core grid map from explicit cells and allowed cardinal edges, making all block masks symmetric. Side display cells are intentionally blocked. Compound door boxes leave holes open; never use whole arch bounds. Render geometry is not gameplay authority.
7. Tall door clearance is 2.4 authoring metres; a 2 m dungeon ceiling is incompatible. Test eye-height, all headings, camera back offset/near clip and seams in motion.
8. Run authorized repository tests, native/browser loading and actual target-device profiling. Keep asset checks separate from runtime acceptance. Do not call this plug-and-play, final production art or runtime-tested before evidence exists.

## Rebuild / extend offline
Tested offline with Python 3.12.14, NumPy 2.3.5, Pillow 12.3.0, jsonschema 4.26.0 and Blender 4.3.2 on this authoring machine. This is an observed toolchain, not a compatibility promise for every version. Python requires NumPy, Pillow and jsonschema. Blender is used only for scene render QA. Source scripts can be run from this extracted root; `python3 assembly-src/assemble_themes.py <theme-id>` assembles from existing mesh and material files, and Blender background `--python assembly-src/render_themes.py -- <theme-id>` renders the real GLB. New modules require geometry-manifest entries with actual bounds/pivots/connectors/UVs/collision boxes and measured counts. New routes require connected explicit navigation and lane/opening checks.

Texture role changes are not fake PBR. Author any optional normal/ORM maps separately and update manifests explicitly. Do not derive unverified maps from color merely to fill missing slots.

## Optional source and regeneratable views
In this compact Essentials edition, source masters and the editable .blend are in the separately hashed Source Art bundle. Restore that bundle first for source-level rebuilds. Native previews/contact summaries are included; large per-theme overviews and historical material repeat PNGs are not bundled and can be regenerated with the supplied render helpers.
