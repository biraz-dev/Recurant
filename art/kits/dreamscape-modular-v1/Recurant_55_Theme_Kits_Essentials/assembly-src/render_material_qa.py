#!/usr/bin/env python3
"""Regenerate material contact/repeat diagnostics from authoritative delivered PNGs."""
import pathlib,json,sys
from PIL import Image
ROOT=pathlib.Path(__file__).resolve().parents[1]
for mf in sorted((ROOT/'themes').glob('*/theme-manifest.json')):
 m=json.loads(mf.read_text());out=mf.parent/'qa/regenerated';out.mkdir(parents=True,exist_ok=True);contact=Image.new('RGB',(512,512))
 for i,role in enumerate(['wall','ground','structure','detail']):
  im=Image.open(ROOT/m['material_bindings'][role]['basecolor']).convert('RGB');contact.paste(im,((i%2)*256,(i//2)*256));repeat=Image.new('RGB',(768,768))
  for y in range(3):
   for x in range(3):repeat.paste(im,(x*256,y*256))
  repeat.save(out/(role+'_repeat_3x3.png'))
 contact.save(out/'material_contact.png')
 print(m['theme_id'])
