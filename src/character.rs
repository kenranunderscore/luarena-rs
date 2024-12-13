use core::fmt;
use std::path::{Path, PathBuf};

pub mod lua;
pub mod meta;
pub mod wasm;

pub use meta::*;

use crate::{
    color::Color,
    math_utils::{self, Point},
    settings,
};

#[derive(PartialEq, Eq, Hash, Clone, Debug, Copy)]
pub struct Id(pub uuid::Uuid);

impl From<uuid::Uuid> for Id {
    fn from(value: uuid::Uuid) -> Self {
        Self(value)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Intent {
    pub direction: MovementDirection,
    pub distance: f32,
    pub attack: bool,
    pub turn_angle: f32,
    pub turn_head_angle: f32,
    pub turn_arms_angle: f32,
}

impl Default for Intent {
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

pub struct Character {
    pub implementation: Box<dyn Impl>,
    pub intent: Intent,
}

impl Character {
    pub fn new(implementation: Box<dyn Impl>) -> Self {
        Self {
            implementation,
            intent: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct CurrentCharacterState {
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
    pub attack_cooldown: u8,
    pub turn_remaining: f32,
    pub head_turn_remaining: f32,
    pub arms_turn_remaining: f32,
}

impl CurrentCharacterState {
    pub fn from_state(state: &State, intent: &Intent) -> Self {
        Self {
            x: state.pos.x,
            y: state.pos.y,
            hp: state.hp,
            heading: state.heading,
            head_heading: state.head_heading,
            arms_heading: state.arms_heading,
            attack_cooldown: state.attack_cooldown,
            turn_remaining: intent.turn_angle,
            head_turn_remaining: intent.turn_head_angle,
            arms_turn_remaining: intent.turn_arms_angle,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Tick(u32, CurrentCharacterState),
    RoundStarted(u32),
    RoundEnded(Option<Meta>),
    RoundDrawn,
    RoundWon,
    EnemySeen(String, Point),
    Death,
    EnemyDied(String),
    HitBy(Meta),
    AttackHit(Meta, Point),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MovementDirection {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(PartialEq, Debug)]
pub enum Command {
    Move(MovementDirection, f32),
    Attack,
    Turn(f32),
    TurnHead(f32),
    TurnArms(f32),
}

impl Command {
    pub fn index(&self) -> i32 {
        match self {
            Command::Move(_, _) => 0,
            Command::Attack => 1,
            Command::Turn(_) => 2,
            Command::TurnHead(_) => 3,
            Command::TurnArms(_) => 4,
        }
    }
}

pub struct Commands {
    pub value: Vec<Command>,
}

impl Commands {
    pub fn none() -> Self {
        Self { value: vec![] }
    }
}

impl From<Vec<Command>> for Commands {
    fn from(value: Vec<Command>) -> Self {
        Self { value }
    }
}

#[derive(Debug)]
pub struct EventError {
    pub message: String,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub trait Impl {
    fn on_event(&mut self, event: &Event) -> Result<Commands, EventError>;
}

pub struct Stats {
    pub rounds_won: u32,
}

pub struct State {
    pub hp: f32,
    pub pos: Point,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
    pub attack_cooldown: u8,
    pub stats: Stats,
}

impl State {
    pub fn new() -> Self {
        Self {
            hp: (settings::INITIAL_HP),
            pos: Point::zero(),
            heading: 0.0,
            head_heading: 0.0,
            arms_heading: 0.0,
            attack_cooldown: 0,
            stats: Stats { rounds_won: 0 },
        }
    }

    // TODO: also randomize headings?
    pub fn reset(&mut self, next_pos: Point) {
        self.hp = settings::INITIAL_HP;
        self.heading = 0.0;
        self.head_heading = 0.0;
        self.arms_heading = 0.0;
        self.pos = next_pos;
    }

    pub fn effective_head_heading(&self) -> f32 {
        math_utils::normalize_absolute_angle(self.heading + self.head_heading)
    }

    pub fn effective_arms_heading(&self) -> f32 {
        math_utils::normalize_absolute_angle(self.heading + self.arms_heading)
    }

    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }
}

pub fn log_msg(character_name: &str, msg: &str) {
    println!("[{}]: {msg}", character_name);
}
