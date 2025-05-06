use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;
use url::Url;

use crate::resolver::{Error, ResolveInfo, Resolver, Result};

use super::web::WebResolver;

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheRecord {
  url: Url,
  web_url: Url,
  last_updated: DateTime<Utc>,
}

pub struct KmfResolver {
  station_url_base: Url,
  inner: WebResolver,
}

impl KmfResolver {
  pub fn new(
    cache_record_file: PathBuf,
    cache_dir: PathBuf,
    ca_cache_dir: PathBuf,
  ) -> Result<Self> {
    Ok(Self {
      station_url_base: Url::parse("https://kmf-station.zice.top/").expect("it should be ok"),
      inner: WebResolver::new(cache_record_file, cache_dir, ca_cache_dir)?,
    })
  }

  fn extract_url(url: Url) -> Option<(String, String)> {
    if matches!(url.scheme(), "kmf") {
      let (modid, version) = url
        .path()
        .split_once('@')
        .unwrap_or_else(|| (url.path(), "latest"));
      Some((modid.to_string(), version.to_string()))
    } else {
      None
    }
  }
}

impl KmfResolver {
  fn translate_url_to_web(&self, url: Url) -> Result<Url> {
    if !self.can_resolve(url.to_owned()) {
      return Err(Error::CannotResolve);
    }
    let (modid, version) = url
      .path()
      .split_once('@')
      .unwrap_or_else(|| (url.path(), "latest"));
    Ok(
      self
        .station_url_base
        .join("mod/")?
        .join(format!("{}/", modid).as_str())?
        .join(version)?,
    )
  }

  pub fn can_resolve(&self, url: Url) -> bool {
    matches!(url.scheme(), "kmf")
  }

  pub async fn resolve(&self, url: Url) -> Result<ResolveInfo> {
    if !self.can_resolve(url.to_owned()) {
      return Err(Error::CannotResolve);
    }
    let (modid, _version) = Self::extract_url(url.to_owned()).expect("it should be ok");

    let web_url = self.translate_url_to_web(url.to_owned())?;
    let web_resolve_info = self.inner.resolve(web_url).await?;

    Ok(ResolveInfo {
      id: modid.to_owned(),
      url: url.to_owned(),
      last_updated: web_resolve_info.last_updated,
      size: web_resolve_info.size,
    })
  }

  pub async fn is_up_to_date(&self, url: Url) -> Result<bool> {
    if !self.can_resolve(url.to_owned()) {
      return Err(Error::CannotResolve);
    }
    let (modid, version) = Self::extract_url(url.to_owned()).expect("it should be ok");

    self
      .inner
      .is_up_to_date(
        self
          .station_url_base
          .join("mod")?
          .join(modid.as_str())?
          .join(version.as_str())?,
      )
      .await
  }

  pub async fn cache(&self, url: Url) -> Result<PathBuf> {
    if !self.can_resolve(url.to_owned()) {
      return Err(Error::CannotResolve);
    }
    let (modid, version) = Self::extract_url(url.to_owned()).expect("it should be ok");

    debug!("modid, version: {}, {}", modid, version);

    let web_url = self.translate_url_to_web(url.to_owned())?;

    debug!("URL: {}", web_url);

    self.inner.cache(web_url).await
  }

  pub async fn clear_cache(&self) -> Result<()> {
    self.inner.clear_cache().await
  }
}

#[async_trait]
impl Resolver for KmfResolver {
  fn can_resolve(&self, url: Url) -> bool {
    self.can_resolve(url)
  }
  async fn resolve(&self, url: Url) -> Result<ResolveInfo> {
    self.resolve(url).await
  }
  async fn is_up_to_date(&self, url: Url) -> Result<bool> {
    self.is_up_to_date(url).await
  }
  async fn cache(&self, url: Url) -> Result<PathBuf> {
    self.cache(url).await
  }
  async fn clear_cache(&self) -> Result<()> {
    self.clear_cache().await
  }
}
