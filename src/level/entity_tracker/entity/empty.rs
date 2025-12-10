use nalgebra::{Point2, point};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, GameAction},
            wire_diagram::Wire,
        },
        light_grid::LightGrid,
    },
};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Empty;

#[typetag::serde]
impl Entity for Empty {
    fn update(
        &mut self,
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
        _wire: Option<&mut Wire>,
    ) -> Option<GameAction> {
        None
    }

    fn position(&self) -> Point2<f64> {
        point![0.0, 0.0]
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(Empty)
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }
}
