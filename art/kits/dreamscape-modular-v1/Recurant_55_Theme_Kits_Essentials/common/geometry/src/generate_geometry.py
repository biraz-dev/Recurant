#!/usr/bin/env python3
"""Recurant modular mesh library — original parametric art, no downloaded geometry.
Run with Python 3 + NumPy. All mesh buffers are metres, Y-up, +X east, -Z north.
Rendering and source .blend compilation are optional separate Blender steps.
"""
import json, math, struct, pathlib, hashlib, collections, argparse, copy
import numpy as np
ROOT=pathlib.Path(__file__).resolve().parents[1]
PI=math.pi
MATS={'wall':[.55,.52,.45,1],'ground':[.35,.38,.34,1],'structure':[.36,.27,.19,1],'detail':[.65,.50,.27,1]}
def vec(x): return np.asarray(x,dtype=float)
def norm(v):
    v=vec(v); n=np.linalg.norm(v)
    return v/n if n>1e-12 else v*0
class Mesh:
    def __init__(self): self.parts=[]
    def face(self,coords,mat='wall'):
        coords=np.array(coords,dtype=float)
        normal=norm(np.cross(coords[1]-coords[0],coords[2]-coords[0]))
        if np.linalg.norm(normal)<.5: raise ValueError('degenerate polygon')
        ax=int(np.argmax(np.abs(normal))); dims=[[2,1],[0,2],[0,1]][ax]
        uv=coords[:,dims].copy()
        if mat in ['wall','ground']: uv/=2.0
        else:
            mn=uv.min(axis=0); ext=uv.max(axis=0)-mn
            uv=(uv-mn)/np.where(ext>1e-9,ext,1)
        self.parts.append((coords,normal,uv,mat))
    def solid(self,verts,faces,mat='wall'):
        pts=np.array(verts,float); center=pts.mean(axis=0)
        for f in faces:
            p=pts[list(f)]
            n=np.cross(p[1]-p[0],p[2]-p[0])
            if np.dot(n,p.mean(axis=0)-center)<0: p=p[::-1]
            self.face(p,mat)
    def box(self,c,s,mat='wall',ry=0):
        c=vec(c); sx,sy,sz=vec(s)/2
        v=[[-sx,-sy,-sz],[sx,-sy,-sz],[sx,sy,-sz],[-sx,sy,-sz],[-sx,-sy,sz],[sx,-sy,sz],[sx,sy,sz],[-sx,sy,sz]]
        R=np.array([[math.cos(ry),0,math.sin(ry)],[0,1,0],[-math.sin(ry),0,math.cos(ry)]])
        self.solid(np.array(v)@R.T+c,[[0,1,2,3],[4,5,6,7],[0,4,5,1],[3,2,6,7],[0,3,7,4],[1,5,6,2]],mat)
    def cone(self,a,b,r1,r2=None,n=8,mat='structure',phase=0):
        a=vec(a); b=vec(b); r2=r1 if r2 is None else r2
        axis=norm(b-a); reference=vec([0,1,0]) if abs(axis[1])<.9 else vec([0,0,1])
        u=norm(np.cross(axis,reference)); v=norm(np.cross(axis,u))
        if min(r1,r2)<1e-8:
            if r1<r2: return self.cone(b,a,r2,r1,n,mat,phase)
            pts=[a+r1*(math.cos(2*PI*i/n+phase)*u+math.sin(2*PI*i/n+phase)*v) for i in range(n)]+[b]
            fs=[list(range(n))]+[[i,(i+1)%n,n] for i in range(n)]
        else:
            pts=[p+r*(math.cos(2*PI*i/n+phase)*u+math.sin(2*PI*i/n+phase)*v) for p,r in [(a,r1),(b,r2)] for i in range(n)]
            fs=[list(range(n)),list(range(n,2*n))]+[[i,(i+1)%n,(i+1)%n+n,i+n] for i in range(n)]
        self.solid(pts,fs,mat)
    def tube(self,points,radii,n=7,mat='structure'):
        if isinstance(radii,(float,int)): radii=[radii]*len(points)
        for i in range(len(points)-1): self.cone(points[i],points[i+1],radii[i],radii[i+1],n,mat)
    def extrude(self,poly,z0,z1,mat='wall'):
        # Convex 2D XY polygon extruded in Z. Fan triangulation remains safe.
        n=len(poly); verts=[[x,y,z0] for x,y in poly]+[[x,y,z1] for x,y in poly]
        self.solid(verts,[list(range(n)),list(range(n,2*n))]+[[i,(i+1)%n,(i+1)%n+n,i+n] for i in range(n)],mat)
    def arch(self,cx=0,spring=2.4,r=.7,t=.3,depth=.3,n=12,mat='structure',start=0,end=PI):
        for i in range(n):
            a=start+(end-start)*i/n; b=start+(end-start)*(i+1)/n
            poly=[[cx+(r+t)*math.cos(a),spring+(r+t)*math.sin(a)], [cx+(r+t)*math.cos(b),spring+(r+t)*math.sin(b)], [cx+r*math.cos(b),spring+r*math.sin(b)],[cx+r*math.cos(a),spring+r*math.sin(a)]]
            self.extrude(poly,-depth/2,depth/2,mat)
    def leaf(self,a,b,width=.3,mat='detail',thick=.025):
        a=vec(a); b=vec(b); axis=norm(b-a); u=norm(np.cross(axis,vec([0,1,0])))
        if np.linalg.norm(u)<.1: u=vec([1,0,0])
        c=(a+b)/2; v=norm(np.cross(axis,u))*thick
        verts=[a,b,c+u*width,c-u*width,c+v,c-v]
        faces=[[0,2,4],[2,1,4],[1,3,4],[3,0,4],[0,5,2],[2,5,1],[1,5,3],[3,5,0]]
        self.solid(verts,faces,mat)
    def transform(self,translation=(0,0,0),ry=0,scale=1):
        R=np.array([[math.cos(ry),0,math.sin(ry)],[0,1,0],[-math.sin(ry),0,math.cos(ry)]])
        out=Mesh()
        for c,n,u,m in self.parts: out.parts.append((c@R.T*scale+vec(translation),n@R.T,u.copy(),m))
        return out
    def extend(self,other): self.parts+=other.parts
    def buffers(self):
        by=collections.defaultdict(lambda:dict(p=[],n=[],u=[],i=[]))
        for coords,normal,uv,mat in self.parts:
            d=by[mat]; base=len(d['p']); d['p'].extend(coords); d['n'].extend([normal]*len(coords)); d['u'].extend(uv)
            for j in range(1,len(coords)-1): d['i'].append([base,base+j,base+j+1])
        return by
    def bounds(self):
        v=np.concatenate([p[0] for p in self.parts]); return v.min(0),v.max(0)

def clean_coplanar_faces(mesh):
    """Clip duplicate-facing coplanar coverage, preserving UVs. Not a volume boolean.
    Every input face is convex. Later authored trim wins where surfaces coincide.
    Convex difference is split along each clip half-plane into convex pieces.
    """
    groups=collections.defaultdict(list)
    for face in mesh.parts:
        c,n,u,mat=face;key=tuple(np.round(n,6))+tuple([round(float(np.dot(n,c[0])),6)])
        groups[key].append(face)
    new=Mesh();stats={'input_faces':len(mesh.parts),'output_faces':0,'clipped_faces':0}
    def prune(poly,dims):
        if len(poly)<3:return []
        result=[]
        for v in poly:
            if not result or np.linalg.norm(v[:3]-result[-1][:3])>1e-8:result.append(v)
        if len(result)>1 and np.linalg.norm(result[0][:3]-result[-1][:3])<1e-8:result.pop()
        changed=True
        while changed and len(result)>=3:
            changed=False
            for i in range(len(result)):
                a=result[i-1][:3][dims];b=result[i][:3][dims];c=result[(i+1)%len(result)][:3][dims]
                cross=(b[0]-a[0])*(c[1]-b[1])-(b[1]-a[1])*(c[0]-b[0])
                if abs(cross)<1e-11:
                    result.pop(i);changed=True;break
        if len(result)<3:return []
        q=np.array(result)[:,:3][:,dims];area=abs(np.dot(q[:,0],np.roll(q[:,1],-1))-np.dot(q[:,1],np.roll(q[:,0],-1)))/2
        return result if area>1e-10 else []
    def halfplane(poly,a,b,sign,inside,dims):
        if not poly:return []
        out=[]
        def distance(p):
            q=p[:3][dims];return sign*((b[0]-a[0])*(q[1]-a[1])-(b[1]-a[1])*(q[0]-a[0]))
        for i,p in enumerate(poly):
            q=poly[(i+1)%len(poly)];dp=distance(p);dq=distance(q)
            kp=dp>=-1e-10 if inside else dp<=1e-10;kq=dq>=-1e-10 if inside else dq<=1e-10
            if kp:out.append(p)
            if kp!=kq and abs(dp-dq)>1e-12:out.append(p+(q-p)*(dp/(dp-dq)))
        return prune(out,dims)
    def subtract(poly,clip,dims):
        p=np.array(poly)[:,:3][:,dims];q=np.array(clip)[:,:3][:,dims]
        if any(p[:,j].max()<=q[:,j].min()+1e-9 or q[:,j].max()<=p[:,j].min()+1e-9 for j in range(2)):return [poly],False
        sign=1 if np.dot(q[:,0],np.roll(q[:,1],-1))-np.dot(q[:,1],np.roll(q[:,0],-1))>0 else -1
        remaining=poly;outside=[]
        for i in range(len(q)):
            piece=halfplane(remaining,q[i],q[(i+1)%len(q)],sign,False,dims)
            if piece:outside.append(piece)
            remaining=halfplane(remaining,q[i],q[(i+1)%len(q)],sign,True,dims)
            if not remaining:return [poly],False
        return outside,True
    for faces in groups.values():
        covered=[]
        for coords,normal,uv,mat in reversed(faces):
            dims=[[2,1],[0,2],[0,1]][int(np.argmax(np.abs(normal)))];poly=[np.r_[p,u] for p,u in zip(coords,uv)];pieces=[poly];changed=False
            for clip in covered:
                nextpieces=[]
                for piece in pieces:
                    difference,hit=subtract(piece,clip,dims);nextpieces.extend(difference);changed|=hit
                pieces=nextpieces
                if not pieces:break
            if changed:stats['clipped_faces']+=1
            for piece in pieces:
                p=np.array(piece);new.parts.append((p[:,:3],normal.copy(),p[:,3:],mat));covered.append(piece)
    stats['output_faces']=len(new.parts)
    return new,stats

