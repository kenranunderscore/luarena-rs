use raylib::prelude::*;

use crate::game::{GameData, PlayerData};
use crate::math_utils::Point;
use crate::{math_utils, settings::*};

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

fn players<'a>(
    d: &mut raylib::drawing::RaylibDrawHandle,
    players: impl Iterator<Item = &'a PlayerData>,
) {
    for player in players {
        let player_color = to_raylib_color(&player.color);
        player_vision(d, player.x, player.y, player.head_heading);
        player_arms(d, player.x, player.y, player.arms_heading);
        heading(d, player.x, player.y, player.heading, player_color);
        d.draw_circle(player.x, player.y, PLAYER_RADIUS as f32, player_color);
    }
}

fn attacks(d: &mut raylib::drawing::RaylibDrawHandle, attacks: &[Point]) {
    for attack in attacks {
        let attack_color = Color::RED;
        d.draw_circle(
            attack.x.round() as i32,
            attack.y.round() as i32,
            ATTACK_RADIUS,
            attack_color,
        );
    }
}

pub fn game(d: &mut raylib::drawing::RaylibDrawHandle, game_data: &GameData) {
    players(d, game_data.players.iter());
    attacks(d, &game_data.attacks);
}
