use std::{env, path::PathBuf};

use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    input::keyboard::KeyInput,
    winit::keyboard::{Key, NamedKey},
};

pub(crate) mod collections;

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
}

impl State {
    fn new(_ctx: &mut Context) -> GameResult<Self> {
        Ok(State { fullscreen: true })
    }
}

impl EventHandler for State {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        Ok(())
    }

    fn draw(&mut self, _ctx: &mut Context) -> GameResult {
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
}
