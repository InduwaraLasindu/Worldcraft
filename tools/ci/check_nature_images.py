"""Reject empty/bare native art captures in addition to the shader-log gate."""
from pathlib import Path
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
