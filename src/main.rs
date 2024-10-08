use std::{sync::mpsc, time::Duration};

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

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();

    std::thread::spawn(move || -> LuaResult<()> {
        // FIXME: IDs and random positions -> do this in add_(lua)_player or
        // something like that
        let dir1 = "players/kai";
        let player1 = Player::new(dir1, 0, 70.0, 450.0)?;
        let meta1 = LuaImpl::read_meta(dir1)?;
        let lua_impl1 = load_lua_player(dir1, &meta1)?;

        let dir2 = "players/lloyd";
        let player2 = Player::new(dir2, 1, 700.0, 440.0)?;
        let meta2 = LuaImpl::read_meta(dir2)?;
        let lua_impl2 = load_lua_player(dir2, &meta2)?;

        let mut game = Game::new();
        game.add_lua_player(player1, lua_impl1)?;
        game.add_lua_player(player2, lua_impl2)?;

        let delay = Duration::from_millis(5);
        run_game(&mut game, &delay, &game_writer)
    });

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.draw_fps(5, 5);

        let mut latest_data = None;
        while let Ok(data) = game_reader.try_recv() {
            latest_data = Some(data);
        }
        if let Some(data) = latest_data {
            d.clear_background(raylib::prelude::Color::BLACK);
            render::game(&mut d, &data);
        }
    }

    Ok(())
}
