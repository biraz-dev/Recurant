> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Tidal Island — material support notes

Four actual independent RGB diffuse textures: `wall`, `ground`, `structure`, `detail`, each at 256 and 128 pixels. The 2×2 atlas is a generation source, not a runtime scene texture. Geometry, layout and collision recipes are supplied by the shared-kit assembly; these texture notes do not substitute for meshes.

## Binding and use

- **wall** — flat pale lilac gray weathered coastal basalt with subtle salt wear. `materials/wall_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **ground** — flat cream coral sand with fine wind ripples and a few tiny shell granules. `materials/ground_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **structure** — flat shallow clear turquoise water ripple surface, no objects or landscape reflections. `materials/structure_diffuse_256.png`. 0-1 per component.
- **detail** — flat pale porous coral limestone with tiny pearl and peach mineral pores. `materials/detail_diffuse_256.png`. 0-1 per component.

For water-bearing themes, `structure` is an **opaque painted water surface**. Do not accidentally bind it to tree trunks, columns or architecture. Bind bark-bearing props to `detail`, rock to `wall`, and the walkable lane to `ground`; use explicit per-mesh role remaps in the theme recipe. In forest, the bark is `wall` and leaf coverage is `detail`.

## Distinctive geometry / composition

Suggested shapes: sea cliff, basalt sea stack, coral clump, coastal grass, sand causeway, tidal pool. These are assembly targets, not a claim that the material atlas contains those meshes. Preserve a central 1.2 m clear travel lane and at least 2.4 m headroom under arches/branches. Keep pools beside the path unless collision explicitly permits crossing.

Concept direction: A luminous remote tidal island at first-person eye level. A broad pale-sand path begins in the foreground and follows a dry curved causeway around a clear turquoise tidal lagoon toward an inland gap between pale lavender sea cliffs. Weathered basalt stacks, shallow reef pools, coastal grasses and pearl-colored rock, distant low islands and great soft clouds. No boats or buildings. Quiet wonder, clean traversable path, water kept beside the route.

## Actual QA and limitations

- **wall**: Some horizontal edge/value discontinuity is visible in the 2x2 diagnostic; conceal joins with separate rock forms.
- **ground**: Fine sand ripple direction and brightness are consistent; exact grains do not match.
- **structure**: Fine ripple seams and periodic highlights visible; clamp individual pool panels.
- **detail**: Pore pattern is quiet but repeats; suitable for coral-rock components, no coral mesh implied by texture.

All four single surfaces and their actual 2×2 repeat diagnostics were visually inspected. There is no frame, UI, text, perspective scene, or synthetic normal map in the runtime textures. Opposing edges are not pixel-identical. `wall` and `ground` are repeat **candidates**, not certified seamless textures; `structure` and `detail` are non-repeat component panels. Use geometry breaks, separated rocks and component-local UVs to control joins. A strict seamless requirement needs a targeted imagegen repair and repeat reinspection, not a hidden programmatic edge blend. The common contract additionally recommends a 3×3 and moving-camera engine test before shipping.

Runtime source is sRGB diffuse with painted local shading, not physically calibrated albedo. Default roughness 0.85, metallic 0; no alpha, transparency or emission is implied by bright concept art. Start with nearest sampling; evaluate shimmer at 320×240.

## Provenance

Generated with built-in image_gen. Unmodified atlas in `art_sources/material_atlas_v1.png`; exact prompt in `prompts/material_atlas_v1.txt`. Only authorized crop, resampling and diagnostic repetition were applied. Concept master/native/4× preview are in the companion biome-previews directory. All artwork is newly generated original environmental interpretation; no third-party art is distributed.
