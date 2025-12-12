use std::f64::consts::PI;

use macroquad::{
    color::Color,
    texture::{self, DrawTextureParams, Texture2D},
};
use nalgebra::{Point2, Vector2, point, vector};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap},
    level::{
        EntityKey, UPDATE_TPS,
        entity_tracker::{
            EntityTracker,
            entity::{Entity, GameAction},
        },
        light_grid::LightGrid,
    },
};

pub const LOGIC_GATE_TEXTURE_START: Point2<f32> = point![32.0, 48.0];
pub const LOGIC_GATE_TEXTURE_SIZE: Vector2<f32> = vector![16.0, 16.0];

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LogicGate {
    pub position: Point2<f64>,
    pub kind: LogicGateKind,
    pub inputs: Vec<EntityKey>,
    pub direction: LogicGateDirection,
    #[serde(skip)]
    pub powered: bool,
    #[serde(skip)]
    pub was_powered: bool,
    #[serde(skip, default = "default_time_powered")]
    pub time_powered: u16,
}

pub fn default_time_powered() -> u16 {
    u16::MAX
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Debug)]
pub enum LogicGateDirection {
    #[default]
    East,
    North,
    West,
    South,
}

impl LogicGateDirection {
    pub fn angle(self) -> f64 {
        match self {
            LogicGateDirection::East => 0.0,
            LogicGateDirection::North => PI * 1.5,
            LogicGateDirection::West => PI,
            LogicGateDirection::South => PI * 0.5,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum LogicGateKind {
    And,
    Or,
    Not,
    Passthrough,
    Toggle { state: bool, active: bool },
    Hold { state: bool },
    Start,
    End,
}

impl LogicGateKind {
    pub fn is_single_input(self) -> bool {
        match self {
            LogicGateKind::And => false,
            LogicGateKind::Or => false,
            LogicGateKind::Not => true,
            LogicGateKind::Passthrough => true,
            LogicGateKind::Toggle { .. } => true,
            LogicGateKind::Hold { .. } => true,
            LogicGateKind::Start => true,
            LogicGateKind::End => true,
        }
    }
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
        if self.powered == self.was_powered {
            self.time_powered = self.time_powered.saturating_add(1);
        } else {
            self.was_powered = self.powered;
            self.time_powered = 0;
        }

        None
    }

    fn draw_effect_back(&mut self, texture_atlas: &Texture2D) {
        let texture_position = LOGIC_GATE_TEXTURE_START
            + LOGIC_GATE_TEXTURE_SIZE.component_mul(&match self.kind {
                LogicGateKind::And => vector![0.0, 0.0],
                LogicGateKind::Or => vector![1.0, 0.0],
                LogicGateKind::Not => vector![2.0, 0.0],
                LogicGateKind::Passthrough => vector![3.0, 0.0],
                LogicGateKind::Toggle { active, .. } => vector![4.0, active as u8 as f32],
                LogicGateKind::Hold { .. } => vector![5.0, 0.0],
                LogicGateKind::Start | LogicGateKind::End => return,
            });

        let position = self.position.map(|x| x as f32) - LOGIC_GATE_TEXTURE_SIZE / 2.0;

        texture::draw_texture_ex(
            texture_atlas,
            position.x,
            position.y,
            self.power_color().unwrap(),
            DrawTextureParams {
                source: Some(crate::new_texture_rect(
                    texture_position,
                    LOGIC_GATE_TEXTURE_SIZE,
                )),
                rotation: self.direction.angle() as f32,
                ..Default::default()
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

    fn should_recieve_inputs(&self) -> bool {
        false
    }

    fn inputs(&self) -> &[EntityKey] {
        &self.inputs
    }

    fn try_add_input(&mut self, key: EntityKey) {
        if self.kind.is_single_input() && !self.inputs.is_empty() {
            return;
        }

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
        self.powered = match &mut self.kind {
            LogicGateKind::And => inputs.iter().copied().reduce(|a, b| a && b),
            LogicGateKind::Or => inputs.iter().copied().reduce(|a, b| a || b),
            LogicGateKind::Not => inputs.first().copied().map(|x| !x),
            LogicGateKind::Passthrough | LogicGateKind::Start | LogicGateKind::End => {
                inputs.first().copied()
            }
            LogicGateKind::Toggle { state, active } => {
                if inputs.first().copied().unwrap_or_default() {
                    if *active {
                        *active = false;
                        *state ^= true;
                    }
                } else {
                    *active = true;
                }

                Some(*state)
            }
            LogicGateKind::Hold { state } => {
                if !*state {
                    *state = inputs.first().copied().unwrap_or_default();
                }
                Some(*state)
            }
        }
        .unwrap_or_default();

        self.powered
    }

    fn offset_of_wire(&self, wire_end: Vector2<f64>) -> Vector2<f64> {
        let distance = match self.kind {
            LogicGateKind::And => 9.0,
            LogicGateKind::Or => 9.0,
            LogicGateKind::Not => 5.0,
            LogicGateKind::Passthrough | LogicGateKind::Start | LogicGateKind::End => 0.0,
            LogicGateKind::Toggle { .. } => 6.0,
            LogicGateKind::Hold { .. } => {
                return vector![wire_end.x.clamp(-7.0, 7.0), wire_end.y.clamp(-9.0, 9.0)];
            }
        };

        wire_end.map(|x| x.clamp(-distance, distance))
    }

    fn power_color(&self) -> Option<Color> {
        if matches!(self.kind, LogicGateKind::End) {
            None
        } else {
            Some(power_color(self.powered, self.time_powered as usize))
        }
    }
}

pub fn power_color(powered: bool, time_powered: usize) -> Color {
    let time_powered = time_powered as f32 / UPDATE_TPS as f32;
    let transition = match time_powered {
        ..0.1 => 0.0,
        0.1..0.6 => (time_powered - 0.1) / 0.5,
        0.6.. => 1.0,
        _ => 0.0,
    };

    if powered {
        Color::new(
            transition,
            0.9 + 0.1 * 2.0 * (transition - 0.5).abs(),
            1.0 - 0.5 * transition,
            1.0,
        )
    } else {
        let brightness = 0.4 - 0.2 * transition;

        Color::new(brightness, brightness, brightness, 1.0)
    }
}
