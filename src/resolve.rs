use std::time::SystemTime;

use chrono::{DateTime, Utc};
pub use error::Error;
use headers::{ContentLength, HeaderMapExt, LastModified};
use reqwest_middleware::ClientWithMiddleware;
use url::Url;

use crate::util::generate_url_id;

mod error;

pub struct Resolver {
  reqwest_client: ClientWithMiddleware,
}

impl Resolver {
  pub fn new() -> Result<Self, Error> {
    Ok(Self {
      reqwest_client: {
        let client = reqwest::Client::builder().build()?;

        reqwest_middleware::ClientBuilder::new(client).build()
      },
    })
  }

  pub async fn resolve(&self, specifier: Url) -> Result<Resolved, Error> {
    match specifier.scheme() {
      "http" | "https" => {
        let res = self
          .reqwest_client
          .head(specifier.to_owned())
          .send()
          .await?;

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
        let id = generate_url_id(specifier.as_str());
        let source = res.url().to_owned();

        Ok(Resolved {
          content_length,
          id,
          last_updated,
          specifier,
          source,
        })
      }
      _ => todo!("实现更多MOD URL"),
    }
  }
}

pub struct Resolved {
  pub id: String,
  pub specifier: Url,
  pub source: Url,
  pub last_updated: DateTime<Utc>,
  pub content_length: u64,
}
