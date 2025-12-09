use nalgebra::{Point2, Scalar, Vector2, point, vector};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
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

#[derive(Clone, Debug)]
pub struct Elevator {
    pub position: Point2<f64>,
    pub direction: ElevatorDirection,
    pub door: Option<EntityKey>,

    pub used: bool,
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
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) {
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
}