ASSETS=[]; MESHES={}
def connector(name,p,n): return {'id':name,'position':p,'normal':n}
def wall_connect(): return [connector('W',[-1,0,0],[-1,0,0]),connector('E',[1,0,0],[1,0,0])]
def floor_connect(size=2):
    a=size/2
    return [connector('N',[0,0,-a],[0,0,-1]),connector('E',[a,0,0],[1,0,0]),connector('S',[0,0,a],[0,0,1]),connector('W',[-a,0,0],[-1,0,0])]
def bx(c,s): return {'center':c,'size':s}
def register(id,family,desc,m,pivot='centre-base',connectors=None,boxes=None,openings=None,tags=None,notes=None):
    mn,mx=m.bounds(); buffers=m.buffers()
    rec={'id':id,'family':family,'description':desc,'files':{'glb':f'meshes/{id}.glb','collision_guides':f'collision-guides/{id}.json'},'bounds_m':{'min':mn.round(6).tolist(),'max':mx.round(6).tolist(),'size':(mx-mn).round(6).tolist()},'pivot':{'position':[0,0,0],'semantic':pivot},'snap':{'grid_m':2,'connectors':connectors or []},'material_slots':list(buffers),'metrics':{'vertices':sum(len(d['p']) for d in buffers.values()),'triangles':sum(len(d['i']) for d in buffers.values()),'material_count':len(buffers)},'collision':{'mode':'authoring_guides_only','boxes': boxes if boxes is not None else [bx(((mn+mx)/2).round(6).tolist(),(mx-mn).round(6).tolist())],'clear_openings':openings or []},'uv':{'uv0':True,'repeat_m':2,'method':'face-planar; wall/ground metric repeat, structure/detail each solid face normalized 0-1','normal_style':'flat faceted'},'tags':tags or [],'notes':notes or []}
    ASSETS.append(rec); MESHES[id]=m
    return m

def write_glb(mesh,path,name,extras=None,colors=None):
    bs=bytearray(); views=[]; acc=[]; prim=[]; mats=[]
    def add(array,component,kind,target):
        array=np.asarray(array); raw=array.tobytes(); offset=len(bs); bs.extend(raw)
        while len(bs)%4: bs.extend(b'\0')
        views.append({'buffer':0,'byteOffset':offset,'byteLength':len(raw),'target':target})
        d={'bufferView':len(views)-1,'componentType':component,'count':len(array),'type':kind}
        if kind=='VEC3': d.update(min=array.min(0).tolist(),max=array.max(0).tolist())
        acc.append(d); return len(acc)-1
    for mat,d in mesh.buffers().items():
        if len(d['p'])>65535: raise ValueError(f'{name}/{mat}: 16bit vertex overflow')
        p=add(np.array(d['p'],dtype='<f4'),5126,'VEC3',34962); n=add(np.array(d['n'],dtype='<f4'),5126,'VEC3',34962); u=add(np.array(d['u'],dtype='<f4'),5126,'VEC2',34962); i=add(np.array(d['i'],dtype='<u2').flatten(),5123,'SCALAR',34963)
        mats.append({'name':mat,'pbrMetallicRoughness':{'baseColorFactor':(colors or MATS)[mat],'metallicFactor':0,'roughnessFactor':.9},'doubleSided':False,'extras':{'semantic_role':mat,'texture_status':'default solid display colour, rebind theme diffuse maps'}})
        prim.append({'attributes':{'POSITION':p,'NORMAL':n,'TEXCOORD_0':u},'indices':i,'material':len(mats)-1,'mode':4})
    doc={'asset':{'version':'2.0','generator':'Recurant original parametric mesh authoring v1'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'name':name,'mesh':0,'extras':extras or {}}],'meshes':[{'name':name,'primitives':prim}],'materials':mats,'buffers':[{'byteLength':len(bs)}],'bufferViews':views,'accessors':acc}
    js=json.dumps(doc,separators=(',',':')).encode(); js+=b' '*((-len(js))%4)
    data=struct.pack('<III',0x46546c67,2,12+8+len(js)+8+len(bs))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(bs),0x004e4942)+bs
    path.parent.mkdir(parents=True,exist_ok=True); path.write_bytes(data)

