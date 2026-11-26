//! Planned changes (the ghost), applying them, undo and redo.

use std::collections::BTreeMap;

use crate::geom::Pos;
use crate::world::{Block, World};

/// What a tool would change. The adapter draws this as the ghost.
pub type Plan = BTreeMap<Pos, Block>;

#[derive(Default)]
pub struct History {
    undo: Vec<Vec<(Pos, Block, Block)>>,
    redo: Vec<Vec<(Pos, Block, Block)>>,
}

const MAX_UNDO: usize = 64;

impl History {
    /// Writes the plan and returns how many blocks changed.
    pub fn apply(&mut self, world: &mut dyn World, plan: &Plan) -> usize {
        let changes: Vec<_> = plan
            .iter()
            .map(|(&p, &new)| (p, world.get(p), new))
            .filter(|&(_, old, new)| old != new)
            .collect();
        for &(p, _, new) in &changes {
            world.set(p, new);
        }
        let n = changes.len();
        if n > 0 {
            self.undo.push(changes);
            if self.undo.len() > MAX_UNDO {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        n
    }

    pub fn undo(&mut self, world: &mut dyn World) -> bool {
        let Some(changes) = self.undo.pop() else { return false };
        for &(p, old, _) in &changes {
            world.set(p, old);
        }
        self.redo.push(changes);
        true
    }

    pub fn redo(&mut self, world: &mut dyn World) -> bool {
        let Some(changes) = self.redo.pop() else { return false };
        for &(p, _, new) in &changes {
            world.set(p, new);
        }
        self.undo.push(changes);
        true
    }
}
