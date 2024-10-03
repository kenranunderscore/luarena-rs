use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;

use crate::math_utils;
use crate::math_utils::HALF_PI;
use crate::settings::*;

#[derive(Debug, PartialEq, Eq, Clone)]
enum MovementDirection {
    Forward,
    Backward,
    Left,
    Right,
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

#[derive(PartialEq, Debug)]
enum PlayerCommand {
    Attack(f32),
    Turn(f32),
    TurnHead(f32),
    TurnArms(f32),
    Move(MovementDirection, f32),
}

impl PlayerCommand {
    fn index(&self) -> i32 {
        match self {
            PlayerCommand::Move(_, _) => 0,
            PlayerCommand::Attack(_) => 1,
            PlayerCommand::Turn(_) => 2,
            PlayerCommand::TurnHead(_) => 3,
            PlayerCommand::TurnArms(_) => 4,
        }
    }
}

impl<'a> FromLua<'a> for PlayerCommand {
    fn from_lua(value: LuaValue<'a>, _lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Table(t) => match t.get::<&str, String>("tag")?.as_str() {
                "move" => {
                    let dist = t.get("distance")?;
                    let dir: MovementDirection = t.get("direction")?;
                    Ok(PlayerCommand::Move(dir, dist))
                }
                "attack" => Ok(PlayerCommand::Attack(t.get("angle")?)),
                "turn" => Ok(PlayerCommand::Turn(t.get("angle")?)),
                "turn_head" => Ok(PlayerCommand::TurnHead(t.get("angle")?)),
                "turn_arms" => Ok(PlayerCommand::TurnArms(t.get("angle")?)),
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
            PlayerCommand::Turn(angle) => {
                let t = create_tagged_table(&lua, "turn")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            PlayerCommand::TurnHead(angle) => {
                let t = create_tagged_table(&lua, "turn_head")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            PlayerCommand::TurnArms(angle) => {
                let t = create_tagged_table(&lua, "turn_arms")?;
                t.set("angle", angle)?;
                Ok(LuaValue::Table(t))
            }
            PlayerCommand::Move(dir, dist) => {
                let t = create_tagged_table(&lua, "move")?;
                t.set("distance", dist)?;
                t.set("direction", dir)?;
                Ok(LuaValue::Table(t))
            }
        }
    }
}

struct LuaPlayer {
    lua: Lua,
    key: LuaRegistryKey,
}

pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
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

pub struct PlayerMeta {
    pub name: String,
    pub color: Color,
    pub version: String,
    entrypoint: String,
}

impl LuaPlayer {
    pub fn read_meta(player_dir: &str) -> LuaResult<PlayerMeta> {
        let lua = Lua::new();
        // FIXME: use PathBuf or similar
        let meta_file = format!("{player_dir}/meta.lua");
        let code = std::fs::read_to_string(meta_file)?;
        lua.load(&code).exec()?;
        let name = lua.globals().get("name")?;
        let color = lua.globals().get("color")?;
        let version = lua.globals().get("version")?;
        let entrypoint = match lua.globals().get("entrypoint") {
            Ok(file_name) => file_name,
            Err(_) => String::from("main.lua"),
        };
        Ok(PlayerMeta {
            name,
            color,
            version,
            entrypoint,
        })
    }

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

    fn on_tick(&self, tick: i32) -> LuaResult<Vec<PlayerCommand>> {
        let res: Vec<PlayerCommand> = self.table()?.call_function("on_tick", tick)?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn dist_sqr(&self, p: &Point) -> i32 {
        (self.x - p.x).pow(2) + (self.y - p.y).pow(2)
    }

    pub fn dist(&self, p: &Point) -> f32 {
        let d = self.dist_sqr(p) as f32;
        d.sqrt()
    }
}

struct PlayerIntent {
    direction: MovementDirection,
    distance: f32,
    attack: bool,
    turn_angle: f32,
    turn_head_angle: f32,
    turn_arms_angle: f32,
}

impl Default for PlayerIntent {
    fn default() -> Self {
        Self {
            direction: MovementDirection::Forward,
            distance: 0.0,
            turn_head_angle: 0.0,
            turn_arms_angle: 0.0,
            attack: false,
            turn_angle: 0.0,
        }
    }
}

#[derive(Clone)]
struct NextMove {
    pos: Point,
    distance: f32,
}

pub struct Player {
    pub id: u8,
    lua_player: LuaPlayer,
    pub meta: PlayerMeta,
    pub pos: Rc<RefCell<Point>>,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
    intent: PlayerIntent,
    next_move: NextMove,
}

impl Player {
    pub fn new(player_dir: &str, id: u8, x: i32, y: i32) -> LuaResult<Player> {
        let meta = LuaPlayer::read_meta(player_dir)?;
        let res = Self {
            id,
            lua_player: load_lua_player(player_dir, &meta)?,
            meta,
            pos: Rc::new(RefCell::new(Point { x, y })),
            heading: 0.0,
            head_heading: 0.0,
            arms_heading: 0.0,
            next_move: NextMove {
                pos: Point { x, y },
                distance: 0.0,
            },
            intent: Default::default(),
        };
        res.register_lua_library()?;
        Ok(res)
    }

