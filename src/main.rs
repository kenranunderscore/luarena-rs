use std::{
    path::Path,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};

use game::*;
use render::GameRenderer;
use settings::*;

mod color;
mod game;
mod math_utils;
mod player;
mod render;
mod settings;

// fn _run_replay(
//     history_file: &Path,
//     sender: Sender<StepEvents>,
//     delay: &Duration,
//     cancel: &Arc<AtomicBool>,
// ) -> Option<()> {
//     let f = std::fs::File::open(history_file).ok()?;
//     let steps: Vec<StepEvents> = todo!();
//     for step_events in steps {
//         if cancel.load(Ordering::Relaxed) {
//             break;
//         }
//         sender
//             .send(step_events)
//             .expect("Failed sending step events");
//         std::thread::sleep(*delay);
//     }
//     Some(())
// }

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
        game.add_wasm_player(Path::new("players/nya"))?;

        let delay = Duration::from_millis(7);
        // run_replay(Path::new("events"), game_writer, &delay, &cancel_ref).unwrap();
        run_game(&mut game, &delay, game_writer, &cancel_ref)?;
        Ok(())
    });

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
