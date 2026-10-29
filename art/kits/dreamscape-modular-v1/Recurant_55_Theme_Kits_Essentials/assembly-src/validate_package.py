#!/usr/bin/env python3
"""Asset-only validation, not a Recurant game test. Requires numpy/Pillow/jsonschema."""
import pathlib,json,sys,struct,hashlib,collections
sys.dont_write_bytecode=True
import numpy as np
from PIL import Image
from jsonschema import Draft202012Validator
from assemble_themes import read_glb,accessor,ROLES
ROOT=pathlib.Path(sys.argv[1]) if len(sys.argv)>1 else pathlib.Path(__file__).resolve().parents[1]
errors=[];checks=collections.Counter();records=[]
def check(ok,label):
 checks['assertions']+=1
 if not ok:errors.append(label)
def glb(path):
 d,b=read_glb(path);check(d['asset']['version']=='2.0',str(path)+' gltf version');check(not d.get('extensionsRequired'),str(path)+' requires extension')
 for n in d['nodes']:check(not any(k in n for k in ['matrix','translation','rotation','scale']),str(path)+' node transforms')
 for mesh in d['meshes']:
  for p in mesh['primitives']:
   check(p.get('mode',4)==4,str(path)+' triangles');a=p['attributes'];check(all(k in a for k in ['POSITION','NORMAL','TEXCOORD_0']),str(path)+' attributes')
   pts=accessor(d,b,a['POSITION']);ns=accessor(d,b,a['NORMAL']);uv=accessor(d,b,a['TEXCOORD_0']);idx=accessor(d,b,p['indices']).flatten()
   check(np.isfinite(pts).all() and np.isfinite(ns).all() and np.isfinite(uv).all(),str(path)+' nonfinite');check(len(pts)<=65535 and idx.max()<len(pts),str(path)+' indices');check(d['accessors'][p['indices']]['componentType']==5123,str(path)+' uint16')
   check(np.max(np.abs(np.linalg.norm(ns,axis=1)-1))<.001,str(path)+' unit normals')
   tri=pts[idx.reshape(-1,3)];cross=np.cross(tri[:,1]-tri[:,0],tri[:,2]-tri[:,0]);check(np.min(np.linalg.norm(cross,axis=1))>1e-10,str(path)+' degenerate triangle');check(np.min(np.sum(cross*ns[idx.reshape(-1,3)[:,0]],axis=1))>0,str(path)+' winding')
 checks['glb_files']+=1
 return d,b
schema=json.loads((ROOT/'common/spec/theme_manifest.schema.json').read_text());catalog=json.loads((ROOT/'common/geometry/geometry-manifest.json').read_text());ids={a['id'] for a in catalog['assets']}
for p in sorted((ROOT/'common/geometry/meshes').glob('*.glb')):glb(p)
for folder in sorted((ROOT/'themes').iterdir()):
 mf=folder/'theme-manifest.json'
 if not mf.exists():continue
 m=json.loads(mf.read_text())
 for err in Draft202012Validator(schema).iter_errors(m):errors.append(str(mf)+': '+err.message)
 for r in ROLES:
  bind=m['material_bindings'][r];p=ROOT/bind['basecolor'];check(p.exists(),str(p)+' missing')
  if p.exists():im=Image.open(p);im.verify();check(Image.open(p).size==(256,256),str(p)+' dimensions');checks['runtime_pngs']+=1
  low=ROOT/bind['low_memory_basecolor'];check(low.exists(),str(low)+' missing');check(Image.open(low).size==(128,128),str(low)+' low dimensions');checks['low_memory_pngs']+=1
 for a in m['asset_recipe']:check(a['module_id'] in ids,m['theme_id']+' unknown module '+a['module_id'])
 for key in ['layout','glb','preview']:check((ROOT/m['assembly'][key]).is_file(),m['theme_id']+' missing '+key)
 p=ROOT/m['assembly']['glb'];d,b=glb(p);check(set(x['name'] for x in d['materials'])==set(ROLES),m['theme_id']+' roles');check(len(d['images'])==4,m['theme_id']+' embedded textures')
 for im in d['images']:
  v=d['bufferViews'][im['bufferView']];raw=b[v.get('byteOffset',0):v.get('byteOffset',0)+v['byteLength']];ext=ROOT/m['material_bindings'][im['name']]['basecolor'];check(hashlib.sha256(raw).digest()==hashlib.sha256(ext.read_bytes()).digest(),m['theme_id']+' embedded differs '+im['name'])
 if m['theme_id']=='onebit-spiritual-landscape':
  check('KHR_materials_unlit' in d.get('extensionsUsed',[]),'onebit unlit extension absent')
  for mat in d['materials']:check('KHR_materials_unlit' in mat.get('extensions',{}),'onebit lit material')
  for path in list((folder/'materials').glob('*.png'))+[p for p in [folder/'assembly/overview.png',folder/'assembly/native_320x240.png'] if p.exists()]:
   im=Image.open(path);pixels=np.asarray(im.convert('RGB'));check(im.mode=='RGB' and np.all((pixels==0)|(pixels==255)) and np.all(pixels[:,:,0]==pixels[:,:,1]) and np.all(pixels[:,:,1]==pixels[:,:,2]),str(path)+' not strict binary RGB')
   checks['onebit_binary_images']+=1
 l=json.loads((ROOT/m['assembly']['layout']).read_text());nav=l['navigation'];allowed={tuple(x['cell']) for x in nav['cells'] if x['walkable']};edges=collections.defaultdict(set)
 for e in nav['edges']:
  a,b=tuple(e['a']),tuple(e['b']);check(e['bidirectional'] and a in allowed and b in allowed and sum(abs(x-y) for x,y in zip(a,b))==1,m['theme_id']+' edge invalid');edges[a].add(b);edges[b].add(a)
 seen=set();todo=[tuple(nav['start_cell'])]
 while todo:
  a=todo.pop()
  if a in seen:continue
  seen.add(a);todo.extend(edges[a]-seen)
 check(seen==allowed and tuple(nav['goal_cell']) in seen,m['theme_id']+' disconnected');check(m['qa']['lane_guide_clear'],m['theme_id']+' lane guide');check(m['runtime_integration_verified'] is False,m['theme_id']+' improper runtime claim');checks['themes']+=1
for p in sorted(ROOT.rglob('*')):
 if p.is_file() and p.name not in ['FILE_INVENTORY.json','SHA256SUMS.txt','VALIDATION_REPORT.json']:
  rel=str(p.relative_to(ROOT));check(not rel.startswith('/') and '..' not in pathlib.PurePosixPath(rel).parts,'unsafe path '+rel);records.append(dict(path=rel,bytes=p.stat().st_size,sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
report=dict(status='PASS' if not errors else 'FAIL',checks=dict(checks),errors=errors,asset_only=True,runtime_tests='not run',seamless_certification=False,visual_review='See QA_VISUAL_REVIEW.md; image existence is not a visual pass.')
(ROOT/'VALIDATION_REPORT.json').write_text(json.dumps(report,indent=2)+'\n');(ROOT/'FILE_INVENTORY.json').write_text(json.dumps(dict(files=records,file_count=len(records),total_bytes=sum(x['bytes'] for x in records)),indent=2)+'\n');(ROOT/'SHA256SUMS.txt').write_text(''.join(x['sha256']+'  '+x['path']+'\n' for x in records))
print(json.dumps(report,indent=2));sys.exit(bool(errors))
