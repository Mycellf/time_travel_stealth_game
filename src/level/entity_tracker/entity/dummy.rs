use macroquad::{color::Color, math::Rect, shapes};
use nalgebra::{Point2, UnitVector2, Vector2};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, ViewKind},
        },
        light_grid::{AngleRange, LightArea, LightGrid},
    },
};

#[derive(Clone, Debug)]
pub struct Dummy {
    pub position: Point2<f64>,
    pub size: Vector2<f64>,

    pub view_direction: UnitVector2<f64>,
    pub view_width: f64,

    pub view_area: Option<LightArea>,
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
    fn update(
        &mut self,
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) {
    }

    fn update_view_area(&mut self, light_grid: &mut LightGrid) {
        self.view_area = Some(light_grid.trace_light_from(
            self.position,
            Some(AngleRange::from_direction_and_width(
                self.view_direction,
                self.view_width,
            )),
        ));
    }

    fn draw_front(&mut self) {
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

    fn view_area(&self) -> Option<LightArea> {
        self.view_area.clone()
    }

    fn view_kind(&self) -> Option<ViewKind> {
        Some(ViewKind::Past)
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }
}
