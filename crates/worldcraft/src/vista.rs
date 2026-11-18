//! One bounded background vista build and one horizon ocean per active world.
use crate::{
    assets::{self, Nature},
    session::*,
    streaming::{ChunkRoot, Stream},
};
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use worldcraft_core::ChunkCoord;
#[derive(Clone, Copy, PartialEq)]
struct Key {
    epoch: u64,
    revision: u64,
    center: ChunkCoord,
    radius: i64,
    distance: u32,
}
struct VistaMesh {
    terrain: Mesh,
    triangles: usize,
    forests: Vec<Mesh>,
}
struct StagedVista {
    key: Key,
    mesh: VistaMesh,
    handles: Vec<Handle<Mesh>>,
}
#[derive(Resource, Default)]
pub struct Vista {
    key: Option<Key>,
    pending: Option<(Key, Task<VistaMesh>)>,
    staged: Option<StagedVista>,
    terrain: Option<Entity>,
    ocean: Option<Entity>,
    ocean_key: Option<(ChunkCoord, u32)>,
    epoch: u64,
}
pub fn update(
    mut commands: Commands,
    mut vista: ResMut<Vista>,
    session: Res<Session>,
    player: Res<Player>,
    graphics: Res<Graphics>,
    nature: Res<Nature>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stream: ResMut<Stream>,
    limits: Res<crate::performance::Performance>,
) {
    if vista.epoch != session.epoch {
        for e in [vista.terrain.take(), vista.ocean.take()]
            .into_iter()
            .flatten()
        {
            commands.entity(e).despawn();
        }
        // Retain running work until it completes, so changing worlds cannot overfill the worker.
        vista.key = None;
        vista.ocean_key = None;
        vista.staged = None;
        vista.epoch = session.epoch;
        stream.far_triangles = 0;
    }
    let Some(g) = session.generator.as_ref() else {
        // Unloading a world must also release completed off-thread mesh results.
        if let Some((_, task)) = vista.pending.as_mut() {
            if block_on(future::poll_once(task)).is_some() {
                vista.pending = None;
            }
        }
        return;
    };
    if limits.suspended {
        return;
    }
    let budget = limits.budget();
    if !budget.far {
        vista.staged = None;
        if let Some(e) = vista.terrain.take() {
            commands.entity(e).despawn();
        }
        stream.far_triangles = 0;
        vista.key = None;
        // Retire an in-flight build before allowing another far job.
        if let Some((_, task)) = vista.pending.as_mut() {
            if block_on(future::poll_once(task)).is_some() {
                vista.pending = None;
            }
        }
    }
    let center = ChunkCoord::at(player.position.x, player.position.z);
    let distance = if !budget.far {
        2048
    } else {
        match limits.quality(graphics.quality) {
            Quality::Low => 8192,
            Quality::Balanced => 16384,
            Quality::High => 32768,
        }
    };
    let key = Key {
        epoch: session.epoch,
        revision: session.revision,
        center,
        radius: limits.radius(graphics.radius),
        distance,
    };
    if let Some((pending, task)) = vista.pending.as_mut() {
        if let Some(mesh) = block_on(future::poll_once(task)) {
            let completed = *pending;
            vista.pending = None;
            if completed == key && budget.far {
                vista.staged = Some(StagedVista {
                    key,
                    mesh,
                    handles: Vec::new(),
                });
            }
        }
    }
    if vista.staged.as_ref().is_some_and(|s| s.key != key) {
        vista.staged = None;
    }
    if let Some(staged) = &mut vista.staged {
        // Keep the previous landscape visible while preparing a bounded number
        // of forest sectors per frame. A rebase or edit retires the stale batch.
        for _ in 0..(budget.uploads * 3) {
            let Some(mesh) = staged.mesh.forests.pop() else {
                break;
            };
            staged.handles.push(meshes.add(mesh));
        }
        if staged.mesh.forests.is_empty() {
            let staged = vista.staged.take().unwrap();
            if let Some(entity) = vista.terrain.take() {
                commands.entity(entity).despawn();
            }
            stream.far_triangles = staged.mesh.triangles;
            vista.terrain = Some(
                commands
                    .spawn((
                        ChunkRoot(center),
                        Mesh3d(meshes.add(staged.mesh.terrain)),
                        MeshMaterial3d(nature.terrain.clone()),
                        Transform::default(),
                        Name::new("Distant terrain clipmap"),
                    ))
                    .with_children(|p| {
                        for mesh in staged.handles {
                            p.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(nature.skyline.clone()),
                                bevy::light::NotShadowCaster,
                                crate::foliage::NoRayProxy,
                            ));
                        }
                    })
                    .id(),
            );
            vista.key = Some(key);
        }
    }
    if vista.ocean.is_none() || vista.ocean_key != Some((center, distance)) {
        if let Some(e) = vista.ocean.take() {
            commands.entity(e).despawn();
        }
        let root = commands
            .spawn((
                ChunkRoot(center),
                Transform::default(),
                Visibility::default(),
                Name::new("Horizon ocean"),
            ))
            .id();
        commands.entity(root).with_children(|p| {
            p.spawn((
                Mesh3d(
                    meshes.add(
                        Plane3d::default()
                            .mesh()
                            .size(distance as f32 * 4.0, distance as f32 * 4.0),
                    ),
                ),
                MeshMaterial3d(nature.water.clone()),
                Transform::from_xyz(0.0, g.settings().sea_level as f32, 0.0),
            ));
        });
        vista.ocean = Some(root);
        vista.ocean_key = Some((center, distance));
    }
    if budget.far
        && vista.pending.is_none()
        && vista.staged.is_none()
        && vista.key != Some(key)
        && stream.loaded.len() >= stream.desired.min(9)
    {
        let g = std::sync::Arc::clone(g);
        vista.pending = Some((
            key,
            AsyncComputeTaskPool::get().spawn(async move {
                let terrain = worldcraft_core::vista::build_continuous_vista(
                    &g,
                    center,
                    key.radius,
                    f64::from(key.distance),
                );
                let forests = crate::skyline::horizon(&g, center, &terrain);
                VistaMesh {
                    triangles: terrain.indices.len() / 3,
                    terrain: assets::mesh(terrain),
                    forests: forests
                        .into_iter()
                        .filter_map(crate::skyline::Batch::into_mesh)
                        .collect(),
                }
            }),
        ));
    }
}
