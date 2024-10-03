use std::f32::consts::PI;

pub const HALF_PI: f32 = PI / 2.0;
pub const TWO_PI: f32 = PI * 2.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::assert_float_eq;
    use std::f32::consts::PI;

    mod normalize_abs_angle {
        use super::*;

        #[test]
        fn angle_greater_than_2pi() {
            let res = normalize_abs_angle(7.0);
            assert_float_eq!(res, 0.7168, abs <= 0.0001);
        }

        #[test]
        fn angle_greater_than_4pi() {
            let res = normalize_abs_angle(5.0 * PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_less_than_0() {
            let res = normalize_abs_angle(-PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_less_than_minus_2pi() {
            let res = normalize_abs_angle(-5.0 * PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_between_0_and_2pi() {
            let res = normalize_abs_angle(4.123);
            assert_float_eq!(res, 4.123, abs <= 0.0001);
        }
    }
}
