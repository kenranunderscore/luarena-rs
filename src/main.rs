use mlua::prelude::*;
use raylib::prelude::*;

mod game;
mod math_utils;
mod render;
mod settings;

use game::*;
use settings::*;

fn main() -> LuaResult<()> {
    // FIXME: IDs
    let player1 = Player::new("players/kai", 1, 70, 450)?;
    let player2 = Player::new("players/lloyd", 2, 700, 440)?;

    let mut state = GameState::new();
    state.players = vec![player1, player2];
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .msaa_4x()
        .build();
    let mut event_manager = EventManager::new();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.draw_fps(5, 5);
        d.clear_background(raylib::prelude::Color::BLACK);
        step(&mut state, &mut event_manager)?;
        render::game(&mut d, &state);
        state.tick += 1;
    }
    Ok(())
}
