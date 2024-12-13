use core::fmt;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use rand::rngs::ThreadRng;
use rand::Rng;

use crate::character::{self, Character, MovementDirection};
use crate::config::BattleConfiguration;
use crate::math_utils::{self, Point, Sector, HALF_PI};
use crate::settings::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct AttackId(usize);

#[derive(Clone, Debug)]
pub struct Attack {
    pub id: AttackId,
    pub pos: Point,
    pub owner: character::Meta,
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
    Won(character::Meta),
    Draw,
}

#[derive(Debug, Clone, Copy)]
pub struct Round(u16);

#[derive(Debug, Clone, Copy)]
pub struct Tick(u32);

impl Tick {
    pub fn advance(&mut self) {
        self.0 += 1;
    }
}

pub struct Game {
    pub tick: Tick,
    pub round: Round,
    pub characters: HashMap<character::Meta, character::State>,
    pub impls: HashMap<character::Meta, Character>,
    pub attacks: Vec<Attack>,
    pub round_state: RoundState,
    attack_ids: AttackIds,
}

#[derive(Debug)]
pub struct AddCharacterError(pub String);

impl fmt::Display for AddCharacterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<mlua::Error> for AddCharacterError {
    fn from(err: mlua::Error) -> Self {
        Self(format!("{err}"))
    }
}

impl Game {
    pub fn new() -> Game {
        Self {
            tick: Tick(0),
            round: Round(1),
            characters: HashMap::new(),
            impls: HashMap::new(),
            attacks: vec![],
            attack_ids: AttackIds::new(),
            round_state: RoundState::Ongoing,
        }
    }

    pub fn with_characters(character_dirs: &[PathBuf]) -> Result<Self, AddCharacterError> {
        let mut game = Self::new();
        game.add_characters(character_dirs)?;
        Ok(game)
    }

    fn add_characters(&mut self, character_dirs: &[PathBuf]) -> Result<(), AddCharacterError> {
        for dir in character_dirs.iter() {
            self.add_character(dir)?;
        }
        Ok(())
    }

    fn add_character(&mut self, character_dir: &Path) -> Result<(), AddCharacterError> {
        let mut meta = character::Meta::from_toml_file(&character_dir.join("meta.toml"))
            .map_err(|e| AddCharacterError(e.0))?;
        let character_state = character::State::new();
        if self.impls.contains_key(&meta) {
            meta.instance += 1;
        }
        let extension = meta.entrypoint.extension().and_then(|s| s.to_str());
        let implementation = match extension {
            Some("lua") => character::lua::LuaImpl::load(character_dir, &meta)
                .map_err(|e| AddCharacterError(e.to_string()))
                .map(|character_impl| Box::new(character_impl) as Box<dyn character::Impl>)?,
            Some("wasm") => character::wasm::WasmImpl::load(character_dir, &meta)
                .map_err(|e| AddCharacterError(e.message))
                .map(|character_impl| Box::new(character_impl) as Box<dyn character::Impl>)?,
            Some(unexpected) => {
                return Err(AddCharacterError(format!(
                    "Unexpected entrypoint extension: {unexpected}"
                )))
            }
            None => {
                return Err(AddCharacterError(
                    "Entrypoint extension undetectable".to_string(),
                ))
            }
        };
        self.impls
            .insert(meta.clone(), Character::new(implementation));
        self.characters.insert(meta, character_state);
        Ok(())
    }

    pub fn init_round(&mut self, round: Round, event_manager: &mut EventManager) {
        self.tick = Tick(0);
        self.round = round;
        self.round_state = RoundState::Ongoing;
        self.attacks = vec![];
        let mut characters = HashMap::new();
        let rng = rand::thread_rng();
        let randomized_positions = random_positions(self.characters.len(), rng);
        for ((meta, character_state), p) in
            self.characters.iter_mut().zip(randomized_positions.iter())
        {
            // FIXME: it would be nice to change state later (as with the rest),
            // but that creates problems with the check for "round over"
            character_state.reset(p.clone());
            characters.insert(meta.clone(), p.clone());
        }
        event_manager.init_round(round, characters);
        for (_, character) in self.impls.iter_mut() {
            character.intent = Default::default();
        }
    }

