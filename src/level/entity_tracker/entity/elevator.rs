use macroquad::math::Rect;
use nalgebra::{Point2, Scalar, Vector2, vector};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{
                Entity,
                elevator_door::{ElevatorDoor, ElevatorDoorOrientation},
            },
        },
        light_grid::LightGrid,
    },
};

pub const ELEVATOR_SIZE: Vector2<f64> = vector![16.0, 16.0];

#[derive(Clone, Debug)]
pub struct Elevator {
    pub position: Point2<f64>,
    pub direction: ElevatorDirection,
    pub door: Option<EntityKey>,

    pub closing_time: Option<FrameIndex>,
    pub occupants: Vec<EntityKey>,
}

impl Elevator {
    pub fn new(position: Point2<f64>, direction: ElevatorDirection) -> Self {
        Self {
            position,
            direction,
            door: None,

            closing_time: None,
            occupants: Vec::new(),
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
}

impl Entity for Elevator {
    fn update(
        &mut self,
        frame: FrameIndex,
        mut entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) {
        if let Some(door) = self.door {
            let collision_rect = TileRect::from_rect_inclusive(self.collision_rect());

            if self.closing_time.is_none() {
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

            let mut close = false;

            let door = entities[door].inner.as_door().unwrap();
            door.open = if let Some(closing_time) = self.closing_time {
                frame < closing_time
            } else {
                if !door.open && !door.blocked {
                    close = true;
                }
                self.occupants.is_empty()
            };

            if close {
                self.closing_time = Some(frame);
                initial_state[*entities.protected_slot()]
                    .inner
                    .as_elevator()
                    .unwrap()
                    .closing_time = self.closing_time;
            }
        }
    }

    fn position(&self) -> Point2<f64> {
        self.position
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
