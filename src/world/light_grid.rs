use std::mem;

use nalgebra::{Point2, UnitVector2, Vector2};

use crate::collections::tile_grid::{TileGrid, TileIndex};

#[derive(Clone, Debug)]
pub struct LightGrid {
    pub grid: TileGrid<Option<MaterialKind>>,
    pub corners: TileGrid<Vec<Corner>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub location: TileIndex,
    pub direction: CornerDirection,
}

#[derive(Clone, Copy, Debug)]
pub enum CornerDirection {
    ConvexNorthEast = 0b000,
    ConvexNorthWest = 0b001,
    ConvexSouthEast = 0b010,
    ConvexSouthWest = 0b011,
    ConcaveNorthEast = 0b100,
    ConcaveNorthWest = 0b101,
    ConcaveSouthEast = 0b110,
    ConcaveSouthWest = 0b111,
}

impl CornerDirection {
    pub const CONVEX_CONCAVE_MASK: u8 = 0b100;
    pub const NORTH_SOUTH_MASK: u8 = 0b010;
    pub const EAST_WEST_MASK: u8 = 0b001;
    pub const DIRECTION_MASK: u8 = Self::NORTH_SOUTH_MASK | Self::EAST_WEST_MASK;

    pub fn new(concave: bool, south: bool, west: bool) -> Self {
        unsafe {
            mem::transmute::<u8, CornerDirection>(
                concave as u8 * Self::CONVEX_CONCAVE_MASK
                    | south as u8 * Self::NORTH_SOUTH_MASK
                    | west as u8 * Self::EAST_WEST_MASK,
            )
        }
    }

    pub fn is_concave(self) -> bool {
        self as u8 & Self::CONVEX_CONCAVE_MASK == Self::CONVEX_CONCAVE_MASK
    }

    pub fn is_convex(self) -> bool {
        !self.is_concave()
    }

    pub fn is_south(self) -> bool {
        self as u8 & Self::NORTH_SOUTH_MASK == Self::NORTH_SOUTH_MASK
    }

    pub fn is_north(self) -> bool {
        !self.is_south()
    }

    pub fn is_west(self) -> bool {
        self as u8 & Self::EAST_WEST_MASK == Self::EAST_WEST_MASK
    }

    pub fn is_east(self) -> bool {
        !self.is_west()
    }

    pub fn contains_offset<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        let horizontal_ok = if self.is_east() {
            offset[0] >= T::default()
        } else {
            offset[0] <= T::default()
        };

        let vertical_ok = if self.is_north() {
            offset[1] <= T::default()
        } else {
            offset[1] >= T::default()
        };

        if self.is_convex() {
            horizontal_ok || vertical_ok
        } else {
            horizontal_ok && vertical_ok
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MaterialKind {
    Solid,
    Mirror,
}

#[derive(Clone, Debug)]
pub struct LightArea {
    pub origin: Point2<f64>,
    pub rays: Vec<Vector2<f64>>,
    pub range: Option<AngleRange>,
}

#[derive(Clone, Copy, Debug)]
pub struct AngleRange {
    pub left: UnitVector2<f64>,
    pub right: UnitVector2<f64>,
}
