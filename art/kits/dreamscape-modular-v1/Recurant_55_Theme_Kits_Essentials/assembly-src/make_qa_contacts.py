#!/usr/bin/env python3
"""QA-only labelled contact sheets from actual 3D renders (resize and montage)."""
import pathlib,json
from PIL import Image,ImageDraw,ImageFont
from theme_recipes import R
ROOT=pathlib.Path(__file__).resolve().parents[1];OUT=ROOT/'qa/theme-assemblies';OUT.mkdir(parents=True,exist_ok=True)
font=ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',14)
for family in ['organic','architecture','technical','landscape']:
 for view in ['overview','native_320x240']:
  slugs=[s for s,r in R.items() if r['family']==family];w,h=320,266;sheet=Image.new('RGB',(4*w,((len(slugs)+3)//4)*h),(22,26,30));draw=ImageDraw.Draw(sheet)
  for i,slug in enumerate(slugs):
   p=ROOT/'themes'/slug/'assembly'/f'{view}.png';x=(i%4)*w;y=(i//4)*h
   if p.exists():im=Image.open(p).convert('RGB');im.thumbnail((320,240));sheet.paste(im,(x,y))
   else:draw.text((x+8,y+100),'RENDER PENDING',font=font,fill='red')
   draw.text((x+6,y+244),R[slug]['title'],font=font,fill='white')
  sheet.save(OUT/f'{family}_{view}_contact.png')
 print('CONTACT',family)
