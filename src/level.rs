use std::{fs, mem};

use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{KeyCode, MouseButton},
    material,
    prelude::{Material, MaterialParams, PipelineParams, ShaderSource},
    texture::{self, DrawTextureParams, FilterMode, Image, Texture2D},
    window,
};
use nalgebra::{Point2, Vector2, point};
use slotmap::{SlotMap, new_key_type};

use crate::{
    collections::{
        slot_guard::SlotGuard,
        tile_grid::{TileGrid, TileIndex},
    },
    level::{
        entity_tracker::{
            EntityTracker,
            entity::{Entity, ViewKind},
        },
        light_grid::{LightGrid, Pixel},
        tile::{TILE_KINDS, Tile, TileKind, TileKindKey},
    },
};

pub(crate) mod entity_tracker;
pub(crate) mod light_grid;
pub(crate) mod tile;

pub const TILE_SIZE: isize = 8;

pub struct Level {
    pub initial_state: Vec<Box<dyn Entity>>,

    pub entities: SlotMap<EntityKey, EntityTracker>,
    pub input_readers: Vec<EntityKey>,

    pub texture_atlas: Texture2D,
    pub mask_texture: Camera2D,
    pub mask_material: Material,

    pub tile_grid: TileGrid<Option<Tile>>,
    pub light_grid: LightGrid,

    pub brushes: Vec<TileKindKey>,
    pub brush: usize,
    pub drawing: bool,
    pub precise_fill: bool,
    pub full_vision: bool,
}

new_key_type! {
    pub struct EntityKey;
}

