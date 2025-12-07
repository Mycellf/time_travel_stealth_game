use std::fmt::Debug;

use macroquad::input::{KeyCode, MouseButton};
use nalgebra::{Point2, Vector2};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::{EntityTracker, entity::player::Player},
        light_grid::{AngleRange, LightGrid},
    },
};

pub(crate) mod dummy;
pub(crate) mod player;

pub trait Entity: 'static + Debug {
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    );

    fn draw(&mut self);

    fn position(&self) -> Point2<f64>;

    fn view_range(&self) -> Option<AngleRange> {
        None
    }

    fn view_kind(&self) -> Option<ViewKind> {
        None
    }

    fn duplicate(&self) -> Box<dyn Entity>;

    fn always_visible(&self) -> bool;

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
