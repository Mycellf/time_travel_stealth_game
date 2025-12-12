use std::fmt::Debug;

use macroquad::{
    color::Color,
    input::{KeyCode, MouseButton},
    texture::Texture2D,
};
use nalgebra::{Point2, Vector2, vector};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::{
    collections::{history::FrameIndex, slot_guard::GuardedSlotMap, tile_grid::TileRect},
    level::{
        EntityKey,
        entity_tracker::{
            EntityTracker,
            entity::{elevator::Elevator, elevator_door::ElevatorDoor, player::Player},
        },
        light_grid::{LightArea, LightGrid},
    },
};

pub(crate) mod button;
pub(crate) mod elevator;
pub(crate) mod elevator_door;
pub(crate) mod empty;
pub(crate) mod logic_gate;
pub(crate) mod player;

#[typetag::serde(tag = "type")]
pub trait Entity: 'static + Debug {
    /// Called `UPDATE_TPS` times per second.
    #[must_use]
    fn update(
        &mut self,
        frame: FrameIndex,
        entities: GuardedSlotMap<EntityKey, EntityTracker>,
        light_grid: &mut LightGrid,
        initial_state: &mut SlotMap<EntityKey, EntityTracker>,
    ) -> Option<GameAction>;

    /// Called for each entity after everything has had `update` called.
    fn update_view_area(&mut self, _light_grid: &mut LightGrid) {}

    /// Called just before an entity is teleported back to the start of the level, good for setting
    /// any player inputs to use a recording in stead.
    fn travel_to_beginning(&mut self, _past: &mut EntityTracker) {}

    /// Drawn behind every other layer, before wall tiles. Not used to occlude the wall mask.
    /// Good for drawing parts of an entity that should logically be part of the floor.
    fn draw_floor(&mut self, _texture_atlas: &Texture2D) {}

    /// Drawn behind every layer but draw_floor, and after all tiles. Used to occlude the wall mask
    /// if enabled. Good for drawing parts of an entity that will be inside of light blocking pixels.
    fn draw_wall(&mut self, _texture_atlas: &Texture2D) {}

    /// Occluded by light, but not used to occlude the wall mask. Good for drawing generic entities
    /// that shouldn't be visible outside the field of view.
    fn draw_back(&mut self, _texture_atlas: &Texture2D) {}

    /// Not occluded by light. Drawn just in front of `draw_back`.
    fn draw_effect_back(&mut self, _texture_atlas: &Texture2D) {}

    /// Not occluded by light. Drawn just in front of `draw_effect_back`.
    fn draw_overlay_back(&mut self, _texture_atlas: &Texture2D) {}

    /// Not occluded by light. Good for drawing entities that should always be on screen.
    fn draw_front(&mut self, _texture_atlas: &Texture2D) {}

    /// Not occluded by light. Drawn just in front of `draw_front`.
    fn draw_effect_front(&mut self, _texture_atlas: &Texture2D) {}

    /// Not occluded by light. Drawn just in front of `draw_effect_front`.
    fn draw_overlay_front(&mut self, _texture_atlas: &Texture2D) {}

    /// The set of tiles an entity would collide with, if applicable.
    fn collision_rect(&self) -> Option<TileRect> {
        None
    }

    /// The field of view of this entity, if applicable.
    fn view_area(&self) -> Option<LightArea> {
        None
    }

    /// The style this entity's view should be drawn as, if applicable.
    fn view_kind(&self) -> Option<ViewKind> {
        None
    }

    /// Check if this entity is within a certain field of view, if applicable.
    fn is_within_view_area(&self, _light_grid: &LightGrid, _area: &LightArea) -> bool {
        false
    }

    /// The state of this entity if seen according to `is_within_view_area`.
    fn visible_state(&self) -> Option<EntityVisibleState> {
        None
    }

    fn position(&self) -> Point2<f64>;

    fn position_mut(&mut self) -> Option<&mut Point2<f64>> {
        None
    }

    /// A hack to get cloning entities to work. Typically an implementation looks like this:
    ///
    /// ```
    /// fn duplicate(&self) -> Box<dyn Entity> {
    ///     Box::new(self.clone())
    /// }
    /// ```
    fn duplicate(&self) -> Box<dyn Entity>;

    /// Called when this entity is first loaded from the initial state. Best used to add any needed
    /// child entities to the list, e.g. the elevator's door.
    ///
    /// For the duration of this call, `entities[key]` is the `Empty` entity.
    fn spawn(&mut self, _key: EntityKey, _entities: &mut SlotMap<EntityKey, EntityTracker>) {}

    /// If this returns true, the elevator that this entity is destined for will break, if any.
    fn is_dead(&self) -> bool {
        false
    }

    /// Called once when this entity is copied into the level. If it returns true, player inputs
    /// will be passed to the `key_down`, `key_up`, `mouse_down`, `mouse_up`, and `mouse_moved`
    /// functions.
    fn should_recieve_inputs(&self) -> bool;

    /// `should_recieve_inputs` must return true for inputs to be passed through to this.
    fn key_down(&mut self, _input: KeyCode) {}

    /// `should_recieve_inputs` must return true for inputs to be passed through to this.
    fn key_up(&mut self, _input: KeyCode) {}

    /// `should_recieve_inputs` must return true for inputs to be passed through to this.
    fn mouse_down(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    /// `should_recieve_inputs` must return true for inputs to be passed through to this.
    fn mouse_up(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    /// `should_recieve_inputs` must return true for inputs to be passed through to this.
    fn mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {}

    fn inputs(&self) -> &[EntityKey] {
        &[]
    }

    fn try_add_input(&mut self, _key: EntityKey) {}

    fn try_remove_input(&mut self, _key: EntityKey) {}

    fn evaluate(
        &mut self,
        _entities: GuardedSlotMap<EntityKey, EntityTracker>,
        _inputs: &[bool],
    ) -> bool {
        false
    }

    fn offset_of_wire(&self, _wire_end: Vector2<f64>) -> Vector2<f64> {
        vector![0.0, 0.0]
    }

    fn power_color(&self) -> Option<Color> {
        None
    }

    /// If this entity is a `Player`, return Some(self).
    ///
    /// This should only be overridden by something which is or contains a `Player`.
    fn as_player(&mut self) -> Option<&mut Player> {
        None
    }

    /// If this entity is an `ElevatorDoor`, return Some(self).
    ///
    /// This should only be overridden by something which is or contains an `ElevatorDoor`.
    fn as_door(&mut self) -> Option<&mut ElevatorDoor> {
        None
    }

    /// If this entity is an `Elevator`, return Some(self).
    ///
    /// This should only be overridden by something which is or contains an `Elevator`.
    fn as_elevator(&mut self) -> Option<&mut Elevator> {
        None
    }

    /// If this entity is an `Elevator`, return true.
    ///
    /// This should only be overridden by something which is or contains an `Empty`.
    fn is_empty(&self) -> bool {
        false
    }
}

impl Clone for Box<dyn Entity> {
    fn clone(&self) -> Self {
        self.duplicate()
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub enum GameAction {
    StartFadeOut,
    SoftReset,
    HardResetKeepPlayer,
    HardReset,
    LoadLevel(String),
}

#[derive(Clone, Copy, Debug)]
pub enum ViewKind {
    Present,
    Past { confusion: f64 },
}

impl ViewKind {
    pub fn confusion(self) -> f64 {
        match self {
            ViewKind::Present => -f64::INFINITY,
            ViewKind::Past { confusion } => confusion,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct EntityVisibleState {
    pub position: Point2<f32>,
    pub extra: u64,
}

impl EntityVisibleState {
    pub fn new(position: Point2<f64>, extra: u64) -> Self {
        Self {
            position: position.map(|x| x as f32),
            extra,
        }
    }

    pub fn position(&self) -> Point2<f64> {
        self.position.map(|x| x as f64)
    }
}
