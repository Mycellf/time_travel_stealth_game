use std::{
    array,
    cmp::Ordering,
    mem,
    ops::{Index, IndexMut},
};

use earcut::Earcut;
use macroquad::{
    color::{Color, colors},
    math::{Vec2, Vec3, Vec3Swizzles, Vec4},
    models::{self, Mesh},
    shapes,
    texture::{self, FilterMode, Texture2D},
    ui::Vertex,
};
use nalgebra::{Point2, Scalar, UnitComplex, UnitVector2, Vector2, point, vector};

use crate::collections::tile_grid::{Empty, TileGrid, TileIndex};

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
    pub const MAXIMUM_RAY_RANGE: f64 = 2048.0;

    pub fn fill_tile(&mut self, index: TileIndex, pixel: Pixel) {
        let corner = index * super::TILE_SIZE;

        for x in corner.x..corner.x + super::TILE_SIZE {
            for y in corner.y..corner.y + super::TILE_SIZE {
                self[point![x, y]] = pixel;
            }
        }
    }

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
                    array::from_fn(|u| {
                        self.grid[point![x + u as isize - 1, y + v as isize - 1]].blocks_light()
                    })
                });

                for &direction in CornerDirection::from_neighborhood(neighborhood) {
                    self.corners.push(Corner {
                        location: point![x as f64, y as f64],
                        direction,
                    })
                }
            }
        }
    }

    pub fn draw(&mut self, solid_color: Color, none_color: Color) {
        if self.grid.bounds().area() == 0 {
            return;
        }

        let solid_color: [u8; 4] = solid_color.into();
        let none_color: [u8; 4] = none_color.into();

        // TODO: This should definitely be cached somewhere.
        let texture = Texture2D::from_rgba8(
            self.grid.bounds().size.x as u16,
            self.grid.bounds().size.y as u16,
            &self
                .grid
                .as_slice()
                .iter()
                .map(|pixel| {
                    if pixel.is_none() {
                        none_color
                    } else {
                        solid_color
                    }
                })
                .flatten()
                .collect::<Vec<_>>(),
        );
        texture.set_filter(FilterMode::Nearest);

        let origin = self.grid.bounds().origin.map(|x| x as f32);

        texture::draw_texture(&texture, origin.x, origin.y, colors::WHITE);

        // let rectangle = Mesh::new_rectangle(
        //     ctx,
        //     DrawMode::Fill(FillOptions::default()),
        //     Rect::one(),
        //     Color::WHITE,
        // )?;
        //
        // for &corner in self.corners() {
        //     let rotation = corner.direction.left_angle();
        //     let origin = corner.location.map(|x| x as f32);
        //
        //     canvas.draw(
        //         &rectangle,
        //         DrawParam {
        //             color: if corner.direction.is_convex() {
        //                 Color::RED
        //             } else {
        //                 Color::BLUE
        //             },
        //             transform: Transform::Values {
        //                 dest: origin.into(),
        //                 rotation,
        //                 scale: vector![0.2, 0.2].into(),
        //                 offset: point![0.0, 0.0].into(),
        //             },
        //             ..Default::default()
        //         },
        //     );
        // }
    }

    pub fn trace_light_from(
        &mut self,
        origin: Point2<f64>,
        angle_range: Option<AngleRange>,
    ) -> LightArea {
        let mut area = LightArea {
            origin,
            rays: Vec::new(),
            range: angle_range,
        };

        let mut unorganized_rays = Vec::new();

        // HACK: Update the corners, then get them without rust thinking we still need a unique
        // pointer to self.
        let _ = self.corners();

        let collision_function = |_, index| self[index].blocks_light();

        if let Some(range) = &area.range {
            let (finish, direction) = raycast(
                collision_function,
                area.origin,
                range.left,
                Self::MAXIMUM_RAY_RANGE,
            );

            unorganized_rays.push(Ray::new(
                finish - area.origin,
                RayPartition::RightEdge,
                direction,
            ));

            let (finish, direction) = raycast(
                collision_function,
                area.origin,
                range.right,
                Self::MAXIMUM_RAY_RANGE,
            );

            unorganized_rays.push(Ray::new(
                finish - area.origin,
                RayPartition::LeftEdge,
                direction,
            ));
        } else {
            // HACK: The lighting system breaks when one of these rays hits a corner, so use an
            // irrational number for their direction.
            use std::f64::consts::PI;

            for direction in [
                UnitVector2::new_normalize(vector![1.0, PI]),
                UnitVector2::new_normalize(vector![-PI, 1.0]),
                UnitVector2::new_normalize(vector![-1.0, -PI]),
                UnitVector2::new_normalize(vector![PI, -1.0]),
            ] {
                let (finish, direction) = raycast(
                    collision_function,
                    area.origin,
                    direction,
                    Self::MAXIMUM_RAY_RANGE,
                );

                unorganized_rays.push(Ray::new(
                    finish - area.origin,
                    RayPartition::None,
                    direction,
                ));
            }
        }

        for corner in &self.corners {
            let offset_to_corner = corner.location - area.origin;

            if !(corner.direction.contains_offset(-offset_to_corner)
                && area
                    .range
                    .is_none_or(|range| range.contains_offset(offset_to_corner)))
            {
                continue;
            }

            let Some(direction_to_corner) = UnitVector2::try_new(offset_to_corner, f64::EPSILON)
            else {
                continue;
            };

            let (finish, direction) = raycast(
                collision_function,
                area.origin,
                direction_to_corner,
                Self::MAXIMUM_RAY_RANGE,
            );

            if (finish - area.origin).magnitude_squared()
                < offset_to_corner.magnitude_squared() - 1e-3
            {
                continue;
            }

            let ray_continues = !(corner.direction.is_concave()
                || corner.direction.contains_offset_strict(-offset_to_corner));

            if !corner.direction.should_skip(-offset_to_corner) {
                unorganized_rays.push(Ray::new(
                    corner.location - area.origin,
                    if corner.direction.is_on_edge(-offset_to_corner) {
                        if corner.direction.is_convex()
                            ^ corner.direction.is_on_left_edge(-offset_to_corner)
                        {
                            RayPartition::Right
                        } else {
                            RayPartition::Left
                        }
                    } else if ray_continues {
                        if corner.direction.is_offset_to_left(-offset_to_corner) {
                            RayPartition::Left
                        } else {
                            RayPartition::Right
                        }
                    } else {
                        RayPartition::None
                    },
                    if ray_continues {
                        Some(RayCollisionNormal::Corner(corner.direction, true))
                    } else {
                        direction
                    },
                ));
            }

            if ray_continues {
                unorganized_rays.push(Ray::new(
                    finish - area.origin,
                    if corner.direction.is_offset_to_left(-offset_to_corner) {
                        RayPartition::Right
                    } else {
                        RayPartition::Left
                    },
                    direction,
                ));
            }
        }

        let reference = match area.range {
            Some(range) => range.left,
            None => UnitVector2::new_normalize(unorganized_rays[0].offset),
        };

        unorganized_rays
            .sort_unstable_by(|&lhs, &rhs| compare_ray_angles(lhs, rhs, reference, 0.0));

        for chunk in unorganized_rays
            .chunk_by(|&lhs, &rhs| compare_ray_angles(lhs, rhs, reference, 1e-6) == Ordering::Equal)
        {
            if chunk.len() <= 1 {
                area.rays.push(chunk[0].into());

                continue;
            }

            let mut shortest = chunk[0];
            let mut longest = chunk[0];

            for &ray in chunk.iter().skip(1) {
                if ray.magnitude < shortest.magnitude {
                    shortest = ray;
                } else if ray.magnitude > longest.magnitude {
                    longest = ray;
                }
            }

            if (shortest.magnitude - longest.magnitude).abs() <= 1e-6 {
                area.rays.push(shortest.into());
                continue;
            }

            let (left, right) = match shortest.partition.cmp(&longest.partition) {
                Ordering::Less => (longest, shortest),
                Ordering::Equal => (shortest, longest),
                Ordering::Greater => (shortest, longest),
            };

            area.rays.push(left.into());
            area.rays.push(right.into());
        }

        area
    }
}

