use std::mem;

use earcut::Earcut;
use macroquad::{
    color::colors,
    input::{KeyCode, MouseButton},
    models,
};
use nalgebra::{Point2, Vector2};
use slotmap::{SlotMap, new_key_type};

use crate::level::{
    entity::{Entity, EntityTracker},
    light_grid::{LightGrid, Pixel},
};

pub(crate) mod entity;
pub(crate) mod light_grid;

pub struct Level {
    pub initial_state: Vec<Box<dyn Entity>>,

    pub entities: SlotMap<EntityKey, EntityTracker>,
    pub input_readers: Vec<EntityKey>,

    pub light_grid: LightGrid,
    pub brush: Option<Pixel>,
}

new_key_type! {
    pub struct EntityKey;
}

impl Level {
    pub fn new(initial_state: Vec<Box<dyn Entity>>) -> Level {
        let mut result = Level {
            initial_state,

            entities: SlotMap::default(),
            input_readers: Vec::new(),

            light_grid: LightGrid::default(),
            brush: None,
        };

        result.load_initial_state();

        result
    }

    pub fn load_initial_state(&mut self) {
        self.entities.clear();
        self.input_readers.clear();

        let initial_state = mem::take(&mut self.initial_state);

        for entity in initial_state.iter() {
            self.insert_entity(entity.duplicate());
        }

        self.initial_state = initial_state;
    }

    pub fn insert_entity(&mut self, entity: Box<dyn Entity>) {
        let needs_input = entity.should_recieve_inputs();
        let key = self.entities.insert(EntityTracker::new(entity.duplicate()));

        if needs_input {
            self.input_readers.push(key);
        }
    }

    pub fn update(&mut self) {
        for (_, entity) in &mut self.entities {
            entity.update();
        }
    }

    pub fn draw(&mut self) {
        self.light_grid.draw(colors::BLANK, colors::GRAY);

        for (_, entity) in &self.entities {
            if let Some(view_range) = entity.inner.view_range() {
                let position = entity.inner.position();

                let area = self.light_grid.trace_light_from(position, Some(view_range));
                if let Some(mesh) = area.mesh(&mut Earcut::new()) {
                    models::draw_mesh(&mesh);
                }
            }
        }

        for (_, entity) in &mut self.entities {
            entity.draw();
        }
    }

    pub fn key_down(&mut self, input: KeyCode) {
        for &key in &self.input_readers {
            self.entities[key].key_down(input);
        }
    }

    pub fn key_up(&mut self, input: KeyCode) {
        for &key in &self.input_readers {
            self.entities[key].key_up(input);
        }
    }

    pub fn mouse_down(&mut self, input: MouseButton, position: Point2<f64>) {
        for &key in &self.input_readers {
            self.entities[key].mouse_down(input, position);
        }

        match input {
            MouseButton::Left => {
                let pixel = &mut self.light_grid[position.map(|x| x.floor() as isize)];

                let brush = if pixel.blocks_light() {
                    Pixel::None
                } else {
                    Pixel::Solid
                };

                self.brush = Some(brush);

                *pixel = brush;
            }
            _ => (),
        }
    }

    pub fn mouse_up(&mut self, input: MouseButton, position: Point2<f64>) {
        for &key in &self.input_readers {
            self.entities[key].mouse_up(input, position);
        }

        match input {
            MouseButton::Left => {
                self.brush = None;
            }
            _ => (),
        }
    }

    pub fn mouse_moved(&mut self, position: Point2<f64>, delta: Vector2<f64>) {
        for &key in &self.input_readers {
            self.entities[key].mouse_moved(position, delta);
        }

        if let Some(brush) = self.brush {
            let index = position.map(|x| x.floor() as isize);

            if self.light_grid[index] != brush {
                self.light_grid[index] = brush;
            }
        }
    }
}
