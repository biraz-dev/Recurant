#!/usr/bin/env python3
"""Build original, texture-bound demonstration assemblies from the shared GLB library.
This is an offline asset authoring helper, NOT a Recurant runtime converter.
Python 3, NumPy, Pillow and jsonschema. Safe to rerun; original material files unchanged.
"""
import sys,json,pathlib,struct,math,hashlib,collections,re,copy
import numpy as np
from PIL import Image
from theme_recipes import R
ROOT=pathlib.Path(__file__).resolve().parents[1]
GEO=ROOT/'common/geometry'
ROLES=['wall','ground','structure','detail']
CACHE={}
def read_glb(path):
 data=path.read_bytes();magic,version,total=struct.unpack_from('<III',data);assert magic==0x46546c67 and version==2 and total==len(data)
 off=12;doc=None;binary=None
 while off<len(data):
  length,kind=struct.unpack_from('<II',data,off);payload=data[off+8:off+8+length];off+=8+length
  if kind==0x4e4f534a:doc=json.loads(payload)
  elif kind==0x004e4942:binary=payload
 return doc,binary

def accessor(d,b,idx):
 a=d['accessors'][idx];v=d['bufferViews'][a['bufferView']];types={5126:'<f4',5123:'<u2',5125:'<u4',5121:'u1'};width={'VEC2':2,'VEC3':3,'SCALAR':1}[a['type']]
 assert 'byteStride' not in v
 return np.frombuffer(b,dtype=types[a['componentType']],count=a['count']*width,offset=v.get('byteOffset',0)+a.get('byteOffset',0)).copy().reshape(-1,width)

def load_module(mid):
 if mid not in CACHE:
  d,b=read_glb(GEO/'meshes'/f'{mid}.glb');parts=[]
  for n in d['nodes']:
   assert not any(k in n for k in ['matrix','translation','rotation','scale']), (mid,'unbaked node')
   if 'mesh' not in n:continue
   for p in d['meshes'][n['mesh']]['primitives']:
    parts.append(dict(role=d['materials'][p['material']]['name'],p=accessor(d,b,p['attributes']['POSITION']),n=accessor(d,b,p['attributes']['NORMAL']),u=accessor(d,b,p['attributes']['TEXCOORD_0']),i=accessor(d,b,p['indices']).reshape(-1,3)))
  CACHE[mid]=parts
 return CACHE[mid]

