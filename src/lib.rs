//! Axirizon: builder tools with a ghost preview.
//! `edit`, `geom`, `session`, `tools` and `world` do not know the game;
//! this file connects them to Horizon's mod API.
//!
//! Input is read on the client, the world is edited on the integrated
//! server's tick, and the client draws what the server last computed.
//! That is why this only works in single player.

pub mod edit;
pub mod geom;
pub mod session;
pub mod tools;
pub mod world;

#[cfg(test)]
mod tests;

use std::sync::{Mutex, MutexGuard};

use mod_api::*;

use crate::geom::{Axis, Dir, Pos, Selection};
use crate::session::{Hit, Input, Session};
use crate::tools::Tool;
use crate::world::{Block, World, AIR};

/// Most ghost blocks drawn at once.
const MAX_DRAWN: usize = 680;

#[derive(Default)]
struct Client {
    active: bool,
    queue: Vec<Input>,
    lock: Option<Axis>,
    look: Option<Dir>,
    ticks: u32,
    tool: usize,
}

#[derive(Default)]
struct Preview {
    selection: Option<Selection>,
    /// Ghost position, and whether the block would become air.
    ghost: Vec<(Pos, bool)>,
}

#[derive(Default)]
struct Server {
    level: Option<LevelRef>,
    session: Session,
    /// Local ids of the block states seen. Index 0 is air.
    states: Vec<BlockState>,
}

#[derive(Default)]
struct Mod {
    client: Mutex<Client>,
    preview: Mutex<Preview>,
    server: Mutex<Server>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

struct Level<'a> {
    host: &'a mut dyn ServerHost,
    level: LevelRef,
    states: std::cell::RefCell<&'a mut Vec<BlockState>>,
    air: BlockState,
}

impl World for Level<'_> {
    fn get(&self, p: Pos) -> Block {
        let Some(state) = self.level.block_state(&*self.host, [p.x, p.y, p.z]) else { return AIR };
        if state == self.air {
            return AIR;
        }
        let mut states = self.states.borrow_mut();
        let i = states.iter().position(|&s| s == state).unwrap_or_else(|| {
            states.push(state);
            states.len() - 1
        });
        i as Block
    }

    fn set(&mut self, p: Pos, block: Block) {
        let state = if block == AIR {
            self.air
        } else {
            match self.states.borrow().get(block as usize) {
                Some(&s) => s,
                None => return,
            }
        };
        let _ = self.level.set_block(self.host, [p.x, p.y, p.z], state, BlockUpdateFlags::CLIENTS);
    }
}

fn dir_from_face(face: [i32; 3]) -> Option<Dir> {
    Dir::ALL.into_iter().find(|d| {
        let o = d.offset();
        [o.x, o.y, o.z] == face
    })
}

fn dir_from_look(yaw: f32, pitch: f32) -> Dir {
    if pitch > 45.0 {
        return Dir::Down;
    }
    if pitch < -45.0 {
        return Dir::Up;
    }
    match ((yaw / 90.0).round() as i32).rem_euclid(4) {
        0 => Dir::South,
        1 => Dir::West,
        2 => Dir::North,
        _ => Dir::East,
    }
}

fn target(host: &dyn ClientHost) -> Option<Hit> {
    match host.crosshair_target()? {
        CrosshairTarget::Block { pos, hit, .. } => {
            Some(Hit { pos: Pos::new(pos.x, pos.y, pos.z), side: dir_from_face(hit.face)? })
        }
        _ => None,
    }
}

fn cube(p: Pos, grow: f64) -> [[f64; 3]; 8] {
    let (a, b) = (-grow, 1.0 + grow);
    let (x, y, z) = (p.x as f64, p.y as f64, p.z as f64);
    std::array::from_fn(|i| {
        [
            x + if i & 1 == 0 { a } else { b },
            y + if i & 2 == 0 { a } else { b },
            z + if i & 4 == 0 { a } else { b },
        ]
    })
}

