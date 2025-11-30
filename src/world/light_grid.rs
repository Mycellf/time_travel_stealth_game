use std::{
    array,
    cmp::Ordering,
    mem,
    ops::{Index, IndexMut},
};

use ggez::{
    Context, GameResult,
    graphics::{
        Canvas, Color, DrawMode, DrawParam, FillOptions, Image, ImageFormat, Mesh, Rect, Transform,
    },
};
use nalgebra::{Point2, Scalar, UnitVector2, Vector2, point, vector};

use crate::collections::tile_grid::{TileGrid, TileIndex};

pub type Pixel = Option<MaterialKind>;

#[derive(Clone, Default, Debug)]
pub struct LightGrid {
    grid: TileGrid<Pixel>,
    updated: bool,
    corners: Vec<Corner>,
}

impl Index<TileIndex> for LightGrid {
    type Output = Pixel;

    fn index(&self, index: TileIndex) -> &Self::Output {
        &self.grid[index]
    }
}

impl IndexMut<TileIndex> for LightGrid {
    fn index_mut(&mut self, index: TileIndex) -> &mut Self::Output {
        self.updated = true;
        &mut self.grid[index]
    }
}

impl LightGrid {
    pub fn corners(&mut self) -> &[Corner] {
        if self.updated {
            self.updated = false;
            self.regenerate_corners();
        }

        &self.corners
    }

