use std::slice;

use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct WireDiagram {
    pub wires: SlotMap<WireKey, Wire>,
    pub gates: Vec<WireGateTracker>,
}

new_key_type! {
    pub struct WireKey;
}

pub type WireData = u16;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Wire {
    pub display_width: u8,
    pub data: WireData,
}

impl Default for Wire {
    fn default() -> Self {
        Self {
            display_width: 1,
            data: 0,
        }
    }
}

impl Wire {
    /// # Panics
    ///
    /// Will panic if `channel` >= `WireData::BITS`
    pub fn set_channel(&mut self, channel: u8, value: bool) {
        assert!(channel < WireData::BITS as u8);
        let mask = 1 << channel as WireData;
        if value {
            self.data |= mask;
        } else {
            self.data &= !mask;
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct WireGateTracker {
    pub inner: WireGate,
    pub position: Point2<f64>,
    pub state: Wire,
}

impl WireGateTracker {
    pub fn evaluate(&mut self, wires: &mut SlotMap<WireKey, Wire>) {
        self.state = self.inner.evaluate(wires);
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum WireGate {
    And {
        inputs: Vec<WireKey>,
        output: WireKey,
    },
    Or {
        inputs: Vec<WireKey>,
        output: WireKey,
    },
    Not {
        input: WireKey,
        output: WireKey,
    },
    Split {
        input: WireKey,
        outputs: Vec<WireKey>,
    },
}

impl WireGate {
    pub fn evaluate(&self, wires: &mut SlotMap<WireKey, Wire>) -> Wire {
        match self {
            WireGate::And { inputs, output } => {
                let result = Self::reduce_inputs(wires, inputs, |a, b| a & b);
                wires[*output] = result.clone();
                result
            }
            WireGate::Or { inputs, output } => {
                let result = Self::reduce_inputs(wires, inputs, |a, b| a | b);
                wires[*output] = result.clone();
                result
            }
            WireGate::Not { input, output } => {
                let input = &wires[*input];
                let result = Wire {
                    data: !input.data,
                    ..*input
                };
                wires[*output] = result.clone();
                result
            }
            WireGate::Split { input, outputs } => {
                let input = wires[*input].clone();
                for output in outputs {
                    wires[*output] = input.clone();
                }
                input
            }
        }
    }

    pub fn inputs(&self) -> &[WireKey] {
        match self {
            WireGate::And { inputs, .. } | WireGate::Or { inputs, .. } => inputs,
            WireGate::Not { input, .. } | WireGate::Split { input, .. } => slice::from_ref(input),
        }
    }

    pub fn outputs(&self) -> &[WireKey] {
        match self {
            WireGate::Split { outputs, .. } => outputs,
            WireGate::And { output, .. }
            | WireGate::Or { output, .. }
            | WireGate::Not { output, .. } => slice::from_ref(output),
        }
    }

    fn reduce_inputs(
        wires: &SlotMap<WireKey, Wire>,
        inputs: &[WireKey],
        function: impl FnMut(WireData, WireData) -> WireData,
    ) -> Wire {
        let inputs = inputs.iter().map(|&key| &wires[key]);
        Wire {
            display_width: inputs
                .clone()
                .map(|wire| wire.display_width)
                .max()
                .unwrap_or(1),
            data: inputs.map(|wire| wire.data).reduce(function).unwrap_or(0),
        }
    }
}
