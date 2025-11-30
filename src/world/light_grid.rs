use std::{cmp::Ordering, mem};

use ggez::{
    Context,
    graphics::{Canvas, DrawParam, Image, ImageFormat, Transform},
};
use nalgebra::{Point2, UnitVector2, Vector2, point, vector};

use crate::collections::tile_grid::{TileGrid, TileIndex};

type Tile = Option<MaterialKind>;

#[derive(Clone, Default, Debug)]
pub struct LightGrid {
    pub grid: TileGrid<Tile>,
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

impl LightGrid {
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) {
        if self.grid.bounds().area() == 0 {
            return;
        }

        // TODO: This should definitely be cached somewhere.
        let image = Image::from_pixels(
            ctx,
            &self
                .grid
                .as_slice()
                .iter()
                .map(|pixel| match pixel {
                    Some(_) => [0, 0, 0, 255],
                    None => [0; 4],
                })
                .flatten()
                .collect::<Vec<_>>(),
            ImageFormat::Rgba8UnormSrgb,
            self.grid.bounds().size.x as u32,
            self.grid.bounds().size.y as u32,
        );

        let origin = self.grid.bounds().origin.map(|x| x as f32);

        canvas.draw(
            &image,
            DrawParam {
                transform: Transform::Values {
                    dest: origin.into(),
                    rotation: 0.0,
                    scale: vector![1.0, 1.0].into(),
                    offset: point![0.0, 0.0].into(),
                },
                ..Default::default()
            },
        );
    }

    pub fn raycast_with(
        &self,
        mut function: impl FnMut(Point2<f64>, Tile) -> bool,
        start: Point2<f64>,
        direction: UnitVector2<f64>,
        max_distance: f64,
    ) -> Point2<f64> {
        const EPSILON: f64 = 1e-6;

        let mut location = start;
        let mut index = Self::index_of_location(location, direction.into_inner());

        if function(location, self.grid[index]) {
            return location;
        }

        let on_x_edge = direction.x == 0.0 && location.x.rem_euclid(1.0) == 0.0;
        let on_y_edge = direction.y == 0.0 && location.y.rem_euclid(1.0) == 0.0;

        if on_x_edge && function(location, self.grid[index + vector![-1, 0]])
            || on_y_edge && function(location, self.grid[index + vector![0, -1]])
        {
            return location;
        }

        let dir_sign_x = if direction.x > 0.0 { 1 } else { -1 };
        let dir_sign_y = if direction.y > 0.0 { 1 } else { -1 };

        let max_distance_squared = (max_distance - EPSILON).powi(2);

        loop {
            let mut time_x =
                (1.0 - (location.x * direction.x.signum()).rem_euclid(1.0)) / direction.x.abs();
            let time_y =
                (1.0 - (location.y * direction.y.signum()).rem_euclid(1.0)) / direction.y.abs();

            if (time_x - time_y).abs() < EPSILON {
                time_x = time_y;
            }

            let time_x = time_x;

            match time_x.partial_cmp(&time_y) {
                Some(Ordering::Less) => {
                    Self::move_in_direction(&mut location.x, direction.x);
                    location.y += time_x * direction.y;
                }
                Some(Ordering::Equal) => {
                    Self::move_in_direction(&mut location.x, direction.x);
                    Self::move_in_direction(&mut location.y, direction.y);
                }
                Some(Ordering::Greater) => {
                    location.x += time_y * direction.x;
                    Self::move_in_direction(&mut location.y, direction.y);
                }
                None => unreachable!(),
            }

            if (start - location).magnitude_squared() >= max_distance_squared {
                return start + direction.into_inner() * max_distance;
            }

            match time_x.partial_cmp(&time_y) {
                Some(Ordering::Less) => {
                    if on_y_edge && function(location, self.grid[index + vector![dir_sign_x, -1]]) {
                        return location;
                    }
                }
                Some(Ordering::Equal) => {
                    if function(location, self.grid[index + vector![dir_sign_x, 0]])
                        || function(location, self.grid[index + vector![0, dir_sign_y]])
                    {
                        return location;
                    }
                }
                Some(Ordering::Greater) => {
                    if on_x_edge && function(location, self.grid[index + vector![-1, dir_sign_y]]) {
                        return location;
                    }
                }
                None => unreachable!(),
            }

            index = Self::index_of_location(location, direction.into_inner());
            if function(location, self.grid[index]) {
                return location;
            }
        }
    }

    fn move_in_direction(location: &mut f64, direction: f64) {
        if direction > 0.0 {
            *location = location.floor() + 1.0;
        } else {
            *location = location.ceil() - 1.0;
        }
    }

    fn index_of_location(location: Point2<f64>, direction: Vector2<f64>) -> Point2<isize> {
        Vector2::from_fn(|i, _| {
            (if direction[i] >= 0.0 {
                location[i].floor()
            } else {
                location[i].ceil() - 1.0
            } as isize)
        })
        .into()
    }
}
