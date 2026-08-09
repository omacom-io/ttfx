//! Static effect registry (replaces upstream pkgutil discovery).

pub mod bouncyballs;
pub mod common;
pub mod decrypt;
pub mod errorcorrect;
pub mod expand;
pub mod middleout;
pub mod overflow;
pub mod pour;
pub mod print_effect;
pub mod rain;
pub mod random_sequence;
pub mod scattered;
pub mod slice;
pub mod slide;
pub mod spray;
pub mod unstable;
pub mod wipe;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Characters fall from the top of the canvas as balls before settling into place.
    Bouncyballs(bouncyballs::BouncyBallsConfig),
    /// Display a movie style decryption effect.
    Decrypt(decrypt::DecryptConfig),
    /// Some characters start in the wrong position and are corrected in sequence.
    Errorcorrect(errorcorrect::ErrorCorrectConfig),
    /// Expands the text from a single point.
    Expand(expand::ExpandConfig),
    /// Text expands in a single row or column in the middle of the canvas then out.
    Middleout(middleout::MiddleoutConfig),
    /// Input text overflows and scrolls the terminal in a random order until eventually appearing ordered.
    Overflow(overflow::OverflowConfig),
    /// Pours the characters into position from the given direction.
    Pour(pour::PourConfig),
    /// Lines are printed one at a time following a print head. Print head performs line feed, carriage return.
    #[command(name = "print")]
    Print(print_effect::PrintConfig),
    /// Rain characters onto the canvas until the input text is formed.
    Rain(rain::RainConfig),
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
    /// Text is scattered across the canvas and moves into position.
    Scattered(scattered::ScatteredConfig),
    /// Slices the input in half and slides it into place from opposite directions.
    Slice(slice::SliceConfig),
    /// Slide characters into view from outside the terminal.
    Slide(slide::SlideConfig),
    /// Draws the characters spawning at varying rates from a single point.
    Spray(spray::SprayConfig),
    /// Spawn characters jumbled, explode them to the edge of the canvas, then reassemble them in the correct layout.
    Unstable(unstable::UnstableConfig),
    /// Wipes the text across the terminal to reveal characters.
    Wipe(wipe::WipeConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Bouncyballs(config) => Box::new(bouncyballs::BouncyBalls::new(config.clone())),
            EffectCommand::Decrypt(config) => Box::new(decrypt::Decrypt::new(config.clone())),
            EffectCommand::Errorcorrect(config) => Box::new(errorcorrect::ErrorCorrect::new(config.clone())),
            EffectCommand::Expand(config) => Box::new(expand::Expand::new(config.clone())),
            EffectCommand::Middleout(config) => Box::new(middleout::Middleout::new(config.clone())),
            EffectCommand::Overflow(config) => Box::new(overflow::Overflow::new(config.clone())),
            EffectCommand::Pour(config) => Box::new(pour::Pour::new(config.clone())),
            EffectCommand::Print(config) => Box::new(print_effect::Print::new(config.clone())),
            EffectCommand::Rain(config) => Box::new(rain::Rain::new(config.clone())),
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
            EffectCommand::Scattered(config) => Box::new(scattered::Scattered::new(config.clone())),
            EffectCommand::Slice(config) => Box::new(slice::Slice::new(config.clone())),
            EffectCommand::Slide(config) => Box::new(slide::Slide::new(config.clone())),
            EffectCommand::Spray(config) => Box::new(spray::Spray::new(config.clone())),
            EffectCommand::Unstable(config) => Box::new(unstable::Unstable::new(config.clone())),
            EffectCommand::Wipe(config) => Box::new(wipe::Wipe::new(config.clone())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Bouncyballs(_) => "bouncyballs",
            EffectCommand::Decrypt(_) => "decrypt",
            EffectCommand::Errorcorrect(_) => "errorcorrect",
            EffectCommand::Expand(_) => "expand",
            EffectCommand::Middleout(_) => "middleout",
            EffectCommand::Overflow(_) => "overflow",
            EffectCommand::Pour(_) => "pour",
            EffectCommand::Print(_) => "print",
            EffectCommand::Rain(_) => "rain",
            EffectCommand::Randomsequence(_) => "randomsequence",
            EffectCommand::Scattered(_) => "scattered",
            EffectCommand::Slice(_) => "slice",
            EffectCommand::Slide(_) => "slide",
            EffectCommand::Spray(_) => "spray",
            EffectCommand::Unstable(_) => "unstable",
            EffectCommand::Wipe(_) => "wipe",
        }
    }
}
