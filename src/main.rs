use mlua::prelude::*;
use raylib::prelude::*;

struct game_state {
    tick: i16,
}

struct lua_player {
    lua_state: Lua,
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

fn draw_line_in_direction(
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

fn render_player(mut d: raylib::drawing::RaylibDrawHandle, p: &lua_player) {
    d.draw_circle(p.x, p.y, 25.0, Color::GREENYELLOW);
}

fn advance_player(p: &mut lua_player) {
    p.x += 1;
}

fn main() {
    let mut player1 = lua_player {
        lua_state: Lua::new(),
        x: 30,
        y: 50,
    };
    let mut state = game_state { tick: 0 };
    let (mut rl, thread) = raylib::init().size(400, 400).title("hello world").build();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        state.tick += 1;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::GRAY);
        advance_player(&mut player1);
        render_player(d, &player1);
    }
}
