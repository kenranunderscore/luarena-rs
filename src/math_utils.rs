pub const HALF_PI: f32 = crate::PI as f32 / 2.0;
pub const TWO_PI: f32 = crate::PI as f32 * 2.0;

pub fn clamp(x: f32, lower: f32, upper: f32) -> f32 {
    f32::min(f32::max(lower, x), upper)
}

pub fn normalize_abs_angle(angle: f32) -> f32 {
    if angle >= TWO_PI {
        normalize_abs_angle(angle - TWO_PI)
    } else if angle < 0.0 {
        normalize_abs_angle(angle + TWO_PI)
    } else {
        angle
    }
}
