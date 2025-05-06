use std::{collections::HashMap, path::PathBuf, time::SystemTime};

use crate::{
  resolver::{Error, ResolveInfo, Result},
  util::{empty_dir, unzip_file},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use headers::{ContentLength, HeaderMapExt, LastModified};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
  fs::{self, File},
  io,
};
use tokio_util::io::StreamReader;
use tracing::debug;
use url::Url;

use crate::resolver::Resolver;

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheRecord {
  url: Url,
  last_updated: DateTime<Utc>,
}

impl From<ResolveInfo> for CacheRecord {
  fn from(value: ResolveInfo) -> Self {
    Self {
      last_updated: value.last_updated,
      url: value.url,
    }
  }
}

pub struct WebResolver {
  cache_record_file: PathBuf,
  cache_dir: PathBuf,
  reqwest_client: reqwest_middleware::ClientWithMiddleware,
}

impl WebResolver {
  pub fn new(
    cache_record_file: PathBuf,
    cache_dir: PathBuf,
    ca_cache_dir: PathBuf,
  ) -> Result<Self> {
    Ok(Self {
      cache_record_file,
      cache_dir,
      reqwest_client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(Cache(HttpCache {
          mode: CacheMode::Default,
          manager: CACacheManager { path: ca_cache_dir },
          options: HttpCacheOptions::default(),
        }))
        .build(),
    })
  }
}

impl WebResolver {
  async fn read_cache_record(&self) -> Result<HashMap<String, CacheRecord>> {
    Ok(toml::from_str(
      fs::read_to_string(self.cache_record_file.as_path())
        .await?
        .as_str(),
    )?)
  }

  async fn write_cache_record(&self, cache_record: &HashMap<String, CacheRecord>) -> Result<()> {
    Ok(
      fs::write(
        self.cache_record_file.as_path(),
        toml::to_string(cache_record)?.as_bytes(),
      )
      .await?,
    )
  }
}

impl WebResolver {
  pub fn can_resolve(&self, url: Url) -> bool {
    matches!(url.scheme(), "http" | "https")
  }

  pub async fn resolve(&self, url: Url) -> Result<ResolveInfo> {
    if !self.can_resolve(url.to_owned()) {
      return Err(Error::CannotResolve);
    }

    let res = self.reqwest_client.head(url.to_owned()).send().await?;

    let headers = res.headers();
    let content_length = headers
      .typed_get::<ContentLength>()
      .map(|x| x.0)
      .unwrap_or_default();
    let last_updated = headers
      .typed_get::<LastModified>()
      .map(Into::<SystemTime>::into)
      .map(|x| x.into())
      .unwrap_or_default();
    let id = hex::encode(Sha256::digest(url.as_str().as_bytes()).to_vec().as_slice());

    Ok(ResolveInfo {
      size: content_length,
      id,
      url,
      last_updated,
    })
  }

  pub async fn is_up_to_date(&self, url: Url) -> Result<bool> {
    let cache_record = self.read_cache_record().await?;
    let Some((_, cache_record)) = cache_record.iter().find(|(_, v)| v.url == url) else {
      return Ok(false);
    };
    let latest_resolve_info = self.resolve(url.to_owned()).await?;
    Ok(cache_record.last_updated == latest_resolve_info.last_updated)
  }

  pub async fn cache(&self, url: Url) -> Result<PathBuf> {
    let resolve_info = self.resolve(url.to_owned()).await?;
    let cache_dir = self.cache_dir.join(resolve_info.id.as_str());
    if self.is_up_to_date(url.to_owned()).await? {
      debug!("reuse current cache: {:?}", cache_dir);
      // 不需要重新缓存
      return Ok(cache_dir.to_owned());
    }

    let cache_record = resolve_info.to_owned().into();
    let mut cache_records = self.read_cache_record().await?;
    cache_records.insert(resolve_info.id.to_string(), cache_record);
    let res = self.reqwest_client.get(url.to_owned()).send().await?;
    debug!("make temp dir");
    let temp_dir = temp_dir::TempDir::new()?;
    let temp_file = temp_dir.path().join("cache");
    {
      let mut read = StreamReader::new(
        res
          .bytes_stream()
          .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err)),
      );
      let mut write = File::options()
        .create_new(true)
        .write(true)
        .open(temp_file.as_path())
        .await?;
      io::copy(&mut read, &mut write).await?;
    }
    debug!("empty cache dir: {:?}", cache_dir);
    empty_dir(cache_dir.as_path()).await?;
    debug!("unzip {:?} -> {:?}", temp_file, cache_dir);
    unzip_file(File::open(temp_file.as_path()).await?, cache_dir.as_path()).await?;
    self.write_cache_record(&cache_records).await?;
    Ok(cache_dir)
  }

  pub async fn clear_cache(&self) -> Result<()> {
    empty_dir(self.cache_dir.as_path()).await?;
    Ok(())
  }
}

#[async_trait]
impl Resolver for WebResolver {
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
