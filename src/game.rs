use core::fmt;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use rand::Rng;

use crate::math_utils::{self, Point, Sector, HALF_PI};
use crate::player::{self, MovementDirection, Player};
use crate::{lua_player, settings::*, wasm_player};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct AttackId(usize);

#[derive(Clone, Debug)]
pub struct Attack {
    pub id: AttackId,
    pub pos: Point,
    pub owner: player::Meta,
    pub heading: f32,
    pub velocity: f32,
}

pub struct AttackIds {
    next: usize,
}

impl AttackIds {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn next(&mut self) -> AttackId {
        let next = self.next;
        self.next += 1;
        AttackId(next)
    }
}

pub enum RoundState {
    Ongoing,
    Won(player::Meta),
    Draw,
}

pub struct Game {
    pub tick: u32,
    pub round: u32,
    pub players: HashMap<player::Meta, player::State>,
    pub impls: HashMap<player::Meta, Player>,
    pub attacks: Vec<Attack>,
    pub round_state: RoundState,
    attack_ids: AttackIds,
}

pub struct AddPlayerError {
    pub message: String,
}

impl fmt::Display for AddPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<mlua::Error> for AddPlayerError {
    fn from(err: mlua::Error) -> Self {
        Self {
            message: format!("{err}"),
        }
    }
}

impl Game {
    pub fn new() -> Game {
        Self {
            tick: 0,
            round: 1,
            players: HashMap::new(),
            impls: HashMap::new(),
            attacks: vec![],
            attack_ids: AttackIds::new(),
            round_state: RoundState::Ongoing,
        }
    }

    // FIXME: consolidate with below
    pub fn add_lua_player(&mut self, player_dir: &Path) -> Result<(), AddPlayerError> {
        let meta = player::Meta::from_toml_file(&player_dir.join("meta.toml"))
            .map_err(|e| AddPlayerError { message: e.0 })?;
        let player_state = player::State::new();
        let intent = player::Intent::default();
        let lua_impl = lua_player::LuaImpl::load(player_dir, &meta)?;
        self.impls.insert(
            meta.clone(),
            Player {
                implementation: Box::new(lua_impl),
                intent,
            },
        );
        self.players.insert(meta, player_state);
        Ok(())
    }

    pub fn add_wasm_player(&mut self, player_dir: &Path) -> Result<(), AddPlayerError> {
        let meta = player::Meta::from_toml_file(&player_dir.join("meta.toml"))
            .map_err(|e| AddPlayerError { message: e.0 })?;
        let player_state = player::State::new();
        let intent = player::Intent::default();
        let wasm_impl = wasm_player::WasmImpl::load(&player_dir.join("main.wasm"))
            .map_err(|e| AddPlayerError { message: e.message })?;
        self.impls.insert(
            meta.clone(),
            Player {
                implementation: Box::new(wasm_impl),
                intent,
            },
        );
        self.players.insert(meta, player_state);
        Ok(())
    }

    pub fn init_round(&mut self, round: u32, event_manager: &mut EventManager) {
        self.tick = 0;
        self.round = round;
        self.round_state = RoundState::Ongoing;
        self.attacks = vec![];
        let min = PLAYER_RADIUS + 5.0;
        let max_x = WIDTH as f32 - PLAYER_RADIUS - 5.0;
        let max_y = HEIGHT as f32 - PLAYER_RADIUS - 5.0;
        let mut rng = rand::thread_rng();
        let mut players = HashMap::new();
        for (meta, player_state) in self.players.iter_mut() {
            // FIXME: don't create collisions
            let random_pos = Point {
                x: rng.gen_range(min..max_x) as f32,
                y: rng.gen_range(min..max_y) as f32,
            };
            // FIXME: it would be nice to change state later (as with the rest),
            // but that creates problems with the check for "round over"
            player_state.reset(random_pos.clone());
            players.insert(meta.clone(), random_pos);
        }
        event_manager.init_round(round, players);
        for (_, player) in self.impls.iter_mut() {
            player.intent = Default::default();
        }
    }

