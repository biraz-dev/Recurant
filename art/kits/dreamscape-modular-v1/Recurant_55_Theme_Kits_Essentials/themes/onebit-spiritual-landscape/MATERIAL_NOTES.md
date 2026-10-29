> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# One-bit Spiritual Landscape — material delivery

Original quiet, non-denominational landscape language: pale open void, isolated weathered monoliths, stepping stones, a low pedestal and an empty ring/arch. No borrowed sacred iconography, people, text, UI, or named fictional world.

## Supplied assets

- Four actual opaque RGB PNG color textures: `wall`, `ground`, `structure`, `detail`, each at 256×256 and 128×128.
- Every delivery texture contains **exactly two RGB colors**: `(0,0,0)` and `(255,255,255)`; no alpha channel. “1-bit” describes the visual palette, not PNG storage bit depth.
- Wall and ground are repeat candidates; structure and detail are local 0–1 clamp panels. Use the four material slots in `common/spec/asset_contract.json`.
- Original built-in imagegen atlas in `art_sources/material-atlas-master.png`; prompts in `prompts/`. No PBR data maps are fabricated or implied.
- Actual wall/ground 2×2 and 3×3 repeat tests and a four-role contact sheet are in `qa/`.

## Processing and provenance

Both concept and texture atlas were generated with the built-in image generator using explicit black/white-only prompts. The originals nevertheless contain antialiased/near-neutral values and are retained unchanged. The user's explicit “1bit” request authorizes the necessary final palette conversion.

`derive_assets.py` reproduces only deterministic delivery operations: exact quadrant crops, nearest-neighbour 256px material resize, luminance threshold 128, RGB encoding, and nearest-neighbour 128px variants. No seam painting or procedural replacement art was used. The concept uses an uncropped 4:3 BOX downsample to 320×240, threshold 160, and exact 4× nearest-neighbour preview. QA records source and final hashes/color counts.

## Visual review and limits

Wall and ground have been inspected in actual 2×2 and 3×3 repetitions. They have a continuous granular/stipple impression without a conspicuous hard seam, but repeating density patches remain visible. Edge pixels are not mathematically identical: **not certified seamless**. The structure's strong cracks and the detail's dark carved hatching are intentionally non-repeating panels. Fine pixel marks may shimmer in motion; camera-motion testing is not supplied.

## Keeping the rendered image one-bit

The textures alone **do not guarantee a one-bit rendered frame**. Use unlit color materials (for example supported `KHR_materials_unlit`) with neutral white factors and nearest minification/magnification, a black/white background, and no gray-producing fog/lighting, tone mapping, interpolated texture filtering, or antialiasing. Where the render pipeline cannot guarantee that, apply a final binary luminance threshold to the final rendered frame after all shading/postprocessing. Normal lit PBR fallback is not strictly one-bit.

The shared modular assembly may demonstrate an unlit proposal and thresholded stills. These are asset-preview evidence only: no game shader, import adapter, collision system, scene, or rendering pipeline has been integrated or tested in Recurant. Respect existing common geometry, grid, pivots and collision intent rather than treating the pictured floating path as confirmed traversal logic.

## Shared geometry intent

Suitable original shared modules include standing stones, rock slabs, crystal obelisks, ceremonial pedestals, empty ring geometry, and simple arches. Use a sparse open composition and keep the central travel lane readable. Geometry/assembly manifests are maintained by the package assembler; do not import both layout instances and the assembled GLB.
