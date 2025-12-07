use std::{array, mem};

use macroquad::{color::Color, input::KeyCode, math::Rect, shapes};
use nalgebra::{Point2, UnitVector2, Vector2, point};
use slotmap::{SecondaryMap, SlotMap};

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
            entity::{Entity, EntityVisibleState, ViewKind},
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
    pub environment_history: SecondaryMap<EntityKey, History<EntityVisibleState>>,

    pub view_area: Option<LightArea>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayerState {
    Active,
    Reset,
    Replay,
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

    pub fn draw(&self) {
        let corner = self.position - self.size / 2.0;

        shapes::draw_rectangle(
            corner.x as f32,
            corner.y as f32,
            self.size.x as f32,
            self.size.y as f32,
            Color::new(1.0, 0.0, 0.0, 1.0),
        );
    }

    pub fn edges(&self) -> [[Point2<f64>; 2]; 4] {
        let corners = [[1, 1], [-1, 1], [-1, -1], [1, -1]].map(|offset| {
            self.position
                + Vector2::from(offset)
                    .map(|x| x as f64)
                    .component_mul(&(self.size / 2.0))
        });

        array::from_fn(|i| [corners[i], corners[(i + 1) % corners.len()]])
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
                let motion = self.motion_input.normalized_output() * self.speed * UPDATE_DT;

                self.move_along_axis::<0>(light_grid, motion.x);
                self.move_along_axis::<1>(light_grid, motion.y);

                self.update_view_direction();

                self.history.try_insert(frame, self.get_history_entry());

                if let Some(view_area) = &self.view_area {
                    for (key, entity) in entities.iter() {
                        if entity.inner.is_within_view_area(light_grid, view_area) {
                            let state = entity.inner.visible_state().unwrap();
                            if !self.environment_history.contains_key(key) {
                                self.environment_history.insert(key, History::default());
                            }

                            self.environment_history[key].try_insert(frame, state);
                        }
                    }
                }
            }
            PlayerState::Reset | PlayerState::Replay => {
                let old_self = initial_state[*entities.protected_slot()]
                    .inner
                    .as_player_mut()
                    .unwrap();

                old_self.state = PlayerState::Recording;
                old_self.history = mem::take(&mut self.history);
                old_self.environment_history = mem::take(&mut self.environment_history);

                if self.state == PlayerState::Reset {
                    self.state = PlayerState::Active;
                    initial_state.insert(EntityTracker::new(Box::new(self.clone())));
                }

                self.state = PlayerState::Disabled;
            }
            PlayerState::Recording => {
                if let Some(entry) = self.history.get(frame) {
                    self.position = entry.position.map(|x| x as f64);
                    self.mouse_position = entry.mouse_position.map(|x| x as f64);

                    self.update_view_direction();

                    if let Some(view_area) = &self.view_area {
                        for (key, entity) in entities.iter() {
                            if entity.inner.is_within_view_area(light_grid, view_area) {
                                let state = entity.inner.visible_state().unwrap();
                                if self
                                    .environment_history
                                    .get(key)
                                    .is_none_or(|history| Some(&state) != history.get(frame))
                                {
                                    self.state = PlayerState::Dead;
                                }
                            } else if self
                                .environment_history
                                .get(key)
                                .is_some_and(|history| history.get(frame).is_some())
                            {
                                self.state = PlayerState::Dead;
                            }
                        }
                    }
                } else {
                    self.state = PlayerState::Disabled;
                }
            }
            PlayerState::Disabled | PlayerState::Dead => (),
        }
    }

    fn update_view_area(&mut self, light_grid: &mut LightGrid) {
        self.view_area = match self.state {
            PlayerState::Active | PlayerState::Recording => Some(light_grid.trace_light_from(
                self.position,
                Some(AngleRange::from_direction_and_width(
                    self.view_direction,
                    self.view_width,
                )),
            )),
            PlayerState::Reset
            | PlayerState::Replay
            | PlayerState::Disabled
            | PlayerState::Dead => None,
        };
    }

    fn draw_back(&mut self) {
        match self.state {
            PlayerState::Dead => self.draw(),
            _ => (),
        }
    }

    fn draw_front(&mut self) {
        match self.state {
            PlayerState::Active | PlayerState::Reset | PlayerState::Recording => self.draw(),
            _ => (),
        }
    }

    fn is_within_view_area(&self, light_grid: &LightGrid, view_area: &LightArea) -> bool {
        if self.state == PlayerState::Disabled {
            return false;
        }

        self.edges()
            .into_iter()
            .any(|line| view_area.edge_intersects_line(line))
            || view_area
                .range
                .is_none_or(|range| range.contains_offset(self.position - view_area.origin))
                && light_grid.contains_path(view_area.origin, self.position)
    }

    fn visible_state(&self) -> Option<EntityVisibleState> {
        if self.state == PlayerState::Disabled {
            None
        } else {
            Some(EntityVisibleState::new(self.position, 0))
        }
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
            PlayerState::Replay => None,
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
            KeyCode::Y => self.state = PlayerState::Replay,
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
