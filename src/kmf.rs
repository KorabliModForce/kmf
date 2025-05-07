use std::{path::PathBuf, time::Duration};

use crate::{
  config::Config,
  resolver::{
    self,
    impls::{kmf::KmfResolver, web::WebResolver},
  },
  task::Task,
  util::{async_copy_dir, ensure_dir, get_game_versions},
};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};

mod error;

use error::Error;
use url::Url;

pub struct Kmf {
  default_game: Option<Url>,
  multi_progress: MultiProgress,
  resolvers: Vec<Box<dyn resolver::Resolver>>,
}

impl Kmf {
  pub async fn try_from_config(config: &Config) -> Result<Self, Error> {
    let default_game = config
      .default_game
      .as_ref()
      .map(|x| x.parse().expect("invalid config 'default_game'"));
    let cache_dir = ensure_dir(config.cache_dir.as_path()).await?;

    let multi_progress = MultiProgress::with_draw_target(match config.progress_draw_target {
      crate::config::ProgressDrawTargetType::Stdout => ProgressDrawTarget::stdout(),
      crate::config::ProgressDrawTargetType::Hidden => ProgressDrawTarget::hidden(),
    });

    Ok(Self {
      default_game,
      multi_progress,
      resolvers: vec![
        Box::new(KmfResolver::new(cache_dir.join("kmf_resolver")).await?),
        Box::new(WebResolver::new(cache_dir.join("web_resolver")).await?),
      ],
    })
  }

  async fn cache_mod(&self, url: &Url) -> Result<(PathBuf, String), Error> {
    let Some(resolver) = self
      .resolvers
      .iter()
      .find(|r| r.can_resolve(url.to_owned()))
    else {
      return Err(Error::ModNotFound);
    };
    let resolve_info = resolver.resolve(url.to_owned()).await?;
    let dir = resolver.cache(url.to_owned()).await?;
    Ok((dir, resolve_info.id))
  }

  async fn task_install(&self, url: &Url, game: &Url) -> Result<(), Error> {
    let pb = self.multi_progress.add(ProgressBar::new_spinner());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("缓存中");
    let (mod_cache_root, _id) = self.cache_mod(url).await?;
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
