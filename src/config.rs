use std::path::PathBuf;

use clap::*;

#[derive(Debug, Clone, Args)]
pub struct BattleConfiguration {
    #[arg(short = 'c', long = "character")]
    pub characters: Vec<PathBuf>,
    #[arg(short = 'r', long = "rounds", default_value_t = 10)]
    pub rounds: u16,
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(clap::Subcommand, Debug)]
pub enum Mode {
    Battle {
        #[arg(short = 'H', long = "headless", default_value_t = false)]
        headless: bool,
        #[clap(flatten)]
        battle_configuration: BattleConfiguration,
    },
    Replay {
        #[arg()]
        recording: PathBuf,
    },
}