    pub fn living_players(&self) -> impl Iterator<Item = (&player::Meta, &player::State)> {
        self.players.iter().filter(|(_, p)| p.alive())
    }

    pub fn player_state(&mut self, meta: &player::Meta) -> &mut player::State {
        self.players
            .iter_mut()
            .find(|(m, _)| *m == meta)
            .expect("player {id} not found")
            .1
    }

    pub fn attack(&mut self, id: &AttackId) -> &mut Attack {
        self.attacks
            .iter_mut()
            .find(|attack| attack.id == *id)
            .expect("attack {id} not found")
    }

    pub fn player(&mut self, meta: &player::Meta) -> &mut Player {
        self.impls.get_mut(meta).unwrap()
    }
}

fn reduce_commands(commands: &mut Vec<player::Command>) {
    // FIXME: check whether they're really reduced
    commands.sort_by_key(|cmd| cmd.index());
}

fn valid_position(p: &Point) -> bool {
    p.x >= PLAYER_RADIUS
        && p.x <= WIDTH as f32 - PLAYER_RADIUS
        && p.y >= PLAYER_RADIUS
        && p.y <= HEIGHT as f32 - PLAYER_RADIUS
}

#[derive(Clone, Debug)]
pub struct Delta {
    pub value: Point,
}

impl Delta {
    pub fn new(value: Point) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    Tick(u32),
    RoundStarted(u32, HashMap<player::Meta, Point>),
    RoundEnded(Option<player::Meta>),
    PlayerHeadTurned(player::Meta, f32),
    PlayerArmsTurned(player::Meta, f32),
    Hit(AttackId, player::Meta, player::Meta, Point),
    AttackAdvanced(AttackId, Point),
    AttackMissed(AttackId),
    // FIXME: does this really need the owner id twice?
    AttackCreated(player::Meta, Attack),
    PlayerPositionUpdated(player::Meta, Delta),
    PlayerTurned(player::Meta, f32),
    PlayerDied(player::Meta),
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn transition_players(game: &mut Game, event_manager: &mut EventManager) {
    // TODO: is a HashMap appropriate here? is there a smarter way?
    let mut next_positions: HashMap<player::Id, (Delta, Point)> = HashMap::new();
    for (meta, player_state) in game.living_players() {
        let player = game.impls.get(&meta).unwrap();
        let delta = math_utils::clamp(player.intent.turn_angle, -MAX_TURN_RATE, MAX_TURN_RATE);
        event_manager.record(GameEvent::PlayerTurned(meta.clone(), delta));
        let heading = math_utils::normalize_absolute_angle(player_state.heading + delta);
        let velocity = f32::min(player.intent.distance, MAX_VELOCITY);
        let dir_heading = match player.intent.direction {
            MovementDirection::Forward => 0.0,
            MovementDirection::Backward => math_utils::PI,
            MovementDirection::Left => -HALF_PI,
            MovementDirection::Right => HALF_PI,
        };
        let movement_heading = heading + dir_heading;
        let dx = movement_heading.sin() * velocity;
        let dy = -movement_heading.cos() * velocity;
        let delta = Delta::new(Point { x: dx, y: dy });
        let pos = &player_state.pos;
        let next_pos = pos.add(&delta.value);
        if valid_position(&next_pos) {
            next_positions.insert(meta.id.clone(), (delta, next_pos));
        } else {
            next_positions.insert(meta.id.clone(), (Delta::new(Point::zero()), pos.clone()));
        };

        transition_heads(
            &meta,
            player_state,
            player.intent.turn_head_angle,
            event_manager,
        );
        transition_arms(
            &meta,
            player_state,
            player.intent.turn_arms_angle,
            event_manager,
        );
    }

    for (meta, _) in game.living_players() {
        let (delta, next) = next_positions.get(&meta.id).unwrap();
        let mut collides = false;
        for (other_id, (_, other_next)) in next_positions.iter() {
            if meta.id != *other_id {
                if players_collide(&next, &other_next) {
                    // TODO: collision event
                    collides = true;
                }
            }
        }
        let event = if !collides {
            GameEvent::PlayerPositionUpdated(meta.clone(), delta.clone())
        } else {
            GameEvent::PlayerPositionUpdated(meta.clone(), Delta::new(Point::zero()))
        };
        event_manager.record(event);
    }
}

fn transition_heads(
    meta: &player::Meta,
    player_state: &player::State,
    turn_angle: f32,
    event_manager: &mut EventManager,
) {
    let delta = math_utils::clamp(turn_angle, -MAX_HEAD_TURN_RATE, MAX_HEAD_TURN_RATE);
    let current_heading = player_state.head_heading;
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::PlayerHeadTurned(meta.clone(), effective_delta));
}

