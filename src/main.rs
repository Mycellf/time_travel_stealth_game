use std::{env, path::PathBuf};

use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{Canvas, Color, DrawParam, Mesh, Rect, Sampler},
    input::keyboard::KeyInput,
    winit::{
        event::MouseButton,
        keyboard::{Key, NamedKey},
    },
};
use nalgebra::{Point2, UnitVector2, Vector2, point, vector};

use crate::world::light_grid::{LightGrid, MaterialKind};

pub(crate) mod collections;
pub(crate) mod world;

fn main() -> GameResult {
    let mut builder =
        ContextBuilder::new("time_travel_stealth_game", "CODER-J").default_conf(Conf {
            window_mode: WindowMode {
                // width: todo!(),
                // height: todo!(),
                // maximized: true,
                fullscreen_type: FullscreenType::Desktop,
                // borderless: todo!(),
                // transparent: todo!(),
                min_width: 300.0,
                min_height: 300.0,
                // max_width: todo!(),
                // max_height: todo!(),
                // resizable: todo!(),
                // visible: todo!(),
                // resize_on_scale_factor_change: todo!(),
                // logical_size: todo!(),
                ..Default::default()
            },
            window_setup: WindowSetup {
                title: "Time Travel Stealth Game".to_owned(),
                // samples: todo!(),
                // vsync: todo!(),
                // icon: todo!(),
                // srgb: todo!(),
                ..Default::default()
            },
            backend: Backend::default(),
        });

    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest_dir);
        path.push("resources");
        println!("Adding path {path:?}");
        builder = builder.add_resource_path(path);
    }

    let (mut ctx, event_loop) = builder.build()?;

    let state = State::new(&mut ctx)?;

    event::run(ctx, event_loop, state)
}

#[derive(Debug)]
pub(crate) struct State {
    fullscreen: bool,
    window_size: Vector2<f32>,

    raycast_start: Point2<f64>,
    raycast_direction: UnitVector2<f64>,
    raycast_finish: Point2<f64>,

    light_grid: LightGrid,
}

impl State {
    fn new(_ctx: &mut Context) -> GameResult<Self> {
        Ok(State {
            fullscreen: true,
            window_size: vector![800.0, 600.0],

            raycast_start: point![0.0, 0.0],
            raycast_direction: UnitVector2::new_normalize(vector![-1.0, 2.0]),
            raycast_finish: point![0.0, 0.0],

            light_grid: LightGrid::default(),
        })
    }
}

impl State {
    fn screen_rect(&self) -> Rect {
        rectangle_of_centered_camera(self.window_size, point![0.0, 0.0], 10.0)
    }

    fn screen_to_world(&self, point: Point2<f32>) -> Point2<f32> {
        let world = self.screen_rect();
        let screen = Rect::new(0.0, 0.0, self.window_size.x, self.window_size.y);

        transform_between_rectangles(screen, world, point)
    }
}

impl EventHandler for State {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        self.raycast_finish = self.light_grid.raycast_with(
            |_, pixel| pixel.is_some(),
            self.raycast_start,
            self.raycast_direction,
            100.0,
        );

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLUE));
        canvas.set_sampler(Sampler::nearest_clamp());

        canvas.set_screen_coordinates(self.screen_rect());

        self.light_grid.draw(ctx, &mut canvas);

        let line = Mesh::new_line(
            ctx,
            &[self.raycast_start, self.raycast_finish].map(|point| point.map(|x| x as f32)),
            0.1,
            Color::RED,
        )?;

        canvas.draw(&line, DrawParam::default());

        canvas.finish(ctx)?;

        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
        if repeated {
            return Ok(());
        }

        match input.event.logical_key {
            Key::Named(NamedKey::Escape) => {
                ctx.request_quit();
            }
            Key::Named(NamedKey::F11) => {
                self.fullscreen ^= true;

                ctx.gfx.set_fullscreen(if self.fullscreen {
                    FullscreenType::Desktop
                } else {
                    FullscreenType::Windowed
                })?;
            }
            _ => (),
        }

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        let mouse_position = self.screen_to_world(point![x, y]);

        match button {
            MouseButton::Left => {
                let index = mouse_position.map(|x| x.floor() as isize);
                let pixel = &mut self.light_grid.grid[index];

                match pixel {
                    Some(_) => *pixel = None,
                    None => *pixel = Some(MaterialKind::Solid),
                }
            }
            MouseButton::Right => {
                if let Some(raycast_direction) = UnitVector2::try_new(
                    mouse_position.map(|x| x as f64) - self.raycast_start,
                    f64::EPSILON,
                ) {
                    self.raycast_direction = raycast_direction;
                }
            }
            MouseButton::Middle => {
                self.raycast_start = mouse_position.map(|x| x as f64);
            }
            _ => (),
        }

        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, width: f32, height: f32) -> GameResult {
        self.window_size = vector![width, height];

        Ok(())
    }
}

fn rectangle_of_centered_camera(
    screen_size: Vector2<f32>,
    center: Point2<f32>,
    height: f32,
) -> Rect {
    let size = vector![height * screen_size.x / screen_size.y, height];
    let corner = center - size / 2.0;

    Rect::new(corner.x, corner.y, size.x, size.y)
}

fn transform_between_rectangles(
    source: Rect,
    destination: Rect,
    point: Point2<f32>,
) -> Point2<f32> {
    let source_origin = point![source.x, source.y];
    let source_size = vector![source.w, source.h];

    let destination_origin = point![destination.x, destination.y];
    let destination_size = vector![destination.w, destination.h];

    destination_origin
        + (point - source_origin)
            .component_div(&source_size)
            .component_mul(&destination_size)
}
