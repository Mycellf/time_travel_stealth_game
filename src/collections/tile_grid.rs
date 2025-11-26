use std::{
    mem,
    ops::{Index, IndexMut},
};

use ggez::graphics::Rect;
use nalgebra::{Point2, Vector2, point, vector};

pub type TileIndex = Point2<isize>;
pub type TileIndexOffset = Vector2<isize>;

#[derive(Clone, Debug)]
pub struct TileGrid<T: Empty> {
    bounds: TileRect,
    tiles: Box<[T]>,
}

pub trait Empty: Default + 'static {
    /// The value returned by this should match the value of default for this type.
    fn empty() -> &'static Self;
}

impl<T: 'static> Empty for Option<T> {
    fn empty() -> &'static Self {
        &None
    }
}

impl<T: Empty> Default for TileGrid<T> {
    fn default() -> Self {
        Self {
            bounds: TileRect::default(),
            tiles: Box::new([]),
        }
    }
}

impl<T: Empty> Index<TileIndex> for TileGrid<T> {
    type Output = T;

    fn index(&self, index: TileIndex) -> &Self::Output {
        let Some(index) = self.linear_index_of(index) else {
            return T::empty();
        };

        &self.tiles[index]
    }
}

impl<T: Empty> IndexMut<TileIndex> for TileGrid<T> {
    fn index_mut(&mut self, index: TileIndex) -> &mut Self::Output {
        self.expand_to_fit_index(index);

        let index = self
            .linear_index_of(index)
            .expect("Tile index should be present");

        &mut self.tiles[index]
    }
}

impl<T: Empty> TileGrid<T> {
    pub fn bounds(&self) -> TileRect {
        self.bounds
    }

    pub fn as_slice(&self) -> &[T] {
        &self.tiles
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        &mut self.tiles
    }

    fn linear_index_of(&self, index: TileIndex) -> Option<usize> {
        let tile_offset = index - self.bounds.origin;
        let tile_offset = vector![
            usize::try_from(tile_offset.x).ok()?,
            usize::try_from(tile_offset.y).ok()?,
        ];

        (tile_offset.x < self.bounds.size.x && tile_offset.y < self.bounds.size.y)
            .then(|| tile_offset.x + tile_offset.y * self.bounds.size.x)
    }

    /// Returns whether or not any expansion occurred
    pub fn expand_to_fit_index(&mut self, index: TileIndex) -> bool {
        let mut bounds = self.bounds;
        let expanded = bounds.expand_to_include_index(index, self.bounds.size / 2);

        if expanded {
            self.set_bounds(bounds);
        }

        expanded
    }

    /// Returns whether or not any expansion occurred
    pub fn expand_to_fit_bounds(&mut self, bounds: TileRect) -> bool {
        let mut bounds = self.bounds;
        let expanded = bounds.expand_to_include_bounds(bounds, self.bounds.size / 2);

        if expanded {
            self.set_bounds(bounds);
        }

        expanded
    }

    pub fn set_bounds(&mut self, bounds: TileRect) {
        let offset = bounds.origin - self.bounds.origin;

        self.tiles = (0..bounds.area())
            .map(|i| {
                let new_index = vector![i % bounds.size.x, i / bounds.size.x,];
                let old_index =
                    Vector2::from_fn(|i, _| new_index[i].wrapping_add_signed(offset[i]));

                if old_index.x < self.bounds.size.x && old_index.y < self.bounds.size.y {
                    mem::take(&mut self.tiles[old_index.x + old_index.y * self.bounds.size.x])
                } else {
                    T::default()
                }
            })
            .collect();

        self.bounds = bounds;
    }

