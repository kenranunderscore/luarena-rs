use std::{sync::mpsc, time::Duration};

use mlua::prelude::*;
use raylib::prelude::*;

mod game;
mod math_utils;
mod render;
mod settings;

use game::*;
use render::{GameData, PlayerData};
use settings::*;

fn main() -> LuaResult<()> {
    let (game_writer, game_reader) = mpsc::channel();

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();

    std::thread::spawn(move || -> LuaResult<()> {
        let mut game = Game::new();
        let mut lua_impls: Vec<LuaImpl> = Vec::new();
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

        game.add_lua_player(player1, lua_impl1, &mut lua_impls)?;
        game.add_lua_player(player2, lua_impl2, &mut lua_impls)?;

        let mut event_manager = EventManager::new();
        loop {
            std::thread::sleep(Duration::from_millis(5));
            step(&mut game, &mut event_manager, &mut lua_impls)?;

            let mut game_data = GameData::new();
            for player in game.living_players() {
                let p = player.pos.read().unwrap();
                game_data.players.push(PlayerData {
                    color: player.meta.color.clone(),
                    x: p.x,
                    y: p.y,
                    heading: player.heading,
                    head_heading: player.effective_head_heading(),
                    arms_heading: player.effective_arms_heading(),
                });
            }
            for attack in game.attacks.iter() {
                game_data.attacks.push(attack.pos.clone());
            }
            game_writer.send(game_data).unwrap();
        }
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
