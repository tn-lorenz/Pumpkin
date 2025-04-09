use pumpkin_util::math::vector2::Vector2;

use crate::{
    chunk::{ChunkData, ChunkSections, SubChunk},
    generation::{generator::GeneratorInit, Seed, WorldGenerator},
};

pub struct VoidGenerator;

impl GeneratorInit for VoidGenerator {
    fn new(_seed: Seed) -> Self {
        Self {}
    }
}

impl WorldGenerator for VoidGenerator {
    fn generate_chunk(&self, at: &Vector2<i32>) -> ChunkData {
        // Create an array of empty SubChunk instances
        // We need to create each SubChunk individually since it doesn't implement Clone
        let mut sections = Vec::with_capacity(24);
        for _ in 0..24 {
            sections.push(SubChunk::default());
        }
        
        // The minimum y level for Minecraft 1.18+ is -64
        let min_y = -64;
        
        ChunkData {
            section: ChunkSections::new(sections.into_boxed_slice(), min_y),
            heightmap: Default::default(),
            position: *at,
            dirty: true,
            block_ticks: Default::default(),
            fluid_ticks: Default::default(),   
        }
    }
}
