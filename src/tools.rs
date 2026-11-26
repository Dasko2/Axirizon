//! The builder tools of the hotbar. Each one only computes a `Plan`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::edit::Plan;
use crate::geom::{Dir, Pos, Selection};
use crate::world::{Block, World, AIR};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Move,
    Clone,
    Stack,
    Smear,
    Extrude,
    Erase,
    Symmetry,
}

impl Tool {
    /// Hotbar order.
    pub const HOTBAR: [Tool; 7] = [
        Tool::Move,
        Tool::Clone,
        Tool::Stack,
        Tool::Smear,
        Tool::Extrude,
        Tool::Erase,
        Tool::Symmetry,
    ];
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Flags {
    /// Never overwrite a block that is already there.
    pub keep_existing: bool,
    /// Copy air too, so it carves into the destination.
    pub copy_air: bool,
}

/// Largest selection or plan, to keep the game responsive.
pub const MAX_BLOCKS: u64 = 262_144;
/// Largest face one extrude step works on.
const MAX_FACE: usize = 4_096;

/// Blocks of a selection, relative to its minimum corner.
#[derive(Clone, Debug)]
pub struct Clipboard {
    pub size: Pos,
    pub blocks: BTreeMap<Pos, Block>,
}

impl Clipboard {
    pub fn copy(world: &dyn World, sel: Selection) -> Self {
        Self {
            size: sel.size(),
            blocks: sel.positions().map(|p| (p.sub(sel.min), world.get(p))).collect(),
        }
    }

    /// Quarter turn clockwise seen from above. Positions only: the facing
    /// of stairs, logs and the like is not rotated yet.
    pub fn rotate_y(&self) -> Self {
        let blocks = self
            .blocks
            .iter()
            .map(|(p, &b)| (Pos::new(self.size.z - 1 - p.z, p.y, p.x), b))
            .collect();
        Self { size: Pos::new(self.size.z, self.size.y, self.size.x), blocks }
    }

    /// Mirror along the axis of `dir`. Positions only, as for rotation.
    pub fn flip(&self, dir: Dir) -> Self {
        let blocks = self
            .blocks
            .iter()
            .map(|(&p, &b)| {
                let mut q = p;
                match dir.axis() {
                    crate::geom::Axis::X => q.x = self.size.x - 1 - p.x,
                    crate::geom::Axis::Y => q.y = self.size.y - 1 - p.y,
                    crate::geom::Axis::Z => q.z = self.size.z - 1 - p.z,
                }
                (q, b)
            })
            .collect();
        Self { size: self.size, blocks }
    }
}

fn paste(plan: &mut Plan, world: &dyn World, clip: &Clipboard, origin: Pos, flags: Flags) {
    for (&rel, &block) in &clip.blocks {
        if block == AIR && !flags.copy_air {
            continue;
        }
        let at = origin.add(rel);
        let current = plan.get(&at).copied().unwrap_or_else(|| world.get(at));
        if flags.keep_existing && current != AIR {
            continue;
        }
        plan.insert(at, block);
    }
}

pub fn erase(sel: Selection) -> Plan {
    sel.positions().map(|p| (p, AIR)).collect()
}

/// Move: clear the source, then paste at `source.min + offset`.
pub fn move_to(world: &dyn World, sel: Selection, clip: &Clipboard, offset: Pos, flags: Flags) -> Plan {
    let mut plan = erase(sel);
    paste(&mut plan, world, clip, sel.min.add(offset), flags);
    plan
}

pub fn clone_to(world: &dyn World, sel: Selection, clip: &Clipboard, offset: Pos, flags: Flags) -> Plan {
    let mut plan = Plan::new();
    paste(&mut plan, world, clip, sel.min.add(offset), flags);
    plan
}

/// Stack: `count` is a number of whole copies per axis, signed.
pub fn stack(world: &dyn World, sel: Selection, clip: &Clipboard, count: Pos, flags: Flags) -> Plan {
    let step = clip.size;
    let mut plan = Plan::new();
    let range = |n: i32| n.min(0)..=n.max(0);
    let total = [count.x, count.y, count.z]
        .iter()
        .map(|n| n.unsigned_abs() as u64 + 1)
        .product::<u64>();
    if total.saturating_mul(sel.volume()) > MAX_BLOCKS {
        return plan;
    }
    for x in range(count.x) {
        for y in range(count.y) {
            for z in range(count.z) {
                if (x, y, z) != (0, 0, 0) {
                    let offset = Pos::new(x * step.x, y * step.y, z * step.z);
                    paste(&mut plan, world, clip, sel.min.add(offset), flags);
                }
            }
        }
    }
    plan
}

/// Smear: a copy at every one-block step on the way to `offset`.
pub fn smear(world: &dyn World, sel: Selection, clip: &Clipboard, offset: Pos, flags: Flags) -> Plan {
    let steps = offset.x.abs().max(offset.y.abs()).max(offset.z.abs());
    let mut plan = Plan::new();
    if (steps as u64).saturating_mul(sel.volume()) > MAX_BLOCKS {
        return plan;
    }
    for i in 1..=steps {
        let at = Pos::new(offset.x * i / steps, offset.y * i / steps, offset.z * i / steps);
        paste(&mut plan, world, clip, sel.min.add(at), flags);
    }
    plan
}

/// Connected blocks of the same kind as `start` whose `face` is exposed.
fn face(world: &dyn World, start: Pos, face: Dir) -> Vec<Pos> {
    let kind = world.get(start);
    let n = face.offset();
    let exposed = |p: Pos| world.get(p) == kind && world.get(p.add(n)) == AIR;
    if kind == AIR || !exposed(start) {
        return Vec::new();
    }
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let mut out = Vec::new();
    while let Some(p) = queue.pop_front() {
        out.push(p);
        for d in Dir::ALL.iter().filter(|d| d.axis() != face.axis()) {
            let q = p.add(d.offset());
            if seen.len() < MAX_FACE && exposed(q) && seen.insert(q) {
                queue.push_back(q);
            }
        }
    }
    out
}

pub fn extrude(world: &dyn World, hit: Pos, side: Dir) -> Plan {
    let kind = world.get(hit);
    face(world, hit, side).into_iter().map(|p| (p.add(side.offset()), kind)).collect()
}

pub fn shrink(world: &dyn World, hit: Pos, side: Dir) -> Plan {
    face(world, hit, side).into_iter().map(|p| (p, AIR)).collect()
}

/// Mirror planes through the centre of a block.
#[derive(Clone, Copy, Debug)]
pub struct Symmetry {
    pub center: Pos,
    pub x: bool,
    pub z: bool,
}

impl Symmetry {
    /// Adds the mirrored changes. Original changes win on overlap.
    pub fn extend(&self, plan: &mut Plan) {
        for (enabled, mirror_x) in [(self.x, true), (self.z, false)] {
            if !enabled {
                continue;
            }
            let copies: Vec<_> = plan
                .iter()
                .map(|(&p, &b)| {
                    let mut q = p;
                    if mirror_x {
                        q.x = 2 * self.center.x - p.x;
                    } else {
                        q.z = 2 * self.center.z - p.z;
                    }
                    (q, b)
                })
                .collect();
            for (p, b) in copies {
                plan.entry(p).or_insert(b);
            }
        }
    }
}
