use std::{cell::RefCell, rc::Rc};

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

    fn table(&self) -> LuaResult<LuaTable> {
        let t = self.lua.registry_value(&self.key)?;
        Ok(t)
    }

    fn on_tick(&self, tick: i32) -> LuaResult<i32> {
        let res = self.table()?.call_function("on_tick", tick)?;
        Ok(res)
    }
}

struct Player {
    lua_player: LuaPlayer,
    x: Rc<RefCell<i32>>,
    y: Rc<RefCell<i32>>,
}

impl Player {
    fn register_lua_library(&self) -> LuaResult<()> {
        {
            let lua = &self.lua_player.lua;
            let me = lua.create_table()?;

            let x_ref = Rc::clone(&self.x);
            let x = lua.create_function(move |_, _: ()| Ok(*x_ref.borrow()))?;
            me.set("x", x)?;

            let y_ref = Rc::clone(&self.y);
            let y = lua.create_function(move |_, _: ()| Ok(*y_ref.borrow()))?;
            me.set("y", y)?;

            lua.globals().set("me", me)?;
            Ok(())
        }
    }
}

struct GameState {
    tick: i32,
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
        d.draw_circle(*p.x.borrow(), *p.y.borrow(), 25.0, Color::GREENYELLOW);
    }
}

// FIXME: is there a way to say "immutable Vec, but mutable elements?"
fn advance_players(players: &mut Vec<Player>) {
    for p in players.iter_mut() {
        let mut x = p.x.borrow_mut();
        *x += 1;
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
        x: Rc::new(RefCell::new(30)),
        y: Rc::new(RefCell::new(50)),
    };
    player1.register_lua_library()?;
    let players = vec![player1];
    let mut state = GameState { tick: 0, players };
    let (mut rl, thread) = raylib::init().size(400, 400).title("hello world").build();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        state.tick += 1;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::GRAY);
        step(&mut state);
        render_players(d, &state.players);
        let p = state.players.first().expect("player here");
        p.lua_player.on_tick(state.tick)?;
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
    fn call_on_tick() {
        let player = LuaPlayer::new("return { on_tick = function(n) return n+1 end }")
            .expect("lua player could not be created");
        let res: i32 = player.on_tick(17).expect("on_tick failed");
        assert_eq!(res, 18);
    }
}
