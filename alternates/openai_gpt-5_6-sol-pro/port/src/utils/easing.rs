pub type EasingFn = fn(f64) -> f64;

pub fn linear(progress: f64) -> f64 {
    progress.clamp(0.0, 1.0)
}

pub fn in_sine(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (progress * std::f64::consts::FRAC_PI_2).cos()
}

pub fn out_sine(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    (progress * std::f64::consts::FRAC_PI_2).sin()
}

pub fn in_out_sine(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    -((std::f64::consts::PI * progress).cos() - 1.0) / 2.0
}

pub fn in_quad(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    progress * progress
}

pub fn out_quad(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - progress) * (1.0 - progress)
}

pub fn in_out_quad(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress < 0.5 {
        2.0 * progress * progress
    } else {
        1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0
    }
}

pub fn in_cubic(progress: f64) -> f64 {
    progress.clamp(0.0, 1.0).powf(3.0)
}

pub fn out_cubic(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    1.0 - (1.0 - progress).powf(3.0)
}

pub fn in_out_cubic(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress < 0.5 {
        4.0 * progress.powf(3.0)
    } else {
        1.0 - (-2.0 * progress + 2.0).powf(3.0) / 2.0
    }
}

pub fn in_expo(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress == 0.0 {
        0.0
    } else {
        2.0_f64.powf(10.0 * progress - 10.0)
    }
}

pub fn out_expo(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * progress)
    }
}

pub fn in_out_expo(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);

    if progress == 0.0 || progress == 1.0 {
        progress
    } else if progress < 0.5 {
        2.0_f64.powf(20.0 * progress - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f64.powf(-20.0 * progress + 10.0)) / 2.0
    }
}

#[derive(Debug, Clone)]
pub struct EasingTracker {
    total_steps: u32,
    current_step: u32,
    easing: EasingFn,
}

impl EasingTracker {
    pub fn new(total_steps: u32, easing: EasingFn) -> Self {
        Self {
            total_steps: total_steps.max(1),
            current_step: 0,
            easing,
        }
    }

    pub fn reset(&mut self) {
        self.current_step = 0;
    }

    pub fn is_finished(&self) -> bool {
        self.current_step >= self.total_steps
    }

    pub fn progress(&self) -> f64 {
        let raw = self.current_step as f64 / self.total_steps as f64;
        (self.easing)(raw.clamp(0.0, 1.0))
    }

    pub fn step(&mut self) -> f64 {
        let progress = self.progress();

        if self.current_step < self.total_steps {
            self.current_step += 1;
        }

        progress
    }
}
