use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  time::Duration,
};

use crate::{
  config::Config,
  resolve::{Resolved, Resolver},
  task::Task,
  util::{async_copy_dir, empty_dir, get_game_versions, io_copy_with_progressbar, unzip_file},
};
use cache_download_record::CacheDownloadRecord;
use chrono::TimeZone;
use futures::TryStreamExt;
use headers::{ContentType, HeaderMapExt};
use indicatif::{MultiProgress, ProgressBar};
use temp_dir::TempDir;
use tokio::{
  fs::{self, File},
  io::AsyncWriteExt,
};

mod cache_download_record;
mod error;

use error::Error;
use tokio_util::io::StreamReader;
use tracing::debug;
use url::Url;

pub struct Kmf {
  default_game: Option<Url>,
  cache_dir: PathBuf,
  reqwest_client: reqwest_middleware::ClientWithMiddleware,
  resolver: Resolver,
  multi_progress: MultiProgress,
}

impl Kmf {
  pub async fn try_from_config(config: &Config) -> Result<Self, Error> {
    let default_game = config
      .default_game
      .as_ref()
      .map(|x| x.parse().expect("invalid config 'default_game'"));
    let cache_dir = {
      let cache_dir = config.cache_dir.to_owned();
      debug!("cache dir: {cache_dir:?}");
      if !fs::try_exists(cache_dir.as_path()).await? {
        fs::create_dir_all(cache_dir.as_path()).await?;
      }
      if !fs::metadata(cache_dir.as_path()).await?.is_dir() {
        return Err(Error::CacheDirIsNotADir);
      }
      cache_dir.to_owned()
    };

    let client = reqwest::Client::new();
    let client = reqwest_middleware::ClientBuilder::new(client).build();

    let multi_progress = MultiProgress::new();

    Ok(Self {
      default_game,
      cache_dir,
      reqwest_client: client,
      resolver: Resolver::new()?,
      multi_progress,
    })
  }

  async fn cache_download_record(&self) -> Result<HashMap<String, CacheDownloadRecord>, Error> {
    let cache_download_record = self.cache_dir.join("cache_download_record.toml");
    if !fs::try_exists(cache_download_record.as_path()).await? {
      let mut file = fs::File::create_new(cache_download_record.as_path()).await?;
      let default_record = HashMap::<String, CacheDownloadRecord>::default();
      file
        .write_all(toml::to_string(&default_record)?.as_bytes())
        .await?;
      return Ok(default_record);
    }
    let cache_download_record = fs::read_to_string(cache_download_record.as_path()).await?;
    let cache_download_record = toml::from_str(cache_download_record.as_str())?;
    Ok(cache_download_record)
  }

  async fn set_cache_download_record(
    &self,
    record: &HashMap<String, CacheDownloadRecord>,
  ) -> Result<(), Error> {
    fs::write(
      self.cache_dir.join("cache_download_record.toml"),
      toml::to_string(&record)?.as_bytes(),
    )
    .await?;

    Ok(())
  }

  async fn cache_mod_to_dir(&self, source: &Url, dir: &Path) -> Result<(), Error> {
    let temp_dir = TempDir::with_prefix("kmf")?;
    let zip_file = match source.scheme() {
      "http" | "https" => {
        let Resolved { content_length, .. } = self.resolver.resolve(source.to_owned()).await?;

        let file_path = temp_dir.path().join("mod.zip");

        let res = self.reqwest_client.get(source.as_ref()).send().await?;

        debug!(
          "response content_type: {}",
          res.headers().typed_get::<ContentType>().unwrap()
        );

        debug!("downloading");
        io_copy_with_progressbar(
          StreamReader::new(
            res
              .bytes_stream()
              .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
          ),
          File::create_new(file_path.as_path()).await?,
          content_length,
          "Downloading",
          Some(&self.multi_progress),
        )
        .await?;
        debug!("download completed");
        File::open(file_path.as_path()).await?
      }
      _ => todo!(),
    };

    if fs::try_exists(dir).await? {
      empty_dir(dir).await?;
    } else {
      fs::create_dir_all(dir).await?;
    }
    debug!("dir ensured: {dir:?}");

    unzip_file(zip_file, dir).await?;

    Ok(())
  }

