#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),
}
