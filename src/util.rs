use std::path::{Path, PathBuf};

use async_zip::base::read::seek::ZipFileReader;
use error::UnzipFileError;
use futures::{FutureExt, future::BoxFuture};
use indicatif::{MultiProgress, ProgressBar};
use tokio::{
  fs::{self, File, OpenOptions, create_dir_all},
  io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
};

use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub mod error;

pub use error::GetGameVersionsError;

/// Returns a relative path without reserved names, redundant separators, ".", or "..".
pub fn sanitize_file_path(path: &str) -> PathBuf {
  // Replaces backwards slashes
  path
    .replace('\\', "/")
    // Sanitizes each component
    .split('/')
    .map(sanitize_filename::sanitize)
    .collect()
}

/// Extracts everything from the ZIP archive to the output directory
pub async fn unzip_file(archive: File, out_dir: &Path) -> Result<(), UnzipFileError> {
  let archive = BufReader::new(archive).compat();
  let mut reader = ZipFileReader::new(archive).await?;
  for index in 0..reader.file().entries().len() {
    let entry = reader.file().entries().get(index).unwrap();
    let path = out_dir.join(sanitize_file_path(entry.filename().as_str().unwrap()));
    // If the filename of the entry ends with '/', it is treated as a directory.
    // This is implemented by previous versions of this crate and the Python Standard Library.
    // https://docs.rs/async_zip/0.0.8/src/async_zip/read/mod.rs.html#63-65
    // https://github.com/python/cpython/blob/820ef62833bd2d84a141adedd9a05998595d6b6d/Lib/zipfile.py#L528
    let entry_is_dir = entry.dir().unwrap();

    let mut entry_reader = reader.reader_without_entry(index).await?;

    if entry_is_dir {
      // The directory may have been created if iteration is out of order.
      if !path.exists() {
        create_dir_all(&path).await?;
      }
    } else {
      // Creates parent directories. They may not exist if iteration is out of order
      // or the archive does not contain directory entries.
      let parent = path
        .parent()
        .expect("A file entry should have parent directories");
      if !parent.is_dir() {
        create_dir_all(parent).await?;
      }
      let writer = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await?;
      futures_lite::io::copy(&mut entry_reader, &mut writer.compat_write()).await?;

      // Closes the file and manipulates its metadata here if you wish to preserve its metadata from the archive.
    }
  }
  Ok(())
}

// 获取游戏根目录下所有的版本
// 默认返回排序为降序
pub async fn get_game_versions(
  game_root: impl AsRef<Path>,
) -> Result<Vec<String>, GetGameVersionsError> {
  let game_root = game_root.as_ref();
  let bin_dir = {
    let bin_dir = game_root.join("bin");
    if !fs::try_exists(bin_dir.as_path()).await? {
      return Err(GetGameVersionsError::IllegalGameDirStructure);
    }
    if !fs::metadata(bin_dir.as_path()).await?.is_dir() {
      return Err(GetGameVersionsError::GameDirIsNotADir);
    }
    bin_dir
  };

  let versions = {
    let mut entries = fs::read_dir(bin_dir.as_path()).await?;
    let mut versions = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
      if !entry.metadata().await?.is_dir() {
        continue;
      }
      let file_name = entry.file_name().to_string_lossy().to_string();
      let Ok(file_name_num) = file_name.parse::<u64>() else {
        continue;
      };
      versions.push((file_name, file_name_num));
    }

    versions.sort_by(|x, y| x.1.cmp(&y.1));
    versions
      .into_iter()
      .map(|(x, _)| x.to_string())
      .rev()
      .collect()
  };

  Ok(versions)
}

#[allow(dead_code)]
pub async fn io_copy_with_progressbar(
  mut read: impl AsyncRead + Unpin,
  mut write: impl AsyncWrite + Unpin,
  len: u64,
  title: impl Into<String>,
  multi_progress: Option<&MultiProgress>,
) -> Result<(), GetGameVersionsError> {
  let pb = multi_progress
    .map(|x| x.add(ProgressBar::new(len)))
    .unwrap_or_else(|| ProgressBar::new(len));
  pb.set_message(title.into());
  let mut buf = [0u8; 1024];
  loop {
    let len = read.read(buf.as_mut()).await?;
    if len == 0 {
      pb.finish();
      return Ok(());
    }

    write.write_all(buf[..len].as_ref()).await?;
    pb.inc(len as u64);
  }
}

pub async fn empty_dir(dir_path: &Path) -> Result<(), std::io::Error> {
  match fs::remove_dir_all(dir_path).await {
    Ok(_) => {}
    Err(err) => match err.kind() {
      std::io::ErrorKind::NotFound => {}
      _ => return Err(err),
    },
  }
  fs::create_dir_all(dir_path).await?;
  Ok(())
}

pub async fn ensure_dir(dir_path: &Path) -> Result<&Path, std::io::Error> {
  if !fs::try_exists(dir_path).await? {
    fs::create_dir_all(dir_path).await?;
  } else if fs::metadata(dir_path).await?.is_file() {
    fs::remove_file(dir_path).await?;
    fs::create_dir_all(dir_path).await?;
  }
  Ok(dir_path)
}

pub async fn ensure_file(file_path: &Path) -> Result<&Path, std::io::Error> {
  if !fs::try_exists(file_path).await? {
    fs::create_dir_all(file_path.parent().expect("File always has parent")).await?;
    File::create_new(file_path).await?;
  } else if fs::metadata(file_path).await?.is_dir() {
    fs::remove_dir_all(file_path).await?;
    File::create_new(file_path).await?;
  }
  Ok(file_path)
}

fn async_copy_dir_inner(
  src: PathBuf,
  dst: PathBuf,
) -> BoxFuture<'static, Result<(), tokio::io::Error>> {
  async move {
    fs::create_dir_all(dst.as_path()).await?;
    let mut entries = fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
      let path = entry.path();
      let target = dst.join(entry.file_name());

      if path.is_dir() {
        async_copy_dir(path, target).await?;
      } else {
        fs::copy(&path, &target).await?;
      }
    }
    Ok(())
  }
  .boxed()
}

pub async fn async_copy_dir(src: PathBuf, dst: PathBuf) -> Result<(), tokio::io::Error> {
  async_copy_dir_inner(src, dst).await
}
