use std::f64::consts::SQRT_2;

use macroquad::input::KeyCode;
use nalgebra::{Vector2, vector};

#[derive(Clone, Debug)]
pub struct DirectionalInput {
    pub x_axis: AxialInput,
    pub y_axis: AxialInput,
}

impl Default for DirectionalInput {
    fn default() -> Self {
        Self::new(KeyCode::D, KeyCode::W, KeyCode::A, KeyCode::S)
    }
}

impl DirectionalInput {
    pub fn new(right: KeyCode, up: KeyCode, left: KeyCode, down: KeyCode) -> DirectionalInput {
        DirectionalInput {
            x_axis: AxialInput::new(right, left),
            y_axis: AxialInput::new(down, up),
        }
    }

    pub fn key_down(&mut self, key_down: KeyCode) {
        self.x_axis.key_down(key_down.clone());
        self.y_axis.key_down(key_down);
    }

    pub fn key_up(&mut self, key_up: KeyCode) {
        self.x_axis.key_up(key_up.clone());
        self.y_axis.key_up(key_up);
    }

    pub fn clear_keys_down(&mut self) {
        self.x_axis.clear_keys_down();
        self.y_axis.clear_keys_down();
    }

    pub fn raw_output(&self) -> Vector2<i8> {
        vector![self.x_axis.output, self.y_axis.output]
    }

    pub fn rectangular_output(&self) -> Vector2<f64> {
        self.raw_output().map(|x| x as f64)
    }

    pub fn normalized_output(&self) -> Vector2<f64> {
        let mut output = self.rectangular_output();
        if output.x != 0.0 && output.y != 0.0 {
            output /= SQRT_2;
        }
        output
    }

    pub fn stateless_raw_output(&self) -> Vector2<i8> {
        vector![
            self.x_axis.stateless_output(),
            self.y_axis.stateless_output()
        ]
    }

    pub fn stateless_rectangular_output(&self) -> Vector2<f64> {
        self.stateless_raw_output().map(|x| x as f64)
    }

    pub fn stateless_normalized_output(&self) -> Vector2<f64> {
        let mut output = self.stateless_rectangular_output();
        if output.x != 0.0 && output.y != 0.0 {
            output /= SQRT_2;
        }
        output
    }
}

#[derive(Clone, Debug)]
pub struct AxialInput {
    pub positive: KeyCode,
    pub positive_down: bool,
    pub negative: KeyCode,
    pub negative_down: bool,
    pub output: i8,
}

impl AxialInput {
    pub fn new(positive: KeyCode, negative: KeyCode) -> AxialInput {
        AxialInput {
            positive,
            positive_down: false,
            negative,
            negative_down: false,
            output: 0,
        }
    }

    pub fn key_down(&mut self, key_down: KeyCode) {
        if key_down == self.positive {
            self.output = 1;
            self.positive_down = true;
        } else if key_down == self.negative {
            self.output = -1;
            self.negative_down = true;
        }
    }

    pub fn key_up(&mut self, key_up: KeyCode) {
        if key_up == self.positive {
            self.output = if self.negative_down { -1 } else { 0 };
            self.positive_down = false;
        } else if key_up == self.negative {
            self.output = if self.positive_down { 1 } else { 0 };
            self.negative_down = false;
        }
    }

    pub fn clear_keys_down(&mut self) {
        self.positive_down = false;
        self.negative_down = false;
        self.output = 0;
    }

    pub fn stateless_output(&self) -> i8 {
        self.positive_down as i8 - self.negative_down as i8
    }
}

#[derive(Clone, Debug)]
pub struct ButtonInput {
    pub key: KeyCode,
    pub is_down: bool,
}

impl ButtonInput {
    pub fn new(key: KeyCode) -> ButtonInput {
        ButtonInput {
            key,
            is_down: false,
        }
    }

    pub fn key_down(&mut self, key_down: KeyCode) {
        if self.key == key_down {
            self.is_down = true;
        }
    }

    pub fn key_up(&mut self, key_up: KeyCode) {
        if self.key == key_up {
            self.is_down = false;
        }
    }

    pub fn clear_keys_down(&mut self) {
        self.is_down = false;
    }
}
