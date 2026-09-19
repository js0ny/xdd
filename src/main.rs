mod config;
mod editor;
mod link;
mod platform;
mod resolver;

use std::{error::Error, fmt};

use clap::{Parser, Subcommand};

use crate::{link::LinkFormat, resolver::resolve};

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
    Open {
        url: String,
    },
    Resolve {
        url: String,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Roots {
        #[command(subcommand)]
        command: RootsCommand,
    },
    Link {
        directory: String,
        #[arg(long)]
        root: Option<String>,
        #[arg(long, value_enum, default_value_t = LinkFormat::Plain)]
        format: LinkFormat,
        #[arg(long)]
        label: Option<String>,
    },
    Register,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Path,
    Edit,
}

#[derive(Debug, Subcommand)]
enum RootsCommand {
    List,
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
        Command::Config { command } => match command {
            ConfigCommand::Path => {
                println!("{}", config::path()?.display());
                Ok(())
            }
            ConfigCommand::Edit => editor::edit(&config::create_if_missing()?),
        },
        Command::Roots { command } => match command {
            RootsCommand::List => {
                let config = config::load()?;
                let mut roots = config.roots.into_iter().collect::<Vec<_>>();
                roots.sort_by(|left, right| left.0.cmp(&right.0));
                for (name, path) in roots {
                    println!("{name} -> {}", path.display());
                }
                Ok(())
            }
        },
        Command::Link {
            directory,
            root,
            format,
            label,
        } => {
            let config = config::load()?;
            println!(
                "{}",
                link::create(
                    &config,
                    &directory,
                    root.as_deref(),
                    format,
                    label.as_deref()
                )?
            );
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