/// Compares the counter clockwise angle from reference to lhs to that of rhs
fn compare_ray_angles(lhs: Ray, rhs: Ray, reference: UnitVector2<f64>, epsilon: f64) -> Ordering {
    let lhs_cos_angle = counter_clockwise_cos_angle(lhs, reference);
    let rhs_cos_angle = counter_clockwise_cos_angle(rhs, reference);

    if (lhs_cos_angle - rhs_cos_angle).abs() <= epsilon {
        Ordering::Equal
    } else {
        lhs_cos_angle.total_cmp(&rhs_cos_angle)
    }
}

fn counter_clockwise_cos_angle(ray: Ray, reference: UnitVector2<f64>) -> f64 {
    let result = cos_angle(ray, reference);
    if reference.perp(&ray.offset) >= -1e-6 {
        -result
    } else {
        result + 3.0
    }
}

fn cos_angle(lhs: Ray, rhs: UnitVector2<f64>) -> f64 {
    lhs.offset.dot(&rhs) / lhs.magnitude
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub location: Point2<f64>,
    pub direction: CornerDirection,
}

#[derive(Clone, Copy, Debug)]
pub enum RayCollisionNormal {
    Wall(WallDirection),
    Corner(CornerDirection, bool),
}

impl RayCollisionNormal {
    pub fn out<T: From<i8> + Scalar>(self) -> Vector2<T> {
        match self {
            RayCollisionNormal::Wall(wall_direction) => wall_direction.out(),
            RayCollisionNormal::Corner(corner_direction, _) => corner_direction.out(),
        }
    }

