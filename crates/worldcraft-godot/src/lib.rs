//! Godot bridge for Worldcraft's versioned Rust generator and save format.
use godot::{classes::RefCounted, prelude::*};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
};
use worldcraft_core::{
    config::seed_from_text,
    ground_cover, lakes,
    mesh::MeshData,
    scatter::{scatter, Species},
    trees, Biome, Brush, BrushKind, ChunkCoord, ChunkKey, EditHistory, Generator,
    GeneratorSettings, Preset, Stroke, WorldFile, CHUNK_SIZE,
};

mod terrain3d_bridge;

struct WorldcraftExtension;

#[gdextension]
unsafe impl ExtensionLibrary for WorldcraftExtension {}

const MAX_JOBS: usize = 4;

enum Region {
    Tile(ChunkKey, u8),
    Horizon(ChunkCoord, i64, f64),
}

struct Job {
    region: Region,
    generator: Arc<Generator>,
    epoch: i64,
}

struct Completed {
    coord: ChunkCoord,
    lod: u8,
    horizon: bool,
    epoch: i64,
    mesh: MeshData,
    nature: BTreeMap<(u8, u8), Vec<f32>>,
    turf: Vec<Vec<f32>>,
}

fn build(job: Job) -> Completed {
    let Job {
        region,
        generator,
        epoch,
    } = job;
    match region {
        Region::Tile(key, detail) => {
            let mesh = worldcraft_core::mesh::build_habitat_mesh(&generator, key);
            let mut nature = BTreeMap::new();
            let [x, z] = key.coord.origin();
            for instance in scatter(&generator, key.coord, if detail == 2 { 0 } else { detail }) {
                let [wx, y, wz] = instance.position;
                let shape = Species::TREES
                    .iter()
                    .position(|&s| s == instance.species)
                    .map(|family| {
                        trees::shape(trees::recipe(generator.settings().seed, family, wx, wz))
                    });
                let variant = if let Some(shape) = shape {
                    usize::from(shape.prototype)
                } else if instance.species == Species::Rock {
                    instance.variant % if detail == 1 { 2 } else { 4 }
                } else {
                    instance.variant % 2
                };
                let buffer = nature
                    .entry((instance.species as u8, variant as u8))
                    .or_default();
                instance_transform(
                    buffer,
                    [(wx - x) as f32, y as f32, (wz - z) as f32],
                    [0.0, 1.0, 0.0],
                    instance.scale,
                    instance.yaw,
                    variant as u8,
                    shape,
                );
            }
            for prop in ground_cover::scatter(&generator, key.coord, key.lod, detail, &mesh) {
                let variant = if detail == 1 { 0 } else { prop.variant };
                let buffer = nature.entry((prop.kind as u8, variant)).or_default();
                instance_transform(
                    buffer, prop.local, prop.up, prop.scale, prop.yaw, variant, None,
                );
            }
            let turf = if detail == 0 {
                ground_cover::grass(&generator, key.coord, key.lod, &mesh)
                    .into_iter()
                    .filter(|patch| !patch.is_empty())
                    .map(|patch| {
                        let mut buffer = Vec::with_capacity(16 * patch.len());
                        for (prop, dryness) in patch {
                            instance_transform(
                                &mut buffer,
                                prop.local,
                                prop.up,
                                prop.scale,
                                prop.yaw,
                                prop.variant,
                                None,
                            );
                            let end = buffer.len() - 1;
                            buffer[end] = dryness;
                        }
                        buffer
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Completed {
                coord: key.coord,
                lod: key.lod,
                horizon: false,
                epoch,
                mesh,
                nature,
                turf,
            }
        }
        Region::Horizon(coord, radius, reach) => Completed {
            coord,
            lod: 3,
            horizon: true,
            epoch,
            mesh: worldcraft_core::vista::build_habitat_vista(&generator, coord, radius, reach),
            nature: distant_forest(&generator, coord, radius, reach.min(1730.0)),
            turf: Vec::new(),
        },
    }
}

fn distant_forest(
    generator: &Generator,
    center: ChunkCoord,
    inner: i64,
    reach: f64,
) -> BTreeMap<(u8, u8), Vec<f32>> {
    use worldcraft_core::{generation::unit, scatter::canopy_species};
    let mut result = BTreeMap::new();
    let settings = generator.settings();
    let seed = settings.seed ^ 0x5393_f207;
    if settings.vegetation < 0.07 {
        return result;
    }
    let [origin_x, origin_z] = center.origin();
    let focus_x = origin_x + CHUNK_SIZE * 0.5;
    let focus_z = origin_z + CHUNK_SIZE * 0.5;
    const SPACING: f64 = 15.5;
    let min = |v: f64| ((v - reach) / SPACING).floor() as i64;
    let max = |v: f64| ((v + reach) / SPACING).ceil() as i64;
    for iz in min(focus_z)..=max(focus_z) {
        for ix in min(focus_x)..=max(focus_x) {
            let random = |salt| unit(seed ^ salt, ix, iz);
            let (x, z) = (
                (ix as f64 + random(1)) * SPACING,
                (iz as f64 + random(2)) * SPACING,
            );
            if (x - focus_x).hypot(z - focus_z) > reach
                || ChunkCoord::at(x, z).distance(center) <= inner
            {
                continue;
            }
            let sample = generator.preview_sample(x, z);
            if sample.height < generator.water_level(x, z) + 3.0 || sample.vegetation < 0.08 {
                continue;
            }
            let blend = generator.biome_blend(x, z, &sample);
            let stand =
                0.62 + worldcraft_core::geology::gradient(seed, x / 177.0, z / 177.0) * 0.65;
            if random(3)
                > (f64::from(blend.habitat().canopy) * sample.vegetation * stand * 1.32).min(0.94)
            {
                continue;
            }
            let slope_x = (generator.preview_height(x + 8.0, z) - sample.height) / 8.0;
            let slope_z = (generator.preview_height(x, z + 8.0) - sample.height) / 8.0;
            if slope_x.hypot(slope_z) > 0.79 {
                continue;
            }
            let pine =
                canopy_species(&blend, sample.temperature, random(4) as f32) == Species::Pine;
            instance_transform(
                result.entry((u8::from(pine), 0)).or_default(),
                [
                    (x - origin_x) as f32,
                    sample.height as f32 - 2.8,
                    (z - origin_z) as f32,
                ],
                [0.0, 1.0, 0.0],
                (0.62 + random(5) * 0.86) as f32,
                (random(6) * std::f64::consts::TAU) as f32,
                0,
                None,
            );
        }
    }
    result
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn instance_transform(
    buffer: &mut Vec<f32>,
    p: [f32; 3],
    up: [f32; 3],
    scale: f32,
    yaw: f32,
    variant: u8,
    shape: Option<trees::Shape>,
) {
    let (sin, cos) = yaw.sin_cos();
    let right = cross(up, [sin, 0.0, cos]);
    let length = right[0].hypot(right[1].hypot(right[2]));
    let right = right.map(|v| v / length);
    let forward = cross(right, up);
    let v = f32::from(variant);
    let tall = scale * shape.map_or_else(|| 0.86 + 0.23 * (yaw * 1.16 + v).sin(), |s| s.height);
    let spread = scale * shape.map_or(1.0, |s| s.spread);
    for row in 0..3 {
        buffer.extend([
            right[row] * spread,
            up[row] * tall,
            forward[row] * spread,
            p[row],
        ]);
    }
    buffer.extend([
        (yaw * 0.511).fract(),
        (yaw * 0.737).fract(),
        0.39 + 0.37 * (yaw + v).sin() + 0.17 * scale,
        shape.map_or(1.0, |s| s.clearance),
    ]);
}

fn mesh_packet(mesh: MeshData) -> VarDictionary {
    let mut packet = VarDictionary::new();
    let positions: PackedVector3Array = mesh
        .positions
        .into_iter()
        .map(|[x, y, z]| Vector3::new(x, y, z))
        .collect();
    let normals: PackedVector3Array = mesh
        .normals
        .into_iter()
        .map(|[x, y, z]| Vector3::new(x, y, z))
        .collect();
    let colors: PackedColorArray = mesh
        .colors
        .into_iter()
        .enumerate()
        .map(|(i, [r, g, b, a])| {
            let snow = mesh
                .snow_cover
                .get(i)
                .map_or(a, |v| f32::from(v[0]) / 255.0);
            Color::from_rgba(r, g, b, snow)
        })
        .collect();
    let uv: PackedVector2Array = mesh
        .uv
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| {
            mesh.snow_cover.get(i).map_or(Vector2::new(x, y), |v| {
                Vector2::new(f32::from(v[2]) / 255.0, f32::from(v[3]) / 255.0)
            })
        })
        .collect();
    let indices: PackedInt32Array = mesh
        .indices
        .chunks_exact(3)
        .flat_map(|i| [i[0] as i32, i[2] as i32, i[1] as i32])
        .collect();
    packet.set("positions", &positions.to_variant());
    packet.set("normals", &normals.to_variant());
    packet.set("colors", &colors.to_variant());
    packet.set("uv", &uv.to_variant());
    packet.set("indices", &indices.to_variant());
    packet
}

#[derive(GodotClass)]
#[class(base=RefCounted)]
struct WorldcraftNative {
    base: Base<RefCounted>,
    world: WorldFile,
    generator: Arc<Generator>,
    sender: Option<mpsc::SyncSender<Job>>,
    results: mpsc::Receiver<Option<Completed>>,
    worker: Option<JoinHandle<()>>,
    current_epoch: Arc<AtomicI64>,
    jobs: usize,
    epoch: i64,
    history: EditHistory,
    cache_dir: Option<PathBuf>,
}

#[godot_api]
impl IRefCounted for WorldcraftNative {
    fn init(base: Base<RefCounted>) -> Self {
        let (sender, queue) = mpsc::sync_channel(MAX_JOBS);
        let (finish, results) = mpsc::channel();
        let current_epoch = Arc::new(AtomicI64::new(0));
        let worker_epoch = Arc::clone(&current_epoch);
        let worker = std::thread::Builder::new()
            .name("worldcraft-terrain".into())
            .spawn(move || {
                while let Ok(job) = queue.recv() {
                    let job: Job = job;
                    let result =
                        (job.epoch == worker_epoch.load(Ordering::Acquire)).then(|| build(job));
                    if finish.send(result).is_err() {
                        break;
                    }
                }
            })
            .expect("Start terrain worker");
        let world = WorldFile::new("Worldcraft".into(), GeneratorSettings::default());
        let generator = Arc::new(Generator::for_world(&world).expect("Default world"));
        Self {
            base,
            world,
            generator,
            sender: Some(sender),
            results,
            worker: Some(worker),
            current_epoch,
            jobs: 0,
            epoch: 0,
            history: EditHistory::default(),
            cache_dir: None,
        }
    }
}

impl Drop for WorldcraftNative {
    fn drop(&mut self) {
        self.current_epoch.store(i64::MIN, Ordering::Release);
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl WorldcraftNative {
    fn next_epoch(&mut self) {
        self.epoch += 1;
        self.current_epoch.store(self.epoch, Ordering::Release);
    }

    fn install(&mut self, world: WorldFile, generator: Generator) {
        if let Some(cache) = &self.cache_dir {
            generator.set_region_cache_dir(cache.clone());
        }
        self.history = EditHistory::from_transactions(world.edits.clone());
        self.world = world;
        self.generator = Arc::new(generator);
        self.next_epoch();
    }

    fn request(&mut self, region: Region) -> bool {
        if self.jobs >= MAX_JOBS {
            return false;
        }
        let job = Job {
            region,
            epoch: self.epoch,
            generator: self.generator.clone(),
        };
        if self
            .sender
            .as_ref()
            .expect("Live worker")
            .try_send(job)
            .is_err()
        {
            return false;
        }
        self.jobs += 1;
        true
    }

    fn apply_history(&mut self) {
        let edits = self
            .history
            .transactions
            .iter()
            .flatten()
            .cloned()
            .collect();
        self.generator = Arc::new(self.generator.with_edits(edits).expect("Validated edits"));
        self.world.edits = self.history.transactions.clone();
        self.next_epoch();
    }
    fn generate_world(&mut self, seed: GString, preset: i64, version: u32) -> VarDictionary {
        let preset = Preset::ALL
            .get(preset as usize)
            .copied()
            .unwrap_or_default();
        let settings = GeneratorSettings::for_preset(preset, seed_from_text(&seed.to_string()));
        let mut world = WorldFile::new(preset.label().into(), settings);
        world.generator_version = version;
        let generator = Generator::for_world(&world).expect("Validated preset");
        world.camera = generator.spawn_position();
        if let Some(lake) = generator.spawn_lake() {
            let ([x, z], [tx, tz]) = lake.shore();
            world.yaw = (x - tx).atan2(z - tz) as f32;
            world.pitch = -0.035;
        }
        self.install(world, generator);
        self.info()
    }
}

#[godot_api]
impl WorldcraftNative {
    #[func]
    fn set_region_cache_directory(&mut self, directory: GString) {
        self.cache_dir = Some(directory.to_string().into());
    }

    #[func]
    fn create_world(&mut self, seed: GString, preset: i64) -> VarDictionary {
        self.generate_world(seed, preset, worldcraft_core::GENERATOR_VERSION)
    }

    #[func]
    fn create_regional_world(&mut self, seed: GString, preset: i64) -> VarDictionary {
        self.generate_world(
            seed,
            preset,
            worldcraft_core::config::REGIONAL_GENERATOR_VERSION,
        )
    }

    #[func]
    fn open_world(&mut self, path: GString) -> VarDictionary {
        let opened = worldcraft_core::load_world(Path::new(&path.to_string())).and_then(|world| {
            let generator = Generator::for_world(&world)?;
            Ok((world, generator))
        });
        match opened {
            Ok((world, generator)) => {
                self.install(world, generator);
                self.info()
            }
            Err(error) => {
                let mut result = VarDictionary::new();
                result.set("error", error.as_str());
                result
            }
        }
    }

    #[func]
    fn save_world(
        &mut self,
        path: GString,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
    ) -> GString {
        let mut snapshot = self.world.clone();
        snapshot.camera = [x, y, z];
        snapshot.yaw = yaw;
        snapshot.pitch = pitch;
        match worldcraft_core::save_world(Path::new(&path.to_string()), &snapshot) {
            Ok(()) => {
                self.world = snapshot;
                GString::new()
            }
            Err(error) => GString::from(error.as_str()),
        }
    }

    #[func]
    fn info(&self) -> VarDictionary {
        let mut info = VarDictionary::new();
        let [x, y, z] = self.world.camera;
        info.set("x", x);
        info.set("y", y);
        info.set("z", z);
        info.set("yaw", self.world.yaw);
        info.set("pitch", self.world.pitch);
        info.set("seed", self.world.settings.seed as i64);
        info.set("seed_text", self.world.settings.seed.to_string().as_str());
        info.set("sea", self.world.settings.sea_level);
        info.set("preset", self.world.settings.preset.label());
        info.set("generator_version", i64::from(self.world.generator_version));
        info
    }

    #[func]
    fn climate(&self, x: f64, z: f64) -> VarDictionary {
        let s = self.generator.preview_sample(x, z);
        let mut data = VarDictionary::new();
        data.set("biome", s.biome.label());
        data.set("kind", s.biome as i64);
        data.set("height", s.height);
        data.set("temperature", s.temperature);
        data.set("humidity", s.moisture);
        data
    }

    #[func]
    fn ground_height(&self, x: f64, z: f64) -> f64 {
        const STEP: f64 = CHUNK_SIZE / 32.0;
        let a = (x / STEP).floor() * STEP;
        let b = (z / STEP).floor() * STEP;
        let (u, v) = ((x - a) / STEP, (z - b) / STEP);
        let east = self.generator.interactive_height(a + STEP, b);
        let south = self.generator.interactive_height(a, b + STEP);
        if u + v <= 1.0 {
            let h = self.generator.interactive_height(a, b);
            h + u * (east - h) + v * (south - h)
        } else {
            let h = self.generator.interactive_height(a + STEP, b + STEP);
            h + (1.0 - u) * (south - h) + (1.0 - v) * (east - h)
        }
    }

    #[func]
    fn brush(&mut self, x: f64, z: f64, kind: i64, radius: f64, strength: f64, biome: i64) -> bool {
        if ![x, z]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= worldcraft_core::config::MAX_WORLD_COORD)
        {
            return false;
        }
        let Some(&kind) = BrushKind::ALL.get(kind as usize) else {
            return false;
        };
        let radius = radius.clamp(1.0, 256.0);
        let mut target_height = 0.0;
        if matches!(kind, BrushKind::Flatten | BrushKind::Smooth) {
            target_height = self.generator.height(x, z);
            if kind == BrushKind::Smooth {
                let offset = radius * 0.35;
                target_height = (target_height
                    + [
                        [x - offset, z],
                        [x + offset, z],
                        [x, z - offset],
                        [x, z + offset],
                    ]
                    .map(|[a, b]| self.generator.height(a, b))
                    .iter()
                    .sum::<f64>())
                    * 0.2;
            }
        }
        let stroke = Stroke {
            center: [x, z],
            brush: Brush {
                kind,
                radius,
                strength,
                target_height,
                biome: Biome::ALL.get(biome as usize).copied().unwrap_or_default(),
                density: (strength / 5.0).min(2.0),
            },
        };
        if self.history.push(vec![stroke]).is_err() {
            return false;
        }
        self.apply_history();
        true
    }

    #[func]
    fn undo(&mut self, redo: bool) -> bool {
        let changed = if redo {
            self.history.redo()
        } else {
            self.history.undo()
        };
        if changed {
            self.apply_history();
        }
        changed
    }

    #[func]
    fn request_tile(
        &mut self,
        x: i64,
        z: i64,
        lod: i64,
        foliage: i64,
        neighbors: Vector4i,
    ) -> bool {
        let lod = lod.clamp(0, 3) as u8;
        let edges =
            [neighbors.x, neighbors.y, neighbors.z, neighbors.w].map(|v| v.clamp(0, 3) as u8);
        self.request(Region::Tile(
            ChunkKey {
                coord: ChunkCoord { x, z },
                lod,
                edges,
                revision: 0,
            },
            foliage.clamp(0, 2) as u8,
        ))
    }

    #[func]
    fn request_horizon(&mut self, x: i64, z: i64, radius: i64, reach: f64) -> bool {
        self.request(Region::Horizon(
            ChunkCoord { x, z },
            radius.clamp(1, 6),
            reach.clamp(1024.0, 16384.0),
        ))
    }

    #[func]
    fn poll_chunk(&mut self) -> VarDictionary {
        while let Ok(result) = self.results.try_recv() {
            self.jobs -= 1;
            let Some(result) = result else {
                continue;
            };
            if result.epoch != self.epoch {
                continue;
            }
            let mut data = mesh_packet(result.mesh);
            let nature: Array<VarDictionary> = result
                .nature
                .into_iter()
                .map(|((species, variant), values)| {
                    let mut batch = VarDictionary::new();
                    batch.set("species", i64::from(species));
                    batch.set("variant", i64::from(variant));
                    batch.set(
                        "buffer",
                        &PackedFloat32Array::from(values.as_slice()).to_variant(),
                    );
                    batch
                })
                .collect();
            data.set("x", result.coord.x);
            data.set("z", result.coord.z);
            data.set("lod", i64::from(result.lod));
            data.set("horizon", result.horizon);
            data.set("nature", &nature.to_variant());
            let turf: Array<PackedFloat32Array> = result
                .turf
                .iter()
                .map(|values| PackedFloat32Array::from(values.as_slice()))
                .collect();
            data.set("turf", &turf.to_variant());
            return data;
        }
        VarDictionary::new()
    }

    #[func]
    fn lakes(&self, x: f64, z: f64) -> Array<VarDictionary> {
        let [cx, cz] = lakes::cell(x, z);
        let mut result = Array::new();
        for dz in -1..=1 {
            for dx in -1..=1 {
                if let Some(lake) = self.generator.lake([cx + dx, cz + dz]) {
                    let mut data = mesh_packet(lake.mesh());
                    data.set(
                        "cell",
                        Vector2i::new(lake.cell[0] as i32, lake.cell[1] as i32),
                    );
                    data.set("x", lake.center[0]);
                    data.set("y", lake.level);
                    data.set("z", lake.center[1]);
                    result.push(&data);
                }
            }
        }
        result
    }

    #[func]
    fn tree_meshes(&self, family: i64, profile: i64, coarse: bool) -> VarDictionary {
        if !(0..trees::HEIGHTS.len() as i64).contains(&family) || !(0..4).contains(&profile) {
            return VarDictionary::new();
        }
        let (wood, leaves) = trees::meshes(
            self.world.settings.seed,
            family as usize,
            profile as u8,
            coarse,
        );
        let mut data = VarDictionary::new();
        data.set("wood", &mesh_packet(wood).to_variant());
        data.set("leaves", &mesh_packet(leaves).to_variant());
        data
    }

    #[func]
    fn tree_atlas(&self) -> PackedByteArray {
        PackedByteArray::from(trees::foliage_atlas(self.world.settings.seed).as_slice())
    }

    #[func]
    fn tree_view(&self, family: i64) -> PackedFloat64Array {
        let Some(&species) = Species::TREES.get(family as usize) else {
            return PackedFloat64Array::new();
        };
        let family = family as usize;
        let [sx, _, sz] = self.generator.spawn_position();
        let origin = ChunkCoord::at(sx, sz);
        let mut found = None;
        for ring in 0_i64..=4 {
            for z in -ring..=ring {
                for x in -ring..=ring {
                    if x.abs().max(z.abs()) != ring {
                        continue;
                    }
                    let coord = ChunkCoord {
                        x: origin.x + x,
                        z: origin.z + z,
                    };
                    for tree in scatter(&self.generator, coord, 1) {
                        if tree.species != species {
                            continue;
                        }
                        let [wx, _, wz] = tree.position;
                        let score = (wx - sx).hypot(wz - sz);
                        if found.as_ref().is_none_or(|(best, _)| score < *best) {
                            found = Some((score, tree));
                        }
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
        let Some((_, tree)) = found else {
            return PackedFloat64Array::new();
        };
        let [x, y, z] = tree.position;
        let shape = trees::shape(trees::recipe(self.world.settings.seed, family, x, z));
        let height = f64::from(trees::HEIGHTS[family] * tree.scale * shape.height);
        let view_x = x + 1.03 * height;
        let view_z = z + 0.55 * height;
        let view_y = (y + 0.36 * height).max(self.generator.height(view_x, view_z) + 1.75);
        [view_x, view_y, view_z, x, y + 0.51 * height, z]
            .into_iter()
            .collect()
    }

    #[func]
    fn river(&self) -> VarDictionary {
        if self.world.settings.preset == Preset::RiverValley {
            let settings = &self.world.settings;
            let mut packet = mesh_packet(worldcraft_core::river::animated_water_mesh(settings));
            let ([x, y, z], width) = worldcraft_core::river::plunge_pool(settings);
            packet.set("plunge", Vector3::new(x as f32, y as f32, z as f32));
            packet.set("plunge_half_width", width);
            packet
        } else {
            VarDictionary::new()
        }
    }

    #[func]
    fn waterfall_camera(&self) -> PackedFloat64Array {
        let (camera, target) = worldcraft_core::river::waterfall_view(&self.world.settings);
        camera.into_iter().chain(target).collect()
    }
}
