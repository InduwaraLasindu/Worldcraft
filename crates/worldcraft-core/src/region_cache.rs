//! Disk cache for unedited, deterministic terrain bakes.
//!
//! A damaged, incompatible or missing file is a cache miss. Writes publish a
//! complete frame in one rename, so readers can continue using the old entry
//! until the replacement has finished encoding.

use crate::terrain_region::{RegionCoord, TerrainRegion, REGION_PADDING_M, REGION_SIZE_M};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAGIC: &[u8; 8] = b"WCR\0\r\n\x1a\n";
const SCHEMA: u32 = 2;
const HEADER_SIZE: usize = 48;
// The shipped cache stores the 1 m grid, including both padding margins and
// the shared boundary vertex. Coarser bakes fit within the same limit.
const MAX_SIDE: usize = (REGION_SIZE_M + 2 * REGION_PADDING_M + 1) as usize;
const MAX_FILE_BYTES: u64 = (MAX_SIDE * MAX_SIDE * 4 * 4 + 1024 * 1024) as u64;
const LAYOUT_SIZE: usize = 36;
const BLOCK_SAMPLES: usize = 2048;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionCacheKey {
    pub seed: u64,
    pub settings_hash: u64,
    pub generator_version: u32,
    pub coord: RegionCoord,
}

impl RegionCacheKey {
    fn header(self) -> [u8; HEADER_SIZE] {
        let mut header = [0; HEADER_SIZE];
        header[..8].copy_from_slice(MAGIC);
        header[8..12].copy_from_slice(&SCHEMA.to_le_bytes());
        header[12..16].copy_from_slice(&self.generator_version.to_le_bytes());
        header[16..24].copy_from_slice(&self.seed.to_le_bytes());
        header[24..32].copy_from_slice(&self.settings_hash.to_le_bytes());
        header[32..40].copy_from_slice(&self.coord.x.to_le_bytes());
        header[40..48].copy_from_slice(&self.coord.z.to_le_bytes());
        header
    }
}

#[derive(Clone, Debug)]
pub struct RegionCache {
    root: PathBuf,
}

