#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("reqwest: {0}")]
  Reqwest(#[from] reqwest::Error),
  #[error("reqwest_middleware: {0}")]
  ReqwestMiddleware(#[from] reqwest_middleware::Error),
}
