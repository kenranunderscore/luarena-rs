use mlua::prelude::*;
use raylib::prelude::*;

struct GameState {
    tick: i16,
    players: Vec<LuaPlayer>,
}

struct LuaPlayer {
    _lua_state: Lua,
    x: i32,
    y: i32,
}

fn _main_lua() -> LuaResult<()> {
    let l = Lua::new();
    let mt = l.create_table()?;
    mt.set(1, "one")?;
    mt.set("two", 2)?;
    l.globals().set("mmm", mt)?;
    l.load("for k,v in pairs(mmm) do print(k,v) end").exec()?;
    Ok(())
}

fn _draw_line_in_direction(
    mut d: raylib::drawing::RaylibDrawHandle,
    x: i32,
    y: i32,
    angle: f32,
    length: f32,
) {
    let dx = angle.sin() * length;
    let dy = angle.cos() * length;
    d.draw_line(
        x,
        y,
        x + dx.round() as i32,
        y - dy.round() as i32,
        Color::RED,
    );
}

fn render_players(mut d: raylib::drawing::RaylibDrawHandle, players: &Vec<LuaPlayer>) {
    for p in players {
        d.draw_circle(p.x, p.y, 25.0, Color::GREENYELLOW);
    }
}

// FIXME: is there a way to say "immutable Vec, but mutable elements?"
fn advance_players(players: &mut Vec<LuaPlayer>) {
    for p in players.iter_mut() {
        p.x += 1;
    }
}

fn create_lua_player(_file_path: &str, x: i32, y: i32) -> LuaResult<LuaPlayer> {
    let ls = Lua::new();
    ls.load_from_std_lib(LuaStdLib::ALL_SAFE)?;
    let code = std::fs::read_to_string("foo.lua")?;
    ls.load(&code).exec()?;
    Ok(LuaPlayer {
        _lua_state: ls,
        x,
        y,
    })
}

fn step(state: &mut GameState) {
    advance_players(&mut state.players);
}

fn main() -> LuaResult<()> {
    let player1 = create_lua_player("foo.lua", 30, 50)?;
    let player2 = create_lua_player("foo.lua", 100, 220)?;
    let players = vec![player1, player2];
    let mut state = GameState { tick: 0, players };
    let (mut rl, thread) = raylib::init().size(400, 400).title("hello world").build();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        state.tick += 1;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::GRAY);
        step(&mut state);
        render_players(d, &state.players);
    }
    Ok(())
}
