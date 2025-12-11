use macroquad::{
    color::colors,
    math::Rect,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Vector2, point, vector};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, GameAction},
        },
        light_grid::LightGrid,
    },
};

pub const BUTTON_SIZE: Vector2<f64> = vector![8.0, 8.0];

pub const BUTTON_TEXTURE_POSITION: Point2<f32> = point![0.0, 48.0];
pub const BUTTON_TEXTURE_SIZE: Vector2<f32> = vector![16.0, 16.0];

pub const BUTTON_TOP_POSITION: Point2<f32> = point![4.0, 51.0];
pub const BUTTON_TOP_SIZE: Vector2<f32> = vector![8.0, 8.0];

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct Button {
    pub position: Point2<f64>,
    #[serde(skip)]
    pub pressed: bool,
}

impl Button {
    pub fn collision_rect(&self) -> Rect {
        let corner = self.position - BUTTON_SIZE / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            BUTTON_SIZE.x as f32,
            BUTTON_SIZE.y as f32,
        )
    }
}

#[typetag::serde]
impl Entity for Button {
    fn update(
        &mut self,
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        None
    }

    fn draw_floor(&mut self, texture_atlas: &Texture2D) {
        texture::draw_texture_ex(
            texture_atlas,
            self.position.x as f32 - BUTTON_TEXTURE_SIZE.x / 2.0,
            self.position.y as f32 - BUTTON_TEXTURE_SIZE.y / 2.0,
            colors::WHITE,
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    BUTTON_TEXTURE_POSITION
                        + vector![
                            if self.pressed {
                                BUTTON_TEXTURE_SIZE.x
                            } else {
                                0.0
                            },
                            0.0,
                        ],
                    BUTTON_TEXTURE_SIZE,
                )),
                ..Default::default()
            },
        );
    }

    fn draw_back(&mut self, texture_atlas: &Texture2D) {
        if self.pressed {
            return;
        }

        texture::draw_texture_ex(
            texture_atlas,
            self.position.x as f32 - BUTTON_TOP_SIZE.x / 2.0,
            self.position.y as f32 - BUTTON_TOP_SIZE.y / 2.0 - 1.0,
            colors::WHITE,
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    BUTTON_TOP_POSITION,
                    BUTTON_TOP_SIZE,
                )),
                ..Default::default()
            },
        );
    }

    fn collision_rect(&self) -> Option<TileRect> {
        Some(TileRect::from_rect_inclusive(self.collision_rect()))
    }

    fn position(&self) -> Point2<f64> {
        self.position
    }

    fn position_mut(&mut self) -> Option<&mut Point2<f64>> {
        Some(&mut self.position)
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }

    fn evaluate(
        &mut self,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _inputs: &[bool],
    ) -> bool {
        let collision_rect = TileRect::from_rect_inclusive(self.collision_rect());

        self.pressed = entities.iter().any(|(_, entity)| {
            entity
                .inner
                .collision_rect()
                .is_some_and(|rect| rect.intersects(&collision_rect))
        });

        self.pressed
    }
}
