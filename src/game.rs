use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;

use crate::math_utils::HALF_PI;
use crate::math_utils::{self, Point};
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

struct LuaPlayer {
    lua: Lua,
    key: LuaRegistryKey,
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

#[derive(Clone)]
struct NextMove {
    pos: Point,
    distance: f32,
}

pub struct Player {
    pub id: u8,
    pub hp: f32,
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
            hp: 100.0,
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

    pub fn alive(&self) -> bool {
        self.hp > 0.0
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

        let attack_cmd = lua.create_function(|_, _: ()| Ok(PlayerCommand::Attack))?;
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

enum RoundState {
    Ongoing,
    Won(u8),
    Draw,
}

pub struct Game {
    pub tick: i32,
    pub round: i32,
    pub players: Vec<Player>,
    pub attacks: Vec<Attack>,
    pub round_state: RoundState,
    attack_ids: Ids,
}

impl Game {
    pub fn new() -> Game {
        Self {
            tick: 0,
            round: 1,
            players: Vec::new(),
            attacks: vec![],
            attack_ids: Ids::new(),
            round_state: RoundState::Ongoing,
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
}

fn load_lua_player(player_dir: &str, meta: &PlayerMeta) -> LuaResult<LuaPlayer> {
    // FIXME: use PathBuf or similar
    let file = format!("{player_dir}/{0}", meta.entrypoint);
    let code = std::fs::read_to_string(file)?;
    LuaPlayer::new(&code)
}

fn reduce_commands(commands: &mut Vec<PlayerCommand>) {
    // FIXME: check whether they're really reduced
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
    RoundOver(Option<u8>),
    PlayerHeadTurned(u8, f32),
    PlayerArmsTurned(u8, f32),
    EnemySeen(u8, String, Point),
    Hit(usize, u8, u8, Point),
    AttackAdvanced(usize, Point),
    AttackMissed(usize),
    AttackCreated(u8, Attack),
}

impl GameEvent {
    fn index(&self) -> i32 {
        match self {
            GameEvent::RoundStarted(_) => 0,
            GameEvent::RoundOver(_) => 1,
            GameEvent::Tick(_) => 2,
            GameEvent::Hit(_, _, _, _) => 3,
            GameEvent::AttackMissed(_) => 4,
            GameEvent::AttackAdvanced(_, _) => 5,
            GameEvent::AttackCreated(_, _) => 6,
            GameEvent::PlayerHeadTurned(_, _) => 7,
            GameEvent::PlayerArmsTurned(_, _) => 8,
            GameEvent::EnemySeen(_, _, _) => 9,
        }
    }
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn transition_players(game: &mut Game, event_manager: &mut EventManager) {
    // FIXME: refactor: probably no need to mutate players for the next
    // positions, or even keep them in there as state at all!
    for player in game.players.iter_mut() {
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
            event_manager.record(GameEvent::PlayerHeadTurned(player.id, heading));
            // FIXME: how to handle intent when using events for the state?
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
            event_manager.record(GameEvent::PlayerArmsTurned(player.id, heading));
            player.intent.turn_arms_angle = remaining;
        }
    }

    let next_positions: Vec<(u8, NextMove)> = game
        .players
        .iter()
        .map(|player| (player.id, player.next_move.clone()))
        .collect();
    game.players.iter_mut().for_each(|player| {
        if !next_positions.iter().any(|(id, next_move)| {
            *id != player.id && players_collide(&player.pos.borrow(), &next_move.pos)
        }) {
            let mut pos = player.pos.borrow_mut();
            pos.x = player.next_move.pos.x;
            pos.y = player.next_move.pos.y;
            player.intent.distance = player.next_move.distance;
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
    player: &Player,
    player_events: Vec<PlayerEvent>,
) -> LuaResult<Vec<PlayerCommand>> {
    let mut commands = Vec::new();
    for e in player_events.iter() {
        commands.append(&mut player.lua_player.on_event(&e)?);
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
                player.pos.borrow().clone(),
            )
        })
        .collect();
    for player in game.living_players() {
        for (id, name, pos) in player_positions.iter() {
            if *id != player.id {
                if can_spot(
                    &player.pos.borrow(),
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
    for player in game.players.iter_mut() {
        if player.intent.attack {
            player.intent.attack = false;
            let attack = Attack {
                id: game.attack_ids.next(),
                owner: player.id,
                pos: player.pos.borrow().clone(),
                velocity: 2.5,
                heading: player.arms_heading,
            };
            event_manager.record(GameEvent::AttackCreated(player.id, attack));
        }
    }
}

fn inside_arena(x: i32, y: i32) -> bool {
    x >= 0 && x <= WIDTH && y >= 0 && y <= HEIGHT
}

fn attack_hits_player<'a>(
    attack: &Attack,
    mut players: impl Iterator<Item = &'a Player>,
) -> Option<&'a Player> {
    players.find(|player| {
        player.id != attack.owner
            && attack.pos.dist(&player.pos.borrow()) <= ATTACK_RADIUS + PLAYER_RADIUS as f32
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

    pub fn end_tick(&mut self) {
        self.current_events.sort_by_key(|event| event.index());
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
        let (x, y) = math_utils::line_endpoint(
            attack.pos.x as f32,
            attack.pos.y as f32,
            attack.velocity,
            attack.heading,
        );
        let new_x = x.round() as i32;
        let new_y = y.round() as i32;
        if inside_arena(new_x, new_y) {
            if let Some(player) = attack_hits_player(&attack, game.living_players()) {
                println!("player hit! it was {}", player.meta.name);
                event_manager.record(GameEvent::Hit(
                    attack.id,
                    attack.owner,
                    player.id,
                    attack.pos.clone(),
                ));
            } else {
                event_manager.record(GameEvent::AttackAdvanced(
                    attack.id,
                    Point { x: new_x, y: new_y },
                ));
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
            // FIXME/IDEA: really store only the delta, as for event sourcing?
            GameEvent::PlayerHeadTurned(id, heading) => {
                game.player(*id).head_heading = *heading;
            }
            GameEvent::PlayerArmsTurned(id, heading) => {
                game.player(*id).arms_heading = *heading;
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

pub fn step(game: &mut Game, event_manager: &mut EventManager) -> LuaResult<()> {
    event_manager.init_tick(game);
    // FIXME: EnemySeen is unnecessary/useless as a game event -> it only
    // matters for players
    determine_vision_events(game, event_manager);
    transition_players(game, event_manager);
    create_attacks(game, event_manager);
    transition_attacks(game, event_manager);
    event_manager.end_tick();

    let game_events: &[GameEvent] = event_manager.current_events();
    advance_game_state(game, game_events);

    for player in game.players.iter_mut() {
        let player_events = game_events_to_player_events(player, game_events);
        let mut commands = dispatch_player_events(player, player_events)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                PlayerCommand::Attack => player.intent.attack = true,
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

pub fn run_round(game: &mut Game, event_manager: &mut EventManager) -> LuaResult<()> {
    step(game, event_manager)?;
    match game.round_state {
        RoundState::Ongoing => run_round(game, event_manager),
        RoundState::Won(id) => {
            println!("Player {id} has won!");
            Ok(())
        }
        RoundState::Draw => {
            println!("--- DRAW ---");
            Ok(())
        }
    }
}

pub fn run_game(game: &mut Game) -> LuaResult<()> {
    let mut event_manager = EventManager::new();
    let max_rounds = 2;
    for round in 1..max_rounds {
        game.round = round;
        run_round(game, &mut event_manager)?;
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
            LuaPlayer::new("return {}").expect("lua player could not be created");
        }

        #[test]
        fn call_on_tick() {
            let player = LuaPlayer::new("return { on_tick = function(n) return { { tag = \"move\", distance = 13.12, direction = \"left\" } } end }")
                .expect("lua player could not be created");
            let res: Vec<PlayerCommand> = player.on_event(&PlayerEvent::Tick(17)).unwrap();
            let cmd = res.first().expect("some command");
            assert_eq!(*cmd, PlayerCommand::Move(MovementDirection::Left, 13.12));
        }

        #[test]
        fn call_on_tick_if_missing() {
            let player = LuaPlayer::new("return {}").unwrap();
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
                &Point { x: 400, y: 400 },
                -PI / 4.0,
                &Point { x: 500, y: 300 },
                25.0,
                1.4,
            );
            assert!(!visible)
        }

        #[test]
        fn first_quadrant_too_far_right() {
            let visible = can_spot(
                &Point { x: 400, y: 400 },
                3.0 * PI / 4.0,
                &Point { x: 500, y: 300 },
                25.0,
                1.4,
            );
            assert!(!visible);
        }

        #[test]
        fn first_quadrant_head_on() {
            let visible = can_spot(
                &Point { x: 400, y: 400 },
                PI / 4.0,
                &Point { x: 500, y: 300 },
                25.0,
                1.4,
            );
            assert!(visible);
        }

        #[test]
        fn first_quadrant_target_larger_than_vision_angle() {
            let visible = can_spot(
                &Point { x: 400, y: 400 },
                0.7,
                &Point { x: 500, y: 300 },
                50.0,
                0.1,
            );
            assert!(visible);
        }

        // FIXME: test other quadrants and relative positions
    }
}
