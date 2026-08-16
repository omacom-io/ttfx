//! Named easing functions (mirrors terminaltexteffects/utils/easing.py).
//! A subset of the 31 upstream easings sufficient for engine motion/animation
//! stepping; more can be added mechanically following the same signature.

pub type EasingFunction = fn(f64) -> f64;

pub fn linear(t: f64) -> f64 {
    t
}

pub fn ease_in_sine(t: f64) -> f64 {
    1.0 - ((t * std::f64::consts::PI) / 2.0).cos()
}

pub fn ease_out_sine(t: f64) -> f64 {
    ((t * std::f64::consts::PI) / 2.0).sin()
}

pub fn ease_in_out_sine(t: f64) -> f64 {
    -(( (std::f64::consts::PI * t).cos() - 1.0) / 2.0)
}

pub fn ease_in_quad(t: f64) -> f64 {
    t * t
}

pub fn ease_out_quad(t: f64) -> f64 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub fn ease_in_out_quad(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

pub fn ease_in_cubic(t: f64) -> f64 {
    t.powf(3.0)
}

pub fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powf(3.0)
}

pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t.powf(3.0)
    } else {
        1.0 - (-2.0 * t + 2.0).powf(3.0) / 2.0
    }
}

pub fn ease_in_expo(t: f64) -> f64 {
    if t == 0.0 {
        0.0
    } else {
        2.0f64.powf(10.0 * t - 10.0)
    }
}

pub fn ease_out_expo(t: f64) -> f64 {
    if t == 1.0 {
        1.0
    } else {
        1.0 - 2.0f64.powf(-10.0 * t)
    }
}

/// Cubic bezier easing constructor, mirrors `easing.make_easing`.
///
/// This is a direct transcription of the CPython reference implementation's
/// Newton's-method root find (same constants, same iteration count, same
/// early-exit thresholds) so that outputs match bit-for-bit (within f64
/// precision) with the Python source.
pub fn make_easing(x1: f64, y1: f64, x2: f64, y2: f64) -> impl Fn(f64) -> f64 {
    // Compute Bezier curve x for a given parameter t.
    fn sample_curve_x(t: f64, x1: f64, x2: f64) -> f64 {
        let a = 3.0 * x1 * (1.0 - t).powf(2.0) * t;
        let b = 3.0 * x2 * (1.0 - t) * t.powf(2.0);
        let c = t.powf(3.0);
        a + b + c
    }

    // Compute Bezier curve y for a given parameter t.
    fn sample_curve_y(t: f64, y1: f64, y2: f64) -> f64 {
        let a = 3.0 * y1 * (1.0 - t).powf(2.0) * t;
        let b = 3.0 * y2 * (1.0 - t) * t.powf(2.0);
        let c = t.powf(3.0);
        a + b + c
    }

    // Compute derivative of curve x with respect to t.
    fn sample_curve_derivative_x(t: f64, x1: f64, x2: f64) -> f64 {
        let a = 3.0 * (1.0 - t).powf(2.0) * x1;
        let b = 6.0 * (1.0 - t) * t * (x2 - x1);
        let c = 3.0 * t.powf(2.0) * (1.0 - x2);
        a + b + c
    }

    move |progress: f64| -> f64 {
        // Clamp progress between 0 and 1.
        if progress <= 0.0 {
            return 0.0;
        }
        if progress >= 1.0 {
            return 1.0;
        }

        // Find t such that sample_curve_x(t) is close to progress.
        let mut t = progress; // initial guess
        for _ in 0..20 {
            let x_est = sample_curve_x(t, x1, x2);
            let dx = x_est - progress;
            if dx.abs() < 1e-5 {
                break;
            }
            let d = sample_curve_derivative_x(t, x1, x2);
            if d.abs() < 1e-6 {
                break;
            }
            t -= dx / d;
        }
        sample_curve_y(t, y1, y2)
    }
}
