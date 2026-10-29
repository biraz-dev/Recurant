> Compact-delivery note: original source masters/.blend require the optional Source Art restore. Historical repeat/contact PNG references below are reviewed authoring evidence, not bundled runtime dependencies; regenerate current diagnostics with assembly-src/render_material_qa.py.

# Crystal Machine Ship — diffuse materials

Four original face-on painted materials, generated with built-in image_gen. Actual texture files, not concept scenes. Source atlas and exact prompts retained.

## Files and mapping
- `materials/{wall,ground,structure,detail}_diffuse_256.png`: 256×256 RGB PNG, opaque, sRGB.
- Corresponding `_128.png`: nearest-neighbor low-resolution copies.
- Material IDs: `crystalline-machine-sanctuary__<role>`. Roles match the shared modular geometry library.
- Wall/ground: practical two-axis repeat at 2m nominal scale, visually checked in 2×2 repeats. Use REPEAT. They are **not pixel-perfect seamless-certified**; see the honest notes below. Cover visible module boundaries with trims if needed.
- Structure/detail: **nonrepeat panels**, CLAMP_TO_EDGE; keep UVs within0–1 or crop a suitable material region. Do not automatically tile these decorative panels.

## Visual QA
- Wall: Industrial panel breaks work as intentional module joins; repeat unit is recognizable.
- Ground: Violet floor grid aligns coherently; minor line-weight shifts at boundaries.
- Four distinct face-on surface treatments; no scene perspective or labels. Some surfaces intentionally have painted micro-highlights/bevel marks, appropriate to the stylized diffuse look but not physically calibrated.
- `qa/material_contact_sheet.png` shows all four native files; `qa/<role>_repeat_2x2.png` provides real repeats, including diagnostic repeats of nonrepeat panels.
- Edge-difference measurements in `material-manifest.json` are diagnostics, not proof of seamlessness.

## Engine treatment
Base color/diffuse only. No normal, roughness, metalness, height, opacity, or emission map is supplied or implied. In-engine reflection/emission must be authored separately. Bright glass and ember colors are painted diffuse pigments, not an emission map.

Prefer nearest texture sampling for crisp low-resolution painterly pixels. No lossy compression during QA. Test mipmapping in the actual target engine; manifests do not imply mip-safe atlases because independent textures, not the source atlas, are the production inputs.

## Provenance and processing
Only crop and nearest-neighbor resizing were used after built-in image generation. Four exact quadrant crops of the retained source atlas supply the final textures. No programmatic seam painting or fabricated PBR derivatives. `material-manifest.json` identifies the selected source and prompt for every material.
