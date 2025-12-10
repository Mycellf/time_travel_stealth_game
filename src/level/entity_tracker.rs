use macroquad::input::{KeyCode, MouseButton};
use nalgebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::entity::{Entity, GameAction, empty::Empty},
        light_grid::LightGrid,
    },
};

pub(crate) mod entity;
pub(crate) mod wire_diagram;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntityTracker {
    pub inner: Box<dyn Entity>,
}

impl Default for EntityTracker {
    fn default() -> Self {
        Self::new(Box::new(Empty))
    }
}

impl EntityTracker {
    pub fn new(inner: Box<dyn Entity>) -> Self {
        EntityTracker { inner: inner }
    }

    #[must_use]
    pub fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        self.inner
            .update(frame, entities, light_grid, initial_state)
    }

    pub fn key_down(&mut self, input: KeyCode) {
        if self.inner.should_recieve_inputs() {
            self.inner.key_down(input);
        }
    }

    pub fn key_up(&mut self, input: KeyCode) {
        if self.inner.should_recieve_inputs() {
            self.inner.key_up(input);
        }
    }

    pub fn mouse_down(&mut self, input: MouseButton, position: Point2<f64>) {
        if self.inner.should_recieve_inputs() {
            self.inner.mouse_down(input, position);
        }
    }

    pub fn mouse_up(&mut self, input: MouseButton, position: Point2<f64>) {
        if self.inner.should_recieve_inputs() {
            self.inner.mouse_up(input, position);
        }
    }

    pub fn mouse_moved(&mut self, position: Point2<f64>, delta: Vector2<f64>) {
        if self.inner.should_recieve_inputs() {
            self.inner.mouse_moved(position, delta);
        }
    }
}
