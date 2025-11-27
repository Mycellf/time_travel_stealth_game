use std::sync::LazyLock;

use slotmap::{SlotMap, new_key_type};

use crate::collections::{
    multi_tile_grid::{MultiTileGrid, TileShape},
    tile_grid::{TileGrid, TileIndexOffset, TileRect},
};

#[derive(Clone, Debug)]
pub struct Tiles {
    pub tiles: MultiTileGrid<TileData, TileKey>,
}

#[derive(Clone, Debug)]
pub struct TileData {}

new_key_type! {
    pub struct TileKey;
}

impl TileShape for TileKey {
    fn offsets(&self) -> impl Iterator<Item = TileIndexOffset> {
        TILE_ENTRIES.entries[*self].offsets()
    }
}

pub static TILE_ENTRIES: LazyLock<TileEntries> = LazyLock::new(TileEntries::default);

#[derive(Clone, Debug)]
pub struct TileEntries {
    pub entries: SlotMap<TileKey, TileEntry>,
}

impl Default for TileEntries {
    fn default() -> Self {
        Self {
            entries: SlotMap::default(),
        }
    }
}

impl TileEntries {
    pub fn insert(&mut self, mut shape: TileEntry) -> TileKey {
        shape.shape.shrink_to_fit();

        self.entries.insert(shape)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TileEntry {
    shape: TileGrid<bool>,
}

impl TileShape for TileEntry {
    fn offsets(&self) -> impl Iterator<Item = TileIndexOffset> {
        self.shape.iter().map(|(offset, _)| offset.coords)
    }

    fn bounds_hint(&self) -> Option<TileRect> {
        Some(self.shape.bounds())
    }
}
