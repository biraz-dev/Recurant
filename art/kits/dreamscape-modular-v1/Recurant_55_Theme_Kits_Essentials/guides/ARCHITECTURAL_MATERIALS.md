# Architectural / surreal material handoff

## Contents

**15 themes, 60 distinct diffuse materials, 120 production PNG files.** Four roles per theme: `wall`, `ground`, `structure`, `detail`. Each role has 256×256 and 128×128 versions. All files are opaque RGB interpreted as sRGB. Built-in image generation was used, not the API/CLI.

Files: `themes/<slug>/materials/<role>_diffuse_256.png` (or `_128.png`). Read each `material-manifest.json` before binding. Exact source crop rectangles, generation prompts, repair prompts where used, source masters and per-role QA are included.

## Import / UV contract

- `wall` and `ground`: practical repeated diffuse surfaces; target 2 metres per texture repeat, UV0 metric planar. All 30 were viewed in a 2×2 repeat. **These are visually usable stylized repeats, not mathematically edge-identical seamless textures.** Slight mortar width, mineral grain, stitch phase and color variations remain. Break very long expanses with structural meshes; inspect the specific material at final camera distance.
- `structure` and `detail`: intentional nonrepeat component surfaces and decorative panels. Map UV0 0–1 once per local component and CLAMP. Do not blindly apply the wall 2m repeat rule to these panels.
- Use nearest/point magnification to preserve pixel clusters; test nearest-mipmap sampling for distance, especially woven materials. Avoid lossy texture compression for final pixel fidelity.
- No alpha, sprite silhouettes, object meshes or fake PBR channels are hidden in these files. No normal, roughness, metallic, height, AO, opacity or emissive maps were generated. Artist-painted relief shading is baked into diffuse color and is not real geometric relief.
- Treat illustrated bottle/flower/celestial motifs as surface decorations, not object sprites. In particular, bottle, rug, shoji lattice, embroidery and relief silhouettes still need actual mesh geometry if they affect the silhouette.

## QA and provenance

- `qa/architectural/all-15-materials-contact.png`: 15-theme overview, four material samples per card.
- Each `themes/<slug>/qa/material-contact.png`: four 256px material samples.
- Each `themes/<slug>/qa/wall-ground-repeat-2x2.png`: actual repeated wall/ground test images.
- `qa/architectural/validation-report.json`: size/mode/presence and uniqueness checks. 60 unique 256px hashes, 120 valid production PNGs, no errors.
- Only crop/nearest resize was performed on generated production pixels. Some crops align whole brick courses or motif periods, rather than blindly splitting the atlas at 50%. Astral atlas quadrants were slightly uneven; its actual boundary was respected. Source masters are intact.
- Alabaster, crypt, dingy cellar, dream city, and embroidered world received one targeted built-in image generation seam correction each; originals and revised masters are retained.
- `extract_materials.py` is the documented basic extraction workflow, not a seam-repair algorithm. Its config and original sources are preserved for reproducibility. Final manifests include additional import/provenance notes; do not run extraction casually over final outputs.

## Per-theme material / prop application hints

### alabaster-temple
- Wall: Ivory alabaster ashlar with pale rose veins.
- Ground: Pale cream marble square tiles with fine champagne-gold grout.
- Structure: Ivory marble with repeated gold leaf bands.
- Detail: Nonrepeat sun-and-petal alabaster relief panel.

### crypt-catacombs
- Wall: weathered gray-purple limestone ashlar in horizontal staggered courses, fine chipped edges, quiet recess joints.
- Ground: worn lavender slate paving in square tiles, mottled quiet blue-gray with small chips, dark thin grout.
- Structure: pale carved limestone with restrained interlocking funerary leaf band in horizontal courses, material only.
- Detail: square shallow relief rosette and funerary knot in aged limestone, no skull object and no text.

### dingy-cellar
- Wall: Old ochre-coated stone/brick masonry with chipped plaster residue in regular courses.
- Ground: weathered dark sandstone rectangular paving, muted umber and gray, orthogonal grid with irregular joints.
- Structure: aged dark brown oak boards, straight grain and small nail marks, full square wooden surface.
- Detail: square patch of old woven burlap sackcloth with a subtle stitched central grain-sheaf motif, nonrepeating flat textile.

### dream-city-alleys
- Wall: Sunbleached peach and apricot terracotta masonry with low-contrast plaster-like surface wear.
- Ground: warm buff and terra-cotta small cobblestone paving in orderly repeating rows, strictly top-down, no grass object.
- Structure: weathered teal-painted timber narrow planks with pale exposed wood grain, flat diffuse.
- Detail: square ornamental glazed ceramic tile panel, original interlaced star-and-flower design in dusty teal mustard and coral, nonrepeating.

### subterranean-elven-realm
- Wall: deep blue-black dressed obsidian masonry with restrained violet mineral veins, rectangular low-relief blocks.
- Ground: dark indigo polished stone square paving with thin pale lilac geometric inlay, matte diffuse no reflected light.
- Structure: deep charcoal carved stone with thin repetitive elegant angular silver-violet tracery, absolutely no raised architectural object.
- Detail: square subtle amethyst and silver spiderweb-like abstract radial sigil in a dark obsidian surface, ornamental pattern not lettering, no glow.

