use std::{fs, str::FromStr};

use macroquad::{
    color::colors,
    input::{KeyCode, MouseButton},
    math::Rect,
    shapes, text,
    texture::{self, DrawTextureParams},
};
use nalgebra::{Point2, Vector2, point, vector};

use crate::{
    collections::tile_grid::TileGrid,
    level::{
        Level, TILE_SIZE,
        entity_tracker::entity::{
            Entity, GameAction,
            elevator::{Elevator, ElevatorDirection},
            player::Player,
        },
        tile::{self, TILE_KINDS, Tile},
    },
};

#[derive(Clone, Default, Debug)]
pub struct LevelEditor {
    pub command_input: String,
    pub cursor: Option<usize>,

    pub command_input_history: Vec<String>,
    pub command_input_history_index: usize,

    pub command: Option<Command>,
    pub selected_entity: Option<usize>,
    pub grabbing: Option<Vector2<f64>>,
}

#[derive(Debug)]
pub enum Command {
    Delete,
    Tile(Option<Tile>),
    Entity(Box<dyn Entity>),
    Save(Option<String>),
    Load(Option<String>),
    Clear,
}

impl Clone for Command {
    fn clone(&self) -> Self {
        match self {
            Self::Delete => Self::Delete,
            Self::Tile(kind) => Self::Tile(kind.clone()),
            Self::Entity(entity) => Self::Entity(entity.duplicate()),
            Self::Save(path) => Self::Save(path.clone()),
            Self::Load(path) => Self::Load(path.clone()),
            Self::Clear => Self::Clear,
        }
    }
}

impl Command {
    pub fn use_entity_selection(&self) -> bool {
        match self {
            Command::Delete => true,
            Command::Tile(_) => false,
            Command::Entity(_) => true,
            Command::Save(_) => false,
            Command::Load(_) => false,
            Command::Clear => false,
        }
    }

    pub fn is_single_use(&self) -> bool {
        match self {
            Command::Delete => false,
            Command::Tile(_) => false,
            Command::Entity(_) => true,
            Command::Save(_) => true,
            Command::Load(_) => true,
            Command::Clear => true,
        }
    }
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words = s.split_whitespace().collect::<Vec<_>>();

        match words.get(0) {
            Some(&"delete") => Ok(Command::Delete),
            Some(&"tile") => {
                if words.get(1) == Some(&"empty") {
                    Ok(Command::Tile(None))
                } else {
                    for (key, tile) in &*TILE_KINDS.lock().unwrap() {
                        if words.get(1) == Some(&tile.name.as_str()) {
                            return Ok(Command::Tile(Some(Tile { kind: key })));
                        }
                    }

                    Err(())
                }
            }
            Some(&"entity") => {
                let entity: Box<dyn Entity> = match words.get(1) {
                    Some(&"player") => Box::new(Player::default()),
                    Some(&"elevator") => Box::new(Elevator::new(
                        point![0.0, 0.0],
                        match words.get(2) {
                            Some(&"east") => ElevatorDirection::East,
                            Some(&"north") => ElevatorDirection::North,
                            Some(&"west") => ElevatorDirection::West,
                            Some(&"south") => ElevatorDirection::South,
                            _ => return Err(()),
                        },
                        match words.get(3) {
                            None | Some(&"loop") => GameAction::SoftReset,
                            Some(&"entry") => GameAction::HardResetSavePlayerPosition,
                            _ => return Err(()),
                        },
                    )),
                    _ => return Err(()),
                };

                Ok(Command::Entity(entity))
            }
            Some(&"save") => Ok(Command::Save(words.get(1).map(|&path| path.to_owned()))),
            Some(&"load") => Ok(Command::Load(words.get(1).map(|&path| path.to_owned()))),
            Some(&"clear") => Ok(Command::Clear),
            _ => Err(()),
        }
    }
}

