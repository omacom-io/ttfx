pub mod easing;
pub mod geometry;
pub mod graphics;

pub use easing::{make_easing, Ease, Easing, EasingTracker, SequenceEaser};
pub use geometry::Coord;
pub use graphics::{Color, ColorPair, Gradient, GradientDirection};

/// Python 3 `round()` — ties toward even (banker's rounding).
pub fn round_half_even(x: f64) -> i64 {
    if !x.is_finite() {
        return 0;
    }
    let trunc = x.trunc();
    let frac = x - trunc;
    let abs_frac = frac.abs();
    if abs_frac < 0.5 {
        return trunc as i64;
    }
    if abs_frac > 0.5 {
        return if x.is_sign_positive() {
            trunc as i64 + 1
        } else {
            trunc as i64 - 1
        };
    }
    let n = trunc as i64;
    if n % 2 == 0 {
        n
    } else if x.is_sign_positive() {
        n + 1
    } else {
        n - 1
    }
}

/// Python floor division (`a // b`) for integers.
pub fn floor_div(a: i32, b: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    let q = a / b;
    let r = a % b;
    if r != 0 && ((a < 0) ^ (b < 0)) {
        q - 1
    } else {
        q
    }
}
