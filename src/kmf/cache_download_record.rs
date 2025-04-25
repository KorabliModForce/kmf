use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheDownloadRecord {
  specifier: String,
  source: String,
  last_update: String,
}

impl CacheDownloadRecord {
  pub fn new(specifier: Url, source: Url, last_update: NaiveDateTime) -> Self {
    Self {
      specifier: specifier.to_string(),
      source: source.to_string(),
      last_update: last_update.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
    }
  }

  pub fn specifier(&self) -> Url {
    self.specifier.parse().expect("invalid url")
  }

  pub fn source(&self) -> Url {
    self.source.parse().expect("invalid url")
  }

  pub fn last_updated(&self) -> NaiveDateTime {
    self.last_update.parse().expect("invalid date time")
  }
}
