mod download;
pub(crate) mod init;

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
    }

    Ok(())
}

fn load_config(file_name: &str) -> Result<Config> {
    let content = std::fs::read_to_string(file_name)
        .map_err(|e| anyhow!("Failed to read config file: {}", e))?;
    let config: Config =
        toml::from_str(&content).map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
    Ok(config)
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
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    name: String,
    problem_url: String,
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use std::fs;

    #[test]
    fn init() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_file_path = temp_dir.path().join("ahc_tools.toml");

        let mut cmd = Command::cargo_bin("ahc").unwrap();
        cmd.arg("init")
            .arg("test_project")
            .current_dir(temp_dir.path())
            .assert()
            .success();

        assert!(config_file_path.exists());
        let content = fs::read_to_string(config_file_path).unwrap();
        assert!(content.contains("test_project"));
    }

    #[test]
    fn download() {
        let mut server = mockito::Server::new();
        let html_mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(format!(
                "<a href=\"{}/tools.zip\">ローカル版</a>",
                server.url()
            ))
            .create();
        let zip_mock = server
            .mock("GET", "/tools.zip")
            .with_status(200)
            .with_header("content-type", "application/zip")
            .with_body_from_file("src/tests/fixtures/test_archive.zip")
            .create();

        let temp_dir = tempfile::tempdir().unwrap();
        let config_file_path = temp_dir.path().join("ahc_tools.toml");
        let config = format!(
            r#"
                name = "test_contest"
                problem_url = "{}"
            "#,
            server.url()
        );
        fs::write(&config_file_path, config).unwrap();

        let mut cmd = Command::cargo_bin("ahc").unwrap();
        cmd.arg("download")
            .current_dir(temp_dir.path())
            .assert()
            .success();

        let file_path = temp_dir.path().join("tools/mock.txt");
        assert!(file_path.exists());

        html_mock.assert();
        zip_mock.assert();
    }
}
