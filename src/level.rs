use std::{fs, mem};

use macroquad::{
    camera::{self, Camera2D},
    color::{Color, colors},
    input::{KeyCode, MouseButton},
    material,
    prelude::{Material, MaterialParams, PipelineParams, ShaderSource},
    shapes,
    texture::{self, DrawTextureParams, FilterMode, Image, Texture2D},
    window,
};
use nalgebra::{Point2, Vector2, point, vector};
use slotmap::{SecondaryMap, SlotMap, new_key_type};

use crate::{
    collections::{
        history::{FrameIndex, History},
        slot_guard::SlotGuard,
        tile_grid::{TileGrid, TileIndex},
    },
    level::{
        entity_tracker::{
            EntityTracker,
            entity::{GameAction, ViewKind, player::PlayerState},
        },
        level_editor::LevelEditor,
        light_grid::{LightGrid, Pixel},
        tile::{TILE_KINDS, Tile, TileKind},
    },
};

pub(crate) mod entity_tracker;
pub(crate) mod level_editor;
pub(crate) mod light_grid;
pub(crate) mod tile;

pub const TILE_SIZE: isize = 8;

pub const UPDATE_TPS: usize = 60;
pub const UPDATE_DT: f64 = 1.0 / UPDATE_TPS as f64;
pub const MAX_UPDATES_PER_TICK: usize = 4;

/// TODO: Consider using the include_dir crate for embedding all of the levels into the binary
pub struct Level {
    pub path: String,
    pub level_data: Option<Vec<u8>>,

    pub hard_reset_state: SlotMap<EntityKey, EntityTracker>,

    pub soft_reset_state: SlotMap<EntityKey, EntityTracker>,
    pub mouse_position: Point2<f64>,

    pub frame: FrameIndex,
    pub fade_out_frame: Option<FrameIndex>,
    pub entities: SlotMap<EntityKey, EntityTracker>,
    pub input_readers: Vec<EntityKey>,

    pub texture_atlas: Texture2D,
    pub mask_texture: Camera2D,
    pub mask_material: Material,

    pub wall_texture: Camera2D,
    pub wall_mask_material: Material,

    pub tile_grid: TileGrid<Option<Tile>>,
    pub light_grid: LightGrid,

    pub shift_held: bool,
    pub control_held: bool,
    pub alt_held: bool,
    pub left_mouse_held: bool,
    pub right_mouse_held: bool,
    pub middle_mouse_held: bool,

    pub level_editor_active: bool,
    pub editor: LevelEditor,

    pub occlude_wall_shadows: bool,
}

new_key_type! {
    pub struct EntityKey;
}

