"""Reject empty/bare native art captures in addition to the shader-log gate."""
from pathlib import Path
import json
import numpy as np
from PIL import Image
for species in ['broadleaf','pine','palm']:
    path=Path('worldcraft-nature-'+species+'.png')
    with Image.open(path) as im:
        image=np.asarray(im.convert('RGB'),dtype=np.int16)
    h,w,_=image.shape
    # The fixture's sky/upper crowns occupy this area. Ignore terrain, status text
    # and UI, which otherwise make a missing-canopy scene appear nonempty.
    roi=image[int(h*0.24):int(h*0.53),int(w*0.20):int(w*0.80)]
    foliage=(roi[:,:,1]>roi[:,:,0]*1.10+3)&(roi[:,:,1]>roi[:,:,2]*1.10+3)&(roi[:,:,1]>20)
    count=int(foliage.sum())
    if count<32:raise AssertionError(f'{species}: only {count} canopy-colored pixels; inspect the native output')
    print(f'{species}: {count} canopy-colored pixels above the terrain horizon')

# Clear daytime clouds must not become a black ceiling. This exposed a volume
# scale/attenuation mismatch that the successful shader log gate could not see.
with Image.open('worldcraft-nature-clouds-high.png') as im:
    sky=np.asarray(im.convert('RGB'),dtype=np.float32)
h,w,_=sky.shape
sky=sky[int(h*0.24):int(h*0.55),int(w*0.12):int(w*0.88)]
luma=sky[:,:,0]*0.2126+sky[:,:,1]*0.7152+sky[:,:,2]*0.0722
dark=float((luma<35).mean())
if dark>0.04:raise AssertionError(f'Daytime cloud layer contains {dark:.1%} near-black sky pixels')
print(f'Daytime sky: {dark:.2%} near-black pixels')

# Require visible cloud contribution, rather than accepting an empty blue sky.
# Both views share the same observer, lighting and zero wind. GPU command counts
# ensure this difference comes from the actual custom rendering path.
with Image.open('worldcraft-nature-clouds-clear.png') as im:
    clear=np.asarray(im.convert('RGB'),dtype=np.float32)
clear=clear[int(h*0.24):int(h*0.55),int(w*0.12):int(w*0.88)]
assert clear.shape==sky.shape
changed=float((np.abs(clear-sky).max(axis=2)>3).mean())
if changed<0.01:raise AssertionError(f'Cloud toggle changed only {changed:.2%} of the sky; inspect the native pass')
report=json.loads(Path('nature-render-audit.json').read_text())
if report.get('cloud_draw_commands',0)<5:raise AssertionError('Cloud draw commands were not observed')
print(f'Cloud toggle: {changed:.2%} of sky visibly changed; {report["cloud_draw_commands"]} draw commands')
