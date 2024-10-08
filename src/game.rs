use std::collections::HashMap;
use std::sync::{mpsc, Arc, RwLock};

use mlua::prelude::*;

use crate::math_utils::{self, Point, HALF_PI};
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
    pub version: String,
    entrypoint: String,
}

pub struct LuaImpl {
    lua: Lua,
    key: LuaRegistryKey,
    intent: PlayerIntent,
}

impl LuaImpl {
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

    fn table(&self) -> LuaResult<LuaTable> {
        let t = self.lua.registry_value(&self.key)?;
        Ok(t)
    }

    fn call_event_handler<A>(&self, name: &str, args: A) -> LuaResult<Vec<PlayerCommand>>
    where
        A: for<'a> IntoLuaMulti<'a>,
    {
        let t = self.table()?;
        if t.contains_key(name)? {
            // TODO: decide whether to allow not returning anything from
            // handlers
            let res = t.call_function(name, args)?;
            Ok(res)
        } else {
            Ok(vec![])
        }
    }

    pub fn on_event(&self, event: &PlayerEvent) -> LuaResult<Vec<PlayerCommand>> {
        match event {
            PlayerEvent::Tick(n) => self.call_event_handler("on_tick", *n),
            PlayerEvent::RoundStarted(n) => self.call_event_handler("on_round_started", *n),
            PlayerEvent::EnemySeen(name, pos) => {
                self.call_event_handler("on_enemy_seen", (name.to_string(), pos.x, pos.y))
            }
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

pub struct Player {
    pub id: u8,
    pub hp: f32,
    pub meta: PlayerMeta,
    pub pos: Arc<RwLock<Point>>,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
}

impl Player {
    pub fn new(player_dir: &str, id: u8, x: f32, y: f32) -> LuaResult<Player> {
        let meta = LuaImpl::read_meta(player_dir)?;
        let res = Self {
            id,
            hp: 100.0,
            meta,
            pos: Arc::new(RwLock::new(Point { x, y })),
            heading: 0.0,
            head_heading: 0.0,
            arms_heading: 0.0,
        };
        Ok(res)
    }

    pub fn effective_head_heading(&self) -> f32 {
        math_utils::normalize_abs_angle(self.heading + self.head_heading)
    }

    pub fn effective_arms_heading(&self) -> f32 {
        math_utils::normalize_abs_angle(self.heading + self.arms_heading)
    }

    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }
}

fn register_lua_library(player: &Player, lua_player: &LuaImpl) -> LuaResult<()> {
    let lua = &lua_player.lua;
    let me = lua.create_table()?;

    let pos_ref = Arc::clone(&player.pos);
    let x = lua.create_function(move |_, _: ()| Ok(pos_ref.read().unwrap().x))?;
    me.set("x", x)?;

    // need to clone the ref again, as we move to make the closure work
    let pos_ref = Arc::clone(&player.pos);
    let y = lua.create_function(move |_, _: ()| Ok(pos_ref.read().unwrap().y))?;
    me.set("y", y)?;

    let move_cmd = lua.create_function(|_, dist: f32| {
        Ok(PlayerCommand::Move(MovementDirection::Forward, dist))
    })?;
    me.set("move", move_cmd)?;

    let move_backward_cmd = lua.create_function(|_, dist: f32| {
        Ok(PlayerCommand::Move(MovementDirection::Backward, dist))
    })?;
    me.set("move_backward", move_backward_cmd)?;

    let move_left_cmd =
        lua.create_function(|_, dist: f32| Ok(PlayerCommand::Move(MovementDirection::Left, dist)))?;
    me.set("move_left", move_left_cmd)?;

    let move_right_cmd = lua
        .create_function(|_, dist: f32| Ok(PlayerCommand::Move(MovementDirection::Right, dist)))?;
    me.set("move_right", move_right_cmd)?;

    let attack_cmd = lua.create_function(|_, _: ()| Ok(PlayerCommand::Attack))?;
    me.set("attack", attack_cmd)?;

    let turn_cmd = lua.create_function(|_, angle: f32| Ok(PlayerCommand::Turn(angle)))?;
    me.set("turn", &turn_cmd)?;

    let turn_head_cmd = lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnHead(angle)))?;
    me.set("turn_head", turn_head_cmd)?;