    fn regenerate_corners(&mut self) {
        self.corners.clear();

        let bounds = self.grid.bounds();

        for x in bounds.left()..bounds.right() + 2 {
            for y in bounds.top()..bounds.bottom() + 2 {
                let neighborhood = array::from_fn(|v| {
                    array::from_fn(|u| self.grid[point![x + u as isize - 1, y + v as isize - 1]])
                });

                for &direction in CornerDirection::from_neighborhood(neighborhood) {
                    self.corners.push(Corner {
                        location: point![x, y],
                        direction,
                    })
                }
            }
        }
    }

    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if self.grid.bounds().area() == 0 {
            return Ok(());
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

        let rectangle = Mesh::new_rectangle(
            ctx,
            DrawMode::Fill(FillOptions::default()),
            Rect::one(),
            Color::WHITE,
        )?;

        for &corner in self.corners() {
            let rotation = corner.direction.left_angle();
            let origin = corner.location.map(|x| x as f32);

            canvas.draw(
                &rectangle,
                DrawParam {
                    color: if corner.direction.is_convex() {
                        Color::RED
                    } else {
                        Color::BLUE
                    },
                    transform: Transform::Values {
                        dest: origin.into(),
                        rotation,
                        scale: vector![0.2, 0.2].into(),
                        offset: point![0.0, 0.0].into(),
                    },
                    ..Default::default()
                },
            );
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Corner {
    pub location: TileIndex,
    pub direction: CornerDirection,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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

    pub fn from_neighborhood(neighborhood: [[Pixel; 2]; 2]) -> &'static [Self] {
        match neighborhood {
            [[None, None], [None, None]]
            | [[Some(_), Some(_)], [None, None]]
            | [[None, None], [Some(_), Some(_)]]
            | [[None, Some(_)], [None, Some(_)]]
            | [[Some(_), None], [Some(_), None]]
            | [[Some(_), Some(_)], [Some(_), Some(_)]] => &[],

            [[Some(_), None], [None, None]] => &[CornerDirection::ConvexSouthEast],
            [[None, Some(_)], [None, None]] => &[CornerDirection::ConvexSouthWest],
            [[None, None], [Some(_), None]] => &[CornerDirection::ConvexNorthEast],
            [[None, None], [None, Some(_)]] => &[CornerDirection::ConvexNorthWest],

            [[Some(_), None], [None, Some(_)]] => &[
                CornerDirection::ConcaveNorthEast,
                CornerDirection::ConcaveSouthWest,
            ],
            [[None, Some(_)], [Some(_), None]] => &[
                CornerDirection::ConcaveNorthWest,
                CornerDirection::ConcaveSouthEast,
            ],

            [[Some(_), None], [Some(_), Some(_)]] => &[CornerDirection::ConcaveNorthEast],
            [[None, Some(_)], [Some(_), Some(_)]] => &[CornerDirection::ConcaveNorthWest],
            [[Some(_), Some(_)], [Some(_), None]] => &[CornerDirection::ConcaveSouthEast],
            [[Some(_), Some(_)], [None, Some(_)]] => &[CornerDirection::ConcaveSouthWest],
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
        let (horizontal_ok, vertical_ok) = self.filter_offsets(offset);

        if self.is_convex() {
            horizontal_ok || vertical_ok
        } else {
            horizontal_ok && vertical_ok
        }
    }

    pub fn contains_offset_strict<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        let (horizontal_ok, vertical_ok) = self.filter_offsets(offset);

        horizontal_ok && vertical_ok
    }

    fn filter_offsets<T: PartialOrd + Default>(self, offset: Vector2<T>) -> (bool, bool) {
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

        (horizontal_ok, vertical_ok)
    }

    pub fn out<T: From<i8> + Scalar>(self) -> Vector2<T> {
        match self {
            CornerDirection::ConcaveNorthEast | CornerDirection::ConvexNorthEast => vector![1, -1],
            CornerDirection::ConcaveNorthWest | CornerDirection::ConvexNorthWest => vector![-1, -1],
            CornerDirection::ConcaveSouthEast | CornerDirection::ConvexSouthEast => vector![1, 1],
            CornerDirection::ConcaveSouthWest | CornerDirection::ConvexSouthWest => vector![-1, 1],
        }
        .map(T::from)
    }

    pub fn out_angle(self) -> f32 {
        use std::f32::consts::PI;

        match self {
            CornerDirection::ConcaveNorthEast | CornerDirection::ConvexNorthEast => PI * 7.0 / 4.0,
            CornerDirection::ConcaveNorthWest | CornerDirection::ConvexNorthWest => PI * 5.0 / 4.0,
            CornerDirection::ConcaveSouthEast | CornerDirection::ConvexSouthEast => PI * 1.0 / 4.0,
            CornerDirection::ConcaveSouthWest | CornerDirection::ConvexSouthWest => PI * 3.0 / 4.0,
        }
    }

    pub fn left_angle(self) -> f32 {
        use std::f32::consts::PI;

        match self {
            CornerDirection::ConcaveNorthEast | CornerDirection::ConvexNorthEast => PI * 3.0 / 2.0,
            CornerDirection::ConcaveNorthWest | CornerDirection::ConvexNorthWest => PI,
            CornerDirection::ConcaveSouthEast | CornerDirection::ConvexSouthEast => 0.0,
            CornerDirection::ConcaveSouthWest | CornerDirection::ConvexSouthWest => PI * 1.0 / 2.0,
        }
    }

    pub fn right_angle(self) -> f32 {
        use std::f32::consts::PI;

        match self {
            CornerDirection::ConcaveNorthEast | CornerDirection::ConvexNorthEast => 0.0,
            CornerDirection::ConcaveNorthWest | CornerDirection::ConvexNorthWest => PI * 3.0 / 2.0,
            CornerDirection::ConcaveSouthEast | CornerDirection::ConvexSouthEast => PI * 1.0 / 2.0,
            CornerDirection::ConcaveSouthWest | CornerDirection::ConvexSouthWest => PI,
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

pub fn raycast_with(
    mut function: impl FnMut(Point2<f64>, TileIndex) -> bool,
    start: Point2<f64>,
    direction: UnitVector2<f64>,
    max_distance: f64,
) -> (Point2<f64>, bool) {
    const EPSILON: f64 = 1e-6;

    let mut location = start;
    let mut index = index_of_location(location, direction.into_inner());

    let on_x_edge = direction.x == 0.0 && location.x.rem_euclid(1.0) == 0.0;
    let on_y_edge = direction.y == 0.0 && location.y.rem_euclid(1.0) == 0.0;

    let a = if on_x_edge {
        function(location, index + vector![-1, 0])
    } else if on_y_edge {
        function(location, index + vector![0, -1])
    } else {
        true
    };

    let b = function(location, index);

    if a && b {
        return (location, true);
    }

    let mut last_a = a;
    let mut last_b = b;

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
                move_in_direction(&mut location.x, direction.x);
                location.y += time_x * direction.y;
            }
            Some(Ordering::Equal) => {
                move_in_direction(&mut location.x, direction.x);
                move_in_direction(&mut location.y, direction.y);
            }
            Some(Ordering::Greater) => {
                location.x += time_y * direction.x;
                move_in_direction(&mut location.y, direction.y);
            }
            None => unreachable!(),
        }

        if (start - location).magnitude_squared() >= max_distance_squared {
            return (start + direction.into_inner() * max_distance, false);
        }

        if time_x == time_y
            && function(location, index + vector![dir_sign_x, 0])
            && function(location, index + vector![0, dir_sign_y])
        {
            return (location, true);
        }

        index = index_of_location(location, direction.into_inner());
        if !last_a {
            last_a = if on_x_edge {
                function(location, index + vector![-1, 0])
            } else if on_y_edge {
                function(location, index + vector![0, -1])
            } else {
                true
            };
        }

        if !last_b {
            last_b = function(location, index);
        }

        if last_a && last_b {
            return (location, true);
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