impl Level {
    pub fn new(path: String) -> Level {
        let texture_atlas = Texture2D::from_image(
            &Image::from_file_with_format(crate::TEXTURE_ATLAS, None).unwrap(),
        );
        texture_atlas.set_filter(FilterMode::Nearest);

        if TILE_KINDS.lock().unwrap().is_empty() {
            tile::add_tile_kind(TileKind {
                name: "brick1".to_owned(),
                pixel_kind: Pixel::Solid,
                texture_location: point![0, 0],
            });
            tile::add_tile_kind(TileKind {
                name: "brick2".to_owned(),
                pixel_kind: Pixel::Solid,
                texture_location: point![1, 0],
            });
            tile::add_tile_kind(TileKind {
                name: "wood".to_owned(),
                pixel_kind: Pixel::None,
                texture_location: point![0, 1],
            });
            tile::add_tile_kind(TileKind {
                name: "hourglass".to_owned(),
                pixel_kind: Pixel::None,
                texture_location: point![1, 1],
            });
        }

        Level {
            path,
            level_data: None,

            hard_reset_state: SlotMap::default(),

            soft_reset_state: SlotMap::default(),
            mouse_position: point![0.0, 0.0],

            frame: 0,
            fade_out_frame: None,
            entities: SlotMap::default(),
            input_readers: Vec::new(),

            texture_atlas,
            mask_texture: Self::new_render_target(),
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

            wall_texture: Self::new_render_target(),
            wall_mask_material: material::load_material(
                ShaderSource::Glsl {
                    vertex: DEFAULT_VERTEX_SHADER,
                    fragment: MASK_FRAGMENT_SHADER,
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

            shift_held: false,
            control_held: false,
            alt_held: false,
            left_mouse_held: false,
            right_mouse_held: false,
            middle_mouse_held: false,

            level_editor_active: false,
            editor: LevelEditor::default(),

            occlude_wall_shadows: true,
        }
    }

    pub fn save(&mut self) -> Vec<u8> {
        let config = bincode::config::standard();

        self.tile_grid.shrink_to_fit();
        let mut level = bincode::serde::encode_to_vec(&self.tile_grid, config).unwrap();

        level.append(&mut bincode::serde::encode_to_vec(&self.hard_reset_state, config).unwrap());

        level
    }

    pub fn load_from_level_data(&mut self) {
        let data = if let Some(level_data) = &self.level_data {
            level_data
        } else {
            let data = fs::read(&self.path).unwrap();
            self.level_data = Some(data);

            self.level_data.as_ref().unwrap()
        };

        let (tile_grid, read) =
            bincode::serde::decode_from_slice(data, bincode::config::standard()).unwrap();

        let data = &data[read..];

        let (initial_state, _) =
            bincode::serde::decode_from_slice(data, bincode::config::standard()).unwrap();

        self.hard_reset_state = initial_state;
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

    pub fn entities_from_initial_state(
        initial_state: &SlotMap<EntityKey, EntityTracker>,
    ) -> SlotMap<EntityKey, EntityTracker> {
        let mut entities = initial_state.clone();

        for key in entities.keys().collect::<Vec<_>>() {
            let mut entity = mem::take(&mut entities[key]);

            entity.inner.spawn(key, &mut entities);

            entities[key] = entity;
        }

        entities
    }

    pub fn reset(&mut self) {
        self.load_from_level_data();

        self.soft_reset_state = Self::entities_from_initial_state(&self.hard_reset_state);

        self.load_initial_entities();
    }

    pub fn load_initial_entities(&mut self) {
        self.entities.clone_from(&self.soft_reset_state);
        self.input_readers.clear();

        for (key, entity) in &self.entities {
            if entity.inner.should_recieve_inputs() {
                self.input_readers.push(key);
            }
        }

        self.frame = 0;
        self.fade_out_frame = None;
    }

    pub fn step_at_level_start(&mut self) {
        self.mouse_moved(self.mouse_position, vector![0.0, 0.0]);
        self.update();
    }

    pub fn update(&mut self) {
        if self.level_editor_active {
            self.update_level_editor();
        } else {
            self.update_game();
        }
    }

    pub fn update_game(&mut self) {
        let mut actions = Vec::new();

        for key in self.entities.keys().collect::<Vec<_>>() {
            let (entity, guard) = SlotGuard::new(&mut self.entities, key);

            let action = entity.update(
                self.frame,
                guard,
                &mut self.light_grid,
                &mut self.soft_reset_state,
            );

            actions.extend(action);
        }

        for (_, entity) in &mut self.entities {
            entity.inner.update_view_area(&mut self.light_grid);
        }

        let mut stack = self.entities.keys().collect::<Vec<_>>();
        let mut updates = SecondaryMap::default();
        let mut visited = SecondaryMap::default();

        while let Some(&key) = stack.last() {
            visited.insert(key, ());
            let entity = &self.entities[key];
            let input_sources = entity.inner.inputs();
            let mut inputs = Vec::new();
            for &key in input_sources {
                if let Some(&input) = updates.get(key) {
                    inputs.push(input);
                } else if visited.contains_key(key) {
                    // Better than failing or entering an infinite loop
                    inputs.push(false);
                } else {
                    stack.push(key);
                }
            }

            if inputs.len() < input_sources.len() {
                continue;
            }

            let key = stack.pop().unwrap();
            if updates.contains_key(key) {
                continue;
            }
            let (entity, guard) = SlotGuard::new(&mut self.entities, key);

            let result = entity.inner.evaluate(guard, &inputs);

            updates.insert(key, result);
        }

        self.entities.retain(|_, entity| !entity.inner.is_empty());

        self.frame = self
            .frame
            .checked_add(1)
            .expect("Game should not run for a galactically long time.");

        match actions.iter().max() {
            Some(GameAction::StartFadeOut) => {
                if self.fade_out_frame.is_none() {
                    self.fade_out_frame = Some(self.frame + 16);
                }
            }
            Some(GameAction::HardReset) => {
                self.reset();
                self.step_at_level_start();
            }
            Some(GameAction::HardResetKeepPlayer) => {
                let mut saved_player = None;

                for &key in &self.input_readers {
                    if let Some(player) = self.entities[key].inner.as_player()
                        && player.state == PlayerState::Active
                    {
                        let mut player = player.clone();

                        player.history = History::default();
                        player.environment_history.clear();

                        saved_player = Some(player);
                        break;
                    }
                }

                self.reset();

                if let Some(saved_player) = saved_player {
                    for &key in &self.input_readers {
                        if let Some(player) = self.entities[key].inner.as_player()
                            && player.state == PlayerState::Active
                        {
                            *player = saved_player;
                            break;
                        }
                    }
                }

                self.step_at_level_start();
            }
            Some(GameAction::SoftReset) => {
                self.load_initial_entities();
                self.step_at_level_start();
            }
            Some(GameAction::LoadLevel(path)) => {
                self.path.clone_from(path);
                self.level_data = None;

                self.reset();
                self.step_at_level_start();
            }
            None => (),
        }
    }

    pub fn draw(&mut self) {
        if self.level_editor_active {
            self.draw_level_editor();
        } else {
            self.draw_game();
        }
    }

    pub fn draw_game(&mut self) {
        Self::update_render_target(&mut self.mask_texture);
        if self.occlude_wall_shadows {
            Self::update_render_target(&mut self.wall_texture);
        }

        // Trace vision
        let view_areas = self
            .entities
            .iter()
            .map(|(_, entity)| Some((entity.inner.view_area()?, entity.inner.view_kind()?)))
            .flatten()
            .collect::<Vec<_>>();

        let past_visibility = if let Some(fade_out_frame) = self.fade_out_frame {
            fade_out_frame.saturating_sub(self.frame)
        } else {
            self.frame
        }
        .min(16) as f32
            / 16.0;

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
        for (_, entity) in &mut self.entities {
            entity.inner.draw_floor(&self.texture_atlas);
        }

        {
            if self.occlude_wall_shadows {
                camera::push_camera_state();
                camera::set_camera(&self.wall_texture);
                window::clear_background(colors::BLANK);
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
            for (_, entity) in &mut self.entities {
                entity.inner.draw_wall(&self.texture_atlas);
            }

            if self.occlude_wall_shadows {
                camera::set_default_camera();

                texture::draw_texture_ex(
                    &self.wall_texture.render_target.as_ref().unwrap().texture,
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
        }

        // Vision occluded entities
        for (_, entity) in &mut self.entities {
            entity.inner.draw_back(&self.texture_atlas);
        }

        // Vision mask
        camera::push_camera_state();
        camera::set_camera(&self.mask_texture);
        window::clear_background(colors::BLACK);

        material::gl_use_material(&self.mask_material);

        let mut indecies = (0..view_areas.len()).collect::<Vec<_>>();
        indecies.sort_unstable_by(|&a, &b| {
            view_areas[a]
                .1
                .confusion()
                .total_cmp(&view_areas[b].1.confusion())
        });

        for &i in &indecies {
            let &(ref area, kind) = &view_areas[i];

            match kind {
                ViewKind::Present => {
                    area.draw_wall_lighting(colors::BLANK);
                }
                ViewKind::Past { confusion } => {
                    area.draw_wall_lighting(Color::new(
                        past_visibility,
                        past_visibility * confusion as f32,
                        0.0,
                        1.0 - 0.8 * past_visibility,
                    ));
                }
            }
        }

        if self.occlude_wall_shadows {
            material::gl_use_material(&self.wall_mask_material);

            let screen_rect = crate::screen_rect();

            texture::draw_texture_ex(
                &self.wall_texture.render_target.as_ref().unwrap().texture,
                screen_rect.x,
                screen_rect.y,
                colors::WHITE,
                DrawTextureParams {
                    dest_size: Some(screen_rect.size()),
                    ..Default::default()
                },
            );

            material::gl_use_material(&self.mask_material);
        }

        for &i in &indecies {
            let &(ref area, kind) = &view_areas[i];

            match kind {
                ViewKind::Present => {
                    area.draw_direct_lighting(colors::BLANK);
                }
                ViewKind::Past { confusion } => {
                    area.draw_direct_lighting(Color::new(
                        past_visibility,
                        past_visibility * confusion as f32,
                        0.0,
                        1.0 - 0.8 * past_visibility,
                    ));
                }
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

        // Always visible entities
        for (_, entity) in &mut self.entities {
            entity.inner.draw_effect_back(&self.texture_atlas);
        }

        Self::draw_wires(&self.entities, false);

        for (_, entity) in &mut self.entities {
            entity.inner.draw_overlay_back(&self.texture_atlas);
        }

        for (_, entity) in &mut self.entities {
            entity.inner.draw_front(&self.texture_atlas);
        }

        for (_, entity) in &mut self.entities {
            entity.inner.draw_effect_front(&self.texture_atlas);
        }

        for (_, entity) in &mut self.entities {
            entity.inner.draw_overlay_front(&self.texture_atlas);
        }
    }

    pub fn draw_wires(entities: &SlotMap<EntityKey, EntityTracker>, show_hidden: bool) {
        for (_, entity) in entities {
            for &key in entity.inner.inputs() {
                let input = &entities[key];
                let color = input.inner.power_color();

                if !show_hidden && color.is_none() {
                    continue;
                }

                let color = color.unwrap_or(colors::MAROON);

                let offset = entity.inner.position() - input.inner.position();
                let start = input.inner.position() + input.inner.offset_of_wire(offset);

                let offset = start - entity.inner.position();
                let end = entity.inner.position() + entity.inner.offset_of_wire(offset);

                shapes::draw_line(
                    start.x as f32,
                    start.y as f32,
                    end.x as f32,
                    end.y as f32,
                    2.0,
                    color,
                );
            }
        }
    }

    pub fn new_render_target() -> Camera2D {
        let mut camera = Camera2D::from_display_rect(crate::screen_rect());
        camera.zoom.y *= -1.0;

        let size = crate::screen_pixel_size();
        camera.render_target = Some(texture::render_target(size.x, size.y));
        camera
            .render_target
            .as_ref()
            .unwrap()
            .texture
            .set_filter(FilterMode::Nearest);

        camera
    }

    pub fn update_render_target(camera: &mut Camera2D) {
        let mut new_zoom = Camera2D::from_display_rect(crate::screen_rect()).zoom;
        new_zoom.y *= -1.0;
        camera.zoom = new_zoom;

        let render_target = camera.render_target.as_mut().unwrap();
        let size = crate::screen_pixel_size();
        if size != Vector2::from(render_target.texture.size()).map(|x| x as u32) {
            *render_target = texture::render_target(size.x, size.y);
        }
    }

    pub fn text_input(&mut self, input: char) {
        if self.level_editor_active {
            self.level_editor_text_input(input);
        }
    }

    pub fn key_down(&mut self, input: KeyCode) {
        match input {
            KeyCode::LeftShift | KeyCode::RightShift => {
                self.shift_held = true;
            }
            KeyCode::LeftControl | KeyCode::RightControl => {
                self.control_held = true;
            }
            KeyCode::LeftAlt | KeyCode::RightAlt => {
                self.alt_held = true;
            }
            KeyCode::Key0 | KeyCode::Kp0 if self.shift_held => {
                self.level_editor_active ^= true;

                if !self.level_editor_active {
                    self.exit_level_editor();

                    self.level_data = Some(self.save());

                    self.reset();
                    self.step_at_level_start();
                }
            }
            KeyCode::Escape => {
                if !self.level_editor_active
                    || self.editor.cursor.is_none()
                        && self.editor.command_input.is_empty()
                        && self.alt_held
                {
                    window::miniquad::window::quit();
                }
            }
            _ => (),
        }

        if self.level_editor_active {
            self.level_editor_key_down(input);
        } else {
            self.input_readers.retain(|&key| {
                let Some(entity) = self.entities.get_mut(key) else {
                    return false;
                };

                entity.key_down(input);

                true
            });
        }
    }

    pub fn key_up(&mut self, input: KeyCode) {
        match input {
            KeyCode::LeftShift | KeyCode::RightShift => {
                self.shift_held = false;
            }
            KeyCode::LeftControl | KeyCode::RightControl => {
                self.control_held = false;
            }
            KeyCode::LeftAlt | KeyCode::RightAlt => {
                self.alt_held = false;
            }
            _ => (),
        }

        if self.level_editor_active {
            self.level_editor_key_up(input);
        } else {
            self.input_readers.retain(|&key| {
                let Some(entity) = self.entities.get_mut(key) else {
                    return false;
                };

                entity.key_up(input);

                true
            });
        }
    }

    pub fn mouse_down(&mut self, input: MouseButton, position: Point2<f64>) {
        match input {
            MouseButton::Left => {
                self.left_mouse_held = true;
            }
            MouseButton::Right => {
                self.right_mouse_held = true;
            }
            MouseButton::Middle => {
                self.middle_mouse_held = true;
            }
            _ => (),
        }

        if self.level_editor_active {
            self.level_editor_mouse_down(input, position);
        } else {
            self.input_readers.retain(|&key| {
                let Some(entity) = self.entities.get_mut(key) else {
                    return false;
                };

                entity.mouse_down(input, position);

                true
            });
        }
    }

    pub fn mouse_up(&mut self, input: MouseButton, position: Point2<f64>) {
        match input {
            MouseButton::Left => {
                self.left_mouse_held = false;
            }
            MouseButton::Right => {
                self.right_mouse_held = false;
            }
            MouseButton::Middle => {
                self.middle_mouse_held = false;
            }
            _ => (),
        }

        if self.level_editor_active {
            self.level_editor_mouse_up(input, position);
        } else {
            self.input_readers.retain(|&key| {
                let Some(entity) = self.entities.get_mut(key) else {
                    return false;
                };

                entity.mouse_up(input, position);

                true
            });
        }
    }

    pub fn mouse_moved(&mut self, position: Point2<f64>, delta: Vector2<f64>) {
        self.mouse_position = position;

        if self.level_editor_active {
            self.level_editor_mouse_moved(position, delta);
        } else {
            self.input_readers.retain(|&key| {
                let Some(entity) = self.entities.get_mut(key) else {
                    return false;
                };

                entity.mouse_moved(position, delta);

                true
            });
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
        gl_FragColor = color * texture2D(Texture, uv);
    }
"#;

pub const MASK_FRAGMENT_SHADER: &str = r#"
    #version 100
    varying lowp vec4 color;
    varying lowp vec2 uv;

    uniform sampler2D Texture;

    void main() {
        if (texture2D(Texture, uv) == vec4(0.0, 0.0, 0.0, 0.0)) {
            gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        } else {
            discard;
        }
    }
"#;
