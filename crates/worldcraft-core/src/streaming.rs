use serde::{Deserialize, Serialize};
use crate::config::MAX_WORLD_COORD;
pub const CHUNK_SIZE: f64 = 128.0;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkCoord { pub x: i64, pub z: i64 }
impl ChunkCoord {
    pub fn at(x: f64, z: f64) -> Self {
        Self { x: (x / CHUNK_SIZE).floor() as i64, z: (z / CHUNK_SIZE).floor() as i64 }
    }
    pub fn origin(self) -> [f64; 2] { [self.x as f64 * CHUNK_SIZE, self.z as f64 * CHUNK_SIZE] }
    pub fn distance(self, other: Self) -> i64 { (self.x - other.x).abs().max((self.z - other.z).abs()) }
    pub fn valid(self) -> bool {
        self.origin().iter().all(|v| v.abs() <= MAX_WORLD_COORD - CHUNK_SIZE)
    }
}
pub fn lod_for(coord: ChunkCoord, center: ChunkCoord) -> u8 {
    match coord.distance(center) { 0..=1 => 0, 2..=3 => 1, 4..=6 => 2, _ => 3 }
}
/// Clockwise edges: north (-Z), east (+X), south (+Z), west (-X).
pub fn edge_lods(coord: ChunkCoord, center: ChunkCoord) -> [u8; 4] {
    [lod_for(ChunkCoord { z: coord.z - 1, ..coord }, center),
     lod_for(ChunkCoord { x: coord.x + 1, ..coord }, center),
     lod_for(ChunkCoord { z: coord.z + 1, ..coord }, center),
     lod_for(ChunkCoord { x: coord.x - 1, ..coord }, center)]
}
pub fn wanted(center: ChunkCoord, radius: i64) -> Vec<ChunkCoord> {
    let mut coords = Vec::new();
    for z in -radius..=radius { for x in -radius..=radius {
        let c = ChunkCoord { x: center.x + x, z: center.z + z };
        if c.valid() { coords.push(c); }
    }}
    coords.sort_by_key(|c| ((c.x-center.x).pow(2) + (c.z-center.z).pow(2), c.z, c.x));
    coords
}