impl Level {
    pub fn update_level_editor(&mut self) {
        if let Some((offset, selection)) = self.editor.grabbing.zip(self.editor.selected_entity) {
            if let Some(position) = self.initial_state[selection].position_mut() {
                *position = self.mouse_position + offset;

                if self.shift_held {
                    position.apply(|x| *x = (*x / 4.0).round() * 4.0);
                }
            }
        } else {
            self.editor.selected_entity = None;
            let mut closest_distance = f64::INFINITY;

            if self
                .editor
                .command
                .as_ref()
                .is_none_or(|command| command.use_entity_selection())
            {
                for (i, entity) in self.initial_state.iter().enumerate() {
                    if let Some(collision_rect) = entity.collision_rect() {
                        if Rect::new(
                            collision_rect.origin.x as f32,
                            collision_rect.origin.y as f32,
                            collision_rect.size.x as f32,
                            collision_rect.size.y as f32,
                        )
                        .contains(self.mouse_position.map(|x| x as f32).into())
                        {
                            self.editor.selected_entity = Some(i);
                            break;
                        }
                    } else {
                        let position = entity.position();
                        let distance = (self.mouse_position - position).magnitude();
                        if distance < 24.0 && distance < closest_distance {
                            closest_distance = distance;
                            self.editor.selected_entity = Some(i);
                        }
                    }
                }
            }
        }
    }

    pub fn draw_level_editor(&mut self) {
        self.level_editor_draw_level_contents();

        for (i, entity) in self.initial_state.iter().enumerate() {
            let color = if self.editor.selected_entity == Some(i) {
                colors::GREEN
            } else {
                colors::MAGENTA
            };

            if let Some(collision_rect) = entity.collision_rect() {
                shapes::draw_rectangle_lines(
                    collision_rect.origin.x as f32,
                    collision_rect.origin.y as f32,
                    collision_rect.size.x as f32,
                    collision_rect.size.y as f32,
                    1.0,
                    color,
                );
            } else {
                let position = entity.position();

                shapes::draw_rectangle(
                    position.x as f32 - 1.0,
                    position.y as f32 - 1.0,
                    2.0,
                    2.0,
                    color,
                );
            }
        }

        let screen_rect = crate::screen_rect();

        if !self.editor.command_input.is_empty() || self.editor.cursor.is_some() {
            let mut start = point![screen_rect.x, screen_rect.y + screen_rect.h - 4.0];

            if start.y - 32.0 < self.mouse_position.y as f32 {
                start.y = screen_rect.y + 12.0;
            }

            let text = format!("/{}", self.editor.command_input);

            let width = text::measure_text(&text, None, 16, 1.0).width;

            shapes::draw_rectangle(start.x, start.y - 10.0, width, 12.0, colors::BLACK);

            text::draw_text(&text, start.x, start.y, 16.0, colors::WHITE);

            if let Some(cursor) = self.editor.cursor {
                start.x += text::measure_text(&text[..1 + cursor], None, 16, 1.0).width;

                shapes::draw_rectangle(start.x, start.y - 10.0, 1.0, 12.0, colors::WHITE);
            }
        }
    }

    pub fn set_tile_at_mouse_position(&mut self, tile: Option<Tile>) {
        let index = (self.mouse_position / TILE_SIZE as f64).map(|x| x.floor() as isize);
        self.set_tile(index, tile);
    }

