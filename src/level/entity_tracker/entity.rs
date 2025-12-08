use std::fmt::Debug;

use macroquad::input::{KeyCode, MouseButton};
use nalgebra::{Point2, Vector2};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::{EntityTracker, entity::player::Player},
        light_grid::{LightArea, LightGrid},
    },
};

pub(crate) mod player;

pub trait Entity: 'static + Debug {
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    );

    fn update_view_area(&mut self, _light_grid: &mut LightGrid) {}

    fn draw_wall(&mut self) {}

    fn draw_back(&mut self) {}

    fn draw_front(&mut self) {}

    fn position(&self) -> Point2<f64>;

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

    fn should_recieve_inputs(&self) -> bool;

    fn key_down(&mut self, _input: KeyCode) {}

    fn key_up(&mut self, _input: KeyCode) {}

    fn mouse_down(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_up(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {}

    fn as_player_mut(&mut self) -> Option<&mut Player> {
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ViewKind {
    Present,
    Past,
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
