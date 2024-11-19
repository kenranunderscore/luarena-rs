use std::path::Path;
use std::sync::Arc;

use mlua::prelude::*;

use crate::color::Color;
use crate::math_utils::{self, Point};
use crate::player::*;

impl<'a> IntoLua<'a> for Point {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        let t = lua.create_table()?;
        t.set("x", self.x)?;
        t.set("y", self.y)?;
        Ok(LuaValue::Table(t))
    }
}

impl<'a> FromLua<'a> for MovementDirection {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::String(s) => match s.to_str()? {
                "forward" => Ok(MovementDirection::Forward),
                "backward" => Ok(MovementDirection::Backward),
                "left" => Ok(MovementDirection::Left),
                "right" => Ok(MovementDirection::Right),
                other => todo!("invalid direction: {other}"),
            },
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "MovementDirection",
                message: Some("expected valid direction".to_string()),
            }),
        }
    }
}

impl<'a> IntoLua<'a> for MovementDirection {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        let s = match self {
            MovementDirection::Forward => "forward",
            MovementDirection::Backward => "backward",
            MovementDirection::Left => "left",
            MovementDirection::Right => "right",
        };
        s.into_lua(lua)
    }
}

impl<'a> FromLua<'a> for Command {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Table(t) => match t.get::<&str, String>("tag")?.as_str() {
                "move" => {
                    let dist = t.get("distance")?;
                    let dir: MovementDirection = t.get("direction")?;
                    Ok(Command::Move(dir, dist))
                }
                "attack" => Ok(Command::Attack),
                "turn" => Ok(Command::Turn(t.get("angle")?)),
                "turn_head" => Ok(Command::TurnHead(t.get("angle")?)),
                "turn_arms" => Ok(Command::TurnArms(t.get("angle")?)),
                s => todo!("invalid tag: {s}"),
            },
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "PlayerCommand",
                message: Some("expected valid player command".to_string()),
            }),
        }
    }
}

impl<'a> FromLua<'a> for Commands {
    fn from_lua(value: LuaValue<'a>, lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Nil => Ok(Commands::none()),
            _ => Ok(Commands::from(Vec::<Command>::from_lua(value, lua)?)),
        }
    }
}

fn create_tagged_table<'a>(lua: &'a Lua, tag: &str) -> LuaResult<LuaTable<'a>> {
    let t = lua.create_table()?;
    t.set("tag", tag)?;
    Ok(t)
}

impl<'a> IntoLua<'a> for Command {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        match self {
            Command::Attack => {
                let t = create_tagged_table(&lua, "attack")?;
                Ok(LuaValue::Table(t))
            }
            Command::Turn(angle) => {
                let t = create_tagged_table(&lua, "turn")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            Command::TurnHead(angle) => {
                let t = create_tagged_table(&lua, "turn_head")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            Command::TurnArms(angle) => {
                let t = create_tagged_table(&lua, "turn_arms")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            Command::Move(dir, dist) => {
                let t = create_tagged_table(&lua, "move")?;
                t.set("distance", dist)?;
                t.set("direction", dir)?;
                Ok(LuaValue::Table(t))
            }
        }
    }
}

impl<'a> FromLua<'a> for Color {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        // TODO: read from hex string as well
        match value {
            LuaValue::Table(t) => Ok(Color {
                red: t.get("red")?,
                green: t.get("green")?,
                blue: t.get("blue")?,
            }),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Color",
                message: Some("expected valid direction".to_string()),
            }),
        }
    }
}

pub struct LuaImpl {
    lua: Lua,
    key: LuaRegistryKey,
}

