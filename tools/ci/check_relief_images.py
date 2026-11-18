"""Check fixed-camera parallax against a repeated no-POM exposure, plus camera travel."""
import argparse
import json
from pathlib import Path
import numpy as np
from PIL import Image


def check(root, rock_only=False):
    report=json.loads((root/'relief-render-audit.json').read_text())
    rows={row['label']:row for row in report['cases']}
    labels=['rock-flat','rock-repeat','rock-pom']
    if not rock_only: labels+=['rock-travel','floor-flat','floor-pom','rock-1k','coast-sunset','snow-line']
    assert list(rows)==labels and report['authored_instances']==0 and report['status']=='passed'
    images={}
    for label,row in rows.items():
        assert row['side']==(4096 if row['bc'] and label not in ('rock-1k','coast-sunset','snow-line') else 1024)
        assert row['image']=='worldcraft-relief-'+label+'.png'
        with Image.open(root/row['image']) as img:
            img.verify()
        with Image.open(root/row['image']) as img:
            pixels=np.asarray(img.convert('RGB'),dtype=np.int16)
        assert pixels.shape[0]>=200 and pixels.shape[1]>=300
        assert float(pixels.std())>8 and np.percentile(pixels,96)-np.percentile(pixels,4)>23, label
        images[label]=pixels
    def delta(a,b):
        left,right=images[a],images[b]
        assert left.shape==right.shape
        h,w=left.shape[:2]
        crop=np.abs(left-right)[int(.46*h):int(.92*h),int(.15*w):int(.74*w)]
        return {'mean_rgb':float(crop.mean()),'fraction_over_4':float((crop.max(axis=2)>4).mean())}
    def same_camera(a,b):
        assert all(rows[a][k]==rows[b][k] for k in ['camera','yaw','pitch','seed','hour'])
    same_camera('rock-repeat','rock-flat')
    same_camera('rock-flat','rock-pom')
    assert rows['rock-flat']['pom_layers']==0 and rows['rock-pom']['pom_layers']==24
    noise=delta('rock-flat','rock-repeat')
    stone=delta('rock-flat','rock-pom')
    assert noise['mean_rgb']<1.0, noise
    assert stone['mean_rgb']>noise['mean_rgb']+0.8 and stone['fraction_over_4']>0.045,stone
    result={'status':'passed','source':report['source'],'roi_xyxy_fractions':[.15,.46,.74,.92],'repeat':noise,'rock':stone}
    if not rock_only:
        same_camera('floor-flat','floor-pom')
        same_camera('rock-pom','rock-1k')
        assert rows['floor-flat']['pom_layers']==0 and rows['floor-pom']['pom_layers']==24
        assert rows['rock-travel']['camera']!=rows['rock-pom']['camera']
        floor=delta('floor-flat','floor-pom')
        assert floor['fraction_over_4']>0.012 and floor['mean_rgb']>0.45,floor
        result['forest_floor']=floor
        res=delta('rock-pom','rock-1k')
        if rows['rock-pom']['bc']: assert res['mean_rgb']>0.35,res
        result['native_4k_to_1k']=res
        assert rows['coast-sunset']['seed']==20261116 and rows['snow-line']['seed']==324
    (root/'relief-image-audit.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rock-only',action='store_true')
    parser.add_argument('--root',type=Path,default=Path('.'))
    args=parser.parse_args();check(args.root,args.rock_only)
