use bevy::prelude::*;

use wow_chunky::chunks::shared::C3Vector;

pub static ADT_SIZE: f32 = 533.33333;
pub static CHUNK_SIZE: f32 = ADT_SIZE / 16.;


/// ADT block coordinates.
/// Ranges from (0, 0) to (64, 64), where (32, 32) is the center.
struct ADTLocation {
    x: u32,
    y: u32,
}

/// Chunk coordinates within an ADT.
/// Ranges from (0, 0) to (16, 16).
struct ChunkLocation {
    x: u32,
    y: u32,
}

/// Helper type to handle converting between Bevy and WoW coordinate systems.
/// Stores positions in WoW format by default.
struct WorldPosition {
    x: f32,
    y: f32,
    z: f32,
}

impl From<C3Vector> for WorldPosition {
    fn from(value: C3Vector) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }
}

impl From<Vec3> for WorldPosition {
    fn from(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: value.z,
            z: value.y,
        }
    }
}