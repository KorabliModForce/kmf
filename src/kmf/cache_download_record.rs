use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheDownloadRecord {
  source: String,
  last_update: String,
}

impl CacheDownloadRecord {
  pub fn new(source: Url, last_update: NaiveDateTime) -> Self {
    Self {
      source: source.to_string(),
      last_update: last_update.format("%Y-%m-%dT%H:%M:%S%.f").to_string(),
    }
  }

  pub fn source(&self) -> Url {
    self.source.parse().expect("invalid url")
  }

  pub fn last_updated(&self) -> NaiveDateTime {
    self.last_update.parse().expect("invalid date time")
  }
}
