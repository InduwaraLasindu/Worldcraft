"""Prepare embedded CC0 terrain maps on a build machine. Never runs in the product.
Requires Pillow and numpy; pins source URLs, byte counts and SHA-256 in material_sources.json.
"""
import argparse, hashlib, json, pathlib, subprocess, tempfile
import numpy as np
from PIL import Image, __version__ as pillow_version

ROOT=pathlib.Path(__file__).resolve().parents[1]
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def prepare(cache):
    spec=json.loads((ROOT/'tools/material_sources.json').read_text())
    cache.mkdir(parents=True,exist_ok=True)
    sources={}
    for item in spec['sources']:
        target=cache/item['file']
        valid=lambda: target.is_file() and target.stat().st_size==item['size'] and digest(target)==item['sha256']
        if not valid():
            part=target.with_suffix('.part')
            # curl's TLS verification stays enabled. No credentials or APIs in the app.
            subprocess.run(['curl','--fail','--silent','--show-error','--location','--max-time','60',
                '--retry','2','--max-filesize',str(item['size']),'-H','User-Agent: Worldcraft (open source terrain editor)',item['url'],'--output',str(part)],check=True)
            if part.stat().st_size!=item['size'] or digest(part)!=item['sha256']:
                part.unlink(missing_ok=True);raise ValueError('Material source integrity failure: '+item['file'])
            part.replace(target)
        with Image.open(target) as image:
            image=image.convert('RGB').resize((spec['side'],spec['side']),Image.Resampling.LANCZOS)
            sources[item['asset'],item['kind']]=np.asarray(image,dtype=np.float32)/255.0
    color_bytes=bytearray();normal_bytes=bytearray()
    for asset in spec['layers']:
        encoded=sources[asset,'Diffuse']
        color=np.where(encoded<=0.04045,encoded/12.92,((encoded+0.055)/1.055)**2.4)
        normal=sources[asset,'nor_gl']*2.0-1.0
        rough=sources[asset,'Rough'][...,0:1]
        while True:
            enc=np.where(color<=0.0031308,color*12.92,1.055*np.maximum(color,0.0)**(1.0/2.4)-0.055)
            rgba=np.concatenate([enc,np.ones_like(rough)],axis=2)
            color_bytes.extend(np.rint(np.clip(rgba,0,1)*255).astype(np.uint8).tobytes())
            normal=normal/np.maximum(np.linalg.norm(normal,axis=2,keepdims=True),1e-6)
            packed=np.concatenate([normal*0.5+0.5,rough],axis=2)
            normal_bytes.extend(np.rint(np.clip(packed,0,1)*255).astype(np.uint8).tobytes())
            if color.shape[0]==1:break
            def reduce(a):return (a[::2,::2]+a[1::2,::2]+a[::2,1::2]+a[1::2,1::2])*0.25
            color=reduce(color);normal=reduce(normal);rough=reduce(rough)
    out=ROOT/'crates/worldcraft/embedded';out.mkdir(exist_ok=True)
    records=[]
    for name,data in [('terrain-color.bin',color_bytes),('terrain-normal-rough.bin',normal_bytes)]:
        target=out/name
        # Rename only complete outputs; a failed download/build never leaves half a pack.
        part=target.with_suffix('.part');part.write_bytes(data);part.replace(target)
        records.append({'file':name,'size':len(data),'sha256':digest(target)})
    report={'license':spec['license'],'side':spec['side'],'layers':len(spec['layers']),'mips':spec['side'].bit_length(),
        'layout':'layer-major RGBA8, forest, rock, grass; color sRGB, OpenGL normal+roughness linear',
        'pillow':pillow_version,'numpy':np.__version__,'outputs':records}
    (out/'manifest.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache',type=pathlib.Path,default=pathlib.Path(tempfile.gettempdir())/'worldcraft-material-sources')
    prepare(parser.parse_args().cache)
