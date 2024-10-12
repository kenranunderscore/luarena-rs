use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, RwLock, RwLockReadGuard};

use mlua::prelude::*;
use rand::Rng;

use crate::math_utils::{self, Point, Sector, HALF_PI};
use crate::settings::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MovementDirection {
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
pub enum PlayerCommand {
    Move(MovementDirection, f32),
    Attack,
    Turn(f32),
    TurnHead(f32),
    TurnArms(f32),
}

impl PlayerCommand {
    fn index(&self) -> i32 {
        match self {
            PlayerCommand::Move(_, _) => 0,
            PlayerCommand::Attack => 1,
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
                "attack" => Ok(PlayerCommand::Attack),
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

pub struct PlayerCommands {
    value: Vec<PlayerCommand>,
}

impl PlayerCommands {
    fn none() -> Self {
        Self { value: vec![] }
    }
}

impl From<Vec<PlayerCommand>> for PlayerCommands {
    fn from(value: Vec<PlayerCommand>) -> Self {
        Self { value }
    }
}

impl<'a> FromLua<'a> for PlayerCommands {
    fn from_lua(value: LuaValue<'a>, lua: &'a Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Nil => Ok(PlayerCommands::none()),
            _ => Ok(PlayerCommands::from(Vec::<PlayerCommand>::from_lua(
                value, lua,
            )?)),
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
            PlayerCommand::Attack => {
                let t = create_tagged_table(&lua, "attack")?;
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

#[derive(Clone)]
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
    pub _version: String,
    entrypoint: String,
}

impl PlayerMeta {
    pub fn from_lua(player_dir: &Path) -> LuaResult<PlayerMeta> {
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
        Ok(PlayerMeta {
            name,
            color,
            _version: version,
            entrypoint,
        })
    }
}

pub struct LuaImpl {
    lua: Lua,
    key: LuaRegistryKey,
    intent: ReadableFromLua<PlayerIntent>,
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
            intent: Default::default(),
        })
    }

    fn intent(&self) -> RwLockReadGuard<'_, PlayerIntent> {
        self.intent.read().unwrap()
    }

    fn table(&self) -> LuaResult<LuaTable> {
        let t = self.lua.registry_value(&self.key)?;
        Ok(t)
    }

    fn call_event_handler<A>(&self, name: &str, args: A) -> LuaResult<PlayerCommands>
    where
        A: for<'a> IntoLuaMulti<'a>,
    {
        let t = self.table()?;
        if t.contains_key(name)? {
            t.call_function(name, args)
        } else {
            Ok(PlayerCommands::none())
        }
    }

    pub fn on_event(&self, event: &PlayerEvent) -> LuaResult<PlayerCommands> {
        match event {
            PlayerEvent::Tick(n) => self.call_event_handler("on_tick", *n),
            PlayerEvent::RoundStarted(n) => self.call_event_handler("on_round_started", *n),
            PlayerEvent::EnemySeen(name, pos) => {
                self.call_event_handler("on_enemy_seen", (name.to_string(), pos.x, pos.y))
            }
            PlayerEvent::HitBy(id) => self.call_event_handler("on_hit_by", *id),
            PlayerEvent::AttackHit(id, pos) => {
                self.call_event_handler("on_attack_hit", (*id, pos.x, pos.y))
            }
            PlayerEvent::Death => self.call_event_handler("on_death", ()),
        }
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

type ReadableFromLua<T> = Arc<RwLock<T>>;

pub struct Player {
    pub id: u8,
    pub hp: ReadableFromLua<f32>,
    pub meta: PlayerMeta,
    pub pos: ReadableFromLua<Point>,
    pub heading: ReadableFromLua<f32>,
    pub head_heading: ReadableFromLua<f32>,
    pub arms_heading: ReadableFromLua<f32>,
    pub attack_cooldown: ReadableFromLua<u8>,
}

impl Player {
    pub fn new(meta: PlayerMeta, id: u8) -> Self {
        Self {
            id,
            hp: Arc::new(RwLock::new(INITIAL_HP)),
            meta,
            pos: Arc::new(RwLock::new(Point::zero())),
            heading: Arc::new(RwLock::new(0.0)),
            head_heading: Arc::new(RwLock::new(0.0)),
            arms_heading: Arc::new(RwLock::new(0.0)),
            attack_cooldown: Arc::new(RwLock::new(0)),
        }
    }

    // TODO: also randomize headings?
    pub fn reset(&mut self, next_pos: Point) {
        *self.hp.write().unwrap() = INITIAL_HP;
        self.set_heading(0.0);
        self.set_head_heading(0.0);
        self.set_arms_heading(0.0);
        let mut pos = self.pos.write().unwrap();
        pos.set_to(&next_pos);
    }

    // This looks like Java, and I feel like there has to be a better way, but
    // in this case I'm fine with hiding the `RwLock` usage where possible. It
    // might even come in handy if I find a better way to model and share the
    // state with Lua.

    pub fn heading(&self) -> f32 {
        *self.heading.read().unwrap()
    }

    pub fn set_heading(&mut self, heading: f32) {
        *self.heading.write().unwrap() = heading;
    }

    pub fn head_heading(&self) -> f32 {
        *self.head_heading.read().unwrap()
    }

    pub fn set_head_heading(&mut self, heading: f32) {
        *self.head_heading.write().unwrap() = heading;
    }

    pub fn arms_heading(&self) -> f32 {
        *self.arms_heading.read().unwrap()
    }

    pub fn set_arms_heading(&mut self, heading: f32) {
        *self.arms_heading.write().unwrap() = heading;
    }

    pub fn hp(&self) -> f32 {
        *self.hp.read().unwrap()
    }

    pub fn pos(&self) -> RwLockReadGuard<'_, Point> {
        self.pos.read().unwrap()
    }

    pub fn attack_cooldown(&self) -> u8 {
        *self.attack_cooldown.read().unwrap()
    }

    pub fn set_attack_cooldown(&mut self, cd: u8) {
        *self.attack_cooldown.write().unwrap() = cd;
    }

    pub fn effective_head_heading(&self) -> f32 {
        math_utils::normalize_absolute_angle(self.heading() + self.head_heading())
    }

    pub fn effective_arms_heading(&self) -> f32 {
        math_utils::normalize_absolute_angle(self.heading() + self.arms_heading())
    }

    pub fn alive(&self) -> bool {
        self.hp() > 0.0
    }
}

fn register_player_state_accessors(
    player: &Player,
    t: &mut LuaTable,
    lua_impl: &LuaImpl,
) -> LuaResult<()> {
    let lua = &lua_impl.lua;

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

    let r = Arc::clone(&lua_impl.intent);
    t.set(
        "turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_angle))?,
    )?;

    let r = Arc::clone(&lua_impl.intent);
    t.set(
        "head_turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_head_angle))?,
    )?;

    let r = Arc::clone(&lua_impl.intent);
    t.set(
        "arms_turn_remaining",
        lua.create_function(move |_, _: ()| Ok(r.read().unwrap().turn_arms_angle))?,
    )?;

    Ok(())
}

fn register_player_commands(t: &mut LuaTable, lua: &Lua) -> LuaResult<()> {
    let move_ = lua.create_function(|_, dist: f32| {
        Ok(PlayerCommand::Move(MovementDirection::Forward, dist))
    })?;
    t.set("move", move_)?;

    let move_backward = lua.create_function(|_, dist: f32| {
        Ok(PlayerCommand::Move(MovementDirection::Backward, dist))
    })?;
    t.set("move_backward", move_backward)?;

    let move_left =
        lua.create_function(|_, dist: f32| Ok(PlayerCommand::Move(MovementDirection::Left, dist)))?;
    t.set("move_left", move_left)?;

    let move_right = lua
        .create_function(|_, dist: f32| Ok(PlayerCommand::Move(MovementDirection::Right, dist)))?;
    t.set("move_right", move_right)?;

    let attack = lua.create_function(|_, _: ()| Ok(PlayerCommand::Attack))?;
    t.set("attack", attack)?;

    let turn = lua.create_function(|_, angle: f32| Ok(PlayerCommand::Turn(angle)))?;
    t.set("turn", &turn)?;

    let turn_head = lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnHead(angle)))?;
    t.set("turn_head", turn_head)?;

    let turn_arms = lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnArms(angle)))?;
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

