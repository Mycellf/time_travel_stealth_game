use std::{array, f32::consts::PI};

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
            entity::{Entity, EntityVisibleState, GameAction},
        },
        light_grid::{LightArea, LightGrid, Pixel},
    },
};

pub const ELEVATOR_DOOR_TEXTURE_POSITION: Point2<f32> = point![0.0, 24.0];
pub const ELEVATOR_DOOR_TEXTURE_SIZE: Vector2<f32> = vector![8.0, 16.0];
pub const ELEVATOR_DOOR_TEXTURE_OFFSET: Vector2<f32> = vector![-4.0, -8.0];

pub const ELEVATOR_DOOR_SIZE: Vector2<usize> = vector![4, 16];
pub const ELEVATOR_DOOR_OFFSET: Vector2<f64> = vector![-2.0, -8.0];

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ElevatorDoor {
    pub position: Point2<f64>,

    pub extent: usize,
    pub open: bool,
    pub blocked: bool,
    pub lighting_needs_update: bool,

    pub orientation: ElevatorDoorOrientation,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ElevatorDoorOrientation {
    Vertical,
    Horizontal,
}

impl ElevatorDoor {
    pub fn update_light_grid(&mut self, light_grid: &mut LightGrid) {
        self.lighting_needs_update = false;

        let start_position = (self.position + self.offset()).map(|x| x.floor() as isize);

        let air = if self.extent == 0 || self.open {
            Pixel::None
        } else {
            Pixel::Transparent
        };

        for y in 0..ELEVATOR_DOOR_SIZE.y {
            let pixel = if y < self.extent { Pixel::Solid } else { air };

            for x in 0..ELEVATOR_DOOR_SIZE.x / 2 {
                let offset_1 = match self.orientation {
                    ElevatorDoorOrientation::Vertical => vector![x as isize, y as isize],
                    ElevatorDoorOrientation::Horizontal => vector![y as isize, 2 + x as isize],
                };

                let offset_2 = match self.orientation {
                    ElevatorDoorOrientation::Vertical => vector![2 + x as isize, 15 - y as isize],
                    ElevatorDoorOrientation::Horizontal => vector![15 - y as isize, x as isize],
                };

                light_grid[start_position + offset_1] = pixel;
                light_grid[start_position + offset_2] = pixel;
            }
        }
    }

    pub fn edges(&self) -> [[Point2<f64>; 2]; 4] {
        let corners = [[1, 1], [-1, 1], [-1, -1], [1, -1]].map(|offset| {
            self.position
                + Vector2::from(offset)
                    .map(|x| x as f64)
                    .component_mul(&(ELEVATOR_DOOR_SIZE.map(|x| x as f64) / 2.0))
        });

        array::from_fn(|i| [corners[i], corners[(i + 1) % corners.len()]])
    }

    pub fn collision_rect(&self) -> TileRect {
        TileRect {
            origin: (self.position + self.offset()).map(|x| x.floor() as isize),
            size: self.size(),
        }
    }

    pub fn offset(&self) -> Vector2<f64> {
        match self.orientation {
            ElevatorDoorOrientation::Vertical => ELEVATOR_DOOR_OFFSET,
            ElevatorDoorOrientation::Horizontal => ELEVATOR_DOOR_OFFSET.yx(),
        }
    }

    pub fn size(&self) -> Vector2<usize> {
        match self.orientation {
            ElevatorDoorOrientation::Vertical => ELEVATOR_DOOR_SIZE,
            ElevatorDoorOrientation::Horizontal => ELEVATOR_DOOR_SIZE.yx(),
        }
    }
}

#[typetag::serde]
impl Entity for ElevatorDoor {
    fn update(
        &mut self,
        _frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        let previous_extent = self.extent;
        self.blocked = false;
        if self.open {
            self.extent = self.extent.saturating_sub(1);
        } else {
            if self.extent > 0
                || 'outer: {
                    let collision_rect = self.collision_rect();

                    for (_, entity) in entities.iter() {
                        if entity
                            .inner
                            .collision_rect()
                            .is_some_and(|rect| rect.intersects(&collision_rect))
                        {
                            break 'outer false;
                        }
                    }

                    true
                }
            {
                self.extent = (self.extent + 1).min(16);
            } else {
                self.blocked = true;
            }
        }

        if self.extent != previous_extent || self.lighting_needs_update {
            self.update_light_grid(light_grid);
        }

        None
    }

    fn draw_wall(&mut self, texture_atlas: &Texture2D) {
        let position = self.position.map(|x| x as f32) + ELEVATOR_DOOR_TEXTURE_OFFSET;

        let hidden = (16 - self.extent) as f32;

        let rotation = match self.orientation {
            ElevatorDoorOrientation::Vertical => 0.0,
            ElevatorDoorOrientation::Horizontal => PI / 2.0,
        };

        texture::draw_texture_ex(
            texture_atlas,
            position.x,
            position.y,
            colors::WHITE,
            DrawTextureParams {
                source: Some(Rect::new(
                    ELEVATOR_DOOR_TEXTURE_POSITION.x,
                    ELEVATOR_DOOR_TEXTURE_POSITION.y + hidden,
                    ELEVATOR_DOOR_TEXTURE_SIZE.x,
                    ELEVATOR_DOOR_TEXTURE_SIZE.y - hidden,
                )),
                rotation,
                pivot: Some(self.position.map(|x| x as f32).into()),
                ..Default::default()
            },
        );

        texture::draw_texture_ex(
            texture_atlas,
            position.x,
            position.y + hidden,
            colors::WHITE,
            DrawTextureParams {
                source: Some(Rect::new(
                    ELEVATOR_DOOR_TEXTURE_POSITION.x,
                    ELEVATOR_DOOR_TEXTURE_POSITION.y + hidden,
                    ELEVATOR_DOOR_TEXTURE_SIZE.x,
                    ELEVATOR_DOOR_TEXTURE_SIZE.y - hidden,
                )),
                flip_x: true,
                flip_y: true,
                rotation,
                pivot: Some(self.position.map(|x| x as f32).into()),
                ..Default::default()
            },
        );
    }

    fn is_within_view_area(&self, light_grid: &LightGrid, view_area: &LightArea) -> bool {
        self.edges()
            .into_iter()
            .any(|line| view_area.edge_intersects_line(line))
            || view_area
                .range
                .is_none_or(|range| range.contains_offset(self.position - view_area.origin))
                && light_grid.contains_path(view_area.origin, self.position)
    }

    fn visible_state(&self) -> Option<EntityVisibleState> {
        Some(EntityVisibleState::new(self.position, self.extent as u64))
    }

    fn collision_rect(&self) -> Option<TileRect> {
        (self.extent > 0).then(|| self.collision_rect())
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

    fn is_door(&self) -> bool {
        true
    }

    fn as_door(&mut self) -> Option<&mut ElevatorDoor> {
        Some(self)
    }
}
