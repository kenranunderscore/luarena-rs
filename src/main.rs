use std::{
    sync::{Arc, RwLock},
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
    let game_ref = Arc::new(RwLock::new(Game::new()));
    let writable_game_ref = Arc::clone(&game_ref);

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();

    std::thread::spawn(move || {
        // FIXME: error handling and clean up this mess
        let mut lua_impls: Vec<LuaImpl> = Vec::new();
        {
            let mut initial_game = writable_game_ref.write().unwrap();

            // FIXME: IDs and random positions -> do this in add_(lua)_player or
            // something like that
            let dir1 = "players/kai";
            let player1 = Player::new(dir1, 1, 70, 450)?;
            let meta1 = LuaImpl::read_meta(dir1)?;
            let lua_impl1 = load_lua_player(dir1, &meta1)?;

            let dir2 = "players/lloyd";
            let player2 = Player::new(dir2, 2, 700, 440)?;
            let meta2 = LuaImpl::read_meta(dir2)?;
            let lua_impl2 = load_lua_player(dir2, &meta2)?;

            initial_game.add_lua_player(player1, lua_impl1, &mut lua_impls)?;
            initial_game.add_lua_player(player2, lua_impl2, &mut lua_impls)?;
        }

        let mut event_manager = EventManager::new();
        loop {
            std::thread::sleep(Duration::from_millis(5));
            let mut game = writable_game_ref.write().unwrap();
            step(&mut game, &mut event_manager, &mut lua_impls).expect("step failed");
        }
        let res: LuaResult<()> = Ok(());
        res
    });

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.draw_fps(5, 5);
        d.clear_background(raylib::prelude::Color::BLACK);
        let game = game_ref.read().unwrap();
        render::game(&mut d, &game);
    }

    Ok(())
}
