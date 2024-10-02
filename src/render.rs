use raylib::prelude::*;

use crate::settings::*;
use crate::game::Player;

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
    draw_line_in_direction(d, x, y, heading, 2.0 * PLAYER_RADIUS as f32, Color::RED);
}

fn player_arms(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    draw_line_in_direction(d, x, y, heading, 1.5 * PLAYER_RADIUS as f32, Color::YELLOW);
}

fn heading(d: &mut raylib::drawing::RaylibDrawHandle, x: i32, y: i32, heading: f32) {
    draw_line_in_direction(
        d,
        x,
        y,
        heading,
        1.6 * PLAYER_RADIUS as f32,
        Color::GREENYELLOW,
    );
    draw_line_in_direction(
        d,
        x,
        y,
        heading + PI as f32,
        1.2 * PLAYER_RADIUS as f32,
        Color::GREENYELLOW,
    );
    draw_line_in_direction(
        d,
        x,
        y,
        heading + PI as f32 / 2.0,
        1.2 * PLAYER_RADIUS as f32,
        Color::GREENYELLOW,
    );
    draw_line_in_direction(
        d,
        x,
        y,
        heading - PI as f32 / 2.0,
        1.2 * PLAYER_RADIUS as f32,
        Color::GREENYELLOW,
    );
}

pub fn players(d: &mut raylib::drawing::RaylibDrawHandle, players: &Vec<Player>) {
    for p in players {
        let pos = p.pos.borrow();
        player_vision(d, pos.x, pos.y, p.effective_head_heading());
        player_arms(d, pos.x, pos.y, p.effective_arms_heading());
        heading(d, pos.x, pos.y, p.heading);
        d.draw_circle(pos.x, pos.y, PLAYER_RADIUS as f32, Color::GREENYELLOW);
    }
}
