//! Static effect registry (replaces upstream pkgutil discovery).

pub mod binarypath;
pub mod bouncyballs;
pub mod common;
pub mod errorcorrect;
pub mod expand;
pub mod middleout;
pub mod orbittingvolley;
pub mod pour;
pub mod rain;
pub mod random_sequence;
pub mod rings;
pub mod scattered;
pub mod slice;
pub mod slide;
pub mod spray;
pub mod wipe;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Binary representations of each character move towards the home coordinate of the character.
    Binarypath(binarypath::BinaryPathConfig),
    /// Characters fall from the top of the canvas as balls before settling into place.
    Bouncyballs(bouncyballs::BouncyBallsConfig),
    /// Some characters start in the wrong position and are corrected in sequence.
    Errorcorrect(errorcorrect::ErrorCorrectConfig),
    /// Expands the text from a single point.
    Expand(expand::ExpandConfig),
    /// Text expands in a single row or column in the middle of the canvas then out.
    Middleout(middleout::MiddleoutConfig),
    /// Four launchers orbit the canvas firing volleys of characters inward to build the input text from the center out.
    Orbittingvolley(orbittingvolley::OrbittingVolleyConfig),
    /// Pours the characters into position from the given direction.
    Pour(pour::PourConfig),
    /// Rain characters onto the canvas until the input text is formed.
    Rain(rain::RainConfig),
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
    /// Characters are dispersed and form into spinning rings.
    Rings(rings::RingsConfig),
    /// Text is scattered across the canvas and moves into position.
    Scattered(scattered::ScatteredConfig),
    /// Slices the input in half and slides it into place from opposite directions.
    Slice(slice::SliceConfig),
    /// Slide characters into view from outside the terminal.
    Slide(slide::SlideConfig),
    /// Draws the characters spawning at varying rates from a single point.
    Spray(spray::SprayConfig),
    /// Wipes the text across the terminal to reveal characters.
    Wipe(wipe::WipeConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Binarypath(config) => Box::new(binarypath::BinaryPath::new(config.clone())),
            EffectCommand::Bouncyballs(config) => Box::new(bouncyballs::BouncyBalls::new(config.clone())),
            EffectCommand::Errorcorrect(config) => Box::new(errorcorrect::ErrorCorrect::new(config.clone())),
            EffectCommand::Expand(config) => Box::new(expand::Expand::new(config.clone())),
            EffectCommand::Middleout(config) => Box::new(middleout::Middleout::new(config.clone())),
            EffectCommand::Orbittingvolley(config) => {
                Box::new(orbittingvolley::OrbittingVolley::new(config.clone()))
            }
            EffectCommand::Pour(config) => Box::new(pour::Pour::new(config.clone())),
            EffectCommand::Rain(config) => Box::new(rain::Rain::new(config.clone())),
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
            EffectCommand::Rings(config) => Box::new(rings::Rings::new(config.clone())),
            EffectCommand::Scattered(config) => Box::new(scattered::Scattered::new(config.clone())),
            EffectCommand::Slice(config) => Box::new(slice::Slice::new(config.clone())),
            EffectCommand::Slide(config) => Box::new(slide::Slide::new(config.clone())),
            EffectCommand::Spray(config) => Box::new(spray::Spray::new(config.clone())),
            EffectCommand::Wipe(config) => Box::new(wipe::Wipe::new(config.clone())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Binarypath(_) => "binarypath",
            EffectCommand::Bouncyballs(_) => "bouncyballs",
            EffectCommand::Errorcorrect(_) => "errorcorrect",
            EffectCommand::Expand(_) => "expand",
            EffectCommand::Middleout(_) => "middleout",
            EffectCommand::Orbittingvolley(_) => "orbittingvolley",
            EffectCommand::Pour(_) => "pour",
            EffectCommand::Rain(_) => "rain",
            EffectCommand::Randomsequence(_) => "randomsequence",
            EffectCommand::Rings(_) => "rings",
            EffectCommand::Scattered(_) => "scattered",
            EffectCommand::Slice(_) => "slice",
            EffectCommand::Slide(_) => "slide",
            EffectCommand::Spray(_) => "spray",
            EffectCommand::Wipe(_) => "wipe",
        }
    }
}