    pub fn living_characters(&self) -> impl Iterator<Item = (&character::Meta, &character::State)> {
        self.characters.iter().filter(|(_, p)| p.alive())
    }

    pub fn character_state(&mut self, meta: &character::Meta) -> &mut character::State {
        self.characters
            .iter_mut()
            .find(|(m, _)| *m == meta)
            .expect(&format!("character {} not found", meta.display_name()))
            .1
    }

    pub fn attack(&mut self, id: &AttackId) -> &mut Attack {
        self.attacks
            .iter_mut()
            .find(|attack| attack.id == *id)
            .expect("attack {id} not found")
    }

    pub fn character(&mut self, meta: &character::Meta) -> &mut Character {
        self.impls.get_mut(meta).unwrap()
    }

    pub fn print_stats(&self) {
        println!("  Rounds won:");
        for (meta, character_state) in self.characters.iter() {
            let stats = &character_state.stats;
            println!(
                "    {}: {} rounds won",
                meta.display_name(),
                stats.rounds_won
            );
        }
    }
}

fn random_positions(n: usize, mut rng: ThreadRng) -> Vec<Point> {
    let wall_dist = 20.0;
    let min = CHARACTER_RADIUS + wall_dist;
    let max_x = WIDTH as f32 - CHARACTER_RADIUS - wall_dist;
    let max_y = HEIGHT as f32 - CHARACTER_RADIUS - wall_dist;
    let mut positions = vec![];
    for _i in 0..n {
        loop {
            let new_p = Point {
                x: rng.gen_range(min..max_x) as f32,
                y: rng.gen_range(min..max_y) as f32,
            };
            if !positions.iter().any(|p| characters_collide(p, &new_p)) {
                positions.push(new_p);
                break;
            }
        }
    }
    positions
}

fn reduce_commands(commands: &mut Vec<character::Command>) {
    // FIXME: check whether they're really reduced
    commands.sort_by_key(|cmd| cmd.index());
}

