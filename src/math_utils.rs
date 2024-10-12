use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

pub const HALF_PI: f32 = PI / 2.0;
pub const TWO_PI: f32 = PI * 2.0;
pub const PI: f32 = std::f32::consts::PI;

#[derive(Clone, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn dist_sqr(&self, p: &Point) -> f32 {
        let dx = self.x - p.x;
        let dy = self.y - p.y;
        dx * dx + dy * dy
    }

    pub fn dist(&self, p: &Point) -> f32 {
        let d = self.dist_sqr(p) as f32;
        d.sqrt()
    }

    pub fn add(&self, p: &Point) -> Self {
        Self {
            x: self.x + p.x,
            y: self.y + p.y,
        }
    }

    pub fn set_to(&mut self, p: &Point) {
        self.x = p.x;
        self.y = p.y;
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl Distribution<Point> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Point {
        let (rand_x, rand_y) = rng.gen();
        Point {
            x: rand_x,
            y: rand_y,
        }
    }
}

pub fn line_endpoint(x: f32, y: f32, len: f32, angle: f32) -> Point {
    let dx = angle.sin() * len;
    let dy = -angle.cos() * len;
    Point {
        x: x + dx,
        y: y + dy,
    }
}

pub fn clamp(x: f32, lower: f32, upper: f32) -> f32 {
    f32::min(f32::max(lower, x), upper)
}

pub fn angle_between(p: &Point, q: &Point) -> f32 {
    let dx = q.x - p.x;
    let dy = q.y - p.y;
    f32::atan2(dy as f32, dx as f32) + HALF_PI
}

pub fn normalize_absolute_angle(angle: f32) -> f32 {
    if angle >= TWO_PI {
        normalize_absolute_angle(angle - TWO_PI)
    } else if angle < 0.0 {
        normalize_absolute_angle(angle + TWO_PI)
    } else {
        angle
    }
}

pub fn normalize_relative_angle(angle: f32) -> f32 {
    if angle >= -PI && angle < PI {
        angle
    } else {
        let a = normalize_absolute_angle(angle);
        if a >= PI {
            a - TWO_PI
        } else {
            a
        }
    }
}

/// A `Sector` is defined by some angle designating the middle of the sector,
/// and an offset that goes in each direction.
///
/// Example: Sector { angle: PI, delta: HALF_PI } describes the sector going
/// from 90 degrees to 270 degrees.
#[derive(Debug, Clone)]
pub struct Sector {
    angle: f32,
    delta: f32,
}

impl Sector {
    pub fn new(angle: f32, delta: f32) -> Self {
        Self { angle, delta }
    }

    pub fn left(&self) -> f32 {
        self.angle - self.delta
    }

    pub fn right(&self) -> f32 {
        self.angle + self.delta
    }