const FACES: [[usize; 4]; 6] =
    [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
const EDGES: [[usize; 2]; 12] = [
    [0, 1], [2, 3], [4, 5], [6, 7], [0, 2], [1, 3], [4, 6], [5, 7], [0, 4], [1, 5], [2, 6], [3, 7],
];

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if !host.playing() {
            return;
        }
        let key: String = event.layout_key.as_ref().map(|k| k.to_string()).unwrap_or_else(|| event.key.to_string());
        let physical = event.key.to_string();
        let name = key.rsplit('.').next().unwrap_or("");
        let down = !matches!(event.action, KeyAction::Release);
        let press = matches!(event.action, KeyAction::Press);
        let mut c = lock(&self.client);
        if name == "b" && press {
            c.active = !c.active;
            c.lock = None;
            c.queue.push(Input::Cancel);
            let text = if c.active { "Axirizon active (1-7 : outils)" } else { "Axirizon desactive" };
            drop(c);
            let _ = host.show_action_bar(Text::literal(text));
            return;
        }
        if !c.active {
            return;
        }
        let axis = match name {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        };
        // Bit 2 of the modifiers is Control.
        let control = event.modifiers & 2 != 0;
        if let Some(a) = axis {
            c.lock = (down && !control).then_some(a);
        }
        if !press {
            return;
        }
        // The digit row, whatever the keyboard layout prints on it.
        if let Some(n) = physical.strip_prefix("key.keyboard.").and_then(|d| d.parse::<usize>().ok()) {
            if (1..=Tool::HOTBAR.len()).contains(&n) {
                c.tool = n - 1;
                c.queue.push(Input::Slot(n - 1));
            }
        }
        let look = c.look.unwrap_or(Dir::North);
        match name {
            "z" if control => c.queue.push(Input::Undo),
            "y" if control => c.queue.push(Input::Redo),
            "r" if control => c.queue.push(Input::Rotate),
            "f" if control => c.queue.push(Input::Flip { look }),
            "backspace" => c.queue.push(Input::Cancel),
            _ => {}
        }
    }

    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        if !host.playing() || !lock(&self.client).active || !(0..=1).contains(&event.button) {
            return Flow::Continue;
        }
        if event.pressed {
            let hit = target(host);
            let input = if event.button == 0 { Input::LeftClick(hit) } else { Input::RightClick(hit) };
            lock(&self.client).queue.push(input);
        }
        Flow::Cancel
    }

    fn mouse_scrolling(&self, host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
        if !host.playing() {
            return Flow::Continue;
        }
        let mut c = lock(&self.client);
        if !c.active {
            return Flow::Continue;
        }
        let steps = event.accumulated[1];
        if steps != 0 {
            let look = c.look.unwrap_or(Dir::North);
            let lock = c.lock;
            c.queue.push(Input::Scroll { steps, look, lock });
        }
        Flow::Cancel
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        let look = host.camera().map(|c| dir_from_look(c.yaw, c.pitch));
        let message = {
            let mut c = lock(&self.client);
            c.look = look;
            c.ticks = c.ticks.wrapping_add(1);
            (c.active && c.ticks % 20 == 0).then(|| format!("Axirizon : {:?}", Tool::HOTBAR[c.tool]))
        };
        if let Some(text) = message {
            let _ = host.show_action_bar(Text::literal(text));
        }
    }

    fn server_tick(&self, host: &mut dyn ServerHost, _: &mut ServerTickEvent<'_>) {
        let inputs = std::mem::take(&mut lock(&self.client).queue);
        if inputs.is_empty() {
            return;
        }
        // The overworld only, for now.
        let Some(level) = host.level("minecraft:overworld") else { return };
        let Some(air) = host.block_state_id("minecraft:air") else { return };
        let mut server = lock(&self.server);
        // Another world: forget the selection and the undo history.
        if server.level != Some(level) {
            *server = Server { level: Some(level), states: vec![air], ..Server::default() };
        }
        let Server { session, states, .. } = &mut *server;
        let mut world = Level { host, level, states: std::cell::RefCell::new(states), air };
        for input in inputs {
            session.handle(&mut world, input);
        }
        let mut preview = lock(&self.preview);
        preview.selection = session.selection();
        preview.ghost = session.ghost().iter().take(MAX_DRAWN).map(|(&p, &b)| (p, b == AIR)).collect();
    }

    fn submit_custom_geometry(&self, _: &dyn ClientHost, event: &mut SubmitCustomGeometryEvent<'_>) {
        if !lock(&self.client).active {
            return;
        }
        let preview = lock(&self.preview);
        if let Some(s) = preview.selection {
            let (lo, hi) = (cube(s.min, 0.01), cube(s.max, 0.01));
            let v: [[f64; 3]; 8] = std::array::from_fn(|i| {
                std::array::from_fn(|k| if i & (1 << k) == 0 { lo[0][k] } else { hi[7][k] })
            });
            let edges = EDGES.map(|e| WorldLine::new(v[e[0]], v[e[1]], 0xFFFFD020, 3.0, WorldDepth::ThroughWalls));
            let _ = event.draw.lines(&edges);
        }
        let mut quads = Vec::new();
        let mut lines = Vec::new();
        for &(p, removed) in &preview.ghost {
            // Blue for a block that would appear, red for one that would go.
            let (fill, edge) = if removed { (0x30FF4040, 0xFFFF4040) } else { (0x6040C0FF, 0xFF80E0FF) };
            let v = cube(p, 0.002);
            quads.extend(FACES.iter().map(|f| WorldQuad::new(f.map(|i| v[i]), fill, WorldDepth::Tested)));
            lines.extend(EDGES.iter().map(|e| WorldLine::new(v[e[0]], v[e[1]], edge, 1.0, WorldDepth::Tested)));
        }
        // A refused batch (view capacity) only hides that part of the ghost.
        for batch in quads.chunks(600) {
            let _ = event.draw.quads(batch);
        }
        for batch in lines.chunks(240) {
            let _ = event.draw.lines(batch);
        }
    }
}
