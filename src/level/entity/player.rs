use ggez::{
    Context,
    graphics::{Canvas, Color, DrawMode, DrawParam, Mesh, Rect, Transform},
    input::keyboard::KeyInput,
    winit::platform::modifier_supplement::KeyEventExtModifierSupplement,
};
use nalgebra::{Point2, Vector2, point};

use crate::{input::DirectionalInput, level::entity::Entity};

#[derive(Clone, Debug)]
pub struct Player {
    pub position: Point2<f64>,
    pub size: Vector2<f64>,

    pub motion_input: DirectionalInput,
    pub speed: f64,
}

impl Entity for Player {
    fn update(&mut self, ctx: &mut Context) {
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

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn should_recieve_inputs(&self) -> bool {
        true
    }

    fn key_down(&mut self, input: KeyInput, _is_repeat: bool) {
        self.motion_input
            .key_down(input.event.key_without_modifiers());
    }

    fn key_up(&mut self, input: KeyInput) {
        self.motion_input
            .key_up(input.event.key_without_modifiers());
    }
}