impl LuaImpl {
    pub fn new(code: &str) -> LuaResult<Self> {
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

    fn call_event_handler<A>(&self, name: &str, args: A) -> Result<Commands, EventError>
    where
        A: for<'a> IntoLuaMulti<'a>,
    {
        let t = self.table()?;
        let res = if t.contains_key(name)? {
            t.call_function(name, args)?
        } else {
            Commands::none()
        };
        Ok(res)
    }

    fn register_lua_library(
        &self,
        player_state: &State,
        intent: &ReadableFromImpl<Intent>,
    ) -> LuaResult<()> {
        let lua = &self.lua;
        let mut me = lua.create_table()?;
        register_player_state_accessors(player_state, &mut me, &lua, intent)?;
        register_player_commands(&mut me, lua)?;
        lua.globals().set("me", me)?;
        register_utils(lua)?;
        Ok(())
    }

    // FIXME: what to use that's more generic than `Path`?
    pub fn load(
        player_dir: &Path,
        entrypoint: &Path,
        player_state: &State,
        intent: &ReadableFromImpl<Intent>,
    ) -> LuaResult<Self> {
        let file = player_dir.join(entrypoint);
        let code = std::fs::read_to_string(file)?;
        let res = Self::new(&code)?;
        res.register_lua_library(player_state, intent)?;
        Ok(res)
    }
}

impl Impl for LuaImpl {
    fn on_event(&mut self, event: &Event) -> Result<Commands, EventError> {
        match event {
            Event::Tick(n) => self.call_event_handler("on_tick", *n),
            Event::RoundStarted(n) => self.call_event_handler("on_round_started", *n),
            Event::EnemySeen(name, pos) => {
                self.call_event_handler("on_enemy_seen", (name.to_string(), pos.clone()))
            }
            Event::HitBy(id) => self.call_event_handler("on_hit_by", *id),
            Event::AttackHit(id, pos) => {
                self.call_event_handler("on_attack_hit", (*id, pos.clone()))
            }
            Event::Death => self.call_event_handler("on_death", ()),
        }
    }
}

impl Meta {
    pub fn from_lua(player_dir: &Path) -> LuaResult<(Meta, String)> {
        let lua = Lua::new();
        let meta_file = player_dir.join("meta.lua");
        let code = std::fs::read_to_string(meta_file)?;
        lua.load(&code).exec()?;
        let name = lua.globals().get("name")?;
        let color = lua.globals().get("color")?;
        let version = lua.globals().get("version")?;
        let entrypoint = match lua.globals().get("entrypoint") {
            Ok(file_name) => file_name,
            Err(_) => String::from("main.lua"),
        };
        Ok((
            Meta {
                name,
                color,
                _version: version,
            },
            entrypoint,
        ))
    }
}

impl From<mlua::Error> for EventError {
    fn from(err: mlua::Error) -> Self {
        Self {
            message: format!("{err}"),
        }
    }
}

fn register_player_state_accessors(
    player: &State,
    t: &mut LuaTable,
    lua: &Lua,
    intent: &ReadableFromImpl<Intent>,
) -> LuaResult<()> {
    // Each accessor needs its own reference to the data, that's why we need to
    // clone the Arcs multiple times
    let r = Arc::clone(&player.pos);
    t.set(
        "x",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().x))?,
    )?;

    let r = Arc::clone(&player.pos);
    t.set(
        "y",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().y))?,
    )?;

    let r = Arc::clone(&player.hp);
    t.set(
        "hp",
        lua.create_function(move |_, _: ()| Ok(*r.read().unwrap()))?,
    )?;

    let r = Arc::clone(&player.heading);
    t.set(
        "heading",
        lua.create_function(move |_, _: ()| Ok(*r.read().unwrap()))?,
    )?;

    let r = Arc::clone(&player.head_heading);
    t.set(
        "head_heading",
        lua.create_function(move |_, _: ()| Ok(*r.read().unwrap()))?,
    )?;

    let r = Arc::clone(&player.arms_heading);
    t.set(
        "arms_heading",
        lua.create_function(move |_, _: ()| Ok(*r.read().unwrap()))?,
    )?;

    let r = Arc::clone(&player.attack_cooldown);
    t.set(
        "attack_cooldown",
        lua.create_function(move |_, _: ()| Ok(*r.read().unwrap()))?,
    )?;

    let r = Arc::clone(&intent);
    t.set(
        "turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_angle))?,
    )?;

    let r = Arc::clone(&intent);
    t.set(
        "head_turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_head_angle))?,
    )?;

    let r = Arc::clone(&intent);
    t.set(
        "arms_turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_arms_angle))?,
    )?;

    let name = player.meta.name.clone();
    t.set(
        "log",
        lua.create_function(move |_, msg: LuaString| {
            let msg = msg.to_str()?;
            println!("[{name}] {msg}");
            Ok(())
        })?,
    )?;

    Ok(())
}

fn register_player_commands(t: &mut LuaTable, lua: &Lua) -> LuaResult<()> {
    let move_ =
        lua.create_function(|_, dist: f32| Ok(Command::Move(MovementDirection::Forward, dist)))?;
    t.set("move", move_)?;

    let move_backward =
        lua.create_function(|_, dist: f32| Ok(Command::Move(MovementDirection::Backward, dist)))?;
    t.set("move_backward", move_backward)?;

    let move_left =
        lua.create_function(|_, dist: f32| Ok(Command::Move(MovementDirection::Left, dist)))?;
    t.set("move_left", move_left)?;

    let move_right =
        lua.create_function(|_, dist: f32| Ok(Command::Move(MovementDirection::Right, dist)))?;
    t.set("move_right", move_right)?;

    let attack = lua.create_function(|_, _: ()| Ok(Command::Attack))?;
    t.set("attack", attack)?;

    let turn = lua.create_function(|_, angle: f32| Ok(Command::Turn(angle)))?;
    t.set("turn", &turn)?;

    let turn_head = lua.create_function(|_, angle: f32| Ok(Command::TurnHead(angle)))?;
    t.set("turn_head", turn_head)?;

    let turn_arms = lua.create_function(|_, angle: f32| Ok(Command::TurnArms(angle)))?;
    t.set("turn_arms", turn_arms)?;

    Ok(())
}

fn register_utils(lua: &Lua) -> LuaResult<()> {
    let utils = lua.create_table()?;
    utils.set(
        "normalize_absolute_angle",
        lua.create_function(|_, angle: f32| Ok(math_utils::normalize_absolute_angle(angle)))?,
    )?;
    utils.set(
        "normalize_relative_angle",
        lua.create_function(|_, angle: f32| Ok(math_utils::normalize_relative_angle(angle)))?,
    )?;
    utils.set(
        "to_radians",
        lua.create_function(|_, angle: f32| Ok(angle.to_radians()))?,
    )?;
    utils.set(
        "from_radians",
        lua.create_function(|_, angle: f32| Ok(angle.to_degrees()))?,
    )?;

    lua.globals().set("utils", utils)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod lua_player {
        use super::*;

        #[test]
        fn lua_player_can_be_loaded_from_code() {
            LuaImpl::new("return {}").expect("lua player could not be created");
        }

        #[test]
        fn call_on_tick() {
            let mut player = LuaImpl::new("return { on_tick = function(n) return { { tag = \"move\", distance = 13.12, direction = \"left\" } } end }")
                .expect("lua player could not be created");
            let res: Commands = player.on_event(&Event::Tick(17)).unwrap();
            let cmd = res.value.first().expect("some command");
            assert_eq!(*cmd, Command::Move(MovementDirection::Left, 13.12));
        }

        #[test]
        fn call_on_tick_if_missing() {
            let mut player = LuaImpl::new("return {}").unwrap();
            let res: Commands = player.on_event(&Event::Tick(17)).unwrap();
            assert_eq!(res.value.len(), 0);
        }
    }
}