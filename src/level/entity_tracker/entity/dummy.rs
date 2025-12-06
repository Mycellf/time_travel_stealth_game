use macroquad::{color::Color, math::Rect, shapes};
use nalgebra::{Point2, UnitVector2, Vector2, point};

use crate::{
    collections::tile_grid::TileRect,
    level::{
        entity_tracker::entity::{Entity, ViewKind},
        light_grid::{AngleRange, LightGrid},
    },
};

#[derive(Clone, Debug)]
pub struct Dummy {
    pub position: Point2<f64>,
    pub size: Vector2<f64>,

    pub view_direction: UnitVector2<f64>,
    pub view_width: f64,
}

impl Dummy {
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

impl Entity for Dummy {
    fn update(&mut self, _light_grid: &mut LightGrid) {}

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
        Some(ViewKind::Past)
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn always_visible(&self) -> bool {
        true
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }
}

impl Dummy {
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