    let turn_arms_cmd = lua.create_function(|_, angle: f32| Ok(PlayerCommand::TurnArms(angle)))?;
    me.set("turn_arms", turn_arms_cmd)?;

    lua.globals().set("me", me)?;
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
    pub tick: i32,
    pub round: i32,
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

    pub fn add_lua_player(&mut self, player: Player, lua_impl: LuaImpl) -> LuaResult<()> {
        register_lua_library(&player, &lua_impl)?;
        self.players.push(player);
        self.lua_impls.push(lua_impl);
        Ok(())
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

    pub fn lua_impl(&mut self, id: u8) -> &mut LuaImpl {
        // FIXME: player id should be index
        &mut self.lua_impls[id as usize]
    }
}

pub fn load_lua_player(player_dir: &str, meta: &PlayerMeta) -> LuaResult<LuaImpl> {
    // FIXME: use PathBuf or similar
    let file = format!("{player_dir}/{0}", meta.entrypoint);
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

pub enum GameEvent {
    Tick(i32),
    RoundStarted(i32),
    RoundOver(Option<u8>),
    PlayerHeadTurned(u8, f32),
    PlayerArmsTurned(u8, f32),
    EnemySeen(u8, String, Point),
    Hit(usize, u8, u8, Point),
    AttackAdvanced(usize, Point),
    AttackMissed(usize),
    AttackCreated(u8, Attack),
    PlayerPositionUpdated(u8, Point),
    PlayerTurned(u8, f32),
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn transition_players(game: &mut Game, event_manager: &mut EventManager) {
    // TODO: is a HashMap here appropriate? is there a smarter way?
    let mut next_positions: HashMap<u8, Point> = HashMap::new();
    for (index, player) in game.players.iter().enumerate() {
        let lua_impl = &game.lua_impls[index];
        let delta = math_utils::clamp(lua_impl.intent.turn_angle, -MAX_TURN_RATE, MAX_TURN_RATE);
        event_manager.record(GameEvent::PlayerTurned(player.id, delta));
        let heading = math_utils::normalize_abs_angle(player.heading + delta);
        let velocity = f32::min(lua_impl.intent.distance, MAX_VELOCITY);
        let dir_heading = match lua_impl.intent.direction {
            MovementDirection::Forward => 0.0,
            MovementDirection::Backward => crate::PI as f32,
            MovementDirection::Left => -HALF_PI,
            MovementDirection::Right => HALF_PI,
        };
        let movement_heading = heading + dir_heading;
        let dx = movement_heading.sin() * velocity;
        let dy = -movement_heading.cos() * velocity;
        let delta = Point { x: dx, y: dy };
        next_positions.insert(player.id, delta);

        transition_heads(player, lua_impl, event_manager);
        transition_arms(player, lua_impl, event_manager);
    }

    for player in game.players.iter() {
        if !next_positions.iter().any(|(id, next_pos)| {
            *id != player.id && players_collide(&player.pos.read().unwrap(), &next_pos)
        }) {
            event_manager.record(GameEvent::PlayerPositionUpdated(
                player.id,
                next_positions.get(&player.id).unwrap().clone(),
            ));
        }
    }
}

fn transition_heads(player: &Player, lua_impl: &LuaImpl, event_manager: &mut EventManager) {
    let delta = math_utils::clamp(
        lua_impl.intent.turn_head_angle,
        -MAX_HEAD_TURN_RATE,
        MAX_HEAD_TURN_RATE,
    );
    event_manager.record(GameEvent::PlayerHeadTurned(player.id, delta));
}

fn transition_arms(player: &Player, lua_impl: &LuaImpl, event_manager: &mut EventManager) {
    let delta = math_utils::clamp(
        lua_impl.intent.turn_arms_angle,
        -MAX_ARMS_TURN_RATE,
        MAX_ARMS_TURN_RATE,
    );
    event_manager.record(GameEvent::PlayerArmsTurned(player.id, delta));
}

fn players_collide(p: &Point, q: &Point) -> bool {
    p.dist(q) <= 2.0 * (PLAYER_RADIUS as f32)
}

#[derive(Debug)]
pub enum PlayerEvent {
    Tick(i32),
    RoundStarted(i32),
    EnemySeen(String, Point),
}

fn game_events_to_player_events(player: &Player, game_events: &[GameEvent]) -> Vec<PlayerEvent> {
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
        GameEvent::RoundOver(_) => acc,
        GameEvent::EnemySeen(id, target, pos) => {
            if *id == player.id {
                acc.push(PlayerEvent::EnemySeen(target.clone(), pos.clone()));
            }
            acc
        }
        GameEvent::PlayerTurned(_, _) => acc,
        GameEvent::PlayerPositionUpdated(_, _) => acc,
        GameEvent::PlayerHeadTurned(_, _) => acc,
        GameEvent::PlayerArmsTurned(_, _) => acc,
        GameEvent::Hit(_, _, _, _) => acc,
        GameEvent::AttackAdvanced(_, _) => acc,
        GameEvent::AttackMissed(_) => acc,
        GameEvent::AttackCreated(_, _) => acc,
    })
}

fn can_spot(
    origin: &Point,
    view_angle: f32,
    target: &Point,
    player_radius: f32,
    angle_of_vision: f32,
) -> bool {
    // FIXME: test this most likely overly complicated stuff
    let delta = angle_of_vision / 2.0;
    let left = view_angle - delta;
    let right = view_angle + delta;
    let dist = origin.dist(target);
    let alpha = f32::atan(player_radius / dist);
    let angle = math_utils::normalize_abs_angle(math_utils::angle_between(origin, target));
    let alpha_left = angle - alpha;
    let alpha_right = angle + alpha;
    math_utils::between(alpha_left, left, right)
        || math_utils::between(alpha_right, left, right)
        || (alpha_left <= left && alpha_right >= right)
}

fn dispatch_player_events(
    player_events: Vec<PlayerEvent>,
    lua_player: &LuaImpl,
) -> LuaResult<Vec<PlayerCommand>> {
    let mut commands = Vec::new();
    for e in player_events.iter() {
        commands.append(&mut lua_player.on_event(&e)?);
    }
    Ok(commands)
}

fn determine_vision_events(game: &Game, event_manager: &mut EventManager) {
    // FIXME: learn how to do this in a better way!
    let player_positions: Vec<(u8, String, Point)> = game
        .players
        .iter()
        .map(|player| {
            (
                player.id,
                player.meta.name.clone(),
                player.pos.read().unwrap().clone(),
            )
        })
        .collect();
    for player in game.living_players() {
        for (id, name, pos) in player_positions.iter() {
            if *id != player.id {
                if can_spot(
                    &player.pos.read().unwrap(),
                    player.effective_head_heading(),
                    &pos,
                    PLAYER_RADIUS as f32,
                    ANGLE_OF_VISION,
                ) {
                    event_manager.record(GameEvent::EnemySeen(
                        player.id,
                        name.to_string(),
                        pos.clone(),
                    ));
                }
            }
        }
    }
}

fn create_attacks(game: &mut Game, event_manager: &mut EventManager) {
    for (index, player) in game.players.iter_mut().enumerate() {
        let lua_impl = &mut game.lua_impls[index];
        if lua_impl.intent.attack {
            lua_impl.intent.attack = false;
            let attack = Attack {
                id: game.attack_ids.next(),
                owner: player.id,
                pos: player.pos.read().unwrap().clone(),
                velocity: 2.5,
                heading: player.arms_heading,
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
            && attack.pos.dist(&player.pos.read().unwrap()) <= ATTACK_RADIUS + PLAYER_RADIUS as f32
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

    // FIXME: don't pass the whole game state
    pub fn init_tick(&mut self, game: &Game) {
        self.current_events = tick_events(game);
    }

    pub fn record(&mut self, event: GameEvent) {
        self.current_events.push(event);
    }

    pub fn current_events(&self) -> &[GameEvent] {
        &self.current_events
    }
}

fn tick_events(game: &Game) -> Vec<GameEvent> {
    let mut events = vec![GameEvent::Tick(game.tick)];
    if game.tick == 0 {
        events.push(GameEvent::RoundStarted(game.round));
    }
    events
}

fn transition_attacks(game: &Game, event_manager: &mut EventManager) {
    for attack in game.attacks.iter() {
        let new_pos = math_utils::line_endpoint(
            attack.pos.x as f32,
            attack.pos.y as f32,
            attack.velocity,
            attack.heading,
        );
        if inside_arena(&new_pos) {
            if let Some(player) = attack_hits_player(&attack, game.living_players()) {
                // FIXME: new_pos or old position here?
                event_manager.record(GameEvent::Hit(attack.id, attack.owner, player.id, new_pos));
            } else {
                event_manager.record(GameEvent::AttackAdvanced(attack.id, new_pos));
            }
        } else {
            event_manager.record(GameEvent::AttackMissed(attack.id));
        }
    }
}

fn advance_game_state(game: &mut Game, game_events: &[GameEvent]) {
    for event in game_events.iter() {
        match event {
            GameEvent::Tick(_) => {}
            GameEvent::RoundStarted(_) => {}
            GameEvent::RoundOver(_) => todo!("handle end of round"),
            GameEvent::PlayerPositionUpdated(id, delta) => {
                let distance;
                {
                    let player = game.player(*id);
                    let mut pos = player.pos.write().unwrap();
                    distance = pos.dist(&Point { x: 0.0, y: 0.0 }); // TODO: length of a Vec2
                    pos.x += delta.x;
                    pos.y += delta.y;
                }
                let lua_impl = game.lua_impl(*id);
                lua_impl.intent.distance = f32::max(lua_impl.intent.distance - distance, 0.0)
            }
            GameEvent::PlayerTurned(id, delta) => {
                {
                    let player = game.player(*id);
                    player.heading = math_utils::normalize_abs_angle(player.heading + *delta);
                }
                let lua_impl = game.lua_impl(*id);
                lua_impl.intent.turn_angle = if lua_impl.intent.turn_angle.abs() < MAX_TURN_RATE {
                    0.0
                } else {
                    lua_impl.intent.turn_angle - *delta
                };
            }
            GameEvent::PlayerHeadTurned(id, delta) => {
                {
                    let player = game.player(*id);
                    let heading = clamp_turn_angle(player.head_heading + *delta);
                    player.head_heading = heading;
                }
                let lua_impl = game.lua_impl(*id);
                lua_impl.intent.turn_head_angle =
                    if lua_impl.intent.turn_head_angle.abs() < MAX_HEAD_TURN_RATE {
                        0.0
                    } else {
                        lua_impl.intent.turn_head_angle - *delta
                    };
            }
            GameEvent::PlayerArmsTurned(id, delta) => {
                {
                    let player = game.player(*id);
                    let heading = clamp_turn_angle(player.arms_heading + *delta);
                    player.arms_heading = heading;
                }
                let lua_impl = game.lua_impl(*id);
                lua_impl.intent.turn_arms_angle =
                    if lua_impl.intent.turn_arms_angle.abs() < MAX_ARMS_TURN_RATE {
                        0.0
                    } else {
                        lua_impl.intent.turn_arms_angle - *delta
                    };
            }
            GameEvent::EnemySeen(_, _, _) => {}
            GameEvent::Hit(attack_id, _, victim_id, _) => {
                if let Some(index) = game
                    .attacks
                    .iter()
                    .position(|attack| attack.id == *attack_id)
                {
                    game.attacks.remove(index);
                }
                game.player(*victim_id).hp -= ATTACK_DAMAGE;
            }
            GameEvent::AttackAdvanced(id, pos) => {
                let attack = game
                    .attacks
                    .iter_mut()
                    .find(|attack| attack.id == *id)
                    .expect("attack {id} not found");
                attack.pos.x = pos.x;
                attack.pos.y = pos.y;
            }
            GameEvent::AttackMissed(id) => {
                if let Some(index) = game.attacks.iter().position(|attack| attack.id == *id) {
                    game.attacks.remove(index);
                }
            }
            GameEvent::AttackCreated(_owner, attack) => {
                game.attacks.push(attack.clone());
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

pub fn step(
    game: &mut Game,
    event_manager: &mut EventManager,
    game_writer: &mpsc::Sender<GameData>,
) -> LuaResult<()> {
    event_manager.init_tick(game);
    // FIXME: EnemySeen is unnecessary/useless as a game event -> it only
    // matters for players
    determine_vision_events(game, event_manager);
    transition_players(game, event_manager);
    create_attacks(game, event_manager);
    transition_attacks(game, event_manager);

    let game_events: &[GameEvent] = event_manager.current_events();
    advance_game_state(game, game_events);

    for (index, player) in game.players.iter_mut().enumerate() {
        let player_events = game_events_to_player_events(player, game_events);
        let lua_impl = &mut game.lua_impls[index];
        let mut commands = dispatch_player_events(player_events, lua_impl)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                PlayerCommand::Attack => lua_impl.intent.attack = true,
                PlayerCommand::Turn(angle) => lua_impl.intent.turn_angle = *angle,
                PlayerCommand::TurnHead(angle) => lua_impl.intent.turn_head_angle = *angle,
                PlayerCommand::TurnArms(angle) => lua_impl.intent.turn_arms_angle = *angle,
                PlayerCommand::Move(dir, dist) => {
                    lua_impl.intent.direction = dir.clone();
                    lua_impl.intent.distance = *dist;
                }
            }
        }
    }

    let mut game_data = GameData::new();
    for player in game.living_players() {
        let p = player.pos.read().unwrap();
        game_data.players.push(PlayerData {
            color: player.meta.color.clone(),
            x: p.x.round() as i32,
            y: p.y.round() as i32,
            heading: player.heading,
            head_heading: player.effective_head_heading(),
            arms_heading: player.effective_arms_heading(),
        });
    }
    for attack in game.attacks.iter() {
        game_data.attacks.push(attack.pos.clone());
    }
    game_writer.send(game_data).unwrap();

    game.tick += 1;
    Ok(())
}

pub fn run_round(
    game: &mut Game,
    event_manager: &mut EventManager,
    // FIXME: find out whether it's better to pass such things via & or without
    delay: &std::time::Duration,
    game_writer: &mpsc::Sender<GameData>,
) -> LuaResult<()> {
    loop {
        std::thread::sleep(*delay);
        step(game, event_manager, game_writer)?;
        match game.round_state {
            RoundState::Ongoing => {}
            RoundState::Won(id) => {
                println!("Player {id} has won!");
                break Ok(());
            }
            RoundState::Draw => {
                println!("--- DRAW ---");
                break Ok(());
            }
        }
    }
}

pub fn run_game(
    game: &mut Game,
    delay: &std::time::Duration,
    game_writer: &mpsc::Sender<GameData>,
) -> LuaResult<()> {
    let mut event_manager = EventManager::new();
    let max_rounds = 2;
    for round in 1..max_rounds {
        game.round = round;
        run_round(game, &mut event_manager, delay, game_writer)?;
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
            let res: Vec<PlayerCommand> = player.on_event(&PlayerEvent::Tick(17)).unwrap();
            let cmd = res.first().expect("some command");
            assert_eq!(*cmd, PlayerCommand::Move(MovementDirection::Left, 13.12));
        }

        #[test]
        fn call_on_tick_if_missing() {
            let player = LuaImpl::new("return {}").unwrap();
            let res: Vec<PlayerCommand> = player.on_event(&PlayerEvent::Tick(17)).unwrap();
            assert_eq!(res.len(), 0);
        }
    }

    mod can_spot {
        use std::f32::consts::PI;

        use super::*;

        #[test]
        fn first_quadrant_too_far_left() {
            let visible = can_spot(
                &Point { x: 400.0, y: 400.0 },
                -PI / 4.0,
                &Point { x: 500.0, y: 300.0 },
                25.0,
                1.4,
            );
            assert!(!visible)
        }

        #[test]
        fn first_quadrant_too_far_right() {
            let visible = can_spot(
                &Point { x: 400.0, y: 400.0 },
                3.0 * PI / 4.0,
                &Point { x: 500.0, y: 300.0 },
                25.0,
                1.4,
            );
            assert!(!visible);
        }

        #[test]
        fn first_quadrant_head_on() {
            let visible = can_spot(
                &Point { x: 400.0, y: 400.0 },
                PI / 4.0,
                &Point { x: 500.0, y: 300.0 },
                25.0,
                1.4,
            );
            assert!(visible);
        }

        #[test]
        fn first_quadrant_target_larger_than_vision_angle() {
            let visible = can_spot(
                &Point { x: 400.0, y: 400.0 },
                0.7,
                &Point { x: 500.0, y: 300.0 },
                50.0,
                0.1,
            );
            assert!(visible);
        }

        // FIXME: test other quadrants and relative positions
    }
}
