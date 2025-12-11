use std::f64::consts::PI;

use macroquad::{
    color::{Color, colors},
    math::Rect,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Scalar, Vector2, point, vector};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey, UPDATE_TPS,
        entity_tracker::{
            EntityTracker,
            entity::{
                Entity, GameAction,
                elevator_door::{ElevatorDoor, ElevatorDoorOrientation},
            },
        },
        light_grid::LightGrid,
    },
};

pub const ELEVATOR_SIZE_INNER: Vector2<f64> = vector![16.0, 16.0];
pub const ELEVATOR_SIZE_OUTER: Vector2<f64> = vector![24.0, 24.0];

pub const ELEVATOR_FLOOR_TEXTURE_POSITION: Point2<f32> = point![8.0, 24.0];
pub const ELEVATOR_FLOOR_TEXTURE_SIZE: Vector2<f32> = vector![16.0, 16.0];

pub const ELEVATOR_WALLS_TEXTURE_POSITION: Point2<f32> = point![24.0, 24.0];
pub const ELEVATOR_WALLS_TEXTURE_SIZE: Vector2<f32> = vector![24.0, 24.0];

pub const ELEVATOR_SYMBOL_TEXTURE_POSITION: Point2<f32> = point![0.0, 40.0];
pub const ELEVATOR_SYMBOL_TEXTURE_SIZE: Vector2<f32> = vector![8.0, 8.0];

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Elevator {
    pub position: Point2<f64>,
    pub direction: ElevatorDirection,
    pub action: GameAction,
    pub input: Option<EntityKey>,
    pub powered_on: bool,

    #[serde(skip)]
    pub door: Option<EntityKey>,
    #[serde(skip)]
    pub state: ElevatorState,
}

#[derive(Clone, Debug)]
pub enum ElevatorState {
    Running {
        held_open: bool,
        state: ElevatorRunningState,
    },
    Closing {
        close_time: FrameIndex,
    },
    Waiting {
        close_time: FrameIndex,
        remaining_time: usize,
    },
    Used,
    Broken,
}

#[derive(Clone, Default, Debug)]
pub enum ElevatorRunningState {
    #[default]
    Active,
    Recording {
        close_time: FrameIndex,
        expected_occupants: Vec<EntityKey>,
    },
}

impl Default for ElevatorState {
    fn default() -> Self {
        Self::Running {
            held_open: true,
            state: ElevatorRunningState::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ElevatorDirection {
    East,
    North,
    West,
    South,
}

impl ElevatorDirection {
    pub fn offset<T: From<i8> + Scalar>(self) -> Vector2<T> {
        match self {
            ElevatorDirection::East => vector![1, 0],
            ElevatorDirection::North => vector![0, -1],
            ElevatorDirection::West => vector![-1, 0],
            ElevatorDirection::South => vector![0, 1],
        }
        .map(|x| x.into())
    }

    pub fn angle(self) -> f64 {
        match self {
            ElevatorDirection::East => 0.0,
            ElevatorDirection::North => PI * 1.5,
            ElevatorDirection::West => PI,
            ElevatorDirection::South => PI * 0.5,
        }
    }
}

impl Elevator {
    pub fn new(position: Point2<f64>, direction: ElevatorDirection, action: GameAction) -> Self {
        Self {
            position,
            direction,
            action,
            input: None,
            powered_on: true,

            door: None,
            state: ElevatorState::default(),
        }
    }

    pub fn is_door_open(&self) -> bool {
        match self.state {
            ElevatorState::Running { held_open, .. } => held_open || self.powered_on,
            ElevatorState::Closing { .. } => false,
            ElevatorState::Waiting { .. } => false,
            ElevatorState::Used => false,
            ElevatorState::Broken => true,
        }
    }

    pub fn is_marker_bright(&self) -> bool {
        match self.state {
            ElevatorState::Running { .. } => self.powered_on,
            ElevatorState::Closing { .. } => false,
            ElevatorState::Waiting { .. } => false,
            ElevatorState::Used => false,
            ElevatorState::Broken => true,
        }
    }

    pub fn intersections<'a>(
        entities: impl IntoIterator<Item = (EntityKey, &'a EntityTracker)>,
        rectangle: Rect,
    ) -> impl Iterator<Item = EntityKey> {
        Self::intersections_by(entities, rectangle, TileRect::intersects)
    }

    pub fn occupants<'a>(
        entities: impl IntoIterator<Item = (EntityKey, &'a EntityTracker)>,
        rectangle: Rect,
    ) -> impl Iterator<Item = EntityKey> {
        Self::intersections_by(entities, rectangle, TileRect::contains)
    }

    fn intersections_by<'a>(
        entities: impl IntoIterator<Item = (EntityKey, &'a EntityTracker)>,
        rectangle: Rect,
        mut function: impl FnMut(&TileRect, &TileRect) -> bool,
    ) -> impl Iterator<Item = EntityKey> {
        let collision_rect = TileRect::from_rect_inclusive(rectangle);

        entities
            .into_iter()
            .filter(move |&(_, entity)| {
                entity
                    .inner
                    .collision_rect()
                    .is_some_and(|rect| function(&collision_rect, &rect))
            })
            .map(|(key, _)| key)
    }

    pub fn get_door<'a>(
        key: Option<EntityKey>,
        entities: &'a mut GuardedSlotMap<EntityKey, EntityTracker>,
    ) -> &'a mut ElevatorDoor {
        entities[key.expect("Elevators should have a door")]
            .inner
            .as_door()
            .expect("The door should be a door")
    }

    pub fn inner_collision_rect(position: Point2<f64>) -> Rect {
        let corner = position - ELEVATOR_SIZE_INNER / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            ELEVATOR_SIZE_INNER.x as f32,
            ELEVATOR_SIZE_INNER.y as f32,
        )
    }

