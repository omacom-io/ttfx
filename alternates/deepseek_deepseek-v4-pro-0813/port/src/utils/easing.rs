pub type EasingFn = fn(f32) -> f32;

#[inline]
fn clamp01(t: f32) -> f32 {
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        t
    }
}

pub fn linear(t: f32) -> f32 {
    clamp01(t)
}

pub fn ease_in_quad(t: f32) -> f32 {
    let t = clamp01(t);
    t * t
}

pub fn ease_out_quad(t: f32) -> f32 {
    let t = clamp01(t);
    t * (2.0 - t)
}

pub fn ease_in_out_quad(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

pub fn ease_in_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    let u = t - 1.0;
    u * u * u + 1.0
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        (t - 1.0) * (2.0 * t - 2.0) * (2.0 * t - 2.0) + 1.0
    }
}

pub fn ease_in_quart(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t * t
}

pub fn ease_out_quart(t: f32) -> f32 {
    let t = clamp01(t);
    let u = t - 1.0;
    1.0 - u * u * u * u
}

pub fn ease_in_out_quart(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        let u = t - 1.0;
        1.0 - 8.0 * u * u * u * u
    }
}

pub fn ease_in_quint(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * t * t * t
}

pub fn ease_out_quint(t: f32) -> f32 {
    let t = clamp01(t);
    let u = t - 1.0;
    1.0 + u * u * u * u * u
}

pub fn ease_in_out_quint(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        16.0 * t * t * t * t * t
    } else {
        let u = t - 1.0;
        1.0 + 16.0 * u * u * u * u * u
    }
}

pub fn ease_in_sine(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
}

pub fn ease_out_sine(t: f32) -> f32 {
    let t = clamp01(t);
    (t * std::f32::consts::FRAC_PI_2).sin()
}

pub fn ease_in_out_sine(t: f32) -> f32 {
    let t = clamp01(t);
    -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
}

pub fn ease_in_expo(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * (t - 1.0))
    }
}

pub fn ease_out_expo(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

pub fn ease_in_out_expo(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

pub fn ease_in_circ(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - (1.0 - t * t).sqrt()
}

pub fn ease_out_circ(t: f32) -> f32 {
    let t = clamp01(t);
    (1.0 - (t - 1.0).powi(2)).sqrt()
}

pub fn ease_in_out_circ(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
    }
}

pub fn ease_in_back(t: f32) -> f32 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    c3 * t * t * t - c1 * t * t
}

pub fn ease_out_back(t: f32) -> f32 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_in_out_back(t: f32) -> f32 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c2 = c1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
    } else {
        ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
    }
}

pub fn ease_out_elastic(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

pub fn ease_in_elastic(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    -(2.0_f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
}

pub fn ease_in_out_elastic(t: f32) -> f32 {
    let t = clamp01(t);
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c5 = (2.0 * std::f32::consts::PI) / 4.5;
    if t < 0.5 {
        -(2.0_f32.powf(20.0 * t - 10.0)) * ((20.0 * t - 11.125) * c5).sin() / 2.0
    } else {
        (2.0_f32.powf(-20.0 * t + 10.0)) * ((20.0 * t - 11.125) * c5).sin() / 2.0 + 1.0
    }
}

fn bounce_out(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

pub fn ease_out_bounce(t: f32) -> f32 {
    let t = clamp01(t);
    bounce_out(t)
}

pub fn ease_in_bounce(t: f32) -> f32 {
    let t = clamp01(t);
    1.0 - bounce_out(1.0 - t)
}

pub fn ease_in_out_bounce(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        ease_in_bounce(t * 2.0) * 0.5
    } else {
        ease_out_bounce(t * 2.0 - 1.0) * 0.5 + 0.5
    }
}
