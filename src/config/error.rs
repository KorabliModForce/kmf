#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("io: {0}")]
  Io(#[from] std::io::Error),
  #[error("toml::de: {0}")]
  TomlDe(#[from] toml::de::Error),
}
