use crate::Config;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use url::Url;

#[derive(Args)]
pub(crate) struct InitArgs {
    name: String,
    #[arg(short, long)]
    force: bool,
}

pub(crate) fn init(args: InitArgs, file_name: &str) -> Result<()> {
    let path = std::path::Path::new(&file_name);
    if !args.force && path.exists() {
        return Err(anyhow!(
            "{} already exists. Use --force to overwrite",
            file_name
        ));
    }

    let config = Config {
        name: args.name.clone(),
        problem_url: build_default_problem_url(&args.name)?,
    };
    let config_str = toml::to_string(&config)
        .context(format!("Failed to serialize config to TOML: {:?}", config))?;

    std::fs::write(path, config_str)
        .context(format!("Failed to write config to file: {}", file_name))?;
    eprintln!(
        "{}",
        format!("Initialized project with name: {}", args.name).green()
    );
    Ok(())
}

fn build_default_problem_url(name: &String) -> Result<String> {
    let base_url = "https://atcoder.jp/contests";
    let mut url = Url::parse(base_url).context(anyhow!("Failed to parse URL: {}", base_url))?;

    url.path_segments_mut()
        .map_err(|_| anyhow!("Failed to set path segments"))?
        .push(name)
        .push("tasks")
        .push(format!("{}_a", name).as_str());
    url.query_pairs_mut().append_pair("lang", "ja");

    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_CONFIG_FILE_NAME;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn init_creates_config_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(DEFAULT_CONFIG_FILE_NAME);
        let args = InitArgs {
            name: "test_project".to_string(),
            force: false,
        };

        init(args, file_path.to_str().unwrap()).unwrap();

        assert!(file_path.exists());
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("test_project"));
    }

    #[test]
    fn init_overwrites_existing_file_with_force() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(DEFAULT_CONFIG_FILE_NAME);
        fs::write(&file_path, "existing content").unwrap();

        let args = InitArgs {
            name: "new_project".to_string(),
            force: true,
        };

        init(args, file_path.to_str().unwrap()).unwrap();

        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("new_project"));
        assert!(!content.contains("existing content"));
    }

    #[test]
    fn init_fails_if_file_exists_without_force() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(DEFAULT_CONFIG_FILE_NAME);
        fs::write(&file_path, "existing content").unwrap();

        let args = InitArgs {
            name: "new_project".to_string(),
            force: false,
        };

        let result = init(args, file_path.to_str().unwrap());
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("already exists"));
    }

    #[test]
    fn build_default_url() {
        let url = build_default_problem_url(&"ahc001".to_string()).unwrap();
        assert_eq!(
            url,
            "https://atcoder.jp/contests/ahc001/tasks/ahc001_a?lang=ja"
        );
    }
}