    pub fn get_disjoint_mut<const N: usize>(
        &mut self,
        indexes: [TileIndex; N],
    ) -> Option<[&mut T; N]> {
        for &index in &indexes {
            self.expand_to_fit_index(index);
        }

        let linear_indexes = indexes.map(|index| {
            self.linear_index_of(index)
                .expect("Should have expanded to fit all indexes")
        });

        self.tiles.get_disjoint_mut(linear_indexes).ok()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TileRect {
    pub origin: TileIndex,
    pub size: Vector2<usize>,
}

impl Default for TileRect {
    fn default() -> Self {
        Self {
            origin: point![0, 0],
            size: vector![0, 0],
        }
    }
}

impl TileRect {
    pub fn from_rect_inclusive(rect: Rect) -> TileRect {
        TileRect::from_rect(rect, f32::floor, f32::ceil)
    }

    pub fn from_rect_exclusive(rect: Rect) -> TileRect {
        TileRect::from_rect(rect, f32::ceil, f32::floor)
    }

    pub fn from_rect_round_to_min(rect: Rect) -> TileRect {
        TileRect::from_rect(rect, f32::floor, f32::floor)
    }

    fn from_rect(rect: Rect, min_fn: impl Fn(f32) -> f32, max_fn: impl Fn(f32) -> f32) -> TileRect {
        let min_corner = point![min_fn(rect.left()) as isize, min_fn(rect.top()) as isize];
        let max_corner = point![
            max_fn(rect.right()) as isize,
            max_fn(rect.bottom()) as isize
        ];

        TileRect {
            origin: min_corner,
            size: (max_corner - min_corner).map(|x| (x + 1).max(0) as usize),
        }
    }

    pub fn min_corner(&self) -> TileIndex {
        self.origin
    }

    pub fn max_corner(&self) -> TileIndex {
        Vector2::from_fn(|i, _| self.origin[i].checked_add_unsigned(self.size[i]).unwrap()).into()
    }

    pub fn left(&self) -> isize {
        self.origin.x
    }

    pub fn right(&self) -> isize {
        self.origin.x.checked_add_unsigned(self.size.x).unwrap() - 1
    }

    pub fn top(&self) -> isize {
        self.origin.y
    }

    pub fn bottom(&self) -> isize {
        self.origin.y.checked_add_unsigned(self.size.y).unwrap() - 1
    }

    pub fn intersects(&self, rhs: &TileRect) -> bool {
        self.left() <= rhs.right()
            && rhs.left() <= self.right()
            && self.top() <= rhs.bottom()
            && rhs.top() <= self.bottom()
    }

    pub fn contains_point(&self, point: TileIndex) -> bool {
        self.left() <= point.x
            && self.right() >= point.x
            && self.top() <= point.y
            && self.bottom() >= point.y
    }

    pub fn contains(&self, rhs: &TileRect) -> bool {
        self.left() <= rhs.left()
            && self.right() >= rhs.right()
            && self.top() <= rhs.top()
            && self.bottom() >= rhs.bottom()
    }

    pub fn area(&self) -> usize {
        self.size.x * self.size.y
    }

    pub fn is_empty(&self) -> bool {
        self.size.x == 0 || self.size.y == 0
    }

    pub fn expand_to_include_index(
        &mut self,
        index: TileIndex,
        minimum_nonzero_expansion: Vector2<usize>,
    ) -> bool {
        self.expand_to_include_bounds(
            TileRect {
                origin: index,
                size: vector![1, 1],
            },
            minimum_nonzero_expansion,
        )
    }

    pub fn expand_to_include_bounds(
        &mut self,
        bounds: TileRect,
        minimum_nonzero_expansion: Vector2<usize>,
    ) -> bool {
        let mut expanded = false;
        let min_expansion = minimum_nonzero_expansion;

        if self.is_empty() {
            expanded = true;
            *self = bounds;
        }

        let origin: Point2<_> = Vector2::from_fn(|i, _| {
            if bounds.origin[i] < self.origin[i] {
                expanded = true;
                bounds.origin[i].min(self.origin[i].saturating_sub(min_expansion[i] as isize))
            } else {
                self.origin[i]
            }
        })
        .into();

        let end_corner = self.max_corner();
        let bounds_end_corner = bounds.max_corner();

        let end_corner: Point2<_> = Vector2::from_fn(|i, _| {
            if bounds_end_corner[i] > end_corner[i] {
                expanded = true;
                bounds_end_corner[i].max(end_corner[i].saturating_add(min_expansion[i] as isize))
            } else {
                end_corner[i]
            }
        })
        .into();

        self.origin = origin;
        self.size = Vector2::from_fn(|i, _| origin[i].abs_diff(end_corner[i]));

        expanded
    }
}
