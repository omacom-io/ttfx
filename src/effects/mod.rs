//! Static effect registry (replaces upstream pkgutil discovery).

pub mod common;
pub mod expand;
pub mod random_sequence;
pub mod wipe;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Expands the text from a single point.
    Expand(expand::ExpandConfig),
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
    /// Wipes the text across the terminal to reveal characters.
    Wipe(wipe::WipeConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Expand(config) => Box::new(expand::Expand::new(config.clone())),
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
            EffectCommand::Wipe(config) => Box::new(wipe::Wipe::new(config.clone())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Expand(_) => "expand",
            EffectCommand::Randomsequence(_) => "randomsequence",
            EffectCommand::Wipe(_) => "wipe",
        }
    }
}
