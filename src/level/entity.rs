use std::fmt::Debug;

use ggez::{Context, graphics::Canvas, input::keyboard::KeyInput, winit::event::MouseButton};
use nalgebra::{Point2, Vector2};

use crate::level::light_grid::AngleRange;

pub(crate) mod player;

#[derive(Debug)]
pub struct EntityTracker {
    pub inner: Box<dyn Entity>,
}

impl EntityTracker {
    pub fn new(inner: Box<dyn Entity>) -> Self {
        EntityTracker { inner: inner }
    }

    pub fn update(&mut self, ctx: &mut Context) {
        self.inner.update(ctx);
    }

    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
        self.inner.draw(ctx, canvas);
    }

    pub fn key_down(&mut self, input: KeyInput, is_repeat: bool) {
        self.inner.key_down(input, is_repeat);
    }

    pub fn key_up(&mut self, input: KeyInput) {
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

pub trait Entity: 'static + Debug {
    fn update(&mut self, ctx: &mut Context);

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas);

    fn position(&self) -> Point2<f64>;

    fn view_range(&self) -> Option<AngleRange> {
        None
    }

    fn duplicate(&self) -> Box<dyn Entity>;

    fn should_recieve_inputs(&self) -> bool;

    fn key_down(&mut self, _input: KeyInput, _is_repeat: bool) {}

    fn key_up(&mut self, _input: KeyInput) {}

    fn mouse_down(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_up(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    fn mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {}
}
