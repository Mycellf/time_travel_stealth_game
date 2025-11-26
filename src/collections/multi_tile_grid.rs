use std::mem;

use nalgebra::{Vector2, vector};

use crate::collections::tile_grid::{Empty, TileGrid, TileIndex, TileIndexOffset, TileRect};

#[derive(Clone, Default, Debug)]
pub struct MultiTileGrid<T: Tile, S: TileShape> {
    data: TileGrid<TileEntry<(T, S)>>,
}

#[derive(Clone, Copy, Default, Debug)]
enum TileEntry<T> {
    #[default]
    Empty,
    Origin(T),
    Offset(Vector2<i8>),
}

impl<T: 'static> Empty for TileEntry<T> {
    fn empty() -> &'static Self {
        &TileEntry::Empty
    }

    /// Returns `true` if the tile entry is [`Empty`].
    ///
    /// [`Empty`]: TileEntry::Empty
    fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl<T> TileEntry<T> {
    fn offset(&self) -> Option<TileIndexOffset> {
        match self {
            TileEntry::Empty => None,
            TileEntry::Origin(_) => Some(vector![0, 0]),
            TileEntry::Offset(offset) => Some(offset.map(isize::from)),
        }
    }

    /// Returns `true` if the tile entry is [`Origin`].
    ///
    /// [`Origin`]: TileEntry::Origin
    #[must_use]
    fn is_origin(&self) -> bool {
        matches!(self, Self::Origin(..))
    }

    /// Returns `true` if the tile entry is [`Offset`].
    ///
    /// [`Offset`]: TileEntry::Offset
    #[must_use]
    fn is_offset(&self) -> bool {
        matches!(self, Self::Offset(..))
    }
}

pub trait Tile: 'static {}

impl<T: 'static> Tile for T {}

pub trait TileShape: 'static {
    /// Returned offsets must fit within a `Vector2<i8>`. Returning `[0, 0]` has no effect, as it
    /// is implied to be present no matter what is returned. Although the offsets do not need to be
    /// disjoint, returning fewer duplicates improves performance.
    fn offsets(&self) -> impl Iterator<Item = TileIndexOffset>;

    fn bounds_hint(&self) -> Option<TileRect> {
        None
    }
}

impl<T: Tile, S: TileShape> MultiTileGrid<T, S> {
    pub fn origin_of(&self, index: TileIndex) -> Option<TileIndex> {
        let offset = self.data[index].offset()?;

        Some(index + offset)
    }

    pub fn get_tile(&self, index: TileIndex) -> Option<IndexedTile<'_, T, S>> {
        if let Some(origin) = self.origin_of(index) {
            let TileEntry::Origin((tile, shape)) = &self.data[origin] else {
                unreachable!("Origin index should contain an origin entry")
            };

            Some(IndexedTile {
                tile,
                shape,
                origin,
            })
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, index: TileIndex) -> Option<IndexedTileMut<'_, T, S>> {
        if let Some(origin) = self.origin_of(index) {
            let TileEntry::Origin((tile, shape)) = &mut self.data[origin] else {
                unreachable!("Origin index should contain an origin entry")
            };

            Some(IndexedTileMut {
                tile,
                shape,
                origin,
            })
        } else {
            None
        }
    }

    pub fn insert_tile(
        &mut self,
        origin: TileIndex,
        tile: T,
        shape: S,
    ) -> Result<(), TileInsertError> {
        for offset in shape.offsets().chain([vector![0, 0]]) {
            let index = origin + offset;
            if !self.data[index].is_empty() {
                return Err(TileInsertError::Overlap { conflict: index });
            }
        }

        unsafe { self.insert_tile_unchecked(origin, tile, shape) };

        Ok(())
    }

    pub fn insert_tile_overwriting(
        &mut self,
        origin: TileIndex,
        tile: T,
        shape: S,
    ) -> Vec<RemovedTile<T, S>> {
        let mut removed_tiles = Vec::new();

        for offset in shape.offsets().chain([vector![0, 0]]) {
            let index = origin + offset;
            if let Some(removed_tile) = self.remove_tile(index) {
                removed_tiles.push(removed_tile);
            }
        }

        unsafe { self.insert_tile_unchecked(origin, tile, shape) };

        removed_tiles
    }

    pub unsafe fn insert_tile_unchecked(&mut self, origin: TileIndex, tile: T, shape: S) {
        if let Some(bounds) = shape.bounds_hint() {
            self.data.expand_to_fit_bounds(bounds);
        }

        for offset in shape.offsets() {
            self.data[origin + offset] =
                TileEntry::Offset(-offset.map(|x| i8::try_from(x).expect("Shape is too big")));
        }

        self.data[origin] = TileEntry::Origin((tile, shape));
    }

    pub fn remove_tile(&mut self, index: TileIndex) -> Option<RemovedTile<T, S>> {
        let origin = self.origin_of(index)?;

        let entry = mem::take(&mut self.data[origin]);

        let TileEntry::Origin((tile, shape)) = entry else {
            unreachable!("Origin index should contain an origin entry")
        };

        for offset in shape.offsets() {
            let index = origin + offset;
            self.data[index] = TileEntry::Empty;
        }

        Some(RemovedTile {
            tile,
            shape,
            origin,
        })
    }

    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RemovedTile<T: Tile, S: TileShape> {
    tile: T,
    shape: S,
    origin: TileIndex,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedTile<'a, T: Tile, S: TileShape> {
    tile: &'a T,
    shape: &'a S,
    origin: TileIndex,
}

#[derive(Debug)]
pub struct IndexedTileMut<'a, T: Tile, S: TileShape> {
    tile: &'a mut T,
    shape: &'a S,
    origin: TileIndex,
}

#[derive(Clone, Copy, Debug)]
pub enum TileInsertError {
    Overlap { conflict: TileIndex },
}
