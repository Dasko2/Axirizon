//! Turns player input into tool actions. The adapter forwards input here
//! and draws `selection()` and `ghost()` every frame.

use crate::edit::{History, Plan};
use crate::geom::{Axis, Dir, Pos, Selection};
use crate::tools::{self, Clipboard, Flags, Symmetry, Tool, MAX_BLOCKS};
use crate::world::World;

/// The block under the crosshair and the face being looked at.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub pos: Pos,
    pub side: Dir,
}

#[derive(Clone, Copy, Debug)]
pub enum Input {
    /// Hotbar slot, from 0.
    Slot(usize),
    LeftClick(Option<Hit>),
    RightClick(Option<Hit>),
    /// `steps` > 0 pushes away. `look` is where the player faces. `lock` is
    /// the held X, Y or Z key.
    Scroll { steps: i32, look: Dir, lock: Option<Axis> },
    Rotate,
    Flip { look: Dir },
    Undo,
    Redo,
    Cancel,
}

pub struct Session {
    pub tool: Tool,
    pub flags: Flags,
    pub symmetry: Option<Symmetry>,
    first: Option<Pos>,
    second: Option<Pos>,
    clip: Option<Clipboard>,
    offset: Pos,
    ghost: Plan,
    history: History,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            tool: Tool::Move,
            flags: Flags::default(),
            symmetry: None,
            first: None,
            second: None,
            clip: None,
            offset: Pos::default(),
            ghost: Plan::new(),
            history: History::default(),
        }
    }
}

impl Default for Pos {
    fn default() -> Self {
        Pos::new(0, 0, 0)
    }
}

impl Session {
    pub fn selection(&self) -> Option<Selection> {
        Some(Selection::new(self.first?, self.second?))
    }

    /// Blocks to draw as the ghost.
    pub fn ghost(&self) -> &Plan {
        &self.ghost
    }

    fn reset_preview(&mut self) {
        self.clip = None;
        self.offset = Pos::default();
        self.ghost.clear();
    }

    pub fn handle(&mut self, world: &mut dyn World, input: Input) {
        match input {
            Input::Slot(i) => {
                if let Some(&tool) = Tool::HOTBAR.get(i) {
                    self.tool = tool;
                    self.reset_preview();
                }
            }
            Input::LeftClick(Some(hit)) => match self.tool {
                Tool::Extrude => self.commit(world, tools::shrink(world, hit.pos, hit.side)),
                Tool::Symmetry => self.symmetry = None,
                _ => {
                    self.first = Some(hit.pos);
                    self.reset_preview();
                }
            },
            Input::RightClick(hit) => self.right_click(world, hit),
            Input::LeftClick(None) => {}
            Input::Scroll { steps, look, lock } => self.scroll(world, steps, look, lock),
            Input::Rotate => self.transform(world, |c| c.rotate_y()),
            Input::Flip { look } => self.transform(world, |c| c.flip(look)),
            Input::Undo => {
                self.history.undo(world);
                self.reset_preview();
            }
            Input::Redo => {
                self.history.redo(world);
                self.reset_preview();
            }
            Input::Cancel => {
                self.first = None;
                self.second = None;
                self.reset_preview();
            }
        }
    }

    fn right_click(&mut self, world: &mut dyn World, hit: Option<Hit>) {
        match (self.tool, hit) {
            (Tool::Extrude, Some(h)) => {
                let plan = tools::extrude(world, h.pos, h.side);
                self.commit(world, plan);
            }
            // Each right click cycles: X, then Z, then both.
            (Tool::Symmetry, Some(h)) => {
                let (x, z) = match self.symmetry {
                    Some(s) if s.center == h.pos && s.x && !s.z => (false, true),
                    Some(s) if s.center == h.pos && !s.x && s.z => (true, true),
                    _ => (true, false),
                };
                self.symmetry = Some(Symmetry { center: h.pos, x, z });
            }
            (Tool::Extrude | Tool::Symmetry, None) => {}
            // A ghost is showing: confirm it.
            _ if self.clip.is_some() => {
                let plan = std::mem::take(&mut self.ghost);
                // After a move, the selection follows the blocks.
                if self.tool == Tool::Move {
                    self.first = self.first.map(|p| p.add(self.offset));
                    self.second = self.second.map(|p| p.add(self.offset));
                }
                self.history.apply(world, &plan);
                self.reset_preview();
            }
            // Erase: a right click once both corners exist confirms.
            (Tool::Erase, _) if self.selection().is_some() => {
                if let Some(sel) = self.selection() {
                    self.commit(world, tools::erase(sel));
                }
            }
            (_, Some(h)) => {
                let fits = self
                    .first
                    .map_or(true, |a| Selection::new(a, h.pos).volume() <= MAX_BLOCKS);
                if fits {
                    self.second = Some(h.pos);
                }
            }
            (_, None) => {}
        }
    }

    fn commit(&mut self, world: &mut dyn World, mut plan: Plan) {
        if let Some(s) = self.symmetry {
            s.extend(&mut plan);
        }
        self.history.apply(world, &plan);
    }

    fn scroll(&mut self, world: &dyn World, steps: i32, look: Dir, lock: Option<Axis>) {
        if !matches!(self.tool, Tool::Move | Tool::Clone | Tool::Stack | Tool::Smear) {
            return;
        }
        let Some(sel) = self.selection() else { return };
        if sel.volume() > MAX_BLOCKS {
            return;
        }
        let delta = match lock {
            None => look.offset().scale(steps),
            Some(axis) => {
                // Keep the sign of the view when it is on the locked axis.
                let sign = if look.axis() == axis {
                    let o = look.offset();
                    o.x + o.y + o.z
                } else {
                    1
                };
                let unit = match axis {
                    Axis::X => Pos::new(1, 0, 0),
                    Axis::Y => Pos::new(0, 1, 0),
                    Axis::Z => Pos::new(0, 0, 1),
                };
                unit.scale(sign * steps)
            }
        };
        if self.clip.is_none() {
            self.clip = Some(Clipboard::copy(world, sel));
        }
        self.offset = self.offset.add(delta);
        self.refresh(world);
    }

    fn transform(&mut self, world: &dyn World, f: impl Fn(&Clipboard) -> Clipboard) {
        if !matches!(self.tool, Tool::Move | Tool::Clone) {
            return;
        }
        let Some(sel) = self.selection() else { return };
        if sel.volume() > MAX_BLOCKS {
            return;
        }
        let clip = self.clip.take().unwrap_or_else(|| Clipboard::copy(world, sel));
        self.clip = Some(f(&clip));
        self.refresh(world);
    }

    fn refresh(&mut self, world: &dyn World) {
        let (Some(sel), Some(clip)) = (self.selection(), self.clip.as_ref()) else { return };
        let (o, f) = (self.offset, self.flags);
        let mut plan = match self.tool {
            Tool::Move => tools::move_to(world, sel, clip, o, f),
            Tool::Clone => tools::clone_to(world, sel, clip, o, f),
            Tool::Stack => tools::stack(world, sel, clip, o, f),
            Tool::Smear => tools::smear(world, sel, clip, o, f),
            _ => Plan::new(),
        };
        if let Some(s) = self.symmetry {
            s.extend(&mut plan);
        }
        self.ghost = plan;
    }
}
