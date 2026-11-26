//! Positions, axes, directions and box selections.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn add(self, o: Pos) -> Pos {
        Pos::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(self, o: Pos) -> Pos {
        Pos::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn scale(self, n: i32) -> Pos {
        Pos::new(self.x * n, self.y * n, self.z * n)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    East,
    West,
    Up,
    Down,
    South,
    North,
}

impl Dir {
    pub const ALL: [Dir; 6] = [Dir::East, Dir::West, Dir::Up, Dir::Down, Dir::South, Dir::North];

    pub fn offset(self) -> Pos {
        match self {
            Dir::East => Pos::new(1, 0, 0),
            Dir::West => Pos::new(-1, 0, 0),
            Dir::Up => Pos::new(0, 1, 0),
            Dir::Down => Pos::new(0, -1, 0),
            Dir::South => Pos::new(0, 0, 1),
            Dir::North => Pos::new(0, 0, -1),
        }
    }

    pub fn axis(self) -> Axis {
        match self {
            Dir::East | Dir::West => Axis::X,
            Dir::Up | Dir::Down => Axis::Y,
            Dir::South | Dir::North => Axis::Z,
        }
    }
}

/// Inclusive box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub min: Pos,
    pub max: Pos,
}

impl Selection {
    pub fn new(a: Pos, b: Pos) -> Self {
        Self {
            min: Pos::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Pos::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    pub fn size(&self) -> Pos {
        self.max.sub(self.min).add(Pos::new(1, 1, 1))
    }

    pub fn volume(&self) -> u64 {
        let s = self.size();
        s.x as u64 * s.y as u64 * s.z as u64
    }

    pub fn positions(self) -> impl Iterator<Item = Pos> {
        (self.min.y..=self.max.y).flat_map(move |y| {
            (self.min.z..=self.max.z)
                .flat_map(move |z| (self.min.x..=self.max.x).map(move |x| Pos::new(x, y, z)))
        })
    }
}
