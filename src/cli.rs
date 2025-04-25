use std::path::PathBuf;

use clap::Parser;
use url::Url;

#[derive(Debug, Parser)]
pub struct Cli {
  #[arg(short, long)]
  pub config: Option<PathBuf>,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
  Install {
    url: Vec<Url>,
    #[arg(long)]
    game: Option<Url>,
  },
}
