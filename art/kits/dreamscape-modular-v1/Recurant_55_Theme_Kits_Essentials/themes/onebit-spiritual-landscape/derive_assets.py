"""Deterministic delivery derivation from preserved built-in imagegen originals.

Only crop, resize, RGB conversion, and the explicitly requested 1-bit threshold;
no procedural painting, PBR inference, seam synthesis, or generative replacement.
"""
from pathlib import Path
from PIL import Image
import hashlib
import json
import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE
PREVIEW = HERE / 'concept'
ROLES = ['wall', 'ground', 'structure', 'detail']

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def stats(path):
    im = Image.open(path)
    rgb = np.asarray(im.convert('RGB'))
    colors, counts = np.unique(rgb.reshape(-1, 3), axis=0, return_counts=True)
    return {'path': str(path.relative_to(ROOT)), 'size': list(im.size),
            'mode': im.mode, 'alpha': 'A' in im.getbands(),
            'unique_rgb_color_count': len(colors),
            'palette': colors.tolist() if len(colors) < 10 else None,
            'black_fraction': float(counts[(colors == [0, 0, 0]).all(axis=1)].sum() / counts.sum()),
            'sha256': sha(path)}

def binary(im, threshold=128):
    return im.convert('L').point(lambda x: 255 if x >= threshold else 0).convert('RGB')

def repeated(im, n):
    sheet = Image.new('RGB', (im.width * n, im.height * n), 'white')
    for y in range(n):
        for x in range(n):
            sheet.paste(im, (x * im.width, y * im.height))
    return sheet

def seam_stats(im):
    a = np.asarray(im).astype(float)
    return {'left_right_mae': round(float(abs(a[:, 0] - a[:, -1]).mean()), 3),
            'top_bottom_mae': round(float(abs(a[0] - a[-1]).mean()), 3),
            'horizontal_internal_mae': round(float(abs(a[:, 1:] - a[:, :-1]).mean()), 3),
            'vertical_internal_mae': round(float(abs(a[1:] - a[:-1]).mean()), 3)}

