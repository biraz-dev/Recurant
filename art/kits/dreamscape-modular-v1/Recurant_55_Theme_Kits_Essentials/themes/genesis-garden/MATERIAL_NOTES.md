> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Genesis Garden — material support notes

Four actual independent RGB diffuse textures: `wall`, `ground`, `structure`, `detail`, each at 256 and 128 pixels. The 2×2 atlas is a generation source, not a runtime scene texture. Geometry, layout and collision recipes are supplied by the shared-kit assembly; these texture notes do not substitute for meshes.

## Binding and use

- **wall** — flat muted ultramarine mineral stone with fine branching ochre-gold veins. `materials/wall_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **ground** — flat pale opal limestone with quiet blue-gray hairline cracks and faint warm mineral specks. `materials/ground_diffuse_256.png`. 1 repeat per 2 authoring metres, only after seam acceptance.
- **structure** — flat luminous deep cerulean water surface with fine soft gold ripple streaks, no reflected scene. `materials/structure_diffuse_256.png`. 0-1 per component.
- **detail** — flat deep teal living bark with restrained branching gold mineral fibers. `materials/detail_diffuse_256.png`. 0-1 per component.

For water-bearing themes, `structure` is an **opaque painted water surface**. Do not accidentally bind it to tree trunks, columns or architecture. Bind bark-bearing props to `detail`, rock to `wall`, and the walkable lane to `ground`; use explicit per-mesh role remaps in the theme recipe. In forest, the bark is `wall` and leaf coverage is `detail`.

## Distinctive geometry / composition

Suggested shapes: branching mineral tree, floating rock shelf, pale stone terrace, pool surface, waterfall plane, mineral standing stone. These are assembly targets, not a claim that the material atlas contains those meshes. Preserve a central 1.2 m clear travel lane and at least 2.4 m headroom under arches/branches. Keep pools beside the path unless collision explicitly permits crossing.

Concept direction: An original surreal primordial creation-garden at first-person eye level, environmental concept rather than an illustration of any existing card. A broad solid pale mineral path curves from bottom center LEFT through shallow azure pools and soft jade terraces. On the RIGHT, a colossal living mineral tree unfurls branching golden light into a vast blue cavernous sky; fluid ribbons of light and cloud seem to seed new foliage across the valley. Far away, irregular floating stone shelves and gentle waterfalls. Asymmetrical composition, winding navigable route, no central triangle, no figures, no humanoid forms. Serene ecstatic wonder, blue-and-gold resonance, luxurious painterly pixel colors.

## Actual QA and limitations

- **wall**: Gold mineral veins are discontinuous at borders and repeat visibly; not seamless-certified.
- **ground**: Pale stone value is coherent, but long mineral cracks reveal repeated boundaries.
- **structure**: Gold ripple lines reveal repetition; opaque painted fluid only, no physical emission.
- **detail**: Branching gold bark forms a strong repeated motif; clamp each hero tree component.

All four single surfaces and their actual 2×2 repeat diagnostics were visually inspected. There is no frame, UI, text, perspective scene, or synthetic normal map in the runtime textures. Opposing edges are not pixel-identical. `wall` and `ground` are repeat **candidates**, not certified seamless textures; `structure` and `detail` are non-repeat component panels. Use geometry breaks, separated rocks and component-local UVs to control joins. A strict seamless requirement needs a targeted imagegen repair and repeat reinspection, not a hidden programmatic edge blend. The common contract additionally recommends a 3×3 and moving-camera engine test before shipping.

Runtime source is sRGB diffuse with painted local shading, not physically calibrated albedo. Default roughness 0.85, metallic 0; no alpha, transparency or emission is implied by bright concept art. Start with nearest sampling; evaluate shimmer at 320×240.

## Provenance

Generated with built-in image_gen. Unmodified atlas in `art_sources/material_atlas_v1.png`; exact prompt in `prompts/material_atlas_v1.txt`. Only authorized crop, resampling and diagnostic repetition were applied. Concept master/native/4× preview are in the companion biome-previews directory. All artwork is newly generated original environmental interpretation; no third-party art is distributed.

The user inspiration was *Genesis Garden*, original Ikoria card illustration credited to **Noah Bradley**. Inspected the [official Wizards card gallery](https://magic.wizards.com/en/news/card-image-gallery/ikoria-lair-behemoths) and its [official card image](https://media.wizards.com/2020/iko/en_338C38MclG.png) on 2026-10-29. Reference observations: blue/gold contrasts and branching, world-forming energy. The generated **Genesis Garden** uses a new asymmetrical path, a side-positioned mineral tree and stepped water gardens, not the source card composition, central figure or triangular focal motif. External reference image was viewed only and is not packaged.
