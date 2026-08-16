pub mod binarypath;
pub mod bouncyballs;
pub mod burn;
pub mod colorshift;
pub mod crumble;
pub mod decrypt;
pub mod errorcorrect;
pub mod expand;
pub mod highlight;
pub mod laseretch;
pub mod matrix;
pub mod rain;
pub mod beams;
pub mod bubbles;
pub mod fireworks;
pub mod middleout;
pub mod orbittingvolley;
pub mod overflow;
pub mod pour;
pub mod print;
pub mod random_sequence;
pub mod rings;
pub mod scattered;
pub mod slice;
pub mod slide;
pub mod smoke;
pub mod spotlights;
pub mod spray;
pub mod swarm;
pub mod sweep;
pub mod synthgrid;
pub mod thunderstorm;
pub mod unstable;
pub mod vhstape;
pub mod waves;
pub mod wipe;
pub mod blackhole;

pub trait Effect {
    fn name(&self) -> &str;
    fn frames(&self, input: &str) -> Vec<String>;
}

pub fn registry() -> Vec<Box<dyn Effect>> {
    vec![
        Box::new(binarypath::Binarypath::new()),
        Box::new(bouncyballs::Bouncyballs::new()),
        Box::new(burn::Burn::new()),
        Box::new(colorshift::Colorshift::new()),
        Box::new(crumble::Crumble::new()),
        Box::new(decrypt::Decrypt::new()),
        Box::new(errorcorrect::Errorcorrect::new()),
        Box::new(expand::Expand::new()),
        Box::new(highlight::Highlight::new()),
        Box::new(laseretch::Laseretch::new()),
        Box::new(matrix::Matrix::new()),
        Box::new(rain::Rain::new()),
        Box::new(beams::Beams::new()),
        Box::new(bubbles::Bubbles::new()),
        Box::new(fireworks::Fireworks::new()),
        Box::new(middleout::Middleout::new()),
        Box::new(orbittingvolley::Orbittingvolley::new()),
        Box::new(overflow::Overflow::new()),
        Box::new(pour::Pour::new()),
        Box::new(print::Print::new()),
        Box::new(random_sequence::RandomSequence::new()),
        Box::new(rings::Rings::new()),
        Box::new(scattered::Scattered::new()),
        Box::new(slice::Slice::new()),
        Box::new(slide::Slide::new()),
        Box::new(smoke::Smoke::new()),
        Box::new(spotlights::Spotlights::new()),
        Box::new(spray::Spray::new()),
        Box::new(swarm::Swarm::new()),
        Box::new(sweep::Sweep::new()),
        Box::new(synthgrid::Synthgrid::new()),
        Box::new(thunderstorm::Thunderstorm::new()),
        Box::new(unstable::Unstable::new()),
        Box::new(vhstape::Vhstape::new()),
        Box::new(waves::Waves::new()),
        Box::new(wipe::Wipe::new()),
        Box::new(blackhole::Blackhole::new())
    ]
}
