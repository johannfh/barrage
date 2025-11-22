use std::collections::HashMap;

use bevy::prelude::*;

pub const FIELD_SIZE: f32 = 4.0;
pub const CHUNK_SIZE: usize = 16;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE as i32;
pub const CHUNK_SIZE_F32: f32 = CHUNK_SIZE as f32;
pub const CHUNK_HALF_SIZE: Vec2 = Vec2::splat(CHUNK_SIZE_F32 * FIELD_SIZE / 2.0);

#[derive(Component, Debug, Clone, Copy)]
pub struct ChunkEntity {
    position: IVec2,
}

impl ChunkEntity {
    #[inline]
    pub const fn position(&self) -> IVec2 {
        self.position
    }
}

struct ChunkData {
    tiles: [[bool; CHUNK_SIZE]; CHUNK_SIZE],
}

impl ChunkData {
    fn new() -> Self {
        Self {
            tiles: [[false; CHUNK_SIZE]; CHUNK_SIZE],
        }
    }

    fn set(&mut self, local_pos: IVec2, value: bool) {
        self.tiles[local_pos.x as usize][local_pos.y as usize] = value;
    }
}

#[derive(Default, Resource)]
pub struct Map {
    chunks: HashMap<IVec2, ChunkData>,
}

impl Map {
    /// Converts global position to chunk position and local position within that chunk.
    ///
    /// # Arguments
    /// * `pos`: The global position as an IVec2.
    ///
    /// # Returns
    /// - `(chunk_pos, local_pos)`: A tuple where `chunk_pos` is the position of the chunk
    ///   containing the global position, and `local_pos` is the position within that chunk.
    #[inline]
    pub const fn global_to_chunk(pos: IVec2) -> (IVec2, IVec2) {
        let chunk_pos = IVec2::new(
            pos.x.div_euclid(CHUNK_SIZE_I32),
            pos.y.div_euclid(CHUNK_SIZE_I32),
        );
        let local_pos = IVec2::new(
            pos.x.rem_euclid(CHUNK_SIZE_I32),
            pos.y.rem_euclid(CHUNK_SIZE_I32),
        );
        (chunk_pos, local_pos)
    }

    #[inline]
    pub const fn chunk_to_global(chunk_pos: IVec2, local_pos: IVec2) -> IVec2 {
        IVec2::new(
            chunk_pos.x * CHUNK_SIZE_I32 + local_pos.x,
            chunk_pos.y * CHUNK_SIZE_I32 + local_pos.y,
        )
    }

    pub fn create_chunk(&mut self, pos: IVec2, commands: &mut Commands) {
        if self.chunks.insert(pos, ChunkData::new()).is_some() {
            // for now, we just panic if chunk exists
            panic!("Chunk at position {:?} already exists!", pos);
        }
        commands.spawn(ChunkEntity { position: pos });
    }

    pub fn try_place(&mut self, pos: IVec2, occlusion_map: &[IVec2]) -> bool {
        // check occlusion
        for offset in occlusion_map {
            let check_pos = pos + offset;
            let chunk_pos = IVec2::new(
                check_pos.x.div_euclid(CHUNK_SIZE_I32),
                check_pos.y.div_euclid(CHUNK_SIZE_I32),
            );
            let local_pos = IVec2::new(
                check_pos.x.rem_euclid(CHUNK_SIZE_I32),
                check_pos.y.rem_euclid(CHUNK_SIZE_I32),
            );
            if let Some(chunk) = self.chunks.get(&chunk_pos) {
                if chunk.tiles[local_pos.x as usize][local_pos.y as usize] {
                    // cannot place, field occupied
                    return false;
                } else {
                    // field is free -> continue checking
                }
            } else {
                // Chunk does not exist -> not loaded yet -> placement fails
                // TODO: handle error and chunk loading properly
                return false;
            }
        }

        // placement possible
        for offset in occlusion_map {
            let place_pos = pos + offset;
            let chunk_pos = IVec2::new(
                place_pos.x.div_euclid(CHUNK_SIZE_I32),
                place_pos.y.div_euclid(CHUNK_SIZE_I32),
            );
            let local_pos = IVec2::new(
                place_pos.x.rem_euclid(CHUNK_SIZE_I32),
                place_pos.y.rem_euclid(CHUNK_SIZE_I32),
            );
            let chunk = self
                .chunks
                .get_mut(&chunk_pos)
                .expect("Chunk must exist here; we checked before");
            assert!(!chunk.tiles[local_pos.x as usize][local_pos.y as usize]);
            chunk.tiles[local_pos.x as usize][local_pos.y as usize] = true;
        }
        // placement successful
        true
    }

    /// Checks if a global position is occupied.
    /// This returns true if the position is occupied or if the chunk is not loaded.
    pub fn is_occupied(&self, chunk_pos: IVec2, local_pos: IVec2) -> bool {
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            chunk.tiles[local_pos.x as usize][local_pos.y as usize]
        } else {
            // Chunk does not exist -> not loaded yet -> consider occupied
            true
        }
    }
}
