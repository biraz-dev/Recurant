> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Redrock Mountain — material support notes

Four actual independent RGB diffuse textures: `wall`, `ground`, `structure`, `detail`, each at 256 and 128 pixels. The 2×2 atlas is a generation source, not a runtime scene texture. Geometry, layout and collision recipes are supplied by the shared-kit assembly; these texture notes do not substitute for meshes.

## Binding and use

- **wall** — flat warm red sandstone face with subtle horizontal strata and restrained cracks. `materials/wall_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **ground** — flat dusty red ochre gravel with small angular stones. `materials/ground_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **structure** — flat cool charcoal basalt with broad quiet fractured planes. `materials/structure_diffuse_256.png`. 0-1 per component.
- **detail** — flat iron mineral crust with sparse dull amber crystal flecks, not glowing lava. `materials/detail_diffuse_256.png`. 0-1 per component.

For water-bearing themes, `structure` is an **opaque painted water surface**. Do not accidentally bind it to tree trunks, columns or architecture. Bind bark-bearing props to `detail`, rock to `wall`, and the walkable lane to `ground`; use explicit per-mesh role remaps in the theme recipe. In forest, the bark is `wall` and leaf coverage is `detail`.

## Distinctive geometry / composition

Suggested shapes: layered cliff, angular boulder, rock arch, gravel ledge, basalt outcrop, mineral patch. These are assembly targets, not a claim that the material atlas contains those meshes. Preserve a central 1.2 m clear travel lane and at least 2.4 m headroom under arches/branches. Keep pools beside the path unless collision explicitly permits crossing.

Concept direction: A magnificent high red-rock mountain wilderness seen at adventurer eye level. A safe broad gravel switchback enters at bottom center and bends right between warm sculpted sandstone shoulders, passing a weathered natural arch to a layered jagged massif far away. Deep indigo canyons, warm high peaks, tiny dark basalt outcrops, soft drifting peach clouds. Majestic and dreamlike, but the walking route is unmistakable.

## Actual QA and limitations

- **wall**: Horizontal strata repeat as visible bands; hide module joins in rock breaks.
- **ground**: Gravel density is visually consistent across borders; scattered pebbles repeat.
- **structure**: Broad basalt cracks have border interruptions; clamp per rock component.
- **detail**: Orange crystal spots become an obvious repeated motif; use one-off mineral patches.

All four single surfaces and their actual 2×2 repeat diagnostics were visually inspected. There is no frame, UI, text, perspective scene, or synthetic normal map in the runtime textures. Opposing edges are not pixel-identical. `wall` and `ground` are repeat **candidates**, not certified seamless textures; `structure` and `detail` are non-repeat component panels. Use geometry breaks, separated rocks and component-local UVs to control joins. A strict seamless requirement needs a targeted imagegen repair and repeat reinspection, not a hidden programmatic edge blend. The common contract additionally recommends a 3×3 and moving-camera engine test before shipping.

Runtime source is sRGB diffuse with painted local shading, not physically calibrated albedo. Default roughness 0.85, metallic 0; no alpha, transparency or emission is implied by bright concept art. Start with nearest sampling; evaluate shimmer at 320×240.

## Provenance

Generated with built-in image_gen. Unmodified atlas in `art_sources/material_atlas_v1.png`; exact prompt in `prompts/material_atlas_v1.txt`. Only authorized crop, resampling and diagnostic repetition were applied. Concept master/native/4× preview are in the companion biome-previews directory. All artwork is newly generated original environmental interpretation; no third-party art is distributed.