impl Level {
    pub fn new(initial_state: Vec<Box<dyn Entity>>) -> Level {
        let texture_atlas = Texture2D::from_image(
            &Image::from_file_with_format(crate::TEXTURE_ATLAS, None).unwrap(),
        );
        texture_atlas.set_filter(FilterMode::Nearest);

        let mut mask_texture = Camera2D::from_display_rect(crate::screen_rect());
        mask_texture.zoom.y *= -1.0;

        let size = crate::screen_pixel_size();
        mask_texture.render_target = Some(texture::render_target(size.x, size.y));

        let mut result = Level {
            initial_state,

            entities: SlotMap::default(),
            input_readers: Vec::new(),

            texture_atlas,
            mask_texture,
            mask_material: material::load_material(
                ShaderSource::Glsl {
                    vertex: DEFAULT_VERTEX_SHADER,
                    fragment: DEFAULT_FRAGMENT_SHADER,
                },
                MaterialParams {
                    pipeline_params: PipelineParams {
                        color_write: (true, true, true, true),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap(),

            tile_grid: TileGrid::default(),
            light_grid: LightGrid::default(),

            brushes: if TILE_KINDS.lock().unwrap().is_empty() {
                drop(TILE_KINDS.lock().unwrap());
                vec![
                    tile::add_tile_kind(TileKind {
                        pixel_kind: Pixel::Solid,
                        texture_location: point![0, 0],
                    }),
                    tile::add_tile_kind(TileKind {
                        pixel_kind: Pixel::Solid,
                        texture_location: point![1, 0],
                    }),
                    tile::add_tile_kind(TileKind {
                        pixel_kind: Pixel::None,
                        texture_location: point![0, 1],
                    }),
                ]
            } else {
                TILE_KINDS.lock().unwrap().keys().collect()
            },
            brush: usize::MAX,
            drawing: false,
            precise_fill: false,
            full_vision: true,
        };

        result.load_initial_state();

        result
    }

    pub fn save(&self) -> Vec<u8> {
        bincode::serde::encode_to_vec(&self.tile_grid, bincode::config::standard()).unwrap()
    }

    pub fn load(&mut self, data: &[u8]) {
        let (tile_grid, _) =
            bincode::serde::decode_from_slice(data, bincode::config::standard()).unwrap();

        self.tile_grid = tile_grid;
        self.light_grid = LightGrid::default();

        let bounds = self.tile_grid.bounds();

        let tile_kinds = TILE_KINDS.lock().unwrap();

        for x in bounds.left()..bounds.right() + 1 {
            for y in bounds.top()..bounds.bottom() + 1 {
                if let Some(tile) = self.tile_grid[point![x, y]] {
                    self.light_grid
                        .fill_tile(point![x, y], tile_kinds[tile.kind].pixel_kind);
                }
            }
        }
    }

    pub fn set_tile(&mut self, index: TileIndex, tile: Option<Tile>) {
        if self.tile_grid[index] != tile {
            self.tile_grid[index] = tile;

            if let Some(tile) = tile {
                self.light_grid.fill_tile(index, tile.get_kind().pixel_kind);
            } else {
                self.light_grid.fill_tile(index, Pixel::default());
            }
        }
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
        for key in self.entities.keys().collect::<Vec<_>>() {
            // SAFETY: SlotMap doesn't allow accessing the same value with unequal keys.
            let (entity, guard) = unsafe { SlotGuard::new(&mut self.entities, key) };

            entity.update(guard, &mut self.light_grid);
        }
    }

    pub fn draw(&mut self) {
        self.update_mask_texture();

        // Trace vision
        let view_areas = self
            .entities
            .iter()
            .map(|(_, entity)| {
                let view_range = entity.inner.view_range()?;
                let view_kind = entity.inner.view_kind()?;

                let position = entity.inner.position();

                Some((
                    self.light_grid.trace_light_from(position, Some(view_range)),
                    view_kind,
                ))
            })
            .flatten()
            .collect::<Vec<_>>();

        // Tiles
        {
            let tile_kinds = tile::TILE_KINDS.lock().unwrap();

            let bounds = self.tile_grid.bounds();
            for x in bounds.left()..bounds.right() + 1 {
                for y in bounds.top()..bounds.bottom() + 1 {
                    let Some(tile) = self.tile_grid[point![x, y]] else {
                        continue;
                    };

                    texture::draw_texture_ex(
                        &self.texture_atlas,
                        x as f32 * TILE_SIZE as f32,
                        y as f32 * TILE_SIZE as f32,
                        colors::WHITE,
                        DrawTextureParams {
                            source: Some(tile_kinds[tile.kind].texture_rect()),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // Vision occluded entities
        for (_, entity) in &mut self.entities {
            if !entity.inner.always_visible() {
                entity.draw();
            }
        }

        // Vision mask
        if !self.full_vision {
            camera::push_camera_state();
            camera::set_camera(&self.mask_texture);
            window::clear_background(colors::BLACK);

            material::gl_use_material(&self.mask_material);

            for &(ref area, kind) in &view_areas {
                match kind {
                    ViewKind::Present => {
                        area.draw_all(colors::BLANK, colors::BLANK);
                    }
                    ViewKind::Past => (),
                }
            }

            for &(ref area, kind) in &view_areas {
                match kind {
                    ViewKind::Past => {
                        area.draw_all(
                            Color::new(1.0, 0.0, 0.0, 0.2),
                            Color::new(1.0, 0.0, 0.0, 0.2),
                        );
                    }
                    ViewKind::Present => (),
                }
            }

            material::gl_use_default_material();
            camera::set_default_camera();

            texture::draw_texture_ex(
                &self.mask_texture.render_target.as_ref().unwrap().texture,
                0.0,
                0.0,
                colors::WHITE,
                DrawTextureParams {
                    dest_size: Some([window::screen_width(), window::screen_height()].into()),
                    ..Default::default()
                },
            );
            camera::pop_camera_state();
        }

        // Always visible entities
        for (_, entity) in &mut self.entities {
            if entity.inner.always_visible() {
                entity.draw();
            }
        }
    }

    pub fn update_mask_texture(&mut self) {
        let mut new_zoom = Camera2D::from_display_rect(crate::screen_rect()).zoom;
        new_zoom.y *= -1.0;
        self.mask_texture.zoom = new_zoom;

        let render_target = self.mask_texture.render_target.as_mut().unwrap();
        let size = crate::screen_pixel_size();
        if size != Vector2::from(render_target.texture.size()).map(|x| x as u32) {
            *render_target = texture::render_target(size.x, size.y);
        }
    }

    pub fn key_down(&mut self, input: KeyCode) {
        match input {
            KeyCode::V => {
                self.full_vision ^= true;
            }
            KeyCode::LeftShift => {
                self.precise_fill = true;
            }
            KeyCode::Key0 => self.brush = usize::MAX,
            KeyCode::Key1 => self.brush = 0,
            KeyCode::Key2 => self.brush = 1,
            KeyCode::Key3 => self.brush = 2,
            KeyCode::Key4 => self.brush = 3,
            KeyCode::Key5 => self.brush = 4,
            KeyCode::Key6 => self.brush = 5,
            KeyCode::Key7 => self.brush = 6,
            KeyCode::Key8 => self.brush = 7,
            KeyCode::Key9 => self.brush = 8,
            KeyCode::Period => {
                if self.precise_fill {
                    fs::write("resources/level", self.save()).unwrap();
                }
            }
            _ => (),
        }

        for &key in &self.input_readers {
            self.entities[key].key_down(input);
        }
    }

    pub fn key_up(&mut self, input: KeyCode) {
        match input {
            KeyCode::LeftShift => {
                self.precise_fill = false;
            }
            _ => (),
        }

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
                let index = position.map(|x| (x.floor() as isize).div_euclid(TILE_SIZE));
                self.set_tile(
                    index,
                    self.brushes.get(self.brush).map(|&kind| Tile { kind }),
                );

                self.drawing = true;
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
                self.drawing = false;
            }
            _ => (),
        }
    }

    pub fn mouse_moved(&mut self, position: Point2<f64>, delta: Vector2<f64>) {
        for &key in &self.input_readers {
            self.entities[key].mouse_moved(position, delta);
        }

        if self.drawing {
            let index = position.map(|x| (x.floor() as isize).div_euclid(TILE_SIZE));
            self.set_tile(
                index,
                self.brushes.get(self.brush).map(|&kind| Tile { kind }),
            );
        }
    }
}

pub const DEFAULT_VERTEX_SHADER: &str = r#"
    #version 100
    attribute vec3 position;
    attribute vec2 texcoord;
    attribute vec4 color0;
    attribute vec4 normal;

    varying lowp vec2 uv;
    varying lowp vec4 color;

    uniform mat4 Model;
    uniform mat4 Projection;

    void main() {
        gl_Position = Projection * Model * vec4(position, 1);
        color = color0 / 255.0;
        uv = texcoord;
    }
"#;

pub const DEFAULT_FRAGMENT_SHADER: &str = r#"
    #version 100
    varying lowp vec4 color;
    varying lowp vec2 uv;

    uniform sampler2D Texture;

    void main() {
        gl_FragColor = color * texture2D(Texture, uv) ;
    }
"#;
