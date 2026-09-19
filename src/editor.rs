use std::{env, path::Path, process::Command};

use shell_words::split;

use crate::{Result, XddError};

pub fn edit(path: &Path) -> Result<()> {
    let editor = env::var("EDITOR")
        .map_err(|_| XddError::new("EDITOR is not set; cannot choose an editor"))?;
    let mut arguments =
        split(&editor).map_err(|error| XddError::new(format!("cannot parse EDITOR: {error}")))?;
    if arguments.is_empty() {
        return Err(XddError::new("EDITOR is empty; cannot choose an editor"));
    }

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

#[cfg(test)]
mod tests {
    use shell_words::split;

    #[test]
    fn parses_editor_arguments_without_a_shell() {
        assert_eq!(
            split("code --wait 'path with spaces'").unwrap(),
            ["code", "--wait", "path with spaces"]
        );
    }
}