    pub fn outer_collision_rect(position: Point2<f64>) -> Rect {
        let corner = position - ELEVATOR_SIZE_OUTER / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            ELEVATOR_SIZE_OUTER.x as f32,
            ELEVATOR_SIZE_OUTER.y as f32,
        )
    }

    pub fn draw_symbol(&self, texture_atlas: &Texture2D, color: Color) {
        let position = self.position.map(|x| x as f32) + 17.0 * self.direction.offset::<f32>()
            - ELEVATOR_SYMBOL_TEXTURE_SIZE / 2.0;

        texture::draw_texture_ex(
            texture_atlas,
            position.x,
            position.y,
            color,
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    ELEVATOR_SYMBOL_TEXTURE_POSITION
                        + vector![
                            match self.action {
                                GameAction::HardResetKeepPlayer => 0.0,
                                GameAction::SoftReset => 8.0,
                                GameAction::LoadLevel(_) => 16.0,
                                _ => 0.0,
                            },
                            0.0
                        ],
                    ELEVATOR_SYMBOL_TEXTURE_SIZE,
                )),
                ..Default::default()
            },
        );
    }
}

#[typetag::serde]
impl Entity for Elevator {
    fn update(
        &mut self,
        frame: FrameIndex,
        mut entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        match &mut self.state {
            ElevatorState::Running { held_open, state } => {
                if *held_open {
                    let empty = Self::intersections(
                        entities.iter(),
                        Self::outer_collision_rect(self.position),
                    )
                    .next()
                    .is_none();

                    if empty {
                        *held_open = false;
                    }
                } else {
                    let door = Self::get_door(self.door, &mut entities);

                    if door.open != self.powered_on {
                        door.open = self.powered_on;
                        door.update_light_grid(light_grid);
                    }
                }

                match state {
                    ElevatorRunningState::Active => {
                        // TODO: Deal with the "dies in the entry elevator" softlock scenario

                        if !*held_open {
                            let occupants = Self::occupants(
                                entities.iter(),
                                Self::inner_collision_rect(self.position),
                            )
                            .collect::<Vec<_>>();

                            let intersections = Self::intersections(
                                entities.iter(),
                                Self::outer_collision_rect(self.position),
                            )
                            .collect::<Vec<_>>();

                            if !occupants.is_empty() && occupants == intersections {
                                let door = Self::get_door(self.door, &mut entities);

                                door.open = false;
                                door.update_light_grid(light_grid);
                                self.state = ElevatorState::Closing { close_time: frame };
                            }
                        }
                    }
                    ElevatorRunningState::Recording {
                        close_time,
                        expected_occupants,
                    } => {
                        // TODO: Close the door

                        for &mut key in expected_occupants {
                            if entities[key].inner.is_dead() {
                                let door = Self::get_door(self.door, &mut entities);
                                door.open = true;
                                door.update_light_grid(light_grid);
                                self.state = ElevatorState::Broken;

                                break;
                            }
                        }
                    }
                }
            }
            ElevatorState::Closing { close_time } => {
                let door = Self::get_door(self.door, &mut entities);

                if door.extent == 16 {
                    self.state = ElevatorState::Waiting {
                        close_time: *close_time,
                        remaining_time: UPDATE_TPS,
                    };
                }
            }
            ElevatorState::Waiting {
                close_time,
                remaining_time,
            } => {
                if *remaining_time > 0 {
                    *remaining_time = remaining_time.saturating_sub(1);
                } else {
                    let occupants =
                        Self::occupants(entities.iter(), Self::inner_collision_rect(self.position))
                            .collect::<Vec<_>>();

                    if matches!(self.action, GameAction::SoftReset) {
                        for &key in &occupants {
                            let entity = &mut entities[key];
                            entity.inner.travel_to_beginning(&mut initial_state[key]);
                            initial_state.insert(entity.clone());
                        }

                        let next_state = initial_state[*entities.protected_slot()]
                            .inner
                            .as_elevator()
                            .expect("Initial state of elevator should be an elevator");

                        next_state.state = ElevatorState::Running {
                            held_open: true,
                            state: ElevatorRunningState::Recording {
                                close_time: *close_time,
                                expected_occupants: occupants,
                            },
                        };
                    }

                    self.state = ElevatorState::Used;

                    return Some(self.action.clone());
                }
            }
            ElevatorState::Used => (),
            ElevatorState::Broken => (),
        }

        None
    }

