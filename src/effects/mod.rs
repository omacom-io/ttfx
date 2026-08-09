//! Static effect registry (replaces upstream pkgutil discovery).

pub mod bouncyballs;
pub mod common;
pub mod errorcorrect;
pub mod rain;
pub mod random_sequence;
pub mod spray;
pub mod wipe;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Characters are bouncy balls falling from the top of the canvas.
    Bouncyballs(bouncyballs::BouncyBallsConfig),
    /// Some characters start in the wrong position and are corrected in sequence.
    Errorcorrect(errorcorrect::ErrorCorrectConfig),
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
    /// Rain characters from the top of the canvas.
    Rain(rain::RainConfig),
    /// Draws the characters spawning at varying rates from a single point.
    Spray(spray::SprayConfig),
    /// Wipes the text across the terminal to reveal characters.
    Wipe(wipe::WipeConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Bouncyballs(config) => Box::new(bouncyballs::BouncyBalls::new(config.clone())),
            EffectCommand::Errorcorrect(config) => Box::new(errorcorrect::ErrorCorrect::new(config.clone())),
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
            EffectCommand::Rain(config) => Box::new(rain::Rain::new(config.clone())),
            EffectCommand::Spray(config) => Box::new(spray::Spray::new(config.clone())),
            EffectCommand::Wipe(config) => Box::new(wipe::Wipe::new(config.clone())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Bouncyballs(_) => "bouncyballs",
            EffectCommand::Errorcorrect(_) => "errorcorrect",
            EffectCommand::Randomsequence(_) => "randomsequence",
            EffectCommand::Rain(_) => "rain",
            EffectCommand::Spray(_) => "spray",
            EffectCommand::Wipe(_) => "wipe",
        }
    }
}