fn valid_position(p: &Point) -> bool {
    p.x >= CHARACTER_RADIUS
        && p.x <= WIDTH as f32 - CHARACTER_RADIUS
        && p.y >= CHARACTER_RADIUS
        && p.y <= HEIGHT as f32 - CHARACTER_RADIUS
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

// TODO: use struct variants maybe
#[derive(Clone, Debug)]
pub enum GameEvent {
    Tick(Tick),
    RoundStarted(Round, HashMap<character::Meta, Point>),
    RoundEnded(Option<character::Meta>),
    CharacterHeadTurned(character::Meta, f32),
    CharacterArmsTurned(character::Meta, f32),
    Hit(AttackId, character::Meta, character::Meta, Point),
    AttackAdvanced(AttackId, Point),
    AttackMissed(AttackId),
    // FIXME: does this really need the owner id twice?
    AttackCreated(character::Meta, Attack),
    CharacterPositionUpdated(character::Meta, Delta),
    CharacterTurned(character::Meta, f32),
    CharacterDied(character::Meta),
}

fn clamp_turn_angle(angle: f32) -> f32 {
    math_utils::clamp(angle, -ANGLE_OF_ACTION, ANGLE_OF_ACTION)
}

fn transition_characters(game: &mut Game, event_manager: &mut EventManager) {
    // TODO: is a HashMap appropriate here? is there a smarter way?
    let mut next_positions: HashMap<character::Meta, (Delta, Point)> = HashMap::new();
    for (meta, character_state) in game.living_characters() {
        let character = game.impls.get(&meta).unwrap();
        let delta = math_utils::clamp(character.intent.turn_angle, -MAX_TURN_RATE, MAX_TURN_RATE);
        event_manager.record(GameEvent::CharacterTurned(meta.clone(), delta));
        let heading = math_utils::normalize_absolute_angle(character_state.heading + delta);
        let velocity = f32::min(character.intent.distance, MAX_VELOCITY);
        let dir_heading = match character.intent.direction {
            MovementDirection::Forward => 0.0,
            MovementDirection::Backward => math_utils::PI,
            MovementDirection::Left => -HALF_PI,
            MovementDirection::Right => HALF_PI,
        };
        let movement_heading = heading + dir_heading;
        let dx = movement_heading.sin() * velocity;
        let dy = -movement_heading.cos() * velocity;
        let delta = Delta::new(Point { x: dx, y: dy });
        let pos = &character_state.pos;
        let next_pos = pos.add(&delta.value);
        if valid_position(&next_pos) {
            next_positions.insert(meta.clone(), (delta, next_pos));
        } else {
            next_positions.insert(meta.clone(), (Delta::new(Point::zero()), pos.clone()));
        };

        transition_heads(
            &meta,
            character_state,
            character.intent.turn_head_angle,
            event_manager,
        );
        transition_arms(
            &meta,
            character_state,
            character.intent.turn_arms_angle,
            event_manager,
        );
    }

    for (meta, _) in game.living_characters() {
        let (delta, next) = next_positions.get(&meta).unwrap();
        let mut collides = false;
        for (other_meta, (_, other_next)) in next_positions.iter() {
            if meta != other_meta {
                if characters_collide(&next, &other_next) {
                    // TODO: collision event
                    collides = true;
                }
            }
        }
        let event = if !collides {
            GameEvent::CharacterPositionUpdated(meta.clone(), delta.clone())
        } else {
            GameEvent::CharacterPositionUpdated(meta.clone(), Delta::new(Point::zero()))
        };
        event_manager.record(event);
    }
}

fn transition_heads(
    meta: &character::Meta,
    character_state: &character::State,
    turn_angle: f32,
    event_manager: &mut EventManager,
) {
    let delta = math_utils::clamp(turn_angle, -MAX_HEAD_TURN_RATE, MAX_HEAD_TURN_RATE);
    let current_heading = character_state.head_heading;
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::CharacterHeadTurned(
        meta.clone(),
        effective_delta,
    ));
}

fn transition_arms(
    meta: &character::Meta,
    character_state: &character::State,
    turn_angle: f32,
    event_manager: &mut EventManager,
) {
    let delta = math_utils::clamp(turn_angle, -MAX_ARMS_TURN_RATE, MAX_ARMS_TURN_RATE);
    let current_heading = character_state.arms_heading;
    let effective_delta = clamp_turn_angle(current_heading + delta) - current_heading;
    event_manager.record(GameEvent::CharacterArmsTurned(
        meta.clone(),
        effective_delta,
    ));
}

fn characters_collide(p: &Point, q: &Point) -> bool {
    p.dist(q) <= 2.0 * (CHARACTER_RADIUS as f32)
}

fn game_events_to_character_events(
    meta: &character::Meta,
    character_state: &character::State,
    intent: &character::Intent,
    game_events: &[GameEvent],
) -> Vec<character::Event> {
    let mut character_events = Vec::new();
    for event in game_events.iter() {
        match event {
            GameEvent::Tick(tick) => {
                character_events.push(character::Event::Tick(
                    tick.0,
                    character::CurrentCharacterState::from_state(&character_state, &intent),
                ));
            }
            GameEvent::RoundStarted(round, _) => {
                character_events.push(character::Event::RoundStarted(round.0));
            }
            GameEvent::RoundEnded(opt_meta) => {
                character_events.push(character::Event::RoundEnded(opt_meta.clone()));
                match opt_meta {
                    Some(winner_meta) => {
                        if winner_meta == meta {
                            character_events.push(character::Event::RoundWon);
                        }
                    }
                    None => character_events.push(character::Event::RoundDrawn),
                }
            }
            GameEvent::CharacterTurned(_, _) => {}
            GameEvent::CharacterPositionUpdated(_, _) => {}
            GameEvent::CharacterHeadTurned(_, _) => {}
            GameEvent::CharacterArmsTurned(_, _) => {}
            GameEvent::Hit(_, owner_meta, victim_meta, pos) => {
                if meta == victim_meta {
                    // FIXME: don't use id
                    character_events.push(character::Event::HitBy(owner_meta.clone()));
                } else if meta == owner_meta {
                    character_events.push(character::Event::AttackHit(
                        victim_meta.clone(),
                        pos.clone(),
                    ));
                }
            }
            GameEvent::AttackAdvanced(_, _) => {}
            GameEvent::AttackMissed(_) => {}
            GameEvent::AttackCreated(_, _) => {}
            GameEvent::CharacterDied(deceased_meta) => {
                let death_event = if meta == deceased_meta {
                    character::Event::Death
                } else {
                    character::Event::EnemyDied(deceased_meta.name.clone())
                };
                character_events.push(death_event);
            }
        }
    }
    character_events
}

