//! Renderer-independent, versioned world generation and durable editing.
pub mod config;
pub mod ecology;
pub mod edit;
pub mod generation;
pub mod mesh;
pub mod persistence;
pub mod scatter;
pub mod streaming;

pub use config::{GeneratorSettings, Preset, GENERATOR_VERSION};
pub use edit::{Brush, BrushKind, EditHistory, Stroke};
pub use generation::{Biome, Generator, Sample};
pub use mesh::{build_mesh, ChunkKey, MeshData};
pub use persistence::{load_world, save_world, WorldFile};
pub use streaming::{ChunkCoord, CHUNK_SIZE};

pub mod erosion;
pub mod geology;

pub mod vista;

pub mod regional;

pub mod export;

pub mod hardware;

pub mod adaptive;
pub mod continuous;
