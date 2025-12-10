use macroquad::{
    camera::{self, Camera2D},
    color::colors,
    input::{KeyCode, MouseButton},
    math::{Rect, Vec2},
    time,
    window::{self, Conf},
};
use nalgebra::{Point2, Vector2, point, vector};

use crate::level::{Level, MAX_UPDATES_PER_TICK, UPDATE_DT};

#[allow(dead_code)]
pub(crate) mod collections;
#[allow(dead_code)]
pub(crate) mod input;
pub(crate) mod level;

pub const START_IN_FULLSCREEN: bool = true;

fn config() -> Conf {
    Conf {
        window_title: "Time Travel Stealth Game".to_owned(),
        fullscreen: START_IN_FULLSCREEN,
        ..Default::default()
    }
}

pub const TEXTURE_ATLAS: &[u8] = include_bytes!("../resources/texture_atlas.png");

/// CREDIT: <https://docs.rs/inventory/0.3.21/inventory/#webassembly-and-constructors> says to do so...
#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __wasm_call_ctors();
}

#[macroquad::main(config)]
async fn main() {
    #[cfg(target_family = "wasm")]
    unsafe {
        __wasm_call_ctors();
    }

    let mut state = State::new();

    let mut mouse_position = point![0.0, 0.0];

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

        const MOUSE_BUTTONS: [MouseButton; 4] = [
            MouseButton::Left,
            MouseButton::Middle,
            MouseButton::Right,
            MouseButton::Unknown,
        ];

        for input in MOUSE_BUTTONS {
            if macroquad::input::is_mouse_button_pressed(input) {
                state.mouse_button_down_event(input, mouse_position);
            }
        }

        for input in MOUSE_BUTTONS {
            if macroquad::input::is_mouse_button_released(input) {
                state.mouse_button_up_event(input, mouse_position);
            }
        }

        while let Some(input) = macroquad::input::get_char_pressed() {
            state.text_input_event(input);
        }

        state.update(time::get_frame_time() as f64);

        state.draw();

        window::next_frame().await;
    }
}

pub(crate) struct State {
    fullscreen: bool,

    level: Level,
    update_time: f64,
}

impl State {
    fn new() -> Self {
        let mut level = Level::new("resources/levels/test".to_owned());

        level.reset();
        level.step_at_level_start();

        State {
            fullscreen: START_IN_FULLSCREEN,

            level,
            update_time: 0.0,
        }
    }
}

impl State {
    pub const SCREEN_HEIGHT: f32 = 256.0;

    fn update(&mut self, dt: f64) {
        self.update_time += dt / UPDATE_DT;

        for _ in 0..MAX_UPDATES_PER_TICK.min(self.update_time.floor() as usize) {
            self.level.update();

            self.update_time -= 1.0;
        }

        self.update_time = self.update_time.min(1.0);
    }

    fn draw(&mut self) {
        window::clear_background(colors::BLACK);

        let mut camera = Camera2D::from_display_rect(screen_rect());
        camera.zoom.y *= -1.0;
        camera::set_camera(&camera);

        self.level.draw();
    }

    fn text_input_event(&mut self, input: char) {
        self.level.text_input(input);
    }

    fn key_down_event(&mut self, input: KeyCode) {
        self.level.key_down(input);

        match input {
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
            .mouse_down(button, screen_to_world(position).map(|x| x as f64));
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, position: Point2<f32>) {
        self.level
            .mouse_up(button, screen_to_world(position).map(|x| x as f64));
    }

    fn mouse_motion_event(&mut self, position: Point2<f32>, delta: Vector2<f32>) {
        self.level.mouse_moved(
            screen_to_world(position).map(|x| x as f64),
            (delta * screen_to_world_scale_factor()).map(|x| x as f64),
        );
    }
}

pub fn rectangle_of_centered_camera(
    screen_size: Vector2<f32>,
    center: Point2<f32>,
    height: f32,
) -> Rect {
    let size = vector![height * screen_size.x / screen_size.y, height];
    let corner = center - size / 2.0;

    Rect::new(corner.x, corner.y, size.x, size.y)
}

pub fn transform_between_rectangles(
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

pub fn get_mouse_position() -> Point2<f32> {
    Point2::from(Vec2::from(macroquad::input::mouse_position()))
}

pub fn screen_rect() -> Rect {
    rectangle_of_centered_camera(
        vector![window::screen_width(), window::screen_height()],
        point![0.0, 0.0],
        State::SCREEN_HEIGHT,
    )
}

pub fn screen_to_world(point: Point2<f32>) -> Point2<f32> {
    let world = screen_rect();
    let screen = Rect::new(0.0, 0.0, window::screen_width(), window::screen_height());

    transform_between_rectangles(screen, world, point)
}

pub fn screen_to_world_scale_factor() -> f32 {
    State::SCREEN_HEIGHT / window::screen_height()
}

pub fn screen_pixel_size() -> Vector2<u32> {
    (window::screen_dpi_scale() * vector![window::screen_width(), window::screen_height()])
        .map(|x| x as u32)
}
