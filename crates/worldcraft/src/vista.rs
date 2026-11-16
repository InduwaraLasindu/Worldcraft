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
use worldcraft_core::{ChunkCoord, MeshData};
#[derive(Clone, Copy, PartialEq)]
struct Key {
    epoch: u64,
    revision: u64,
    center: ChunkCoord,
    radius: i64,
    distance: u32,
}
#[derive(Resource, Default)]
pub struct Vista {
    key: Option<Key>,
    pending: Option<(Key, Task<MeshData>)>,
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
) {
    if vista.epoch != session.epoch {
        for e in [vista.terrain.take(), vista.ocean.take()]
            .into_iter()
            .flatten()
        {
            commands.entity(e).despawn();
        }
        vista.pending = None;
        vista.key = None;
        vista.ocean_key = None;
        vista.epoch = session.epoch;
        stream.far_triangles = 0;
    }
    let Some(g) = session.generator.as_ref() else {
        return;
    };
    let center = ChunkCoord::at(player.position.x, player.position.z);
    let distance = match graphics.quality {
        Quality::Low => 8192,
        Quality::Balanced => 16384,
        Quality::High => 32768,
    };
    let key = Key {
        epoch: session.epoch,
        revision: session.revision,
        center,
        radius: graphics.radius.clamp(2, 12),
        distance,
    };
    if let Some((pending, task)) = vista.pending.as_mut() {
        if let Some(mesh) = block_on(future::poll_once(task)) {
            let completed = *pending;
            vista.pending = None;
            if completed == key {
                if let Some(e) = vista.terrain.take() {
                    commands.entity(e).despawn();
                }
                stream.far_triangles = mesh.indices.len() / 3;
                vista.terrain = Some(
                    commands
                        .spawn((
                            ChunkRoot(center),
                            Mesh3d(meshes.add(assets::mesh(mesh))),
                            MeshMaterial3d(nature.terrain.clone()),
                            Transform::default(),
                            Name::new("Distant terrain clipmap"),
                        ))
                        .id(),
                );
                vista.key = Some(key);
            }
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
    if vista.pending.is_none() && vista.key != Some(key) && stream.loaded.len() >= 9 {
        let g = std::sync::Arc::clone(g);
        vista.pending = Some((
            key,
            AsyncComputeTaskPool::get().spawn(async move {
                worldcraft_core::vista::build_vista(&g, center, key.radius, f64::from(key.distance))
            }),
        ));
    }
}
