use raylib::prelude::*;

use crate::game::{Attack, Player};
use crate::{math_utils, settings::*, GameState};

const VISION_COLOR: Color = Color {
    r: 150,
    g: 150,
    b: 250,
    a: 50,
};

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

fn player_vision(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    let vision_delta = ANGLE_OF_VISION / 2.0;
    let side_len = (WIDTH + HEIGHT) as f32; // don't know whether this is smart or dumb...
    let origin = Vector2::new(x as f32, y as f32);
    let left_angle = math_utils::normalize_abs_angle(heading - vision_delta);
    let (lx, ly) = math_utils::line_endpoint(origin.x, origin.y, side_len, left_angle);
    let right_angle = math_utils::normalize_abs_angle(heading + vision_delta);
    let (rx, ry) = math_utils::line_endpoint(origin.x, origin.y, side_len, right_angle);
    d.draw_triangle(
        Vector2::new(lx, ly),
        origin,
        Vector2::new(rx, ry),
        VISION_COLOR,
    );
}

fn player_arms(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    draw_line_in_direction(d, x, y, heading, 1.5 * PLAYER_RADIUS as f32, Color::YELLOW);
}

fn heading(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32, color: Color) {
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

pub fn players(d: &mut raylib::drawing::RaylibDrawHandle, players: &Vec<Player>) {
    for player in players {
        let pos = player.pos.borrow();
        let player_color = to_raylib_color(&player.meta.color);
        player_vision(d, pos.x, pos.y, player.effective_head_heading());
        player_arms(d, pos.x, pos.y, player.effective_arms_heading());
        heading(d, pos.x, pos.y, player.heading, player_color);
        d.draw_circle(pos.x, pos.y, PLAYER_RADIUS as f32, player_color);
    }
}

pub fn attacks(d: &mut raylib::drawing::RaylibDrawHandle, attacks: &Vec<Attack>) {
    for attack in attacks {
        let attack_color = Color::RED;
        d.draw_circle(attack.pos.x, attack.pos.y, ATTACK_RADIUS, attack_color);
    }
}

pub fn game(d: &mut raylib::drawing::RaylibDrawHandle, state: &GameState) {
    attacks(d, &state.attacks);
    players(d, &state.players);
}
