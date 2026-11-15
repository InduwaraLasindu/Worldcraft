use bevy::{prelude::*, render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology}};
use worldcraft_core::{generation::unit, mesh::MeshData, scatter::{Instance, Species}};

#[derive(Resource)]
pub struct Nature {
    pub trunks: Vec<Handle<Mesh>>,
    pub crowns: Vec<Handle<Mesh>>,
    pub pines: Vec<Handle<Mesh>>,
    pub palms: Vec<Handle<Mesh>>,
    pub rocks: Vec<Handle<Mesh>>,
    pub low_crown: Handle<Mesh>,
    pub bark: Handle<StandardMaterial>,
    pub leaf: Handle<StandardMaterial>,
    pub stone: Handle<StandardMaterial>,
    pub ground: Handle<StandardMaterial>,
    pub terrain: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
}
pub fn mesh(m: MeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.colors)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uv)
        .with_inserted_indices(Indices::U32(m.indices))
}
pub fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut trunks = Vec::new(); let mut crowns = Vec::new();
    let mut pines = Vec::new(); let mut palms = Vec::new(); let mut rocks = Vec::new();
    for variant in 0..4 {
        let seed = 999 + variant;
        let mut wood = MeshData::default();
        tube(&mut wood, Vec3::ZERO, Vec3::new(0.15,9.0,0.0),0.31,0.08,9,[0.3,0.21,0.12,1.0]);
        let mut leaves = MeshData::default();
        for branch in 0..10 {
            let angle = branch as f32 * 2.4 + variant as f32;
            let h = 3.6 + branch as f32 * 0.45;
            let tip = Vec3::new(angle.cos() * 2.6,h + 1.5,angle.sin() * 2.6);
            tube(&mut wood,Vec3::new(0.0,h,0.0),tip,0.1,0.025,6,[0.25,0.18,0.1,1.0]);
            for leaf in 0..36 {
                let a = unit(seed,branch,leaf) as f32 * std::f32::consts::TAU;
                let r = (unit(seed ^ 7,branch,leaf) as f32).sqrt()*1.9;
                let p = tip + Vec3::new(a.cos()*r,unit(seed ^ 8,branch,leaf) as f32*2.1-0.7,a.sin()*r);
                leaf_card(&mut leaves,p,0.38+unit(seed ^ 9,branch,leaf) as f32*0.35,a);
            }
        }
        trunks.push(meshes.add(mesh(wood)));
        crowns.push(meshes.add(mesh(leaves)));
        let mut pine = MeshData::default();
        for layer in 0..8 {
            cone(&mut pine,Vec3::new(0.0,2.4+layer as f32*0.9,0.0),
                2.8-layer as f32*0.29,2.4,12,[0.08,0.19+variant as f32*0.01,0.075,1.0]);
        }
        pines.push(meshes.add(mesh(pine)));
        let mut palm = MeshData::default();
        for branch in 0..13 {
            let a = branch as f32 * std::f32::consts::TAU / 13.0;
            for leaf in 0..9 {
                let t = leaf as f32 / 8.0;
                leaf_card(&mut palm,Vec3::new(a.cos()*t*4.0,8.5+t*1.8-t*t*2.8,a.sin()*t*4.0),0.55,a);
            }
        }
        palms.push(meshes.add(mesh(palm)));
        let rock = Sphere::new(1.0).mesh().ico(1).unwrap();
        rocks.push(meshes.add(rock));
    }
    let mut low = MeshData::default();
    cone(&mut low,Vec3::new(0.0,3.5,0.0),3.0,6.0,8,[0.12,0.23,0.08,1.0]);
    commands.insert_resource(Nature {
        trunks,crowns,pines,palms,rocks,low_crown: meshes.add(mesh(low)),
        bark: materials.add(StandardMaterial { base_color: Color::srgb(0.5,0.4,0.29),perceptual_roughness:0.95,..default() }),
        leaf: materials.add(StandardMaterial { base_color: Color::srgb(0.31,0.52,0.17),
            perceptual_roughness:0.85,cull_mode:None,double_sided:true,diffuse_transmission:0.25,..default() }),
        stone: materials.add(StandardMaterial { base_color: Color::srgb(0.35,0.33,0.28),perceptual_roughness:0.94,..default() }),
        ground: materials.add(StandardMaterial { base_color: Color::WHITE,perceptual_roughness:0.95,cull_mode:None,double_sided:true,..default() }),
        terrain: materials.add(StandardMaterial { base_color: Color::WHITE,perceptual_roughness:0.92,..default() }),
        water: materials.add(StandardMaterial { base_color: Color::srgba(0.055,0.27,0.32,0.72),
            perceptual_roughness:0.16,metallic:0.12,reflectance:0.65,alpha_mode:AlphaMode::Blend,cull_mode:None,..default() }),
    });
}
fn tube(m: &mut MeshData, a: Vec3, b: Vec3, ra: f32, rb: f32, sides: usize, color: [f32;4]) {
    let direction = (b-a).normalize();
    let u = direction.cross(if direction.dot(Vec3::Y).abs() < 0.99 { Vec3::Y } else { Vec3::X }).normalize();
    let v = direction.cross(u);
    let start = m.positions.len() as u32;
    for (p,r) in [(a,ra),(b,rb)] { for i in 0..sides {
        let angle = i as f32 / sides as f32 * std::f32::consts::TAU;
        let n = u*angle.cos()+v*angle.sin();
        m.vertex((p+n*r).to_array(),n.to_array(),color,[i as f32/sides as f32,p.y]);
    }}
    for i in 0..sides {
        let j = (i+1)%sides;
        let (a,b,c,d)=(start+i as u32,start+j as u32,start+sides as u32+i as u32,start+sides as u32+j as u32);
        m.indices.extend_from_slice(&[a,b,c,b,d,c]);
    }
}
fn cone(m: &mut MeshData, p: Vec3, radius:f32, height:f32, sides:usize, color:[f32;4]) {
    tube(m,p,p+Vec3::Y*height,radius,0.02,sides,color);
}
fn leaf_card(m:&mut MeshData,p:Vec3,size:f32,angle:f32) {
    let u=Vec3::new(angle.cos(),0.25,angle.sin())*size;
    let v=Vec3::new(-angle.sin()*0.5,0.5,angle.cos()*0.5)*size;
    let n=u.cross(v).normalize();
    let a=m.vertex((p-u).to_array(),n.to_array(),[0.8,1.0,0.65,1.0],[0.0,0.5]);
    let b=m.vertex((p+v).to_array(),n.to_array(),[0.9,1.0,0.7,1.0],[0.5,0.0]);
    let c=m.vertex((p+u).to_array(),n.to_array(),[0.7,0.9,0.6,1.0],[1.0,0.5]);
    let d=m.vertex((p-v).to_array(),n.to_array(),[0.7,0.9,0.6,1.0],[0.5,1.0]);
    m.indices.extend_from_slice(&[a,b,c,a,c,d]);
}
pub fn ground_mesh(instances:&[Instance],origin:[f64;2])->MeshData {
    let mut m=MeshData::default();
    for i in instances {
        let p=Vec3::new((i.position[0]-origin[0]) as f32,i.position[1] as f32,(i.position[2]-origin[1]) as f32);
        match i.species {
            Species::Grass => for blade in 0..7 {
                let angle=i.yaw+blade as f32*2.4;
                let u=Vec3::new(angle.cos(),0.0,angle.sin());
                let base=p+u*(blade as f32*0.05);
                let height=i.scale*(0.45+blade as f32*0.07);
                let n=Vec3::new(-u.z,0.3,u.x).normalize();
                let a=m.vertex((base-u*0.06).to_array(),n.to_array(),[0.08,0.17,0.035,1.0],[0.0,0.0]);
                let b=m.vertex((base+u*0.06).to_array(),n.to_array(),[0.08,0.17,0.035,1.0],[1.0,0.0]);
                let c=m.vertex((base+Vec3::Y*height+u*0.18).to_array(),n.to_array(),[0.28,0.4,0.09,1.0],[0.5,1.0]);
                m.indices.extend_from_slice(&[a,b,c]);
            },
            Species::Shrub => for leaf in 0..14 {
                let a=i.yaw+leaf as f32*2.4;
                leaf_card(&mut m,p+Vec3::new(a.cos()*0.5,0.35+(leaf%3) as f32*0.2,a.sin()*0.5)*i.scale,0.35*i.scale,a);
            },
            _=>{}
        }
    }
    m
}
