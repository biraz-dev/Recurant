# Recurant — 55-theme modular asset handoff / Essentials

**54 new theme kits + preserved existing sandstone reuse.** This compact package contains150 modular GLB entries (149 unique mesh payloads),54 actual texture-bound showcaseGLBs,256/128 runtime PNGs, native previews, concept references, selected contact sheets, collision/navigation recipes, editable mesh-authoring scripts and complete Claude integration notes.

Extract **all Essentials ZIP volumes into the same directory**. They are independent ZIPs; do not concatenate. Start with `CLAUDE_START_HERE.md`, `THEME_CATALOGUE.md` and a theme's `CLAUDE_README.md`. Every authoritative theme-manifest geometry/material/layout/preview path resolves within Essentials. Choose module instances OR assembledGLB, not both.

## Optional source-art bundle
All original generated material and concept masters, retained source revisions and the shared .blend are preserved in **Recurant_55_Theme_Kits_Source_Art**, stored once per SHA-256. Its safe restore helper recreates exact original source paths in this Essentials tree, including deliberate aliases. Download source volumes only when editing/rederiving source art; they are not required to open the GLBs or integrate the supplied runtime textures. No source pixels were modified for deduplication.

## Deliberately omitted derived diagnostics
Bulky per-theme repeat/contact PNGs, old QA candidate copies and800px per-theme overview PNGs remain in local authoring evidence, not these public/download packages. Their concise QA JSON/metrics and selected contact sheets are included. `assembly-src/render_material_qa.py` recreates current material diagnostics; Blender `assembly-src/render_themes.py` recreates actual-GLB overviews/native views. This is not a claim that every transient authoring file is distributed. No internal job/transfer records, caches or private workspace paths are included.

## Honest acceptance limits
This is a reusable low-poly art-authoring kit, not54 finished concept worlds. Shared mesh families and diagnostic layout are intentional. Repeat surfaces are visually reviewed candidates, not mathematically seamless certified. No fabricated normal/roughness/metallic/AO/height maps. Current Recurant uses customlevel.bin and needs an explicit importer/material adapter; no game code, shader, controller or runtime integration was changed or tested. Collision guides and explicit grid routes are authoring evidence, not gameplay physics. Profile motion/shimmer and target devices separately.

The one-bit theme supplies exact black/white textures and optionalKHR_materials_unlit; its readable actual-render previews additionally use geometry-derived contours and binary Bayer dithering. Those presentation passes are not baked into the GLB or implemented in Recurant. A normal GLB viewer may look different; ordinary lit-gray fallback is not strict one-bit.

Existing sandstone is an explicit preserved repository dependency, not newly rebundled source. Read `themes/sandstone-ruins-existing/REUSE_EXISTING.md` for exact extraction and hash references. Verify package files with `sha256sum -c SHA256SUMS.txt`; read `VALIDATION_REPORT.json` and `QA_VISUAL_REVIEW.md` before import.
