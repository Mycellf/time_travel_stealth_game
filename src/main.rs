use macroquad::{
    camera::{self, Camera2D},
    color::Color,
    input::{KeyCode, MouseButton},
    math::{Rect, Vec2},
    window::{self, Conf},
};
use nalgebra::{Point2, UnitVector2, Vector2, point, vector};

use crate::{
    input::DirectionalInput,
    level::{Level, entity::player::Player},
};

pub(crate) mod collections;
pub(crate) mod input;
pub(crate) mod level;

fn config() -> Conf {
    Conf {
        window_title: "Time Travel Stealth Game".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

#[macroquad::main(config)]
async fn main() {
    let mut state = State::new();

    let mut mouse_position = get_mouse_position();

    loop {
        let last_mouse_position = mouse_position;

        mouse_position = get_mouse_position();
        let mouse_delta = mouse_position - last_mouse_position;

        if mouse_delta != vector![0.0, 0.0] {
            state.mouse_motion_event(mouse_position, mouse_delta);
        }

        for key in macroquad::input::get_keys_pressed() {
            state.key_down_event(key);
        }

        for key in macroquad::input::get_keys_released() {
            state.key_up_event(key);
        }

        for input in [
            MouseButton::Left,
            MouseButton::Middle,
            MouseButton::Right,
            MouseButton::Unknown,
        ] {
            if macroquad::input::is_mouse_button_pressed(input) {
                state.mouse_button_down_event(input, mouse_position);
            }
        }

        for input in [
            MouseButton::Left,
            MouseButton::Middle,
            MouseButton::Right,
            MouseButton::Unknown,
        ] {
            if macroquad::input::is_mouse_button_released(input) {
                state.mouse_button_up_event(input, mouse_position);
            }
        }

        state.update();

        state.draw();

        window::next_frame().await;
    }
}

pub(crate) struct State {
    fullscreen: bool,

    level: Level,
}

impl State {
    fn new() -> Self {
        use std::f64::consts::PI;

        State {
            fullscreen: true,

            level: Level::new(vec![Box::new(Player {
                position: point![0.0, 0.0],
                size: vector![6.0, 6.0],

                mouse_position: point![0.0, 0.0],
                view_direction: UnitVector2::new_normalize(vector![1.0, 0.0]),
                view_width: PI * 1.0 / 2.0,

                speed: 64.0,
                motion_input: DirectionalInput::new(KeyCode::D, KeyCode::W, KeyCode::A, KeyCode::S),
            })]),
        }
    }
}

impl State {
    pub const SCREEN_HEIGHT: f32 = 256.0;

    fn screen_rect(&self) -> Rect {
        rectangle_of_centered_camera(
            vector![window::screen_width(), window::screen_height()],
            point![0.0, 0.0],
            Self::SCREEN_HEIGHT,
        )
    }

    fn screen_to_world(&self, point: Point2<f32>) -> Point2<f32> {
        let world = self.screen_rect();
        let screen = Rect::new(0.0, 0.0, window::screen_width(), window::screen_height());

        transform_between_rectangles(screen, world, point)
    }

    fn screen_to_world_scale_factor(&self) -> f32 {
        Self::SCREEN_HEIGHT / window::screen_height()
    }

    fn update(&mut self) {
        self.level.update();
    }

    fn draw(&mut self) {
        window::clear_background(Color::new(0.5, 0.5, 0.5, 1.0));

        let mut camera = Camera2D::from_display_rect(self.screen_rect());
        camera.zoom.y *= -1.0;
        camera::set_camera(&camera);

        self.level.draw();
    }

    fn key_down_event(&mut self, input: KeyCode) {
        self.level.key_down(input);

        match input {
            KeyCode::Escape => {
                window::miniquad::window::quit();
            }
            KeyCode::F11 => {
                self.fullscreen ^= true;

                window::set_fullscreen(self.fullscreen);
            }
            _ => (),
        }
    }

    fn key_up_event(&mut self, input: KeyCode) {
        self.level.key_up(input);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, position: Point2<f32>) {
        self.level
            .mouse_down(button, self.screen_to_world(position).map(|x| x as f64));
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, position: Point2<f32>) {
        self.level
            .mouse_up(button, self.screen_to_world(position).map(|x| x as f64));
    }

    fn mouse_motion_event(&mut self, position: Point2<f32>, delta: Vector2<f32>) {
        self.level.mouse_moved(
            self.screen_to_world(position).map(|x| x as f64),
            (delta * self.screen_to_world_scale_factor()).map(|x| x as f64),
        );
    }
}

fn rectangle_of_centered_camera(
    screen_size: Vector2<f32>,
    center: Point2<f32>,
    height: f32,
) -> Rect {
    let size = vector![height * screen_size.x / screen_size.y, height];
    let corner = center - size / 2.0;

    Rect::new(corner.x, corner.y, size.x, size.y)
}

fn transform_between_rectangles(
    source: Rect,
    destination: Rect,
    point: Point2<f32>,
) -> Point2<f32> {
    let source_origin = point![source.x, source.y];
    let source_size = vector![source.w, source.h];

    let destination_origin = point![destination.x, destination.y];
    let destination_size = vector![destination.w, destination.h];

    destination_origin
        + (point - source_origin)
            .component_div(&source_size)
            .component_mul(&destination_size)
}

fn get_mouse_position() -> Point2<f32> {
    Point2::from(Vec2::from(macroquad::input::mouse_position()))
}
