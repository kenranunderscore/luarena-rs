use std::{
    path::Path,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};

use clap::Parser;
use game::*;
use render::GameRenderer;
use settings::*;

mod cli;
mod color;
mod game;
mod math_utils;
mod player;
mod render;
mod settings;

fn run_replay(
    history_file: &Path,
    sender: mpsc::Sender<StepEvents>,
    delay: &Duration,
    cancel: &Arc<AtomicBool>,
) -> Option<()> {
    let f = std::fs::File::open(history_file).ok()?;
    let steps: Vec<StepEvents> = todo!();
    for step_events in steps {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        sender
            .send(step_events)
            .expect("Failed sending step events");
        std::thread::sleep(*delay);
    }
    Some(())
}

fn main() {
    let cli = cli::Cli::parse();
    match cli.mode {
        cli::Mode::Battle {
            player_dirs,
            headless,
        } => {
            if headless {
                let game_thread = std::thread::spawn(move || -> Result<(), GameError> {
                    let mut game = Game::new();
                    for player_dir in player_dirs {
                        game.add_lua_player(&player_dir)?;
                    }
                    run_game_headless(&mut game)
                });
                let _ = game_thread.join().unwrap();
            } else {
                let (game_writer, game_reader) = mpsc::channel();
                let cancel = Arc::new(AtomicBool::new(false));
                let cancel_ref = cancel.clone();

                let game_thread = std::thread::spawn(move || -> Result<(), GameError> {
                    let mut game = Game::new();
                    for player_dir in player_dirs {
                        game.add_lua_player(&player_dir)?;
                    }
                    let delay = Duration::from_millis(7);
                    run_game(&mut game, &delay, game_writer, &cancel_ref)
                });

                let (mut rl, thread) = raylib::init()
                    .size(WIDTH, HEIGHT)
                    .title("hello world")
                    .msaa_4x()
                    .build();
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
        }
        cli::Mode::Replay { recording } => {
            // run_replay(Path::new("events"), game_writer, &delay, &cancel_ref).unwrap();
            todo!("show replay {recording:?}")
        }
    };
}
