use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, warn};

use crate::cli::Cli;

mod error;

use error::Error;

type Result<T> = std::result::Result<T, Error>;

/// Kmf Config
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub default_game: Option<String>,
  /// Cache directory
  #[serde(default = "default_cache_dir")]
  pub cache_dir: PathBuf,
  /// Progress draw target
  #[serde(default = "default_progress_draw_target")]
  pub progress_draw_target: ProgressDrawTargetType,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      default_game: None,
      cache_dir: default_cache_dir(),
      progress_draw_target: default_progress_draw_target(),
    }
  }
}

/// Progress draw target type.
/// Stdout: write progress bar to `stdout`.
/// Hidden: do not write progress bar.
#[derive(Debug, Serialize, Deserialize)]
pub enum ProgressDrawTargetType {
  Stdout,
  Hidden,
}

fn default_cache_dir() -> PathBuf {
  ProjectDirs::from("com", "zerodegress", "kmf")
    .expect("project dir unavailable")
    .cache_dir()
    .to_path_buf()
}

fn default_progress_draw_target() -> ProgressDrawTargetType {
  ProgressDrawTargetType::Stdout
}

impl Config {
  /// Construct Config from config file
  pub async fn try_from_config_file(config_file: &Path) -> Result<Self> {
    let config = fs::read_to_string(config_file).await?;
    let config = toml::from_str::<Config>(config.as_str()).unwrap_or_else(|err| {
      warn!("error when deserialize config file: {:?}", err);
      Config::default()
    });
    Ok(config)
  }

  /// Construct Config from cli and config file
  pub async fn try_from_cli(cli: &Cli) -> Result<Self> {
    let config = cli
      .config
      .to_owned()
      .map(async |config| Self::try_from_config_file(config.as_path()).await);

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
