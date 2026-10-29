#!/usr/bin/env python3
"""Arrange actual Blender render thumbnails into labeled catalog QA sheets."""
import json, pathlib, math
from PIL import Image,ImageDraw,ImageFont
ROOT=pathlib.Path(__file__).resolve().parents[1];MAN=json.loads((ROOT/'geometry-manifest.json').read_text());out=ROOT/'previews'
font='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf';bold='/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf'
F=lambda s:ImageFont.truetype(font,s);B=lambda s:ImageFont.truetype(bold,s)
for fam,n in MAN['counts']['families'].items():
    assets=[a for a in MAN['assets'] if a['family']==fam];cols=5;cell=280;rows=math.ceil(len(assets)/cols)
    im=Image.new('RGB',(cols*cell,120+rows*330),(20,26,32));d=ImageDraw.Draw(im)
    d.text((28,20),'RECURANT / '+fam.upper(),font=B(30),fill=(238,230,207));d.text((28,65),f'{n} original meshes  •  2 m authoring grid  •  diffuse-role display colours  •  not concept art',font=F(18),fill=(168,190,197))
    for i,a in enumerate(assets):
        x=(i%cols)*cell;y=120+(i//cols)*330;pic=Image.open(out/'thumbnails'/f"{a['id']}.png").convert('RGB').resize((270,270),Image.Resampling.LANCZOS);im.paste(pic,(x+5,y))
        text=a['id'].replace('_',' ');words=text.split();lines=['']
        for w in words:
            if len(lines[-1])+len(w)>27:lines.append(w)
            else:lines[-1]+=(' ' if lines[-1] else '')+w
        for k,line in enumerate(lines):d.text((x+12,y+275+k*19),line,font=F(15),fill=(232,234,230))
        d.text((x+12,y+315),f"{a['metrics']['triangles']} tris",font=F(12),fill=(132,160,166))
    im.save(out/f'contact_{fam}.png')
# compact all-asset board
cols=10;cell=170;rows=math.ceil(len(MAN['assets'])/cols);im=Image.new('RGB',(cols*cell,100+rows*205),(20,26,32));d=ImageDraw.Draw(im)
d.text((24,18),'RECURANT / 150 REAL MODULAR MESHES',font=B(30),fill=(238,230,207));d.text((24,60),'10 reusable geometry families • default display materials • each asset delivered as its own GLB',font=F(18),fill=(168,190,197))
for i,a in enumerate(MAN['assets']):
    x=(i%cols)*cell;y=100+(i//cols)*205;pic=Image.open(out/'thumbnails'/f"{a['id']}.png").convert('RGB').resize((165,165),Image.Resampling.LANCZOS);im.paste(pic,(x+2,y))
    txt=a['id'].replace('_',' ')
    words=txt.split();lines=['']
    for w in words:
        if len(lines[-1])+len(w)>22:lines.append(w)
        else:lines[-1]+=(' ' if lines[-1] else '')+w
    for k,line in enumerate(lines):d.text((x+5,y+167+k*15),line,font=F(11),fill=(229,234,228))
im.save(out/'all_150_meshes_contact.png')
im=Image.new('RGB',(1800,1650),(20,26,32));d=ImageDraw.Draw(im);d.text((25,20),'RECURANT / SIX REAL ASSEMBLY STUDIES',font=B(32),fill=(238,230,207))
for i,a in enumerate(MAN['assemblies']):
    x=(i%3)*600;y=100+(i//3)*770;pic=Image.open(out/(a['id']+'.png')).convert('RGB');pic.thumbnail((590,680),Image.Resampling.LANCZOS);im.paste(pic,(x+5,y));d.text((x+15,y+690),a['id'].replace('_demo','').replace('_',' ').title(),font=B(22),fill=(229,234,228))
im.save(out/'assembly_contact.png')
print('Made10family sheets, all150overview,6scenecontact')
