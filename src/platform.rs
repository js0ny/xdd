use std::path::Path;

#[cfg(target_os = "linux")]
use std::{fs, process::Command};

use crate::{Result, XddError};

#[cfg(target_os = "linux")]
const OPEN_HANDLER: &str = "xdg-open";

#[cfg(target_os = "windows")]
use std::{os::windows::ffi::OsStrExt, ptr};

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

pub fn open(path: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new(OPEN_HANDLER)
            .arg(path)
            .status()
            .map_err(|error| XddError::new(format!("cannot start {OPEN_HANDLER}: {error}")))?;
        if !status.success() {
            return Err(XddError::new(format!(
                "{OPEN_HANDLER} exited with status {status}"
            )));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        if wide_path[..wide_path.len() - 1].contains(&0) {
            return Err(XddError::new("target path contains NUL"));
        }

        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                ptr::null(),
                wide_path.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if (result as isize) <= 32 {
            return Err(XddError::new(format!(
                "cannot open {}: ShellExecuteW returned {}",
                path.display(),
                result as isize
            )));
        }
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        Err(XddError::new(
            "opening xdd URLs is not supported on this platform",
        ))
    }
}

pub fn register() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        register_linux()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(XddError::new(
            "registering xdd URLs is not supported on this platform",
        ))
    }
}

#[cfg(target_os = "linux")]
fn register_linux() -> Result<()> {
    let base_dirs = directories::BaseDirs::new()
        .ok_or_else(|| XddError::new("cannot determine the user's data directory"))?;
    let applications_dir = base_dirs.data_local_dir().join("applications");
    fs::create_dir_all(&applications_dir).map_err(|error| {
        XddError::new(format!(
            "cannot create applications directory {}: {error}",
            applications_dir.display()
        ))
    })?;

    let executable = std::env::current_exe()
        .map_err(|error| XddError::new(format!("cannot determine current executable: {error}")))?;
    let desktop_file = applications_dir.join("xdd.desktop");
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=xdd\nNoDisplay=true\nExec={} open %u\nMimeType=x-scheme-handler/xdd;\nTerminal=false\n",
        quote_desktop_argument(&executable)?
    );

    fs::write(&desktop_file, contents).map_err(|error| {
        XddError::new(format!("cannot write {}: {error}", desktop_file.display()))
    })?;

    let status = Command::new("xdg-mime")
        .args(["default", "xdd.desktop", "x-scheme-handler/xdd"])
        .status()
        .map_err(|error| XddError::new(format!("cannot start xdg-mime: {error}")))?;
    if !status.success() {
        return Err(XddError::new(format!(
            "xdg-mime exited with status {status}"
        )));
    }

    println!("registered xdd:// with {}", desktop_file.display());
    Ok(())
}

#[cfg(target_os = "linux")]
fn quote_desktop_argument(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or_else(|| XddError::new("current executable path is not valid UTF-8"))?;
    if value.chars().any(char::is_control) {
        return Err(XddError::new(
            "current executable path contains control characters",
        ));
    }

    let mut quoted = String::from('"');
    for character in value.chars() {
        if character == '%' {
            quoted.push_str("%%");
        } else {
            if matches!(character, '\\' | '"' | '`' | '$') {
                quoted.push('\\');
            }
            quoted.push(character);
        }
    }
    quoted.push('"');
    Ok(quoted)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::quote_desktop_argument;
    use std::path::Path;

    #[test]
    fn quotes_desktop_arguments() {
        assert_eq!(
            quote_desktop_argument(Path::new("/tmp/xdd app/$100/%name")).unwrap(),
            r#""/tmp/xdd app/\$100/%%name""#
        );
    }
}
