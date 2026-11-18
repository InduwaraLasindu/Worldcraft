use crate::{
    assets::{self, Nature},
    foliage::Foliage,
    session::*,
};
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use std::{collections::HashMap, sync::Arc, time::Instant};
use worldcraft_core::{
    scatter::{scatter, Instance, Species},
    streaming::{lod_for, wanted},
    *,
};

#[derive(Component)]
pub struct ChunkRoot(pub ChunkCoord);
pub struct Loaded {
    pub entity: Entity,
    pub key: ChunkKey,
    pub terrain_mesh: Handle<Mesh>,
    pub foliage_entity: Entity,
    pub colliders: Vec<[f64; 3]>,
    pub terrain_triangles: usize,
    pub detail: u8,
    eco: bool,
}
struct Build {
    key: ChunkKey,
    mesh: Option<MeshData>,
    instances: Vec<Instance>,
    ground: Vec<(Vec3, MeshData)>,
    ms: f64,
}
struct Pending {
    key: ChunkKey,
    detail: u8,
    epoch: u64,
    eco: bool,
    task: Task<Build>,
}
#[derive(Resource, Default)]
pub struct Stream {
    pub loaded: HashMap<ChunkCoord, Loaded>,
    pending: Vec<Pending>,
    pub desired: usize,
    pub last_ms: f64,
    pub triangles: usize,
    pub far_triangles: usize,
    epoch: u64,
    last_launch: Option<Instant>,
    settled: Option<(u64, u64, ChunkCoord, i64, bool, [u64; 2])>,
    eco: bool,
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
/// Presentation detail is independent of the continuous terrain's topology key.
fn desired_detail(coord: ChunkCoord, center: ChunkCoord, eco: bool) -> u8 {
    let detail = lod_for(coord, center);
    if eco {
        detail.max(1)
    } else {
        detail
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
    limits: Res<crate::performance::Performance>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if stream.epoch != session.epoch {
        for (_, l) in stream.loaded.drain() {
            commands.entity(l.entity).despawn();
        }
        // Keep running synchronous tasks in the budget until they actually finish.
        // Dropping Bevy tasks cannot preempt a mesh build already on a worker.
        stream.settled = None;
        stream.epoch = session.epoch;
        stream.triangles = 0;
    }
    let eco = limits.mode() == crate::performance::Mode::Eco;
    if stream.eco != eco {
        // Replace each resident tile only when its new budget's result is ready.
        // This keeps a mode change from briefly removing the entire landscape.
        stream.eco = eco;
        stream.epoch = session.epoch;
        stream.settled = None;
    }
    if limits.suspended || session.generator.is_none() {
        // Drain completed results, without uploading or scheduling behind a menu.
        for i in (0..stream.pending.len()).rev() {
            if block_on(future::poll_once(&mut stream.pending[i].task)).is_some() {
                stream.pending.swap_remove(i);
            }
        }
        return;
    }
    let budget = limits.budget();
    let Some(generator) = session.generator.as_ref() else {
        return;
    };
    let center = ChunkCoord::at(player.position.x, player.position.z);
    let radius = limits.radius(graphics.radius);
    let plan = (
        session.epoch,
        session.revision,
        center,
        radius,
        eco,
        [player.position.x.to_bits(), player.position.z.to_bits()],
    );
    if stream.settled == Some(plan) && stream.pending.is_empty() {
        return;
    }
    let coords = wanted(center, radius);
    let keys: HashMap<_, _> = coords
        .iter()
        .map(|&coord| {
            (
                coord,
                ChunkKey {
                    coord,
                    lod: worldcraft_core::continuous::lod_at(
                        coord,
                        [player.position.x, player.position.z],
                    ),
                    edges: [0; 4],
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
    // Poll old epochs too; stale synchronous work must still count against the budget.
    let mut uploaded = 0;
    for i in (0..stream.pending.len()).rev() {
        if uploaded >= budget.uploads {
            break;
        }
        let ready = block_on(future::poll_once(&mut stream.pending[i].task));
        if let Some(build) = ready {
            let pending = stream.pending.swap_remove(i);
            if pending.epoch != session.epoch
                || pending.eco != eco
                || pending.detail != desired_detail(build.key.coord, center, eco)
                || keys.get(&build.key.coord) != Some(&build.key)
            {
                continue;
            }
            // A foliage-only result can reuse geometry only while its exact terrain
            // identity is still resident. Stale completions never create a bare tile.
            if build.mesh.is_none()
                && !stream
                    .loaded
                    .get(&build.key.coord)
                    .is_some_and(|old| old.key == build.key)
            {
                continue;
            }
            let old = stream.loaded.remove(&build.key.coord);
            let o = build.key.coord.origin();
            let (root, terrain_mesh, count) = if let Some(terrain) = build.mesh {
                if let Some(old) = old {
                    commands.entity(old.entity).despawn();
                }
                let count = terrain.indices.len() / 3;
                let handle = meshes.add(assets::mesh(terrain));
                let local = Vec3::new(
                    (o[0] - session.origin.x) as f32,
                    0.0,
                    (o[1] - session.origin.z) as f32,
                );
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
                commands.entity(root).with_children(|p| {
                    p.spawn((
                        Mesh3d(handle.clone()),
                        MeshMaterial3d(nature.terrain.clone()),
                    ));
                });
                (root, handle, count)
            } else {
                let old = old.expect("Reuse identity checked above");
                commands.entity(old.foliage_entity).despawn();
                (old.entity, old.terrain_mesh, old.terrain_triangles)
            };
            let foliage_entity = commands
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    Name::new("Nature"),
                ))
                .id();
            commands.entity(root).add_child(foliage_entity);
            let mut colliders = Vec::new();
            commands.entity(foliage_entity).with_children(|p| {
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
                        species if species.is_tree() => {
                            colliders.push([
                                instance.position[0],
                                0.34 * f64::from(instance.scale),
                                instance.position[2],
                            ]);
                            p.spawn((
                                Mesh3d(nature.trunk(instance.species, v).clone()),
                                MeshMaterial3d(nature.bark_for(instance.species).clone()),
                                t,
                                Foliage::Wood,
                                Foliage::Wood.range(radius),
                            ));
                            if !eco && pending.detail < 2 {
                                let crown = nature.crown(instance.species, v, false);
                                p.spawn((
                                    Mesh3d(crown.clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::DetailedCrown,
                                    Foliage::DetailedCrown.range(radius),
                                ));
                                p.spawn((
                                    Mesh3d(nature.crown(instance.species, v, true).clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::PairedCrown,
                                    Foliage::PairedCrown.range(radius),
                                    crate::foliage::NoRayProxy,
                                ));
                            } else {
                                p.spawn((
                                    Mesh3d(nature.crown(instance.species, v, true).clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::SimpleCrown,
                                    Foliage::SimpleCrown.range(radius),
                                ));
                            }
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
                                Foliage::Solid,
                                Foliage::Solid.range(radius),
                            ));
                        }
                        _ => {}
                    }
                }
                for (anchor, ground) in build.ground {
                    p.spawn((
                        Mesh3d(meshes.add(assets::mesh(ground))),
                        MeshMaterial3d(nature.ground.clone()),
                        Transform::from_translation(anchor),
                        Foliage::Ground,
                        Foliage::Ground.range(radius),
                        // Solari 0.20's secondary material layout has no alpha
                        // cutoff. An opaque grass-card BLAS would block light
                        // through every transparent blade gap. Keep primary
                        // raster/depth/shadow masks; omit this ray approximation.
                        crate::foliage::NoRayProxy,
                    ));
                }
            });
            stream.last_ms = build.ms;
            stream.loaded.insert(
                build.key.coord,
                Loaded {
                    entity: root,
                    key: build.key,
                    terrain_mesh,
                    foliage_entity,
                    colliders,
                    terrain_triangles: count,
                    detail: pending.detail,
                    eco,
                },
            );
            uploaded += 1;
        }
    }
    stream.triangles = stream.loaded.values().map(|l| l.terrain_triangles).sum();
    if stream.pending.is_empty()
        && stream.loaded.len() == keys.len()
        && stream.loaded.iter().all(|(coord, l)| {
            keys.get(coord) == Some(&l.key)
                && l.eco == eco
                && l.detail == desired_detail(*coord, center, eco)
        })
    {
        stream.settled = Some(plan);
        return;
    }
    if stream
        .last_launch
        .is_some_and(|t| t.elapsed() < budget.interval)
    {
        return;
    }
    stream.last_launch = Some(Instant::now());
    for coord in coords {
        if stream.pending.len() >= budget.jobs {
            break;
        }
        let key = keys[&coord];
        let detail = desired_detail(coord, center, eco);
        if stream
            .loaded
            .get(&coord)
            .is_some_and(|l| l.key == key && l.eco == eco && l.detail == detail)
            || stream.pending.iter().any(|p| {
                p.key == key && p.eco == eco && p.detail == detail && p.epoch == session.epoch
            })
        {
            continue;
        }
        // A presentation-band/budget update must not resample or reupload an
        // unchanged terrain grid. Full epoch/revision checks still apply above.
        let reuse_terrain = stream.loaded.get(&coord).is_some_and(|old| old.key == key);
        let generator: Arc<Generator> = Arc::clone(generator);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let start = Instant::now();
            let mesh = (!reuse_terrain)
                .then(|| worldcraft_core::continuous::build_continuous(&generator, key));
            let instances = if detail < 3 {
                scatter(&generator, coord, detail)
            } else {
                Vec::new()
            };
            // Build small ground-cover batches on the worker for tighter frustum bounds.
            let o = coord.origin();
            let mut cells: [Vec<Instance>; 16] = std::array::from_fn(|_| Vec::new());
            for instance in &instances {
                if instance.species.is_ground() {
                    let x = ((instance.position[0] - o[0]) / 32.0)
                        .floor()
                        .clamp(0.0, 3.0) as usize;
                    let z = ((instance.position[2] - o[1]) / 32.0)
                        .floor()
                        .clamp(0.0, 3.0) as usize;
                    cells[z * 4 + x].push(*instance);
                }
            }
            let mut surface = cells
                .iter()
                .any(|cell| !cell.is_empty())
                .then(|| crate::ground_surface::GroundSurface::new(&generator, o, mesh.as_ref()));
            let ground = cells
                .iter()
                .filter(|cell| !cell.is_empty())
                .map(|cell| assets::ground_batch(cell, o, surface.as_mut()))
                .collect();
            Build {
                key,
                mesh,
                instances,
                ground,
                ms: start.elapsed().as_secs_f64() * 1000.0,
            }
        });
        stream.pending.push(Pending {
            key,
            detail,
            epoch: session.epoch,
            eco,
            task,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vegetation_refines_without_a_terrain_grid_change_and_eco_avoids_useless_rebuilds() {
        let coord = ChunkCoord { x: 2, z: 0 };
        let a = ChunkCoord { x: 0, z: 0 };
        let b = ChunkCoord { x: 1, z: 0 };
        assert_eq!(worldcraft_core::continuous::lod_at(coord, [64.0, 64.0]), 0);
        assert_eq!(worldcraft_core::continuous::lod_at(coord, [192.0, 64.0]), 0);
        assert_eq!(desired_detail(coord, a, false), 1);
        assert_eq!(desired_detail(coord, b, false), 0);
        assert_eq!(
            desired_detail(coord, a, true),
            desired_detail(coord, b, true)
        );
    }
}
