use std::path::{Component, Path, PathBuf};

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
    Ok(root.join(relative))
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
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
    };

    fn config() -> Config {
        Config {
            roots: HashMap::from([("docs".to_owned(), PathBuf::from("/home/test/Documents"))]),
        }
    }

    #[test]
    fn resolves_encoded_paths() {
        assert_eq!(
            resolve(&config(), "xdd://docs:notes/hello%20world.md").unwrap(),
            PathBuf::from("/home/test/Documents/notes/hello world.md")
        );
    }

    #[test]
    fn allows_dot_segments_without_escaping() {
        assert_eq!(
            resolve(&config(), "xdd://docs:notes/../readme.md").unwrap(),
            PathBuf::from("/home/test/Documents/readme.md")
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
        assert_eq!(
            normalise_absolute_path(Path::new("/home/test/../docs")).unwrap(),
            PathBuf::from("/home/docs")
        );
        assert_eq!(
            normalise_absolute_path(Path::new("/../docs")).unwrap(),
            PathBuf::from("/docs")
        );
    }
}