    pub fn out_angle(self) -> f32 {
        match self {
            RayCollisionNormal::Wall(wall_direction) => wall_direction.out_angle(),
            RayCollisionNormal::Corner(corner_direction, _) => corner_direction.out_angle(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WallDirection {
    East = 0b00,
    North = 0b01,
    West = 0b10,
    South = 0b11,
}

impl WallDirection {
    pub fn out<T: From<i8> + Scalar>(self) -> Vector2<T> {
        match self {
            WallDirection::East => vector![1, 0],
            WallDirection::North => vector![0, -1],
            WallDirection::West => vector![-1, 0],
            WallDirection::South => vector![0, 1],
        }
        .map(T::from)
    }

    pub fn out_angle(self) -> f32 {
        use std::f32::consts::PI;

        match self {
            WallDirection::East => 0.0,
            WallDirection::North => PI / 2.0,
            WallDirection::West => PI,
            WallDirection::South => PI * 3.0 / 2.0,
        }
    }
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

    pub fn from_neighborhood(neighborhood: [[bool; 2]; 2]) -> &'static [Self] {
        match neighborhood {
            [[false, false], [false, false]]
            | [[true, true], [false, false]]
            | [[false, false], [true, true]]
            | [[false, true], [false, true]]
            | [[true, false], [true, false]]
            | [[true, true], [true, true]] => &[],

            [[true, false], [false, false]] => &[CornerDirection::ConvexSouthEast],
            [[false, true], [false, false]] => &[CornerDirection::ConvexSouthWest],
            [[false, false], [true, false]] => &[CornerDirection::ConvexNorthEast],
            [[false, false], [false, true]] => &[CornerDirection::ConvexNorthWest],

            [[true, false], [false, true]] => &[
                CornerDirection::ConcaveNorthEast,
                CornerDirection::ConcaveSouthWest,
            ],
            [[false, true], [true, false]] => &[
                CornerDirection::ConcaveNorthWest,
                CornerDirection::ConcaveSouthEast,
            ],

            [[true, false], [true, true]] => &[CornerDirection::ConcaveNorthEast],
            [[false, true], [true, true]] => &[CornerDirection::ConcaveNorthWest],
            [[true, true], [true, false]] => &[CornerDirection::ConcaveSouthEast],
            [[true, true], [false, true]] => &[CornerDirection::ConcaveSouthWest],
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

    pub fn is_offset_to_left<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        match (self.is_north(), self.is_east()) {
            // Southwest
            (false, false) => offset[0] >= T::default() && offset[1] >= T::default(),
            // Southeast
            (false, true) => offset[0] >= T::default() && offset[1] <= T::default(),
            // Northwest
            (true, false) => offset[0] <= T::default() && offset[1] >= T::default(),
            // Northeast
            (true, true) => offset[0] <= T::default() && offset[1] <= T::default(),
        }
    }

    pub fn is_on_edge<T: PartialOrd + Default + Copy>(self, offset: Vector2<T>) -> bool {
        self.is_on_left_edge(offset) || self.is_on_right_edge(offset)
    }

    pub fn is_on_left_edge<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        match (self.is_north(), self.is_east()) {
            // Southwest
            (false, false) => offset[1] >= T::default() && offset[0] == T::default(),
            // Southeast
            (false, true) => offset[0] >= T::default() && offset[1] == T::default(),
            // Northwest
            (true, false) => offset[0] <= T::default() && offset[1] == T::default(),
            // Northeast
            (true, true) => offset[1] <= T::default() && offset[0] == T::default(),
        }
    }

    pub fn is_on_right_edge<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        match (self.is_north(), self.is_east()) {
            // Southwest
            (false, false) => offset[0] <= T::default() && offset[1] == T::default(),
            // Southeast
            (false, true) => offset[1] >= T::default() && offset[0] == T::default(),
            // Northwest
            (true, false) => offset[1] <= T::default() && offset[0] == T::default(),
            // Northeast
            (true, true) => offset[0] >= T::default() && offset[1] == T::default(),
        }
    }

    pub fn should_skip<T: PartialOrd + Default>(self, offset: Vector2<T>) -> bool {
        match (self.is_north(), self.is_east()) {
            // Southwest
            (false, false) => {
                (offset[0] >= T::default() && offset[1] == T::default())
                    || (offset[1] <= T::default() && offset[0] == T::default())
            }
            // Southeast
            (false, true) => {
                (offset[0] <= T::default() && offset[1] == T::default())
                    || (offset[1] <= T::default() && offset[0] == T::default())
            }
            // Northwest
            (true, false) => {
                (offset[0] >= T::default() && offset[1] == T::default())
                    || (offset[1] >= T::default() && offset[0] == T::default())
            }
            // Northeast
            (true, true) => {
                (offset[0] <= T::default() && offset[1] == T::default())
                    || (offset[1] >= T::default() && offset[0] == T::default())
            }
        }
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

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Pixel {
    None,
    #[default]
    Solid,
}

impl Empty for Pixel {
    fn empty() -> &'static Self {
        &Self::Solid
    }

    fn is_empty(&self) -> bool {
        self == Self::empty()
    }
}

impl Pixel {
    /// Returns `true` if the material kind is [`None`].
    ///
    /// [`None`]: MaterialKind::None
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `false` if the material kind is [`None`].
    ///
    /// [`None`]: MaterialKind::None
    #[must_use]
    pub fn blocks_light(&self) -> bool {
        !self.is_none()
    }

    /// Returns `false` if the material kind is [`None`].
    ///
    /// [`None`]: MaterialKind::None
    #[must_use]
    pub fn blocks_motion(&self) -> bool {
        !self.is_none()
    }
}

#[derive(Clone, Default, Debug)]
pub struct LightArea {
    pub origin: Point2<f64>,
    pub rays: Vec<StoredRay>,
    pub range: Option<AngleRange>,
}

impl LightArea {
    pub const PENETRATION: f32 = 4.0;

    pub fn mesh(&self, color: Color) -> Option<Mesh> {
        let color = color.into();

        let vertices = self
            .rays
            .iter()
            .map(|ray| (self.origin + ray.offset).map(|x| x as f32))
            .chain(self.range.is_some().then(|| self.origin.map(|x| x as f32)))
            .map(|point| Vertex {
                position: Vec3::new(point.x, point.y, 0.0),
                uv: Vec2::ZERO,
                color,
                normal: Vec4::ZERO,
            })
            .collect::<Vec<_>>();

        if vertices.len() >= 3 {
            let mut indices = Vec::new();
            Earcut::new().earcut(
                vertices.iter().map(|x| x.position.xy().into()),
                &[],
                &mut indices,
            );

            Some(Mesh {
                vertices,
                indices,
                texture: None,
            })
        } else {
            None
        }
    }

    pub fn draw(&self, direct_color: Color, wall_color: Color) {
        if let Some(mesh) = self.mesh(direct_color) {
            models::draw_mesh(&mesh);
        }

        for window in self.rays.windows(2) {
            let &[
                StoredRay {
                    offset: left_offset,
                    collision: Some(left_collision),
                },
                StoredRay {
                    offset: right_offset,
                    collision: Some(right_collision),
                },
            ] = window
            else {
                continue;
            };

            let left_out = left_collision.out::<i8>();
            let right_out = right_collision.out::<i8>();

            let x_aligned =
                left_out.x == right_out.x && (left_offset.x - right_offset.x).abs() <= 1e-6;
            let y_aligned =
                left_out.y == right_out.y && (left_offset.y - right_offset.y).abs() <= 1e-6;

            let left_position = (self.origin + left_offset).map(|x| x as f32);
            let right_position = (self.origin + right_offset).map(|x| x as f32);

            if !(x_aligned || y_aligned) {
                continue;
            }

            let (position_1, position_2) = if x_aligned {
                let direction = -left_collision.out::<f32>().x * Self::PENETRATION;

                (left_position + vector![direction, 0.0], right_position)
            } else {
                let direction = -left_collision.out::<f32>().y * Self::PENETRATION;

                (left_position + vector![0.0, direction], right_position)
            };

            let position_min =
                Point2::from(Vector2::from_fn(|i, _| position_1[i].min(position_2[i])));
            let position_max =
                Point2::from(Vector2::from_fn(|i, _| position_1[i].max(position_2[i])));
            let size = position_max - position_min;

            shapes::draw_rectangle(position_min.x, position_min.y, size.x, size.y, wall_color);
        }

        for &StoredRay { offset, collision } in &self.rays {
            if let Some(RayCollisionNormal::Corner(direction, true)) = collision {
                if direction.is_concave() {
                    let position = (self.origin + offset).map(|x| x as f32);
                    shapes::draw_rectangle(
                        position.x - Self::PENETRATION * direction.is_east() as i8 as f32,
                        position.y - Self::PENETRATION * direction.is_south() as i8 as f32,
                        Self::PENETRATION,
                        Self::PENETRATION,
                        wall_color,
                    );
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub offset: Vector2<f64>,
    pub magnitude: f64,
    pub partition: RayPartition,
    pub collision: Option<RayCollisionNormal>,
}

impl Ray {
    pub fn new(
        offset: Vector2<f64>,
        partition: RayPartition,
        collision: Option<RayCollisionNormal>,
    ) -> Self {
        Ray {
            offset,
            magnitude: offset.magnitude(),
            partition,
            collision,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StoredRay {
    pub offset: Vector2<f64>,
    pub collision: Option<RayCollisionNormal>,
}

impl From<Ray> for StoredRay {
    fn from(
        Ray {
            offset, collision, ..
        }: Ray,
    ) -> Self {
        StoredRay { offset, collision }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RayPartition {
    Left,
    LeftEdge,
    None,
    RightEdge,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct AngleRange {
    pub left: UnitVector2<f64>,
    pub right: UnitVector2<f64>,
}

impl AngleRange {
    pub fn from_direction_and_width(direction: UnitVector2<f64>, width: f64) -> AngleRange {
        AngleRange {
            left: UnitComplex::new(-width / 2.0) * direction,
            right: UnitComplex::new(width / 2.0) * direction,
        }
    }

    pub fn contains_offset(&self, offset: Vector2<f64>) -> bool {
        match self.left.perp(&self.right).partial_cmp(&0.0) {
            Some(Ordering::Less) => {
                self.left.perp(&offset) >= -1e-6 || self.right.perp(&offset) <= 1e-6
            }
            Some(Ordering::Equal) => self.left.perp(&offset) >= 0.0,
            Some(Ordering::Greater) => {
                self.left.perp(&offset) >= -1e-6 && self.right.perp(&offset) <= 1e-6
            }

            None => false,
        }
    }
}

#[must_use]
pub fn raycast(
    mut function: impl FnMut(Point2<f64>, TileIndex) -> bool,
    start: Point2<f64>,
    mut direction: UnitVector2<f64>,
    max_distance: f64,
) -> (Point2<f64>, Option<RayCollisionNormal>) {
    const EPSILON: f64 = 1e-6;

    let mut location = start;
    let mut index = index_of_location(location, direction.into_inner());

    let mut on_x_edge = false;
    let mut on_y_edge = false;

    if direction.x.abs() <= EPSILON {
        direction = UnitVector2::new_unchecked(vector![0.0, direction.y.signum()]);
        on_x_edge = true;
    } else if direction.y.abs() <= EPSILON {
        direction = UnitVector2::new_unchecked(vector![direction.x.signum(), 0.0]);
        on_y_edge = true;
    }

    if (location.x.round() - location.x).abs() <= EPSILON {
        location.x = location.x.round();
    } else {
        on_x_edge = false;
    }

    if (location.y.round() - location.y).abs() <= EPSILON {
        location.y = location.y.round();
    } else {
        on_y_edge = false;
    }

    let on_x_edge = on_x_edge;
    let on_y_edge = on_y_edge;

    let mut side_a = if on_x_edge {
        function(location, index + vector![-1, 0])
    } else if on_y_edge {
        function(location, index + vector![0, -1])
    } else {
        false
    };

    let mut side_b = function(location, index);

    if side_a || side_b {
        return (location, None);
    }

    let dir_sign_x = direction.x.signum() as isize;
    let dir_sign_y = direction.y.signum() as isize;

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
            return (start + direction.into_inner() * max_distance, None);
        }

        index = index_of_location(location, direction.into_inner());

        let side_a_before = side_a;
        let side_b_before = side_b;

        let mut side_a_now = false;
        let mut side_b_now = false;
        if on_x_edge || on_y_edge {
            // This branch shouldn't set side_<a/b>_now
            if !side_a {
                if on_x_edge {
                    side_a = function(location, index + vector![-1, 0]);
                } else {
                    side_a = function(location, index + vector![0, -1]);
                }
            }

            if !side_b {
                side_b = function(location, index);
            }
        } else {
            if time_x == time_y {
                side_a_now = function(location, index - vector![dir_sign_x, 0]);
                side_a |= side_a_now;

                side_b_now = function(location, index - vector![0, dir_sign_y]);
                side_b |= side_b_now;
            }

            if function(location, index) {
                return (location, 'direction: {
                    let mut x_direction = time_x < time_y;
                    if time_x == time_y {
                        if side_a_now == side_b_now {
                            break 'direction Some(RayCollisionNormal::Corner(
                                CornerDirection::new(side_a_now, dir_sign_y < 0, dir_sign_x > 0),
                                !side_a_now || !(side_a_before || side_b_before),
                            ));
                        }

                        // If there is also a wall up or down, this is an x-axis wall, else there
                        // is a wall to the left or right and it is a y-axis wall
                        x_direction = side_b_now;
                    }

                    if x_direction {
                        Some(RayCollisionNormal::Wall(if dir_sign_x > 0 {
                            WallDirection::West
                        } else {
                            WallDirection::East
                        }))
                    } else {
                        Some(RayCollisionNormal::Wall(if dir_sign_y > 0 {
                            WallDirection::North
                        } else {
                            WallDirection::South
                        }))
                    }
                });
            }
        }

        if side_a && side_b {
            return (location, {
                if on_y_edge {
                    Some(RayCollisionNormal::Wall(if dir_sign_x > 0 {
                        WallDirection::West
                    } else {
                        WallDirection::East
                    }))
                } else if on_x_edge {
                    Some(RayCollisionNormal::Wall(if dir_sign_y > 0 {
                        WallDirection::North
                    } else {
                        WallDirection::South
                    }))
                } else if side_a_now && side_b_now {
                    Some(RayCollisionNormal::Corner(
                        CornerDirection::new(true, dir_sign_y < 0, dir_sign_x > 0),
                        false,
                    ))
                } else {
                    let (south, west) = match (dir_sign_x, dir_sign_y) {
                        (1, 1) => (!side_a_now, !side_a_now),
                        (1, -1) => (!side_a_now, side_a_now),
                        (-1, 1) => (side_a_now, !side_a_now),
                        (-1, -1) => (side_a_now, side_a_now),
                        _ => unreachable!(),
                    };

                    Some(RayCollisionNormal::Corner(
                        CornerDirection::new(false, south, west),
                        true,
                    ))
                }
            });
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
