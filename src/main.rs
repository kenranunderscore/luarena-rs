use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;
use raylib::prelude::*;

#[derive(PartialEq, Debug)]
enum PlayerCommand {
    Attack(f32),
    TurnHead(f32),
    Move(f32),
}

impl<'a> FromLua<'a> for PlayerCommand {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Table(t) => match t.get::<&str, String>("tag")?.as_str() {
                "move" => Ok(PlayerCommand::Move(t.get("distance")?)),
                "attack" => Ok(PlayerCommand::Attack(t.get("angle")?)),
                "turn_head" => Ok(PlayerCommand::TurnHead(t.get("turn_head")?)),
                "turn_head_right" => Ok(PlayerCommand::TurnHead(t.get("turn_head")?)),
                "turn_head_left" => Ok(PlayerCommand::TurnHead(-t.get("turn_head")?)),
                s => todo!("invalid tag: {}", s),
            },
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Foo",
                message: Some("expected valid player command".to_string()),
            }),
        }
    }
}

fn create_tagged_table<'a>(lua: &'a Lua, tag: &str) -> LuaResult<LuaTable<'a>> {
    let t = lua.create_table()?;
    t.set("tag", tag)?;
    Ok(t)
}

impl<'a> IntoLua<'a> for PlayerCommand {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        match self {
            PlayerCommand::Attack(angle) => {
                let t = create_tagged_table(&lua, "attack")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            PlayerCommand::TurnHead(angle) => {
                let t = create_tagged_table(&lua, "turn_head")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            PlayerCommand::Move(dist) => {
                let t = create_tagged_table(&lua, "move")?;
                t.set("distance", dist)?;
                Ok(LuaValue::Table(t))
            }
        }
    }
}

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

    fn on_tick(&self, tick: i32) -> LuaResult<PlayerCommand> {
        let res = self.table()?.call_function("on_tick", tick)?;
        Ok(res)
    }
}

struct Point {
    x: i32,
    y: i32,
}

struct Player {
    lua_player: LuaPlayer,
    pos: Rc<RefCell<Point>>,
}

impl Player {
    fn register_lua_library(&self) -> LuaResult<()> {
        {
            let lua = &self.lua_player.lua;
            let me = lua.create_table()?;

            let pos_ref = Rc::clone(&self.pos);
            let x = lua.create_function(move |_, _: ()| Ok(pos_ref.borrow().x))?;
            me.set("x", x)?;

            // need to clone the ref again, as we move to make the closure work
            let pos_ref = Rc::clone(&self.pos);
            let y = lua.create_function(move |_, _: ()| Ok(pos_ref.borrow().y))?;
            me.set("y", y)?;

            let move_cmd = lua.create_function(|_, dist: f32| Ok(PlayerCommand::Move(dist)))?;
            me.set("move", move_cmd)?;

            let attack_cmd =
                lua.create_function(|_, angle: f32| Ok(PlayerCommand::Attack(angle)))?;
            me.set("attack", attack_cmd)?;

            let turn_head_cmd =
                lua.create_function(|_, angle: f32| Ok(PlayerCommand::Move(angle)))?;
            me.set("turn_head", turn_head_cmd)?;

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
        let pos = p.pos.borrow();
        d.draw_circle(pos.x, pos.y, 25.0, Color::GREENYELLOW);
    }
}

// FIXME: is there a way to say "immutable Vec, but mutable elements?"
fn advance_players(players: &mut Vec<Player>) {
    for p in players.iter_mut() {
        let mut pos = p.pos.borrow_mut();
        pos.x += 1;
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
        pos: Rc::new(RefCell::new(Point { x: 30, y: 50 })),
    };
    player1.register_lua_library()?;
    let player2 = Player {
        lua_player: load_lua_player("foo.lua")?,
        pos: Rc::new(RefCell::new(Point { x: 50, y: 220 })),
    };
    player2.register_lua_library()?;

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
        let player = LuaPlayer::new(
            "return { on_tick = function(n) return { tag = \"move\", distance = 13.12 } end }",
        )
        .expect("lua player could not be created");
        let res: PlayerCommand = player.on_tick(17).expect("on_tick failed");
        assert_eq!(res, PlayerCommand::Move(13.12));
    }
}
