//! The only thing the game adapter has to provide.

use crate::geom::Pos;

/// A block state id. `AIR` is 0.
pub type Block = u32;
pub const AIR: Block = 0;

pub trait World {
    fn get(&self, pos: Pos) -> Block;
    fn set(&mut self, pos: Pos, block: Block);
}
