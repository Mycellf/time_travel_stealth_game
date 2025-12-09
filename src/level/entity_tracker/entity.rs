use std::fmt::Debug;

use macroquad::{
    input::{KeyCode, MouseButton},
    texture::Texture2D,
};
use nalgebra::{Point2, Vector2};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{elevator::Elevator, elevator_door::ElevatorDoor, player::Player},
        },
        light_grid::{LightArea, LightGrid},
    },
};

pub(crate) mod elevator;
pub(crate) mod elevator_door;
pub(crate) mod empty;
pub(crate) mod player;

pub trait Entity: 'static + Debug {
    #[must_use]
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction>;

    fn update_view_area(&mut self, _light_grid: &mut LightGrid) {}

    fn travel_to_beginning(&mut self, _past: &mut EntityTracker) {}

    fn draw_wall(&mut self, _texture_atlas: &Texture2D) {}

    fn draw_effect_back(&mut self, _texture_atlas: &Texture2D) {}

    fn draw_back(&mut self, _texture_atlas: &Texture2D) {}

    fn draw_front(&mut self, _texture_atlas: &Texture2D) {}

    fn draw_effect_front(&mut self, _texture_atlas: &Texture2D) {}

    fn collision_rect(&self) -> Option<TileRect> {
        None
    }

    fn view_area(&self) -> Option<LightArea> {
        None
    }

    fn view_kind(&self) -> Option<ViewKind> {
        None
    }

    fn is_within_view_area(&self, _light_grid: &LightGrid, _area: &LightArea) -> bool {
        false
    }

    fn visible_state(&self) -> Option<EntityVisibleState> {
        None
    }

    fn duplicate(&self) -> Box<dyn Entity>;

    fn spawn(&mut self, _key: EntityKey, _entities: &mut SlotMap<EntityKey, EntityTracker>) {}

    fn is_dead(&self) -> bool {
        false
    }

    fn should_recieve_inputs(&self) -> bool;

    fn key_down(&mut self, _input: KeyCode) {}

    fn key_up(&mut self, _input: KeyCode) {}

    fn mouse_down(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_up(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {}

    fn as_player(&mut self) -> Option<&mut Player> {
        None
    }

    fn as_door(&mut self) -> Option<&mut ElevatorDoor> {
        None
    }

    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        None
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameAction {
    SoftReset,
    HardReset,
}

#[derive(Clone, Copy, Debug)]
pub enum ViewKind {
    Present,
    Past { confusion: f64 },
}

impl ViewKind {
    pub fn confusion(self) -> f64 {
        match self {
            ViewKind::Present => -f64::INFINITY,
            ViewKind::Past { confusion } => confusion,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EntityVisibleState {
    pub position: Point2<f32>,
    pub extra: u64,
}

impl EntityVisibleState {
    pub fn new(position: Point2<f64>, extra: u64) -> Self {
        Self {
            position: position.map(|x| x as f32),
            extra,
        }
    }

    pub fn position(&self) -> Point2<f64> {
        self.position.map(|x| x as f64)
    }
}
