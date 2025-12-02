use ggez::{
    Context,
    graphics::{Canvas, Color, DrawMode, DrawParam, Mesh, Rect, Transform},
    input::keyboard::KeyInput,
};
use nalgebra::{Point2, UnitVector2, Vector2, point};

use crate::{
    input::{self, DirectionalInput},
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
    fn update(&mut self, ctx: &mut Context) {
        if let Some(new_direction) =
            UnitVector2::try_new(self.mouse_position - self.position, f64::EPSILON)
        {
            self.view_direction = new_direction;
        }

        self.position +=
            self.motion_input.normalized_output() * self.speed * ctx.time.delta().as_secs_f64();
    }

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
        let mesh = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(-0.5, -0.5, 1.0, 1.0),
            Color::WHITE,
        )
        .unwrap();

        canvas.draw(
            &mesh,
            DrawParam {
                color: Color::RED,
                transform: Transform::Values {
                    dest: self.position.map(|x| x as f32).into(),
                    rotation: 0.0,
                    scale: self.size.map(|x| x as f32).into(),
                    offset: point![0.0, 0.0].into(),
                },
                z: 0,
                ..Default::default()
            },
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

    fn key_down(&mut self, input: KeyInput, _is_repeat: bool) {
        self.motion_input
            .key_down(input::cross_platform_key_without_modifiers(input.event));
    }

    fn key_up(&mut self, input: KeyInput) {
        self.motion_input
            .key_up(input::cross_platform_key_without_modifiers(input.event));
    }

    fn mouse_moved(&mut self, position: Point2<f64>, _delta: Vector2<f64>) {
        self.mouse_position = position;
    }
}
