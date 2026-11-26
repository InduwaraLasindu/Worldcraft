"""Bake native 4K dielectric surface maps, block compressed for the desktop GPU."""
from pathlib import Path
import hashlib
import numpy as np
from procedural_materials import surfaces

SIDE = 4096
STRIP = 64
SEEDS = (19013, 47353, 72821)


def lattice(x, y, cells, seed):
    """Periodic smooth hash noise evaluated at output pixel centers."""
    u, v = x * cells, y * cells
    a, b = np.floor(u), np.floor(v)
    u, v = u - a, v - b
    u, v = u*u*(3-2*u), v*v*(3-2*v)
    @np.errstate(over="ignore")  # Unsigned wraparound is integral to the hash.
    def corner(i, j):
        h = ((a.astype(np.int32)+i) % cells).astype(np.uint32)*np.uint32(0x9e3779b1)
        h = h ^ (((b.astype(np.int32)+j) % cells).astype(np.uint32)*np.uint32(0x85ebca77)) ^ np.uint32(seed)
        h = (h ^ (h >> 16))*np.uint32(0x7feb352d)
        h = (h ^ (h >> 15))*np.uint32(0x846ca68b)
        return ((h ^ (h >> 16)) >> 8).astype(np.float32)*(2/16777216) - 1
    return (1-v)*((1-u)*corner(0,0)+u*corner(1,0))+v*((1-u)*corner(0,1)+u*corner(1,1))


def resample(a, x, y):
    side = len(a)
    u, v = x*side-0.5, y*side-0.5
    i, j = np.floor(u).astype(np.int32), np.floor(v).astype(np.int32)
    fx, fy = (u-i).astype(np.float32)[...,None], (v-j).astype(np.float32)[...,None]
    top = a[j%side,i%side]*(1-fx)+a[j%side,(i+1)%side]*fx
    bot = a[(j+1)%side,i%side]*(1-fx)+a[(j+1)%side,(i+1)%side]*fx
    return top*(1-fy)+bot*fy


def srgb(linear):
    return np.where(linear <= .0031308, 12.92*linear, 1.055*np.maximum(0,linear)**(1/2.4)-.055)


def byte(unit):
    return np.rint(np.clip(unit, 0, 1)*255).astype(np.uint8)


def detail(layer, macro, side):
    """Keep visible leaves and fractures registered across color, roughness and height.
    New sub-centimeter crystals, soil crumbs and grass ribs are synthesized at 4K.
    Only a horizontal strip of floating-point intermediates is resident.
    """
    out = np.empty((side, side, 5), dtype=np.uint8)
    x = (np.arange(side,dtype=np.float32)[None,:]+.5)/side
    seed = SEEDS[layer]
    for start in range(0,side,STRIP):
        end = min(side,start+STRIP)
        y = (np.arange(start,end,dtype=np.float32)[:,None]+.5)/side
        base = resample(macro,x,y)
        grain = lattice(x,y,887,seed)
        pore = lattice(x,y,379,seed+1)
        drift = lattice(x,y,43,seed+2)
        if layer == 1:
            # Interrupted quartz seams with gritty facets and small pits in the matrix.
            vein = np.maximum(0,1-abs(lattice(x+.0018*drift,y,61,seed+3))*9)**2
            crystal = np.maximum(0,(grain-.30)*2)*(.4+.6*vein)
            micro = .065*pore+.030*grain+.075*vein+.026*crystal
            pigment = .75+.15*pore+.12*grain+.32*vein+.48*crystal
            rough = base[:,:,3]-.15*vein-.14*crystal+.04*pore
            height = .48+.071*base[:,:,4]+micro
        elif layer == 0:
            crumbs = np.maximum(0,pore+.16)**1.7
            micro = .071*crumbs+.025*grain
            pigment = .86+.15*grain+.32*crumbs+.055*drift
            rough = base[:,:,3]+.045*pore-.035*crumbs
            height = .32+.079*base[:,:,4]+micro
        else:
            rib = np.cos(2*np.pi*(x*277+y*513+.82*drift))
            fibre = np.maximum(0,rib)*(.48+.45*pore)
            pigment = .88+.13*grain+.20*fibre+.07*pore
            rough = base[:,:,3]-.062*fibre+.034*pore
            height = .32+.070*base[:,:,4]+.055*fibre+.029*pore+.017*grain
        out[start:end,:,:3] = byte(srgb(base[:,:,:3]*pigment[...,None]))
        out[start:end,:,3] = byte(rough)
        out[start:end,:,4] = byte(height)
    return out


