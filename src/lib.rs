#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use anni_repo::{
  library::AlbumFolderInfo,
  prelude::{Album, JsonAlbum},
};
use anni_workspace::{AnniWorkspace, ExtractedAlbumInfo, UntrackedWorkspaceAlbum, WorkspaceError};
use napi::bindgen_prelude::Env;
use serde::Serialize;
use std::{
  borrow::Cow,
  error::Error,
  fs::{self},
  num::NonZeroU8,
  path::{Path, PathBuf},
  str::FromStr,
};
use thiserror;
use uuid::Uuid;

// create the error type that represents all errors possible in our program
#[derive(Debug, thiserror::Error)]
enum AnniError {
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  Workspace(#[from] WorkspaceError),
  #[error(transparent)]
  AnniRepo(#[from] anni_repo::error::Error),
}

impl From<AnniError> for napi::Error {
  fn from(value: AnniError) -> Self {
    return napi::Error::new(napi::Status::Unknown, value.to_string());
  }
}

#[allow(dead_code)]
fn convert_error<T: Error>(err: T) -> AnniError
where
  AnniError: From<T>,
{
  return AnniError::from(err);
}

#[allow(dead_code)]
fn serialize_album(json_album: JsonAlbum) -> Result<String, AnniError> {
  let mut album = Album::try_from(json_album).unwrap();
  let album_serialized_text = album.format_to_string();
  Ok(album_serialized_text)
}

// fn deserialize_album(album_toml_str: &str) -> Result<JsonAlbum, AnniError> {
//   let album = Album::from_str(&album_toml_str)?;
//   let album_json = JsonAlbum::from(album);
//   Ok(album_json)
// }

#[allow(dead_code)]
#[napi]
fn read_album_file(env: Env, path: String) -> napi::Result<napi::JsUnknown> {
  let content = fs::read_to_string(path).unwrap();
  let album = Album::from_str(&content).unwrap();
  let album_json = JsonAlbum::from(album);
  let result = env.to_js_value(&album_json)?;
  return Ok(result);
}

#[allow(dead_code)]
#[napi]
fn write_album_file(path: String, album_json_str: String) -> napi::Result<()> {
  let album_json = JsonAlbum::from_str(&album_json_str).unwrap();
  let album_serialized_text = serialize_album(album_json)?;
  fs::write(path, album_serialized_text)?;
  Ok(())
}

#[allow(dead_code)]
#[napi]
fn get_workspace_albums(env: Env, workspace_path: String) -> napi::Result<napi::JsUnknown> {
  let workspace = AnniWorkspace::find(Path::new(&workspace_path)).map_err(convert_error)?;
  let albums = workspace.scan().map_err(convert_error)?;
  let result = env.to_js_value(&albums)?;
  return Ok(result);
}

#[allow(dead_code)]
#[napi]
fn create_album(workspace: String, path: String, disc_num: u8) -> napi::Result<()> {
  let album_id = Uuid::new_v4();
  let workspace_path = Path::new(&workspace);
  let album_path = Path::new(&path);
  let album_disc_num = NonZeroU8::new(disc_num).unwrap_or(NonZeroU8::new(1).unwrap());
  let workspace = AnniWorkspace::find(workspace_path).map_err(convert_error)?;
  workspace
    .create_album(&album_id, &album_path, album_disc_num)
    .map_err(convert_error)?;
  Ok(())
}

#[derive(Serialize)]
struct WorkspaceDiscCopy {
  index: usize,
  path: PathBuf,
  cover: PathBuf,
  tracks: Vec<PathBuf>,
}

#[allow(dead_code)]
#[napi]
fn commit_album_prepare(
  env: Env,
  workspace_path: String,
  album_path: String,
) -> napi::Result<napi::JsUnknown> {
  let workspace = AnniWorkspace::find(Path::new(&workspace_path)).map_err(convert_error)?;
  let mut discs_result: Vec<WorkspaceDiscCopy> = Vec::new();

  let untracked_album = workspace
    .get_untracked_album_overview(&album_path)
    .map_err(convert_error)?;

  for (_, disc) in untracked_album.discs.into_iter().enumerate() {
    discs_result.push(WorkspaceDiscCopy {
      index: disc.index,
      path: disc.path.clone(),
      cover: disc.cover.clone(),
      tracks: disc.tracks.clone(),
    })
  }

  let result = env.to_js_value(&discs_result)?;

  return Ok(result);
}

#[allow(dead_code)]
#[napi]
fn commit_album(workspace_path: String, album_path: String) -> napi::Result<()> {
  let workspace = AnniWorkspace::find(Path::new(&workspace_path)).map_err(convert_error)?;
  let validator = |_album: &UntrackedWorkspaceAlbum| -> bool {
    return true;
  };
  workspace
    .commit(&album_path, Some(validator))
    .map_err(convert_error)?;
  workspace
    .import_tags(
      &album_path,
      |folder_name| {
        let AlbumFolderInfo {
          release_date,
          catalog,
          title,
          edition,
          ..
        } = AlbumFolderInfo::from_str(&folder_name).ok()?;
        Some(ExtractedAlbumInfo {
          title: Cow::Owned(title),
          edition: edition.map(|e| Cow::Owned(e)),
          catalog: Cow::Owned(catalog),
          release_date,
        })
      },
      false,
    )
    .map_err(convert_error)?;

  Ok(())
}

#[allow(dead_code)]
#[napi]
fn publish_album(workspace_path: String, album_path: String) -> napi::Result<()> {
  let workspace = AnniWorkspace::find(Path::new(&workspace_path)).map_err(convert_error)?;
  workspace.apply_tags(&album_path).map_err(convert_error)?;
  workspace
    .publish(album_path, false)
    .map_err(convert_error)?;
  Ok(())
}
