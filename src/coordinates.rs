use bevy::prelude::*;

use wow_chunky::chunks::shared::C3Vector;

pub static ADT_SIZE: f32 = 533.33333;

pub static CHUNK_SIZE: f32 = ADT_SIZE / 16.;
pub static CHUNK_LEEWAY: f32 = 0.001;

/// ADT block coordinates.
/// Ranges from (0, 0) to (64, 64), where (32, 32) is the center.
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ADTPosition {
    pub x: u32,
    pub y: u32,
}

impl From<&WorldPosition> for ADTPosition {
    fn from(position: &WorldPosition) -> Self {
        Self {
            x: ((17066.66656 - position.x) / ADT_SIZE).floor() as u32,
            y: ((17066.66656 - position.y) / ADT_SIZE).floor() as u32,
        }
    }
}

/// Chunk coordinates within an ADT.
/// Ranges from (0, 0) to (16, 16).
#[derive(Debug, Eq, Hash, PartialEq)]
pub struct ChunkPosition {
    pub x: i32,
    pub y: i32,
}

impl From<&WorldPosition> for ChunkPosition {
    fn from(position: &WorldPosition) -> Self {
        let x = if position.x >= 0.0 {
            (((position.x / CHUNK_SIZE) + CHUNK_LEEWAY).floor()) as i32
        } else {
            (((position.x / CHUNK_SIZE) - CHUNK_LEEWAY).ceil()) as i32
        };

        let y = if position.y >= 0.0 {
            (((position.y / CHUNK_SIZE) + CHUNK_LEEWAY).floor()) as i32
        } else {
            (((position.y / CHUNK_SIZE) - CHUNK_LEEWAY).ceil()) as i32
        };

        Self {
            x,
            y,
        }
    }
}

/// Helper type to handle converting between Bevy and WoW coordinate systems.
/// Stores positions in WoW format by default (Z = up).
#[derive(Debug, PartialEq)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<C3Vector> for WorldPosition {
    fn from(position: C3Vector) -> Self {
        Self {
            x: position.x,
            y: position.y,
            z: position.z,
        }
    }
}

impl From<Vec3> for WorldPosition {
    fn from(position: Vec3) -> Self {
        Self {
            x: position.x,
            y: position.z,
            z: position.y,
        }
    }
}