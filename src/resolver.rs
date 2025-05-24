use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use url::Url;

use crate::util::error::UnzipFileError;

pub mod impls;

/// Mod error
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("cannot resolve")]
  CannotResolve,
  #[error("reqwest_middleware: {0}")]
  Reqwest(#[from] reqwest_middleware::Error),
  #[error("io: {0}")]
  Io(#[from] std::io::Error),
  #[error("toml::de: {0}")]
  TomlDe(#[from] toml::de::Error),
  #[error("toml::ser: {0}")]
  TomlSer(#[from] toml::ser::Error),
  #[error("UnzipFile: {0}")]
  UnzipFile(#[from] UnzipFileError),
  #[error("url::Parse: {0}")]
  UrlParse(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Info resolved
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ResolveInfo {
  pub id: String,
  pub url: Url,
  pub last_updated: DateTime<Utc>,
  pub size: u64,
}

/// Mod resolver
#[allow(dead_code)]
#[async_trait]
pub trait Resolver {
  fn can_resolve(&self, url: Url) -> bool;
  async fn resolve(&self, url: Url) -> Result<ResolveInfo>;
  async fn is_up_to_date(&self, url: Url) -> Result<bool>;
  async fn cache(&self, url: Url) -> Result<PathBuf>;
  async fn clear_cache(&self) -> Result<()>;
}
