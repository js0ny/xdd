use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use directories::BaseDirs;
use serde::Deserialize;

use crate::{Result, XddError};

#[derive(Debug)]
pub struct Config {
    pub roots: HashMap<String, PathBuf>,
    pub restrict_to_root: bool,
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    roots: HashMap<String, String>,
    config: Option<FileOptions>,
}

#[derive(Debug, Deserialize)]
struct FileOptions {
    restrict_to_root: Option<bool>,
}

pub fn load() -> Result<Config> {
    let path = path()?;
    let contents = fs::read_to_string(&path).map_err(|error| {
        XddError::new(format!(
            "cannot read configuration {}: {error}",
            path.display()
        ))
    })?;
    let file_config: FileConfig = toml::from_str(&contents).map_err(|error| {
        XddError::new(format!(
            "cannot parse configuration {}: {error}",
            path.display()
        ))
    })?;

    let home = BaseDirs::new()
        .ok_or_else(|| XddError::new("cannot determine the user's home directory"))?
        .home_dir()
        .to_path_buf();

    let restrict_to_root = file_config
        .config
        .and_then(|options| options.restrict_to_root)
        .unwrap_or(true);

    let roots = file_config
        .roots
        .into_iter()
        .map(|(name, path)| {
            if name.is_empty() {
                return Err(XddError::new("root names must not be empty"));
            }
            if name.contains(':') {
                return Err(XddError::new(format!(
                    "root name {name:?} must not contain ':'"
                )));
            }

            let path = expand_home(&path, &home)?;
            if !path.is_absolute() {
                return Err(XddError::new(format!(
                    "root {name:?} must use an absolute path"
                )));
            }

            Ok((name, path))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    Ok(Config {
        roots,
        restrict_to_root,
    })
}

pub fn path() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| XddError::new("cannot determine the user's configuration directory"))?;

    let config_dir = env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| base_dirs.config_dir().to_path_buf());
    if !config_dir.is_absolute() {
        return Err(XddError::new("XDG_CONFIG_HOME must be an absolute path"));
    }

    Ok(config_dir.join("xdd").join("config.toml"))
}

pub fn create_if_missing() -> Result<PathBuf> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            XddError::new(format!(
                "cannot create configuration directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => file.write_all(b"[roots]\n").map_err(|error| {
            XddError::new(format!("cannot initialise {}: {error}", path.display()))
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(XddError::new(format!(
                "cannot create configuration {}: {error}",
                path.display()
            )));
        }
    }

    Ok(path)
}

pub fn expand_user_path(value: &str) -> Result<PathBuf> {
    let home = BaseDirs::new()
        .ok_or_else(|| XddError::new("cannot determine the user's home directory"))?
        .home_dir()
        .to_path_buf();
    expand_home(value, &home)
}

fn expand_home(value: &str, home: &std::path::Path) -> Result<PathBuf> {
    if value == "~" {
        return Ok(home.to_path_buf());
    }
    if let Some(path) = value.strip_prefix("~/") {
        return Ok(home.join(path));
    }
    if value.starts_with('~') {
        return Err(XddError::new(format!(
            "unsupported home-directory expansion in root path {value:?}"
        )));
    }
    Ok(PathBuf::from(value))
}

#[cfg(test)]
mod tests {
    use super::{FileConfig, expand_home};
    use std::path::Path;

    #[test]
    fn expands_home_paths() {
        assert_eq!(
            expand_home("~", Path::new("/home/test")).unwrap(),
            Path::new("/home/test")
        );
        assert_eq!(
            expand_home("~/Documents", Path::new("/home/test")).unwrap(),
            Path::new("/home/test/Documents")
        );
    }

    #[test]
    fn parses_restrict_to_root_option() {
        let default: FileConfig = toml::from_str("[roots]\n").unwrap();
        assert!(default.config.is_none());

        let configured: FileConfig =
            toml::from_str("[roots]\n[config]\nrestrict_to_root = false\n").unwrap();
        assert!(!configured.config.unwrap().restrict_to_root.unwrap());
    }

    #[test]
    fn rejects_named_home_paths() {
        assert!(expand_home("~other/Documents", Path::new("/home/test")).is_err());
    }
}
