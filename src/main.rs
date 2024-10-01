use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;
use raylib::prelude::*;

const PLAYER_RADIUS: i32 = 25;
const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;

#[derive(PartialEq, Debug)]
enum PlayerCommand {
    Attack(f32),
    Turn(f32),
    TurnHead(f32),
    TurnArms(f32),
    Move(f32),
}

impl PlayerCommand {
    fn index(&self) -> i32 {
        match self {
            PlayerCommand::Move(_) => 0,
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
                "move" => Ok(PlayerCommand::Move(t.get("distance")?)),
                "attack" => Ok(PlayerCommand::Attack(t.get("angle")?)),
                "turn" => Ok(PlayerCommand::Turn(t.get("angle")?)),
                "turn_head" => Ok(PlayerCommand::TurnHead(t.get("angle")?)),
                "turn_arms" => Ok(PlayerCommand::TurnArms(t.get("angle")?)),
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

    fn on_tick(&self, tick: i32) -> LuaResult<Vec<PlayerCommand>> {
        let res: Vec<PlayerCommand> = self.table()?.call_function("on_tick", tick)?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn dist_sqr(&self, p: &Point) -> i32 {
        (self.x - p.x).pow(2) + (self.y - p.y).pow(2)
    }

    fn dist(&self, p: &Point) -> f32 {
        let d = self.dist_sqr(p) as f32;
        d.sqrt()
    }
}

struct PlayerIntent {
    distance: f32,
    attack: bool,
    turn_angle: f32,
    turn_head_angle: f32,
    turn_arms_angle: f32,
}

impl Default for PlayerIntent {
    fn default() -> Self {
        Self {
            distance: 0.0,
            turn_head_angle: 0.0,
            turn_arms_angle: 0.0,
            attack: false,
            turn_angle: 0.0,
        }
    }
}

struct Player {
    id: u8,
    lua_player: LuaPlayer,
    pos: Rc<RefCell<Point>>,
    heading: f32,
    head_heading: f32,
    arms_heading: f32,
    intent: PlayerIntent,
    next_pos: Point,
}

impl Player {
    fn new(file_path: &str, id: u8, x: i32, y: i32) -> LuaResult<Player> {
        let res = Self {
            id,
            lua_player: load_lua_player(file_path)?,
            pos: Rc::new(RefCell::new(Point { x, y })),
            heading: 0.0,
            head_heading: 0.0,
            arms_heading: 0.0,
            next_pos: Point { x, y },
            intent: Default::default(),
        };
        res.register_lua_library()?;
        Ok(res)
    }

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
}

struct GameState {
    tick: i32,
    round: i32,
    players: Vec<Player>,
}

impl GameState {
    fn new() -> GameState {
        Self {
            tick: 0,
            round: 1,
            players: Vec::new(),
        }
    }
}

fn draw_line_in_direction(
    d: &mut raylib::drawing::RaylibDrawHandle,
    x: i32,
    y: i32,
    angle: f32,
    length: f32,
    color: raylib::color::Color,
) {
    let dx = angle.sin() * length;
    let dy = angle.cos() * length;
    d.draw_line(x, y, x + dx.round() as i32, y - dy.round() as i32, color);
}

mod render {
    use super::*;
    use raylib::prelude::*;

    fn player_vision(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
        draw_line_in_direction(d, x, y, heading, 2.0 * PLAYER_RADIUS as f32, Color::RED);
    }

    fn player_arms(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
        draw_line_in_direction(d, x, y, heading, 1.5 * PLAYER_RADIUS as f32, Color::YELLOW);
    }

    fn heading(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
        draw_line_in_direction(
            d,
            x,
            y,
            heading,
            1.6 * PLAYER_RADIUS as f32,
            Color::GREENYELLOW,
        );
        draw_line_in_direction(
            d,
            x,
            y,
            heading + PI as f32,
            1.2 * PLAYER_RADIUS as f32,
            Color::GREENYELLOW,
        );
        draw_line_in_direction(
            d,
            x,
            y,
            heading + PI as f32 / 2.0,
            1.2 * PLAYER_RADIUS as f32,
            Color::GREENYELLOW,
        );
        draw_line_in_direction(
            d,
            x,
            y,
            heading - PI as f32 / 2.0,
            1.2 * PLAYER_RADIUS as f32,
            Color::GREENYELLOW,
        );
    }

    pub fn players(d: &mut raylib::drawing::RaylibDrawHandle, players: &Vec<Player>) {
        for p in players {
            let pos = p.pos.borrow();
            player_vision(d, pos.x, pos.y, p.head_heading);
            player_arms(d, pos.x, pos.y, p.arms_heading);
            heading(d, pos.x, pos.y, p.heading);
            d.draw_circle(pos.x, pos.y, 25.0, Color::GREENYELLOW);
        }
    }
}

fn load_lua_player(file_path: &str) -> LuaResult<LuaPlayer> {
    let code = std::fs::read_to_string(file_path)?;
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

enum GameEvent {
    Tick(i32),
    RoundStarted(i32),
    PlayerMoved(u8, Point),
}

fn advance_players(state: &mut GameState, event_manager: &mut EventManager) {
    // FIXME: refactor: probably no need to mutate players for the next
    // positions, or even keep them in there as state at all!
    for player in state.players.iter_mut() {
        let p = player.pos.borrow();

        let next_pos = Point {
            x: p.x + player.intent.distance.round() as i32,
            y: p.y,
        };
        if valid_position(&next_pos) {
            player.next_pos = next_pos;
        } else {
            player.next_pos = p.clone();
        }

        player.heading += player.intent.turn_angle;
        player.head_heading += player.intent.turn_head_angle;
        player.arms_heading += player.intent.turn_arms_angle;
    }

    let next_positions: Vec<(u8, Point)> = state
        .players
        .iter()
        .map(|player| (player.id, player.next_pos.clone()))
        .collect();
    state.players.iter_mut().for_each(|player| {
        if !next_positions.iter().any(|(id, next_pos)| {
            *id != player.id && players_collide(&player.pos.borrow(), next_pos)
        }) {
            let mut pos = player.pos.borrow_mut();
            pos.x = player.next_pos.x;
            pos.y = player.next_pos.y;
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
            PlayerEvent::RoundStarted(_) => todo!("round started handler"),
        }
    }
    Ok(commands)
}

fn step(state: &mut GameState, event_manager: &mut EventManager) -> LuaResult<()> {
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
                PlayerCommand::Move(dist) => player.intent.distance = *dist,
            }
        }
    }
    Ok(())
}

struct EventManager {
    current_events: Vec<GameEvent>,
}

impl EventManager {
    fn new() -> EventManager {
        Self {
            current_events: vec![],
        }
    }

    // FIXME: don't pass the whole game state
    fn next_tick(&mut self, state: &GameState) {
        self.current_events = tick_events(state);
    }

    fn record_event(&mut self, event: GameEvent) {
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

fn main() -> LuaResult<()> {
    // FIXME: IDs
    let player1 = Player::new("players/kai.lua", 1, 30, 50)?;
    let player2 = Player::new("players/lloyd.lua", 2, 400, 50)?;

    let mut state = GameState::new();
    state.players = vec![player1, player2];
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("hello world")
        .build();
    let mut event_manager = EventManager::new();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        state.tick += 1;
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
        step(&mut state, &mut event_manager)?;
        render::players(&mut d, &state.players);
    }
    Ok(())
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
            "return { on_tick = function(n) return { { tag = \"move\", distance = 13.12 } } end }",
        )
        .expect("lua player could not be created");
        let res: Vec<PlayerCommand> = player.on_tick(17).expect("on_tick failed");
        let cmd = res.first().expect("some command");
        assert_eq!(*cmd, PlayerCommand::Move(13.12));
    }
}
