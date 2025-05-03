use crate::{resolver, util};

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("game not specified")]
  GameNotSpecified,
  #[error("version not found: {version}")]
  VersionNotFound { version: String },
  #[error("std::io: {0}")]
  Io(#[from] std::io::Error),
  #[error("kmf::util::get_game_versions: {0}")]
  UtilGetGameVersions(#[from] util::GetGameVersionsError),
  #[error("kmf::util::unzip_file: {0}")]
  UtilUnzipFile(#[from] util::error::UnzipFileError),
  #[error("reqwest: {0}")]
  Reqwest(#[from] reqwest::Error),
  #[error("reqwest_middleware: {0}")]
  ReqwestMiddleware(#[from] reqwest_middleware::Error),
  #[error("toml::de: {0}")]
  TomlDe(#[from] toml::de::Error),
  #[error("toml::ser: {0}")]
  TomlSer(#[from] toml::ser::Error),
  #[error("resolver: {0}")]
  Resolver(#[from] resolver::Error),
  #[error("mod not found")]
  ModNotFound,
}