### gothic-night-castle
- Wall: elegant cold blue-gray limestone ashlar, subtle fine veins and refined wear, regular rectangular masonry.
- Ground: pale blue-gray marble tiled floor with narrow charcoal borders in an orthogonal grid, no reflections.
- Structure: carved gray stone with repeating fine gothic foliate ornament, shallow relief on flat square surface.
- Detail: flat deep crimson damask textile with antique gold ornamental pomegranate-and-thorn brocade, no curtain folds.

### palace-hallways
- Wall: refined warm ivory plaster and pale sandstone ashlar, subtle rose undertones, elegant clean blocks.
- Ground: orthogonal multicolor inlaid marble mosaic tessellation in cream ochre pale rose and muted teal, simple original palace pattern.
- Structure: pale rose marble with slender gilded foliate carved trim in repeated horizontal bands, flat material.
- Detail: square decorative sage-and-gold arabesque textile or painted plaster medallion, balanced nonrepeating original floral geometry.

### potion-cellar
- Wall: warm dark rough limestone blocks with fine dry moss stains in joints, subdued alchemical cellar masonry.
- Ground: worn charcoal and amber stone pavers, orthogonal flat floor pattern, subtle old mineral stains.
- Structure: aged honey-brown wooden worktop planks with fine end-grain, small knife scratches and quiet dusty wear.
- Detail: square flat painted ceramic apothecary decorative panel with nonlettered interlaced leaf and bottle-shaped ornamental silhouettes in moss and muted violet, no actual standing bottles.

### tatami-house
- Wall: warm cream fibrous rice-paper shoji surface with narrow dark wooden horizontal and vertical lattice, exact regular square repeat.
- Ground: straw-gold finely woven tatami mat surface, parallel natural fiber pattern, restrained jade edging only as a flat repeating woven band.
- Structure: warm aged cedar plank surface with straight quiet grain, refined traditional wooden structural material.
- Detail: square indigo and sage dyed textile panel with a restrained original maple leaf and stream motif, flat cloth, no folds.

### tavern-inn
- Wall: warm gray rubble stone masonry in muted amber brown, visible mortar with handmade irregular blocks.
- Ground: warm dark oak floorboards laid flat in staggered joints, subtle fine grain and wear, no perspective.
- Structure: dark walnut structural wood surface with hand-carved shallow vine-and-chevron strip, no beam object.
- Detail: square flat burgundy wool rug with muted ochre and cream geometric border and original central diamond motif, no folds.

### lost-toy-world
- Wall: oversized weathered painted wooden toy block masonry, square and rectangular faded blue red ochre blocks with scuffs, no letters.
- Ground: flat worn toy-board tiled floor in dusty coral sage blue and mustard, modest handpainted chips and wood grain.
- Structure: unpainted aged honey wooden toy surface with straight grain and very subtle rubbed edges, material not object.
- Detail: flat square patchwork felt panel in muted plum honey and faded blue with a simple stitched four-point star and small round button shapes stitched onto the surface, no actual dimensional objects.

### embroidered-world
- Wall: Deep indigo knitted cloth carrying separate small gold floral sprigs in a regular repeat.
- Ground: Warm ivory woven cloth with separated gold diamond medallions and tiny lavender flowers.
- Structure: thick golden braided yarn textile arranged in orderly horizontal cable-knit rows, no freestanding yarn or scenery.
- Detail: square hand-embroidered lavender and gold floral mandala on indigo cloth, visible tiny stitches, nonrepeat flat textile panel.

### astral-sanctuary
- Wall: pearl-white alabaster blocks with delicate blue-lavender veins, very refined thin joints, not glowing.
- Ground: ivory marble square paving with extremely fine muted gold geometric lattice and quiet lavender clouding, no reflections.
- Structure: smooth pearl stone surface with restrained repeating gold and lapis filigree trim, flat material.
- Detail: square midnight-blue enameled surface with a delicate original gold celestial compass of tiny stars and crescent shapes, nonrepeating, no text and no glow.

### mindglass-sanctum
- Wall: dark violet-black polished basalt ashlar with subtle amethyst veins, low contrast geometric rectangular blocks.
- Ground: dark purple-charcoal marble square paving with fine brass geometric lines, dry diffuse no reflections.
- Structure: charcoal stone with narrow vertical muted gold lines and shallow geometric ornamental channels, flat repeat material.
- Detail: square nonrepeating violet stone panel with a subtle original concentric diamond psychic sigil in pale lilac and brass, no bloom, no text.

### basalt-underworld
- Wall: massive blue-black basalt ashlar in rectangular staggered courses, fine chipped stone grain, dark quiet joints.
- Ground: large charcoal slate paving squares with subtle cool blue roughness, small edge chips, strict orthogonal top-down pattern.
- Structure: dark volcanic basalt mineral surface with elongated crystalline grain and thin subdued pewter veins, flat material only.
- Detail: square dark basalt relief panel with a restrained labyrinthine abstract geometric knot in worn pale green-gray mineral inlay, nonrepeat no text.

