//!
//! xtask building block operations such as copy, remove, confirm, and more
//!
//!

use anyhow::{anyhow, Result as AnyResult};
use dialoguer::{theme::ColorfulTheme, Confirm};
use fs_extra as fsx;
use fsx::dir::CopyOptions;
use glob::glob;
use serde_json::Value;
use std::{
    env,
    fs::{create_dir_all, read_dir, remove_dir_all},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    str::FromStr,
};

pub use duct::cmd;
///
/// Remove a set of files given a glob
///
/// # Errors
/// Fails if listing or removal fails
///
pub fn clean_files(pattern: &str) -> AnyResult<()> {
    let files: Result<Vec<PathBuf>, _> = glob(pattern)?.collect();
    files?.iter().try_for_each(remove_file)
}
/// . removes all contents of directory or create it recursively if it does not exist
///
/// # Errors
///
/// This function will return an error if .
pub fn get_clean_directory(path: &PathBuf) -> io::Result<()> {
    if !path.exists() {
        create_dir_all(path)
    } else if path.is_dir() {
        remove_dir_all(path)
    } else {
        Err(io::Error::new(
            ErrorKind::NotADirectory,
            "path is not a directory",
        ))
    }
}

///
/// Remove a single file
///
/// # Errors
/// Fails if removal fails
///
pub fn remove_file<P>(path: P) -> AnyResult<()>
where
    P: AsRef<Path>,
{
    fsx::file::remove(path).map_err(anyhow::Error::msg)
}

///
/// Remove a directory with its contents
///
/// # Errors
/// Fails if removal fails
///
pub fn remove_dir<P>(path: P) -> AnyResult<()>
where
    P: AsRef<Path>,
{
    fsx::dir::remove(path).map_err(anyhow::Error::msg)
}

///
/// Check if path exists
///
pub fn exists<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    std::path::Path::exists(path.as_ref())
}

///
/// Copy entire folder contents
///
/// # Errors
/// Fails if file operations fail
///
pub fn copy_contents<P, Q>(from: P, to: Q, overwrite: bool) -> AnyResult<u64>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut opts = CopyOptions::new();
    opts.content_only = true;
    opts.overwrite = overwrite;
    fsx::dir::copy(from, to, &opts).map_err(anyhow::Error::msg)
}
///
/// Move entire folder contents
pub fn move_contents<P, Q>(from: P, to: Q, overwrite: bool) -> AnyResult<u64>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let mut opts = CopyOptions::new();
    opts.content_only = true;
    opts.overwrite = overwrite;
    fsx::dir::move_dir(from, to, &opts).map_err(anyhow::Error::msg)
}

///
/// Prompt the user to confirm an action
///
/// # Panics
/// Panics if input interaction fails
///
pub fn confirm(question: &str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(question)
        .interact()
        .unwrap()
}

///
/// Gets the cargo root dir
///
pub fn root_dir() -> PathBuf {
    let mut xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.pop();
    xtask_dir
}
/// .Return the closest anchestor containing a Cargo.toml file
pub fn nearest_cargo_dir() -> Result<PathBuf, io::Error> {
    let path = env::current_dir()?;
    let path_ancestors = path.as_path().ancestors();

    for p in path_ancestors {
        let has_cargo = !read_dir(p)?.all(|p| p.unwrap().file_name() != *"Cargo.toml");
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find Cargo.toml",
    ))
}

pub(crate) fn get_cargo_metadata() -> AnyResult<serde_json::Value> {
    let metadata = cmd!("cargo", "metadata", "--format-version", "1").read()?;
    let metadata: Value = serde_json::from_str(&metadata)?;
    Ok(metadata)
}
/// .
/// Returns the root of the workspace
/// # Errors
///
/// This function will return an error if the workspace could not be found.
pub fn get_workspace_root() -> AnyResult<PathBuf> {
    let metadata = get_cargo_metadata()?;
    let path = metadata
        .get("workspace_root")
        .ok_or(anyhow!("Deserialization error"))?
        .to_string()
        .replace('\"', "");

    Ok(PathBuf::from_str(&path)?)
}
