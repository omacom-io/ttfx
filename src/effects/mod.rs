//! Static effect registry (replaces upstream pkgutil discovery).

pub mod common;
pub mod random_sequence;

use clap::Subcommand;

use crate::engine::effect::Effect;

#[derive(Subcommand, Debug, Clone)]
pub enum EffectCommand {
    /// Prints the input data in a random sequence.
    Randomsequence(random_sequence::RandomSequenceConfig),
}

impl EffectCommand {
    pub fn build_effect(&self) -> Box<dyn Effect> {
        match self {
            EffectCommand::Randomsequence(config) => {
                Box::new(random_sequence::RandomSequence::new(config.clone()))
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            EffectCommand::Randomsequence(_) => "randomsequence",
        }
    }
}
