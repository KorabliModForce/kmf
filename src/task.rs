use url::Url;

use crate::cli::{Cli, Command};

pub enum Task {
  Install { url: Vec<Url>, game: Option<Url> },
}

impl Task {
  pub fn from_cli(cli: &Cli) -> Vec<Task> {
    match &cli.command {
      Command::Install { url, game } => vec![Task::Install {
        url: url.to_owned(),
        game: game.to_owned(),
      }],
    }
  }
}
