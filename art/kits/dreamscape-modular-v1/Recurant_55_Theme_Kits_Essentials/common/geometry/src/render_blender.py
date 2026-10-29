"""Optional Blender 4.3+ catalog and render build. Run blender -b -t 4 --python thisfile.
No geometry generation dependencies besides Blender; imports actual delivered GLBs.
"""
import bpy, json, pathlib, math, sys
from mathutils import Vector
ROOT=pathlib.Path(__file__).resolve().parents[1]
MAN=json.loads((ROOT/'geometry-manifest.json').read_text())
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene
scene.render.engine='BLENDER_WORKBENCH'
scene.display.shading.light='STUDIO'
scene.display.shading.studio_light='paint.sl'
scene.display.shading.color_type='MATERIAL'
scene.display.shading.show_shadows=True
scene.display.shading.show_cavity=True
scene.display.shading.cavity_type='BOTH'
scene.display.shading.curvature_ridge_factor=1.5
scene.display.shading.curvature_valley_factor=1.3
scene.display.shading.show_object_outline=True
scene.display.shading.object_outline_color=(.09,.11,.14)
scene.display.shading.background_type='WORLD'
scene.world=bpy.data.worlds.new('Studio charcoal');scene.world.color=(.055,.068,.08)
scene.render.image_settings.file_format='PNG'
scene.render.film_transparent=False
scene.render.resolution_percentage=100
scene.view_settings.view_transform='Standard'
scene.view_settings.look='Medium High Contrast'
scene.render.resolution_x=320;scene.render.resolution_y=320
cam_data=bpy.data.cameras.new('Catalog_camera');cam=bpy.data.objects.new('Catalog_camera',cam_data);scene.collection.objects.link(cam);scene.camera=cam;cam_data.type='ORTHO'
objects=[]
def point_camera(center,extent,direction=(1.15,1.8,1.0)):
    # Blender view: front is negativeY; inspect originalglTFfront(-Z) by Blender+Y.
    target=Vector(center);direction=Vector(direction).normalized();cam.location=target+direction*extent*3
    cam.rotation_euler=(target-cam.location).to_track_quat('-Z','Y').to_euler();cam_data.ortho_scale=extent*1.55;cam_data.lens=50

def imported(path):
    bpy.ops.object.select_all(action='DESELECT');before=set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path),import_shading='NORMALS')
    return [o for o in bpy.data.objects if o not in before and o.type=='MESH']

pre=ROOT/'previews'/'thumbnails';pre.mkdir(parents=True,exist_ok=True)
for i,a in enumerate(MAN['assets']):
    obs=imported(ROOT/a['files']['glb'])
    for ob in obs:
        ob.name=a['id'];ob['recurant_asset_id']=a['id'];ob['pivot_semantic']=a['pivot']['semantic'];ob['metres_per_game_unit']=2;ob['canonical_interchange']=a['files']['glb'];ob.asset_mark()
        for old in list(ob.users_collection):old.objects.unlink(ob)
        col=bpy.data.collections.get(a['family'])
        if not col:col=bpy.data.collections.new(a['family']);scene.collection.children.link(col)
        col.objects.link(ob)
    mn=a['bounds_m']['min'];mx=a['bounds_m']['max'];center=[(mn[0]+mx[0])/2,-(mn[2]+mx[2])/2,(mn[1]+mx[1])/2]
    size=[mx[j]-mn[j] for j in range(3)];extent=max(size)
    direction=(1.15,-1.8,1.0) if a['id'].startswith(('stairs_','ramp_')) else (-1.15,1.8,1.0) if a['id']=='roof_pitch_left_2m' else (1.15,1.8,1.0)
    point_camera(center,extent,direction)
    scene.render.filepath=str(pre/f"{a['id']}.png")
    bpy.ops.render.render(write_still=True)
    for ob in obs:ob.hide_render=True
    objects.append((a,obs))
    if '--pilot' in sys.argv:break
if '--pilot' not in sys.argv:
    # Keep clean local mesh data. The source catalog object placements are intentionally a display grid.
    families=list(MAN['counts']['families'])
    for row,fam in enumerate(families):
        assets=[(a,obs) for a,obs in objects if a['family']==fam]
        for col,(a,obs) in enumerate(assets):
            for ob in obs:ob.location=(col*5,row*8,0);ob.hide_render=False
    cam.location=(30,-25,60);cam.rotation_euler=(Vector((30,30,0))-cam.location).to_track_quat('-Z','Y').to_euler();cam.data.ortho_scale=100
    scene['source_note']='Catalog object positions are for inspection; GLB individual assets carry identity transforms and canonical local pivots.'
    bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'Recurant_modular_geometry_library.blend'))
    # Family combinations imported separately, render only their geometry.
    for ob in bpy.data.objects:
        if ob.type=='MESH':ob.hide_render=True
    for d in MAN['assemblies']:
        obs=imported(ROOT/d['file']);bounds=[]
        for ob in obs:bounds += [ob.matrix_world@Vector(c) for c in ob.bound_box]
        mn=Vector([min(p[j] for p in bounds) for j in range(3)]);mx=Vector([max(p[j] for p in bounds) for j in range(3)])
        point_camera((mn+mx)/2,max(mx-mn));scene.render.resolution_x=1200;scene.render.resolution_y=1000;cam.data.ortho_scale=max(mx-mn)*1.4
        scene.render.filepath=str(ROOT/'previews'/f"{d['id']}.png");bpy.ops.render.render(write_still=True)
        for ob in obs:ob.hide_render=True
print('RECURANT_RENDER_COMPLETE')
