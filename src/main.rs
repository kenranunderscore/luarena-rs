use std::{
    path::Path,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};

use raylib::prelude::*;

mod game;
mod math_utils;
mod render;
mod settings;

use game::*;
use render::GameRenderer;
use settings::*;

fn main() {
    let (game_writer, game_reader) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_ref = cancel.clone();

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();

    let game_thread = std::thread::spawn(move || -> Result<(), GameError> {
        let mut game = Game::new();
        game.add_lua_player(Path::new("players/kai"))?;
        game.add_lua_player(Path::new("players/lloyd"))?;

        let delay = Duration::from_millis(7);
        run_game(&mut game, &delay, game_writer, &cancel_ref)
    });

    // rl.set_target_fps(60);
    let mut renderer = GameRenderer::new(&game_reader);
    while !rl.window_should_close() && !game_thread.is_finished() {
        renderer.step(&mut rl, &thread);
    }

    if game_thread.is_finished() {
        match game_thread.join().unwrap() {
            Ok(_) => println!("game finished"),
            Err(e) => println!("error: {e}"),
        }
    } else {
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = game_thread.join();
    }
}
