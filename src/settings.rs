use crate::math_utils::HALF_PI;

pub const INITIAL_HP: f32 = 100.0;
pub const MAX_TURN_RATE: f32 = 0.05;
pub const MAX_HEAD_TURN_RATE: f32 = 0.1;
pub const MAX_ARMS_TURN_RATE: f32 = 0.08;
pub const ANGLE_OF_VISION: f32 = 0.9 * HALF_PI;
pub const ANGLE_OF_ACTION: f32 = HALF_PI;
pub const CHARACTER_RADIUS: f32 = 25.0;
pub const ATTACK_RADIUS: f32 = 4.0;
pub const ATTACK_DAMAGE: f32 = 10.0;
pub const ATTACK_COOLDOWN: u8 = 35;
pub const WIDTH: i32 = 1600;
pub const HEIGHT: i32 = 1200;
pub const MAX_VELOCITY: f32 = 1.0;
