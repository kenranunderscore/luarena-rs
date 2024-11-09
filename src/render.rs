use std::sync::mpsc::Receiver;

use raylib::prelude::*;

use crate::game::{GameEvent, StepEvents};
use crate::math_utils::Point;
use crate::{math_utils, settings::*};

const VISION_COLOR: Color = Color {
    r: 150,
    g: 150,
    b: 250,
    a: 50,
};

struct PlayerData {
    id: u8,
    color: Color,
    x: f32,
    y: f32,
    heading: f32,
    head_heading: f32,
    arms_heading: f32,
}

impl PlayerData {
    fn new(id: u8, color: crate::game::Color, p: &Point) -> Self {
        Self {
            id,
            color: to_raylib_color(&color),
            x: p.x,
            y: p.y,
            heading: 0.0,
            head_heading: 0.0,
            arms_heading: 0.0,
        }
    }
}

struct GameData {
    players: Vec<PlayerData>,
}

impl GameData {
    fn new() -> Self {
        Self {
            players: Vec::new(),
        }
    }

    fn player(&mut self, id: u8) -> &mut PlayerData {
        self.players.iter_mut().find(|p| p.id == id).unwrap()
    }
}

fn draw_line_in_direction(
    d: &mut RaylibDrawHandle,
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

fn player_vision(d: &mut RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    let vision_delta = ANGLE_OF_VISION / 2.0;
    let side_len = (WIDTH + HEIGHT) as f32; // don't know whether this is smart or dumb...
    let origin = Vector2::new(x as f32, y as f32);
    let left_angle = math_utils::normalize_absolute_angle(heading - vision_delta);
    let left = math_utils::line_endpoint(origin.x, origin.y, side_len, left_angle);
    let right_angle = math_utils::normalize_absolute_angle(heading + vision_delta);
    let right = math_utils::line_endpoint(origin.x, origin.y, side_len, right_angle);
    d.draw_triangle(
        Vector2::new(left.x, left.y),
        origin,
        Vector2::new(right.x, right.y),
        VISION_COLOR,
    );
}

fn player_arms(d: &mut RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    draw_line_in_direction(d, x, y, heading, 1.5 * PLAYER_RADIUS as f32, Color::YELLOW);
}

fn heading(d: &mut RaylibDrawHandle, x: i32, y: i32, heading: f32, color: Color) {
    draw_line_in_direction(d, x, y, heading, 1.6 * PLAYER_RADIUS as f32, color);
    draw_line_in_direction(
        d,
        x,
        y,
        heading + PI as f32,
        1.2 * PLAYER_RADIUS as f32,
        color,
    );
    draw_line_in_direction(
        d,
        x,
        y,
        heading + PI as f32 / 2.0,
        1.2 * PLAYER_RADIUS as f32,
        color,
    );
    draw_line_in_direction(
        d,
        x,
        y,
        heading - PI as f32 / 2.0,
        1.2 * PLAYER_RADIUS as f32,
        color,
    );
}

fn to_raylib_color(color: &crate::game::Color) -> Color {
    Color {
        r: color.red,
        g: color.green,
        b: color.blue,
        a: 255,
    }
}

fn players<'a>(d: &mut RaylibDrawHandle, players: impl Iterator<Item = &'a PlayerData>) {
    for player in players {
        let x = player.x.round() as i32;
        let y = player.y.round() as i32;
        player_vision(d, x, y, player.heading + player.head_heading);
        player_arms(d, x, y, player.heading + player.arms_heading);
        heading(d, x, y, player.heading, player.color);
        d.draw_circle(x, y, PLAYER_RADIUS as f32, player.color);
    }
}

fn attack(d: &mut RaylibDrawHandle, attack: &Point) {
    let attack_color = Color::GOLDENROD;
    d.draw_circle(
        attack.x.round() as i32,
        attack.y.round() as i32,
        ATTACK_RADIUS,
        attack_color,
    );
}

fn draw_game(d: &mut RaylibDrawHandle, game_data: &GameData) {
    players(d, game_data.players.iter());
}

pub struct GameRenderer<'a> {
    event_stream: &'a Receiver<StepEvents>,
    state: GameData,
}

impl<'a> GameRenderer<'a> {
    pub fn new(event_stream: &'a Receiver<StepEvents>) -> Self {
        Self {
            event_stream,
            state: GameData::new(),
        }
    }

    fn process_event(&mut self, d: &mut RaylibDrawHandle, event: GameEvent) {
        match event {
            GameEvent::Tick(_) => {}
            GameEvent::RoundStarted(_, players) => {
                self.state.players = vec![];
                for (id, pos, meta) in players {
                    self.state
                        .players
                        .push(PlayerData::new(id, meta.color, &pos));
                }
            }
            GameEvent::RoundOver(_) => {}
            GameEvent::PlayerPositionUpdated(id, delta) => {
                let player = self.state.player(id);
                player.x = player.x + delta.value.x;
                player.y = player.y + delta.value.y;
            }
            GameEvent::PlayerTurned(id, delta) => {
                let player = self.state.player(id);
                player.heading = player.heading + delta;
            }
            GameEvent::PlayerHeadTurned(id, delta) => {
                let player = self.state.player(id);
                player.head_heading = player.head_heading + delta;
            }
            GameEvent::PlayerArmsTurned(id, delta) => {
                let player = self.state.player(id);
                player.arms_heading = player.arms_heading + delta;
            }
            GameEvent::Hit(_, _, _, _) => {}
            GameEvent::AttackAdvanced(_, pos) => attack(d, &pos),
            GameEvent::AttackMissed(_) => {}
            GameEvent::AttackCreated(_, a) => attack(d, &a.pos),
        }
    }

    pub fn step(&mut self, rl: &mut RaylibHandle, rl_thread: &RaylibThread) {
        let mut d = rl.begin_drawing(rl_thread);
        match self.event_stream.try_recv() {
            Ok(step_events) => {
                for event in step_events.events.into_iter() {
                    self.process_event(&mut d, event);
                }

                d.draw_fps(5, 5);
                d.clear_background(raylib::prelude::Color::BLACK);
                draw_game(&mut d, &self.state);
            }
            // Sender is gone, which is expected if the game has ended
            Err(_) => {}
        }
    }
}