fn register_lua_library(player: &Player, lua_impl: &LuaImpl) -> LuaResult<()> {
    let lua = &lua_impl.lua;
    let mut me = lua.create_table()?;
    register_player_state_accessors(player, &mut me, &lua_impl)?;
    register_player_commands(&mut me, lua)?;
    lua.globals().set("me", me)?;
    register_utils(lua)?;
    Ok(())
}

#[derive(Clone)]
pub struct Attack {
    pub id: usize,
    pub pos: Point,
    pub owner: u8,
    pub heading: f32,
    pub velocity: f32,
}

pub struct Ids {
    next: usize,
}

impl Ids {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn next(&mut self) -> usize {
        let next = self.next;
        self.next += 1;
        next
    }
}

pub enum RoundState {
    Ongoing,
    Won(u8),
    Draw,
}

pub struct Game {
    pub tick: u32,
    pub round: u32,
    pub players: Vec<Player>,
    pub lua_impls: Vec<LuaImpl>,
    pub attacks: Vec<Attack>,
    pub round_state: RoundState,
    attack_ids: Ids,
}

impl Game {
    pub fn new() -> Game {
        Self {
            tick: 0,
            round: 1,
            players: vec![],
            lua_impls: vec![],
            attacks: vec![],
            attack_ids: Ids::new(),
            round_state: RoundState::Ongoing,
        }
    }

