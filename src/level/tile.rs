use std::sync::{LazyLock, Mutex};

use macroquad::math::Rect;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};

use crate::{collections::small_map::SmallMap, level::light_grid::Pixel, new_small_key_type};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Tile {
    pub kind: TileKindKey,
}

impl Tile {
    pub fn get_kind(&self) -> TileKind {
        TILE_KINDS.lock().unwrap()[self.kind].clone()
    }
}

pub static TILE_KINDS: LazyLock<Mutex<SmallMap<TileKindKey, TileKind>>> =
    LazyLock::new(|| Mutex::new(SmallMap::default()));

new_small_key_type! {
    pub struct TileKindKey(u16);
}

pub fn add_tile_kind(tile_kind: TileKind) -> TileKindKey {
    TILE_KINDS.lock().unwrap().insert(tile_kind)
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TileKind {
    pub name: String,
    pub pixel_kind: Pixel,
    pub texture_location: Point2<usize>,
}

impl TileKind {
    pub fn texture_location_f32(&self) -> Point2<f32> {
        self.texture_location
            .map(|x| x as f32 * super::TILE_SIZE as f32)
    }

    pub fn texture_rect(&self) -> Rect {
        let location = self.texture_location_f32();
        Rect::new(
            location.x,
            location.y,
            super::TILE_SIZE as f32,
            super::TILE_SIZE as f32,
        )
    }
}
