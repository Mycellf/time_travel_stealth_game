use macroquad::{
    color::colors,
    input::{KeyCode, MouseButton},
    texture::{self, DrawTextureParams},
};
use nalgebra::{Point2, Vector2, point};

use crate::level::{Level, TILE_SIZE, tile};

#[derive(Clone, Default, Debug)]
pub struct LevelEditor {
    pub command: String,
    pub cursor: Option<usize>,
}

impl Level {
    pub fn update_level_editor(&mut self) {
        println!("{:?}, {:?}", self.editor.cursor, self.editor.command);
    }

    pub fn draw_level_editor(&mut self) {
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

    pub fn level_editor_text_input(&mut self, input: char) {
        if let Some(cursor) = &mut self.editor.cursor {
            match input {
                '\r' | '\n' => {
                    self.editor.cursor = None;
                }
                // Backspace
                '\u{8}' => {
                    if *cursor > 0 {
                        *cursor = self
                            .editor
                            .command
                            .floor_char_boundary(cursor.saturating_sub(1));
                        self.editor.command.remove(*cursor);
                    }
                }
                // Delete
                '\u{7f}' => {
                    if *cursor < self.editor.command.len() {
                        self.editor.command.remove(*cursor);
                    }
                }
                _ => {
                    if let Some(cursor) = &mut self.editor.cursor {
                        self.editor.command.insert(*cursor, input);
                        *cursor += input.len_utf8();
                    }
                }
            }
        } else {
            match input {
                '/' if self.editor.cursor.is_none() => {
                    self.editor.command.clear();
                    self.editor.cursor = Some(0);
                }
                _ => (),
            }
        }
    }

    pub fn level_editor_key_down(&mut self, input: KeyCode) {
        match input {
            KeyCode::Escape => {
                self.editor.command.clear();
                self.editor.cursor = None;
            }
            _ => (),
        }

        if let Some(cursor) = &mut self.editor.cursor {
            match input {
                KeyCode::Left => {
                    *cursor = self
                        .editor
                        .command
                        .floor_char_boundary(cursor.saturating_sub(1))
                }
                KeyCode::Right => {
                    *cursor = self
                        .editor
                        .command
                        .ceil_char_boundary(cursor.saturating_add(1))
                }
                KeyCode::Home => {
                    *cursor = 0;
                }
                KeyCode::End => {
                    *cursor = self.editor.command.len();
                }
                _ => (),
            }
        }
    }

    pub fn level_editor_key_up(&mut self, _input: KeyCode) {}

    pub fn level_editor_mouse_down(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    pub fn level_editor_mouse_up(&mut self, _input: MouseButton, _position: Point2<f64>) {}

    pub fn level_editor_mouse_moved(&mut self, _position: Point2<f64>, _delta: Vector2<f64>) {}
}
