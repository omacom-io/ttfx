//! Static effect registry (replaces upstream pkgutil discovery).

pub mod common;
pub mod expand;
pub mod middleout;
pub mod pour;
pub mod random_sequence;
pub mod scattered;
pub mod slice;
pub mod slide;
pub mod wipe;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Expands the text from a single point.
    Expand(expand::ExpandConfig),
    /// Text expands in a single row or column in the middle of the canvas then out.
    Middleout(middleout::MiddleOutConfig),
    /// Pours the characters into position from the given direction.
    Pour(pour::PourConfig),
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
    /// Text is scattered across the canvas and moves into position.
    Scattered(scattered::ScatteredConfig),
    /// Slices the input in half and slides it into place from opposite directions.
    Slice(slice::SliceConfig),
    /// Slide characters into view from outside the terminal.
    Slide(slide::SlideConfig),
    /// Wipes the text across the terminal to reveal characters.
    Wipe(wipe::WipeConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Expand(config) => Box::new(expand::Expand::new(config.clone())),
            EffectCommand::Middleout(config) => Box::new(middleout::MiddleOut::new(config.clone())),
            EffectCommand::Pour(config) => Box::new(pour::Pour::new(config.clone())),
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
            EffectCommand::Scattered(config) => Box::new(scattered::Scattered::new(config.clone())),
            EffectCommand::Slice(config) => Box::new(slice::Slice::new(config.clone())),
            EffectCommand::Slide(config) => Box::new(slide::Slide::new(config.clone())),
            EffectCommand::Wipe(config) => Box::new(wipe::Wipe::new(config.clone())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Expand(_) => "expand",
            EffectCommand::Middleout(_) => "middleout",
            EffectCommand::Pour(_) => "pour",
            EffectCommand::Randomsequence(_) => "randomsequence",
            EffectCommand::Scattered(_) => "scattered",
            EffectCommand::Slice(_) => "slice",
            EffectCommand::Slide(_) => "slide",
            EffectCommand::Wipe(_) => "wipe",
        }
    }
}
