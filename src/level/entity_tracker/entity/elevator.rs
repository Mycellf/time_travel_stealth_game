use std::{
    f64::consts::{PI, TAU},
    mem,
};

use macroquad::{
    color::{Color, colors},
    math::Rect,
    shapes,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Scalar, UnitComplex, Vector2, point, vector};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey, UPDATE_DT, UPDATE_TPS,
        entity_tracker::{
            EntityTracker,
            entity::{
                Entity, GameAction,
                elevator_door::{ElevatorDoor, ElevatorDoorOrientation},
                empty::Empty,
                logic_gate::{self, LogicGate},
                player::PlayerState,
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

    #[serde(skip)]
    pub powered: Option<bool>,
    #[serde(skip)]
    pub animation_state: u16,
    #[serde(skip)]
    pub door: Option<EntityKey>,
    #[serde(skip)]
    pub state: ElevatorState,
    #[serde(skip)]
    pub sparks: Vec<Spark>,
}

#[derive(Clone, Copy, Debug)]
pub struct Spark {
    pub position: Point2<f64>,
    pub velocity: Vector2<f64>,
    pub color: bool,
    pub age: u16,
    pub flight_time: u16,
    pub max_age: u16,
}

#[derive(Clone, Debug)]
pub enum ElevatorState {
    Running {
        held_open: bool,
        state: ElevatorRunningState,
    },
    Closing {
        close_time: FrameIndex,
        expected_occupants: Option<Vec<EntityKey>>,
    },
    Waiting {
        close_time: FrameIndex,
        remaining_time: usize,
    },
    Used,
    Explode,
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

            powered: None,
            animation_state: 0,
            door: None,
            state: ElevatorState::default(),
            sparks: Vec::new(),
        }
    }

    pub fn is_door_open(&self) -> bool {
        match self.state {
            ElevatorState::Running { held_open, .. } => held_open || self.powered.unwrap_or(true),
            ElevatorState::Closing { .. } => false,
            ElevatorState::Waiting { .. } => false,
            ElevatorState::Used => false,
            ElevatorState::Explode => true,
            ElevatorState::Broken => true,
        }
    }

    pub fn is_loop_complete(&self) -> bool {
        match self.state {
            ElevatorState::Running { .. } => false,
            ElevatorState::Closing { .. } => false,
            ElevatorState::Waiting { .. } => true,
            ElevatorState::Used => true,
            ElevatorState::Explode => false,
            ElevatorState::Broken => false,
        }
    }

    pub fn is_symbol_bright(&self) -> bool {
        match self.state {
            ElevatorState::Running { .. } => true,
            ElevatorState::Closing { .. } => false,
            ElevatorState::Waiting { .. } => false,
            ElevatorState::Used => false,
            ElevatorState::Explode => false,
            ElevatorState::Broken => false,
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

    pub fn color_of_symbol(&self) -> Color {
        let is_bright = self.is_symbol_bright();

        if self.powered.is_some() && is_bright {
            logic_gate::power_color(self.animation_state)
        } else {
            // HACK: Adjusted for the wierd alpha effect we get from using a render target.
            // Roughly follows (desired alpha) ** 0.317906
            let brightness = if is_bright { 0.8 } else { 0.6 };
            Color::new(1.0, 1.0, 1.0, brightness)
        }
    }

    pub fn add_spark(&mut self) {
        const SPARK_VELOCITY: f64 = 128.0;

        let max_age = macroquad::rand::gen_range(UPDATE_TPS as u16 * 1 / 2, UPDATE_TPS as u16 * 1);

        self.sparks.push(Spark {
            position: self.position
                + vector![
                    macroquad::rand::gen_range(-ELEVATOR_SIZE_INNER.x, ELEVATOR_SIZE_INNER.x - 1.0),
                    macroquad::rand::gen_range(-ELEVATOR_SIZE_INNER.y, ELEVATOR_SIZE_INNER.y - 1.0),
                ] / 2.0,
            velocity: UnitComplex::new(macroquad::rand::gen_range(0.0, TAU))
                * vector![
                    macroquad::rand::gen_range(SPARK_VELOCITY / 2.0, SPARK_VELOCITY),
                    0.0,
                ],
            color: false,
            age: 0,
            flight_time: max_age
                - macroquad::rand::gen_range(
                    UPDATE_TPS as u16 * 1 / 20,
                    UPDATE_TPS as u16 * 1 / 10,
                ),
            max_age,
        })
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
        self.animation_state = if self.powered.unwrap_or(false) {
            self.animation_state
                .saturating_add(LogicGate::ANIMATION_STEP)
        } else {
            self.animation_state
                .saturating_sub(LogicGate::ANIMATION_STEP)
        };

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
                }

                match state {
                    ElevatorRunningState::Active => {
                        let intersections = Self::intersections(
                            entities.iter(),
                            Self::outer_collision_rect(self.position),
                        )
                        .collect::<Vec<_>>();

                        if matches!(
                            self.action,
                            GameAction::HardReset | GameAction::HardResetKeepPlayer
                        ) {
                            for key in &intersections {
                                let entity = &mut entities[*key];
                                if entity.inner.is_dead() {
                                    if let Some(position) = entity.inner.position_mut() {
                                        *position +=
                                            self.direction.offset::<f64>() * 12.0 * UPDATE_DT;
                                    }
                                }
                            }
                        }

                        if !*held_open {
                            let door = Self::get_door(self.door, &mut entities);

                            if door.open != self.powered.unwrap_or(true) {
                                door.open = self.powered.unwrap_or(true);
                                door.update_light_grid(light_grid);
                            }

                            let occupants = Self::occupants(
                                entities.iter(),
                                Self::inner_collision_rect(self.position),
                            )
                            .collect::<Vec<_>>();

                            if !occupants.is_empty() && occupants == intersections {
                                let door = Self::get_door(self.door, &mut entities);

                                door.open = false;
                                door.update_light_grid(light_grid);
                                self.state = ElevatorState::Closing {
                                    close_time: frame,
                                    expected_occupants: None,
                                };

                                return Some(GameAction::StartFadeOut);
                            }
                        }
                    }
                    ElevatorRunningState::Recording {
                        close_time,
                        expected_occupants,
                    } => {
                        let door = Self::get_door(self.door, &mut entities);

                        if frame < *close_time {
                            if !*held_open {
                                if door.open != self.powered.unwrap_or(true) {
                                    door.open = self.powered.unwrap_or(true);
                                    door.update_light_grid(light_grid);
                                }
                            }

                            for &mut key in expected_occupants {
                                if entities[key].inner.is_dead() {
                                    let door = Self::get_door(self.door, &mut entities);
                                    door.open = true;
                                    door.update_light_grid(light_grid);
                                    self.state = ElevatorState::Explode;

                                    break;
                                }
                            }
                        } else {
                            door.open = false;
                            door.update_light_grid(light_grid);
                            self.state = ElevatorState::Closing {
                                close_time: frame,
                                expected_occupants: Some(mem::take(expected_occupants)),
                            };
                        }
                    }
                }
            }
            ElevatorState::Closing {
                close_time,
                expected_occupants,
            } => {
                let door = Self::get_door(self.door, &mut entities);

                if door.blocked {
                    door.open = true;
                    door.update_light_grid(light_grid);
                    self.state = ElevatorState::Explode;
                } else if door.extent == 16 {
                    if let Some(expected_occupants) = expected_occupants {
                        let occupants = Self::occupants(
                            entities.iter(),
                            Self::inner_collision_rect(self.position),
                        )
                        .collect::<Vec<_>>();

                        let broken = occupants
                            .iter()
                            .any(|occupant| !expected_occupants.contains(occupant))
                            || expected_occupants
                                .iter()
                                .any(|expected_occupant| !occupants.contains(expected_occupant));

                        if broken {
                            for &mut expected_occupant in expected_occupants {
                                if let Some(player) = entities[expected_occupant].inner.as_player()
                                {
                                    player.state = PlayerState::Dead;
                                    player.confusion = 1.0;
                                }
                            }

                            let door = Self::get_door(self.door, &mut entities);

                            door.open = true;
                            door.update_light_grid(light_grid);
                            self.state = ElevatorState::Explode;

                            return None;
                        }

                        for &mut expected_occupant in expected_occupants {
                            entities[expected_occupant].inner = Box::new(Empty);
                        }

                        self.state = ElevatorState::Used;
                    } else {
                        self.state = ElevatorState::Waiting {
                            close_time: *close_time,
                            remaining_time: UPDATE_TPS,
                        };
                    }
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
            ElevatorState::Explode => {
                for _ in 0..macroquad::rand::gen_range(40, 60) {
                    self.add_spark();
                }

                self.state = ElevatorState::Broken;
            }
            ElevatorState::Broken => {
                const SPARKS_PER_SECOND: usize = 2;

                if macroquad::rand::gen_range(1, UPDATE_TPS) <= SPARKS_PER_SECOND {
                    self.add_spark();
                }

                self.sparks.retain_mut(|spark| {
                    const SPARK_DRAG: f64 = 0.95;
                    const SPARK_BOUNCE_ELASTICITY: f64 = 0.85;

                    if spark.age < spark.flight_time {
                        let old_position = spark.position.x;
                        spark.position.x += spark.velocity.x * UPDATE_DT;
                        if light_grid[spark.position.map(|x| x.round() as isize)].blocks_motion() {
                            spark.position.x = old_position;

                            spark.velocity.x *= -SPARK_BOUNCE_ELASTICITY;
                            spark.velocity.y *= SPARK_BOUNCE_ELASTICITY;
                        }

                        let old_position = spark.position.y;
                        spark.position.y += spark.velocity.y * UPDATE_DT;
                        if light_grid[spark.position.map(|x| x.round() as isize)].blocks_motion() {
                            spark.position.y = old_position;

                            spark.velocity.y *= -SPARK_BOUNCE_ELASTICITY;
                            spark.velocity.x *= SPARK_BOUNCE_ELASTICITY;
                        }

                        spark.velocity *= SPARK_DRAG;
                    } else {
                        spark.velocity = vector![0.0, 0.0];
                    }

                    if macroquad::rand::gen_range(0, spark.max_age)
                        < (spark.max_age - spark.age) / 5
                    {
                        spark.color ^= true;
                    }

                    spark.age = spark.age.saturating_add(1);
                    spark.age < spark.max_age
                })
            }
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
        self.draw_symbol(texture_atlas, colors::WHITE);
    }

    fn draw_effect_back(&mut self, texture_atlas: &Texture2D) {
        self.draw_symbol(texture_atlas, self.color_of_symbol());
    }

    fn draw_effect_front(&mut self, _texture_atlas: &Texture2D) {
        for spark in &self.sparks {
            shapes::draw_rectangle(
                spark.position.x.round() as f32,
                spark.position.y.round() as f32,
                1.0,
                1.0,
                if spark.color {
                    Color::new(1.0, 1.0, 0.5, 1.0)
                } else {
                    Color::new(0.0, 1.0, 1.0, 1.0)
                },
            );
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

    fn asynchronous_output(&self) -> Option<bool> {
        Some(self.is_loop_complete())
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
        if self.powered.is_none() {
            self.animation_state = if inputs.get(0).copied().unwrap_or(false) {
                u16::MAX
            } else {
                0
            };
        }

        self.powered = inputs.get(0).copied();

        self.is_loop_complete()
    }

    fn offset_of_wire(&self, wire_end: Vector2<f64>) -> Vector2<f64> {
        const DISTANCE: f64 = 12.0;

        wire_end.map(|x| x.clamp(-DISTANCE, DISTANCE))
    }

    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        Some(self)
    }
}
