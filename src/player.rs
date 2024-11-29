use core::fmt;
use std::sync::{Arc, RwLock};

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

#[derive(Debug, Clone)]
pub struct Meta {
    pub id: Id,
    pub name: String,
    pub color: Color,
    pub _version: String,
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

pub struct Player {
    pub implementation: Box<dyn Impl>,
    pub intent: ReadableFromImpl<Intent>,
}

impl Player {
    pub fn intent(&self) -> std::sync::RwLockReadGuard<Intent> {
        self.intent.read().unwrap()
    }
}

#[derive(Debug)]
pub enum Event {
    Tick(u32),
    RoundStarted(u32),
    RoundEnded(Option<String>),
    RoundDrawn,
    RoundWon,
    EnemySeen(String, Point),
    Death,
    EnemyDied(String),
    HitBy(Id),
    AttackHit(Id, Point),
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

pub type ReadableFromImpl<T> = Arc<RwLock<T>>;

pub struct State {
    pub hp: ReadableFromImpl<f32>,
    pub meta: Meta,
    pub pos: ReadableFromImpl<Point>,
    pub heading: ReadableFromImpl<f32>,
    pub head_heading: ReadableFromImpl<f32>,
    pub arms_heading: ReadableFromImpl<f32>,
    pub attack_cooldown: ReadableFromImpl<u8>,
}

impl State {
    pub fn new(meta: Meta) -> Self {
        Self {
            meta,
            hp: Arc::new(RwLock::new(settings::INITIAL_HP)),
            pos: Arc::new(RwLock::new(Point::zero())),
            heading: Arc::new(RwLock::new(0.0)),
            head_heading: Arc::new(RwLock::new(0.0)),
            arms_heading: Arc::new(RwLock::new(0.0)),
            attack_cooldown: Arc::new(RwLock::new(0)),
        }
    }

    // TODO: also randomize headings?
    pub fn reset(&mut self, next_pos: Point) {
        *self.hp.write().unwrap() = settings::INITIAL_HP;
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

    pub fn id(&self) -> &Id {
        &self.meta.id
    }

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

    pub fn pos(&self) -> std::sync::RwLockReadGuard<Point> {
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
