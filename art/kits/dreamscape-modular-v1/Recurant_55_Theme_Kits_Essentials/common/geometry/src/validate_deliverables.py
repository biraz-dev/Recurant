#!/usr/bin/env python3
"""Read the actual GLB binaries, not generator state, and verify the handoff."""
import json,struct,pathlib,hashlib,csv
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
DIMS={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4};TYPES={5126:'<f4',5123:'<u2',5125:'<u4',5121:'u1'}
def read_glb(path):
 data=path.read_bytes();magic,ver,length=struct.unpack_from('<III',data);assert magic==0x46546c67 and ver==2 and length==len(data)
 pos=12;doc=None;bin=None
 while pos<len(data):
  n,t=struct.unpack_from('<II',data,pos);pos+=8;chunk=data[pos:pos+n];pos+=n
  if t==0x4e4f534a:doc=json.loads(chunk)
  elif t==0x004e4942:bin=chunk
 assert doc is not None and bin is not None
 def access(i):
  a=doc['accessors'][i];v=doc['bufferViews'][a['bufferView']];assert 'byteStride' not in v
  start=v.get('byteOffset',0)+a.get('byteOffset',0);dt=np.dtype(TYPES[a['componentType']]);n=a['count']*DIMS[a['type']]
  assert start+n*dt.itemsize<=len(bin)
  return np.frombuffer(bin,dtype=dt,count=n,offset=start).reshape((a['count'],DIMS[a['type']]))
 return doc,access

def triangle_intersects_box(v,c,h):
 v=v-c;edges=[v[1]-v[0],v[2]-v[1],v[0]-v[2]];axes=list(np.eye(3))+[np.cross(edges[0],edges[1])]+[np.cross(a,b) for a in edges for b in np.eye(3)]
 for a in axes:
  if np.linalg.norm(a)<1e-9:continue
  p=v@a;r=np.dot(np.abs(a),h)
  if p.min()>r or p.max()<-r:return False
 return True

def inspect(path,rec=None):
 doc,get=read_glb(path);issues=[];pv=[];triangles=[];vertex_count=0;maxindex=0;volume=0
 for node in doc['nodes']:
  if any(k in node for k in ['matrix','translation','rotation','scale']):issues.append('nonidentity serialized node transform')
 for mesh in doc['meshes']:
  for p in mesh['primitives']:
   assert p.get('mode',4)==4
   attrs=p['attributes'];assert {'POSITION','NORMAL','TEXCOORD_0'}<=set(attrs)
   xyz=get(attrs['POSITION']);n=get(attrs['NORMAL']);uv=get(attrs['TEXCOORD_0']);idx=get(p['indices']).flatten();a=doc['accessors'][p['indices']]
   if a['componentType']!=5123:issues.append('indices notuint16')
   assert len(idx)%3==0 and len(xyz)==len(n)==len(uv);assert idx.max()<len(xyz) and idx.max()<=65534
   for x in [xyz,n,uv]:
    if not np.isfinite(x).all():issues.append('nonfinite')
   if np.abs(np.linalg.norm(n,axis=1)-1).max()>1e-5:issues.append('nonunit normals')
   tri=xyz[idx.reshape(-1,3)];cross=np.cross(tri[:,1]-tri[:,0],tri[:,2]-tri[:,0]);area=np.linalg.norm(cross,axis=1)/2
   if (area<1e-10).any():issues.append('degenerate triangle')
   if (np.einsum('ij,ij->i',cross,n[idx[::3]])<-.000001).any():issues.append('normal/winding mismatch')
   volume+=float(np.einsum('ij,ij->i',tri[:,0],np.cross(tri[:,1],tri[:,2])).sum()/6)
   pv.append(xyz);triangles.extend(tri);vertex_count+=len(xyz);maxindex=max(maxindex,int(idx.max()))
 xyz=np.concatenate(pv);mn=xyz.min(0);mx=xyz.max(0)
 if volume<=0:issues.append('nonpositive signedvolume')
 openings=[]
 if rec:
  assert hashlib.sha256(path.read_bytes()).hexdigest()==rec['sha256']
  if not np.allclose(mn,rec['bounds_m']['min'],atol=1e-5) or not np.allclose(mx,rec['bounds_m']['max'],atol=1e-5):issues.append('manifest bounds mismatch')
  if vertex_count!=rec['metrics']['vertices'] or len(triangles)!=rec['metrics']['triangles']:issues.append('manifest metrics mismatch')
  for o in rec['collision']['clear_openings']:
   c=np.array(o['center']);half=np.array([o['width_m']/2,o['height_m']/2,.18])-.000001
   hits=sum(triangle_intersects_box(t,c,half) for t in triangles)
   openings.append({'width_m':o['width_m'],'height_m':o['height_m'],'depth_m':.35,'intersecting_triangles':hits,'test_tolerance_m':.000001,'pass':hits==0})
   if hits:issues.append('declared aperture intersectsgeometry')
 return {'file':str(path.relative_to(ROOT)),'pass':not issues,'issues':issues,'vertices':vertex_count,'triangles':len(triangles),'max_index':maxindex,'signed_volume_m3':round(volume,6),'bounds_min':mn.tolist(),'bounds_max':mx.tolist(),'aperture_checks':openings}