    pub fn add_lua_player(&mut self, player_dir: &Path) -> LuaResult<()> {
        let meta = PlayerMeta::from_lua(player_dir)?;
        let lua_impl = load_lua_player(player_dir, &meta)?;
        let id = self.players.len() as u8; // FIXME
        let player = Player::new(meta, id);
        register_lua_library(&player, &lua_impl)?;
        self.players.push(player);
        self.lua_impls.push(lua_impl);
        Ok(())
    }

    pub fn init_round(&mut self, round: u32) {
        let mut rng = rand::thread_rng();
        self.tick = 0;
        self.round = round;
        self.round_state = RoundState::Ongoing;
        self.attacks = vec![];

        let min = PLAYER_RADIUS + 5.0;
        let max_x = WIDTH as f32 - PLAYER_RADIUS - 5.0;
        let max_y = HEIGHT as f32 - PLAYER_RADIUS - 5.0;
        for player in self.players.iter_mut() {
            // FIXME: don't create collisions
            let random_pos = Point {
                x: rng.gen_range(min..max_x) as f32,
                y: rng.gen_range(min..max_y) as f32,
            };
            player.reset(random_pos);
        }
        for lua_impl in self.lua_impls.iter_mut() {
            *lua_impl.intent.write().unwrap() = Default::default();
        }
    }

    pub fn living_players(&self) -> impl Iterator<Item = &Player> {
        self.players.iter().filter(|player| player.alive())
    }

    pub fn player(&mut self, id: u8) -> &mut Player {
        self.players
            .iter_mut()
            .find(|player| player.id == id)
            .expect("player {id} not found")
    }

    pub fn attack(&mut self, id: usize) -> &mut Attack {
        self.attacks
            .iter_mut()
            .find(|attack| attack.id == id)
            .expect("attack {id} not found")
    }

    pub fn lua_impl(&mut self, id: u8) -> &mut LuaImpl {
        // FIXME: player id should be index
        &mut self.lua_impls[id as usize]
    }
}

pub fn load_lua_player(player_dir: &Path, meta: &PlayerMeta) -> LuaResult<LuaImpl> {
    let file = player_dir.join(&meta.entrypoint);
    let code = std::fs::read_to_string(file)?;
    LuaImpl::new(&code)
}

fn reduce_commands(commands: &mut Vec<PlayerCommand>) {
    // FIXME: check whether they're really reduced
    commands.sort_by_key(|cmd| cmd.index());
}

fn valid_position(p: &Point) -> bool {
    p.x >= PLAYER_RADIUS
        && p.x <= WIDTH as f32 - PLAYER_RADIUS
        && p.y >= PLAYER_RADIUS
        && p.y <= HEIGHT as f32 - PLAYER_RADIUS
}

#[derive(Clone)]
pub struct Delta {
    value: Point,
}

impl Delta {
    pub fn new(value: Point) -> Self {
        Self { value }
    }
}

