# Tidal Island — Claude handoff

**Status:** real modular geometry + four generated diffuse material candidates + texture-bound diagnostic GLB. Not runtime-integrated or shipping-certified.

## Start here
- Read `common/spec/asset_contract.json`, `common/spec/COLLISION_AND_IMPORT.md` and this theme's `theme-manifest.json` from the extracted package root.
- Open `assembly/showcase.glb` to inspect actual geometry and textures. It contains 1,395 triangles / 3 material primitives, with embedded PNGs and identity root transform.
- Rebuild from `assembly/layout.json` and shared `common/geometry/meshes/` **or** import the assembled GLB. Never both.
- Paths in normalized theme/assembly manifests are relative to **package root**; paths in the raw material manifest are relative to this theme folder. Shared geometry manifest files are relative to `common/geometry/`.

## Design and palette
turquoise lagoon, pale coral sand, lilac basalt, sea-glass blue, soft warm cloud light

This is a **landscape** kit. Distinctive supplied forms: boulder large, coral branch, coral shelf cluster, shore bank 2m, rock slab, standing stone, boulder small, coral branch. The collection uses reusable silhouette families but not a box-only recolour: the arch, supports, furniture, rock, plant or machine elements are actual geometry. The 6 × 10 m cutaway is an inspectable assembly, not a reproduction of the larger concept scene. It omits roof/front enclosure to expose the assets.

## Modularity and safe placement
- 2 m authoring grid; +Y up; +X east; −Z north; scale to game by **0.5 exactly once**. Most structural bays are 2 m; read each module's real bounds/connectors/pivot. Do not normalize every mesh or auto-centre it.
- Centre lane is X=3 m, width 1.2 m, headroom 2.4 m. Five cardinal cells connect start [1,0] to goal [1,4]. Side cells are intentionally blocked exhibits, not traversable furnished rooms.
- `assembly/collision-navigation.json` supplies transformed compound boxes and exact allowed edges. They are authoring guides only, not a collision engine. Doorway voids must remain open. No swimming, climbing, animated machinery, hazards, furniture interactions or door-state logic is supplied.
- Small exhibition props use explicit uniform instance scales to fit side pads; modules themselves retain authored metre dimensions. Remaps in `material_role_overrides` are intentional and must be applied before batching an instance recipe. UVs are rebound with the target role: metric dominant-normal projection /2 for wall/ground; each disconnected face island normalized 0–1 for structure/detail. The assembly source implements this rule, preventing clamp streaks.

## Materials and UV status
| Role | Actual surface | UV / wrap |
|---|---|---|
| wall | flat pale lilac gray weathered coastal basalt with subtle salt wear | metric 2 m repeat candidate |
| ground | flat cream coral sand with fine wind ripples and a few tiny shell granules | metric 2 m repeat candidate |
| structure | flat shallow clear turquoise water ripple surface, no objects or landscape reflections | component 0–1 panel clamp |
| detail | flat pale porous coral limestone with tiny pearl and peach mineral pores | component 0–1 panel clamp |

All four runtime candidates are opaque RGB PNG, sRGB, 256 × 256; 128 × 128 alternatives included. These are painted diffuse/base-colour surfaces, not physically measured albedo. **No normal, AO, height, roughness, metallic or emission maps supplied.** Defaults are metallic 0 / roughness 0.85, including stylized metal themes.

Generated wall/ground opposing edges are **not certified seamless**. Consult `material-manifest.json`, material notes and repeat QA. This assembly intentionally exposes repetition; use seams, panel breaks or further authored material revisions before long uninterrupted surfaces. Structure/detail panels clamp. Some UVs are face-projected rather than hand-authored trim islands; disconnected flat-shaded components are intentional low-poly construction. Do not claim lightmap-ready or watertight geometry.

## Claude integration checklist
1. Recheck current repository HEAD and local instructions; this handoff observed b9fdd98 on 2026-10-29 without changing code.
2. Existing game imports custom `level.bin`, not GLB at runtime. Its converter only knows the old five sandstone slots and ignores module node transforms. Add/adapt an offline converter deliberately; four named roles here must not be guessed from numeric material order.
3. Apply embedded textures for DCC preview; bind external PNGs by normalized manifest for the game adapter. Retain authored UV0/normals and bake any later node transforms.
4. Build `zrpg_core::Map` occupancy and symmetric N/E/S/W masks (1/2/4/8) from explicit cells/edges. Missing adjacent allowed edges are forbidden. Sight/attack occlusion is a separate check.
5. Use tall profile for 2.4 m doorway clearance, not 2 m dungeon ceiling. Test camera eye/back-offset, near plane and all look directions. Never silently squash tall assets.
6. Validate at native 320 × 240 and while moving; nearest magnification may shimmer at distance. Test mipmapped minification before shipping.
7. Run repository tests and native/browser builds when authorized. Only then mark runtime gate passed. This package's previews cannot establish target-device performance.

## Extend this theme
Use the exact existing four-role names and add a module to the shared geometry manifest with bounds, pivot, UVs, compound collision guides, connectors and measured triangle count. Mix or duplicate the listed modules into new rooms; keep lane-affecting scenery outside allowed routes. A new layout needs its own connected navigation proof and opening checks. Keep each primitive under 65,535 vertices / uint16 indices. Add optional data maps only from a real approved authoring process, never by relabeling diffuse art.

## Provenance and limits
Original image-generated artwork and original procedural mesh authorship created for this project. No downloaded third-party mesh pack or copied source-world assets are bundled. No exclusive copyright, trademark clearance or third-party licence grant is asserted for generated output; review project-specific release requirements. Legacy sandstone is a separate pre-existing asset with its own provenance, not overwritten by this theme.

Asset-level checks: relative references, readable PNGs/GLBs, finite vertex buffers, identity nodes, four named bindings, 16-bit indices, calculated bounds, lane guide clearance and connected declared route. Visual rendering and repeat checks are recorded separately. **Runtime, controller, dynamic doors, target performance, movement shimmer and production scene completeness remain unverified.**

## Compact delivery scope
This essentials package includes actual meshes, bound runtime PNGs, native preview and QA metadata. Full-resolution source art and the shared .blend are optional: restore them from the separate Source Art bundle before rebuilding textures. Detailed repeat PNGs and 800px per-theme overviews are not shipped; generate fresh diagnostics with the supplied authoring scripts. Historical QA paths in material records describe the reviewed authoring evidence, not required runtime dependencies.
