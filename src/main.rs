use mlua::prelude::*;
use raylib::prelude::*;

mod game;
mod render;
mod settings;

use settings::*;
use game::*;

fn main() -> LuaResult<()> {
    // FIXME: IDs
    let player1 = Player::new("players/kai", 1, 70, 100)?;
    let player2 = Player::new("players/lloyd", 2, 400, 100)?;

    let mut state = GameState::new();
    state.players = vec![player1, player2];
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .build();
    let mut event_manager = EventManager::new();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        state.tick += 1;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(raylib::prelude::Color::BLACK);
        step(&mut state, &mut event_manager)?;
        render::players(&mut d, &state.players);
    }
    Ok(())
}