    pub fn level_editor_text_input(&mut self, input: char) {
        if let Some(cursor) = &mut self.editor.cursor {
            match input {
                '\r' | '\n' => {
                    self.editor.cursor = None;

                    if self.editor.command_input_history.last() != Some(&self.editor.command_input)
                    {
                        self.editor
                            .command_input_history
                            .push(self.editor.command_input.clone());
                    }
                    self.editor.command_input_history_index =
                        self.editor.command_input_history.len();

                    self.editor.command = self.editor.command_input.parse().ok();

                    if let Some(command) = &self.editor.command {
                        if command.is_single_use() {
                            self.editor.command_input.clear();

                            match self.editor.command.take().unwrap() {
                                Command::Entity(entity) => {
                                    self.editor.selected_entity = Some(self.initial_state.len());
                                    self.editor.grabbing = Some(vector![0.0, 0.0]);
                                    self.initial_state.push(entity);
                                }
                                Command::Save(path) => {
                                    if let Some(path) = path {
                                        self.path = path;
                                    }

                                    if self.path.is_empty() {
                                        self.editor
                                            .command_input
                                            .push_str("please specify a directory");
                                    } else {
                                        let level_data = self.save();
                                        if fs::write(&self.path, &level_data).is_ok() {
                                            self.level_data = Some(level_data);
                                        } else {
                                            self.editor.command_input.push_str("invalid directory");
                                        }
                                    }
                                }
                                Command::Load(path) => {
                                    if let Some(path) = path {
                                        self.path = path;
                                    }

                                    if self.path.is_empty() {
                                        self.editor
                                            .command_input
                                            .push_str("please specify a directory");
                                    } else {
                                        self.level_data = None;

                                        self.reset();
                                    }
                                }
                                Command::Clear => {
                                    self.path = "".to_owned();
                                    self.level_data = None;
                                    self.tile_grid = TileGrid::default();
                                    self.initial_state = Vec::new();
                                }
                                _ => (),
                            }
                        } else {
                            match command {
                                _ => (),
                            }
                        }
                    } else {
                        if !self.editor.command_input.is_empty() {
                            self.editor.command_input.clear();
                            self.editor.command_input.push_str("invalid command");
                        }
                    }
                }
                // Backspace
                '\u{8}' => {
                    if *cursor > 0 {
                        *cursor = self
                            .editor
                            .command_input
                            .floor_char_boundary(cursor.saturating_sub(1));
                        self.editor.command_input.remove(*cursor);
                    }
                }
                // Delete
                '\u{7f}' => {
                    if *cursor < self.editor.command_input.len() {
                        self.editor.command_input.remove(*cursor);
                    }
                }
                _ => {
                    if let Some(cursor) = &mut self.editor.cursor
                        && self.editor.command_input.len() < 1024
                    {
                        self.editor.command_input.insert(*cursor, input);
                        *cursor += input.len_utf8();
                    }
                }
            }
        } else {
            match input {
                '/' if self.editor.cursor.is_none() => {
                    self.editor.command_input.clear();
                    self.editor.cursor = Some(0);
                }
                _ => (),
            }
        }
    }

    pub fn level_editor_key_down(&mut self, input: KeyCode) {
        match input {
            KeyCode::Escape => {
                self.editor.command_input.clear();
                self.editor.cursor = None;

                self.editor.command = None;
            }
            _ => (),
        }

        if let Some(cursor) = &mut self.editor.cursor {
            match input {
                KeyCode::Left => loop {
                    *cursor = self
                        .editor
                        .command_input
                        .floor_char_boundary(cursor.saturating_sub(1));

                    if !self.control_held
                        || *cursor == 0
                        || self.editor.command_input.as_bytes()[*cursor].is_ascii_whitespace()
                    {
                        break;
                    }
                },
                KeyCode::Right => loop {
                    *cursor = self
                        .editor
                        .command_input
                        .ceil_char_boundary(cursor.saturating_add(1));

                    if !self.control_held
                        || *cursor >= self.editor.command_input.len()
                        || self.editor.command_input.as_bytes()[*cursor].is_ascii_whitespace()
                    {
                        break;
                    }
                },
                KeyCode::Home => {
                    *cursor = 0;
                }
                KeyCode::End => {
                    *cursor = self.editor.command_input.len();
                }
                KeyCode::Up => {
                    self.editor.command_input_history_index =
                        self.editor.command_input_history_index.saturating_sub(1);
                    self.editor.command_input = self
                        .editor
                        .command_input_history
                        .get(self.editor.command_input_history_index)
                        .cloned()
                        .unwrap_or_default();
                    self.editor.cursor = Some(self.editor.command_input.len());
                }
                KeyCode::Down => {
                    self.editor.command_input_history_index = self
                        .editor
                        .command_input_history_index
                        .saturating_add(1)
                        .min(self.editor.command_input_history.len());
                    self.editor.command_input = self
                        .editor
                        .command_input_history
                        .get(self.editor.command_input_history_index)
                        .cloned()
                        .unwrap_or_default();
                    self.editor.cursor = Some(self.editor.command_input.len());
                }
                _ => (),
            }
        }
    }