impl RegionCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path(&self, key: RegionCacheKey) -> PathBuf {
        self.root
            .join(format!("{:016x}", key.seed))
            .join(format!(
                "v{}-{:016x}",
                key.generator_version, key.settings_hash
            ))
            .join(format!("r_{}_{}.wcr", key.coord.x, key.coord.z))
    }

    pub fn load(&self, key: RegionCacheKey) -> Option<TerrainRegion> {
        self.read(key).ok()
    }

    fn read(&self, key: RegionCacheKey) -> io::Result<TerrainRegion> {
        let file = File::open(self.path(key))?;
        if file.metadata()?.len() > MAX_FILE_BYTES {
            return Err(invalid("compressed region exceeds size limit"));
        }
        let mut file = file.take(MAX_FILE_BYTES + 1);
        let mut header = [0; HEADER_SIZE];
        file.read_exact(&mut header)?;
        if header != key.header() {
            return Err(invalid("region identity or cache format has changed"));
        }
        let mut decoder = zstd::stream::read::Decoder::new(file)?;
        decoder.window_log_max(24)?;
        let region = read_region(key.coord, &mut decoder)?;
        let mut trailing = [0; 1];
        if decoder.read(&mut trailing)? != 0 {
            return Err(invalid("unexpected trailing region data"));
        }
        Ok(region)
    }

    pub fn store(&self, key: RegionCacheKey, region: &TerrainRegion) -> io::Result<()> {
        if region.coord != key.coord {
            return Err(invalid("region coordinates do not match the cache key"));
        }
        let path = self.path(key);
        fs::create_dir_all(path.parent().expect("cache entry has a directory"))?;
        let (temporary, file) = create_temporary(&path)?;
        let result = (|| {
            let mut writer = BufWriter::new(file);
            writer.write_all(&key.header())?;
            let mut encoder = zstd::stream::write::Encoder::new(writer, 3)?;
            encoder.include_checksum(true)?;
            write_region(region, &mut encoder)?;
            let mut writer = encoder.finish()?;
            writer.flush()?;
            let file = writer.into_inner()?;
            if file.metadata()?.len() > MAX_FILE_BYTES {
                return Err(invalid("compressed region exceeds size limit"));
            }
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

struct Layout {
    side: usize,
    active_cells: usize,
    padding_cells: usize,
    spacing_m: f64,
    origin_m: [i64; 2],
}

impl Layout {
    fn len(&self, coord: RegionCoord) -> io::Result<usize> {
        let expected_side = self
            .padding_cells
            .checked_mul(2)
            .and_then(|pad| pad.checked_add(self.active_cells))
            .and_then(|cells| cells.checked_add(1));
        if self.side > MAX_SIDE
            || expected_side != Some(self.side)
            || self.active_cells as f64 * self.spacing_m != REGION_SIZE_M as f64
            || self.padding_cells as f64 * self.spacing_m != REGION_PADDING_M as f64
        {
            return Err(invalid("invalid region grid dimensions"));
        }
        let limit = crate::config::MAX_WORLD_COORD as i64;
        let max_origin = limit - REGION_SIZE_M - 2 * REGION_PADDING_M;
        if [coord.x, coord.z]
            .into_iter()
            .zip(self.origin_m)
            .any(|(axis, origin)| {
                !(-limit..=max_origin).contains(&origin)
                    || axis
                        .checked_mul(REGION_SIZE_M)
                        .and_then(|value| value.checked_sub(REGION_PADDING_M))
                        != Some(origin)
            })
        {
            return Err(invalid("region grid origin does not match its coordinates"));
        }
        Ok(self.side * self.side)
    }

    fn decode(mut reader: impl Read) -> io::Result<Self> {
        let mut bytes = [0; LAYOUT_SIZE];
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            side: u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize,
            active_cells: u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            padding_cells: u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            spacing_m: f64::from_le_bytes(bytes[12..20].try_into().unwrap()),
            origin_m: [
                i64::from_le_bytes(bytes[20..28].try_into().unwrap()),
                i64::from_le_bytes(bytes[28..36].try_into().unwrap()),
            ],
        })
    }

    fn encode(&self, mut writer: impl Write) -> io::Result<()> {
        writer.write_all(&(self.side as u32).to_le_bytes())?;
        writer.write_all(&(self.active_cells as u32).to_le_bytes())?;
        writer.write_all(&(self.padding_cells as u32).to_le_bytes())?;
        writer.write_all(&self.spacing_m.to_le_bytes())?;
        writer.write_all(&self.origin_m[0].to_le_bytes())?;
        writer.write_all(&self.origin_m[1].to_le_bytes())
    }
}

fn read_region(coord: RegionCoord, mut reader: impl Read) -> io::Result<TerrainRegion> {
    let layout = Layout::decode(&mut reader)?;
    let len = layout.len(coord)?;
    Ok(TerrainRegion {
        coord,
        side: layout.side,
        active_cells: layout.active_cells,
        padding_cells: layout.padding_cells,
        spacing_m: layout.spacing_m,
        origin_m: layout.origin_m,
        elevation_m: read_samples(&mut reader, len)?,
        temperature_c: read_samples(&mut reader, len)?,
        precipitation_mm: read_samples(&mut reader, len)?,
        talus_deposit_m: read_samples(&mut reader, len)?,
    })
}

fn write_region(region: &TerrainRegion, mut writer: impl Write) -> io::Result<()> {
    let layout = Layout {
        side: region.side,
        active_cells: region.active_cells,
        padding_cells: region.padding_cells,
        spacing_m: region.spacing_m,
        origin_m: region.origin_m,
    };
    let len = layout.len(region.coord)?;
    layout.encode(&mut writer)?;
    for samples in [
        &region.elevation_m,
        &region.temperature_c,
        &region.precipitation_mm,
        &region.talus_deposit_m,
    ] {
        if samples.len() != len {
            return Err(invalid("region map does not match grid dimensions"));
        }
        write_samples(&mut writer, samples)?;
    }
    Ok(())
}

fn read_samples(mut reader: impl Read, len: usize) -> io::Result<Vec<f32>> {
    let mut values = Vec::with_capacity(len);
    let mut block = [0; 4 * BLOCK_SAMPLES];
    while values.len() < len {
        let count = (len - values.len()).min(BLOCK_SAMPLES);
        let bytes = &mut block[..count * 4];
        reader.read_exact(bytes)?;
        let mut previous = 0;
        for sample in 0..count {
            let delta =
                u32::from_le_bytes(std::array::from_fn(|plane| bytes[plane * count + sample]));
            previous ^= delta;
            let value = f32::from_bits(previous);
            if !value.is_finite() {
                return Err(invalid("non-finite region map value"));
            }
            values.push(value);
        }
    }
    Ok(values)
}

fn write_samples(mut writer: impl Write, values: &[f32]) -> io::Result<()> {
    let mut block = [0; 4 * BLOCK_SAMPLES];
    for batch in values.chunks(BLOCK_SAMPLES) {
        let bytes = &mut block[..batch.len() * 4];
        let mut previous = 0;
        for (sample, &value) in batch.iter().enumerate() {
            if !value.is_finite() {
                return Err(invalid("non-finite region map value"));
            }
            let current = value.to_bits();
            // Nearby elevation and climate values share high bits. Store the XOR
            // bytes in planes so zstd compresses those runs without losing precision.
            for (plane, byte) in (current ^ previous).to_le_bytes().into_iter().enumerate() {
                bytes[plane * batch.len() + sample] = byte;
            }
            previous = current;
        }
        writer.write_all(bytes)?;
    }
    Ok(())
}

fn create_temporary(path: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let serial = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temporary = path.with_extension(format!("tmp.{}.{}", std::process::id(), serial));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestFolder(PathBuf);

    impl Drop for TestFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn entry() -> (TestFolder, RegionCache, RegionCacheKey, TerrainRegion) {
        let folder = TestFolder(std::env::temp_dir().join(format!(
            "worldcraft-wcr-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        )));
        let cache = RegionCache::new(folder.0.clone());
        let coord = RegionCoord { x: -3, z: 2 };
        let key = RegionCacheKey {
            seed: 42,
            settings_hash: 153,
            generator_version: 6,
            coord,
        };
        let side = 89;
        let len = side * side;
        let region = TerrainRegion {
            coord,
            side,
            active_cells: 64,
            padding_cells: 12,
            spacing_m: 16.0,
            origin_m: [-3264, 1856],
            elevation_m: (0..len).map(|index| index as f32 * 3.6 - 16.1).collect(),
            temperature_c: vec![19.6; len],
            precipitation_mm: vec![812.0; len],
            talus_deposit_m: vec![0.025; len],
        };
        (folder, cache, key, region)
    }

    #[test]
    fn negative_region_coordinates_round_trip_all_maps() {
        let (_folder, cache, key, region) = entry();
        assert!(cache.load(key).is_none());
        cache.store(key, &region).unwrap();
        let loaded = cache.load(key).unwrap();
        assert_eq!(loaded.coord, key.coord);
        assert_eq!(loaded.origin_m, region.origin_m);
        assert_eq!(loaded.spacing_m, region.spacing_m);
        assert_eq!(loaded.elevation_m, region.elevation_m);
        assert_eq!(loaded.temperature_c, region.temperature_c);
        assert_eq!(loaded.precipitation_mm, region.precipitation_mm);
        assert_eq!(loaded.talus_deposit_m, region.talus_deposit_m);
    }

    #[test]
    fn foreign_settings_and_corrupt_frames_are_cache_misses() {
        let (_folder, cache, key, region) = entry();
        cache.store(key, &region).unwrap();
        let stale = RegionCacheKey {
            settings_hash: key.settings_hash.wrapping_add(1),
            ..key
        };
        let stale_path = cache.path(stale);
        fs::create_dir_all(stale_path.parent().unwrap()).unwrap();
        fs::copy(cache.path(key), stale_path).unwrap();
        assert!(cache.load(stale).is_none());

        let path = cache.path(key);
        let mut bytes = fs::read(&path).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(path, bytes).unwrap();
        assert!(cache.load(key).is_none());
        cache.store(key, &region).unwrap();
        assert_eq!(cache.load(key).unwrap().elevation_m, region.elevation_m);
    }

    #[test]
    fn rejects_unbounded_grid_and_non_finite_samples() {
        let (_folder, cache, key, mut region) = entry();
        cache.store(key, &region).unwrap();
        let mut bytes = key.header().to_vec();
        let oversized = Layout {
            side: MAX_SIDE + 1,
            active_cells: MAX_SIDE,
            padding_cells: 0,
            spacing_m: 1.0,
            origin_m: [-3 * MAX_SIDE as i64, 2 * MAX_SIDE as i64],
        };
        let mut encoder = zstd::stream::write::Encoder::new(&mut bytes, 3).unwrap();
        oversized.encode(&mut encoder).unwrap();
        encoder.finish().unwrap();
        fs::write(cache.path(key), bytes).unwrap();
        assert!(cache.load(key).is_none());
        region.temperature_c[0] = f32::NAN;
        assert_eq!(
            cache.store(key, &region).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn simultaneous_writes_publish_complete_entries() {
        let (_folder, cache, key, region) = entry();
        cache.store(key, &region).unwrap();
        std::thread::scope(|scope| {
            scope.spawn(|| cache.store(key, &region).unwrap());
            scope.spawn(|| cache.store(key, &region).unwrap());
            assert_eq!(cache.load(key).unwrap().elevation_m, region.elevation_m);
        });
        assert_eq!(cache.load(key).unwrap().elevation_m, region.elevation_m);
        assert_eq!(
            fs::read_dir(cache.path(key).parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }
}
