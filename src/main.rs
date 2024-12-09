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

fn main() {
    let cli = cli::Cli::parse();
    match cli.mode {
        cli::Mode::Battle {
            player_dirs,
            headless,
        } => {
            if headless {
                // FIXME: get rid of unwraps
                let mut game = Game::with_players(&player_dirs).unwrap();
                let _ = run_game_headless(&mut game).unwrap();
            } else {
                with_gui(|writer, cancel| {
                    let player_dirs = player_dirs.clone();
                    let cancel = cancel.clone();
                    std::thread::spawn(move || {
                        let mut game = Game::with_players(&player_dirs)?;
                        let delay = Duration::from_millis(7);
                        run_game(&mut game, &delay, writer, cancel)
                    })
                });
            }
        }
        cli::Mode::Replay { recording } => with_gui(|writer, cancel| {
            let recording = recording.clone();
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                let delay = Duration::from_millis(5);
                run_replay(&recording, writer, &delay, cancel)
            })
        }),
    };
}

fn run_replay(
    history_file: &Path,
    sender: mpsc::Sender<StepEvents>,
    delay: &Duration,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let _f = std::fs::File::open(history_file)
        .map_err(|e| format!("Could not load {history_file:?}. Error: {e}"))?;
    let steps: Vec<StepEvents> = vec![];
    for step_events in steps {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        sender
            .send(step_events)
            .expect("Failed sending step events");
        std::thread::sleep(*delay);
    }
    Ok(())
}

fn with_gui<F, Err>(run: F)
where
    F: Fn(mpsc::Sender<StepEvents>, &Arc<AtomicBool>) -> std::thread::JoinHandle<Result<(), Err>>,
    Err: std::fmt::Debug,
{
    let (game_writer, game_reader) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let game_thread = run(game_writer, &cancel);

    let (mut rl, thread) = raylib::init()
        .log_level(raylib::ffi::TraceLogLevel::LOG_WARNING)
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
            Ok(_) => println!("Game finished"),
            Err(err) => println!("Crash: {err:?}"),
        }
    } else {
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = game_thread.join();
    }
}