    pub fn effective_head_heading(&self) -> f32 {
        math_utils::normalize_abs_angle(self.heading + self.head_heading)
    }

    pub fn effective_arms_heading(&self) -> f32 {
        math_utils::normalize_abs_angle(self.heading + self.arms_heading)
    }

    fn register_lua_library(&self) -> LuaResult<()> {
        let lua = &self.lua_player.lua;
        let me = lua.create_table()?;

        let pos_ref = Rc::clone(&self.pos);
        let x = lua.create_function(move |_, _: ()| Ok(pos_ref.borrow().x))?;
        me.set("x", x)?;

        // need to clone the ref again, as we move to make the closure work
        let pos_ref = Rc::clone(&self.pos);
        let y = lua.create_function(move |_, _: ()| Ok(pos_ref.borrow().y))?;
        me.set("y", y)?;

        let move_cmd = lua.create_function(|_, dist: f32| {
            Ok(PlayerCommand::Move(MovementDirection::Forward, dist))
        })?;
        me.set("move", move_cmd)?;

        let move_backward_cmd = lua.create_function(|_, dist: f32| {
            Ok(PlayerCommand::Move(MovementDirection::Backward, dist))
        })?;
        me.set("move_backward", move_backward_cmd)?;

        let move_left_cmd = lua.create_function(|_, dist: f32| {
            Ok(PlayerCommand::Move(MovementDirection::Left, dist))
        })?;
        me.set("move_left", move_left_cmd)?;

        let move_right_cmd = lua.create_function(|_, dist: f32| {
            Ok(PlayerCommand::Move(MovementDirection::Right, dist))
        })?;
        me.set("move_right", move_right_cmd)?;

        let attack_cmd = lua.create_function(|_, angle: f32| Ok(PlayerCommand::Attack(angle)))?;
        me.set("attack", attack_cmd)?;

        let turn_cmd = lua.create_function(|_, angle: f32| Ok(PlayerCommand::Turn(angle)))?;
        me.set("turn", &turn_cmd)?;

        let turn_head_cmd =
            lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnHead(angle)))?;
        me.set("turn_head", turn_head_cmd)?;

        let turn_arms_cmd =
            lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnArms(angle)))?;
        me.set("turn_arms", turn_arms_cmd)?;

        lua.globals().set("me", me)?;
        Ok(())
    }
}

pub struct GameState {
    pub tick: i32,
    pub round: i32,
    pub players: Vec<Player>,
}

impl GameState {
    pub fn new() -> GameState {
        Self {
            tick: 0,
            round: 1,
            players: Vec::new(),
        }
    }
}

fn load_lua_player(player_dir: &str, meta: &PlayerMeta) -> LuaResult<LuaPlayer> {
    // FIXME: use PathBuf or similar
    let file = format!("{player_dir}/{0}", meta.entrypoint);
    let code = std::fs::read_to_string(file)?;
    LuaPlayer::new(&code)
}

fn reduce_commands(commands: &mut Vec<PlayerCommand>) {
    commands.sort_by_key(|cmd| cmd.index());
}

fn valid_position(p: &Point) -> bool {
    p.x >= PLAYER_RADIUS
        && p.x <= WIDTH - PLAYER_RADIUS
        && p.y >= PLAYER_RADIUS
        && p.y <= HEIGHT - PLAYER_RADIUS
}

