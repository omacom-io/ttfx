use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Easing {
    #[default]
    Linear,
    InSine,
    OutSine,
    InOutSine,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    InOutCubic,
    InQuart,
    OutQuart,
    InOutQuart,
    InQuint,
    OutQuint,
    InOutQuint,
    InExpo,
    OutExpo,
    InOutExpo,
    InCirc,
    OutCirc,
    InOutCirc,
    InBack,
    OutBack,
    InOutBack,
    InElastic,
    OutElastic,
    InOutElastic,
    InBounce,
    OutBounce,
    InOutBounce,
}

impl Easing {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().replace('-', "_").as_str() {
            "linear" => Some(Self::Linear),
            "in_sine" => Some(Self::InSine),
            "out_sine" => Some(Self::OutSine),
            "in_out_sine" => Some(Self::InOutSine),
            "in_quad" => Some(Self::InQuad),
            "out_quad" => Some(Self::OutQuad),
            "in_out_quad" => Some(Self::InOutQuad),
            "in_cubic" => Some(Self::InCubic),
            "out_cubic" => Some(Self::OutCubic),
            "in_out_cubic" => Some(Self::InOutCubic),
            "in_quart" => Some(Self::InQuart),
            "out_quart" => Some(Self::OutQuart),
            "in_out_quart" => Some(Self::InOutQuart),
            "in_quint" => Some(Self::InQuint),
            "out_quint" => Some(Self::OutQuint),
            "in_out_quint" => Some(Self::InOutQuint),
            "in_expo" => Some(Self::InExpo),
            "out_expo" => Some(Self::OutExpo),
            "in_out_expo" => Some(Self::InOutExpo),
            "in_circ" => Some(Self::InCirc),
            "out_circ" => Some(Self::OutCirc),
            "in_out_circ" => Some(Self::InOutCirc),
            "in_back" => Some(Self::InBack),
            "out_back" => Some(Self::OutBack),
            "in_out_back" => Some(Self::InOutBack),
            "in_elastic" => Some(Self::InElastic),
            "out_elastic" => Some(Self::OutElastic),
            "in_out_elastic" => Some(Self::InOutElastic),
            "in_bounce" => Some(Self::InBounce),
            "out_bounce" => Some(Self::OutBounce),
            "in_out_bounce" => Some(Self::InOutBounce),
            _ => None,
        }
    }

    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::InSine => 1.0 - (t * PI / 2.0).cos(),
            Self::OutSine => (t * PI / 2.0).sin(),
            Self::InOutSine => -((PI * t).cos() - 1.0) / 2.0,
            Self::InQuad => t * t,
            Self::OutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::InOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::InCubic => t * t * t,
            Self::OutCubic => 1.0 - (1.0 - t).powi(3),
            Self::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::InQuart => t.powi(4),
            Self::OutQuart => 1.0 - (1.0 - t).powi(4),
            Self::InOutQuart => {
                if t < 0.5 {
                    8.0 * t.powi(4)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            Self::InQuint => t.powi(5),
            Self::OutQuint => 1.0 - (1.0 - t).powi(5),
            Self::InOutQuint => {
                if t < 0.5 {
                    16.0 * t.powi(5)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            Self::InExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0_f64.powf(10.0 * t - 10.0)
                }
            }
            Self::OutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f64.powf(-10.0 * t)
                }
            }
            Self::InOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0_f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            Self::InCirc => 1.0 - (1.0 - t * t).sqrt(),
            Self::OutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            Self::InOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            Self::InBack => {
                const C1: f64 = 1.70158;
                const C3: f64 = C1 + 1.0;
                C3 * t * t * t - C1 * t * t
            }
            Self::OutBack => {
                const C1: f64 = 1.70158;
                const C3: f64 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Self::InOutBack => {
                const C1: f64 = 1.70158;
                const C2: f64 = C1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
                }
            }
            Self::InElastic => in_elastic(t),
            Self::OutElastic => out_elastic(t),
            Self::InOutElastic => {
                const C5: f64 = (2.0 * PI) / 4.5;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -(2.0_f64.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0
                } else {
                    (2.0_f64.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * C5).sin()) / 2.0 + 1.0
                }
            }
            Self::InBounce => 1.0 - out_bounce(1.0 - t),
            Self::OutBounce => out_bounce(t),
            Self::InOutBounce => {
                if t < 0.5 {
                    (1.0 - out_bounce(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + out_bounce(2.0 * t - 1.0)) / 2.0
                }
            }
        }
    }
}

