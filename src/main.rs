use mlua::prelude::*;
use raylib::prelude::*;

struct LuaPlayer {
    lua: Lua,
    key: LuaRegistryKey,
}

impl LuaPlayer {
    fn new(code: &str) -> LuaResult<Self> {
        let lua = Lua::new();
        lua.load_from_std_lib(LuaStdLib::ALL_SAFE)?;
        let table_key = {
            let t: LuaTable = lua.load(code).eval()?;
            lua.create_registry_value(t)?
        };
        Ok(Self {
            lua,
            key: table_key,
        })
    }
}

struct Player {
    lua_player: LuaPlayer,
    x: i32,
    y: i32,
}

struct GameState {
    tick: i16,
    players: Vec<Player>,
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

fn render_players(mut d: raylib::drawing::RaylibDrawHandle, players: &Vec<Player>) {
    for p in players {
        d.draw_circle(p.x, p.y, 25.0, Color::GREENYELLOW);
    }
}

// FIXME: is there a way to say "immutable Vec, but mutable elements?"
fn advance_players(players: &mut Vec<Player>) {
    for p in players.iter_mut() {
        p.x += 1;
    }
}

fn load_lua_player(file_path: &str) -> LuaResult<LuaPlayer> {
    let code = std::fs::read_to_string(file_path)?;
    LuaPlayer::new(&code)
}

fn step(state: &mut GameState) {
    advance_players(&mut state.players);
}

fn main() -> LuaResult<()> {
    let player1 = Player {
        lua_player: load_lua_player("foo.lua")?,
        x: 30,
        y: 50,
    };
    let player2 = Player {
        lua_player: load_lua_player("foo.lua")?,
        x: 100,
        y: 220,
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_player_can_be_loaded_from_code() {
        let _player = LuaPlayer::new("return { on_tick = function(n) print('on_tick') end }")
            .expect("lua player could not be created");
    }

    #[test]
    fn call_lua_player_method() {
        let player = LuaPlayer::new("return { on_tick = function(n) return n+1 end }")
            .expect("lua player could not be created");
        let t: LuaTable = player
            .lua
            .registry_value(&player.key)
            .expect("key not found");
        let res: i32 = t.call_function("on_tick", 17).expect("call failed");
        assert_eq!(res, 18);
    }
}
