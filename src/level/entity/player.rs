use macroquad::{color::Color, input::KeyCode, shapes, time};
use nalgebra::{Point2, UnitVector2, Vector2};

use crate::{
    input::DirectionalInput,
    level::{entity::Entity, light_grid::AngleRange},
};

#[derive(Clone, Debug)]
pub struct Player {
    pub position: Point2<f64>,
    pub size: Vector2<f64>,

    pub mouse_position: Point2<f64>,
    pub view_direction: UnitVector2<f64>,
    pub view_width: f64,

    pub motion_input: DirectionalInput,
    pub speed: f64,
}

impl Entity for Player {
    fn update(&mut self) {
        if let Some(new_direction) =
            UnitVector2::try_new(self.mouse_position - self.position, f64::EPSILON)
        {
            self.view_direction = new_direction;
        }

        self.position +=
            self.motion_input.normalized_output() * self.speed * time::get_frame_time() as f64
    }

    fn draw(&mut self) {
        let corner = self.position - self.size / 2.0;

        shapes::draw_rectangle(
            corner.x as f32,
            corner.y as f32,
            self.size.x as f32,
            self.size.y as f32,
            Color::new(1.0, 0.0, 0.0, 1.0),
        );
    }

    fn position(&self) -> Point2<f64> {
        self.position
    }

    fn view_range(&self) -> Option<AngleRange> {
        Some(AngleRange::from_direction_and_width(
            self.view_direction,
            self.view_width,
        ))
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn should_recieve_inputs(&self) -> bool {
        true
    }

    fn key_down(&mut self, input: KeyCode) {
        self.motion_input.key_down(input);
    }

    fn key_up(&mut self, input: KeyCode) {
        self.motion_input.key_up(input);
    }

    fn mouse_moved(&mut self, position: Point2<f64>, _delta: Vector2<f64>) {
        self.mouse_position = position;
    }
}