fn in_elastic(t: f64) -> f64 {
    const C4: f64 = (2.0 * PI) / 3.0;
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        -(2.0_f64.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * C4).sin()
    }
}

fn out_elastic(t: f64) -> f64 {
    const C4: f64 = (2.0 * PI) / 3.0;
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        2.0_f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
    }
}

fn out_bounce(t: f64) -> f64 {
    const N1: f64 = 7.5625;
    const D1: f64 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl CubicBezier {
    pub fn apply(self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let mut t = x;
        for _ in 0..8 {
            let x_est = sample_bezier(t, self.x1, self.x2);
            let dx = sample_bezier_slope(t, self.x1, self.x2);
            if dx.abs() < 1e-6 {
                break;
            }
            t = (t - (x_est - x) / dx).clamp(0.0, 1.0);
        }
        sample_bezier(t, self.y1, self.y2)
    }
}

fn sample_bezier(t: f64, a: f64, b: f64) -> f64 {
    let u = 1.0 - t;
    3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
}

fn sample_bezier_slope(t: f64, a: f64, b: f64) -> f64 {
    let u = 1.0 - t;
    3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ease {
    Named(Easing),
    Bezier(CubicBezier),
}

impl Ease {
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Self::Named(easing) => easing.apply(t),
            Self::Bezier(bezier) => bezier.apply(t),
        }
    }
}

impl From<Easing> for Ease {
    fn from(easing: Easing) -> Self {
        Self::Named(easing)
    }
}

pub fn make_easing(x1: f64, y1: f64, x2: f64, y2: f64) -> Ease {
    Ease::Bezier(CubicBezier { x1, y1, x2, y2 })
}

#[derive(Clone, Debug)]
pub struct EasingTracker {
    pub ease: Ease,
    pub total_steps: u32,
    pub current_step: u32,
}

impl EasingTracker {
    pub fn new(ease: impl Into<Ease>, total_steps: u32) -> Self {
        Self {
            ease: ease.into(),
            total_steps,
            current_step: 0,
        }
    }

    pub fn next(&mut self) -> f64 {
        if self.total_steps == 0 {
            return self.ease.apply(1.0);
        }
        self.current_step = (self.current_step + 1).min(self.total_steps);
        self.ease
            .apply(self.current_step as f64 / self.total_steps as f64)
    }

    pub fn is_complete(&self) -> bool {
        self.total_steps == 0 || self.current_step >= self.total_steps
    }
}

#[derive(Clone, Debug)]
pub struct SequenceEaser {
    pub parts: Vec<(Ease, u32)>,
    index: usize,
    tracker: Option<EasingTracker>,
}

impl SequenceEaser {
    pub fn new(parts: Vec<(Ease, u32)>) -> Self {
        let tracker = parts
            .first()
            .map(|(ease, steps)| EasingTracker::new(*ease, *steps));
        Self {
            parts,
            index: 0,
            tracker,
        }
    }

    pub fn next(&mut self) -> f64 {
        loop {
            let Some(tracker) = self.tracker.as_mut() else {
                return 1.0;
            };
            let value = tracker.next();
            if !tracker.is_complete() {
                return value;
            }
            self.index += 1;
            if self.index >= self.parts.len() {
                return value;
            }
            let (ease, steps) = self.parts[self.index];
            self.tracker = Some(EasingTracker::new(ease, steps));
        }
    }
}
