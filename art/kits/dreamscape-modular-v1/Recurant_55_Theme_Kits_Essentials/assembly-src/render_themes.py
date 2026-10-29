"""Blender QA of actual texture-bound GLBs; only the requested one-bit theme receives binary threshold postprocessing."""
import bpy,sys,pathlib,json,math
from PIL import Image
import numpy as np
from mathutils import Vector
ROOT=pathlib.Path(__file__).resolve().parents[1]
args=sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [];native_only='--native-only' in args;slugs=[x for x in args if not x.startswith('--')] or [p.name for p in (ROOT/'themes').iterdir() if (p/'assembly/showcase.glb').exists()]
def aim(obj,target):obj.rotation_euler=(Vector(target)-obj.location).to_track_quat('-Z','Y').to_euler()
for slug in slugs:
 folder=ROOT/'themes'/slug
 if not (folder/'assembly/showcase.glb').exists():continue
 bpy.ops.wm.read_factory_settings(use_empty=True)
 bpy.ops.import_scene.gltf(filepath=str(folder/'assembly/showcase.glb'))
 scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.device='CPU';scene.cycles.samples=64;scene.cycles.use_denoising=False
 scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False;scene.world=bpy.data.worlds.new('Neutral studio');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.2,.25,.3,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=.45
 if slug=='onebit-spiritual-landscape':
  scene.world.node_tree.nodes['Background'].inputs[0].default_value=(1,1,1,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=1
  scene.render.use_freestyle=True
  settings=bpy.context.view_layer.freestyle_settings;settings.crease_angle=math.radians(45)
  lines=settings.linesets[0];lines.linestyle=bpy.data.linestyles.new('Binary geometry contours');lines.select_silhouette=True;lines.select_border=True;lines.select_crease=True;lines.linestyle.color=(0,0,0);lines.linestyle.thickness=1.1
 scene.view_settings.view_transform='Standard';scene.view_settings.look='Medium High Contrast';scene.view_settings.exposure=0;scene.view_settings.gamma=1
 for name,loc,power,size in [('Key',(-3,4,10),1700,8),('Fill',(10,-4,8),950,7)]:
  light=bpy.data.lights.new(name,'AREA');light.energy=power;light.shape='DISK';light.size=size;o=bpy.data.objects.new(name,light);scene.collection.objects.link(o);o.location=loc;aim(o,(3,-5,1))
 sun=bpy.data.lights.new('Broad sky','SUN');sun.energy=1.1;sun.angle=math.radians(30);o=bpy.data.objects.new('Broad sky',sun);scene.collection.objects.link(o);o.rotation_euler=(.5,-.5,-.4)
 camera=bpy.data.cameras.new('Camera');o=bpy.data.objects.new('Camera',camera);scene.collection.objects.link(o);scene.camera=o
 o.location=(12,8,11);aim(o,(3,-4.6,1.4));camera.type='ORTHO';camera.ortho_scale=15.8
 scene.render.resolution_x=800;scene.render.resolution_y=600;scene.render.resolution_percentage=100;scene.render.filepath=str(folder/'assembly/overview.png')
 if not native_only:bpy.ops.render.render(write_still=True)
 camera.type='PERSP';camera.sensor_fit='VERTICAL';camera.sensor_height=24;camera.lens=20.78460969;camera.sensor_width=36;camera.clip_start=.05;camera.clip_end=100
 o.location=(3,-.5,1.5);aim(o,(3,-8,1.5));scene.render.resolution_x=320;scene.render.resolution_y=240;scene.render.filepath=str(folder/'assembly/native_320x240.png');bpy.ops.render.render(write_still=True)
 if slug=='onebit-spiritual-landscape':
  for filename in (['native_320x240.png'] if native_only else ['overview.png','native_320x240.png']):
   path=folder/'assembly'/filename;im=Image.open(path).convert('L');a=np.asarray(im,dtype=float)/255.0
   bayer=(np.array([[0,8,2,10],[12,4,14,6],[3,11,1,9],[15,7,13,5]],dtype=float)+.5)/16
   thresholds=np.tile(bayer,((a.shape[0]+3)//4,(a.shape[1]+3)//4))[:a.shape[0],:a.shape[1]]
   Image.fromarray(np.where(a>=thresholds,255,0).astype('uint8'),'L').convert('RGB').save(path)
 report=folder/'assembly/asset-validation.json';d=json.loads(report.read_text());d.pop('visual_review',None);d['visual_review_status']='pending_after_this_render';d['preview_status']='actual GLB rendered by Blender Cycles CPU; visual inspection recorded separately';d['rendered_views']=['overview.png','native_320x240.png'];
 if slug=='onebit-spiritual-landscape':d['strict_binary_preview']={'applied':True,'method':'actual unlit GLB render + geometry-derived Freestyle black contours; ordered4x4 Bayer conversion to RGB black/white; user-requested one-bit presentation'}
 d['native_preview_camera']={'vertical_fov_degrees':60,'eye_height_m':1.5,'resolution_px':[320,240],'back_offset_m':0,'note':'diagnostic fixed camera; runtime controller/back-offset untested'};report.write_text(json.dumps(d,indent=2)+'\n')
 mf=folder/'theme-manifest.json'
 if mf.exists():
  m=json.loads(mf.read_text());m['qa']=d;mf.write_text(json.dumps(m,indent=2)+'\n')
 print('RENDERED_THEME',slug,flush=True)