pub enum GameEvent {
    Tick(u32),
    RoundStarted(u32),
    RoundOver(Option<u8>),
    PlayerHeadTurned(u8, f32),
    PlayerArmsTurned(u8, f32),
    Hit(usize, u8, u8, Point),
    AttackAdvanced(usize, Point),
    AttackMissed(usize),
    AttackCreated(u8, Attack),
    PlayerPositionUpdated(u8, Delta),
    PlayerTurned(u8, f32),
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn transition_players(game: &mut Game, event_manager: &mut EventManager) {
    // TODO: is a HashMap appropriate here? is there a smarter way?
    let mut next_positions: HashMap<u8, (Delta, Point)> = HashMap::new();
    for (index, player) in game.players.iter().enumerate() {
        let lua_impl = &game.lua_impls[index];
        let delta = math_utils::clamp(lua_impl.intent().turn_angle, -MAX_TURN_RATE, MAX_TURN_RATE);
        event_manager.record(GameEvent::PlayerTurned(player.id, delta));
        let heading = math_utils::normalize_absolute_angle(*player.heading.read().unwrap() + delta);
        let velocity = f32::min(lua_impl.intent().distance, MAX_VELOCITY);
        let dir_heading = match lua_impl.intent().direction {
            MovementDirection::Forward => 0.0,
            MovementDirection::Backward => math_utils::PI,
            MovementDirection::Left => -HALF_PI,
            MovementDirection::Right => HALF_PI,
        };
        let movement_heading = heading + dir_heading;
        let dx = movement_heading.sin() * velocity;
        let dy = -movement_heading.cos() * velocity;
        let delta = Delta::new(Point { x: dx, y: dy });
        let pos = player.pos();
        let next_pos = pos.add(&delta.value);
        if valid_position(&next_pos) {
            next_positions.insert(player.id, (delta, next_pos));
        } else {
            next_positions.insert(player.id, (Delta::new(Point::zero()), pos.clone()));
        };

        transition_heads(player, lua_impl, event_manager);
        transition_arms(player, lua_impl, event_manager);
    }

    for player in game.players.iter() {
        let (delta, next) = next_positions.get(&player.id).unwrap();
        let mut collides = false;
        for (other_id, (_, other_next)) in next_positions.iter() {
            if player.id != *other_id {
                if players_collide(&next, &other_next) {
                    // TODO: collision event
                    collides = true;
                }
            }
        }
        let event = if !collides {
            GameEvent::PlayerPositionUpdated(player.id, delta.clone())
        } else {
            GameEvent::PlayerPositionUpdated(player.id, Delta::new(Point::zero()))
        };
        event_manager.record(event);
    }
}

fn transition_heads(player: &Player, lua_impl: &LuaImpl, event_manager: &mut EventManager) {
    let delta = math_utils::clamp(
        lua_impl.intent().turn_head_angle,
        -MAX_HEAD_TURN_RATE,
        MAX_HEAD_TURN_RATE,
    );
    let current_heading = player.head_heading();
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::PlayerHeadTurned(player.id, effective_delta));
}

fn transition_arms(player: &Player, lua_impl: &LuaImpl, event_manager: &mut EventManager) {
    let delta = math_utils::clamp(
        lua_impl.intent().turn_arms_angle,
        -MAX_ARMS_TURN_RATE,
        MAX_ARMS_TURN_RATE,
    );
    let current_heading = player.arms_heading();
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::PlayerArmsTurned(player.id, effective_delta));
}

fn players_collide(p: &Point, q: &Point) -> bool {
    p.dist(q) <= 2.0 * (PLAYER_RADIUS as f32)
}

#[derive(Debug)]
pub enum PlayerEvent {
    Tick(u32),
    RoundStarted(u32),
    EnemySeen(String, Point),
    Death,
    HitBy(u8),
    AttackHit(u8, Point),
}

fn game_events_to_player_events(player: &Player, game_events: &[GameEvent]) -> Vec<PlayerEvent> {
    let mut player_events = Vec::new();
    for event in game_events.iter() {
        match event {
            GameEvent::Tick(n) => {
                player_events.push(PlayerEvent::Tick(*n));
            }
            GameEvent::RoundStarted(n) => {
                player_events.push(PlayerEvent::RoundStarted(*n));
            }
            GameEvent::RoundOver(_) => {}
            GameEvent::PlayerTurned(_, _) => {}
            GameEvent::PlayerPositionUpdated(_, _) => {}
            GameEvent::PlayerHeadTurned(_, _) => {}
            GameEvent::PlayerArmsTurned(_, _) => {}
            GameEvent::Hit(_, owner_id, victim_id, pos) => {
                if player.id == *victim_id {
                    // FIXME: don't use id
                    player_events.push(PlayerEvent::HitBy(*owner_id));
                    if !player.alive() {
                        player_events.push(PlayerEvent::Death);
                    }
                } else if player.id == *owner_id {
                    player_events.push(PlayerEvent::AttackHit(*victim_id, pos.clone()));
                }
            }
            GameEvent::AttackAdvanced(_, _) => {}
            GameEvent::AttackMissed(_) => {}
            GameEvent::AttackCreated(_, _) => {}
        }
    }
    player_events
}

