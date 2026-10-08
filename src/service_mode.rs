use clap::ValueEnum;
#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Documentation,
    Validation,
}