fn transition_arms(
    meta: &player::Meta,
    player_state: &player::State,
    turn_angle: f32,
    event_manager: &mut EventManager,
) {
    let delta = math_utils::clamp(turn_angle, -MAX_ARMS_TURN_RATE, MAX_ARMS_TURN_RATE);
    let current_heading = player_state.arms_heading;
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::PlayerArmsTurned(meta.clone(), effective_delta));
}

fn players_collide(p: &Point, q: &Point) -> bool {
    p.dist(q) <= 2.0 * (PLAYER_RADIUS as f32)
}

fn game_events_to_player_events(
    meta: &player::Meta,
    player_state: &player::State,
    intent: &player::Intent,
    game_events: &[GameEvent],
) -> Vec<player::Event> {
    let mut player_events = Vec::new();
    for event in game_events.iter() {
        match event {
            GameEvent::Tick(n) => {
                player_events.push(player::Event::Tick(
                    *n,
                    player::CurrentPlayerState::from_state(&player_state, &intent),
                ));
            }
            GameEvent::RoundStarted(round, _) => {
                player_events.push(player::Event::RoundStarted(*round));
            }
            GameEvent::RoundEnded(opt_meta) => {
                player_events.push(player::Event::RoundEnded(opt_meta.clone()));
                match opt_meta {
                    Some(winner_meta) => {
                        if winner_meta == meta {
                            player_events.push(player::Event::RoundWon);
                        }
                    }
                    None => player_events.push(player::Event::RoundDrawn),
                }
            }
            GameEvent::PlayerTurned(_, _) => {}
            GameEvent::PlayerPositionUpdated(_, _) => {}
            GameEvent::PlayerHeadTurned(_, _) => {}
            GameEvent::PlayerArmsTurned(_, _) => {}
            GameEvent::Hit(_, owner_meta, victim_meta, pos) => {
                if meta == victim_meta {
                    // FIXME: don't use id
                    player_events.push(player::Event::HitBy(owner_meta.clone()));
                } else if meta == owner_meta {
                    player_events.push(player::Event::AttackHit(victim_meta.clone(), pos.clone()));
                }
            }
            GameEvent::AttackAdvanced(_, _) => {}
            GameEvent::AttackMissed(_) => {}
            GameEvent::AttackCreated(_, _) => {}
            GameEvent::PlayerDied(deceased_meta) => {
                let death_event = if meta == deceased_meta {
                    player::Event::Death
                } else {
                    player::Event::EnemyDied(deceased_meta.name.clone())
                };
                player_events.push(death_event);
            }
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
    player_events: Vec<player::Event>,
    player: &mut Box<dyn player::Impl>,
) -> Result<Vec<player::Command>, player::EventError> {
    let mut commands = Vec::new();
    for e in player_events.iter() {
        commands.append(&mut player.on_event(&e)?.value);
    }
    Ok(commands)
}

fn create_attacks(game: &mut Game, event_manager: &mut EventManager) {
    for (meta, player_state) in game.players.iter_mut() {
        let player = game.impls.get(meta).unwrap();
        let will_attack = player.intent.attack && player_state.attack_cooldown == 0;
        if will_attack {
            let attack = Attack {
                id: game.attack_ids.next(),
                owner: meta.clone(),
                pos: player_state.pos.clone(),
                velocity: 2.5,
                heading: player_state.effective_arms_heading(),
            };
            event_manager.record(GameEvent::AttackCreated(meta.clone(), attack));
        }
    }
}

fn inside_arena(p: &Point) -> bool {
    p.x >= 0.0 && p.x <= WIDTH as f32 && p.y >= 0.0 && p.y <= HEIGHT as f32
}

fn attack_hits_player<'a>(
    attack: &Attack,
    mut players: impl Iterator<Item = (&'a player::Meta, &'a player::State)>,
) -> Option<(&'a player::Meta, &'a player::State)> {
    players.find(|(meta, player)| {
        **meta != attack.owner
            && attack.pos.dist(&player.pos) <= ATTACK_RADIUS + PLAYER_RADIUS as f32
    })
}

#[derive(PartialEq)]
pub enum EventRemembrance {
    Remember,
    Forget,
}

pub struct EventManager {
    current_events: StepEvents,
    all_events: Vec<StepEvents>,
    mode: EventRemembrance,
}

impl EventManager {
    pub fn new(mode: EventRemembrance) -> EventManager {
        Self {
            current_events: StepEvents::new(),
            all_events: vec![],
            mode,
        }
    }

    fn remember_events(&self) -> bool {
        self.mode == EventRemembrance::Remember
    }

    pub fn init_round(&mut self, round: u32, players: HashMap<player::Meta, Point>) {
        if self.remember_events() {
            self.all_events.push(self.current_events.clone());
        }
        self.current_events =
            StepEvents::from_slice(&vec![GameEvent::RoundStarted(round, players)]);
    }

    pub fn init_tick(&mut self, tick: u32) {
        if self.remember_events() {
            self.all_events.push(self.current_events.clone());
        }
        // HACK: find a good solution for the first events in a round
        let tick_event = GameEvent::Tick(tick);
        if tick == 0 {
            self.record(tick_event);
        } else {
            self.current_events.events = vec![tick_event];
        }
    }

    pub fn record(&mut self, event: GameEvent) {
        self.current_events.events.push(event);
    }

    pub fn current_events(&self) -> &StepEvents {
        &self.current_events
    }
}

// FIXME: should take attack type or even concrete attack
fn remaining_hp(current_hp: f32) -> f32 {
    current_hp - ATTACK_DAMAGE
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
            if let Some((meta, player_state)) = attack_hits_player(&attack, game.living_players()) {
                // FIXME: new_pos or old position here?
                event_manager.record(GameEvent::Hit(
                    attack.id,
                    attack.owner.clone(),
                    meta.clone(),
                    next_pos,
                ));
                if remaining_hp(player_state.hp) <= 0.0 {
                    event_manager.record(GameEvent::PlayerDied(meta.clone()));
                }
            } else {
                event_manager.record(GameEvent::AttackAdvanced(attack.id, next_pos));
            }
        } else {
            event_manager.record(GameEvent::AttackMissed(attack.id));
        }
    }
}