fn can_spot(
    origin: &Point,
    view_angle: f32,
    target: &Point,
    player_radius: f32,
    angle_of_vision: f32,
) -> bool {
    let view_sector = Sector::new(view_angle, angle_of_vision / 2.0);
    let d = origin.dist(target);
    let alpha = f32::atan(player_radius / d);
    let angle = math_utils::normalize_absolute_angle(math_utils::angle_between(origin, target));
    let target_sector = Sector::new(angle, alpha);
    view_sector.overlaps(&target_sector)
}

fn dispatch_player_events(
    player_events: Vec<PlayerEvent>,
    lua_player: &LuaImpl,
) -> LuaResult<Vec<PlayerCommand>> {
    let mut commands = Vec::new();
    for e in player_events.iter() {
        commands.append(&mut lua_player.on_event(&e)?.value);
    }
    Ok(commands)
}

fn create_attacks(game: &mut Game, event_manager: &mut EventManager) {
    for (index, player) in game.players.iter_mut().enumerate() {
        let lua_impl = &mut game.lua_impls[index];
        let will_attack = lua_impl.intent().attack && player.attack_cooldown() == 0;
        if will_attack {
            let attack = Attack {
                id: game.attack_ids.next(),
                owner: player.id,
                pos: player.pos().clone(),
                velocity: 2.5,
                heading: player.effective_arms_heading(),
            };
            event_manager.record(GameEvent::AttackCreated(player.id, attack));
        }
    }
}

fn inside_arena(p: &Point) -> bool {
    p.x >= 0.0 && p.x <= WIDTH as f32 && p.y >= 0.0 && p.y <= HEIGHT as f32
}

fn attack_hits_player<'a>(
    attack: &Attack,
    mut players: impl Iterator<Item = &'a Player>,
) -> Option<&'a Player> {
    players.find(|player| {
        player.id != attack.owner
            && attack.pos.dist(&player.pos()) <= ATTACK_RADIUS + PLAYER_RADIUS as f32
    })
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

    pub fn init_tick(&mut self, tick: u32, round: u32) {
        self.current_events = tick_events(tick, round);
    }

    pub fn record(&mut self, event: GameEvent) {
        self.current_events.push(event);
    }

    pub fn current_events(&self) -> &[GameEvent] {
        &self.current_events
    }
}

fn tick_events(tick: u32, round: u32) -> Vec<GameEvent> {
    let mut events = vec![GameEvent::Tick(tick)];
    if tick == 0 {
        events.push(GameEvent::RoundStarted(round));
    }
    events
}

fn transition_attacks(game: &Game, event_manager: &mut EventManager) {
    for attack in game.attacks.iter() {
        let next_pos = math_utils::line_endpoint(
            attack.pos.x as f32,
            attack.pos.y as f32,
            attack.velocity,
            attack.heading,
        );
        if inside_arena(&next_pos) {
            if let Some(player) = attack_hits_player(&attack, game.living_players()) {
                // FIXME: new_pos or old position here?
                event_manager.record(GameEvent::Hit(attack.id, attack.owner, player.id, next_pos));
            } else {
                event_manager.record(GameEvent::AttackAdvanced(attack.id, next_pos));
            }
        } else {
            event_manager.record(GameEvent::AttackMissed(attack.id));
        }
    }
}

