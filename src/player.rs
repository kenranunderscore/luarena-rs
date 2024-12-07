use core::fmt;
use std::path::Path;

pub mod lua;
pub mod wasm;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Meta {
    pub id: Id,
    pub name: String,
    pub color: Color,
    pub version: String,
    // TODO: do this properly (by nesting types)
    pub instance: u8,
}

#[derive(Debug)]
pub struct LoadMetaError(pub String);

impl Meta {
    const DEFAULT_COLOR: Color = Color {
        red: 100,
        green: 100,
        blue: 100,
    };

    pub fn display_name(&self) -> String {
        let instance_counter = if self.instance == 1 {
            String::new()
        } else {
            format!(" ({})", self.instance)
        };
        format!("{}_{}{}", self.name, self.version, instance_counter)
    }

    // FIXME: add proper error handling and refactor
    fn from_toml_str(toml: &str) -> Result<Self, LoadMetaError> {
        let table = toml
            .parse::<toml::Table>()
            .map_err(|_| LoadMetaError("Could not parse TOML table".to_string()))?;
        let name = table["name"].as_str().unwrap().to_string();
        let raw_id = table["id"].as_str().unwrap();
        let id = uuid::Uuid::parse_str(raw_id)
            .map_err(|_| LoadMetaError(format!("expected valid UUID, got {raw_id}")))?
            .into();
        let version = table
            .get("version")
            .map_or("1.0", |v| v.as_str().unwrap_or("1.0"))
            .to_string();
        let color = table.get("color").map_or(Self::DEFAULT_COLOR, |c| {
            c.as_table()
                .map(|color_table| Color {
                    red: color_table["red"].as_integer().unwrap() as u8,
                    green: color_table["green"].as_integer().unwrap() as u8,
                    blue: color_table["blue"].as_integer().unwrap() as u8,
                })
                .unwrap_or(Self::DEFAULT_COLOR)
        });
        Ok(Self {
            name,
            id,
            version,
            color,
            instance: 1,
        })
    }

    pub fn from_toml_file(path: &Path) -> Result<Self, LoadMetaError> {
        let contents = std::fs::read_to_string(path).map_err(|e| LoadMetaError(e.to_string()))?;
        Self::from_toml_str(&contents)
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

pub struct Player {
    pub implementation: Box<dyn Impl>,
    pub intent: Intent,
}

#[derive(Debug)]
pub struct CurrentPlayerState {
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

impl CurrentPlayerState {
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
    Tick(u32, CurrentPlayerState),
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

pub struct State {
    pub hp: f32,
    pub pos: Point,
    pub heading: f32,
    pub head_heading: f32,
    pub arms_heading: f32,
    pub attack_cooldown: u8,
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

pub fn log_msg(player_name: &str, msg: &str) {
    println!("[{}]: {msg}", player_name);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod meta {
        use super::*;

        #[test]
        fn can_be_loaded_from_toml_string() {
            let toml_str = "
name = \"Kai\"
id = \"00000000-0000-0000-0000-000000000000\"
version = \"1.09c\"
[color]
red = 243
green = 0
blue = 13
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.name, "Kai");
            assert_eq!(
                meta.id,
                Id(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap())
            );
            assert_eq!(meta.version, "1.09c");
            assert_eq!(
                meta.color,
                Color {
                    red: 243,
                    green: 0,
                    blue: 13
                }
            );
        }

        #[test]
        fn version_has_default_value() {
            // TODO: add `Version` implementing Default
            let toml_str = "
name = \"Kai\"
id = \"00000000-0000-0000-0000-000000000000\"
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.version, "1.0");
        }

        #[test]
        fn color_has_default_value() {
            // TODO: add `PlayerColor` implementing Default
            let toml_str = "
name = \"Kai\"
id = \"00000000-0000-0000-0000-000000000000\"
";
            let meta = Meta::from_toml_str(toml_str).unwrap();
            assert_eq!(meta.color, Meta::DEFAULT_COLOR);
        }
    }
}
