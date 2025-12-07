use std::mem;

use macroquad::{color::Color, input::KeyCode, math::Rect, shapes};
use nalgebra::{Point2, UnitVector2, Vector2, point};
use slotmap::SlotMap;

use crate::{
    collections::{
        history::{FrameIndex, History},
        slot_guard::GuardedSlotMap,
        tile_grid::TileRect,
    },
    input::DirectionalInput,
    level::{
        EntityKey, UPDATE_DT,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, ViewKind},
        },
        light_grid::{AngleRange, LightArea, LightGrid},
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

    pub state: PlayerState,
    pub history: History<PlayerHistoryEntry>,

    pub view_area: Option<LightArea>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerState {
    Active,
    Reset,
    Recording,
    Disabled,
    Dead,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlayerHistoryEntry {
    pub position: Point2<f32>,
    pub mouse_position: Point2<f32>,
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

    pub fn get_history_entry(&self) -> PlayerHistoryEntry {
        PlayerHistoryEntry {
            position: self.position.map(|x| x as f32),
            mouse_position: self.mouse_position.map(|x| x as f32),
        }
    }

    pub fn update_view_direction(&mut self) {
        if let Some(new_direction) =
            UnitVector2::try_new(self.mouse_position - self.position, f64::EPSILON)
        {
            self.view_direction = new_direction;
        }
    }
}

impl Entity for Player {
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) {
        match self.state {
            PlayerState::Active => {
                self.update_view_direction();

                let motion = self.motion_input.normalized_output() * self.speed * UPDATE_DT;

                self.move_along_axis::<0>(light_grid, motion.x);
                self.move_along_axis::<1>(light_grid, motion.y);

                self.history.try_insert(frame, self.get_history_entry());
            }
            PlayerState::Reset => {
                let old_self = initial_state[*entities.protected_slot()]
                    .inner
                    .as_player_mut()
                    .unwrap();

                old_self.state = PlayerState::Recording;
                old_self.history = mem::take(&mut self.history);

                self.state = PlayerState::Active;
                initial_state.insert(EntityTracker::new(Box::new(self.clone())));

                self.state = PlayerState::Disabled;
            }
            PlayerState::Recording => {
                if let Some(entry) = self.history.get(frame) {
                    self.position = entry.position.map(|x| x as f64);
                    self.mouse_position = entry.mouse_position.map(|x| x as f64);

                    self.update_view_direction();
                } else {
                    self.state = PlayerState::Disabled;
                }
            }
            PlayerState::Disabled => (),
            PlayerState::Dead => (),
        }

        self.view_area = match self.state {
            PlayerState::Active | PlayerState::Reset | PlayerState::Recording => {
                Some(light_grid.trace_light_from(
                    self.position,
                    Some(AngleRange::from_direction_and_width(
                        self.view_direction,
                        self.view_width,
                    )),
                ))
            }
            PlayerState::Disabled | PlayerState::Dead => None,
        };
    }

    fn draw_front(&mut self) {
        if self.state == PlayerState::Disabled {
            return;
        }

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
        match self.state {
            PlayerState::Active => Some(ViewKind::Present),
            PlayerState::Reset => None,
            PlayerState::Recording => Some(ViewKind::Past),
            PlayerState::Disabled => None,
            PlayerState::Dead => None,
        }
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn should_recieve_inputs(&self) -> bool {
        self.state == PlayerState::Active
    }

    fn key_down(&mut self, input: KeyCode) {
        self.motion_input.key_down(input);

        match input {
            KeyCode::T => self.state = PlayerState::Reset,
            _ => (),
        }
    }

    fn key_up(&mut self, input: KeyCode) {
        self.motion_input.key_up(input);
    }

    fn mouse_moved(&mut self, position: Point2<f64>, _delta: Vector2<f64>) {
        self.mouse_position = position;
    }

    fn as_player_mut(&mut self) -> Option<&mut Player> {
        Some(self)
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
                        if (*collision < axis) ^ (displacement > 0.0) {
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
