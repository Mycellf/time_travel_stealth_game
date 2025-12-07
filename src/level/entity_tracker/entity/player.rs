use macroquad::{color::Color, input::KeyCode, math::Rect, shapes, time};
use nalgebra::{Point2, UnitVector2, Vector2, point};

use crate::{
    collections::{slot_guard::GuardedSlotMap, tile_grid::TileRect},
    input::DirectionalInput,
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, ViewKind},
        },
        light_grid::{AngleRange, LightGrid},
    },
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

impl Player {
    pub fn collision_rect(&self) -> Rect {
        let corner = self.position - self.size / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            self.size.x as f32,
            self.size.y as f32,
        )
    }
}

impl Entity for Player {
    fn update(
        &mut self,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
    ) {
        for entity in entities.iter() {
            println!("{entity:?}");
        }

        if let Some(new_direction) =
            UnitVector2::try_new(self.mouse_position - self.position, f64::EPSILON)
        {
            self.view_direction = new_direction;
        }

        let motion =
            self.motion_input.normalized_output() * self.speed * time::get_frame_time() as f64;

        self.move_along_axis::<0>(light_grid, motion.x);
        self.move_along_axis::<1>(light_grid, motion.y);
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

    fn view_kind(&self) -> Option<ViewKind> {
        Some(ViewKind::Present)
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn always_visible(&self) -> bool {
        true
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

impl Player {
    fn move_along_axis<const AXIS: usize>(
        &mut self,
        light_grid: &mut LightGrid,
        displacement: f64,
    ) {
        if displacement.abs() <= f64::EPSILON {
            return;
        }

        let old_position = self.position[AXIS];
        self.position[AXIS] += displacement;

        let bounds = TileRect::from_rect_inclusive(self.collision_rect());

        let mut collision = None;

        for x in bounds.left()..bounds.right() + 1 {
            for y in bounds.top()..bounds.bottom() + 1 {
                if light_grid[point![x, y]].blocks_motion() {
                    let axis = [x, y][AXIS];

                    if let Some(collision) = &mut collision {
                        if (*collision < axis) ^ (displacement < 0.0) {
                            *collision = axis;
                        }
                    } else {
                        collision = Some(axis);
                    }
                }
            }
        }

        if let Some(mut collision) = collision {
            if displacement < 0.0 {
                collision += 1;
            }

            self.position[AXIS] = collision as f64;
            self.position[AXIS] -= self.size[AXIS] * displacement.signum() / 2.0;

            if (self.position[AXIS] < old_position) ^ (displacement < 0.0)
                || (self.position[AXIS] - old_position).abs() > displacement.abs()
            {
                self.position[AXIS] = old_position;
            }
        }
    }
}