    pub fn level_editor_key_up(&mut self, _input: KeyCode) {}

    pub fn level_editor_mouse_down(&mut self, input: MouseButton, _position: Point2<f64>) {
        match input {
            MouseButton::Left => {
                if let Some(selection) = self.editor.selected_entity
                    && self.editor.grabbing.is_none()
                    && self
                        .editor
                        .command
                        .as_ref()
                        .is_none_or(|command| command.use_entity_selection())
                {
                    self.editor.grabbing =
                        Some(self.initial_state[selection].position() - self.mouse_position);
                }
            }
            _ => (),
        }

        match self.editor.command {
            Some(Command::Delete) => match input {
                MouseButton::Right => {
                    if let Some(selection) = self.editor.selected_entity {
                        self.initial_state.remove(selection);
                        self.editor.selected_entity = None;
                    }
                }
                _ => (),
            },
            Some(Command::Tile(tile)) => match input {
                MouseButton::Left => {
                    self.set_tile_at_mouse_position(tile);
                }
                MouseButton::Right => {
                    self.set_tile_at_mouse_position(None);
                }
                _ => (),
            },
            _ => (),
        }
    }

    pub fn level_editor_mouse_up(&mut self, input: MouseButton, _position: Point2<f64>) {
        match input {
            MouseButton::Left => {
                self.editor.grabbing = None;
            }
            _ => (),
        }
    }

    pub fn level_editor_mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {
        match self.editor.command {
            Some(Command::Tile(tile)) => {
                if self.left_mouse_held {
                    self.set_tile_at_mouse_position(tile);
                }

                if self.right_mouse_held {
                    self.set_tile_at_mouse_position(None);
                }
            }
            _ => (),
        }
    }

    pub fn level_editor_draw_level_contents(&mut self) {
        // Non-wall Tiles
        {
            let tile_kinds = tile::TILE_KINDS.lock().unwrap();

            let bounds = self.tile_grid.bounds();
            for x in bounds.left()..bounds.right() + 1 {
                for y in bounds.top()..bounds.bottom() + 1 {
                    let Some(tile) = self.tile_grid[point![x, y]] else {
                        continue;
                    };

                    let kind = &tile_kinds[tile.kind];

                    if kind.pixel_kind.blocks_light() {
                        continue;
                    }

                    texture::draw_texture_ex(
                        &self.texture_atlas,
                        x as f32 * TILE_SIZE as f32,
                        y as f32 * TILE_SIZE as f32,
                        colors::WHITE,
                        DrawTextureParams {
                            source: Some(kind.texture_rect()),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // Floor like entities
        for entity in &mut self.initial_state {
            entity.draw_floor(&self.texture_atlas);
        }

        // Wall Tiles
        {
            let tile_kinds = tile::TILE_KINDS.lock().unwrap();

            let bounds = self.tile_grid.bounds();
            for x in bounds.left()..bounds.right() + 1 {
                for y in bounds.top()..bounds.bottom() + 1 {
                    let Some(tile) = self.tile_grid[point![x, y]] else {
                        continue;
                    };

                    let kind = &tile_kinds[tile.kind];

                    if !kind.pixel_kind.blocks_light() {
                        continue;
                    }

                    texture::draw_texture_ex(
                        &self.texture_atlas,
                        x as f32 * TILE_SIZE as f32,
                        y as f32 * TILE_SIZE as f32,
                        colors::WHITE,
                        DrawTextureParams {
                            source: Some(kind.texture_rect()),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // Wall like entities
        for entity in &mut self.initial_state {
            entity.draw_wall(&self.texture_atlas);
        }

        // Vision occluded entities
        for entity in &mut self.initial_state {
            entity.draw_back(&self.texture_atlas);
        }

        // Always visible entities
        for entity in &mut self.initial_state {
            entity.draw_effect_back(&self.texture_atlas);
        }

        for entity in &mut self.initial_state {
            entity.draw_front(&self.texture_atlas);
        }

        for entity in &mut self.initial_state {
            entity.draw_effect_front(&self.texture_atlas);
        }
    }
}
