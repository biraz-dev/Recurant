"""Crop/resize generated raster art; does not draw replacement art."""
from PIL import Image
from pathlib import Path
import json
R=Path(__file__).resolve().parent
# Inputs are bundled with the kit; no external workspace or generated-output IDs.
for name in ['equipment-pixel-refined.png','panel-generated.png','silhouette-clean-generated.png']:
    if not (R/'source'/name).is_file():
        raise FileNotFoundError('Missing bundled source: '+name)
for name in ['assets','previews']:
    (R/name).mkdir(exist_ok=True)
resample=Image.Resampling.NEAREST
im=Image.open(R/'source/equipment-pixel-refined.png').convert('RGBA').resize((320,240),resample)
im.save(R/'previews/equipment-320x240.png')
im.resize((1280,960),resample).save(R/'previews/equipment-preview-4x.png')
Image.open(R/'source/panel-generated.png').convert('RGBA').resize((320,240),resample).save(R/'assets/panel-320x240.png')
fig=Image.open(R/'source/silhouette-clean-generated.png').convert('RGBA')
assert fig.getchannel('A').getextrema()[0]==0,'Generated silhouette is not transparent'
# Preserve the full native alpha bbox (including faint alpha), matching the approved asset.
fig=fig.crop(fig.getchannel('A').getbbox()).resize((54,124),Image.Resampling.BOX)
fig.save(R/'assets/silhouette-54x124.png')
fig.resize((216,496),Image.Resampling.NEAREST).save(R/'previews/silhouette-alpha-4x.png')
rows=[
 ('head','HEAD',(142,28,36,35),(148,64,24,7)),
 ('body','BODY',(70,75,39,37),(77,113,25,7)),
 ('left_hand','LEFT HAND',(70,132,39,37),(64,170,49,7)),
 ('right_hand','RIGHT HAND',(211,132,39,37),(205,170,53,7)),
 ('ring_1','RING 1',(110,182,29,29),(110,211,30,7)),
 ('ring_2','RING 2',(181,182,29,29),(181,211,30,7)),
 ('amulet','AMULET',(213,75,37,37),(213,113,35,7))]
def crop(box):
 x,y,w,h=box
 return im.crop((x,y,x+w,y+h))
slots=[]
for id,label,box,labelbox in rows:
 art=f'assets/slot-{id}.png'; txt=f'assets/label-{id}.png'
 crop(box).save(R/art); crop(labelbox).save(R/txt)
 x,y,w,h=box
 slots.append({'id':id,'label':label,'rect':dict(x=x,y=y,width=w,height=h),'asset':art,'icon_rect':dict(x=x+(w-16)//2,y=y+(h-16)//2,width=16,height=16),'label_rect':dict(zip(['x','y','width','height'],labelbox)),'label_asset':txt})
crop((112,10,96,11)).save(R/'assets/title-equipment.png')
# Compatibility study only: a directly resized generated slot, NOT a code-drawn substitute.
crop(rows[1][2]).resize((20,20),resample).save(R/'assets/slot-20x20-compat.png')
data={
 'version':'0.1-preview','status':'approved for art handoff; not runtime integrated',
 'canvas':{'width':320,'height':240,'coordinate_origin':'top-left','units':'native pixels','bounds':'x/y inclusive; x+width/y+height exclusive'},
 'rendering':{'scale_filter':'nearest','integer_scaling_only':True,'recommended_icon_size':[16,16]},
 'preview':'previews/equipment-320x240.png','panel':'assets/panel-320x240.png',
 'silhouette':{'asset':'assets/silhouette-54x124.png','rect':{'x':133,'y':75,'width':54,'height':124},'has_transparency':True,'note':'Separately generated clean alpha variant; not pixel-identical to baked preview silhouette.'},
 'title':{'text':'EQUIPMENT','rect':{'x':112,'y':10,'width':96,'height':11},'asset':'assets/title-equipment.png'},
 'slot_count':len(slots),'slots':slots,
 'semantics':{'left_hand':'left-side UI slot; confirm player/anatomical mapping before binding gameplay','right_hand':'right-side UI slot; confirm player/anatomical mapping before binding gameplay','rings':'two distinct accessory slots, not duplicated IDs'},
 'integration_notes':['This layout intentionally contains only seven requested slots.','Existing repo reportedly has eleven slots and a stats panel; do not treat this kit as a same-size drop-in for that layout.','Cropped frames and labels are opaque, not alpha-clean. The panel and silhouette are clean standalone assets.','Full baked preview is the exact visual reference. Modular panel and silhouette are separately generated variants and may differ subtly.','Draw engine bitmap labels over background for localization/readability rather than relying on baked labels.']}
(R/'layout.json').write_text(json.dumps(data,indent=2)+'\n')
assert len(slots)==7 and len({s['id'] for s in slots})==7
for s in slots:
 b=s['rect']; assert 0<=b['x']<b['x']+b['width']<=320 and 0<=b['y']<b['y']+b['height']<=240
 assert Image.open(R/s['asset']).size==(b['width'],b['height'])
assert Image.open(R/'previews/equipment-320x240.png').size==(320,240)
assert Image.open(R/'previews/equipment-preview-4x.png').size==(1280,960)
alpha_min,alpha_max=fig.getchannel('A').getextrema()
assert alpha_min==0 and 0<alpha_max<=255, 'Rebuilt silhouette must have transparent and visible pixels'
report={'native_size':[320,240],'preview_size':[1280,960],'slot_count':7,'slot_ids':[s['id'] for s in slots],'silhouette_size':list(fig.size),'silhouette_alpha_extrema':fig.getchannel('A').getextrema(),'png_count':len(list(R.rglob('*.png'))),'validation':'Dimensions, slot ID count, asset bounds and alpha checked programmatically. Layout/labels checked visually at 4x nearest scale.','limitations':['Not integrated or in-engine tested.','Generated art was normalized to 320x240; not a hand-authored indexed-palette sprite.','Opaque slot/label crops retain a small amount of local background.','Current seven-slot concept differs from repository eleven-slot model.','Transparent silhouette retains partial-alpha boundary pixels and is a matching variant, not exact preview extraction.'],'silhouette_downsample':'BOX; display enlargement NEAREST'}
(R/'QA.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
