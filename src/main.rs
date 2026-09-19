mod config;
mod platform;
mod resolver;

use std::{error::Error, fmt};

use clap::{Parser, Subcommand};

use crate::resolver::resolve;

pub type Result<T> = std::result::Result<T, XddError>;

#[derive(Debug)]
pub struct XddError(String);

impl XddError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for XddError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for XddError {}

#[derive(Debug, Parser)]
#[command(name = "xdd", about = "Cross-platform directory definition")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Open { url: String },
    Resolve { url: String },
    Register,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xdd: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Open { url } => {
            let config = config::load()?;
            let target = resolve(&config, &url)?;
            if !target.exists() {
                return Err(XddError::new(format!(
                    "target does not exist: {}",
                    target.display()
                )));
            }
            platform::open(&target)
        }
        Command::Resolve { url } => {
            let config = config::load()?;
            let target = resolve(&config, &url)?;
            println!("{}", target.display());
            Ok(())
        }
        Command::Register => platform::register(),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn parses_open_command() {
        assert!(matches!(
            Cli::try_parse_from(["xdd", "open", "xdd://docs:file.md"])
                .unwrap()
                .command,
            Command::Open { .. }
        ));
    }

    #[test]
    fn parses_resolve_command() {
        assert!(matches!(
            Cli::try_parse_from(["xdd", "resolve", "xdd://docs:file.md"])
                .unwrap()
                .command,
            Command::Resolve { .. }
        ));
    }
}