def blocks(strip):
    h,w,c = strip.shape
    if h<4 or w<4:
        strip=np.pad(strip,((0,max(0,4-h)),(0,max(0,4-w)),(0,0)),mode='edge')
    return strip.reshape(strip.shape[0]//4,4,strip.shape[1]//4,4,c).transpose(0,2,1,3,4).reshape(-1,16,c)


def indices(bits, count, byte_count):
    packed = np.bitwise_or.reduce(bits.astype(np.uint64) << (np.arange(16,dtype=np.uint64)*count),axis=1)
    return ((packed[:,None] >> (np.arange(byte_count,dtype=np.uint64)*8)) & 255).astype(np.uint8)


def bc4(v):
    hi,lo = v.max(1),v.min(1)
    span = hi.astype(np.int32)-lo
    rank = np.rint((hi[:,None].astype(np.int32)-v)*7/np.maximum(span[:,None],1)).astype(np.uint8)
    index = np.where(rank==0,0,np.where(rank==7,1,rank+1))
    return np.column_stack((hi,lo,indices(index,3,6)))


def bc1(rgb):
    rgb=rgb.astype(np.int32)
    # Luminance endpoints preserve hues within these mostly dielectric blocks.
    y=rgb[:,:,0]*3+rgb[:,:,1]*6+rgb[:,:,2]
    rows=np.arange(len(rgb))
    ends=rgb[rows[:,None],np.column_stack((y.argmax(1),y.argmin(1)))]
    r,g,b=(ends*np.array([31,63,31])+127).transpose(2,0,1)//255
    words=(r<<11)|(g<<5)|b
    hi,lo=words.max(1),words.min(1)
    def decoded(q):
        return np.column_stack((((q>>11)*255+15)//31,(((q>>5)&63)*255+31)//63,((q&31)*255+15)//31))
    c0,c1=decoded(hi),decoded(lo)
    d=c1-c0
    t=np.sum((rgb-c0[:,None])*d[:,None],axis=2)/np.maximum(1,np.sum(d*d,axis=1))[:,None]
    selector=np.array([0,2,3,1],dtype=np.uint8)[np.rint(np.clip(t,0,1)*3).astype(int)]
    return np.column_stack((hi&255,hi>>8,lo&255,lo>>8,indices(selector,2,4))).astype(np.uint8)


def downsample(a, linear_lut):
    # Decode sRGB before averaging. Work in strips to bound temporary memory.
    side=len(a)//2
    dst=np.empty((side,side,5),np.uint8)
    for y in range(0,side,STRIP):
        span=a[2*y:2*min(y+STRIP,side)].reshape(-1,2,side,2,5)
        dst[y:y+STRIP,:,:3]=byte(srgb(linear_lut[span[:,:,:,:,:3]].mean(axis=(1,3))))
        dst[y:y+STRIP,:,3:]=np.rint(span[:,:,:,:,3:].mean(axis=(1,3))).astype(np.uint8)
    return dst


def bake(out, side=SIDE):
    paths=[Path(out)/('procedural-'+s+'.bin') for s in ('pbr-bc3','height-bc4')]
    tmp=[p.with_suffix('.part') for p in paths]
    s=np.arange(256,dtype=np.float32)/255
    lut=np.where(s<=.04045,s/12.92,((s+.055)/1.055)**2.4)
    try:
        with tmp[0].open('wb') as colors, tmp[1].open('wb') as heights:
            for i,(color,_,rough,height) in enumerate(surfaces(1024)):
                macro=np.concatenate((color,rough,height[...,None]),axis=2)
                level=detail(i,macro,side)
                while True:
                    for start in range(0,len(level),STRIP):
                        b=blocks(level[start:start+STRIP])
                        colors.write(np.column_stack((bc4(b[:,:,3]),bc1(b[:,:,:3]))).tobytes())
                        heights.write(bc4(b[:,:,4]).tobytes())
                    if len(level)==1: break
                    level=downsample(level,lut)
        for t,p in zip(tmp,paths): t.replace(p)
    finally:
        for t in tmp: t.unlink(missing_ok=True)
    records=[]
    for p in paths:
        with p.open('rb') as f: digest=hashlib.file_digest(f,'sha256').hexdigest()
        records.append(dict(file=p.name,size=p.stat().st_size,sha256=digest))
    return records
