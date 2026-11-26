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
    skyline: Option<Mesh>,
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
    settled: bool,
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
#[derive(Clone, Copy, PartialEq, Eq)]
struct PlanId {
    epoch: u64,
    revision: u64,
    observer: worldcraft_core::continuous::LodObserver,
    radius: i64,
    eco: bool,
}

pub(crate) struct TilePlan {
    id: PlanId,
    coords: Vec<ChunkCoord>,
    keys: HashMap<ChunkCoord, ChunkKey>,
}

const ROCKS_AND_SKYLINE: u8 = 4;

/// Presentation detail is independent of the continuous terrain's topology key.
fn desired_detail(coord: ChunkCoord, center: ChunkCoord, eco: bool, skyline: bool) -> u8 {
    // The last two rock bands can still contain visible boulders after every
    // detailed tree lies beyond its 480 m range.
    if skyline && (5..=6).contains(&coord.distance(center)) {
        return ROCKS_AND_SKYLINE;
    }
    // The nearest point of band three is 256 m away. All fine crowns
    // have reached their parent by 160 m, with a band of streaming slack.
    let detail = if coord.distance(center) == 3 {
        2
    } else {
        lod_for(coord, center)
    };
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
    mut cached: Local<Option<TilePlan>>,
) {
    if stream.epoch != session.epoch {
        for (_, l) in stream.loaded.drain() {
            commands.entity(l.entity).despawn();
        }
        // Keep running synchronous tasks in the budget until they actually finish.
        // Dropping Bevy tasks cannot preempt a mesh build already on a worker.
        stream.settled = false;
        stream.epoch = session.epoch;
        stream.triangles = 0;
    }
    let eco = limits.mode() == crate::performance::Mode::Eco;
    if stream.eco != eco {
        // Replace each resident tile only when its new budget's result is ready.
        // This keeps a mode change from briefly removing the entire landscape.
        stream.eco = eco;
        stream.epoch = session.epoch;
        stream.settled = false;
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
    let skyline = generator.version() >= 3;
    let eye = [player.position.x, player.position.z];
    let radius = limits.radius(graphics.radius);
    let tree_radius = Foliage::tree_radius(radius, skyline);
    let observer = worldcraft_core::continuous::LodObserver::at(eye);
    let id = PlanId {
        epoch: session.epoch,
        revision: session.revision,
        observer,
        radius,
        eco,
    };
    let center = observer.center;
    if cached.as_ref().is_none_or(|plan| plan.id != id) {
        let coords = wanted(center, radius);
        let keys: HashMap<_, _> = coords
            .iter()
            .map(|&coord| {
                (
                    coord,
                    ChunkKey {
                        coord,
                        lod: observer.lod(coord),
                        edges: [0; 4],
                        revision: *session.chunk_revisions.get(&coord).unwrap_or(&0),
                    },
                )
            })
            .collect();
        stream.desired = coords.len();
        stream.loaded.retain(|coord, tile| {
            if keys.contains_key(coord) {
                return true;
            }
            commands.entity(tile.entity).despawn();
            false
        });
        stream.settled = false;
        *cached = Some(TilePlan { id, coords, keys });
    } else if stream.settled && stream.pending.is_empty() {
        return;
    }
    let plan = cached.as_ref().expect("Active streaming plan");
    let keys = &plan.keys;
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
                || pending.detail != desired_detail(build.key.coord, center, eco, skyline)
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
                                Foliage::Wood.range(tree_radius),
                            ));
                            if !eco && pending.detail < 2 {
                                let crown = nature.crown(instance.species, v, false);
                                p.spawn((
                                    Mesh3d(crown.clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::DetailedCrown,
                                    Foliage::DetailedCrown.range(tree_radius),
                                ));
                                p.spawn((
                                    Mesh3d(nature.crown(instance.species, v, true).clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::PairedCrown,
                                    Foliage::PairedCrown.range(tree_radius),
                                    crate::foliage::NoRayProxy,
                                ));
                            } else {
                                p.spawn((
                                    Mesh3d(nature.crown(instance.species, v, true).clone()),
                                    MeshMaterial3d(nature.foliage(instance.species).clone()),
                                    t,
                                    Foliage::SimpleCrown,
                                    Foliage::SimpleCrown.range(tree_radius),
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
                if let Some(mesh) = build.skyline {
                    p.spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(nature.skyline.clone()),
                        bevy::light::NotShadowCaster,
                        crate::foliage::NoRayProxy,
                    ));
                }
                for (anchor, ground) in build.ground {
                    p.spawn((
                        Mesh3d(meshes.add(assets::mesh(ground))),
                        MeshMaterial3d(nature.ground.clone()),
                        Transform::from_translation(anchor),
                        Foliage::Ground,
                        Foliage::Ground.range(tree_radius),
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
                && l.detail == desired_detail(*coord, center, eco, skyline)
        })
    {
        stream.settled = true;
        return;
    }
    if stream
        .last_launch
        .is_some_and(|t| t.elapsed() < budget.interval)
    {
        return;
    }
    stream.last_launch = Some(Instant::now());
    for &coord in &plan.coords {
        if stream.pending.len() >= budget.jobs {
            break;
        }
        let key = keys[&coord];
        let detail = desired_detail(coord, center, eco, skyline);
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
            let mut instances = if detail == ROCKS_AND_SKYLINE {
                scatter(&generator, coord, 2)
            } else if detail < 3 {
                scatter(&generator, coord, detail)
            } else {
                Vec::new()
            };
            let skyline = if eco || generator.version() < 3 {
                Default::default()
            } else if detail != 3 {
                crate::skyline::near(&instances, coord, generator.settings().seed)
            } else {
                let trees = worldcraft_core::scatter::skyline_scatter(&generator, coord);
                crate::skyline::near(&trees, coord, generator.settings().seed)
            };
            let skyline = skyline.into_mesh();
            if detail == ROCKS_AND_SKYLINE {
                instances.retain(|p| p.species == Species::Rock);
            }
            // Build small ground-cover batches on the worker for tighter frustum bounds.
            let o = coord.origin();
            let mut cells: [Vec<Instance>; 16] = std::array::from_fn(|_| Vec::new());
            let mut has_limb = [false; 16];
            for instance in &instances {
                let ground = instance.species.is_ground();
                let woody_litter = detail == 0
                    && generator.version() >= 3
                    && instance.variant == 1
                    && matches!(
                        instance.species,
                        Species::Broadleaf | Species::Birch | Species::Pine
                    );
                if !ground && !woody_litter {
                    continue;
                }
                let x = ((instance.position[0] - o[0]) / 32.0)
                    .floor()
                    .clamp(0.0, 3.0) as usize;
                let z = ((instance.position[2] - o[1]) / 32.0)
                    .floor()
                    .clamp(0.0, 3.0) as usize;
                let cell = z * 4 + x;
                if ground || !has_limb[cell] {
                    cells[cell].push(*instance);
                    has_limb[cell] |= woody_litter;
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
                skyline,
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
        assert_eq!(desired_detail(coord, a, false, true), 1);
        assert_eq!(desired_detail(coord, b, false, true), 0);
        assert_eq!(
            desired_detail(coord, a, true, true),
            desired_detail(coord, b, true, true)
        );
    }
}
