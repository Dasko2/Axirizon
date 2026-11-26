use std::collections::HashMap;

use axirizon_core::geom::{Dir, Pos};
use axirizon_core::session::{Hit, Input, Session};
use axirizon_core::world::{Block, World, AIR};

#[derive(Default)]
struct Mem(HashMap<Pos, Block>);

impl World for Mem {
    fn get(&self, p: Pos) -> Block {
        *self.0.get(&p).unwrap_or(&AIR)
    }
    fn set(&mut self, p: Pos, b: Block) {
        self.0.insert(p, b);
    }
}

fn p(x: i32, y: i32, z: i32) -> Pos {
    Pos::new(x, y, z)
}

fn hit(x: i32) -> Option<Hit> {
    Some(Hit { pos: p(x, 0, 0), side: Dir::Up })
}

fn setup(slot: usize) -> (Mem, Session) {
    let mut w = Mem::default();
    w.set(p(0, 0, 0), 1);
    w.set(p(1, 0, 0), 2);
    let mut s = Session::default();
    s.handle(&mut w, Input::Slot(slot));
    s.handle(&mut w, Input::LeftClick(hit(0)));
    s.handle(&mut w, Input::RightClick(hit(1)));
    (w, s)
}

fn scroll(steps: i32) -> Input {
    Input::Scroll { steps, look: Dir::East, lock: None }
}

fn row(w: &Mem, n: i32) -> Vec<Block> {
    (0..n).map(|x| w.get(p(x, 0, 0))).collect()
}

#[test]
fn move_previews_then_applies_and_undoes() {
    let (mut w, mut s) = setup(0);
    s.handle(&mut w, scroll(1));
    assert_eq!(row(&w, 3), [1, 2, 0], "ghost must not touch the world");
    assert!(!s.ghost().is_empty());
    s.handle(&mut w, Input::RightClick(None));
    assert_eq!(row(&w, 3), [0, 1, 2]);
    assert!(s.ghost().is_empty());
    s.handle(&mut w, Input::Undo);
    assert_eq!(row(&w, 3), [1, 2, 0]);
    s.handle(&mut w, Input::Redo);
    assert_eq!(row(&w, 3), [0, 1, 2]);
}

#[test]
fn clone_keeps_source() {
    let (mut w, mut s) = setup(1);
    s.handle(&mut w, scroll(2));
    s.handle(&mut w, Input::RightClick(None));
    assert_eq!(row(&w, 4), [1, 2, 1, 2]);
}

#[test]
fn stack_repeats_whole_selection() {
    let (mut w, mut s) = setup(2);
    s.handle(&mut w, scroll(2));
    s.handle(&mut w, Input::RightClick(None));
    assert_eq!(row(&w, 6), [1, 2, 1, 2, 1, 2]);
}

#[test]
fn smear_copies_every_step() {
    let (mut w, mut s) = setup(3);
    s.handle(&mut w, scroll(2));
    s.handle(&mut w, Input::RightClick(None));
    assert_eq!(row(&w, 4), [1, 1, 1, 2]);
}

#[test]
fn extrude_and_shrink_a_face() {
    let mut w = Mem::default();
    w.set(p(0, 0, 0), 1);
    w.set(p(1, 0, 0), 1);
    let mut s = Session::default();
    s.handle(&mut w, Input::Slot(4));
    s.handle(&mut w, Input::RightClick(hit(0)));
    assert_eq!(w.get(p(1, 1, 0)), 1);
    let top = Some(Hit { pos: p(0, 1, 0), side: Dir::Up });
    s.handle(&mut w, Input::LeftClick(top));
    assert_eq!(w.get(p(1, 1, 0)), AIR);
    assert_eq!(w.get(p(1, 0, 0)), 1);
}

#[test]
fn erase_clears_selection() {
    let (mut w, mut s) = setup(5);
    s.handle(&mut w, Input::RightClick(None));
    assert_eq!(row(&w, 2), [0, 0]);
}

#[test]
fn symmetry_mirrors_the_ghost() {
    let (mut w, mut s) = setup(1);
    s.handle(&mut w, Input::Slot(6));
    s.handle(&mut w, Input::RightClick(hit(5)));
    s.handle(&mut w, Input::Slot(1));
    s.handle(&mut w, scroll(2));
    assert_eq!(s.ghost().get(&p(8, 0, 0)), Some(&1));
    assert_eq!(s.ghost().get(&p(7, 0, 0)), Some(&2));
}

#[test]
fn rotate_turns_the_ghost() {
    let (mut w, mut s) = setup(1);
    s.handle(&mut w, Input::Rotate);
    assert_eq!(s.ghost().get(&p(0, 0, 1)), Some(&2));
}