    pub fn overlaps(&self, other: &Sector) -> bool {
        let sl = normalize_absolute_angle(self.left());
        let sr = normalize_absolute_angle(self.right());
        let ol = normalize_absolute_angle(other.left());
        let or = normalize_absolute_angle(other.right());
        fn is_between(x: f32, a: f32, b: f32) -> bool {
            if a <= b {
                a <= x && x <= b
            } else {
                a <= x || x <= b
            }
        }
        is_between(sl, ol, or)
            || is_between(sr, ol, or)
            || is_between(ol, sl, sr)
            || is_between(or, sl, sr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_eq::assert_float_eq;
    use std::f32::consts::PI;

    mod angle_between {
        use super::*;

        #[test]
        fn target_right_above() {
            let angle = angle_between(&Point { x: 10.0, y: 10.0 }, &Point { x: 10.0, y: 5.0 });
            assert_float_eq!(angle, 0.0, abs <= 0.0001);
        }

        #[test]
        fn target_right_below() {
            let angle = angle_between(&Point { x: 10.0, y: 10.0 }, &Point { x: 10.0, y: 50.0 });
            assert_float_eq!(angle, PI, abs <= 0.0001);
        }

        #[test]
        fn target_directly_to_the_right() {
            let angle = angle_between(&Point { x: 10.0, y: 10.0 }, &Point { x: 20.0, y: 10.0 });
            assert_float_eq!(angle, HALF_PI, abs <= 0.0001);
        }

        #[test]
        fn target_directly_to_the_left() {
            let angle = angle_between(&Point { x: 10.0, y: 0.0 }, &Point { x: 0.0, y: 0.0 });
            assert_float_eq!(angle, 270_f32.to_radians(), abs <= 0.0001);
        }

        #[test]
        fn target_at_45_degrees() {
            let angle = angle_between(&Point { x: -1.0, y: 0.0 }, &Point { x: 0.0, y: -1.0 });
            assert_float_eq!(angle, 45_f32.to_radians(), abs <= 0.0001);
        }

        #[test]
        fn target_at_135_degrees() {
            let angle = angle_between(&Point { x: 0.0, y: 0.0 }, &Point { x: 1.0, y: 1.0 });
            assert_float_eq!(angle, 135_f32.to_radians(), abs <= 0.0001);
        }

        #[test]
        fn target_at_225_degrees() {
            let angle = angle_between(&Point { x: 0.0, y: 0.0 }, &Point { x: -1.0, y: 1.0 });
            assert_float_eq!(angle, 225_f32.to_radians(), abs <= 0.0001);
        }

        #[test]
        fn target_at_315_degrees() {
            let angle = angle_between(&Point { x: 200.0, y: 200.0 }, &Point { x: 100.0, y: 100.0 });
            assert_float_eq!(
                normalize_absolute_angle(angle),
                315_f32.to_radians(),
                abs <= 0.0001
            );
        }
    }

    mod normalize_absolute_angle {
        use super::*;

        #[test]
        fn angle_greater_than_2pi() {
            let res = normalize_absolute_angle(7.0);
            assert_float_eq!(res, 0.7168, abs <= 0.0001);
        }

        #[test]
        fn angle_greater_than_4pi() {
            let res = normalize_absolute_angle(5.0 * PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_less_than_0() {
            let res = normalize_absolute_angle(-PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_less_than_minus_2pi() {
            let res = normalize_absolute_angle(-5.0 * PI);
            assert_float_eq!(res, PI, abs <= 0.0001);
        }

        #[test]
        fn angle_between_0_and_2pi() {
            let res = normalize_absolute_angle(4.123);
            assert_float_eq!(res, 4.123, abs <= 0.0001);
        }
    }

    mod normalize_relative_angle {
        use super::*;

        #[test]
        fn angle_greater_than_2pi() {
            let res = normalize_relative_angle(7.0);
            assert_float_eq!(res, 0.7168, abs <= 0.0001);
        }

        #[test]
        fn angle_less_than_minus_pi() {
            let res = normalize_relative_angle(-2.0 * PI);
            assert_float_eq!(res, 0.0, abs <= 0.0001);
        }

        #[test]
        fn angle_between_minus_pi_and_pi() {
            let res = normalize_relative_angle(1.123);
            assert_float_eq!(res, 1.123, abs <= 0.0001);
        }

        #[test]
        fn angle_between_pi_and_2pi() {
            let res = normalize_relative_angle(3.0 / 2.0 * PI);
            assert_float_eq!(res, -PI / 2.0, abs <= 0.0001);
        }
    }

    mod point {
        use super::*;

        #[test]
        fn add() {
            let p = Point { x: 13.0, y: -5.0 };
            let res = p.add(&Point { x: -2.0, y: 4.0 });
            assert_float_eq!(res.x, 11.0, abs <= 0.0001);
            assert_float_eq!(res.y, -1.0, abs <= 0.0001);
        }

        #[test]
        fn set_to() {
            let mut p = Point::zero();
            p.set_to(&Point { x: 3.1, y: -2.5 });
            assert_float_eq!(p.x, 3.1, abs <= 0.0001);
            assert_float_eq!(p.y, -2.5, abs <= 0.0001);
        }
    }

    mod sector {
        mod overlaps {
            use crate::math_utils::*;
            use quickcheck::Arbitrary;
            use quickcheck_macros::quickcheck;

            fn f32_between(lower: f32, upper: f32, g: &mut quickcheck::Gen) -> f32 {
                let mut x = f32::arbitrary(g);
                while x.is_nan() || x.is_infinite() {
                    x = f32::arbitrary(g);
                }
                lower + (x.abs() % (upper - lower))
            }

            impl Arbitrary for Sector {
                fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                    // Only create well-behaved `Sector` values
                    let angle = f32_between(0.0, TWO_PI, g);
                    let delta = f32_between(0.0, PI, g);
                    Sector { angle, delta }
                }
            }

            #[quickcheck]
            fn overlapping_is_commutative(s: Sector, t: Sector) -> bool {
                s.overlaps(&t) == t.overlaps(&s)
            }

            impl Sector {
                fn from_degrees(angle: f32, delta: f32) -> Self {
                    Self {
                        angle: angle.to_radians(),
                        delta: delta.to_radians(),
                    }
                }
            }

            mod first_quadrant {
                use crate::math_utils::*;

                #[test]
                fn too_far_left() {
                    let overlap =
                        Sector::from_degrees(45.0, 30.0).overlaps(&Sector::from_degrees(0.0, 10.0));
                    assert!(!overlap);
                }

                #[test]
                fn too_far_right() {
                    let overlap =
                        Sector::from_degrees(45.0, 30.0).overlaps(&Sector::from_degrees(80.0, 4.0));
                    assert!(!overlap);
                }

                #[test]
                fn target_fully_contained() {
                    let overlap =
                        Sector::from_degrees(45.0, 30.0).overlaps(&Sector::from_degrees(50.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn source_fully_contained() {
                    let overlap = Sector::from_degrees(45.0, 30.0)
                        .overlaps(&Sector::from_degrees(40.0, 45.0));
                    assert!(overlap);
                }

                #[test]
                fn left_only_overlap() {
                    let overlap =
                        Sector::from_degrees(30.0, 25.0).overlaps(&Sector::from_degrees(4.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn right_only_overlap() {
                    let overlap =
                        Sector::from_degrees(30.0, 25.0).overlaps(&Sector::from_degrees(55.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn mixed_with_second() {
                    let overlap = Sector::from_degrees(80.0, 25.0)
                        .overlaps(&Sector::from_degrees(110.0, 7.0));
                    assert!(overlap);
                }
            }

            mod second_quadrant {
                use crate::math_utils::*;

                #[test]
                fn too_far_left() {
                    let overlap = Sector::from_degrees(135.0, 30.0)
                        .overlaps(&Sector::from_degrees(90.0, 10.0));
                    assert!(!overlap);
                }

                #[test]
                fn too_far_right() {
                    let overlap = Sector::from_degrees(135.0, 30.0)
                        .overlaps(&Sector::from_degrees(170.0, 4.0));
                    assert!(!overlap);
                }

                #[test]
                fn target_fully_contained() {
                    let overlap = Sector::from_degrees(135.0, 30.0)
                        .overlaps(&Sector::from_degrees(140.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn source_fully_contained() {
                    let overlap = Sector::from_degrees(135.0, 30.0)
                        .overlaps(&Sector::from_degrees(130.0, 45.0));
                    assert!(overlap);
                }

                #[test]
                fn left_only_overlap() {
                    let overlap = Sector::from_degrees(120.0, 25.0)
                        .overlaps(&Sector::from_degrees(94.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn right_only_overlap() {
                    let overlap = Sector::from_degrees(120.0, 25.0)
                        .overlaps(&Sector::from_degrees(145.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn mixed_with_third() {
                    let overlap = Sector::from_degrees(170.0, 25.0)
                        .overlaps(&Sector::from_degrees(200.0, 7.0));
                    assert!(overlap);
                }
            }

            mod third_quadrant {
                use crate::math_utils::*;

                #[test]
                fn too_far_left() {
                    let overlap = Sector::from_degrees(225.0, 30.0)
                        .overlaps(&Sector::from_degrees(180.0, 10.0));
                    assert!(!overlap);
                }

                #[test]
                fn too_far_right() {
                    let overlap = Sector::from_degrees(225.0, 30.0)
                        .overlaps(&Sector::from_degrees(260.0, 4.0));
                    assert!(!overlap);
                }

                #[test]
                fn target_fully_contained() {
                    let overlap = Sector::from_degrees(225.0, 30.0)
                        .overlaps(&Sector::from_degrees(230.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn source_fully_contained() {
                    let overlap = Sector::from_degrees(225.0, 30.0)
                        .overlaps(&Sector::from_degrees(220.0, 45.0));
                    assert!(overlap);
                }

                #[test]
                fn left_only_overlap() {
                    let overlap = Sector::from_degrees(210.0, 25.0)
                        .overlaps(&Sector::from_degrees(184.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn right_only_overlap() {
                    let overlap = Sector::from_degrees(210.0, 25.0)
                        .overlaps(&Sector::from_degrees(235.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn mixed_with_fourth() {
                    let overlap = Sector::from_degrees(260.0, 25.0)
                        .overlaps(&Sector::from_degrees(290.0, 7.0));
                    assert!(overlap);
                }
            }

            mod fourth_quadrant {
                use crate::math_utils::*;

                #[test]
                fn too_far_left() {
                    let overlap = Sector::from_degrees(315.0, 30.0)
                        .overlaps(&Sector::from_degrees(270.0, 10.0));
                    assert!(!overlap);
                }

                #[test]
                fn too_far_right() {
                    let overlap = Sector::from_degrees(315.0, 30.0)
                        .overlaps(&Sector::from_degrees(350.0, 4.0));
                    assert!(!overlap);
                }

                #[test]
                fn target_fully_contained() {
                    let overlap = Sector::from_degrees(315.0, 30.0)
                        .overlaps(&Sector::from_degrees(320.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn source_fully_contained() {
                    let overlap = Sector::from_degrees(315.0, 30.0)
                        .overlaps(&Sector::from_degrees(310.0, 45.0));
                    assert!(overlap);
                }

                #[test]
                fn left_only_overlap() {
                    let overlap = Sector::from_degrees(300.0, 25.0)
                        .overlaps(&Sector::from_degrees(274.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn right_only_overlap() {
                    let overlap = Sector::from_degrees(300.0, 25.0)
                        .overlaps(&Sector::from_degrees(325.0, 5.0));
                    assert!(overlap);
                }
            }

            mod first_to_fourth_overflow {
                use crate::math_utils::*;

                #[test]
                fn too_far_left() {
                    let overlap = Sector::from_degrees(355.0, 10.0)
                        .overlaps(&Sector::from_degrees(340.0, 2.0));
                    assert!(!overlap);
                }

                #[test]
                fn too_far_right() {
                    let overlap = Sector::from_degrees(355.0, 10.0)
                        .overlaps(&Sector::from_degrees(20.0, 12.0));
                    assert!(!overlap);
                }

                #[test]
                fn target_fully_contained_both_overflowing() {
                    let overlap = Sector::from_degrees(350.0, 30.0)
                        .overlaps(&Sector::from_degrees(355.0, 10.0));
                    assert!(overlap);
                }

                #[test]
                fn target_fully_contained_source_overflowing() {
                    let overlap = Sector::from_degrees(350.0, 30.0)
                        .overlaps(&Sector::from_degrees(352.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn left_only_overlap() {
                    let overlap = Sector::from_degrees(340.0, 40.0)
                        .overlaps(&Sector::from_degrees(290.0, 20.0));
                    assert!(overlap);
                }

                #[test]
                fn right_only_overlap_target_right_overflowing() {
                    let overlap = Sector::from_degrees(350.0, 20.0)
                        .overlaps(&Sector::from_degrees(355.0, 20.0));
                    assert!(overlap);
                }

                #[test]
                fn target_in_next_quadrant_fully_contained() {
                    let overlap =
                        Sector::from_degrees(350.0, 20.0).overlaps(&Sector::from_degrees(5.0, 4.0));
                    assert!(overlap);
                }

                #[test]
                fn target_in_next_quadrant_not_fully_contained() {
                    let overlap =
                        Sector::from_degrees(350.0, 20.0).overlaps(&Sector::from_degrees(9.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn target_in_next_quadrant_underflowing_fully_contained() {
                    let overlap =
                        Sector::from_degrees(350.0, 20.0).overlaps(&Sector::from_degrees(1.0, 5.0));
                    assert!(overlap);
                }

                #[test]
                fn target_in_next_quadrant_underflowing_not_fully_contained() {
                    let overlap = Sector::from_degrees(350.0, 20.0)
                        .overlaps(&Sector::from_degrees(5.0, 10.0));
                    assert!(overlap);
                }
            }
        }
    }
}