fn can_spot(
    origin: &Point,
    view_angle: f32,
    target: &Point,
    character_radius: f32,
    angle_of_vision: f32,
) -> bool {
    let view_sector = Sector::new(view_angle, angle_of_vision / 2.0);
    let d = origin.dist(target);
    let alpha = f32::atan(character_radius / d);
    let angle = math_utils::normalize_absolute_angle(math_utils::angle_between(origin, target));
    let target_sector = Sector::new(angle, alpha);
    view_sector.overlaps(&target_sector)
}

fn dispatch_character_events(
    character_events: Vec<character::Event>,
    character: &mut Box<dyn character::Impl>,
) -> Result<Vec<character::Command>, character::EventError> {
    let mut commands = Vec::new();
    for e in character_events.iter() {
        commands.append(&mut character.on_event(&e)?.value);
    }
    Ok(commands)
}

fn create_attacks(game: &mut Game, event_manager: &mut EventManager) {
    for (meta, character_state) in game.characters.iter() {
        if !character_state.alive() {
            continue;
        }
        let character = game.impls.get(meta).unwrap();
        let will_attack = character.intent.attack && character_state.attack_cooldown == 0;
        if will_attack {
            let attack = Attack {
                id: game.attack_ids.next(),
                owner: meta.clone(),
                pos: character_state.pos.clone(),
                velocity: 2.5,
                heading: character_state.effective_arms_heading(),
            };
            event_manager.record(GameEvent::AttackCreated(meta.clone(), attack));
        }
    }
}

fn inside_arena(p: &Point) -> bool {
    p.x >= 0.0 && p.x <= WIDTH as f32 && p.y >= 0.0 && p.y <= HEIGHT as f32
}