fn advance_game_state(game: &mut Game, events: &[GameEvent]) {
    for event in events {
        match event {
            GameEvent::Tick(_) => {
                game.tick += 1;
                // FIXME: check whether saving the next tick shooting is
                // possible again might be better; but then again we could not
                // as easily add a Lua getter...
                for player_state in game.players.values_mut() {
                    let cd = player_state.attack_cooldown;
                    if cd > 0 {
                        player_state.attack_cooldown = cd - 1;
                    }
                }
            }
            GameEvent::RoundStarted(_, _) => {}
            GameEvent::RoundEnded(opt_winner) => {
                game.round_state = match opt_winner {
                    Some(winner) => RoundState::Won(winner.clone()),
                    None => RoundState::Draw,
                }
            }
            GameEvent::PlayerPositionUpdated(id, delta) => {
                let d;
                {
                    let player = game.player_state(id);
                    let pos = &mut player.pos;
                    d = pos.dist(&Point::zero()); // TODO: length of a Vec2
                    pos.x += delta.value.x;
                    pos.y += delta.value.y;
                }
                let lua_impl = game.player(id);
                let distance = lua_impl.intent.distance;
                lua_impl.intent.distance = f32::max(distance - d, 0.0);
            }
            GameEvent::PlayerTurned(id, delta) => {
                let player = game.player_state(id);
                let heading = player.heading + *delta;
                player.heading = math_utils::normalize_absolute_angle(heading);
                let lua_impl = game.player(id);
                let turn_angle = lua_impl.intent.turn_angle;
                lua_impl.intent.turn_angle = if turn_angle.abs() < MAX_TURN_RATE {
                    0.0
                } else {
                    turn_angle - *delta
                };
            }
            GameEvent::PlayerHeadTurned(id, delta) => {
                let player = game.player_state(id);
                player.head_heading = player.head_heading + *delta;
                let lua_impl = game.player(id);
                let intended = lua_impl.intent.turn_head_angle;
                lua_impl.intent.turn_head_angle =
                    // FIXME: (here and elsewhere): we don't need this if we
                    // stop using float equality checks I think
                    if intended.abs() < MAX_HEAD_TURN_RATE {
                        0.0
                    } else {
                        intended - *delta
                    };
            }
            GameEvent::PlayerArmsTurned(id, delta) => {
                let player = game.player_state(id);
                player.arms_heading = clamp_turn_angle(player.arms_heading + *delta);
                let lua_impl = game.player(id);
                let intended = lua_impl.intent.turn_arms_angle;
                lua_impl.intent.turn_arms_angle = if intended.abs() < MAX_ARMS_TURN_RATE {
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
                game.player_state(victim_id).hp -= ATTACK_DAMAGE;
            }
            GameEvent::AttackAdvanced(id, pos) => {
                let attack = game.attack(id);
                attack.pos.set_to(&pos);
            }
            GameEvent::AttackMissed(id) => {
                if let Some(index) = game.attacks.iter().position(|attack| attack.id == *id) {
                    game.attacks.remove(index);
                }
            }
            GameEvent::AttackCreated(owner, attack) => {
                game.attacks.push(attack.clone());
                let player = game.player_state(owner);
                player.attack_cooldown = ATTACK_COOLDOWN;
                let lua_impl = game.player(owner);
                lua_impl.intent.attack = false;
            }
            GameEvent::PlayerDied(_) => {}
        }
    }
}

fn check_for_round_end(game: &Game, event_manager: &mut EventManager) {
    let nplayers = game.living_players().count();
    match nplayers {
        0 => {
            println!("none");
            event_manager.record(GameEvent::RoundEnded(None));
        }
        1 => {
            println!("some");
            event_manager.record(GameEvent::RoundEnded(Some(
                game.living_players().nth(0).unwrap().0.clone(),
            )));
        }
        _ => {}
    }
}

fn run_players(game: &mut Game, events: &[GameEvent]) -> Result<(), player::EventError> {
    let player_positions: Vec<(player::Meta, Point)> = game
        .living_players()
        .map(|(meta, p)| (meta.clone(), p.pos.clone()))
        .collect();
    for (meta, player_state) in game.players.iter_mut() {
        let intent = &game.impls.get(meta).unwrap().intent;
        let mut player_events = game_events_to_player_events(meta, player_state, intent, events);
        for (other_meta, pos) in player_positions.iter() {
            if other_meta.id != meta.id {
                if can_spot(
                    &player_state.pos,
                    player_state.effective_head_heading(),
                    &pos,
                    PLAYER_RADIUS as f32,
                    ANGLE_OF_VISION,
                ) {
                    player_events.push(player::Event::EnemySeen(
                        other_meta.name.clone(),
                        pos.clone(),
                    ));
                }
            }
        }
        let player = game.impls.get_mut(meta).unwrap();
        let mut commands = dispatch_player_events(player_events, &mut player.implementation)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                player::Command::Attack => player.intent.attack = true,
                player::Command::Turn(angle) => player.intent.turn_angle = *angle,
                player::Command::TurnHead(angle) => {
                    let current = player_state.head_heading;
                    let next = clamp_turn_angle(current + *angle) - current;
                    player.intent.turn_head_angle = next;
                }
                player::Command::TurnArms(angle) => {
                    let current = player_state.arms_heading;
                    let next = clamp_turn_angle(current + *angle) - current;
                    player.intent.turn_arms_angle = next;
                }
                player::Command::Move(dir, dist) => {
                    player.intent.direction = dir.clone();
                    player.intent.distance = *dist;
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct StepEvents {
    pub events: Vec<GameEvent>,
}

impl StepEvents {
    pub fn new() -> Self {
        Self { events: vec![] }
    }

    pub fn from_slice(evts: &[GameEvent]) -> Self {
        let mut events = Vec::new();
        events.extend_from_slice(&evts);
        Self { events }
    }
}

pub fn step(
    game: &mut Game,
    event_manager: &mut EventManager,
    game_writer: &mpsc::Sender<StepEvents>,
) -> Result<(), GameError> {
    event_manager.init_tick(game.tick);
    check_for_round_end(game, event_manager);
    transition_players(game, event_manager);
    create_attacks(game, event_manager);
    transition_attacks(game, event_manager);

    let step_events: &StepEvents = &event_manager.current_events();
    advance_game_state(game, &step_events.events);
    run_players(game, &step_events.events)?;

    game_writer.send(step_events.clone()).unwrap();
    Ok(())
}

pub fn run_round(
    game: &mut Game,
    round: u32,
    event_manager: &mut EventManager,
    delay: &std::time::Duration,
    game_writer: &mpsc::Sender<StepEvents>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), GameError> {
    game.init_round(round, event_manager);
    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        std::thread::sleep(*delay);
        step(game, event_manager, game_writer)?;
        match game.round_state {
            RoundState::Ongoing => {}
            RoundState::Won(ref meta) => {
                println!("Player {} with ID {} has won!", meta.name, meta.id);
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

pub enum GameError {
    AddPlayerError(AddPlayerError),
    LuaPlayerEventError(player::EventError),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::AddPlayerError(inner) => write!(f, "Player could not be added: {inner}"),
            GameError::LuaPlayerEventError(inner) => {
                write!(f, "Communication with player failed: {inner}")
            }
        }
    }
}

impl From<AddPlayerError> for GameError {
    fn from(err: AddPlayerError) -> Self {
        GameError::AddPlayerError(err)
    }
}

impl From<player::EventError> for GameError {
    fn from(err: player::EventError) -> Self {
        GameError::LuaPlayerEventError(err)
    }
}

pub fn run_game(
    game: &mut Game,
    delay: &std::time::Duration,
    game_writer: mpsc::Sender<StepEvents>,
    cancel: &Arc<AtomicBool>,
) -> Result<(), GameError> {
    let mut event_manager = EventManager::new(EventRemembrance::Forget);
    let max_rounds = 1000;
    for round in 1..max_rounds + 1 {
        if cancel.load(Ordering::Relaxed) {
            println!("Game cancelled");
            break;
        }
        run_round(game, round, &mut event_manager, delay, &game_writer, cancel)?;
    }
    Ok(())
}
