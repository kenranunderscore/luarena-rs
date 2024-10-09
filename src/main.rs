use std::{
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};

use mlua::prelude::*;
use raylib::prelude::*;

mod game;
mod math_utils;
mod render;
mod settings;

use game::*;
use settings::*;

fn main() -> LuaResult<()> {
    let (game_writer, game_reader) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_ref = cancel.clone();

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();

    let game_thread = std::thread::spawn(move || -> LuaResult<()> {
        let mut game = Game::new();
        game.add_lua_player("players/kai", 70.0, 450.0)?;
        game.add_lua_player("players/lloyd", 700.0, 440.0)?;

        let delay = Duration::from_millis(1);
        run_game(&mut game, &delay, &game_writer, &cancel_ref)
    });

    rl.set_target_fps(60);
    let mut latest_data = None;
    while !rl.window_should_close() && !game_thread.is_finished() {
        let mut d = rl.begin_drawing(&thread);
        d.draw_fps(5, 5);

        while let Ok(data) = game_reader.try_recv() {
            latest_data = Some(data);
        }
        d.clear_background(raylib::prelude::Color::BLACK);
        if let Some(data) = &latest_data {
            render::game(&mut d, &data);
        }
    }

    if game_thread.is_finished() {
        return game_thread.join().unwrap();
    }

    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