    fn draw_floor(&mut self, texture_atlas: &Texture2D) {
        texture::draw_texture_ex(
            texture_atlas,
            self.position.x as f32 - ELEVATOR_FLOOR_TEXTURE_SIZE.x / 2.0,
            self.position.y as f32 - ELEVATOR_FLOOR_TEXTURE_SIZE.y / 2.0,
            colors::WHITE,
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    ELEVATOR_FLOOR_TEXTURE_POSITION,
                    ELEVATOR_FLOOR_TEXTURE_SIZE,
                )),
                ..Default::default()
            },
        );
    }

    fn draw_wall(&mut self, texture_atlas: &Texture2D) {
        texture::draw_texture_ex(
            texture_atlas,
            self.position.x as f32 - ELEVATOR_WALLS_TEXTURE_SIZE.x / 2.0,
            self.position.y as f32 - ELEVATOR_WALLS_TEXTURE_SIZE.y / 2.0,
            colors::WHITE,
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    ELEVATOR_WALLS_TEXTURE_POSITION,
                    ELEVATOR_WALLS_TEXTURE_SIZE,
                )),
                rotation: self.direction.angle() as f32,
                ..Default::default()
            },
        );
    }

    fn draw_back(&mut self, texture_atlas: &Texture2D) {
        self.draw_symbol(
            texture_atlas,
            if matches!(self.state, ElevatorState::Broken) {
                Color::new(1.0, 0.0, 0.0, 1.0)
            } else {
                colors::WHITE
            },
        );
    }

    fn draw_effect_back(&mut self, texture_atlas: &Texture2D) {
        self.draw_symbol(
            texture_atlas,
            Color {
                a: if self.is_marker_bright() { 0.5 } else { 0.2 },
                ..if matches!(self.state, ElevatorState::Broken) {
                    Color::new(1.0, 0.0, 0.0, 1.0)
                } else {
                    colors::WHITE
                }
            },
        );
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

    fn spawn(&mut self, _key: EntityKey, entities: &mut SlotMap<EntityKey, EntityTracker>) {
        if self.door.is_none() {
            self.door = Some(entities.insert(EntityTracker::new(Box::new(ElevatorDoor {
                position: self.position + 10.0 * self.direction.offset::<f64>(),
                extent: 16,
                open: true,
                blocked: false,
                lighting_needs_update: true,
                orientation: match self.direction {
                    ElevatorDirection::East | ElevatorDirection::West => {
                        ElevatorDoorOrientation::Vertical
                    }
                    ElevatorDirection::North | ElevatorDirection::South => {
                        ElevatorDoorOrientation::Horizontal
                    }
                },
            }))));
        }
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }

    fn inputs(&self) -> &[EntityKey] {
        self.input.as_slice()
    }

    fn try_add_input(&mut self, key: EntityKey) {
        if self.input.is_none() {
            self.input = Some(key);
        }
    }

    fn try_remove_input(&mut self, key: EntityKey) {
        if self.input == Some(key) {
            self.input = None;
        }
    }

    fn evaluate(
        &mut self,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        inputs: &[bool],
    ) -> bool {
        self.powered_on = inputs.get(0).copied().unwrap_or(true);

        matches!(self.state, ElevatorState::Used)
    }

    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        Some(self)
    }
}
