use std::path::Path;

use mlua::prelude::*;

use super::{meta, *};
use crate::color::Color;
use crate::math_utils::{self, Point};

impl<'a> IntoLua<'a> for Point {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        let t = lua.create_table()?;
        t.set("x", self.x)?;
        t.set("y", self.y)?;
        Ok(LuaValue::Table(t))
    }
}

impl<'a> IntoLua<'a> for Id {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        self.0.to_string().into_lua(lua)
    }
}

impl<'a> IntoLua<'a> for &CurrentCharacterState {
    fn into_lua(self, lua: &'a Lua) -> LuaResult<LuaValue<'a>> {
        let t = lua.create_table()?;
        t.set("x", self.x)?;
        t.set("y", self.y)?;
        t.set("hp", self.hp)?;
        t.set("heading", self.heading)?;
        t.set("head_heading", self.head_heading)?;
        t.set("arms_heading", self.arms_heading)?;
        t.set("attack_cooldown", self.attack_cooldown)?;
        t.set("turn_remaining", self.turn_remaining)?;
        t.set("head_turn_remaining", self.head_turn_remaining)?;
        t.set("arms_turn_remaining", self.arms_turn_remaining)?;
        Ok(LuaValue::Table(t))
    }
}

impl<'a> FromLua<'a> for Id {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::String(ref s) => {
                let s = s.to_str()?;
                let uuid =
                    uuid::Uuid::parse_str(s).map_err(|_| mlua::Error::FromLuaConversionError {
                        from: value.type_name(),
                        to: "Id",
                        message: Some(format!("expected valid UUID string, got {s}")),
                    })?;
                Ok(Id(uuid))
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Id",
                message: Some("expected UUID".to_string()),
            }),
        }
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
                // FIXME: implement and test this error case
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
                to: "CharacterCommand",
                message: Some("expected valid character command".to_string()),
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

    fn table(&self) -> LuaResult<LuaTable<'_>> {
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

    fn register_lua_library(&self, meta: &meta::Meta) -> LuaResult<()> {
        let lua = &self.lua;
        let name = meta.display_name();
        let mut me = lua.create_table()?;
        me.set(
            "log",
            lua.create_function(move |_, msg: LuaString| {
                let msg = msg.to_str()?;
                log_msg(&name, msg);
                Ok(())
            })?,
        )?;
        register_commands(&mut me, lua)?;
        lua.globals().set("me", me)?;
        register_utils(lua)?;
        Ok(())
    }

    pub fn load(character_dir: &Path, meta: &meta::Meta) -> LuaResult<Self> {
        let file = character_dir.join(&meta.entrypoint);
        let code = std::fs::read_to_string(file)?;
        let res = Self::new(&code)?;
        res.register_lua_library(meta)?;
        Ok(res)
    }
}

impl Impl for LuaImpl {
    fn on_event(&mut self, event: &Event) -> Result<Commands, EventError> {
        match event {
            Event::Tick(n, state) => self.call_event_handler("on_tick", (*n, state)),
            Event::RoundStarted(n) => self.call_event_handler("on_round_started", *n),
            Event::RoundEnded(opt_winner) => self.call_event_handler(
                "on_round_started",
                opt_winner.as_ref().map(|meta| meta.name.clone()),
            ),
            Event::EnemySeen(name, pos) => {
                self.call_event_handler("on_enemy_seen", (name.to_string(), pos.clone()))
            }
            Event::HitBy(meta) => self.call_event_handler("on_hit_by", meta.name.clone()),
            Event::AttackHit(meta, pos) => {
                self.call_event_handler("on_attack_hit", (meta.name.clone(), pos.clone()))
            }
            Event::Death => self.call_event_handler("on_death", ()),
            Event::EnemyDied(deceased_meta) => {
                self.call_event_handler("on_enemy_death", deceased_meta.to_string())
            }
            Event::RoundDrawn => self.call_event_handler("on_round_drawn", ()),
            Event::RoundWon => self.call_event_handler("on_round_won", ()),
        }
    }
}

impl From<mlua::Error> for EventError {
    fn from(err: mlua::Error) -> Self {
        Self {
            message: format!("{err}"),
        }
    }
}

fn register_commands(t: &mut LuaTable, lua: &Lua) -> LuaResult<()> {
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

    mod lua_character {
        use super::*;

        #[test]
        fn lua_character_can_be_loaded_from_code() {
            LuaImpl::new("return {}").expect("lua character could not be created");
        }

        #[test]
        fn call_on_tick() {
            let mut character = LuaImpl::new("return { on_round_started = function(n) return { { tag = \"move\", distance = 13.12, direction = \"left\" } } end }")
                .expect("lua character could not be created");
            let res: Commands = character.on_event(&Event::RoundStarted(17)).unwrap();
            let cmd = res.value.first().expect("some command");
            assert_eq!(*cmd, Command::Move(MovementDirection::Left, 13.12));
        }

        #[test]
        fn call_on_tick_if_missing() {
            let mut character = LuaImpl::new("return {}").unwrap();
            let res: Commands = character.on_event(&Event::RoundStarted(17)).unwrap();
            assert_eq!(res.value.len(), 0);
        }
    }
}
