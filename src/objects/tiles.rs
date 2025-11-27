use std::sync::LazyLock;

use slotmap::{SlotMap, new_key_type};

use crate::collections::{
    multi_tile_grid::{MultiTileGrid, TileShape},
    tile_grid::{TileGrid, TileIndexOffset, TileRect},
};

#[derive(Clone, Debug)]
pub struct Tiles {
    pub tiles: MultiTileGrid<TileEntry, TileShapeKey>,
}

#[derive(Clone, Debug)]
pub struct TileEntry {}

new_key_type! {
    pub struct TileShapeKey;
}

impl TileShape for TileShapeKey {
    fn offsets(&self) -> impl Iterator<Item = TileIndexOffset> {
        TILE_SHAPES.entries[*self].offsets()
    }
}

pub static TILE_SHAPES: LazyLock<TileShapes> = LazyLock::new(TileShapes::default);

#[derive(Clone, Debug)]
pub struct TileShapes {
    pub entries: SlotMap<TileShapeKey, TileShapeEntry>,
}

impl Default for TileShapes {
    fn default() -> Self {
        Self {
            entries: SlotMap::default(),
        }
    }
}

impl TileShapes {
    pub fn insert(&mut self, mut shape: TileShapeEntry) -> TileShapeKey {
        shape.offsets.shrink_to_fit();

        self.entries.insert(shape)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TileShapeEntry {
    offsets: TileGrid<bool>,
}

impl TileShape for TileShapeEntry {
    fn offsets(&self) -> impl Iterator<Item = TileIndexOffset> {
        self.offsets.iter().map(|(offset, _)| offset.coords)
    }

    fn bounds_hint(&self) -> Option<TileRect> {
        Some(self.offsets.bounds())
    }
}