pub enum GameEvent {
    Tick(i32),
    RoundStarted(i32),
    PlayerMoved(u8, Point),
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn advance_players(state: &mut GameState, event_manager: &mut EventManager) {
    // FIXME: refactor: probably no need to mutate players for the next
    // positions, or even keep them in there as state at all!
    for player in state.players.iter_mut() {
        {
            let delta = math_utils::clamp(player.intent.turn_angle, -MAX_TURN_RATE, MAX_TURN_RATE);
            player.intent.turn_angle = if player.intent.turn_angle.abs() < MAX_TURN_RATE {
                0.0
            } else {
                player.intent.turn_angle - delta
            };
            let heading = math_utils::normalize_abs_angle(player.heading + delta);
            player.heading = heading;

            let velocity = f32::min(player.intent.distance, MAX_VELOCITY);
            let dir_heading = match player.intent.direction {
                MovementDirection::Forward => 0.0,
                MovementDirection::Backward => crate::PI as f32,
                MovementDirection::Left => -HALF_PI,
                MovementDirection::Right => HALF_PI,
            };
            let movement_heading = heading + dir_heading;
            let remaining_distance = f32::max(player.intent.distance - velocity, 0.0);
            let p = player.pos.borrow();
            let dx = movement_heading.sin() * velocity;
            let dy = -movement_heading.cos() * velocity;
            let next_pos = Point {
                x: p.x + dx.round() as i32,
                y: p.y + dy.round() as i32,
            };
            if valid_position(&next_pos) {
                player.next_move.pos = next_pos;
                player.next_move.distance = remaining_distance;
            } else {
                player.next_move.pos = p.clone();
                player.next_move.distance = player.intent.distance;
            }
        }

        {
            let delta = math_utils::clamp(
                player.intent.turn_head_angle,
                -MAX_HEAD_TURN_RATE,
                MAX_HEAD_TURN_RATE,
            );
            let heading = clamp_turn_angle(player.head_heading + delta);
            let remaining = clamp_turn_angle(player.head_heading + delta) - heading;
            player.head_heading = heading;
            player.intent.turn_head_angle = remaining;
        }

        {
            let delta = math_utils::clamp(
                player.intent.turn_arms_angle,
                -MAX_ARMS_TURN_RATE,
                MAX_ARMS_TURN_RATE,
            );
            let heading = clamp_turn_angle(player.arms_heading + delta);
            let remaining = clamp_turn_angle(player.arms_heading + delta) - heading;
            player.arms_heading = heading;
            player.intent.turn_arms_angle = remaining;
        }
    }

    let next_positions: Vec<(u8, NextMove)> = state
        .players
        .iter()
        .map(|player| (player.id, player.next_move.clone()))
        .collect();
    state.players.iter_mut().for_each(|player| {
        if !next_positions.iter().any(|(id, next_move)| {
            *id != player.id && players_collide(&player.pos.borrow(), &next_move.pos)
        }) {
            let mut pos = player.pos.borrow_mut();
            pos.x = player.next_move.pos.x;
            pos.y = player.next_move.pos.y;
            player.intent.distance = player.next_move.distance;
            event_manager.record_event(GameEvent::PlayerMoved(player.id, pos.clone()));
        }
    });
}

fn players_collide(p: &Point, q: &Point) -> bool {
    p.dist(q) <= 2.0 * (PLAYER_RADIUS as f32)
}

#[derive(Debug)]
enum PlayerEvent {
    Tick(i32),
    RoundStarted(i32),
}

fn game_events_to_player_events(
    _player: &Player,
    game_events: &Vec<GameEvent>,
) -> Vec<PlayerEvent> {
    // FIXME: when to generate enemy_seen events?
    game_events.iter().fold(Vec::new(), |mut acc, e| match e {
        GameEvent::Tick(n) => {
            acc.push(PlayerEvent::Tick(*n));
            acc
        }
        GameEvent::RoundStarted(n) => {
            acc.push(PlayerEvent::RoundStarted(*n));
            acc
        }
        GameEvent::PlayerMoved(_other, _pos) => acc,
    })
}

fn dispatch_player_events(
    player: &Player,
    player_events: Vec<PlayerEvent>,
) -> LuaResult<Vec<PlayerCommand>> {
    let mut commands = Vec::new();
    for e in player_events.iter() {
        match e {
            PlayerEvent::Tick(n) => commands.append(&mut player.lua_player.on_tick(*n)?),
            PlayerEvent::RoundStarted(n) => todo!("round started handler for round {n}"),
        }
    }
    Ok(commands)
}

pub fn step(state: &mut GameState, event_manager: &mut EventManager) -> LuaResult<()> {
    event_manager.next_tick(state);
    advance_players(state, event_manager);
    let game_events = &event_manager.current_events;
    for player in state.players.iter_mut() {
        // FIXME: sort events
        let player_events = game_events_to_player_events(player, game_events);
        let mut commands = dispatch_player_events(player, player_events)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                PlayerCommand::Attack(angle) => {
                    player.intent.turn_arms_angle = *angle;
                    player.intent.attack = true;
                }
                PlayerCommand::Turn(angle) => player.intent.turn_angle = *angle,
                PlayerCommand::TurnHead(angle) => player.intent.turn_head_angle = *angle,
                PlayerCommand::TurnArms(angle) => player.intent.turn_arms_angle = *angle,
                PlayerCommand::Move(dir, dist) => {
                    player.intent.direction = dir.clone();
                    player.intent.distance = *dist;
                }
            }
        }
    }
    Ok(())
}

pub struct EventManager {
    current_events: Vec<GameEvent>,
}

impl EventManager {
    pub fn new() -> EventManager {
        Self {
            current_events: vec![],
        }
    }

    // FIXME: don't pass the whole game state
    pub fn next_tick(&mut self, state: &GameState) {
        self.current_events = tick_events(state);
    }

    pub fn record_event(&mut self, event: GameEvent) {
        self.current_events.push(event);
    }
}

fn tick_events(state: &GameState) -> Vec<GameEvent> {
    let mut events = vec![GameEvent::Tick(state.tick)];
    if state.tick == 0 {
        events.push(GameEvent::RoundStarted(state.round));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_player_can_be_loaded_from_code() {
        LuaPlayer::new("return {}").expect("lua player could not be created");
    }

    #[test]
    fn call_on_tick() {
        let player = LuaPlayer::new(
            "return { on_tick = function(n) return { { tag = \"move\", distance = 13.12, direction = \"left\" } } end }",
        )
        .expect("lua player could not be created");
        let res: Vec<PlayerCommand> = player.on_tick(17).expect("on_tick failed");
        let cmd = res.first().expect("some command");
        assert_eq!(*cmd, PlayerCommand::Move(MovementDirection::Left, 13.12));
    }
}
