use std::path::{Component, Path, PathBuf};

use clap::ValueEnum;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};

use crate::{Result, XddError, config, config::Config, resolver::normalise_absolute_path};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LinkFormat {
    Plain,
    Markdown,
    Typst,
    Org,
    Latex,
}

pub fn create(
    config: &Config,
    input: &str,
    requested_root: Option<&str>,
    format: LinkFormat,
    label: Option<&str>,
) -> Result<String> {
    let target = input_path(input)?;
    if !target.exists() {
        return Err(XddError::new(format!(
            "directory does not exist: {}",
            target.display()
        )));
    }
    if !target.is_dir() {
        return Err(XddError::new(format!(
            "path is not a directory: {}",
            target.display()
        )));
    }

    let (root_name, root_path) = select_root(config, &target, requested_root)?;
    let relative = target
        .strip_prefix(&root_path)
        .map_err(|_| XddError::new("directory is outside the selected root"))?;
    let url = make_url(&root_name, relative)?;
    if format == LinkFormat::Plain {
        if label.is_some() {
            return Err(XddError::new(
                "--label can only be used with a formatted link",
            ));
        }
        return Ok(url);
    }

    let label = label
        .map(str::to_owned)
        .unwrap_or_else(|| default_label(&target, &root_name));
    render(format, &url, &label)
}

fn input_path(input: &str) -> Result<PathBuf> {
    let path = config::expand_user_path(input)?;
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|error| XddError::new(format!("cannot determine current directory: {error}")))?
            .join(path)
    };
    normalise_absolute_path(&path)
}

fn select_root(
    config: &Config,
    target: &Path,
    requested_root: Option<&str>,
) -> Result<(String, PathBuf)> {
    if let Some(name) = requested_root {
        let path = config
            .roots
            .get(name)
            .ok_or_else(|| XddError::new(format!("unknown root {name:?}")))?;
        let path = normalise_absolute_path(path)?;
        if !target.starts_with(&path) {
            return Err(XddError::new(format!("directory is outside root {name:?}")));
        }
        return Ok((name.to_owned(), path));
    }

    let mut candidates = config
        .roots
        .iter()
        .filter_map(|(name, path)| {
            let path = normalise_absolute_path(path).ok()?;
            target.starts_with(&path).then_some((name, path))
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err(XddError::new("no root contains the directory"));
    }

    let deepest = candidates
        .iter()
        .map(|(_, path)| path.components().count())
        .max()
        .expect("candidates is not empty");
    candidates.retain(|(_, path)| path.components().count() == deepest);

    if candidates.len() > 1 {
        candidates.sort_by(|left, right| left.0.cmp(right.0));
        let names = candidates
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(XddError::new(format!(
            "directory resolves to the same root through aliases: {names}; use --root to specify"
        )));
    }

    let (name, path) = candidates.pop().expect("candidates has one item");
    Ok((name.to_owned(), path))
}

fn make_url(root_name: &str, relative: &Path) -> Result<String> {
    let root = percent_encode(root_name.as_bytes(), NON_ALPHANUMERIC);
    let mut path = String::new();

    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(XddError::new("cannot encode a non-relative directory path"));
        };
        let component = component
            .to_str()
            .ok_or_else(|| XddError::new("directory path is not valid UTF-8"))?;
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(&percent_encode(component.as_bytes(), NON_ALPHANUMERIC).to_string());
    }

    Ok(format!("xdd://{root}:{path}"))
}

fn default_label(target: &Path, root_name: &str) -> String {
    target
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| root_name.to_owned())
}

fn render(format: LinkFormat, url: &str, label: &str) -> Result<String> {
    if label.chars().any(char::is_control) {
        return Err(XddError::new("label must not contain control characters"));
    }

    match format {
        LinkFormat::Plain => Ok(url.to_owned()),
        LinkFormat::Markdown => Ok(format!("[{}]({url})", escape_markdown(label))),
        LinkFormat::Typst => Ok(format!(
            "#link(\"{}\")[{}]",
            escape_typst(url),
            escape_typst_label(label)
        )),
        LinkFormat::Org => Ok(format!("[[{}][{}]]", escape_org(url), escape_org(label))),
        LinkFormat::Latex => Ok(format!(
            r#"\href{{{}}}{{{}}}"#,
            escape_latex(url),
            escape_latex(label)
        )),
    }
}

fn escape_markdown(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_typst(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_typst_label(value: &str) -> String {
    escape_typst(value).replace(']', "\\]")
}

fn escape_org(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_latex(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str(r"\textbackslash{}"),
            '{' => escaped.push_str(r"\{"),
            '}' => escaped.push_str(r"\}"),
            '$' => escaped.push_str(r"\$"),
            '&' => escaped.push_str(r"\&"),
            '#' => escaped.push_str(r"\#"),
            '%' => escaped.push_str(r"\%"),
            '_' => escaped.push_str(r"\_"),
            '^' => escaped.push_str(r"\textasciicircum{}"),
            '~' => escaped.push_str(r"\textasciitilde{}"),
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{LinkFormat, make_url, render};
    use std::path::Path;

    #[test]
    fn encodes_relative_urls() {
        assert_eq!(
            make_url("projects", Path::new("hello world/xdd")).unwrap(),
            "xdd://projects:hello%20world/xdd"
        );
    }

    #[test]
    fn renders_formats() {
        assert_eq!(
            render(LinkFormat::Markdown, "xdd://projects:xdd", "xdd").unwrap(),
            "[xdd](xdd://projects:xdd)"
        );
        assert_eq!(
            render(LinkFormat::Typst, "xdd://projects:xdd", "xdd").unwrap(),
            "#link(\"xdd://projects:xdd\")[xdd]"
        );
        assert_eq!(
            render(LinkFormat::Org, "xdd://projects:xdd", "xdd").unwrap(),
            "[[xdd://projects:xdd][xdd]]"
        );
        assert_eq!(
            render(LinkFormat::Latex, "xdd://projects:xdd", "xdd").unwrap(),
            r#"\href{xdd://projects:xdd}{xdd}"#
        );
    }
}
