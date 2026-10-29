> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Blackwater Swamp — material support notes

Four actual independent RGB diffuse textures: `wall`, `ground`, `structure`, `detail`, each at 256 and 128 pixels. The 2×2 atlas is a generation source, not a runtime scene texture. Geometry, layout and collision recipes are supplied by the shared-kit assembly; these texture notes do not substitute for meshes.

## Binding and use

- **wall** — flat dark peat-stained gray rock with small patches of jade moss. `materials/wall_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **ground** — flat damp umber mud with fine root fibers and scattered tiny pebbles. `materials/ground_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **structure** — flat nearly black teal still-water ripples with restrained turquoise glints, no reflections of objects. `materials/structure_diffuse_256.png`. 0-1 per component.
- **detail** — flat deeply furrowed gray cypress bark with muted green lichen. `materials/detail_diffuse_256.png`. 0-1 per component.

For water-bearing themes, `structure` is an **opaque painted water surface**. Do not accidentally bind it to tree trunks, columns or architecture. Bind bark-bearing props to `detail`, rock to `wall`, and the walkable lane to `ground`; use explicit per-mesh role remaps in the theme recipe. In forest, the bark is `wall` and leaf coverage is `detail`.

## Distinctive geometry / composition

Suggested shapes: cypress trunk and roots, fallen log, mossy standing stone, cattail clump, raised earth causeway, shallow pool surface. These are assembly targets, not a claim that the material atlas contains those meshes. Preserve a central 1.2 m clear travel lane and at least 2.4 m headroom under arches/branches. Keep pools beside the path unless collision explicitly permits crossing.

Concept direction: A haunting ancient blackwater swamp seen from the adventurer's eye level. A broad dry root-and-earth causeway begins at the bottom center, winds gently left around a pool, then passes between enormous leaning cypress trunks into pale gold mist. Black mirror water, hanging moss, jade ferns, and pale weathered standing stones. An expansive, beautifully designed navigable place, not a cramped corridor.

## Actual QA and limitations

- **wall**: Small lichen clusters repeat visibly, but no strong brightness stripe.
- **ground**: Root fragments terminate at borders; distributed mud value remains consistent.
- **structure**: Water ripples repeat quietly; this remains an opaque painted water panel, not simulated water.
- **detail**: Vertical bark bands repeat conspicuously; clamp to individual trunk/root components.

All four single surfaces and their actual 2×2 repeat diagnostics were visually inspected. There is no frame, UI, text, perspective scene, or synthetic normal map in the runtime textures. Opposing edges are not pixel-identical. `wall` and `ground` are repeat **candidates**, not certified seamless textures; `structure` and `detail` are non-repeat component panels. Use geometry breaks, separated rocks and component-local UVs to control joins. A strict seamless requirement needs a targeted imagegen repair and repeat reinspection, not a hidden programmatic edge blend. The common contract additionally recommends a 3×3 and moving-camera engine test before shipping.

Runtime source is sRGB diffuse with painted local shading, not physically calibrated albedo. Default roughness 0.85, metallic 0; no alpha, transparency or emission is implied by bright concept art. Start with nearest sampling; evaluate shimmer at 320×240.

## Provenance

Generated with built-in image_gen. Unmodified atlas in `art_sources/material_atlas_v1.png`; exact prompt in `prompts/material_atlas_v1.txt`. Only authorized crop, resampling and diagnostic repetition were applied. Concept master/native/4× preview are in the companion biome-previews directory. All artwork is newly generated original environmental interpretation; no third-party art is distributed.
