"""Bake GPU-ready 4K nature and ground mip chains. Build-time dependencies only."""
import io
import math
import random
import struct
import zlib
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

import skyline_atlas
from relief_materials import lattice

SIDE = 4096
NAMES = ('broadleaf', 'birch', 'acacia', 'pine', 'palm', 'grass', 'skyline',
         'bark', 'bark-normal', 'birch-bark', 'birch-normal', 'sediment')


def write_map(path, image, cutout=False, cells=1):
    """Independent deflate streams allow the engine to skip unused mip levels."""
    side = image.width
    levels = side.bit_length() - (2 if cells == 4 else 0)
    sizes = []
    tmp = path.with_suffix('.part')
    with tmp.open('wb') as out:
        out.write(bytes(levels*4))
        reference = image.getchannel('A')
        for i in range(levels):
            size = side >> i
            level = image if not i else image.resize((size,size), Image.Resampling.BOX)
            if i and cutout and size >= cells*4:
                # Preserve coverage per tile, so sparse sprays survive at distance.
                for row in range(cells):
                    for col in range(cells):
                        def box(n):
                            t = n//cells
                            return (col*t,row*t,(col+1)*t,(row+1)*t)
                        hist = reference.crop(box(side)).histogram()
                        target = sum(hist[128:])/sum(hist)
                        tile = level.crop(box(size))
                        alpha = tile.getchannel('A')
                        counts = np.cumsum(alpha.histogram())
                        k = np.searchsorted(counts, (1-target)*counts[-1], side='right')
                        if 0 < k < 255:
                            scale = min(3.5, 128/k)
                            tile.putalpha(alpha.point([min(255,round(x*scale)) if x<255 else 255 for x in range(256)]))
                            level.paste(tile, box(size))
            buf = io.BytesIO()
            level.save(buf,format='DDS',pixel_format='DXT5')
            payload = buf.getbuffer()[128:]
            assert len(payload)==16*((size+3)//4)**2
            packed = zlib.compress(payload, 2)
            sizes.append(len(packed))
            out.write(packed)
        out.seek(0)
        out.write(struct.pack('<'+'I'*levels,*sizes))
    tmp.replace(path)


def vegetation(kind, side=SIDE):
    """Rasterize individual blades and twigs at final resolution."""
    rng = random.Random(879 + NAMES.index(kind))
    image = Image.new('RGBA',(side,side))
    d = ImageDraw.Draw(image)
    def points(p):
        return [(round(x*side),round(y*side)) for x,y in p]
    def line(p, color, width):
        d.line(points(p),fill=(*color,255),width=max(1,round(width*side)),joint='curve')
    def tinted(col, v):
        return tuple(max(0,min(255,round(c*v))) for c in col)
    def blade(a,b,width,color,serrated=False):
        x,y=a; dx=b[0]-x; dy=b[1]-y
        length=math.hypot(dx,dy); nx,ny=-dy/length,dx/length
        bend=rng.uniform(-.45,.45)*width
        def pos(t,q):
            w=math.sin(math.pi*t)**.76
            tooth=1-.085*abs(math.sin(13*math.pi*t)) if serrated else 1
            off=w*(bend+q*width*tooth)
            return (x+dx*t+nx*off,y+dy*t+ny*off)
        ts=[i/40 for i in range(41)]
        mid=[pos(t,0) for t in ts]
        for sign,lux in [(-1,.90),(1,1.05)]:
            d.polygon(points(mid+[pos(t,sign) for t in reversed(ts)]),fill=(*tinted(color,lux),255))
        line(mid,tinted(color,1.19),width*.075)
        if serrated:
            for j in range(1,9):
                t=j/11
                for sign in [-1,1]:
                    line([pos(t,0),pos(t+.035,sign*.52),pos(t+.095,sign*.94)],tinted(color,1.09),width*.029)
    if kind=='grass':
        tw=196/512
        for h in range(4):
            ox=(h%2)*tw; oy=(h//2)*.5
            def uv(x,y):return ox+x*tw,oy+y*.5
            count=(48,35,52,32)[h]
            for i in range(count):
                fan=rng.uniform(-1,1)
                root=(.18,.39,.60,.80)[i%4]+fan*.072
                reach=fan*(.14,.16,.18,.12)[h]
                height=rng.uniform(.51,.96)*(1-.15*abs(fan))
                width=rng.uniform(.008,.019)*(1.4 if h==3 else 1)
                color=tinted((221,225,193),rng.uniform(.78,1.04))
                ts=np.linspace(0,1,23)
                def edge(t,sgn):
                    return uv(root+reach*t*t+sgn*width*(1-t)**.72,height*t)
                poly=[edge(t,-1) for t in ts]+[edge(t,1) for t in reversed(ts)]
                d.polygon(points(poly),fill=(*color,255))
                line([edge(t,0) for t in ts],tinted(color,1.12),.00027)
                if h==1 and i%4==0:
                    for j in range(6):
                        t=.70+j*.045
                        for sign in [-1,1]:
                            blade(edge(t,0),uv(root+reach*t*t+sign*.022,height*(t+.05)),.0013,(212,192,140))
        d.rectangle((math.ceil(side*.80),0,side-1,side-1),fill=(255,255,255,255))
    else:
        origin=(.49,.94); tip=(.51,.065)
        line([origin,(.474,.53),tip],(83,88,42),.010)
        count={'pine':16,'palm':15,'birch':9,'acacia':11,'broadleaf':9}[kind]
        for j in range(count):
            t=(j+.7)/(count+1)
            y=.91-.83*t
            for sign in [-1,1]:
                x=.49+.019*t
                reach=(.41 if kind=='palm' else .36)*math.sin((t*.73+.13)*math.pi)
                end=(x+sign*reach,y-.06-.13*t)
                if kind=='palm':
                    blade((x,y),end,.015+.012*(1-t),tinted((86,117,48),rng.uniform(.82,1.13)))
                    continue
                line([(x,y),end],(87,98,45),.0037)
                n={'pine':18,'acacia':9,'birch':4,'broadleaf':4}[kind]
                for k in range(n):
                    u=(k+.6)/(n+.3)
                    bx=x+(end[0]-x)*u; by=y+(end[1]-y)*u
                    for flank in [-1,1]:
                        l={'pine':.066,'acacia':.049,'birch':.086,'broadleaf':.102}[kind]*rng.uniform(.78,1.15)*(1-.24*u)
                        ex=bx+sign*l*(.42+.17*u)
                        ey=by+l*(flank*.72-.34)
                        w=l*{'pine':.042,'acacia':.15,'birch':.38,'broadleaf':.29}[kind]
                        col={'pine':(74,104,66),'acacia':(98,119,59),'birch':(104,141,55),'broadleaf':(90,121,48)}[kind]
                        blade((bx,by),(ex,ey),w,tinted(col,rng.uniform(.78,1.17)),kind in ('birch','broadleaf'))
    return image


def bark(birch, side=SIDE):
    pigment=np.empty((side,side,4),np.uint8)
    height=np.empty((side,side),np.float32)
    x=(np.arange(side,dtype=np.float32)[None,:]+.5)/side
    for i in range(0,side,64):
        y=(np.arange(i,min(i+64,side),dtype=np.float32)[:,None]+.5)/side
        warp=lattice(x,y,8,605)
        u=x+.025*warp
        plate=lattice(u,y,48,241,12)
        # Integer anisotropic frequencies wrap at both edges.
        fibers=lattice(u,y,392,437,98)
        fine=lattice(u,y,1332,639,333)
        fleck=lattice(x,y,631,325)
        if birch:
            scar=np.clip((lattice(x,y,41,283,164)-.42)*3.5,0,1)
            peel=np.clip((lattice(x,y,96,144,384)-.45)*2.6,0,1)
            h=.48+.15*plate+.09*fibers-.31*scar+.15*peel+.035*fine
            base=(202,196,167); val=.87+.07*plate+.08*fibers-.66*scar-.27*peel+.04*fleck
        else:
            fissure=np.clip(abs(plate+.11*fibers)*4.1,0,1)
            h=.13+.48*fissure+.16*fibers+.075*fine+.045*fleck
            base=(123,104,80); val=.45+.45*fissure+.16*fibers+.11*fine+.08*fleck
        height[i:i+64]=h
        pigment[i:i+64,:,:3]=np.clip(val[...,None]*base,0,255)
        pigment[i:i+64,:,3]=255
    normal=np.empty_like(pigment)
    for i in range(0,side,64):
        rows=np.arange(i,min(i+64,side))
        return_normal(normal[i:i+64], (np.roll(height[rows],1,1)-np.roll(height[rows],-1,1))*.95,
                      (height[(rows-1)%side]-height[(rows+1)%side])*.95, 255)
    return Image.fromarray(pigment),Image.fromarray(normal)


def return_normal(out, nx, ny, alpha):
    z=127.5/np.sqrt(1+nx*nx+ny*ny)
    out[:,:,0]=np.rint(127.5+nx*z);out[:,:,1]=np.rint(127.5+ny*z)
    out[:,:,2]=np.rint(127.5+z);out[:,:,3]=alpha


def ground(out, side=SIDE):
    # Decode the baked native 4K relief height, keeping normals registered to POM.
    full=Path(out,'procedural-height-bc4.bin').read_bytes()
    def raw(name, layer):
        with (out/name).open('rb') as f:
            f.seek(layer*sum(4*(1024>>m)**2 for m in range(11)))
            return Image.frombytes('RGBA',(1024,1024),f.read(4*1024**2)).resize((side,side),Image.Resampling.BILINEAR)
    count=sum(8*((max(1,4096>>m)+3)//4)**2 for m in range(13))
    for layer in range(4):
        sediment=layer==3
        if not sediment:
            start=layer*count
            h=np.asarray(Image.frombytes('L',(4096,4096),full[start:start+8*1024**2],'bcn',4))
        for proc in ([True] if sediment else [False,True]):
            src=raw('shore-sediment.bin' if sediment else ('procedural-normal-rough.bin' if proc else 'terrain-normal-rough.bin'),0 if sediment else layer)
            normal=np.asarray(src).copy()
            color=None if proc else np.asarray(raw('terrain-color.bin',layer)).copy()
            x=(np.arange(side,dtype=np.float32)[None,:]+.5)/side
            for i in range(0,side,64):
                rows=np.arange(i,min(i+64,side))
                y=((np.arange(i-1,i+len(rows)+1)%side).astype(np.float32)[:,None]+.5)/side
                strip=lattice(x,y,1417,681+layer)
                micro=strip[1:-1]
                ny=(strip[2:]-strip[:-2])*.5
                nx=(np.roll(micro,-1,axis=1)-np.roll(micro,1,axis=1))*.5
                a=normal[rows]
                den=np.maximum(a[:,:,2].astype(np.float32)-127.5,15)
                gx=(a[:,:,0].astype(np.float32)-127.5)/den
                gy=(a[:,:,1].astype(np.float32)-127.5)/den
                if proc and not sediment:
                    slab=h[rows].astype(np.float32)
                    gx+=(np.roll(slab,1,1)-np.roll(slab,-1,1))*.010
                    gy+=(h[(rows-1)%side].astype(np.float32)-h[(rows+1)%side].astype(np.float32))*.010
                return_normal(normal[i:i+64],gx-.65*nx,gy-.65*ny,np.clip(a[:,:,3]+micro*(25 if sediment else 8),0,255))
                if color is not None:
                    color[i:i+64,:,:3]=np.clip(color[rows,:,:3]*(1+.115*micro[...,None]),0,255)
            name='sediment' if sediment else ('proc' if proc else 'scan')+'-normal-'+str(layer)
            write_map(out/(name+'.wct'),Image.fromarray(normal))
            if color is not None: write_map(out/('scan-color-'+str(layer)+'.wct'),Image.fromarray(color))


def bake(out):
    for name in NAMES[:6]:
        write_map(out/(name+'.wct'), vegetation(name), True, 2 if name=='grass' else 1)
    tiled=Image.new('RGBA',(SIDE,SIDE))
    for t in range(16):
        family,variant=divmod(t,3) if t!=15 else (1,3)
        tile=skyline_atlas.profile(family,variant,tile=SIDE//4)
        tiled.paste(tile,(t%4*tile.width,t//4*tile.height))
    write_map(out/'skyline.wct',tiled,True,4)
    for birch,names in [(False,NAMES[7:9]),(True,NAMES[9:11])]:
        for name,image in zip(names,bark(birch)):
            write_map(out/(name+'.wct'),image)
    ground(out)


if __name__=='__main__':
    bake(Path(__file__).resolve().parents[1]/'crates/worldcraft/embedded')
