#[derive(Debug, thiserror::Error)]
pub enum GetGameVersionsError {
  #[error("Game dir is not a dir")]
  GameDirIsNotADir,
  #[error("Illegal game dir structure")]
  IllegalGameDirStructure,
  #[error("std::io: {0}")]
  Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum UnzipFileError {
  #[error("async_zip: {0}")]
  AsyncZipError(#[from] async_zip::error::ZipError),
  #[error("std::io: {0}")]
  Io(#[from] std::io::Error),
}
