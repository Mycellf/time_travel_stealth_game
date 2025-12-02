use std::{env, path::PathBuf};

use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    graphics::{Canvas, Color, Rect, Sampler},
    input::keyboard::KeyInput,
    winit::{
        event::MouseButton,
        keyboard::{Key, NamedKey},
        platform::modifier_supplement::KeyEventExtModifierSupplement,
    },
};
use nalgebra::{Point2, UnitVector2, Vector2, point, vector};

use crate::{
    input::DirectionalInput,
    level::{Level, entity::player::Player},
};

pub(crate) mod collections;
pub(crate) mod input;
pub(crate) mod level;

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

pub(crate) struct State {
    fullscreen: bool,
    window_size: Vector2<f32>,

    level: Level,
}

impl State {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        use std::f64::consts::PI;

        Ok(State {
            fullscreen: true,
            window_size: vector![800.0, 600.0],

            level: Level::new(
                ctx,
                vec![Box::new(Player {
                    position: point![0.0, 0.0],
                    size: vector![6.0, 6.0],

                    mouse_position: point![0.0, 0.0],
                    view_direction: UnitVector2::new_normalize(vector![1.0, 0.0]),
                    view_width: PI * 1.0 / 2.0,

                    speed: 40.0,
                    motion_input: DirectionalInput::new(
                        Key::Character("d".into()),
                        Key::Character("w".into()),
                        Key::Character("a".into()),
                        Key::Character("s".into()),
                    ),
                })],
            ),
        })
    }
}

impl State {
    pub const SCREEN_HEIGHT: f32 = 100.0;

    fn screen_rect(&self) -> Rect {
        rectangle_of_centered_camera(self.window_size, point![0.0, 0.0], Self::SCREEN_HEIGHT)
    }

    fn screen_to_world(&self, point: Point2<f32>) -> Point2<f32> {
        let world = self.screen_rect();
        let screen = Rect::new(0.0, 0.0, self.window_size.x, self.window_size.y);

        transform_between_rectangles(screen, world, point)
    }

    fn screen_to_world_scale_factor(&self) -> f32 {
        Self::SCREEN_HEIGHT / self.window_size.y
    }
}

impl EventHandler for State {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.level.update(ctx);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Some(Color::BLACK));
        canvas.set_sampler(Sampler::nearest_clamp());

        canvas.set_screen_coordinates(self.screen_rect());

        self.level.draw(ctx, &mut canvas);

        canvas.finish(ctx)?;

        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, repeated: bool) -> GameResult {
        self.level.key_down(input.clone(), repeated);

        if !repeated {
            match input.event.key_without_modifiers() {
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
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: KeyInput) -> GameResult {
        self.level.key_up(input);

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.level.mouse_down(button, point![x as f64, y as f64]);

        Ok(())
    }

    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.level.mouse_up(button, point![x as f64, y as f64]);

        Ok(())
    }

    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    ) -> GameResult {
        self.level.mouse_moved(
            self.screen_to_world(point![x, y]).map(|x| x as f64),
            (vector![dx, dy] * self.screen_to_world_scale_factor()).map(|x| x as f64),
        );

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
