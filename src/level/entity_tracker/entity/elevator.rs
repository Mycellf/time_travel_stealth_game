use std::f64::consts::PI;

use macroquad::{
    color::colors,
    math::Rect,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Scalar, Vector2, point, vector};
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Elevator {
    pub position: Point2<f64>,
    pub direction: ElevatorDirection,
    pub door: Option<EntityKey>,

    pub closing_time: Option<FrameIndex>,
    pub hold_open: bool,
    pub available: bool,
    pub broken: bool,
    pub occupants: Vec<EntityKey>,
    #[serde(skip, default = "bool_true")]
    pub unlocked: bool,

    pub delay: Option<usize>,
    pub closed: bool,

    pub action: GameAction,
    pub input: Option<EntityKey>,
}

fn bool_true() -> bool {
    true
}

impl Elevator {
    pub fn new(position: Point2<f64>, direction: ElevatorDirection, action: GameAction) -> Self {
        Self {
            position,
            direction,
            door: None,

            closing_time: None,
            hold_open: true,
            available: true,
            broken: false,
            occupants: Vec::new(),
            unlocked: true,

            delay: None,
            closed: false,

            action,
            input: None,
        }
    }

    pub fn inner_collision_rect(&self) -> Rect {
        let corner = self.position - ELEVATOR_SIZE_INNER / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            ELEVATOR_SIZE_INNER.x as f32,
            ELEVATOR_SIZE_INNER.y as f32,
        )
    }

    pub fn outer_collision_rect(&self) -> Rect {
        let corner = self.position - ELEVATOR_SIZE_OUTER / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            ELEVATOR_SIZE_OUTER.x as f32,
            ELEVATOR_SIZE_OUTER.y as f32,
        )
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

