use std::path::PathBuf;

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
        #[arg(short = 'p', long = "player")]
        player_dirs: Vec<PathBuf>,
    },
    ShowReplay {
        #[arg()]
        recording: PathBuf,
    },
}