def main():
    source = PREVIEW / 'source.png'
    src = Image.open(source).convert('RGB')
    assert src.width * 3 == src.height * 4
    native = binary(src.resize((320, 240), Image.Resampling.BOX), 160)
    native_path = PREVIEW / 'reference_native_320x240.png'
    preview_path = PREVIEW / 'preview_4x.png'
    native.save(native_path)
    native.resize((1280, 960), Image.Resampling.NEAREST).save(preview_path)
    atlas_path = HERE / 'art_sources/material-atlas-master.png'
    atlas = Image.open(atlas_path).convert('RGB')
    w, h = atlas.size
    crops = [(0, 0, w//2, h//2), (w//2, 0, w, h//2),
             (0, h//2, w//2, h), (w//2, h//2, w, h)]
    descriptions = [
        'Pale weathered stone with sparse pits and short vertical hatching',
        'Pale granular stone and sand, tiny dark granules and restrained hatch clusters',
        'Ancient monolith surface with bold vertical geological fractures',
        'Dense dark broken engraving and stipple field with small white voids']
    materials, ims, delivered = [], [], [native_path, preview_path]
    for role, crop, desc in zip(ROLES, crops, descriptions):
        final = binary(atlas.crop(crop).resize((256, 256), Image.Resampling.NEAREST))
        ims.append(final)
        p256 = HERE / f'materials/{role}_diffuse_256.png'
        p128 = HERE / f'materials/{role}_diffuse_128.png'
        final.save(p256)
        final.resize((128, 128), Image.Resampling.NEAREST).save(p128)
        delivered.extend([p256, p128])
        materials.append({'role': role,
            'file_256': str(p256.relative_to(HERE)), 'file_128': str(p128.relative_to(HERE)),
            'color_space': 'sRGB', 'channels': 'RGB', 'alpha': False,
            'size': [256, 256], 'alternate_size': [128, 128], 'sizes_available': [256, 128],
            'intended_use': desc, 'map_kind': 'binary baseColor / unlit color',
            'real_height_information': False, 'repeat_intent': role in ROLES[:2],
            'repeat_verified': False, 'seamless_certified': False,
            'wrap_recommendation': 'REPEAT' if role in ROLES[:2] else 'CLAMP',
            'uv_recommendation': '2 metres per repeat candidate' if role in ROLES[:2] else 'Local component UV0 0-1',
            'source_crop_px': crop, 'threshold_luminance_0_255': 128,
            'seam_metrics_rgb_0_255': seam_stats(final),
            'sha256': sha(p256), 'low_memory_sha256': sha(p128),
            'qa_note': 'Pending visual contact inspection; neither tile-edge identity nor motion-shimmer certification implied.'})
    contact = Image.new('RGB', (1024, 256), 'white')
    for i, im in enumerate(ims): contact.paste(im, (i*256, 0))
    contact.save(HERE / 'qa/material-contact.png')
    for n in [2, 3]:
        contact = Image.new('RGB', (256*n*2, 256*n), 'white')
        for i, im in enumerate(ims[:2]): contact.paste(repeated(im, n), (i*256*n, 0))
        contact.save(HERE / f'qa/wall-ground-repeat-{n}x{n}.png')
    qa = {'theme': 'onebit-spiritual-landscape', 'generation_mode': 'builtin image_gen',
          'user_requested_palette': ['#000000', '#FFFFFF'], 'alpha_policy': 'RGB opaque, no alpha',
          'originals_preserved': [stats(source), stats(atlas_path)],
          'delivery_processing_authority': 'User explicitly requested 1bit; threshold/palette conversion enforces this requirement.',
          'concept_derivation': 'No crop. BOX downsample to 320x240, luminance threshold at 160, RGB conversion; exact 4x NEAREST preview.',
          'material_derivation': 'Four exact atlas quadrant crops, NEAREST to 256x256, luminance threshold at 128, RGB; NEAREST 256->128.',
          'pixel_replication_verified': bool(np.array_equal(np.asarray(Image.open(preview_path)), np.repeat(np.repeat(np.asarray(native), 4, axis=0), 4, axis=1))),
          'delivered_files': [stats(p) for p in delivered],
          'palette_verified': all(stats(p)['palette'] == [[0, 0, 0], [255, 255, 255]] for p in delivered),
          'visual_qa': 'pending', 'runtime_integration_verified': False,
          'seamless_certified': False, 'camera_motion_shimmer_tested': False}
    (PREVIEW / 'qa-report.json').write_text(json.dumps(qa, indent=2)+'\n')
    (HERE / 'qa/validation-report.json').write_text(json.dumps(qa, indent=2)+'\n')
    manifest = {'schema': 'recurant.material-kit.v1', 'theme': 'onebit-spiritual-landscape',
        'generator': 'builtin image_gen', 'source_atlas': 'art_sources/material-atlas-master.png',
        'source_size': [w, h], 'prompt': 'prompts/atlas-prompt.txt',
        'derived_processing': qa['material_derivation'],
        'source_palette_warning': 'Original generator output may include grayscale/antialiasing despite binary prompt; preserved source is not certified 1-bit.',
        'delivery_palette': ['#000000', '#FFFFFF'],
        'palette_conversion_authority': qa['delivery_processing_authority'],
        'lighting_note': 'Binary art is not PBR. Use unlit baseColor and nearest sampling, or a final luminance threshold. Ordinary lit shading, fog, postprocessing, antialiasing and linear filtering can introduce gray. Textures alone cannot guarantee binary rendered output.',
        'qa_contacts': ['qa/material-contact.png', 'qa/wall-ground-repeat-2x2.png', 'qa/wall-ground-repeat-3x3.png'],
        'qa_status': 'Pending visual review; pixels validated separately in qa/validation-report.json.',
        'materials': materials, 'runtime_integration_verified': False,
        'pbr_maps_supplied': [], 'common_spec': 'common/spec/asset_contract.json'}
    (HERE/'material-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps({'palette_verified': qa['palette_verified'], 'files_verified': len(delivered),
                      'source_color_counts': [x['unique_rgb_color_count'] for x in qa['originals_preserved']],
                      'material_seams': {x['role']: x['seam_metrics_rgb_0_255'] for x in materials}}, indent=2))

if __name__ == '__main__': main()
