use std::{env, path::PathBuf};

use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{Backend, Conf, FullscreenType, WindowMode, WindowSetup},
    event::{self, EventHandler},
    input::keyboard::KeyInput,
    winit::keyboard::{Key, NamedKey},
};

use crate::objects::pixels::Pixels;

pub(crate) mod collections;
pub(crate) mod objects;

fn main() {
    let mut builder = ContextBuilder::new("pixel_part_simulation", "Mycellf").default_conf(Conf {
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
            title: "Pixel Part Simulation".to_owned(),
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

    let (ctx, event_loop) = builder.build().unwrap();

    let state = State::default();

    event::run(ctx, event_loop, state).unwrap();
}

#[derive(Debug)]
pub(crate) struct State {
    fullscreen: bool,
}

impl Default for State {
    fn default() -> Self {
        State { fullscreen: true }
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

        if input.event.logical_key == Key::Named(NamedKey::Escape) {
            ctx.request_quit();
        }

        if input.event.logical_key == Key::Named(NamedKey::F11) {
            self.fullscreen ^= true;

            ctx.gfx.set_fullscreen(if self.fullscreen {
                FullscreenType::Desktop
            } else {
                FullscreenType::Windowed
            })?;
        }

        Ok(())
    }
}