#[typetag::serde]
impl Entity for Elevator {
    fn update(
        &mut self,
        frame: FrameIndex,
        mut entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        let Some(door) = self.door else {
            return None;
        };

        let collision_rect = TileRect::from_rect_inclusive(self.inner_collision_rect());

        if let Some(_) = self.closing_time {
            for &key in &self.occupants {
                if entities[key].inner.is_dead() {
                    self.broken = true;
                }
            }
        } else {
            self.occupants.clear();
            self.occupants.extend(
                entities
                    .iter()
                    .filter(|(_, entity)| {
                        entity
                            .inner
                            .collision_rect()
                            .is_some_and(|rect| rect.intersects(&collision_rect))
                    })
                    .map(|(key, _)| key),
            );
        }

        if (self.hold_open || entities[door].inner.as_door().unwrap().blocked)
            && self.closing_time.is_none()
            && matches!(
                self.action,
                GameAction::HardResetKeepPlayer | GameAction::HardReset
            )
        {
            let collision_rect = TileRect::from_rect_inclusive(self.outer_collision_rect());

            let occupants = entities
                .iter()
                .filter(|(_, entity)| {
                    entity
                        .inner
                        .collision_rect()
                        .is_some_and(|rect| rect.intersects(&collision_rect))
                })
                .map(|(key, _)| key)
                .collect::<Vec<_>>();

            for occupant in occupants {
                let occupant = &mut entities[occupant];
                if occupant.inner.is_dead() {
                    if let Some(position) = occupant.inner.position_mut() {
                        *position += self.direction.offset::<f64>() * 8.0 * UPDATE_DT;
                    }
                }
            }
        }

        let key = *entities.protected_slot();

        let door = entities[door].inner.as_door().unwrap();
        if let Some(closing_time) = self.closing_time {
            if door.blocked || self.broken {
                self.broken = true;
                door.open = true;
            } else {
                door.open = frame < closing_time;

                if frame == closing_time && !self.unlocked {
                    door.open = false;
                    self.broken = true;
                }
            }
        } else {
            let was_open = door.open;
            door.open = self.occupants.is_empty();
            if self.hold_open {
                if door.open {
                    self.hold_open = false;
                } else {
                    door.open = true;
                }
            }

            if !was_open && !door.open && !door.blocked {
                self.closing_time = Some(frame.saturating_sub(1));
                let initial = initial_state[key].inner.as_elevator().unwrap();
                initial.closing_time = self.closing_time;
                initial.occupants.clone_from(&self.occupants);
            }
        }

        if self.delay.is_none()
            && door.extent == 16
            && !door.open
            && self.closing_time.is_some()
            && !self.broken
            && !self.closed
        {
            self.delay = Some(UPDATE_TPS * 3);
            self.closed = true;
        }

        if let Some(delay) = &mut self.delay {
            if !self.available {
                self.delay = None;

                let actual_occupants = entities
                    .iter()
                    .filter(|(_, entity)| {
                        entity
                            .inner
                            .collision_rect()
                            .is_some_and(|rect| rect.intersects(&collision_rect))
                    })
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>();

                self.broken |= 'verify_contents: {
                    // Check for extra occupants
                    for &key in &actual_occupants {
                        if !self.occupants.contains(&key) {
                            break 'verify_contents true;
                        }
                    }

                    // Check for missing occupants
                    for &key in &self.occupants {
                        if !actual_occupants.contains(&key) {
                            break 'verify_contents true;
                        }
                    }

                    false
                };

                if self.broken {
                    for &key in &self.occupants {
                        if let Some(player) = entities[key].inner.as_player() {
                            player.state = PlayerState::Dead;
                            player.confusion = 1.0;
                        }
                    }
                } else {
                    for &key in &self.occupants {
                        entities[key].inner = Box::new(Empty);
                    }
                }

                None
            } else if *delay == 0 {
                self.available = false;
                initial_state[key].inner.as_elevator().unwrap().available = self.available;
                if self.action == GameAction::SoftReset {
                    for &key in &self.occupants {
                        let entity = &mut entities[key];
                        entity.inner.travel_to_beginning(&mut initial_state[key]);

                        initial_state.insert(entity.clone());
                    }
                }

                Some(self.action.clone())
            } else {
                *delay -= 1;
                Some(GameAction::SetFadeOut)
            }
        } else {
            None
        }
    }

    fn draw_floor(&mut self, texture_atlas: &Texture2D) {
        texture::draw_texture_ex(
            texture_atlas,
            self.position.x as f32 - ELEVATOR_FLOOR_TEXTURE_SIZE.x / 2.0,
            self.position.y as f32 - ELEVATOR_FLOOR_TEXTURE_SIZE.y / 2.0,
            colors::WHITE,
            DrawTextureParams {
                source: Some(Rect::new(
                    ELEVATOR_FLOOR_TEXTURE_POSITION.x,
                    ELEVATOR_FLOOR_TEXTURE_POSITION.y,
                    ELEVATOR_FLOOR_TEXTURE_SIZE.x,
                    ELEVATOR_FLOOR_TEXTURE_SIZE.y,
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
                source: Some(Rect::new(
                    ELEVATOR_WALLS_TEXTURE_POSITION.x,
                    ELEVATOR_WALLS_TEXTURE_POSITION.y,
                    ELEVATOR_WALLS_TEXTURE_SIZE.x,
                    ELEVATOR_WALLS_TEXTURE_SIZE.y,
                )),
                rotation: self.direction.angle() as f32,
                ..Default::default()
            },
        );
    }

    fn draw_effect_back(&mut self, _texture_atlas: &Texture2D) {
        // shapes::draw_circle(
        //     self.position.x as f32,
        //     self.position.y as f32,
        //     4.0,
        //     color::WHITE,
        // );
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

    fn evaluate(
        &mut self,
        mut entities: GuardedSlotMap<EntityKey, EntityTracker>,
        inputs: &[bool],
    ) -> bool {
        self.unlocked = inputs.get(0).copied().unwrap_or(true);

        let Some(door) = self.door else { return false };
        let door = entities[door].inner.as_door().unwrap();

        self.unlocked && !door.open && door.extent == 16 && !self.broken
    }

    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        Some(self)
    }
}
