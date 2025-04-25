use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::debug;

use crate::cli::Cli;

mod error;

use error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub default_game: Option<String>,
  #[serde(default = "default_cache_dir")]
  pub cache_dir: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      default_game: None,
      cache_dir: default_cache_dir(),
    }
  }
}

fn default_cache_dir() -> PathBuf {
  ProjectDirs::from("com", "zerodegress", "kmf")
    .expect("project dir unavailable")
    .cache_dir()
    .to_path_buf()
}

impl Config {
  pub async fn try_from_cli(cli: &Cli) -> Result<Self, Error> {
    let config = cli.config.to_owned().map(async |config| {
      let config = fs::read_to_string(config).await?;
      Ok::<_, Error>(toml::from_str::<Config>(config.as_str())?)
    });

    let config = if let Some(config) = config {
      config.await?
      // TODO: 此处本应检查config是不是有效的
    } else {
      Config::default()
    };

    debug!("cache_dir: {:?}", config.cache_dir);

    Ok(config)
  }
}
