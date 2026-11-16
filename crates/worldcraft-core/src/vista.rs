//! Bounded geometric clipmap rings for distant geography.
//! Near edges use the exact streamed field. Only the distant samples use the
//! analytic preview, with a 128 m transition band before the coarser rings.
use crate::{Generator,ChunkCoord,MeshData,CHUNK_SIZE,streaming::lod_for};
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct Bounds{pub x0:f64,pub z0:f64,pub x1:f64,pub z1:f64}
#[derive(Clone,Copy,Debug)]
pub struct Ring{pub inner:Bounds,pub outer:Bounds,pub step:f64}
pub fn rings(center:ChunkCoord,radius:i64,distance:f64)->Vec<Ring>{
    let radius=radius.clamp(2,12);
    let mut inner=Bounds{x0:(center.x-radius) as f64*CHUNK_SIZE,z0:(center.z-radius) as f64*CHUNK_SIZE,
        x1:(center.x+radius+1) as f64*CHUNK_SIZE,z1:(center.z+radius+1) as f64*CHUNK_SIZE};
    let edge=ChunkCoord{x:center.x+radius+1,z:center.z};
    let mut step=4.0*f64::from(1u32<<lod_for(edge,center));
    let mut out=Vec::new();
    let distance=distance.clamp(2048.0,65536.0);
    for level in 0..12 {
        let margin=if level==0 {128.0}else{step*16.0};
        let align=step*2.0;
        let outer=Bounds{x0:((inner.x0-margin)/align).floor()*align,
            z0:((inner.z0-margin)/align).floor()*align,
            x1:((inner.x1+margin)/align).ceil()*align,z1:((inner.z1+margin)/align).ceil()*align};
        out.push(Ring{inner,outer,step});
        if (outer.x1-outer.x0).min(outer.z1-outer.z0)>=distance*2.0 {break;}
        inner=outer;step*=2.0;
    }
    out
}
#[derive(Clone,Copy)]struct Vertex{h:f64,n:[f32;3],c:[f32;4]}
fn approx(g:&Generator,x:f64,z:f64)->Vertex{
    let s=g.preview_sample(x,z);
    let dx=(g.preview_sample(x+2.0,z).height-g.preview_sample(x-2.0,z).height)*0.25;
    let dz=(g.preview_sample(x,z+2.0).height-g.preview_sample(x,z-2.0).height)*0.25;
    let len=(dx*dx+dz*dz+1.0).sqrt();
    let r=((dx.hypot(dz)-0.5)/0.7).clamp(0.0,1.0) as f32;
    let base=s.biome.color();let rock=[0.29,0.27,0.23];
    let mut c=[1.0;4];for i in 0..3 {c[i]=base[i]*(1.0-r)+rock[i]*r;}
    Vertex{h:s.height,n:[(-dx/len) as f32,(1.0/len) as f32,(-dz/len) as f32],c}
}
fn lerp(a:Vertex,b:Vertex,t:f64)->Vertex{
    let mut n=[0.0;3];let mut c=[0.0;4];
    for i in 0..3 {n[i]=a.n[i]*(1.0-t as f32)+b.n[i]*t as f32;}
    for i in 0..4 {c[i]=a.c[i]*(1.0-t as f32)+b.c[i]*t as f32;}
    Vertex{h:a.h*(1.0-t)+b.h*t,n,c}
}
fn field(g:&Generator,x:f64,z:f64,near:Option<Bounds>)->Vertex{
    let mut v=approx(g,x,z);
    if let Some(b)=near {
        let distance=(b.x0-x).max(x-b.x1).max(b.z0-z).max(z-b.z1).max(0.0);
        let t=(distance/128.0).clamp(0.0,1.0);
        if t<1.0 {v=lerp(Vertex{h:g.height(x,z),n:g.normal(x,z),c:g.color(x,z)},v,t);}
    }
    v
}
pub fn build_vista(g:&Generator,center:ChunkCoord,radius:i64,distance:f64)->MeshData{
    let mut mesh=MeshData::default();let origin=center.origin();
    for (level,ring) in rings(center,radius,distance).iter().enumerate(){
        let b=ring.outer;let s=ring.step;
        let nx=((b.x1-b.x0)/s).round() as usize;
        let nz=((b.z1-b.z0)/s).round() as usize;
        let mut grid=vec![None;(nx+1)*(nz+1)];
        for z in 0..nz {for x in 0..nx {
            let wx=b.x0+x as f64*s;let wz=b.z0+z as f64*s;
            if wx>=ring.inner.x0&&wz>=ring.inner.z0&&wx+s<=ring.inner.x1&&wz+s<=ring.inner.z1 {continue;}
            let mut ids=[0;4];
            for (corner,(dx,dz)) in [(0,0),(1,0),(0,1),(1,1)].into_iter().enumerate(){
                let ix=x+dx;let iz=z+dz;let slot=iz*(nx+1)+ix;
                ids[corner]=*grid[slot].get_or_insert_with(||{
                    let px=b.x0+ix as f64*s;let pz=b.z0+iz as f64*s;
                    let near=(level==0).then_some(ring.inner);
                    let mut v=field(g,px,pz,near);
                    // The denser outer edge follows the next ring's polyline.
                    if iz==0||iz==nz||ix==0||ix==nx {
                        let horizontal=iz==0||iz==nz;let along=if horizontal{px}else{pz};
                        let coarse=s*2.0;let a=(along/coarse).floor()*coarse;
                        let t=(along-a)/coarse;
                        if t>0.0 {
                            let sample=|at|if horizontal{field(g,at,pz,near)}else{field(g,px,at,near)};
                            v=lerp(sample(a),sample(a+coarse),t);
                        }
                    }
                    mesh.vertex([(px-origin[0]) as f32,v.h as f32,(pz-origin[1]) as f32],v.n,v.c,
                        [(px-origin[0]) as f32/8.0,(pz-origin[1]) as f32/8.0])
                });
            }
            let [a,b,c,d]=ids;mesh.indices.extend_from_slice(&[a,c,b,b,c,d]);
        }}
    }
    mesh
}
#[cfg(test)]mod tests{
    use super::*;use crate::{GeneratorSettings,ChunkKey,build_mesh,streaming::edge_lods};
    #[test]fn clipmap_rings_cover_the_annulus_without_overlaps_or_gaps(){
        for center in [ChunkCoord{x:0,z:0},ChunkCoord{x:-7,z:5}]{
            for radius in [2,4,8,12]{
                let rs=rings(center,radius,32768.0);
                assert!(rs.len()<12);
                for pair in rs.windows(2){assert_eq!(pair[0].outer,pair[1].inner);assert_eq!(pair[0].step*2.0,pair[1].step);}
                for r in rs{assert!(r.outer.x0<r.inner.x0&&r.outer.x1>r.inner.x1);
                    for a in [r.inner.x0,r.inner.x1,r.outer.x0,r.outer.x1]{assert_eq!(a.rem_euclid(r.step),0.0);}}
            }
        }
    }
    #[test]fn distant_mesh_is_finite_indexed_and_bounded(){
        let g=Generator::new(GeneratorSettings::default(),vec![]);
        let m=build_vista(&g,ChunkCoord{x:-1,z:1},3,16384.0);
        assert!(m.positions.len()<100_000);
        assert!(m.indices.len()<600_000);
        assert!(m.positions.iter().flatten().all(|v|v.is_finite()));
        assert!(m.indices.iter().all(|&i|(i as usize)<m.positions.len()));
        for tri in m.indices.chunks_exact(3){
            let a=m.positions[tri[0] as usize];let b=m.positions[tri[1] as usize];let c=m.positions[tri[2] as usize];
            assert!((b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])>0.0);
        }
    }
    #[test]fn the_near_border_matches_streamed_terrain_at_negative_coordinates(){
        let g=Generator::new(GeneratorSettings::default(),vec![]);
        let center=ChunkCoord{x:-3,z:-2};let radius=2;
        let far=build_vista(&g,center,radius,2048.0);
        let coord=ChunkCoord{x:center.x-radius,z:center.z};
        let key=ChunkKey{coord,lod:lod_for(coord,center),edges:edge_lods(coord,center),revision:0};
        let near=build_mesh(&g,key);let o=center.origin();let n=coord.origin();
        let step=rings(center,radius,2048.0)[0].step;
        for z in 0..=16{
            let world_z=n[1]+z as f64*step;
            let expected=g.height(n[0],world_z) as f32;
            let found=far.positions.iter().find(|p|p[0]==(n[0]-o[0]) as f32&&p[2]==(world_z-o[1]) as f32).unwrap();
            assert!((found[1]-expected).abs()<0.0001);
            let index=((z as f64*step/4.0) as usize)*33;
            assert!((near.positions[index][1]-found[1]).abs()<0.0001);
        }
    }
}