fn attack_hits_character<'a>(
    attack: &Attack,
    mut characters: impl Iterator<Item = (&'a character::Meta, &'a character::State)>,
) -> Option<(&'a character::Meta, &'a character::State)> {
    characters.find(|(meta, character)| {
        **meta != attack.owner
            && attack.pos.dist(&character.pos) <= ATTACK_RADIUS + CHARACTER_RADIUS as f32
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

    pub fn init_round(&mut self, round: Round, characters: HashMap<character::Meta, Point>) {
        if self.remember_events() {
            self.all_events.push(self.current_events.clone());
        }
        self.current_events =
            StepEvents::from_slice(&vec![GameEvent::RoundStarted(round, characters)]);
    }

    pub fn init_tick(&mut self, tick: Tick) {
        if self.remember_events() {
            self.all_events.push(self.current_events.clone());
        }
        // HACK: find a good solution for the first events in a round
        let tick_event = GameEvent::Tick(tick);
        if tick.0 == 0 {
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
            if let Some((meta, character_state)) =
                attack_hits_character(&attack, game.living_characters())
            {
                // FIXME: new_pos or old position here?
                event_manager.record(GameEvent::Hit(
                    attack.id,
                    attack.owner.clone(),
                    meta.clone(),
                    next_pos,
                ));
                if remaining_hp(character_state.hp) <= 0.0 {
                    event_manager.record(GameEvent::CharacterDied(meta.clone()));
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
                game.tick.advance();
                // FIXME: check whether saving the next tick shooting is
                // possible again might be better; but then again we could not
                // as easily add a Lua getter...
                for character_state in game.characters.values_mut() {
                    let cd = character_state.attack_cooldown;
                    if cd > 0 {
                        character_state.attack_cooldown = cd - 1;
                    }
                }
            }
            GameEvent::RoundStarted(_, _) => {}
            GameEvent::RoundEnded(opt_winner) => match opt_winner {
                Some(winner) => {
                    game.round_state = RoundState::Won(winner.clone());
                    game.character_state(winner).stats.rounds_won += 1;
                }
                None => game.round_state = RoundState::Draw,
            },
            GameEvent::CharacterPositionUpdated(id, delta) => {
                let d;
                {
                    let character = game.character_state(id);
                    let pos = &mut character.pos;
                    d = pos.dist(&Point::zero()); // TODO: length of a Vec2
                    pos.x += delta.value.x;
                    pos.y += delta.value.y;
                }
                let lua_impl = game.character(id);
                let distance = lua_impl.intent.distance;
                lua_impl.intent.distance = f32::max(distance - d, 0.0);
            }
            GameEvent::CharacterTurned(id, delta) => {
                let character = game.character_state(id);
                let heading = character.heading + *delta;
                character.heading = math_utils::normalize_absolute_angle(heading);
                let lua_impl = game.character(id);
                let turn_angle = lua_impl.intent.turn_angle;
                lua_impl.intent.turn_angle = if turn_angle.abs() < MAX_TURN_RATE {
                    0.0
                } else {
                    turn_angle - *delta
                };
            }
            GameEvent::CharacterHeadTurned(id, delta) => {
                let character = game.character_state(id);
                character.head_heading = character.head_heading + *delta;
                let lua_impl = game.character(id);
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
            GameEvent::CharacterArmsTurned(id, delta) => {
                let character = game.character_state(id);
                character.arms_heading = clamp_turn_angle(character.arms_heading + *delta);
                let lua_impl = game.character(id);
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
                game.character_state(victim_id).hp -= ATTACK_DAMAGE;
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
                let character = game.character_state(owner);
                character.attack_cooldown = ATTACK_COOLDOWN;
                let lua_impl = game.character(owner);
                lua_impl.intent.attack = false;
            }
            GameEvent::CharacterDied(_) => {}
        }
    }
}

fn check_for_round_end(game: &Game, event_manager: &mut EventManager) {
    let ncharacters = game.living_characters().count();
    match ncharacters {
        0 => event_manager.record(GameEvent::RoundEnded(None)),
        1 => event_manager.record(GameEvent::RoundEnded(Some(
            game.living_characters().nth(0).unwrap().0.clone(),
        ))),
        _ => {}
    }
}

fn run_characters(game: &mut Game, events: &[GameEvent]) -> Result<(), character::EventError> {
    let character_positions: Vec<(character::Meta, Point)> = game
        .living_characters()
        .map(|(meta, p)| (meta.clone(), p.pos.clone()))
        .collect();
    for (meta, character_state) in game.characters.iter_mut() {
        let intent = &game.impls.get(meta).unwrap().intent;
        let mut character_events =
            game_events_to_character_events(meta, character_state, intent, events);
        for (other_meta, pos) in character_positions.iter() {
            if other_meta != meta {
                if can_spot(
                    &character_state.pos,
                    character_state.effective_head_heading(),
                    &pos,
                    CHARACTER_RADIUS as f32,
                    ANGLE_OF_VISION,
                ) {
                    character_events.push(character::Event::EnemySeen(
                        other_meta.name.clone(),
                        pos.clone(),
                    ));
                }
            }
        }
        let character = game.impls.get_mut(meta).unwrap();
        let mut commands =
            dispatch_character_events(character_events, &mut character.implementation)?;
        reduce_commands(&mut commands);
        for cmd in commands.iter() {
            match cmd {
                character::Command::Attack => character.intent.attack = true,
                character::Command::Turn(angle) => character.intent.turn_angle = *angle,
                character::Command::TurnHead(angle) => {
                    let current = character_state.head_heading;
                    let next = clamp_turn_angle(current + *angle) - current;
                    character.intent.turn_head_angle = next;
                }
                character::Command::TurnArms(angle) => {
                    let current = character_state.arms_heading;
                    let next = clamp_turn_angle(current + *angle) - current;
                    character.intent.turn_arms_angle = next;
                }
                character::Command::Move(dir, dist) => {
                    character.intent.direction = dir.clone();
                    character.intent.distance = *dist;
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
    // FIXME: use custom type and pass in different, compile time-known
    // implementations
    game_writer: Option<&mpsc::Sender<StepEvents>>,
) -> Result<(), GameError> {
    event_manager.init_tick(game.tick);
    check_for_round_end(game, event_manager);
    transition_characters(game, event_manager);
    create_attacks(game, event_manager);
    transition_attacks(game, event_manager);

    let step_events: &StepEvents = &event_manager.current_events();
    advance_game_state(game, &step_events.events);
    run_characters(game, &step_events.events)?;

    if let Some(writer) = game_writer {
        writer.send(step_events.clone()).unwrap();
    }
    Ok(())
}

pub fn run_round(
    game: &mut Game,
    round: Round,
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
        step(game, event_manager, Some(game_writer))?;
        match game.round_state {
            RoundState::Ongoing => {}
            RoundState::Won(ref meta) => {
                println!(
                    "Character {} (ID {}) has won!",
                    meta.display_name(),
                    meta.id
                );
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

#[derive(Debug)]
pub enum GameError {
    AddCharacterError(AddCharacterError),
    LuaCharacterEventError(character::EventError),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::AddCharacterError(inner) => {
                write!(f, "Character could not be added: {inner}")
            }
            GameError::LuaCharacterEventError(inner) => {
                write!(f, "Communication with character failed: {inner}")
            }
        }
    }
}

impl From<AddCharacterError> for GameError {
    fn from(err: AddCharacterError) -> Self {
        GameError::AddCharacterError(err)
    }
}

impl From<character::EventError> for GameError {
    fn from(err: character::EventError) -> Self {
        GameError::LuaCharacterEventError(err)
    }
}

pub fn run_game(
    game: &mut Game,
    battle_configuration: BattleConfiguration,
    delay: &std::time::Duration,
    game_writer: mpsc::Sender<StepEvents>,
    cancel: Arc<AtomicBool>,
) -> Result<(), GameError> {
    let mut event_manager = EventManager::new(EventRemembrance::Forget);
    for round in 1..=battle_configuration.rounds {
        if cancel.load(Ordering::Relaxed) {
            println!("Game cancelled");
            break;
        }
        run_round(
            game,
            Round(round),
            &mut event_manager,
            delay,
            &game_writer,
            &cancel,
        )?;
    }
    println!("GAME OVER");
    game.print_stats();
    Ok(())
}

pub fn run_round_headless(
    game: &mut Game,
    round: Round,
    event_manager: &mut EventManager,
) -> Result<(), GameError> {
    game.init_round(round, event_manager);
    loop {
        step(game, event_manager, None)?;
        match game.round_state {
            RoundState::Ongoing => {}
            RoundState::Won(ref meta) => {
                println!(
                    "Character {} (ID {}) has won!",
                    meta.display_name(),
                    meta.id
                );
                break;
            }
            RoundState::Draw => {
                println!("--- DRAW ---");
                break;
            }
        }
    }
    println!("GAME OVER");
    game.print_stats();
    Ok(())
}

pub fn run_game_headless(
    game: &mut Game,
    battle_configuration: BattleConfiguration,
) -> Result<(), GameError> {
    let mut event_manager = EventManager::new(EventRemembrance::Forget);
    for round in 1..=battle_configuration.rounds {
        run_round_headless(game, Round(round), &mut event_manager)?;
    }
    Ok(())
}