  async fn create_cache_mod(&self, source: Url) -> Result<(PathBuf, String), Error> {
    let mut record = self.cache_download_record().await?;
    let Resolved {
      id,
      source,
      specifier,
      ..
    } = self.resolver.resolve(source.to_owned()).await?;
    let mods_dir = self.cache_dir.join("mods");
    if !fs::try_exists(mods_dir.as_path()).await? {
      fs::create_dir_all(mods_dir.as_path()).await?;
    }
    let mod_dir = mods_dir.join(id.as_str());

    self.cache_mod_to_dir(&source, mod_dir.as_path()).await?;

    record.insert(
      id.to_owned(),
      CacheDownloadRecord::new(specifier, source, chrono::Local::now().naive_local()),
    );

    self.set_cache_download_record(&record).await?;

    Ok((mod_dir, id))
  }

  async fn ensure_lastest_cache_mod(&self, id: &str) -> Result<bool, Error> {
    let record = self.cache_download_record().await?;
    let Some(cache_record) = record.get(id) else {
      return Ok(false);
    };
    let cache_mod_root = self.cache_dir.join("mods").join(id);

    let last_updated = cache_record.last_updated();

    let last_updated = chrono::Local
      .from_local_datetime(&last_updated)
      .single()
      .expect("datetime stored should always be valid");

    let Resolved {
      last_updated: last_modified,
      source,
      specifier,
      ..
    } = self.resolver.resolve(cache_record.source()).await?;

    if last_modified > last_updated {
      let mut record = record.clone();
      record.insert(
        id.to_string(),
        CacheDownloadRecord::new(
          specifier,
          source.to_owned(),
          chrono::Local::now().naive_local(),
        ),
      );
      self
        .cache_mod_to_dir(&source, cache_mod_root.as_path())
        .await?;
      self.set_cache_download_record(&record).await?;
    }

    Ok(true)
  }

  async fn pick_cache_mod(&self, source: &Url) -> Result<(PathBuf, String), Error> {
    let Resolved { specifier, .. } = self.resolver.resolve(source.to_owned()).await?;
    let cache_record = self.cache_download_record().await?;
    if let Some((id, _)) = cache_record
      .iter()
      .find(|(_, x)| x.specifier() == specifier)
    {
      self.ensure_lastest_cache_mod(id).await?;
      let mod_dir = self.cache_dir.join("mods").join(id);
      Ok((mod_dir, id.to_owned()))
    } else {
      self.create_cache_mod(source.to_owned()).await
    }
  }

  async fn task_install(&self, url: &Url, game: &Url) -> Result<(), Error> {
    let pb = self.multi_progress.add(ProgressBar::new_spinner());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("缓存中");
    let (mod_cache_root, _id) = self.pick_cache_mod(url).await?;
    pb.set_message("缓存完成");
    pb.finish();

    let pb = self.multi_progress.add(ProgressBar::new_spinner());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("检查游戏版本中");
    let game_root = match game.scheme() {
      "file" => game.path(),
      _ => todo!(),
    };
    let game_root = PathBuf::from(game_root);
    let game_root = game_root.as_path();
    let version = {
      let version = game
        .query_pairs()
        .find_map(|(k, v)| if k == "version" { Some(v) } else { None });
      let versions = get_game_versions(game_root).await?;
      if let Some(version) = version {
        if versions
          .iter()
          .any(|x| x.as_str() == version.to_string().as_str())
        {
          version.to_string()
        } else {
          return Err(Error::VersionNotFound {
            version: version.to_string(),
          });
        }
      } else {
        versions[0].to_owned()
      }
    };
    pb.set_message("已找到最新版本");
    pb.finish();

    let res_mods_root = PathBuf::from(game_root)
      .join("bin")
      .join(version)
      .join("res_mods");

    let pb = self.multi_progress.add(ProgressBar::new_spinner());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("安装中");
    async_copy_dir(mod_cache_root, res_mods_root).await?;
    pb.set_message("安装完成");
    pb.finish();

    Ok(())
  }

  pub async fn run(&self, task: Task) -> Result<(), Error> {
    match task {
      Task::Install { url, game } => {
        let game = game
          .or(self.default_game.to_owned())
          .ok_or(Error::GameNotSpecified)?;
        for url in url {
          self.task_install(&url, &game).await?;
        }
        Ok(())
      }
    }
  }
}