def main():
 man=json.loads((ROOT/'geometry-manifest.json').read_text());schema=json.loads((ROOT.parent/'spec'/'geometry_manifest.schema.json').read_text())
 import jsonschema
 jsonschema.Draft202012Validator(schema).validate(man)
 checks=[inspect(ROOT/a['files']['glb'],a) for a in man['assets']]
 assemblies=[inspect(ROOT/a['file']) for a in man['assemblies']]
 proxies=[]
 for a in man['assets']:
  path=ROOT/a['files']['collision_guides'];c=json.loads(path.read_text());assert c['mode']=='authoring_guides_only';assert all(np.min(b['size'])>0 for b in c['boxes'])
  if 'preview_glb' in a['collision']:proxies.append(inspect(ROOT/a['collision']['preview_glb']))
 result={'pass':all(c['pass'] for c in checks+assemblies+proxies),'schema_valid':True,'assets_checked':len(checks),'assembly_glbs_checked':len(assemblies),'collision_guide_glbs_checked':len(proxies),'declared_apertures_checked':sum(len(c['aperture_checks']) for c in checks),'all_buffers_read_from_delivered_glb':True,'checks':checks,'assemblies':assemblies,'collision_guides':proxies,'limits':['Aperture checks use triangle versus3Dbox separating-axis test with1micrometre inset tolerance and35cm through-depth. They do not establish gameplay navigation.','Signed volume is total across intentionally overlapping closed component solids; no boolean manifold claim.','No runtime or game code executed.','Diffuse texture quality, seamlessness, shader/culling appearance in engine and navigation are separate integration QA.']}
 (ROOT/'qa'/'deliverable-validation.json').write_text(json.dumps(result,indent=2))
 with (ROOT/'asset-index.csv').open('w',newline='') as f:
  w=csv.writer(f);w.writerow(['id','family','width_m','height_m','depth_m','vertices','triangles','material_slots','pivot','glb'])
  for a in man['assets']:w.writerow([a['id'],a['family'],*a['bounds_m']['size'],a['metrics']['vertices'],a['metrics']['triangles'],'|'.join(a['material_slots']),a['pivot']['semantic'],a['files']['glb']])
 summary={k:result[k] for k in ['pass','schema_valid','assets_checked','assembly_glbs_checked','collision_guide_glbs_checked','declared_apertures_checked']};summary['failures']=[c for c in checks+assemblies+proxies if not c['pass']];print(json.dumps(summary,indent=2))
 if not result['pass']:raise SystemExit(1)
if __name__=='__main__':main()