def write_glb(parts,texturepaths,out,name,unlit=False):
 binary=bytearray();views=[];acc=[];prims=[];materials=[];images=[];textures=[]
 def blob(raw,target=None):
  o=len(binary);binary.extend(raw);binary.extend(b'\0'*((-len(binary))%4));v=dict(buffer=0,byteOffset=o,byteLength=len(raw))
  if target:v['target']=target
  views.append(v);return len(views)-1
 def attr(a,kind,comp,target):
  v=blob(a.tobytes(),target);x=dict(bufferView=v,componentType=comp,count=len(a),type=kind)
  if kind=='VEC3':x.update(min=a.min(0).tolist(),max=a.max(0).tolist())
  acc.append(x);return len(acc)-1
 for ri,role in enumerate(ROLES):
  img=blob(texturepaths[role].read_bytes());images.append(dict(name=role,mimeType='image/png',bufferView=img));textures.append(dict(sampler=0 if role in ['wall','ground'] else 1,source=len(images)-1))
  materials.append(dict(name=role,pbrMetallicRoughness=dict(baseColorFactor=[1,1,1,1],baseColorTexture=dict(index=len(textures)-1),metallicFactor=0,roughnessFactor=.85),alphaMode='OPAQUE',doubleSided=False,extras=dict(semantic_role=role,seamless_certified=False)))
 for ri,role in enumerate(ROLES):
  ds=parts.get(role,[])
  if not ds:continue
  ps=[];ns=[];us=[];ix=[];n=0
  for p in ds:ps.append(p['p']);ns.append(p['n']);us.append(p['u']);ix.append(p['i']+n);n+=len(p['p'])
  assert n<=65535,(name,role,n)
  p=np.concatenate(ps).astype('<f4');normal=np.concatenate(ns).astype('<f4');uv=np.concatenate(us).astype('<f4');idx=np.concatenate(ix).astype('<u2').reshape(-1)
  ai=attr(p,'VEC3',5126,34962);an=attr(normal,'VEC3',5126,34962);au=attr(uv,'VEC2',5126,34962);ii=attr(idx,'SCALAR',5123,34963)
  prims.append(dict(attributes=dict(POSITION=ai,NORMAL=an,TEXCOORD_0=au),indices=ii,material=ri,mode=4))
 d=dict(asset=dict(version='2.0',generator='Original modular theme assembly authoring'),scene=0,scenes=[dict(nodes=[0])],nodes=[dict(name=name,mesh=0)],meshes=[dict(name=name,primitives=prims)],materials=materials,textures=textures,images=images,samplers=[dict(magFilter=9728,minFilter=9728,wrapS=10497,wrapT=10497),dict(magFilter=9728,minFilter=9728,wrapS=33071,wrapT=33071)],buffers=[dict(byteLength=len(binary))],bufferViews=views,accessors=acc)
 if unlit:
  d['extensionsUsed']=['KHR_materials_unlit']
  for material in materials:material['extensions']={'KHR_materials_unlit':{}}
 js=json.dumps(d,separators=(',',':')).encode();js+=b' '*((-len(js))%4)
 out.parent.mkdir(parents=True,exist_ok=True);out.write_bytes(struct.pack('<III',0x46546c67,2,28+len(js)+len(binary))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(binary),0x004e4942)+binary)
 return dict(vertices=sum(x['count'] for x in acc if x['type']=='VEC3')//2,triangles=sum(x['count'] for x in acc if x['type']=='SCALAR')//3,primitives=len(prims),embedded_pngs=len(images))

def materials(slug):
 folder=ROOT/'themes'/slug
 paths=list(folder.glob('material*manifest.json'))+list(folder.glob('material*manifest.json'))
 if not paths:return None
 raw=json.loads(paths[0].read_text());result={}
 for m in raw['materials']:
  role=m.get('role',m.get('slot'));f=m.get('preferred_files',{}).get('256') or m.get('base_color_texture',m.get('file_256',m.get('files',{}).get('256')))
  if role not in ROLES or not f:continue
  p=folder/f
  if not p.exists():return None
  assert Image.open(p).size==(256,256)
  low=m.get('preferred_files',{}).get('128') or m.get('low_res_texture',m.get('file_128',m.get('files',{}).get('128')));result[role]=dict(path=p,relative_path=f,low_relative_path=low,description=m.get('description',m.get('intended_use',m.get('intended_surface',m.get('name','')))),raw=m)
 if len(result)!=4:return None
 return result,raw,paths[0].name

def role_remap(mid,mats,slug):
 # Keep module semantics explicit; natural material roles are chosen by actual descriptions.
 mapping={r:r for r in ROLES}
 kind=None
 if any(k in mid for k in ['tree','trunk','root_','fallen_log']):kind='bark'
 elif any(k in mid for k in ['fern','broadleaf','grass','wheat','cattail','sunflower','cactus']):kind='leaf'
 elif any(k in mid for k in ['boulder','rock_','cave_','crag','cliff','basalt','standing_stone','stalag']):kind='stone'
 elif 'water_surface' in mid:kind='water'
 if slug in ['tissue','organic-living']:kind=None
 keys={'bark':['bark','timber','wood','root'],'leaf':['leaf','leaves','fern','grass','foliage','moss'],'stone':['stone','rock','basalt','mineral','slate','limestone','masonry'],'water':['water','ripples','ripple']}
 if kind:
  score={r:sum(k in v['description'].lower() for k in keys[kind]) for r,v in mats.items() if r!='ground'}
  best=max(score,key=score.get)
  if score[best]>0:
   for r in ['wall','structure','detail']:
    if kind=='leaf' and r!='detail':continue
    mapping[r]=best
 return {a:b for a,b in mapping.items() if a!=b}

def rebind_uv(part,src_role,dst_role):
 if src_role==dst_role:return part['u'].copy()
 if dst_role in ['wall','ground']:
  # Metric planar projection by each flat face's dominant normal axis.
  result=np.zeros_like(part['u'])
  dims=[[2,1],[0,2],[0,1]]
  for axis in range(3):
   selected=np.argmax(np.abs(part['n']),axis=1)==axis
   result[selected]=part['p'][selected][:,dims[axis]]/2
  return result
 # Normalize each disconnected face island, preserving panel-clamp semantics after rebinding.
 parent=list(range(len(part['p'])))
 def find(a):
  while parent[a]!=a:parent[a]=parent[parent[a]];a=parent[a]
  return a
 for tri in part['i']:
  a=find(int(tri[0]))
  for b in tri[1:]:parent[find(int(b))]=a
 groups=collections.defaultdict(list)
 for v in range(len(parent)):groups[find(v)].append(v)
 result=part['u'].copy()
 for vertices in groups.values():
  uv=result[vertices];lo=uv.min(0);size=uv.max(0)-lo;result[vertices]=(uv-lo)/np.where(size>1e-9,size,1)
 return result

def build(slug):
 recipe=R[slug];m=materials(slug)
 if not m:return None
 mats,raw,rawname=m;folder=ROOT/'themes'/slug;gm=json.loads((GEO/'geometry-manifest.json').read_text());catalog={a['id']:a for a in gm['assets']};instances=[];parts=collections.defaultdict(list);worldboxes=[]
 def instance(mid,pos,rot=0,scale=1,role='scenery',cell=None,remap=None):
  assert mid in catalog,(slug,mid)
  k=len(instances);inst=dict(instance_id=f'i{k:03d}',module_id=mid,position_m=list(pos),rotation_degrees_xyz=[0,rot,0],scale_xyz=[scale]*3,role=role,cell=cell)
  remap=role_remap(mid,mats,slug) if remap is None else remap
  if remap:inst['material_role_overrides']=remap
  instances.append(inst);a=math.radians(rot);rotm=np.array([[math.cos(a),0,math.sin(a)],[0,1,0],[-math.sin(a),0,math.cos(a)]])
  for p in load_module(mid):
   q={x:v.copy() if isinstance(v,np.ndarray) else v for x,v in p.items()};q['p']=p['p']@rotm.T*scale+np.array(pos);q['n']=p['n']@rotm.T;q['role']=remap.get(p['role'],p['role']);q['u']=rebind_uv(p,p['role'],q['role']);parts[q['role']].append(q)
  for b in catalog[mid]['collision']['boxes']:
   center=np.array(b['center'])@rotm.T*scale+np.array(pos);size=np.abs(rotm)@np.array(b['size'])*scale
   worldboxes.append(dict(instance_id=inst['instance_id'],module_id=mid,center=center.round(6).tolist(),size=size.round(6).tolist(),role=role))
  return inst
 # Diagnostic cutaway galleries intentionally omit roof and front wall. Full modules remain available.
 for z in range(5):
  for x in range(3):
   instance(recipe['floor'],[2*x+1,0,2*z+1],role='floor',cell=[x,z],remap={})
   if recipe['floor']=='floor_grate_2m':instance('floor_plain_2m',[2*x+1,-.2,2*z+1],role='floor',cell=[x,z],remap={})
 # Decorative opaque water pads use a separate non-colliding surface, not a walkable fluid.
 for wr,mat in mats.items():
  if any(w in mat['description'].lower() for w in ['water']):
   for cell in [[0,1],[2,2]]:instance('water_surface_2m',[2*cell[0]+1,.01,2*cell[1]+1],role='nonwalkable_visual_water',cell=cell,remap={'ground':wr})
   break
 # Rear silhouette: three independent structural bays; open front and sides show mesh topology.
 for x in range(3):
  rear_points=np.concatenate([p['p'] for p in load_module(recipe['wall'])]);rear_z=10.1-rear_points[:,2].min()
  instance(recipe['wall'],[2*x+1,0,float(rear_z)],0,role='rear_boundary',cell=[x,4])
 portal=recipe['portal']
 # Toy arch is a small prop, not a valid 2.4m gameplay doorway: use real timber portal and display toy arch at side.
 if portal=='toy_block_arch':portal='timber_post_beam_gate'
 instance(portal,[3,0,8],role='open_portal',cell=[1,3])
 # Eight distinct furnishings/landmarks, uniformly scaled only to fit blocked-cell exhibit pads.
 for j,mid in enumerate(recipe['props']):
  x=0 if j%2==0 else 2;z=j//2;rot=[0,15,-15,90][j%4];a=math.radians(rot);rotm=np.array([[math.cos(a),0,math.sin(a)],[0,1,0],[-math.sin(a),0,math.cos(a)]])
  points=np.concatenate([p['p'] for p in load_module(mid)])@rotm.T;mn=points.min(0);mx=points.max(0);size=mx-mn;s=min(1,1.8/max(size[0],1e-3),1.8/max(size[2],1e-3))
  center=(mn+mx)/2;position=[2*x+1-center[0]*s,-mn[1]*s,2*z+1-center[2]*s]
  instance(mid,position,rot,s,role='blocked_cell_scenery',cell=[x,z])
 nav=dict(cells=[dict(cell=[x,z],walkable=(x==1),surface_y_m=0,kind='Floor' if x==1 else 'SceneryBlocked') for z in range(5) for x in range(3)],edges=[dict(a=[1,z],b=[1,z+1],bidirectional=True) for z in range(4)],start_cell=[1,0],goal_cell=[1,4],missing_edges='forbidden_between_declared_walkable_neighbours',diagonal=False)
 layout=dict(schema_version='1.0.0',theme_id=recipe['public_id'],title=recipe['title'],manifest_base='package_root',units='meters',to_game_scale=.5,coordinate_system='right-handed +Y up +X east -Z north',instances=instances,navigation=nav,usage='diagnostic cutaway gallery, not a production level; choose instance route OR assembled GLB',material_role_overrides='Applied per module primitive before packing. Recompute UVs on semantic change: target wall/ground uses metric dominant-normal projection /2; target structure/detail normalizes each disconnected face island to0–1. Module importers must apply both role and UV rebinding.')
 (folder/'assembly').mkdir(exist_ok=True);(folder/'assembly/layout.json').write_text(json.dumps(layout,indent=2)+'\n')
 (folder/'assembly/collision-navigation.json').write_text(json.dumps(dict(authority='authoring guides only; zrpg_core::Map remains runtime authority',boxes=worldboxes,navigation=nav,travel_lane=dict(width_m=1.2,headroom_m=2.4,central_x_m=3,z_range_m=[.2,9.8]),fluid_collision='none; solid decks or beds separate'),indent=2)+'\n')
 metrics=write_glb(parts,{r:mats[r]['path'] for r in ROLES},folder/'assembly/showcase.glb',recipe['title'],unlit=(slug=='onebit-spiritual-landscape'))
 # Lane AABB checks explicitly exclude floors; compound doorway boxes used, never full arch bounds.
 collisions=[]
 for b in worldboxes:
  if b['role']=='floor':continue
  lo=np.array(b['center'])-np.array(b['size'])/2;hi=np.array(b['center'])+np.array(b['size'])/2
  if hi[0]>2.40001 and lo[0]<3.59999 and hi[1]>.02 and lo[1]<2.39999 and hi[2]>.2 and lo[2]<9.8:collisions.append(b['instance_id'])
 qa=dict(asset_assembly_generated=True,metrics=metrics,lane_guide_clear=not collisions,lane_guide_conflicts=collisions,navigation_connected=True,runtime_integration_verified=False,preview_status='awaiting rendered inspection',texture_status='diffuse candidates; no PBR data maps or seamless certification',duplicate_scene_routes='do not import both assembly GLB and layout instances')
 if collisions:raise RuntimeError((slug,'lane collisions',collisions))
 public=recipe['public_id'];base=f'themes/{public}'
 bindings={r:dict(basecolor=f"{base}/{mats[r]['relative_path']}",low_memory_basecolor=f"{base}/{mats[r]['low_relative_path']}",color_space='sRGB',uv_mode='tile_repeat_2m' if r in ['wall','ground'] else 'panel_clamp_0_1',alpha_mode='OPAQUE',metallic=0,roughness=.85,seamless_certified=False,description=mats[r]['description']) for r in ROLES}
 uniq=list(dict.fromkeys(i['module_id'] for i in instances))
 manifest=dict(schema_version='1.0.0',theme_id=public,title=recipe['title'],manifest_base='package_root',geometry_manifest='common/geometry/geometry-manifest.json',material_bindings=bindings,asset_recipe=[dict(module_id=mid,role='structure' if mid in [recipe['wall'],portal] else 'walkable_floor' if mid==recipe['floor'] else 'distinctive_prop',notes=catalog[mid]['description']) for mid in uniq],assembly=dict(route='assembled_glb',layout=f'{base}/assembly/layout.json',glb=f'{base}/assembly/showcase.glb',preview=f'{base}/assembly/overview.png',notes='GLB is texture-bound cutaway showcase. All instance transforms baked into vertices. Import GLB OR layout, not both.'),runtime_integration_verified=False,originality_notes='Original generated surface art and original parametric geometry; generic public names, no source-world logos, characters or copied layouts.',qa=qa,dependencies=['common/geometry','common/spec'],material_source_manifest=f'{base}/{rawname}')
 if slug=='onebit-spiritual-landscape':
  manifest['rendering_policy']={'material_model':'KHR_materials_unlit','optional_extension':'KHR_materials_unlit','samplers':'nearest','strict_binary_runtime':'Requires unlit/point-sampled materials and final framebuffer threshold; antialiasing, tone mapping, lighting or unsupported-extension fallback can introduce gray.','preview_postprocess':'Actual GLB render with geometry-derived contour lines and ordered4x4 Bayer binary conversion; explicitly authorized by 1-bit request.','runtime_implemented':False}
 (folder/'theme-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
 (folder/'assembly/asset-validation.json').write_text(json.dumps(qa,indent=2)+'\n')
 palette=raw.get('palette','; '.join(mats[r]['description'] for r in ROLES))
 readme=f'''# {recipe['title']} — Claude handoff

**Status:** real modular geometry + four generated diffuse material candidates + texture-bound diagnostic GLB. Not runtime-integrated or shipping-certified.

## Start here
- Read `common/spec/asset_contract.json`, `common/spec/COLLISION_AND_IMPORT.md` and this theme's `theme-manifest.json` from the extracted package root.
- Open `assembly/showcase.glb` to inspect actual geometry and textures. It contains {metrics['triangles']:,} triangles / {metrics['primitives']} material primitives, with embedded PNGs and identity root transform.
- Rebuild from `assembly/layout.json` and shared `common/geometry/meshes/` **or** import the assembled GLB. Never both.
- Paths in normalized theme/assembly manifests are relative to **package root**; paths in the raw material manifest are relative to this theme folder. Shared geometry manifest files are relative to `common/geometry/`.

## Design and palette
{palette}

This is a **{recipe['family']}** kit. Distinctive supplied forms: {', '.join(mid.replace('_',' ') for mid in recipe['props'])}. The collection uses reusable silhouette families but not a box-only recolour: the arch, supports, furniture, rock, plant or machine elements are actual geometry. The 6 × 10 m cutaway is an inspectable assembly, not a reproduction of the larger concept scene. It omits roof/front enclosure to expose the assets.

## Modularity and safe placement
- 2 m authoring grid; +Y up; +X east; −Z north; scale to game by **0.5 exactly once**. Most structural bays are 2 m; read each module's real bounds/connectors/pivot. Do not normalize every mesh or auto-centre it.
- Centre lane is X=3 m, width 1.2 m, headroom 2.4 m. Five cardinal cells connect start [1,0] to goal [1,4]. Side cells are intentionally blocked exhibits, not traversable furnished rooms.
- `assembly/collision-navigation.json` supplies transformed compound boxes and exact allowed edges. They are authoring guides only, not a collision engine. Doorway voids must remain open. No swimming, climbing, animated machinery, hazards, furniture interactions or door-state logic is supplied.
- Small exhibition props use explicit uniform instance scales to fit side pads; modules themselves retain authored metre dimensions. Remaps in `material_role_overrides` are intentional and must be applied before batching an instance recipe. UVs are rebound with the target role: metric dominant-normal projection /2 for wall/ground; each disconnected face island normalized 0–1 for structure/detail. The assembly source implements this rule, preventing clamp streaks.

## Materials and UV status
| Role | Actual surface | UV / wrap |
|---|---|---|
'''+''.join(f"| {r} | {mats[r]['description']} | {'metric 2 m repeat candidate' if r in ['wall','ground'] else 'component 0–1 panel clamp'} |\n" for r in ROLES)+f'''
All four runtime candidates are opaque RGB PNG, sRGB, 256 × 256; 128 × 128 alternatives included. These are painted diffuse/base-colour surfaces, not physically measured albedo. **No normal, AO, height, roughness, metallic or emission maps supplied.** Defaults are metallic 0 / roughness 0.85, including stylized metal themes.

Generated wall/ground opposing edges are **not certified seamless**. Consult `{rawname}`, material notes and repeat QA. This assembly intentionally exposes repetition; use seams, panel breaks or further authored material revisions before long uninterrupted surfaces. Structure/detail panels clamp. Some UVs are face-projected rather than hand-authored trim islands; disconnected flat-shaded components are intentional low-poly construction. Do not claim lightmap-ready or watertight geometry.

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
'''
 if slug=='onebit-spiritual-landscape':
  readme+='\n## Strict one-bit presentation exception\n\nAll four supplied runtime materials contain only RGB #000000 and #FFFFFF. This showcase declares **KHR_materials_unlit** (optional glTF extension) and nearest filtering. A viewer that ignores that extension can fall back to ordinary lighting and produce gray; that fallback is **not** a strict one-bit result. Actual geometry renders add a geometry-derived black contour pass and ordered4x4 Bayer conversion to preserve sparse texture density while eliminating antialiasing gray. This two-color presentation was requested for the one-bit style, not used to disguise missing geometry.\n\nFor Claude: add an explicit unlit/nearest material path, then apply a geometry-derived black contour pass and final framebuffer two-color dither (or a deliberately tuned binary threshold) after lighting/tone mapping/AA. Disable interpolated color/mipmap blending when literal two-color output is required. Do not claim that white textures plus ordinary gray lighting are one-bit. The current Recurant shader/importer has not been modified to implement this policy. Monoliths, stone steps/pavers, arch, altar/pedestal and ring focus are real modular geometry.\n'
 (folder/'CLAUDE_README.md').write_text(readme)
 return dict(source_id=slug,theme_id=public,title=recipe['title'],family=recipe['family'],module_count=len(uniq),metrics=metrics,folder=str(folder))

def main():
 chosen=sys.argv[1:] or list(R);done=[];pending=[]
 for slug in chosen:
  try:r=build(slug)
  except Exception as e:print('ERROR',slug,str(e),flush=True);raise
  if r:done.append(r);print('ASSEMBLED',slug,r['metrics'],flush=True)
  else:pending.append(slug)
 (ROOT/'assembly-src/build-status.json').write_text(json.dumps(dict(assembled=done,pending_materials=pending),indent=2)+'\n')
 print('SUMMARY',len(done),'assembled',len(pending),'pending')
if __name__=='__main__':main()
