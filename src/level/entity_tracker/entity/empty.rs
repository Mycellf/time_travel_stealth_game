use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, GameAction},
        },
        light_grid::LightGrid,
    },
};

#[derive(Clone, Copy, Debug)]
pub struct Empty;

impl Entity for Empty {
    fn update(
        &mut self,
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        None
    }

    fn duplicate(&self) -> Box<dyn Entity> {
        Box::new(Empty)
    }

    fn should_recieve_inputs(&self) -> bool {
        false
    }
}
