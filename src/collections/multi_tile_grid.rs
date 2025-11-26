use nalgebra::{Vector2, vector};

use crate::collections::tile_grid::{Empty, TileGrid};

pub struct MultiTileGrid<T: 'static> {
    data: TileGrid<TileEntry<T>>,
}

#[derive(Clone, Copy, Default, Debug)]
enum TileEntry<T> {
    #[default]
    Empty,
    Source(T),
    Offset(Vector2<i8>),
}

impl<T: 'static> Empty for TileEntry<T> {
    fn empty() -> &'static Self {
        &TileEntry::Empty
    }
}

impl<T> TileEntry<T> {
    fn offset(&self) -> Option<Vector2<isize>> {
        match self {
            TileEntry::Empty => None,
            TileEntry::Source(_) => Some(vector![0, 0]),
            TileEntry::Offset(offset) => Some(offset.map(isize::from)),
        }
    }
}