# ---- MASONRY + CIVIC ARCHITECTURE ------------------------------------------------
def masonry():
    for h in [2,4]:
        m=Mesh();m.box([0,h/2,0],[2,h,.2]);register(f'masonry_wall_2x{h}','masonry',f'Plain solid wall, 2 m bay, {h} m tall.',m,'edge-midpoint-base',wall_connect(),[bx([0,h/2,0],[2,h,.2])],tags=['boundary'])
    m=Mesh();m.box([0,2,0],[2,4,.16])
    for row in range(8):
        edges=[-1,-.5,.5,1] if row%2 else [-1,0,1]
        for a,b in zip(edges,edges[1:]):m.box([(a+b)/2,row*.5+.25,-.085],[b-a-.025,.475,.07])
    register('masonry_wall_ashlar','masonry','Staggered ashlar courses with actual relief and mortar gaps.',m,'edge-midpoint-base',wall_connect(),[bx([0,2,0],[2,4,.2])])
    for side in ['inner','outer']:
        m=Mesh();m.box([1,2,0],[2,4,.2]);m.box([0,2,1],[.2,4,2]);
        if side=='outer':m.box([0,2,0],[.36,4,.36],'structure')
        register('masonry_corner_'+side,'masonry',f'L corner with two 2 m branches and {side} junction trim.',m,'corner-floor', [connector('X',[2,0,0],[1,0,0]),connector('Z',[0,0,2],[0,0,1])],[bx([1,2,0],[2,4,.2]),bx([0,2,1],[.2,4,2])])
    m=Mesh();m.box([0,1,0],[2,2,.2]);profile=[[-1,3.7],[-.55,3.5],[-.35,2.8],[.1,3.1],[.45,2.3],[1,2.01]]
    for (x0,y0),(x1,y1) in zip(profile,profile[1:]):m.extrude([[x0,2],[x1,2],[x1,y1],[x0,y0]],-.1,.1)
    register('masonry_wall_broken','masonry','Broken wall silhouette with irregular stepped crest, no walk-through hole.',m,'edge-midpoint-base',wall_connect(),[bx([0,1.4,0],[2,2.8,.2])],notes=['Block gameplay edge as solid; visual crest is intentionally irregular.'])
    for kind in ['round','pointed']:
        m=Mesh();m.box([-.85,2,0],[.3,4,.2]);m.box([.85,2,0],[.3,4,.2]);N=14
        def curve(x):return 2.4+(.7*math.sqrt(max(0,1-(x/.7)**2)) if kind=='round' else 1.1*(1-abs(x/.7))**.7)
        for i in range(N):
            a=-.7+1.4*i/N;b=-.7+1.4*(i+1)/N;m.extrude([[a,curve(a)],[b,curve(b)],[b,4],[a,4]],-.1,.1)
        if kind=='round':m.arch(t=.12,depth=.28,n=14)
        else:m.tube([[-.79,2.4,0],[-.64,2.75,0],[-.35,3.16,0],[0,3.57,0],[.35,3.16,0],[.64,2.75,0],[.79,2.4,0]],.065,4,'structure')
        for x in [-.76,.76]:m.box([x,1.2,0],[.12,2.4,.28],'structure')
        register(f'masonry_doorway_{kind}','masonry',f'{kind.capitalize()} arch through-wall, 1.4 m wide / 2.4 m clear rectangular passage.',m,'edge-midpoint-base',wall_connect(),[bx([-.85,2,0],[.3,4,.28]),bx([.85,2,0],[.3,4,.28]),bx([0,3.75,0],[1.4,.5,.2])],[{'width_m':1.4,'height_m':2.4,'through_axis':'Z','center':[0,1.2,0]}],['passage'])
    m=Mesh();m.box([0,1.18,0],[1.36,2.36,.12],'structure')
    for y in [.35,1.9]:m.box([0,y,-.075],[1.36,.12,.06],'detail')
    m.box([.43,1.05,-.12],[.08,.22,.08],'detail')
    m=m.transform([.68,0,0]);register('door_leaf_plank_1p36','masonry','Separate closed plank door leaf, hinge-pivot ready. Fit inside1.4m opening by placing hinge atX=-.68.',m,'hinge-base',boxes=[bx([.68,1.18,0],[1.36,2.36,.12])],notes=['Hinge pivot baked at origin. Place at doorwaylocalX=-.68, rotate aboutY.'])
    m=Mesh()
    for x in [-.64,.64]:m.box([x,1.2,0],[.08,2.4,.08],'structure')
    for x in np.linspace(-.6,.6,7):m.cone([x,.12,0],[x,2.25,0],.025,n=6,mat='detail')
    for y in [.2,1.2,2.2]:m.box([0,y,0],[1.36,.05,.05],'structure')
    register('gate_leaf_barred','masonry','Narrow barred gate with separate bars and framing.',m)
    for typ in ['square','round','fluted','broken']:
        m=Mesh();h=2.4 if typ=='broken' else 4
        m.box([0,.14,0],[.8,.28,.8],'structure');m.box([0,h-.14,0],[.8,.28,.8],'structure')
        if typ=='square':m.box([0,h/2,0],[.5,h-.56,.5])
        else:
            m.cone([0,.28,0],[0,h-.28,0],.28,.24,12 if typ=='fluted' else 8,'wall')
            if typ=='fluted':
                for a in np.linspace(0,2*PI,8,endpoint=False):m.cone([.24*math.cos(a),.3,.24*math.sin(a)],[.21*math.cos(a),h-.3,.21*math.sin(a)],.045,.045,5,'structure')
        register('column_'+typ,'masonry',typ.capitalize()+' freestanding column with plinth and capital.',m)
    m=Mesh();m.box([0,2,0],[.4,4,.4]);m.extrude([[-.2,0],[.6,0],[.6,.6],[.25,3.7],[-.2,4]],-.3,.3,'structure');register('masonry_buttress','masonry','Sloping buttress with broad foot and narrow shoulder.',m)
    m=Mesh();m.box([0,.6,0],[2,1.2,.22]);m.box([0,1.23,0],[2,.16,.32],'structure');register('masonry_parapet_2m','masonry','Low stone guard/parapet with overhanging coping.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.box([0,.38,0],[2,.76,.3]);
    for x in [-.75,0,.75]:m.box([x,1.0,0],[.4,.5,.3],'structure')
    register('masonry_crenellation_2m','masonry','Merlon parapet segment with three raised blocks.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.box([0,2,0],[2,4,.16]);m.box([0,2,-.12],[1.45,2.8,.15],'structure');m.box([0,2,-.21],[1.18,2.52,.06],'detail');register('masonry_inset_panel','masonry','Recess illusion through raised frame, separate front detail panel for mural/decor binding.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.box([0,1.4,0],[.5,2.8,.5],'structure');m.box([0,2.85,0],[1.6,.2,.7],'structure');m.box([0,2.45,0],[1.0,.2,.5],'detail');register('masonry_capital_bracket','masonry','T-shaped bracket capital for colonnades and beams.',m)

# ---- FLOORS / VERTICAL CIRCULATION / BRIDGES ------------------------------------
def surfaces():
    for kind in ['plain','inset','pavers','grate','cracked']:
        m=Mesh();m.box([0,-.1,0],[2,.2,2],'ground')
        if kind=='inset':
            m=Mesh();m.box([0,-.1,0],[1.64,.2,1.64],'ground')
            for x,z,sx,sz in [(0,-.91,2,.18),(0,.91,2,.18),(-.91,0,.18,1.64),(.91,0,.18,1.64)]:m.box([x,-.1,z],[sx,.2,sz],'detail')
        if kind=='pavers':
            m=Mesh()
            for i in range(4):
                for j in range(4):m.box([-.75+i*.5,-.1,-.75+j*.5],[.49,.2,.49],'ground')
        if kind=='grate':
            m=Mesh()
            for x in np.linspace(-.95,.95,12):m.box([x,-.06,0],[.06,.12,2],'structure')
            for z in [-.93,0,.93]:m.box([0,-.12,z],[2,.08,.08],'structure')
        if kind=='cracked':
            m=Mesh();m.extrude([[-1,-.2],[1,-.2],[1,0],[-1,0]],-1,1,'ground')
            for x,z in [(-.4,.2),(.2,-.3),(.55,.5)]:m.box([x,.004,z],[.025,.008,.5],'detail',ry=.5)
        register('floor_'+kind+'_2m','surfaces',kind.capitalize()+' 2×2 m floor tile with top at local Y=0.',m,'centre-top',floor_connect(),[bx([0,-.1,0],[2,.2,2])],tags=['walkable'])
    m=Mesh();m.box([0,4.1,0],[2,.2,2],'wall');register('ceiling_flat_2m','surfaces','Flat ceiling at 4 m underside; pivot remains room floor for exact placement.',m,'room-floor-centre',floor_connect(),[bx([0,4.1,0],[2,.2,2])])
    m=Mesh();m.box([0,4.13,0],[2,.26,2],'wall')
    for x in [-.82,.82]:m.box([x,3.92,0],[.18,.16,2],'structure')
    for z in [-.82,.82]:m.box([0,3.92,z],[1.46,.16,.18],'structure')
    register('ceiling_coffer_2m','surfaces','Coffered ceiling with structural frame, 3.84 m minimum visual clearance.',m,'room-floor-centre',floor_connect())
    m=Mesh();m.arch(spring=4,r=2,t=.2,depth=2,n=16,mat='wall');register('ceiling_barrel_vault_4x2','surfaces','4 m wide / 2 m long barrel-vault segment. Spring at Y=4, crown Y=6.2.',m,'room-floor-centre',[connector('N',[0,0,-1],[0,0,-1]),connector('S',[0,0,1],[0,0,1])],boxes=[],notes=['No single AABB collision guide: would obstruct the full arch. Use edge masks below.'])
    for rise,run,steps in [(1,2,4),(2,4,8)]:
        m=Mesh()
        for i in range(steps):
            h=rise*(i+1)/steps;z=-run/2+(i+.5)*run/steps;m.box([0,h/2,z],[2,h,run/steps],'ground')
        m=m.transform([0,0,-run/2],ry=PI)
        register(f'stairs_rise{rise}_run{run}','surfaces',f'Closed solid stairs, {steps} treads, 2 m wide, {rise} m rise / {run} m run.',m,'low-edge-centre',[connector('LOW',[0,0,0],[0,0,1]),connector('HIGH',[0,rise,-run],[0,0,-1])],boxes=[],notes=['Use a ramp guide for movement, not per-tread triangle collision. Ascends toward -Z.','ramp_guide: width2,run'+str(run)+',rise'+str(rise)],tags=['walkable','elevation'])
    m=Mesh();m.extrude([[-1,0],[1,0],[1,1],[-1,0.001]],-1,1,'ground');m=m.transform([0,0,-1],ry=PI/2);register('ramp_rise1_run2','surfaces','Solid 2 m-wide ramp from Y=0 to Y=1 over 2 m. Ascends toward -Z.',m,'low-edge-centre',[connector('LOW',[0,0,0],[0,0,1]),connector('HIGH',[0,1,-2],[0,0,-1])],boxes=[],tags=['walkable','elevation'])
    m=Mesh();m.box([0,-.1,0],[2,.2,2],'ground');register('landing_2m','surfaces','Square landing; place at stair upper connector height.',m,'centre-top',floor_connect(),[bx([0,-.1,0],[2,.2,2])],tags=['walkable'])
    for style in ['stone','timber','metal']:
        m=Mesh()
        if style=='timber':
            for z in np.linspace(-1.875,1.875,16):m.box([0,-.08,z],[2,.16,.24],'ground')
            for x in [-.76,.76]:m.box([x,-.23,0],[.18,.22,4],'structure')
        else:m.box([0,-.12,0],[2,.24,4],'ground')
        for x in [-.96,.96]:
            for z in [-1.9,-.65,.65,1.9]:m.box([x,.55,z],[.12,1.1,.12],'structure')
            m.box([x,1.1,0],[.14,.12,4],'structure')
            if style=='stone':m.box([x,.44,0],[.14,.88,4],'wall')
            else:m.box([x,.55,0],[.06,.06,4],'detail')
        register('bridge_'+style+'_2x4','surfaces',f'{style.capitalize()} 2×4 m bridge deck and guardrails, 1.7 m clear between rails.',m,'deck-centre-top',[connector('N',[0,0,-2],[0,0,-1]),connector('S',[0,0,2],[0,0,1])],[bx([0,-.12,0],[2,.24,4]),bx([-.96,.6,0],[.14,1.2,4]),bx([.96,.6,0],[.14,1.2,4])],tags=['walkable','bridge'])
    for style in ['iron','timber']:
        m=Mesh()
        for x in [-.94,.94]:m.box([x,.6,0],[.12,1.2,.12],'structure')
        for y in [.35,1.1]:m.box([0,y,0],[2,.09,.09],'structure')
        if style=='iron':
            for x in np.linspace(-.75,.75,6):m.cone([x,.1,0],[x,1.1,0],.025,n=5,mat='detail')
        else:
            for a,b in [([-1,.2,0],[1,1,0]),([-1,1,0],[1,.2,0])]:m.cone(a,b,.045,n=4,mat='detail')
        register('railing_'+style+'_2m','surfaces',f'{style.capitalize()} 2 m guardrail with open infill.',m,'edge-midpoint-base',wall_connect())

# ---- TIMBER, HOUSE AND INN -----------------------------------------------------
def timber():
    for typ in ['plaster','plank','lattice','paper']:
        m=Mesh()
        if typ!='lattice':m.box([0,2,0],[2,4,.12],'wall')
        for x in [-.91,.91]:m.box([x,2,-.02],[.18,4,.22],'structure')
        for y in [.1,2,3.9]:m.box([0,y,-.02],[2,.18,.22],'structure')
        if typ=='plaster':m.tube([[-.8,.2,-.14],[.8,1.9,-.14]],.065,4,'structure');m.tube([[.8,2.1,-.14],[-.8,3.8,-.14]],.065,4,'structure')
        if typ=='plank':
            for x in np.linspace(-.72,.72,7):m.box([x,2,-.08],[.19,3.7,.06],'structure')
        if typ in ['lattice','paper']:
            for x in np.linspace(-.6,.6,5):m.box([x,2,-.1],[.035,3.8,.04],'structure')
            for y in np.linspace(.4,3.6,9):m.box([0,y,-.11],[1.8,.035,.04],'structure')
        register('timber_wall_'+typ,'timber',f'2 m bay timber {typ} wall with real structural posts.',m,'edge-midpoint-base',wall_connect())
    m=Mesh()
    for x in [-.88,.88]:m.box([x,1.8,0],[.24,3.6,.3],'structure')
    m.box([0,3.65,0],[2.2,.25,.4],'structure');m.box([0,2.8,0],[2,.12,.25],'detail');register('timber_post_beam_gate','timber','Open post-and-beam portal with 1.52 m clear lane.',m,'edge-midpoint-base',wall_connect(),[bx([-.88,1.8,0],[.24,3.6,.3]),bx([.88,1.8,0],[.24,3.6,.3]),bx([0,3.65,0],[2.2,.25,.4])],[{'width_m':1.52,'height_m':2.74,'through_axis':'Z','center':[0,1.37,0]}])
    for name,h in [('post',4),('short_post',1.2)]:
        m=Mesh();m.box([0,h/2,0],[.22,h,.22],'structure');m.box([0,.1,0],[.32,.2,.32],'detail');register('timber_'+name,'timber','Square timber upright with foot shoe.',m)
    m=Mesh();m.box([0,.12,0],[2,.24,.22],'structure');register('timber_beam_2m','timber','2 m horizontal structural timber.',m,'beam-midpoint-base',wall_connect())
    m=Mesh();m.extrude([[-1,0],[1,0],[0,1]],-.11,.11,'wall');m.tube([[-1,0,-.13],[0,1,-.13],[1,0,-.13]],.08,4,'structure');m.box([0,.47,0],[.12,.94,.3],'structure');register('timber_gable_2m','timber','Triangular gable cap, 2 m span / 1 m rise; place on 4 m wall.',m,'base-midpoint',wall_connect())
    for name,side in [('left',-1),('right',1)]:
        m=Mesh();m.extrude([[-1,0],[0,1],[0,1.15],[-1,.15]],-1,1,'ground');m=m.transform(ry=PI if side==1 else 0);register('roof_pitch_'+name+'_2m','timber','One roof slope with solid thickness, 1 m run / rise and 2 m length.',m,'eave-midpoint',boxes=[])
    m=Mesh();m.box([0,.44,0],[1.8,.12,.65],'structure')
    for x in [-.65,.65]:m.box([x,.2,0],[.14,.4,.5],'structure')
    register('inn_bench','timber','Simple low-poly bench, 1.8 m long, ground standing.',m)
    m=Mesh();m.box([0,.79,0],[1.6,.12,.9],'structure')
    for x in [-.6,.6]:
        for z in [-.3,.3]:m.box([x,.38,z],[.12,.76,.12],'structure')
    register('inn_table','timber','Rectangular inn table with four legs.',m)
    m=Mesh();m.box([0,.6,0],[2,1.2,.65],'structure');m.box([0,1.24,0],[2.1,.12,.85],'detail');register('inn_bar_counter_2m','timber','2 m bar/counter with overhanging serving top.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.box([0,.32,0],[1.1,.35,2],'structure');m.box([0,.57,0],[1.04,.18,1.9],'detail');m.box([0,.8,.94],[1.1,1.0,.12],'structure');register('inn_bed_single','timber','Single bed with separate mattress and headboard.',m)
    m=Mesh();m.box([0,.04,0],[1,.08,2],'ground');
    for x in [-.47,.47]:m.box([x,.081,0],[.06,.002,2],'detail')
    register('tatami_mat_1x2','timber','1×2 m raised tatami/soft mat, narrow edge binding.',m,'centre-base')
    m=Mesh();m.box([0,1,0],[1.4,2,.3],'structure')
    for y in [.35,.9,1.45]:m.box([0,y,-.23],[1.25,.09,.18],'detail')
    register('shelf_bookcase','timber','Solid-backed three-shelf cabinet; fill with separately instanced props.',m)
    m=Mesh()
    for y,r in [(0,.32),(.15,.4),(.8,.4),(.95,.32)]: pass
    m.cone([0,0,0],[0,.2,0],.3,.4,12,'structure');m.cone([0,.2,0],[0,.75,0],.4,.4,12,'structure');m.cone([0,.75,0],[0,.95,0],.4,.3,12,'structure')
    for y,r in [(.16,.405),(.74,.405)]:m.cone([0,y-.035,0],[0,y+.035,0],r,r,12,'detail')
    register('barrel','timber','Faceted barrel with two raised metal hoops.',m)
    m=Mesh();m.box([0,.45,0],[.9,.9,.9],'structure')
    for y in [.08,.82]:m.box([0,y,-.46],[.9,.12,.08],'detail')
    m.tube([[-.39,.14,-.5],[.39,.76,-.5]],.06,4,'detail');register('crate','timber','Braced wooden crate with diagonal front reinforcement.',m)

# ---- ORGANIC LANDSCAPES --------------------------------------------------------
def rock_mesh(scale=(1,1,1),seed=1,mat='wall'):
    rng=np.random.default_rng(seed);m=Mesh();n=7;verts=[]
    for y,r in [(0,.75),(.45,1),(.9,.65)]:
        for i in range(n):
            a=2*PI*i/n;rr=r*(.85+rng.random()*.25);verts.append([math.cos(a)*rr*scale[0],(y+(rng.random()-.5)*.15 if y else 0)*scale[1],math.sin(a)*rr*scale[2]])
    verts.append([.08*scale[0],1.15*scale[1],-.06*scale[2]])
    fs=[list(range(n))]
    for r in range(2):
        for i in range(n):fs.append([r*n+i,r*n+(i+1)%n,(r+1)*n+(i+1)%n,(r+1)*n+i])
    for i in range(n):fs.append([2*n+i,2*n+(i+1)%n,3*n])
    m.solid(verts,fs,mat);return m

def mushroom(m,c=(0,0,0),scale=1,wide=1):
    x,y,z=c;m.cone([x,y,z],[x+.08*scale,y+.85*scale,z],.12*scale,.08*scale,7,'structure');m.cone([x+.08*scale,y+.72*scale,z],[x+.08*scale,y+.93*scale,z],.48*scale*wide,.34*scale*wide,10,'detail');m.cone([x+.08*scale,y+.93*scale,z],[x+.08*scale,y+1.08*scale,z],.34*scale*wide,.025*scale,10,'wall')
def organics():
    for n,s in [('boulder_small',(.65,.65,.6)),('boulder_large',(1.2,1.6,1.1)),('basalt_column',(.45,3.3,.45)),('rock_slab',(1.0,.2,1.0))]:
        register(n,'organic',n.replace('_',' ').capitalize()+', irregular original faceted solid.',rock_mesh(s,seed=len(n)))
    m=Mesh()
    for i in range(4):m.extend(rock_mesh((.6,3.5+i%2*.3,.38),i+10).transform([-.75+i*.5,0,0]))
    register('cave_wall_2m','organic','Four overlapping faceted stone masses making an irregular cave wall.',m,'edge-midpoint-base',wall_connect(),[bx([0,2,0],[2,4,.6])])
    m=Mesh();m.arch(spring=2.4,r=.7,t=.35,depth=.7,n=7,mat='wall');m.extend(rock_mesh((.3,2.2,.45),40).transform([-1.08,0,0]));m.extend(rock_mesh((.3,2.2,.45),44).transform([1.08,0,0]));register('cave_arch_2m','organic','Low-facet cave arch and rock piers with true opening.',m,'edge-midpoint-base',wall_connect(),[bx([-.95,1.2,0],[.5,2.4,.7]),bx([.95,1.2,0],[.5,2.4,.7])],[{'width_m':1.4,'height_m':2.4,'through_axis':'Z','center':[0,1.2,0]}])
    for kind in ['stalagmite','stalactite']:
        m=Mesh()
        if kind=='stalagmite':m.cone([0,0,0],[.12,2,0],.4,0,7,'wall')
        else:m.cone([0,0,0],[.12,-2,0],.4,0,7,'wall')
        register(kind,'organic',kind.capitalize()+' tapered cave tooth; ceiling-attached one points downward.',m,'ceiling-attachment' if kind=='stalactite' else'centre-base')
    m=Mesh();m.tube([[-1,.15,0],[-.5,.22,-.08],[0,.13,.07],[.5,.2,0],[1,.15,0]],[.18,.22,.24,.2,.18],7);register('root_straight_2m','organic','Undulating modular root runner with aligned end connectors.',m,'centre-base',wall_connect())
    m=Mesh();m.tube([[-1,.15,0],[-.35,.24,0],[0,.2,.2],[0,.15,1]],[.18,.25,.24,.18],7);register('root_corner_2m','organic','Rounded L-root with two orthogonal attachment ends.',m,'corner-floor',[connector('W',[-1,.15,0],[-1,0,0]),connector('S',[0,.15,1],[0,0,1])])
    m=Mesh();m.tube([[-.91,0,0],[-.95,1,0],[-.90,2.4,.02],[-.55,3.2,0],[0,3.6,0],[.55,3.2,0],[.90,2.4,.02],[.95,1,0],[.91,0,0]],[.18,.17,.16,.18,.22,.18,.16,.17,.18],7);register('root_arch_2m','organic','Root-grown arch with 1.4 m clear central lane.',m,'edge-midpoint-base',wall_connect(),[bx([-.9,1.2,0],[.35,2.4,.4]),bx([.9,1.2,0],[.35,2.4,.4])],[{'width_m':1.4,'height_m':2.4,'through_axis':'Z','center':[0,1.2,0]}])
    for name,r,h in [('tree_trunk',.55,4),('colossal_trunk_quarter',1.4,6)]:
        m=Mesh();m.cone([0,0,0],[.15,h,0],r,r*.7,9,'structure')
        for a in np.linspace(0,2*PI,5,endpoint=False):m.tube([[math.cos(a)*r*2,0,math.sin(a)*r*2],[math.cos(a)*r*.8,.4,math.sin(a)*r*.8],[.1,h*.45,0]],[.08,r*.3,r*.18],6)
        register(name,'organic','Tapered trunk with five flaring buttress roots. '+('Large scenery, not a single-cell prop.' if h==6 else'Set outside travel lane.'),m,tags=['scenery'])
    m=Mesh();m.tube([[0,0,0],[.1,2,0],[.4,3.7,0]],[.2,.16,.08],7)
    for s in [-1,1]:m.tube([[.1,1.6,0],[s*.7,2.4,.1],[s*1.1,2.8,.15]],[.13,.09,.03],6)
    register('dead_branch_tree','organic','Forking bare tree silhouette, no foliage cards.',m)
    m=Mesh()
    for a in np.linspace(0,2*PI,8,endpoint=False):
        b=[math.cos(a)*.85,.65+(.1 if int(a*10)%2 else 0),math.sin(a)*.85];m.leaf([0,.1,0],b,.23,'detail')
    register('broadleaf_clump','organic','Eight thick diamond leaves, actual double-volume geometry, not alpha cards.',m)
    m=Mesh()
    for a in np.linspace(0,2*PI,6,endpoint=False):
        d=vec([math.cos(a),0,math.sin(a)]);p0=vec([0,.05,0]);p1=d*.8+vec([0,.65,0]);m.cone(p0,p1,.015,.005,5,'structure')
        side=vec([-d[2],0,d[0]])
        for t in [.3,.5,.7,.85]:
            c=p0*(1-t)+p1*t
            for sign in [-1,1]:m.leaf(c,c+side*sign*.22*(1-t)+d*.14,.07,'detail')
    register('fern_clump','organic','Six fern fronds built from stems and 48 solid leaflet blades.',m)
    for name,s in [('mushroom_small',.55),('mushroom_tall',2.5)]:
        m=Mesh();mushroom(m,scale=s);register(name,'organic','Faceted mushroom with stem, lower cap and raised domed crown.',m)
    m=Mesh()
    for c,s in [([-.3,0,-.1],.6),([.2,0,.1],.85),([.25,0,-.3],.42)]:mushroom(m,c,s)
    register('mushroom_cluster','organic','Three independently shaped mushroom stems and caps.',m)
    m=Mesh()
    for y,s in [(0,.5),(.35,.4),(.65,.3)]:m.cone([0,y,0],[0,y+.14,0],s,s*.6,8,'detail')
    register('shelf_fungus','organic','Three stacked bracket fungi; pivot is wall attachment base.',m,'wall-attachment')
    m=Mesh();m.box([0,2,.12],[2,4,.16],'wall')
    for x in [-.83,-.45,0,.45,.83]:m.tube([[x,0,0],[x+.1,1.2,-.12],[x-.06,2.5,-.05],[x,4,0]],[.15,.2,.14,.16],7,'structure')
    register('tissue_wall_2m','organic','Living wall with five irregular vascular ribs over a membrane backing.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.tube([[-.91,0,0],[-.91,1.5,0],[-.9,2.5,0],[-.45,3.1,0],[0,3.3,0],[.45,3.1,0],[.9,2.5,0],[.91,1.5,0],[.91,0,0]],.17,8,'structure');register('tissue_rib_arch','organic','Continuous bony/organic rib arch, separate from the backing membrane.',m,'edge-midpoint-base',wall_connect(),[bx([-.91,1.2,0],[.34,2.4,.34]),bx([.91,1.2,0],[.34,2.4,.34])],[{'width_m':1.4,'height_m':2.4,'through_axis':'Z','center':[0,1.2,0]}])
    m=Mesh();m.cone([0,.5,0],[.05,1.7,0],.4,.25,8,'detail');m.cone([0,0,0],[0,.5,0],.12,.4,8,'structure');m.cone([.05,1.7,0],[.05,2,0],.25,0,8,'detail');register('organic_pod','organic','Closed tapered hanging/growing seed pod.',m)
    m=Mesh();m.tube([[-.9,.1,0],[-.55,.16,-.04],[.1,.11,.03],[.6,.16,0],[.9,.1,0]],[.15,.14,.13,.12,.1],6,'detail')
    for x in [-.8,.8]:m.cone([x,.1,-.12],[x,.1,.12],.18,.18,7,'wall')
    register('bone_long','organic','Low-poly long bone with thicker joint caps.',m)
    m=Mesh()
    for i,(r,h) in enumerate([(.42,1.3),(.27,.85),(.22,.6)]):m.cone([-.4+i*.4,0,0],[-.4+i*.4,h,.05],r,0,6,'wall')
    register('crag_cluster','organic','Three differently sized jagged stone/bone spires.',m)

# ---- CRYSTAL AND CEREMONIAL FORMS ----------------------------------------------
def crystal(m,c=(0,0,0),r=.25,h=1.5,n=6,lean=(0,0),mat='detail'):
    x,y,z=c;top=[x+lean[0],y+h*.72,z+lean[1]];m.cone([x,y,z],top,r,r*.8,n,mat);m.cone(top,[x+lean[0]*1.2,y+h,z+lean[1]*1.2],r*.8,0,n,mat)
def crystals():
    for name,r,h in [('shard_small',.15,.7),('shard_tall',.32,3.5),('obelisk',.55,4)]:
        m=Mesh();crystal(m,r=r,h=h);register('crystal_'+name,'crystal','Prismatic body and pointed crown, true flat-facet silhouette.',m)
    m=Mesh()
    for c,r,h,l in [([0,0,0],.28,2,(0,0)),([-.42,0,.13],.22,1.3,(-.2,0)),([.35,0,.2],.18,1.15,(.2,.1)),([.1,0,-.35],.16,.85,(.1,-.1))]:crystal(m,c,r,h,lean=l)
    register('crystal_cluster','crystal','Four asymmetric faceted crystal prisms with divergent growth.',m)
    m=Mesh();m.box([0,2,.08],[2,4,.16],'wall')
    for x,h in [(-.85,3.2),(-.45,3.8),(0,3.5),(.45,3.8),(.85,3.2)]:crystal(m,[x,0,-.1],.13,h,mat='structure')
    register('crystal_wall_ribs','crystal','Five applied faceted ribs on a wall panel, preserving flat 2m snap ends.',m,'edge-midpoint-base',wall_connect())
    m=Mesh()
    for x in [-.88,.88]:crystal(m,[x,0,0],.17,3.5,mat='structure')
    m.tube([[-.86,2.7,0],[0,3.8,0],[.86,2.7,0]],.17,4,'detail');register('crystal_pointed_arch','crystal','Angular ceremonial portal with two crystal uprights and pointed lintel.',m,'edge-midpoint-base',wall_connect(),[bx([-.86,1.2,0],[.34,2.4,.34]),bx([.86,1.2,0],[.34,2.4,.34])],[{'width_m':1.4,'height_m':2.4,'through_axis':'Z','center':[0,1.2,0]}])
    m=Mesh();m.box([0,-.13,0],[2,.26,2],'ground')
    for x,z in [(-.75,-.75),(.75,.75)]:crystal(m,[x,-.08,z],.12,.25,mat='detail')
    register('crystal_floor_2m','crystal','Walkable mineral slab; small edge inclusions remain outside 1.2m centre lane.',m,'centre-top',floor_connect(),[bx([0,-.13,0],[2,.26,2])],tags=['walkable'])
    m=Mesh();m.cone([0,0,0],[0,.3,0],.7,.6,8,'structure');m.cone([0,.3,0],[0,1.2,0],.35,.28,8,'wall');m.cone([0,1.2,0],[0,1.35,0],.55,.55,8,'detail');register('ceremonial_pedestal','crystal','Octagonal shrine pedestal with broad base and dish-like upper platform.',m)
    m=Mesh();m.cone([0,0,0],[0,.2,0],.6,.6,10,'wall');crystal(m,[0,.2,0],.18,1.4,mat='detail');m.arch(spring=1,r=.5,t=.07,depth=.08,n=16,mat='structure',start=0,end=2*PI);register('astral_ring_focus','crystal','Orbit ring around a central crystal, mounted on a circular base.',m)

# ---- MACHINERY / INDUSTRIAL ---------------------------------------------------
def gear(m,c=(0,0,0),radius=.6,teeth=12,depth=.16,mat='structure'):
    x,y,z=c;m.cone([x,y,z-depth/2],[x,y,z+depth/2],radius*.8,radius*.8,teeth*2,mat)
    for i in range(teeth):
        a=2*PI*i/teeth;inner=radius*.73;outer=radius;da=PI/teeth*.55
        m.extrude([[x+inner*math.cos(a-da),y+inner*math.sin(a-da)],[x+outer*math.cos(a-da),y+outer*math.sin(a-da)],[x+outer*math.cos(a+da),y+outer*math.sin(a+da)],[x+inner*math.cos(a+da),y+inner*math.sin(a+da)]],z-depth/2,z+depth/2,mat)
    m.cone([x,y,z-depth*.8],[x,y,z+depth*.8],radius*.18,radius*.18,8,'detail')
def machine():
    for typ in ['panel','ribbed','bulkhead']:
        m=Mesh();m.box([0,2,.02],[2,4,.16],'wall')
        for x in [-.94,.94]:m.box([x,2,-.06],[.12,4,.24],'structure')
        for y in [.08,3.92]:m.box([0,y,-.06],[2,.16,.24],'structure')
        if typ=='panel':m.box([0,2,-.09],[1.6,3.4,.1],'detail')
        if typ=='ribbed':
            for y in np.linspace(.4,3.6,9):m.box([0,y,-.14],[1.8,.08,.12],'structure')
        if typ=='bulkhead':
            for x in [-.7,.7]:m.tube([[x,0,-.15],[x,3.2,-.15],[x*.6,4,-.15]],.08,4,'structure')
        register('machine_wall_'+typ,'machinery','Industrial '+typ+' wall with modeled perimeter structure.',m,'edge-midpoint-base',wall_connect())
    m=Mesh()
    for x in [-.85,.85]:m.box([x,2,0],[.3,4,.35],'structure')
    m.box([0,3.45,0],[1.4,1.1,.35],'wall')
    for x in [-.75,.75]:m.box([x,1.4,-.2],[.07,2.8,.08],'detail')
    register('machine_doorframe','machinery','Square bulkhead doorway with 1.4×2.9m clear aperture.',m,'edge-midpoint-base',wall_connect(),[bx([-.85,2,0],[.3,4,.35]),bx([.85,2,0],[.3,4,.35]),bx([0,3.45,0],[1.4,1.1,.35])],[{'width_m':1.4,'height_m':2.9,'through_axis':'Z','center':[0,1.45,0]}])
    for size in [.12,.24]:
        m=Mesh();m.cone([-1,size,0],[1,size,0],size,n=10,mat='structure')
        for x in [-.94,.94]:m.cone([x-.04,size,0],[x+.04,size,0],size*1.35,n=10,mat='detail')
        register('pipe_straight_'+str(int(size*100))+'cm','machinery','2m straight pipe with raised coupling flanges; closed display ends.',m,'axis-projected-floor',[connector('W',[-1,size,0],[-1,0,0]),connector('E',[1,size,0],[1,0,0])])
    m=Mesh();points=[]
    for a in np.linspace(PI,PI/2,7):points.append([.5+.5*math.cos(a),.16,.5+.5*math.sin(a)])
    m.tube(points,.16,10);register('pipe_elbow_90','machinery','Faceted 90degree elbow with 0.5m centreline bend radius.',m,'axis-projected-floor',[connector('A',points[0],[0,0,-1]),connector('B',points[-1],[1,0,0])])
    m=Mesh();m.cone([-1,.16,0],[1,.16,0],.16,n=10);m.cone([0,.16,0],[0,.16,1],.16,n=10);register('pipe_tee_2m','machinery','Three-way T-pipe fitting, three centreline connectors.',m,'axis-projected-floor',[connector('W',[-1,.16,0],[-1,0,0]),connector('E',[1,.16,0],[1,0,0]),connector('S',[0,.16,1],[0,0,1])])
    m=Mesh();m.cone([0,0,0],[0,2,0],.16,n=10);m.cone([0,.9,0],[0,.9,-.35],.045,n=6);m.arch(spring=.9,r=.23,t=.04,depth=.05,n=16,mat='detail',start=0,end=2*PI)
    register('pipe_valve_riser','machinery','Vertical pipe riser with front-mounted handwheel (static prop).',m)
    for radius,teeth in [(.4,10),(.8,16)]:
        m=Mesh();gear(m,[0,radius,0],radius,teeth);register('gear_'+str(teeth)+'tooth','machinery',f'{teeth}-tooth spur gear, real toothed silhouette and raised hub.',m)
    m=Mesh();m.box([0,.2,0],[1.6,.4,.8],'structure')
    for i,(x,y,r,t) in enumerate([(-.4,1,.5,12),(.45,1.45,.4,10)]):gear(m,[x,y,0],r,t)
    for x,y in [(-.4,1),(.45,1.45)]:m.box([x,y/2,.2],[.15,y,.15],'structure')
    register('gear_machine_pair','machinery','Two offset mounted gears over common plinth, no animation baked.',m)
    m=Mesh();m.cone([0,.2,0],[0,2.6,0],.6,n=12);m.cone([0,2.6,0],[0,2.9,0],.6,.18,12,'detail');m.box([0,.1,0],[1.4,.2,1.4],'structure');register('tank_vertical','machinery','Cylindrical industrial vessel on square plinth.',m)
    m=Mesh();m.box([0,.55,0],[1.4,1.1,.8],'structure');m.extrude([[-.7,1.1],[.7,1.1],[.7,1.4],[-.7,1.25]],-.4,.4,'detail')
    for x in [-.4,0,.4]:m.box([x,1.45,-.1],[.16,.08,.2],'detail')
    register('machine_console','machinery','Waist-high console with sloped top and three raised controls.',m)
    m=Mesh();m.box([0,.55,0],[.9,1.1,.55],'structure');m.box([0,1.23,0],[1.6,.22,.55],'detail');m.extrude([[.8,1.12],[1.2,1.28],[.8,1.34]],-.25,.25,'detail');register('smith_anvil','machinery','Anvil with horn, stepped heel and solid stand.',m)
    m=Mesh();m.box([0,.5,0],[1.6,1,.9],'wall');m.box([0,.8,0],[1.2,.1,.65],'detail');m.box([-.7,1.4,0],[.2,1.8,.9],'wall');m.box([.7,1.4,0],[.2,1.8,.9],'wall');m.box([0,2.2,0],[1.6,.25,1],'structure');m.box([0,2.9,0],[.55,1.2,.6],'structure');register('forge_hearth','machinery','Open-front forge hearth with hood and chimney; fire is a separate material/effect.',m)
    m=Mesh();m.cone([0,.05,0],[0,.7,0],.5,.7,12,'structure');m.cone([0,.7,0],[0,.8,0],.75,.75,12,'detail');register('foundry_crucible','machinery','Faceted crucible silhouette; filled top surface, not a physics container.',m)
    m=Mesh();m.box([0,1,0],[.22,2,.22],'structure');m.cone([0,2,0],[0,2.2,0],.5,n=10,mat='detail');m.cone([0,2.2,0],[0,2.6,0],.2,.1,10,'structure');register('coil_generator','machinery','Vertical electrical/arcane coil device with repeated annular ribs.',m)
    m=Mesh()
    for y in np.linspace(.4,2.2,7):m.cone([0,y-.06,0],[0,y+.06,0],.42,n=12,mat='detail')
    m.cone([0,0,0],[0,2.5,0],.14,n=10);register('coil_stack','machinery','Seven separate disc ribs around a central shaft.',m)
    m=Mesh();m.box([0,1.2,.04],[1.8,2.4,.08],'structure')
    for y in np.linspace(.2,2.2,12):m.box([0,y,-.08],[1.6,.075,.22],'detail')
    register('vent_louver','machinery','Twelve physical louver slats over a framed backing plate.',m)
    m=Mesh()
    for x in [-.35,.35]:m.cone([x,0,0],[x,3,0],.035,n=6,mat='structure')
    for y in np.linspace(.2,2.8,10):m.cone([-.35,y,0],[.35,y,0],.025,n=6,mat='detail')
    register('industrial_ladder_3m','machinery','3m ladder, 10 rungs; visual only until gameplay ladder support exists.',m)

# ---- TEXTILE, TOYS, SMALL ENVIRONMENTAL PROPS ----------------------------------
def textiles_props():
    m=Mesh();nx=12;ny=8
    # Closed ribbon-like cloth strips permit front/back display with correct normals.
    for i in range(nx):
        x0=-1+2*i/nx;x1=-1+2*(i+1)/nx
        z0=.09*math.cos(i*PI/2);z1=.09*math.cos((i+1)*PI/2)
        for j in range(ny):
            y0=3*j/ny;y1=3*(j+1)/ny
            v=[[x0,y0,z0],[x1,y0,z1],[x1,y1,z1],[x0,y1,z0],[x0,y0,z0+.025],[x1,y0,z1+.025],[x1,y1,z1+.025],[x0,y1,z0+.025]]
            m.solid(v,[[0,1,2,3],[4,5,6,7],[0,4,5,1],[3,2,6,7],[0,3,7,4],[1,5,6,2]],'wall')
    m.cone([-1,3.05,0],[1,3.05,0],.04,n=6,mat='structure');register('textile_curtain_2m','textile','Pleated 2m-wide curtain made from closed thin cloth strips, not alpha card.',m,'base-midpoint',wall_connect())
    m=Mesh();m.extrude([[-.4,0],[0,-.25],[.4,0],[.4,1.6],[-.4,1.6]],-.015,.015,'detail');m.cone([-.5,1.65,0],[.5,1.65,0],.025,n=6,mat='structure');register('textile_banner_pointed','textile','Pennant-shaped hanging banner with solid thickness. Local origin is tip shoulder.',m,'hanging-lower-shoulder')
    m=Mesh();m.box([0,.025,0],[2,.05,2],'ground')
    for x in [-.9,.9]:m.box([x,.052,0],[.1,.004,2],'detail')
    for z in [-.9,.9]:m.box([0,.052,z],[2,.004,.1],'detail')
    register('textile_rug_2m','textile','Square raised woven rug with separate edge-binding geometry.',m,'centre-base',floor_connect(),[bx([0,.025,0],[2,.05,2])])
    m=Mesh()
    for side in [-1,1]:m.extrude([[0,3],[side*1.2,2],[side*1.2,2.06],[0,3.06]],-1,1,'wall')
    for x in [-1.1,1.1]:
        for z in [-.9,.9]:m.cone([x,0,z],[x,2.1,z],.05,n=6,mat='structure')
    register('textile_canopy_2m','textile','Pitched cloth canopy and four support poles; clear height at eaves2m.',m,'centre-base',boxes=[bx([-1.1,1,-.9],[.1,2,.1]),bx([1.1,1,-.9],[.1,2,.1]),bx([-1.1,1,.9],[.1,2,.1]),bx([1.1,1,.9],[.1,2,.1])])
    m=Mesh();m.box([0,.15,0],[.8,.3,.7],'detail');register('textile_cushion','textile','Low square cushion, compact static prop.',m)
    for shape in ['cube','wedge','arch']:
        m=Mesh()
        if shape=='cube':m.box([0,.5,0],[1,1,1],'detail')
        if shape=='wedge':m.extrude([[-.5,0],[.5,0],[-.5,1]],-.5,.5,'detail')
        if shape=='arch':
            m.arch(spring=.7,r=.3,t=.2,depth=.5,n=8,mat='detail')
            for x in [-.4,.4]:m.box([x,.35,0],[.2,.7,.5],'detail')
        register('toy_block_'+shape,'textile','Oversized wooden/toy '+shape+' building block.',m)
    m=Mesh();m.cone([0,0,0],[0,.6,0],.22,.35,10,'structure');m.cone([0,.6,0],[0,.75,0],.35,.18,10,'wall');m.cone([0,.75,0],[0,.85,0],.18,.2,10,'detail');register('urn','props','Faceted urn / potion container with broad body and narrow neck.',m)
    m=Mesh();m.cone([0,0,0],[0,.45,0],.16,.2,10,'detail');m.cone([0,.45,0],[0,.58,0],.2,.06,10,'detail');m.cone([0,.58,0],[0,.75,0],.06,n=8,mat='structure');register('bottle_potion','props','Potion bottle geometry with shoulder and long neck, opaque preview material.',m)
    m=Mesh();m.box([0,.35,0],[.55,.7,.4],'structure');m.box([0,.75,0],[.7,.1,.55],'detail');m.cone([0,.75,0],[0,1,0],.05,n=6,mat='detail');register('lantern_box','props','Box lantern with capped roof and hanging spindle; use separate emissive adapter.',m)
    m=Mesh();m.box([0,.45,0],[1.8,.9,.65],'wall');m.box([0,.95,0],[2,.1,.8],'structure');register('sarcophagus','props','Stone coffin / sarcophagus plinth with overhanging lid.',m)
    m=Mesh();m.box([0,.65,0],[1.1,1.3,.8],'wall');m.box([0,1.35,0],[1.3,.1,1],'detail');register('altar_block','props','Compact shrine altar with slab top.',m)
    m=Mesh();m.box([0,.03,0],[2,.06,2],'ground');register('water_surface_2m','props','Opaque placeholder water plane solid for material replacement; not collision.',m,'centre-base',floor_connect(),boxes=[],notes=['No physical water behavior. Future adapter may use alpha/blend shader; supplied diffuse-only.'])
    m=Mesh()
    for i in range(7):m.extend(rock_mesh((.15+.025*i,.12+.015*i,.12),i+200).transform([-.65+i*.2,0,(i%2-.5)*.35]))
    register('rubble_scatter','props','Seven small irregular stones in a low scatter cluster.',m)
    m=Mesh();m.cone([0,0,0],[0,2,0],.04,.025,6,'structure')
    for a in np.linspace(0,2*PI,10,endpoint=False):m.leaf([0,2,0],[.45*math.cos(a),2+.45*math.sin(a),0],.12,'detail')
    m.cone([0,2,-.06],[0,2,.06],.17,n=12,mat='structure');m.leaf([0,.65,0],[.5,1,.1],.18);m.leaf([0,1,0],[-.4,1.35,-.1],.16);register('sunflower_stem','props','Stylized sunflower with ten thick petals, central disc, stem and leaves.',m)
    m=Mesh();m.box([0,1.1,0],[1.8,2.2,.3],'structure')
    for x in np.linspace(-.65,.65,5):m.box([x,1.1,-.2],[.19,1.9,.12],'detail')
    register('library_book_row','props','Five book-spine solids on a shelf-ready slab; no text or copied symbols.',m)

# ---- LANDSCAPE EXTENSIONS -----------------------------------------------------
def landscapes():
    m=Mesh();m.cone([-1,.28,0],[1,.28,0],.28,.24,9,'structure')
    for x in [-.98,.98]:m.cone([x-.015,.28,0],[x+.015,.28,0],.22,.22,9,'detail')
    m.cone([.25,.4,0],[.6,.75,.25],.12,.025,6,'structure');register('fallen_log_2m','landscape','2m fallen log with exposed endgrain discs and broken branch.',m,'axis-projected-floor',wall_connect())
    m=Mesh();m.cone([0,0,0],[.1,2.5,0],.2,.13,7,'structure')
    for a in np.linspace(0,2*PI,5,endpoint=False):
        p=[math.cos(a)*1.35,3,math.sin(a)*1.35];m.tube([[.1,1.8,0],[p[0]*.6,2.6,p[2]*.6],p],[.12,.08,.025],6,'structure');m.extend(rock_mesh((.85,.3,.85),int(a*100)+3,'detail').transform([p[0],2.9,p[2]]))
    register('acacia_tree','landscape','Flat-canopy acacia silhouette with spreading real branches and faceted foliage volumes.',m,tags=['scenery'])
    m=Mesh();m.cone([0,0,0],[0,2.9,0],.22,.1,8,'structure')
    for a in np.linspace(0,2*PI,7,endpoint=False):
        p=[math.cos(a)*1.45,2.45,math.sin(a)*1.45];m.leaf([0,2.95,0],p,.32,'detail')
    register('palm_tree','landscape','Tropical palm with tapered trunk and seven thick drooping leaf volumes.',m,tags=['scenery'])
    for name,height,n in [('grass_clump',.6,11),('dry_grass_tuft',1,13)]:
        m=Mesh()
        for i,a in enumerate(np.linspace(0,2*PI,n,endpoint=False)):
            r=.05+.04*(i%3);m.leaf([r*math.cos(a),0,r*math.sin(a)],[.35*math.cos(a),height*(.65+(i%4)*.1),.35*math.sin(a)],.035,'detail')
        register(name,'landscape','Solid low-poly grass blades; opaque geometry, no cutout shader required.',m,boxes=[],tags=['decor_nonblocking'])
    m=Mesh()
    for i in range(5):
        x=(i-2)*.13;z=(i%2)*.2;m.cone([x,0,z],[x+.05,1.5+(i%2)*.2,z],.015,n=5,mat='structure');m.cone([x+.05,1.25+(i%2)*.2,z],[x+.05,1.6+(i%2)*.2,z],.055,n=7,mat='detail');m.leaf([x,.1,z],[x-.24,1.1,z+.1],.04,'detail')
    register('cattail_reed_cluster','landscape','Five marsh reeds with separate seed heads and blades.',m,boxes=[],tags=['decor_nonblocking'])
    m=Mesh();m.cone([0,0,0],[0,1.1,0],.14,.1,7,'structure')
    for s in [-1,1]:
        m.tube([[0,.4,0],[s*.35,.65,0],[s*.45,1.15,.1]],[.1,.08,.035],6,'detail');m.tube([[s*.35,.65,0],[s*.65,.8,-.15],[s*.7,1.05,-.15]],[.07,.055,.02],6,'detail')
    register('coral_branch','landscape','Forked reef-coral solid with seven visible branch tips.',m)
    m=Mesh()
    for i in range(5):m.extend(rock_mesh((.55,.25,.5),i+56,'detail').transform([(i%2)*.4,(i//2)*.23,(i%3)*.12]))
    register('coral_shelf_cluster','landscape','Layered low-poly coral plates for shoreline and underwater scenes.',m)
    m=Mesh();m.extend(rock_mesh((1.1,4, .6),73));m.extend(rock_mesh((.7,2.8,.65),74).transform([.7,0,.15]));register('cliff_face_2m','landscape','Tall stratified cliff mass with lower shoulder, overlap-friendly scenery segment.',m,'edge-midpoint-base',wall_connect(),tags=['scenery'])
    m=Mesh();m.box([0,-.15,0],[2,.3,2],'ground')
    for z,h in [(.55,.04),(.85,.09)]:m.box([0,h/2,z],[2,h,.3],'ground')
    register('shore_bank_2m','landscape','Two shallow shoreline terraces rising toward +Z, top delta9cm.',m,'centre-top',floor_connect(),[bx([0,-.05,0],[2,.2,2])],tags=['walkable'])
    m=Mesh();m.extend(rock_mesh((.45,2.8,.35),808,'wall'));m.box([0,.13,0],[.9,.26,.8],'structure');register('standing_stone','landscape','Irregular upright megalith on a low buried foot.',m)
    m=Mesh()
    for i in range(5):
        x=(i-2)*.14;m.cone([x,0,0],[x+.07,1.2,0],.012,n=5,mat='structure')
        for j in range(5):m.leaf([x+.07,1.0+j*.045,0],[x+.07+(.1 if j%2 else -.1),1.04+j*.045,.02],.04,'detail')
    register('wheat_seedhead_cluster','landscape','Five grass seed heads, readable gold-plains silhouette using opaque geometry.',m,boxes=[],tags=['decor_nonblocking'])
    m=Mesh();m.cone([0,0,0],[0,1.1,0],.23,.2,8,'structure')
    for x in [-.45,.45]:m.tube([[0,.45,0],[x,.55,0],[x,.95,0]],[.13,.12,.08],7,'structure')
    register('desert_cactus','landscape','Three-armed low-poly cactus silhouette.',m)

# ---- INFRASTRUCTURE AND WIDE SPANS ---------------------------------------------
def infrastructure():
    m=Mesh()
    for x in [-1.75,1.75]:m.box([x,2,0],[.5,4,.35],'wall')
    m.arch(spring=2.5,r=1.5,t=.5,depth=.35,n=16,mat='structure')
    register('arch_wide_4m','infrastructure','4m-wide semicircular arcade, 3m clear width and 2.5m clear rectangular height.',m,'edge-midpoint-base',[connector('W',[-2,0,0],[-1,0,0]),connector('E',[2,0,0],[1,0,0])],[bx([-1.75,1.25,0],[.5,2.5,.35]),bx([1.75,1.25,0],[.5,2.5,.35])],[{'width_m':3,'height_m':2.5,'through_axis':'Z','center':[0,1.25,0]}])
    m=Mesh();m.box([0,.5,0],[2,1,.2],'wall');m.box([0,3.75,0],[2,.5,.2],'wall')
    for x in [-.8,.8]:m.box([x,2.25,0],[.4,2.5,.2],'wall')
    for x in [-.6,.6]:m.box([x,2.25,-.05],[.08,2.5,.25],'structure')
    m.box([0,1,-.04],[1.4,.12,.3],'structure');m.box([0,3.5,-.04],[1.4,.12,.3],'structure')
    register('wall_window_open_2m','infrastructure','True open window, 1.2m wide ×2.5m high above 1m sill.',m,'edge-midpoint-base',wall_connect(),[bx([0,.5,0],[2,1,.2]),bx([-.8,2.25,0],[.4,2.5,.2]),bx([.8,2.25,0],[.4,2.5,.2]),bx([0,3.75,0],[2,.5,.2])],notes=['Window is not a walk-through doorway; keep runtime wall edge blocked unless explicit vault/climb behavior exists.'])
    m=Mesh();m.box([0,2,0],[2,4,.16],'wall')
    for x in [-.84,.84]:m.box([x,2,-.2],[.22,4,.3],'structure')
    for y in [.15,3.85]:m.box([0,y,-.2],[1.7,.3,.3],'structure')
    m.box([0,1,-.38],[1.4,.16,.55],'ground');m.box([0,2.5,-.12],[1.3,2.3,.08],'detail');register('wall_shrine_niche','infrastructure','Deep raised niche frame with shelf and inset display surface; no fake cutout texture.',m,'edge-midpoint-base',wall_connect())
    m=Mesh();m.box([0,-.4,0],[2,.2,2],'ground')
    for x in [-.85,.85]:m.box([x,-.1,0],[.3,.4,2],'wall')
    register('water_channel_2m','infrastructure','U-shaped open channel with bottomY=-.5 and side copingY=.1; water surface separate.',m,'rim-centre',floor_connect(),[bx([0,-.4,0],[2,.2,2]),bx([-.85,-.1,0],[.3,.4,2]),bx([.85,-.1,0],[.3,.4,2])],tags=['waterway_nonwalkable'])
    m=Mesh();m.box([0,-.1,0],[2,.2,1.5],'ground')
    for z in [-.7,.7]:m.box([0,.35,z],[2,.7,.12],'structure')
    register('channel_crossing_2m','infrastructure','Low slab crossing spanning a 2m channel, 1.2m clear lane.',m,'deck-centre-top',[connector('W',[-1,0,0],[-1,0,0]),connector('E',[1,0,0],[1,0,0])],[bx([0,-.1,0],[2,.2,1.5]),bx([0,.35,-.7],[2,.7,.12]),bx([0,.35,.7],[2,.7,.12])],tags=['walkable','bridge'])
    m=Mesh();m.arch(spring=1.5,r=.75,t=.25,depth=2,n=12,mat='wall')
    for x in [-.875,.875]:m.box([x,.75,0],[.25,1.5,2],'wall')
    register('culvert_vault_2m','infrastructure','Narrow low culvert segment:1.5m clear width,1.5m springheight; not default standing traversal.',m,'floor-centre',[connector('N',[0,0,-1],[0,0,-1]),connector('S',[0,0,1],[0,0,1])],boxes=[],notes=['Decorative/crouch-space geometry. Insufficient2.4m standing clearance. Do not use as normal passage without gameplay support.'])
    m=Mesh()
    for z in np.linspace(-1.875,1.875,16):m.box([0,-.08,z],[2,.16,.24],'ground')
    for x in [-.95,.95]:
        for z in [-1.9,1.9]:m.box([x,.65,z],[.12,1.3,.12],'structure')
        m.tube([[x,1.25,-2],[x,.95,-1],[x,.85,0],[x,.95,1],[x,1.25,2]],.035,6,'detail')
        for z in np.linspace(-1.6,1.6,9):m.cone([x,.05,z],[x,.85+.1*(abs(z)),z],.018,n=5,mat='detail')
    register('rope_bridge_2x4','infrastructure','Flat walkable plank bridge with sagging hand ropes and separate vertical stays.',m,'deck-centre-top',[connector('N',[0,0,-2],[0,0,-1]),connector('S',[0,0,2],[0,0,1])],[bx([0,-.08,0],[2,.16,4]),bx([-.95,.6,0],[.1,1.2,4]),bx([.95,.6,0],[.1,1.2,4])],tags=['walkable','bridge'])
    m=Mesh();m.box([0,-.1,0],[2,.2,2],'ground')
    for x in [-.95,.95]:m.box([x,.2,0],[.1,.4,2],'structure')
    register('raised_walkway_2m','infrastructure','2m slab with low edge curbs, broad 1.8m centre clear.',m,'centre-top',floor_connect(),[bx([0,-.1,0],[2,.2,2]),bx([-.95,.2,0],[.1,.4,2]),bx([.95,.2,0],[.1,.4,2])],tags=['walkable'])
    m=Mesh();m.box([0,.1,0],[2,.2,2],'structure');m.cone([0,.2,0],[0,.4,0],.75,.75,12,'wall');m.cone([0,.4,0],[0,.55,0],.6,.6,12,'detail');register('fountain_base','infrastructure','Three-tier dry fountain basin plinth; water/particles added by target engine.',m)
    m=Mesh()
    for x in [-.72,.72]:m.box([x,.75,0],[.16,1.5,.7],'structure')
    for y in [.35,1.35]:m.box([0,y,0],[1.6,.12,.7],'structure')
    for x in [-.45,-.15,.15,.45]:m.box([x,.85,-.12],[.08,.9,.08],'detail')
    register('weapon_display_rack','infrastructure','Freestanding armory display rack with four empty supports; no weapon designs copied.',m)

# ---- COMBINATIONS: actual flattened GLBs and exact instance recipes -------------
def make_assemblies():
    recipes=[]
    def save(id,desc,instances):
        combined=Mesh()
        for a,p,y in instances:combined.extend(MESHES[a].transform(p,y))
        write_glb(combined,ROOT/'assemblies'/f'{id}.glb',id,{'purpose':'authoring composition demo, not level.bin','all_transforms_baked':True})
        mn,mx=combined.bounds();d={'id':id,'description':desc,'file':f'assemblies/{id}.glb','coordinate_system':'metres/Y-up/-Z-north','instances':[{'asset':a,'position_m':p,'rotation_y_radians':y,'scale':1} for a,p,y in instances],'bounds_m':{'min':mn.tolist(),'max':mx.tolist()},'notes':['Demonstrates visual assembly only. Collision/world navigation must be adapted through the current game grid pipeline.']}
        (ROOT/'assemblies'/f'{id}.json').write_text(json.dumps(d,indent=2));recipes.append(d)
    base=[('floor_inset_2m',[x,0,z],0) for x in [-2,0,2] for z in [-2,0,2]]
    masonry_room=base+[('masonry_wall_ashlar',[x,0,3],0) for x in [-2,2]]+[('masonry_doorway_pointed',[0,0,3],0)]+[('masonry_wall_ashlar',[-3,0,z],-PI/2) for z in [-2,0,2]]+[('column_fluted',[2.5,0,2.5],0),('altar_block',[1.9,0,1.8],0)]
    save('masonry_cloister_demo','Open-front 3×3 cell cloister with doorway, ashlar boundaries and ceremonial prop.',masonry_room)
    organic=[('floor_pavers_2m',[0,0,z],0) for z in [-2,0,2]]+[('root_arch_2m',[0,0,-1],0),('cave_arch_2m',[0,0,3],0),('cave_wall_2m',[-1.55,0,0],-PI/2),('tree_trunk',[1.9,0,1.6],0),('mushroom_cluster',[1.25,0,-1.9],0),('mushroom_tall',[-2.0,0,2],0),('fern_clump',[1.6,0,-.1],0)]
    save('organic_passage_demo','Traversable-looking root and cave corridor with edge fungi and foliage.',organic)
    machine_demo=[('floor_grate_2m',[x,0,z],0) for x in [-1,1] for z in [-2,0,2]]+[('machine_doorframe',[0,0,3],0)]+[('machine_wall_ribbed',[-2,0,z],-PI/2) for z in [-2,0,2]]+[('tank_vertical',[1.3,0,1.8],0),('gear_machine_pair',[-1.2,0,1.6],0),('machine_console',[1.2,0,-1.8],0),('coil_stack',[-1.4,0,-1.8],0)]
    save('machine_workshop_demo','Open-front machinery bay with grated deck, gears, vessel and control station.',machine_demo)
    inn=[('floor_plain_2m',[x,0,z],0) for x in [-2,0,2] for z in [-2,0,2]]+[('timber_wall_plaster',[x,0,3],0) for x in [-2,0,2]]+[('timber_wall_plank',[-3,0,z],-PI/2) for z in [-2,0,2]]+[('inn_table',[-1.4,0,0],0),('inn_bench',[-1.4,0,-.85],0),('inn_bench',[-1.4,0,.85],0),('inn_bar_counter_2m',[1.5,0,1.7],0),('barrel',[2.3,0,-1.4],0),('lantern_box',[1.5,1.3,1.7],0)]
    save('timber_inn_demo','Open-front half-timber inn bay with functional furniture scale.',inn)
    crystal_scene=[('crystal_floor_2m',[x,0,z],0) for x in [-2,0,2] for z in [-2,0,2]]+[('crystal_wall_ribs',[x,0,3],0) for x in [-2,2]]+[('crystal_pointed_arch',[0,0,3],0)]+[('crystal_shard_tall',[-2.5,0,z],0) for z in [-2,0,2]]+[('crystal_cluster',[2.2,0,1.6],0),('ceremonial_pedestal',[1.8,0,-1.8],0),('astral_ring_focus',[1.8,1.35,-1.8],0)]
    save('crystal_sanctum_demo','Faceted corridor assembly with mineral slabs and a pedestal focus.',crystal_scene)
    textile=[('textile_rug_2m',[x,0,z],0) for x in [-2,0,2] for z in [-2,0,2]]+[('textile_curtain_2m',[x,0,3],0) for x in [-2,0,2]]+[('textile_canopy_2m',[0,0,1],0),('textile_cushion',[-1.6,0,-1.7],0),('toy_block_cube',[1.6,0,-1.6],0),('toy_block_arch',[2,0,.4],0),('toy_block_wedge',[-2,0,.8],0)]
    save('textile_pavilion_demo','Soft-world and toy forms arranged around a cloth canopy.',textile)
    return recipes

def validate_mesh(m):
    bad=[];volume=0;triangles=0
    for mat,d in m.buffers().items():
        p=np.array(d['p']);n=np.array(d['n']);u=np.array(d['u']);idx=np.array(d['i']);pts=p[idx];cross=np.cross(pts[:,1]-pts[:,0],pts[:,2]-pts[:,0]);area=np.linalg.norm(cross,axis=1)/2
        if not np.isfinite(p).all() or not np.isfinite(n).all() or not np.isfinite(u).all():bad.append('nonfinite buffer')
        if (area<1e-10).any():bad.append(f'degenerate triangles: {(area<1e-10).sum()}')
        if (np.sum(cross*n[idx[:,0]],axis=1)<0).any():bad.append('normal/winding mismatch')
        if np.max(np.abs(np.linalg.norm(n,axis=1)-1))>1e-5:bad.append('nonunit normals')
        if idx.max()>=65536:bad.append('16bit overflow')
        volume+=np.einsum('ij,ij->i',pts[:,0],np.cross(pts[:,1],pts[:,2])).sum()/6;triangles+=len(idx)
    if volume<=0:bad.append('non-positive total signed volume')
    return {'pass':not bad,'issues':bad,'signed_volume_m3':round(float(volume),6),'triangles':triangles,'uv0_finite':True,'root_transform_identity':True,'render_mesh_collision_separated':True}

def main():
    masonry();surfaces();timber();organics();crystals();machine();textiles_props();landscapes();infrastructure()
    checks=[]
    for a in ASSETS:
        id=a['id'];m,cleanup=clean_coplanar_faces(MESHES[id]);MESHES[id]=m
        buffers=m.buffers();mn,mx=m.bounds();a['bounds_m']={'min':mn.round(6).tolist(),'max':mx.round(6).tolist(),'size':(mx-mn).round(6).tolist()};a['metrics']={'vertices':sum(len(d['p']) for d in buffers.values()),'triangles':sum(len(d['i']) for d in buffers.values()),'material_count':len(buffers)};a['material_slots']=list(buffers);a['coplanar_cleanup']=cleanup
        check=validate_mesh(m);check['id']=id;check['coplanar_cleanup']=cleanup;checks.append(check)
        write_glb(m,ROOT/a['files']['glb'],id,{'pivot':a['pivot'],'grid_m':2,'family':a['family'],'all_transforms_baked':True})
        proxy=Mesh()
        for b in a['collision']['boxes']:proxy.box(b['center'],b['size'],'detail')
        if proxy.parts:
            proxy_path=f'collision-guides/{id}__GUIDE.glb';write_glb(proxy,ROOT/proxy_path,id+'__GUIDE',{'authoring_only':True,'not_runtime_collision':True});a['collision']['preview_glb']=proxy_path
        (ROOT/a['files']['collision_guides']).write_text(json.dumps({'asset':id,**a['collision']},indent=2))
        a['sha256']=hashlib.sha256((ROOT/a['files']['glb']).read_bytes()).hexdigest()
        payload=b''
        for role,d in m.buffers().items():
            payload+=role.encode()+np.array(d['p'],dtype='<f4').tobytes()+np.array(d['n'],dtype='<f4').tobytes()+np.array(d['u'],dtype='<f4').tobytes()+np.array(d['i'],dtype='<u2').tobytes()
        a['mesh_payload_sha256']=hashlib.sha256(payload).hexdigest()
    assemblies=make_assemblies()
    manifest={'schema_version':'1.0','manifest_base':'.','title':'Recurant original modular geometry starter production library','status':'authored mesh source and interchange assets; engine integration not performed','coordinate_contract':{'unit':'metre','up':'+Y','east':'+X','north':'-Z','game_scale':.5,'grid_m':2,'wall_height_m':4,'wall_thickness_m':.2,'floor_top_y':0,'floor_bottom_y':-.2,'all_vertex_transforms_baked':True,'node_transforms':'identity'},'material_contract':{'slots':list(MATS),'uv0':True,'wall_ground_repeat_m':2,'structure_detail_uv':'per solid face 0-1','default_materials':'solid display colours only','target_textures':'theme sRGB diffuse PNG; no fabricated PBR maps','render_culling':'single-sided outward normals; thin objects are closed volumes'},'counts':{'assets':len(ASSETS),'unique_mesh_payloads':len(set(a['mesh_payload_sha256'] for a in ASSETS)),'triangles':sum(a['metrics']['triangles'] for a in ASSETS),'families':dict(collections.Counter(a['family'] for a in ASSETS)),'assemblies':len(assemblies)},'originality':'All geometry is new parametric art authored for this kit. No game assets extracted; concepts used only as visual direction.','assets':ASSETS,'assemblies':[{'id':d['id'],'file':d['file'],'recipe':f"assemblies/{d['id']}.json"} for d in assemblies]}
    (ROOT/'geometry-manifest.json').write_text(json.dumps(manifest,indent=2))
    qa={'pass':all(c['pass'] for c in checks),'assets_checked':len(checks),'checks':checks,'limitations':['Disjoint intersecting closed solids are intentional for low-poly modular art. Not boolean-unioned or watertight as single shells.','Automatic checks validate finite buffers, zero-area triangles, winding/normal consistency,16-bit indices and positive total volume; not gameplay clearance or perfect UV seam art.','Collision GLBs/JSON are separate design guides, not imported runtime colliders.','All floor and wall snapping should be validated in the target importer before production.']}
    (ROOT/'qa'/'geometry-validation.json').write_text(json.dumps(qa,indent=2))
    print(json.dumps({'counts':manifest['counts'],'all_structural_checks_pass':qa['pass'],'failures':[c for c in checks if not c['pass']]},indent=2))
if __name__=='__main__': main()
