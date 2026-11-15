use crate::{
    assets::{self, Nature},
    session::*,
};
use bevy::{
    math::DVec3,
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use std::{collections::HashMap, sync::Arc, time::Instant};
use worldcraft_core::{
    scatter::{scatter, Instance, Species},
    streaming::{edge_lods, lod_for, wanted},
    *,
};

#[derive(Component)]
pub struct ChunkRoot(pub ChunkCoord);
pub struct Loaded {
    pub entity: Entity,
    pub key: ChunkKey,
    pub colliders: Vec<[f64; 3]>,
    pub terrain_triangles: usize,
}
struct Build {
    key: ChunkKey,
    mesh: MeshData,
    instances: Vec<Instance>,
    ms: f64,
}
struct Pending {
    key: ChunkKey,
    task: Task<Build>,
}
#[derive(Resource, Default)]
pub struct Stream {
    pub loaded: HashMap<ChunkCoord, Loaded>,
    pending: Vec<Pending>,
    pub desired: usize,
    pub last_ms: f64,
    pub triangles: usize,
    epoch: u64,
}
impl Stream {
    pub fn queue(&self) -> usize {
        self.pending.len()
    }
    pub fn blocked(&self, x: f64, z: f64, radius: f64) -> bool {
        let c = ChunkCoord::at(x, z);
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(l) = self.loaded.get(&ChunkCoord {
                    x: c.x + dx,
                    z: c.z + dz,
                }) {
                    for p in &l.colliders {
                        if (p[0] - x).hypot(p[2] - z) < radius + p[1] {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut stream: ResMut<Stream>,
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    nature: Res<Nature>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if stream.epoch != session.epoch {
        for (_, l) in stream.loaded.drain() {
            commands.entity(l.entity).despawn();
        }
        stream.pending.clear();
        stream.epoch = session.epoch;
        stream.triangles = 0;
    }
    let Some(generator) = session.generator.as_ref() else {
        return;
    };
    let center = ChunkCoord::at(player.position.x, player.position.z);
    let coords = wanted(center, graphics.radius.clamp(2, 12));
    let keys: HashMap<_, _> = coords
        .iter()
        .map(|&coord| {
            (
                coord,
                ChunkKey {
                    coord,
                    lod: lod_for(coord, center),
                    edges: edge_lods(coord, center),
                    revision: *session.chunk_revisions.get(&coord).unwrap_or(&0),
                },
            )
        })
        .collect();
    stream.desired = coords.len();
    let stale: Vec<_> = stream
        .loaded
        .keys()
        .filter(|c| !keys.contains_key(c))
        .copied()
        .collect();
    for c in stale {
        if let Some(l) = stream.loaded.remove(&c) {
            commands.entity(l.entity).despawn();
        }
    }
    // At most two GPU uploads per frame and four bounded background jobs.
    let mut uploaded = 0;
    for i in (0..stream.pending.len()).rev() {
        if uploaded >= 2 {
            break;
        }
        let ready = block_on(future::poll_once(&mut stream.pending[i].task));
        if let Some(build) = ready {
            stream.pending.swap_remove(i);
            if keys.get(&build.key.coord) != Some(&build.key) {
                continue;
            }
            if let Some(old) = stream.loaded.remove(&build.key.coord) {
                commands.entity(old.entity).despawn();
            }
            let o = build.key.coord.origin();
            let local = Vec3::new(
                (o[0] - session.origin.x) as f32,
                0.0,
                (o[1] - session.origin.z) as f32,
            );
            let count = build.mesh.indices.len() / 3;
            let root = commands
                .spawn((
                    ChunkRoot(build.key.coord),
                    Transform::from_translation(local),
                    Visibility::default(),
                    Name::new(format!(
                        "Terrain {},{} L{}",
                        build.key.coord.x, build.key.coord.z, build.key.lod
                    )),
                ))
                .id();
            let mut colliders = Vec::new();
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Mesh3d(meshes.add(assets::mesh(build.mesh))),
                    MeshMaterial3d(nature.terrain.clone()),
                ));
                // Ocean planes are generated only for chunks containing submerged terrain.
                if generator.height(o[0] + 64.0, o[1] + 64.0) < generator.settings.sea_level + 160.0
                {
                    p.spawn((
                        Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0))),
                        MeshMaterial3d(nature.water.clone()),
                        Transform::from_xyz(64.0, generator.settings.sea_level as f32, 64.0),
                    ));
                }
                for instance in &build.instances {
                    let pos = Vec3::new(
                        (instance.position[0] - o[0]) as f32,
                        instance.position[1] as f32,
                        (instance.position[2] - o[1]) as f32,
                    );
                    let t = Transform::from_translation(pos)
                        .with_rotation(Quat::from_rotation_y(instance.yaw))
                        .with_scale(Vec3::splat(instance.scale));
                    let v = instance.variant;
                    match instance.species {
                        Species::Broadleaf | Species::Pine | Species::Palm => {
                            colliders.push([
                                instance.position[0],
                                0.34 * f64::from(instance.scale),
                                instance.position[2],
                            ]);
                            p.spawn((
                                Mesh3d(nature.trunks[v].clone()),
                                MeshMaterial3d(nature.bark.clone()),
                                t,
                            ));
                            let crown = if build.key.lod >= 2 {
                                &nature.low_crown
                            } else {
                                match instance.species {
                                    Species::Pine => &nature.pines[v],
                                    Species::Palm => &nature.palms[v],
                                    _ => &nature.crowns[v],
                                }
                            };
                            p.spawn((
                                Mesh3d(crown.clone()),
                                MeshMaterial3d(nature.leaf.clone()),
                                t,
                            ));
                        }
                        Species::Rock => {
                            colliders.push([
                                instance.position[0],
                                0.7 * f64::from(instance.scale),
                                instance.position[2],
                            ]);
                            p.spawn((
                                Mesh3d(nature.rocks[v].clone()),
                                MeshMaterial3d(nature.stone.clone()),
                                t.with_scale(Vec3::new(
                                    instance.scale * 1.3,
                                    instance.scale * 0.7,
                                    instance.scale,
                                )),
                            ));
                        }
                        _ => {}
                    }
                }
                let ground = assets::ground_mesh(&build.instances, o);
                if !ground.indices.is_empty() {
                    p.spawn((
                        Mesh3d(meshes.add(assets::mesh(ground))),
                        MeshMaterial3d(nature.ground.clone()),
                    ));
                }
            });
            stream.last_ms = build.ms;
            stream.loaded.insert(
                build.key.coord,
                Loaded {
                    entity: root,
                    key: build.key,
                    colliders,
                    terrain_triangles: count,
                },
            );
            uploaded += 1;
        }
    }
    stream.triangles = stream.loaded.values().map(|l| l.terrain_triangles).sum();
    stream
        .pending
        .retain(|p| keys.get(&p.key.coord) == Some(&p.key));
    for coord in coords {
        if stream.pending.len() >= 4 {
            break;
        }
        let key = keys[&coord];
        if stream.loaded.get(&coord).is_some_and(|l| l.key == key)
            || stream.pending.iter().any(|p| p.key == key)
        {
            continue;
        }
        let generator: Arc<Generator> = Arc::clone(generator);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let start = Instant::now();
            let mesh = build_mesh(&generator, key);
            let instances = if key.lod < 3 {
                scatter(&generator, coord, key.lod)
            } else {
                Vec::new()
            };
            Build {
                key,
                mesh,
                instances,
                ms: start.elapsed().as_secs_f64() * 1000.0,
            }
        });
        stream.pending.push(Pending { key, task });
    }
}
pub fn relative(position: [f64; 3], origin: DVec3) -> Vec3 {
    Vec3::new(
        (position[0] - origin.x) as f32,
        (position[1] - origin.y) as f32,
        (position[2] - origin.z) as f32,
    )
}
