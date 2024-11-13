use core::fmt;

use crate::math_utils::Point;

#[derive(Debug)]
pub enum PlayerEvent {
    Tick(u32),
    RoundStarted(u32),
    EnemySeen(String, Point),
    Death,
    HitBy(u8),
    AttackHit(u8, Point),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MovementDirection {
    Forward,
    Backward,
    Left,
    Right,
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
    pub fn index(&self) -> i32 {
        match self {
            PlayerCommand::Move(_, _) => 0,
            PlayerCommand::Attack => 1,
            PlayerCommand::Turn(_) => 2,
            PlayerCommand::TurnHead(_) => 3,
            PlayerCommand::TurnArms(_) => 4,
        }
    }
}

pub struct PlayerCommands {
    pub value: Vec<PlayerCommand>,
}

impl PlayerCommands {
    pub fn none() -> Self {
        Self { value: vec![] }
    }
}

impl From<Vec<PlayerCommand>> for PlayerCommands {
    fn from(value: Vec<PlayerCommand>) -> Self {
        Self { value }
    }
}

#[derive(Debug)]
pub struct PlayerEventError {
    pub message: String,
}

impl fmt::Display for PlayerEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub trait PlayerImpl {
    fn on_event(&self, event: &PlayerEvent) -> Result<PlayerCommands, PlayerEventError>;
}
