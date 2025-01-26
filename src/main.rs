mod commit;
mod download;
mod init;
mod pahcer;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_CONFIG_FILE_NAME: &str = "ahc_tools.toml";

fn main() {
    if let Err(e) = run_command(Cli::parse()) {
        eprintln!("{}", format!("Error: {}", e).yellow().bold());
        std::process::exit(1);
    }
}

fn run_command(cli: Cli) -> Result<()> {
    let config_file_name = cli
        .config_file_name
        .as_deref()
        .unwrap_or(DEFAULT_CONFIG_FILE_NAME);

    // Load config file except for init command
    let config = match cli.command {
        Commands::Init(_) => None,
        _ => Some(load_config(config_file_name)?),
    };

    match cli.command {
        Commands::Init(args) => {
            init::init(args, config_file_name)?;
        }
        Commands::Download(args) => {
            download::download(args, config.unwrap())?;
        }
        Commands::Commit(args) => {
            commit::commit(args, config.unwrap())?;
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    config_file_name: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Init(init::InitArgs),
    Download(download::DownloadArgs),
    Commit(commit::CommitArgs),
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    general: General,
}

#[derive(Serialize, Deserialize, Debug)]
struct General {
    name: String,
    problem_url: String,
}

fn load_config(file_name: &str) -> Result<Config> {
    let content = std::fs::read_to_string(file_name)
        .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
    let config: Config =
        toml::from_str(&content).map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
    Ok(config)
}
