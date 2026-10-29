> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Wild Savanna — material support notes

Four actual independent RGB diffuse textures: `wall`, `ground`, `structure`, `detail`, each at 256 and 128 pixels. The 2×2 atlas is a generation source, not a runtime scene texture. Geometry, layout and collision recipes are supplied by the shared-kit assembly; these texture notes do not substitute for meshes.

## Binding and use

- **wall** — flat weathered ochre sandstone with broad quiet cracks and earth-red stains. `materials/wall_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **ground** — flat dusty terracotta earth with small warm pebbles and fine dry cracks. `materials/ground_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **structure** — flat short sun-dried tawny savanna grass groundcover with muted olive undergrowth. `materials/structure_diffuse_256.png`. 0-1 per component.
- **detail** — flat gray-brown acacia bark with fine irregular grooves and subtle pale lichen. `materials/detail_diffuse_256.png`. 0-1 per component.

For water-bearing themes, `structure` is an **opaque painted water surface**. Do not accidentally bind it to tree trunks, columns or architecture. Bind bark-bearing props to `detail`, rock to `wall`, and the walkable lane to `ground`; use explicit per-mesh role remaps in the theme recipe. In forest, the bark is `wall` and leaf coverage is `detail`.

## Distinctive geometry / composition

Suggested shapes: acacia tree, dry grass clump, ochre boulder, mesa cliff, red-earth trail, fallen branch. These are assembly targets, not a claim that the material atlas contains those meshes. Preserve a central 1.2 m clear travel lane and at least 2.4 m headroom under arches/branches. Keep pools beside the path unless collision explicitly permits crossing.

Concept direction: An original vast untamed savanna at first-person eye level, evoking the emotional scale of a classic 16-bit wild continent without recreating any game location. A broad red-earth animal trail begins at bottom center and winds left through tawny grass toward distant violet mesas. A few flat-topped acacia trees with sculptural dark trunks, wind-worn ochre boulders, a low ridge and a huge luminous apricot-blue sky. Not desert dunes. Expansive quiet wilderness, no animals or people, a continuous obvious traversable ground route.

## Actual QA and limitations

- **wall**: Ochre color is coherent; broad red stain motifs repeat.
- **ground**: Red-earth gravel repeats quietly, though exact border pixels differ.
- **structure**: Grass forms a repeated diagonal pattern; clamp separate terrain/grass patches.
- **detail**: Vertical acacia grain repeats; clamp trunk components and orient grain vertically.

All four single surfaces and their actual 2×2 repeat diagnostics were visually inspected. There is no frame, UI, text, perspective scene, or synthetic normal map in the runtime textures. Opposing edges are not pixel-identical. `wall` and `ground` are repeat **candidates**, not certified seamless textures; `structure` and `detail` are non-repeat component panels. Use geometry breaks, separated rocks and component-local UVs to control joins. A strict seamless requirement needs a targeted imagegen repair and repeat reinspection, not a hidden programmatic edge blend. The common contract additionally recommends a 3×3 and moving-camera engine test before shipping.

Runtime source is sRGB diffuse with painted local shading, not physically calibrated albedo. Default roughness 0.85, metallic 0; no alpha, transparency or emission is implied by bright concept art. Start with nearest sampling; evaluate shimmer at 320×240.

## Provenance

Generated with built-in image_gen. Unmodified atlas in `art_sources/material_atlas_v1.png`; exact prompt in `prompts/material_atlas_v1.txt`. Only authorized crop, resampling and diagnostic repetition were applied. Concept master/native/4× preview are in the companion biome-previews directory. All artwork is newly generated original environmental interpretation; no third-party art is distributed.

The user inspiration was the feeling of a classic FFVI wild-continent savanna. This is an original acacia/mesa environment, not a copied map, screenshot, character or named game asset.
