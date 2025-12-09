use std::f64::consts::PI;

use macroquad::{
    color::colors,
    math::Rect,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Scalar, Vector2, point, vector};
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
                empty::Empty,
                player::PlayerState,
            },
        },
        light_grid::LightGrid,
    },
};

pub const ELEVATOR_SIZE: Vector2<f64> = vector![16.0, 16.0];

pub const ELEVATOR_FLOOR_TEXTURE_POSITION: Point2<f32> = point![8.0, 24.0];
pub const ELEVATOR_FLOOR_TEXTURE_SIZE: Vector2<f32> = vector![16.0, 16.0];

pub const ELEVATOR_WALLS_TEXTURE_POSITION: Point2<f32> = point![24.0, 24.0];
pub const ELEVATOR_WALLS_TEXTURE_SIZE: Vector2<f32> = vector![24.0, 24.0];

#[derive(Clone, Debug)]
pub struct Elevator {
    pub position: Point2<f64>,
    pub direction: ElevatorDirection,
    pub door: Option<EntityKey>,

    pub closing_time: Option<FrameIndex>,
    pub available: bool,
    pub broken: bool,
    pub occupants: Vec<EntityKey>,

    pub delay: Option<usize>,
}

impl Elevator {
    pub fn new(position: Point2<f64>, direction: ElevatorDirection) -> Self {
        Self {
            position,
            direction,
            door: None,

            closing_time: None,
            available: true,
            broken: false,
            occupants: Vec::new(),

            delay: None,
        }
    }

    pub fn collision_rect(&self) -> Rect {
        let corner = self.position - ELEVATOR_SIZE / 2.0;

        Rect::new(
            corner.x as f32,
            corner.y as f32,
            ELEVATOR_SIZE.x as f32,
            ELEVATOR_SIZE.y as f32,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

        let collision_rect = TileRect::from_rect_inclusive(self.collision_rect());

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

        let key = *entities.protected_slot();

        let door = entities[door].inner.as_door().unwrap();
        if let Some(closing_time) = self.closing_time {
            if door.blocked || self.broken {
                self.broken = true;
                door.open = true;
            } else {
                door.open = frame < closing_time;
            }
        } else {
            let was_open = door.open;
            door.open = self.occupants.is_empty();

            if !was_open && !door.open && !door.blocked {
                self.closing_time = Some(frame);
                let initial = initial_state[key].inner.as_elevator().unwrap();
                initial.closing_time = self.closing_time;
                initial.occupants.clone_from(&self.occupants);
            }
        }

        if self.delay.is_none() && door.extent == 16 && !door.open && self.closing_time.is_some() {
            self.delay = Some(UPDATE_TPS * 3);
        }

        if let Some(delay) = &mut self.delay {
            if !self.available {
                for key in entities
                    .iter()
                    .filter(|(_, entity)| {
                        entity
                            .inner
                            .collision_rect()
                            .is_some_and(|rect| rect.intersects(&collision_rect))
                    })
                    .map(|(key, _)| key)
                {
                    if !self.occupants.contains(&key) {
                        self.broken = true;
                        self.delay = None;

                        break;
                    }
                }

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
                for &key in &self.occupants {
                    let entity = &mut entities[key];
                    entity.inner.travel_to_beginning(&mut initial_state[key]);

                    initial_state.insert(entity.clone());
                }

                Some(GameAction::SoftReset)
            } else {
                *delay -= 1;
                None
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

    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        Some(self)
    }
}
