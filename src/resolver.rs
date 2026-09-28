use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use percent_encoding::percent_decode_str;

use crate::{Result, XddError, config::Config};

pub fn resolve(config: &Config, input: &str) -> Result<PathBuf> {
    let (root_name, relative) = parse_url(input)?;
    let root = config
        .roots
        .get(&root_name)
        .ok_or_else(|| XddError::new(format!("unknown root {root_name:?}")))?;
    let root = normalise_absolute_path(root)?;

    let relative = normalise_relative_path(&relative)?;
    let target = root.join(relative);
    if config.restrict_to_root {
        check_containment(&root, &target)?;
    }
    Ok(target)
}

fn check_containment(root: &Path, target: &Path) -> Result<()> {
    let actual_target = match fs::canonicalize(target) {
        Ok(path) => path,
        // Missing targets retain lexical resolution; there is no final target to check yet.
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(XddError::new(format!(
                "cannot resolve target {}: {error}",
                target.display()
            )));
        }
    };
    let actual_root = fs::canonicalize(root).map_err(|error| {
        XddError::new(format!("cannot resolve root {}: {error}", root.display()))
    })?;
    if !actual_target.starts_with(&actual_root) {
        return Err(XddError::new(format!(
            "target resolves outside root: {}",
            target.display()
        )));
    }
    Ok(())
}

fn parse_url(input: &str) -> Result<(String, String)> {
    let remainder = input
        .strip_prefix("xdd://")
        .ok_or_else(|| XddError::new("URL must use the xdd:// scheme"))?;

    if remainder.contains('?') || remainder.contains('#') {
        return Err(XddError::new(
            "query parameters and fragments are not supported",
        ));
    }

    let separator = remainder
        .find(':')
        .ok_or_else(|| XddError::new("URL must contain ':' between root and path"))?;
    let encoded_root = &remainder[..separator];
    let encoded_path = &remainder[separator + 1..];

    let root = decode_component(encoded_root, "root")?;
    if root.is_empty() {
        return Err(XddError::new("URL root must not be empty"));
    }
    if root.contains(':') {
        return Err(XddError::new("URL root must not contain ':'"));
    }

    Ok((root, decode_component(encoded_path, "path")?))
}

fn decode_component(value: &str, component: &str) -> Result<String> {
    validate_percent_encoding(value)?;
    let decoded = percent_decode_str(value)
        .decode_utf8()
        .map_err(|error| XddError::new(format!("invalid UTF-8 in URL {component}: {error}")))?
        .into_owned();

    if decoded.contains('\0') {
        return Err(XddError::new(format!(
            "URL {component} must not contain NUL"
        )));
    }
    Ok(decoded)
}

fn validate_percent_encoding(value: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(XddError::new("URL contains an invalid percent escape"));
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    Ok(())
}

pub fn normalise_absolute_path(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(XddError::new("path must be absolute"));
    }
    normalise_components(path, true)
}

fn normalise_relative_path(path: &str) -> Result<PathBuf> {
    normalise_components(Path::new(path), false)
}

fn normalise_components(path: &Path, absolute: bool) -> Result<PathBuf> {
    let mut normalised = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalised.push(part),
            Component::ParentDir => {
                if !normalised.pop() && !absolute {
                    return Err(XddError::new("path escapes its root"));
                }
            }
            Component::RootDir | Component::Prefix(_) if absolute => {
                normalised.push(component.as_os_str());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(XddError::new("path must be relative to its root"));
            }
        }
    }

    Ok(normalised)
}

#[cfg(test)]
mod tests {
    use super::{normalise_absolute_path, resolve};
    use crate::config::Config;
    use std::collections::HashMap;

    fn config() -> Config {
        Config {
            roots: HashMap::from([(
                "docs".to_owned(),
                std::env::temp_dir().join("xdd-test/Documents"),
            )]),
            restrict_to_root: true,
        }
    }

    #[test]
    fn resolves_encoded_paths() {
        assert_eq!(
            resolve(&config(), "xdd://docs:notes/hello%20world.md").unwrap(),
            config().roots["docs"].join("notes/hello world.md")
        );
    }

    #[test]
    fn allows_dot_segments_without_escaping() {
        assert_eq!(
            resolve(&config(), "xdd://docs:notes/../readme.md").unwrap(),
            config().roots["docs"].join("readme.md")
        );
    }

    #[test]
    fn rejects_escape_attempts() {
        assert!(resolve(&config(), "xdd://docs:../secret").is_err());
        assert!(resolve(&config(), "xdd://docs:%2e%2e/secret").is_err());
        assert!(resolve(&config(), "xdd://docs:/etc/passwd").is_err());
    }

    #[test]
    fn rejects_invalid_url_parts() {
        assert!(resolve(&config(), "xdd://docs:%ZZ").is_err());
        assert!(resolve(&config(), "xdd://docs:file?open=1").is_err());
        assert!(resolve(&config(), "xdd://docs%3A:file").is_err());
    }

    #[test]
    fn normalises_absolute_paths_lexically() {
        let base = std::env::temp_dir();
        assert_eq!(
            normalise_absolute_path(&base.join("test/../docs")).unwrap(),
            base.join("docs")
        );

        let root = base.ancestors().last().unwrap();
        assert_eq!(
            normalise_absolute_path(&root.join("../docs")).unwrap(),
            root.join("docs")
        );
    }

    #[cfg(unix)]
    #[test]
    fn restricts_symlink_targets_to_root() {
        use std::{
            fs,
            os::unix::fs::symlink,
            time::{SystemTime, UNIX_EPOCH},
        };

        struct Fixture(std::path::PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let fixture = Fixture(std::env::temp_dir().join(format!(
            "xdd-guard-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        )));
        let root = fixture.0.join("root");
        let outside = fixture.0.join("outside");
        fs::create_dir_all(root.join("inside")).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(root.join("inside/file"), "inside").unwrap();
        fs::write(outside.join("file"), "outside").unwrap();
        symlink(root.join("inside"), root.join("safe")).unwrap();
        symlink(&outside, root.join("escape")).unwrap();

        let mut config = config();
        config.roots.insert("docs".to_owned(), root.clone());
        assert_eq!(
            resolve(&config, "xdd://docs:safe/file").unwrap(),
            root.join("safe/file")
        );
        assert!(resolve(&config, "xdd://docs:escape/file").is_err());
        assert_eq!(
            resolve(&config, "xdd://docs:escape/missing").unwrap(),
            root.join("escape/missing")
        );
        assert!(
            crate::link::create(
                &config,
                root.join("escape").to_str().unwrap(),
                None,
                crate::link::LinkFormat::Plain,
                None,
            )
            .is_err()
        );

        let root_alias = fixture.0.join("alias");
        symlink(&root, &root_alias).unwrap();
        config.roots.insert("docs".to_owned(), root_alias.clone());
        assert_eq!(
            resolve(&config, "xdd://docs:inside/file").unwrap(),
            root_alias.join("inside/file")
        );
        assert!(resolve(&config, "xdd://docs:escape/file").is_err());

        config.restrict_to_root = false;
        assert_eq!(
            resolve(&config, "xdd://docs:escape/file").unwrap(),
            root_alias.join("escape/file")
        );
    }
}
