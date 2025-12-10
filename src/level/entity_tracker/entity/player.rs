use std::{array, cmp::Ordering, f64::consts::PI, mem};

use macroquad::{
    color::{Color, colors},
    input::KeyCode,
    math::Rect,
    shapes,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, UnitVector2, Vector2, point, vector};
use serde::{Deserialize, Serialize};
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
            entity::{Entity, EntityVisibleState, GameAction, ViewKind},
            wire_diagram::Wire,
        },
        light_grid::{AngleRange, LightArea, LightGrid},
    },
};

pub const CONFUSION_EFFECT_START: Point2<f32> = point![0.0, 16.0];
pub const CONFUSION_EFFECT_SIZE: Vector2<f32> = vector![8.0, 8.0];
pub const CONFUSION_EFFECT_OFFSET: Vector2<f32> = vector![-4.0, -12.0];
pub const CONFUSION_EFFECT_RUN: usize = 10;

pub fn rect_of_confusion_effect(paradox_level: f64) -> Rect {
    let effect_index = ((paradox_level.clamp(0.0, 1.0) * CONFUSION_EFFECT_RUN as f64).floor()
        as usize)
        .min(CONFUSION_EFFECT_RUN - 1);

    Rect::new(
        CONFUSION_EFFECT_START.x + effect_index as f32 * CONFUSION_EFFECT_SIZE.x,
        CONFUSION_EFFECT_START.y,
        CONFUSION_EFFECT_SIZE.x,
        CONFUSION_EFFECT_SIZE.y,
    )
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Player {
    pub position: Point2<f64>,
    pub size: Vector2<f64>,

    pub mouse_position: Point2<f64>,
    pub view_direction: UnitVector2<f64>,
    pub view_width: f64,

    #[serde(skip)]
    pub motion_input: DirectionalInput,
    pub speed: f64,

    pub state: PlayerState,
    #[serde(skip)]
    pub history: History<PlayerHistoryEntry>,
    #[serde(skip)]
    pub environment_history: SecondaryMap<EntityKey, History<EntityVisibleState>>,

    pub confusion: f64,
    pub paradox_position: Option<(f64, Point2<f64>)>,

    #[serde(skip)]
    pub view_area: Option<LightArea>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            position: point![0.0, 0.0],
            size: vector![6.0, 6.0],

            mouse_position: point![0.0, 0.0],
            view_direction: UnitVector2::new_normalize(vector![1.0, 0.0]),
            view_width: 120.0 * PI / 180.0,

            speed: 64.0,
            motion_input: DirectionalInput::default(),

            state: PlayerState::Active,
            history: History::default(),
            environment_history: SecondaryMap::default(),

            confusion: 0.0,
            paradox_position: None,

            view_area: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum PlayerState {
    Active,
    Recording,
    Dead,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlayerHistoryEntry {
    pub position: Point2<f32>,
    pub mouse_position: Point2<f32>,
}

impl Player {
    pub const CONFUSION_TIME: f64 = 0.1;
    pub const CONFUSION_FALLOFF_DISTANCE: f64 = 24.0;
    pub const CONFUSION_DISTANCE_THRESHOLD: f64 = 12.0;

    pub const RECOVERY_TIME: f64 = 5.0;

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
            if self.state == PlayerState::Dead {
                Color::new(0.5, 0.0, 0.0, 1.0)
            } else {
                Color::new(1.0, 0.0, 0.0, 1.0)
            },
        );
    }

    pub fn draw_question_mark(&self, texture_atlas: &Texture2D, confusion: f64, color: Color) {
        let source = rect_of_confusion_effect(confusion);
        let position = self.position.map(|x| x as f32) + CONFUSION_EFFECT_OFFSET;

        texture::draw_texture_ex(
            texture_atlas,
            position.x.round(),
            position.y.round(),
            color,
            DrawTextureParams {
                source: Some(source),
                ..Default::default()
            },
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

    pub fn paradox_level(
        &self,
        frame: FrameIndex,
        entities: &GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &LightGrid,
    ) -> Option<(f64, Point2<f64>)> {
        let view_area = self.view_area.as_ref()?;
        let mut exists = SecondaryMap::default();

        let mut error = None;
        let mut position = None;

        for (key, entity) in entities.iter() {
            exists.insert(key, ());

            let mut current_state @ Some(_) = entity.inner.visible_state() else {
                continue;
            };

            if !entity.inner.is_within_view_area(light_grid, view_area) {
                current_state = None;
            };

            let expected_state = self
                .environment_history
                .get(key)
                .and_then(|history| history.get(frame))
                .copied();

            let this_error = self.compare_states(current_state, expected_state);
            if this_error > error {
                error = this_error;
                position = Some(current_state.or(expected_state).unwrap().position());
            }
        }

        for (key, history) in &self.environment_history {
            if exists.contains_key(key) {
                continue;
            }

            if let Some(expected_state) = history.get(frame).copied() {
                let this_error = self.compare_states(None, Some(expected_state));
                if this_error > error {
                    error = this_error;
                    position = Some(expected_state.position());
                }
            }
        }

        error.zip(position)
    }

    pub fn compare_states(
        &self,
        current_state: Option<EntityVisibleState>,
        expected_state: Option<EntityVisibleState>,
    ) -> Option<f64> {
        if current_state != expected_state {
            let [current_distance, expected_distance] =
                [current_state, expected_state].map(|state| {
                    state
                        .map(|state| (state.position() - self.position).magnitude())
                        .unwrap_or(f64::INFINITY)
                });

            // Distance != f64::INFINITY: None == None, so we can't get two of them
            let distance = current_distance.min(expected_distance);

            let [current_angle, expected_angle] = [current_state, expected_state].map(|state| {
                state
                    .map(|state| (state.position() - self.position).angle(&self.view_direction))
                    .unwrap_or(f64::INFINITY)
            });

            // Angle != f64::INFINITY: None == None, so we can't get two of them
            let angle = current_angle.min(expected_angle);

            Some(
                (Self::CONFUSION_FALLOFF_DISTANCE / distance).clamp(0.0, 1.0)
                    * (1.0 - 2.0 * angle / self.view_width).clamp(0.25, 1.0),
            )
        } else {
            None
        }
    }
}

#[typetag::serde]
impl Entity for Player {
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
        _wire: Option<&mut Wire>,
    ) -> Option<GameAction> {
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
            PlayerState::Recording => {
                if let Some(entry) = self.history.get(frame) {
                    const MAXIMUM_TEMPORAL_OFFSET: usize = 2;

                    self.position = entry.position.map(|x| x as f64);
                    self.mouse_position = entry.mouse_position.map(|x| x as f64);

                    self.update_view_direction();

                    // Check a few previous and next frames as well to account for differences in
                    // entity insertion order.
                    if let Some(Some((paradox_level, paradox_position))) =
                        (frame - MAXIMUM_TEMPORAL_OFFSET..frame + MAXIMUM_TEMPORAL_OFFSET + 1)
                            .map(|frame| self.paradox_level(frame, &entities, light_grid))
                            .min_by(|a, b| {
                                a.unzip()
                                    .0
                                    .partial_cmp(&b.unzip().0)
                                    .unwrap_or(Ordering::Equal)
                            })
                    {
                        self.confusion += (paradox_level / Self::CONFUSION_TIME) * UPDATE_DT;
                        self.paradox_position = Some((paradox_level, paradox_position));
                    } else {
                        self.confusion -= (1.0 / Self::RECOVERY_TIME) * UPDATE_DT;
                        self.paradox_position = None;
                    }

                    if self.confusion > 1.0 {
                        self.state = PlayerState::Dead;
                    }
                }
            }
            PlayerState::Dead => {
                self.confusion -= (1.0 / Self::RECOVERY_TIME) * UPDATE_DT;
            }
        }

        self.confusion = self.confusion.clamp(0.0, 1.0);

        None
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
            PlayerState::Dead => None,
        };
    }

    fn travel_to_beginning(&mut self, past: &mut EntityTracker) {
        let old_self = past.inner.as_player().unwrap();

        if old_self.state == PlayerState::Active {
            old_self.state = PlayerState::Recording;
            old_self.history = mem::take(&mut self.history);
            old_self.environment_history = mem::take(&mut self.environment_history);
        }
    }

    fn draw_back(&mut self, _texture_atlas: &Texture2D) {
        match self.state {
            PlayerState::Dead => self.draw(),
            _ => (),
        }
    }

    fn draw_effect_back(&mut self, _texture_atlas: &Texture2D) {
        if let Some((_, paradox_position)) = self.paradox_position
            && self.state == PlayerState::Recording
            && self.confusion > 0.0
        {
            let mut color = 1.0;

            let mut position = self.position.map(|x| x as f32);

            let displacement = (paradox_position - self.position).map(|x| x as f32);
            let distance = displacement.magnitude();

            let spacing = 4.0 / self.confusion as f32;
            let offset = displacement / distance * spacing;

            let size = 1.0;

            let iterations = distance / spacing;

            for _ in 0..iterations.ceil() as usize {
                shapes::draw_rectangle(
                    (position.x - size / 2.0).round(),
                    (position.y - size / 2.0).round(),
                    size,
                    size,
                    Color::new(
                        1.0,
                        if color < self.confusion { 0.0 } else { 1.0 },
                        0.0,
                        1.0,
                    ),
                );

                position += offset;
                color += 1.0 / iterations as f64;
                color %= 1.0;
            }
        }
    }

    fn draw_front(&mut self, _texture_atlas: &Texture2D) {
        match self.state {
            PlayerState::Active | PlayerState::Recording => self.draw(),
            _ => (),
        }
    }

    fn draw_effect_front(&mut self, texture_atlas: &Texture2D) {
        match self.state {
            PlayerState::Recording => {
                if self.confusion > 0.0 {
                    self.draw_question_mark(texture_atlas, self.confusion, colors::WHITE);
                }
            }
            PlayerState::Dead => {
                if self.confusion > 0.0 {
                    self.draw_question_mark(
                        texture_atlas,
                        1.0,
                        Color {
                            a: self.confusion as f32,
                            ..colors::WHITE
                        },
                    );
                }
            }
            _ => (),
        }
    }

    fn is_within_view_area(&self, light_grid: &LightGrid, view_area: &LightArea) -> bool {
        self.edges()
            .into_iter()
            .any(|line| view_area.edge_intersects_line(line))
            || (view_area
                .range
                .is_none_or(|range| range.contains_offset(self.position - view_area.origin))
                || (self.position - view_area.origin).magnitude_squared()
                    <= Self::CONFUSION_DISTANCE_THRESHOLD.powi(2))
                && light_grid.contains_path(view_area.origin, self.position)
    }

    fn visible_state(&self) -> Option<EntityVisibleState> {
        Some(EntityVisibleState::new(
            self.position,
            (self.state == PlayerState::Dead) as u64,
        ))
    }

    fn collision_rect(&self) -> Option<TileRect> {
        Some(TileRect::from_rect_inclusive(self.collision_rect()))
    }

    fn view_area(&self) -> Option<LightArea> {
        self.view_area.clone()
    }

    fn view_kind(&self) -> Option<ViewKind> {
        match self.state {
            PlayerState::Active => Some(ViewKind::Present),
            PlayerState::Recording => Some(ViewKind::Past {
                confusion: self.confusion,
            }),
            PlayerState::Dead => None,
        }
    }

    fn position(&self) -> Point2<f64> {
        self.position
    }

    fn position_mut(&mut self) -> Option<&mut Point2<f64>> {
        Some(&mut self.position)
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(self.clone())
    }

    fn is_dead(&self) -> bool {
        self.state == PlayerState::Dead
    }

    fn should_recieve_inputs(&self) -> bool {
        self.state == PlayerState::Active
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

    fn as_player(&mut self) -> Option<&mut Player> {
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