fn advance_game_state(game: &mut Game, game_events: &[GameEvent]) {
    for event in game_events.iter() {
        match event {
            GameEvent::Tick(_) => {
                // FIXME: check whether saving the next tick shooting is
                // possible again might be better; but then again we could not
                // as easily add a Lua getter...
                for player in game.players.iter_mut() {
                    let cd = player.attack_cooldown();
                    if cd > 0 {
                        player.set_attack_cooldown(cd - 1);
                    }
                }
            }
            GameEvent::RoundStarted(_) => {}
            GameEvent::RoundOver(winner) => {
                game.round_state = match winner {
                    Some(winner) => RoundState::Won(*winner),
                    None => RoundState::Draw,
                }
            }
            GameEvent::PlayerPositionUpdated(id, delta) => {
                let d;
                {
                    let player = game.player(*id);
                    let mut pos = player.pos.write().unwrap();
                    d = pos.dist(&Point::zero()); // TODO: length of a Vec2
                    pos.x += delta.value.x;
                    pos.y += delta.value.y;
                }
                let lua_impl = game.lua_impl(*id);
                let distance = lua_impl.intent().distance;
                lua_impl.intent.write().unwrap().distance = f32::max(distance - d, 0.0);
            }
            GameEvent::PlayerTurned(id, delta) => {
                let player = game.player(*id);
                let heading = player.heading() + *delta;
                player.set_heading(math_utils::normalize_absolute_angle(heading));
                let lua_impl = game.lua_impl(*id);
                let turn_angle = lua_impl.intent().turn_angle;
                lua_impl.intent.write().unwrap().turn_angle = if turn_angle.abs() < MAX_TURN_RATE {
                    0.0
                } else {
                    turn_angle - *delta
                };
            }
            GameEvent::PlayerHeadTurned(id, delta) => {
                let player = game.player(*id);
                let heading = player.head_heading() + *delta;
                player.set_head_heading(heading);
                let lua_impl = game.lua_impl(*id);
                let intended = lua_impl.intent().turn_head_angle;
                lua_impl.intent.write().unwrap().turn_head_angle =
                    // FIXME: (here and elsewhere): we don't need this if we
                    // stop using float equality checks I think
                    if intended.abs() < MAX_HEAD_TURN_RATE {
                        0.0
                    } else {
                        intended - *delta
                    };
            }
            GameEvent::PlayerArmsTurned(id, delta) => {
                let player = game.player(*id);
                let heading = clamp_turn_angle(player.arms_heading() + *delta);
                player.set_arms_heading(heading);
                let lua_impl = game.lua_impl(*id);
                let intended = lua_impl.intent().turn_arms_angle;
                lua_impl.intent.write().unwrap().turn_arms_angle =
                    if intended.abs() < MAX_ARMS_TURN_RATE {
                        0.0
                    } else {
                        intended - *delta
                    };
            }
            GameEvent::Hit(attack_id, _, victim_id, _) => {
                if let Some(index) = game
                    .attacks
                    .iter()
                    .position(|attack| attack.id == *attack_id)
                {
                    game.attacks.remove(index);
                }
                *game.player(*victim_id).hp.write().unwrap() -= ATTACK_DAMAGE;
            }
            GameEvent::AttackAdvanced(id, pos) => {
                let attack = game.attack(*id);
                attack.pos.set_to(&pos);
            }
            GameEvent::AttackMissed(id) => {
                if let Some(index) = game.attacks.iter().position(|attack| attack.id == *id) {
                    game.attacks.remove(index);
                }
            }
            GameEvent::AttackCreated(owner, attack) => {
                game.attacks.push(attack.clone());
                let player = game.player(*owner);
                player.set_attack_cooldown(ATTACK_COOLDOWN);
                let lua_impl = game.lua_impl(*owner);
                lua_impl.intent.write().unwrap().attack = false;
            }
        }
    }
}

pub struct PlayerData {
    pub color: crate::game::Color,
    pub x: i32,
    pub y: i32,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
}

pub struct GameData {
    pub players: Vec<PlayerData>,
    pub attacks: Vec<Point>,
}

impl GameData {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            attacks: Vec::new(),
        }
    }
}

fn check_for_round_end(game: &Game, event_manager: &mut EventManager) {
    let nplayers = game.living_players().count();
    match nplayers {
        0 => event_manager.record(GameEvent::RoundOver(None)),
        1 => event_manager.record(GameEvent::RoundOver(Some(
            game.living_players().nth(0).unwrap().id,
        ))),
        _ => {}
    }
}

fn write_game_data(game: &Game, game_writer: &mpsc::Sender<GameData>) {
    let mut game_data = GameData::new();
    for player in game.living_players() {
        let p = player.pos();
        game_data.players.push(PlayerData {
            color: player.meta.color.clone(),
            x: p.x.round() as i32,
            y: p.y.round() as i32,
            heading: *player.heading.read().unwrap(),
            head_heading: player.effective_head_heading(),
            arms_heading: player.effective_arms_heading(),
        });
    }
    for attack in game.attacks.iter() {
        game_data.attacks.push(attack.pos.clone());
    }
    game_writer.send(game_data).unwrap();
}

