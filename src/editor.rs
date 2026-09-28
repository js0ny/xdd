use std::{env, path::Path, process::Command};

use shell_words::split;

use crate::{Result, XddError};

#[cfg(windows)]
const DEFAULT_EDITOR: &str = "edit.exe";
#[cfg(not(windows))]
const DEFAULT_EDITOR: &str = "vim";

pub fn edit(path: &Path) -> Result<()> {
    let editor = match env::var("EDITOR") {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(error) => return Err(XddError::new(format!("cannot read EDITOR: {error}"))),
    };
    let mut arguments = editor_arguments(editor.as_deref())?;

    let executable = arguments.remove(0);
    let status = Command::new(&executable)
        .args(arguments)
        .arg(path)
        .status()
        .map_err(|error| XddError::new(format!("cannot start editor {executable:?}: {error}")))?;
    if !status.success() {
        return Err(XddError::new(format!(
            "editor {executable:?} exited with status {status}"
        )));
    }
    Ok(())
}

fn editor_arguments(editor: Option<&str>) -> Result<Vec<String>> {
    let editor = editor
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_EDITOR);
    let arguments =
        split(editor).map_err(|error| XddError::new(format!("cannot parse EDITOR: {error}")))?;
    if arguments.is_empty() {
        return Err(XddError::new("EDITOR does not specify an executable"));
    }
    Ok(arguments)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_EDITOR, editor_arguments};

    #[test]
    fn uses_platform_editor_when_unset_or_empty() {
        assert_eq!(editor_arguments(None).unwrap(), [DEFAULT_EDITOR]);
        assert_eq!(editor_arguments(Some(" \t")).unwrap(), [DEFAULT_EDITOR]);
    }

    #[test]
    fn parses_editor_arguments_without_a_shell() {
        assert_eq!(
            editor_arguments(Some("code --wait 'path with spaces'")).unwrap(),
            ["code", "--wait", "path with spaces"]
        );
    }
}
