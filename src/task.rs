use url::Url;

use crate::cli::{Cli, Command};

pub enum Task {
  /// Install mods
  Install {
    /// Mods url
    /// Note: only supports `https`/`http` or `kmf`
    url: Vec<Url>,
    /// Game url
    /// Note: only supports `file` scheme for now
    game: Option<Url>,
  },
}

impl Task {
  /// Construct task from cli
  pub fn from_cli(cli: &Cli) -> Vec<Task> {
    match &cli.command {
      Command::Install { url, game } => vec![Task::Install {
        url: url.to_owned(),
        game: game.to_owned(),
      }],
    }
  }
}