fn run_lua_players(game: &mut Game, events: &[GameEvent]) -> LuaResult<()> {
    // FIXME: learn how to do this in a better way!
    let player_positions: Vec<(u8, String, Point)> = game
        .players
        .iter()
        .map(|player| (player.id, player.meta.name.clone(), player.pos().clone()))
        .collect();
    // FIXME: only living players?
    for (index, player) in game.players.iter_mut().enumerate() {
        let mut player_events = game_events_to_player_events(player, events);
        for (id, name, pos) in player_positions.iter() {
            if *id != player.id {
                if can_spot(
                    &player.pos(),
                    player.effective_head_heading(),
                    &pos,
                    PLAYER_RADIUS as f32,
                    ANGLE_OF_VISION,
                ) {
                    player_events.push(PlayerEvent::EnemySeen(name.clone(), pos.clone()));
                }
            }
        }
        let lua_impl = &mut game.lua_impls[index];
        let mut commands = dispatch_player_events(player_events, lua_impl)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                PlayerCommand::Attack => lua_impl.intent.write().unwrap().attack = true,
                PlayerCommand::Turn(angle) => lua_impl.intent.write().unwrap().turn_angle = *angle,
                PlayerCommand::TurnHead(angle) => {
                    let current = player.head_heading();
                    let next = clamp_turn_angle(current + *angle) - current;
                    lua_impl.intent.write().unwrap().turn_head_angle = next;
                }
                PlayerCommand::TurnArms(angle) => {
                    let current = player.arms_heading();
                    let next = clamp_turn_angle(current + *angle) - current;
                    lua_impl.intent.write().unwrap().turn_arms_angle = next;
                }
                PlayerCommand::Move(dir, dist) => {
                    lua_impl.intent.write().unwrap().direction = dir.clone();
                    lua_impl.intent.write().unwrap().distance = *dist;
                }
            }
        }
    }
    Ok(())
}

pub fn step(
    game: &mut Game,
    event_manager: &mut EventManager,
    game_writer: &mpsc::Sender<GameData>,
) -> LuaResult<()> {
    event_manager.init_tick(game.tick, game.round);
    check_for_round_end(game, event_manager);
    transition_players(game, event_manager);
    create_attacks(game, event_manager);
    transition_attacks(game, event_manager);

    let game_events: &[GameEvent] = event_manager.current_events();
    advance_game_state(game, game_events);
    run_lua_players(game, game_events)?;

    write_game_data(&game, game_writer);

    game.tick += 1;
    Ok(())
}

pub fn run_round(
    game: &mut Game,
    event_manager: &mut EventManager,
    delay: &std::time::Duration,
    game_writer: &mpsc::Sender<GameData>,
    cancel: &Arc<AtomicBool>,
) -> LuaResult<()> {
    loop {
        if cancel.load(Ordering::Relaxed) {
            println!("Game cancelled");
            break;
        }

        std::thread::sleep(*delay);
        step(game, event_manager, game_writer)?;
        match game.round_state {
            RoundState::Ongoing => {}
            RoundState::Won(id) => {
                println!("Player {id} has won!");
                break;
            }
            RoundState::Draw => {
                println!("--- DRAW ---");
                break;
            }
        }
    }
    Ok(())
}

pub fn run_game(
    game: &mut Game,
    delay: &std::time::Duration,
    game_writer: &mpsc::Sender<GameData>,
    cancel: &Arc<AtomicBool>,
) -> LuaResult<()> {
    let mut event_manager = EventManager::new();
    let max_rounds = 2;
    for round in 1..max_rounds + 1 {
        game.init_round(round);
        run_round(game, &mut event_manager, delay, game_writer, cancel)?;
    }
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
            let player = LuaImpl::new("return { on_tick = function(n) return { { tag = \"move\", distance = 13.12, direction = \"left\" } } end }")
                .expect("lua player could not be created");
            let res: PlayerCommands = player.on_event(&PlayerEvent::Tick(17)).unwrap();
            let cmd = res.value.first().expect("some command");
            assert_eq!(*cmd, PlayerCommand::Move(MovementDirection::Left, 13.12));
        }

        #[test]
        fn call_on_tick_if_missing() {
            let player = LuaImpl::new("return {}").unwrap();
            let res: PlayerCommands = player.on_event(&PlayerEvent::Tick(17)).unwrap();
            assert_eq!(res.value.len(), 0);
        }
    }
}
