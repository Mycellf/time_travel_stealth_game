use macroquad::input::{KeyCode, MouseButton};
use nalgebra::{Point2, Vector2};

use crate::level::{entity_tracker::entity::Entity, light_grid::LightGrid};

pub(crate) mod entity;

#[derive(Debug)]
pub struct EntityTracker {
    pub inner: Box<dyn Entity>,
}

impl EntityTracker {
    pub fn new(inner: Box<dyn Entity>) -> Self {
        EntityTracker { inner: inner }
    }

    pub fn update(&mut self, light_grid: &mut LightGrid) {
        self.inner.update(light_grid);
    }

    pub fn draw(&mut self) {
        self.inner.draw();
    }

    pub fn key_down(&mut self, input: KeyCode) {
        self.inner.key_down(input);
    }

    pub fn key_up(&mut self, input: KeyCode) {
        self.inner.key_up(input);
    }

    pub fn mouse_down(&mut self, input: MouseButton, position: Point2<f64>) {
        self.inner.mouse_down(input, position);
    }

    pub fn mouse_up(&mut self, input: MouseButton, position: Point2<f64>) {
        self.inner.mouse_up(input, position);
    }

    pub fn mouse_moved(&mut self, position: Point2<f64>, delta: Vector2<f64>) {
        self.inner.mouse_moved(position, delta);
    }
}

impl Clone for EntityTracker {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.duplicate(),
        }
    }
}
