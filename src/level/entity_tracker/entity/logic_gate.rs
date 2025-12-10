use nalgebra::Point2;
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LogicGate {
    pub position: Point2<f64>,
    pub kind: LogicGateKind,
    pub inputs: Vec<EntityKey>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum LogicGateKind {
    And,
    Or,
    Not,
    Passthrough,
}

#[typetag::serde]
impl Entity for LogicGate {
    fn update(
        &mut self,
        _frame: FrameIndex,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _light_grid: &mut LightGrid,
        _initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction> {
        None
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

    fn should_recieve_inputs(&self) -> bool {
        false
    }

    fn inputs(&self) -> &[EntityKey] {
        &self.inputs
    }

    fn try_add_input(&mut self, key: EntityKey) {
        if !self.inputs.contains(&key) {
            self.inputs.push(key);
        }
    }

    fn try_remove_input(&mut self, key: EntityKey) {
        if let Some(i) = self.inputs.iter().position(|&input| input == key) {
            self.inputs.remove(i);
        }
    }

    fn evaluate(
        &mut self,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        inputs: &[bool],
    ) -> bool {
        match self.kind {
            LogicGateKind::And => inputs.iter().copied().reduce(|a, b| a && b),
            LogicGateKind::Or => inputs.iter().copied().reduce(|a, b| a || b),
            LogicGateKind::Not => inputs.first().copied().map(|x| !x),
            LogicGateKind::Passthrough => inputs.first().copied(),
        }
        .unwrap_or_default()
    }
}
