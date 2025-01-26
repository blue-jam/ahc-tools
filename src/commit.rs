use crate::pahcer::ExecResult;
use crate::Config;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use git2::Repository;
use std::io::Write;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct CommitArgs {
    message: String,
}

pub(crate) fn commit(args: CommitArgs, _config: Config) -> Result<()> {
    if args.message.is_empty() {
        return Err(anyhow!("Commit message is empty"));
    }

    let repo = Repository::open_from_env().context("Failed to open git repository")?;
    let updated_file_paths = list_updated_files(&repo)?;

    if updated_file_paths.is_empty() {
        return Err(anyhow!("Nothing to commit"));
    }

    let result_file_paths = filter_and_sort_result_files(&updated_file_paths);

    if result_file_paths.is_empty() {
        // Ask if the user wants to commit anyway
        let mut input = String::new();
        print!("No result files found. Commit anyway? [y/N]: ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            return Ok(());
        }
        let message = args.message.to_string();
        return commit_staged(&repo, &message);
    }

    let result = read_exec_result(&repo, result_file_paths)?;
    let commit_message = build_commit_message(&args, &result);

    commit_staged(&repo, &commit_message)
}

fn list_updated_files(repo: &Repository) -> Result<Vec<PathBuf>> {
    let diff = repo.diff_tree_to_index(Some(&repo.head()?.peel_to_tree()?), None, None)?;
    if diff.deltas().count() == 0 {
        return Ok(vec![]);
    }

    let mut updated_file_paths = vec![];
    diff.foreach(
        &mut |delta, _hunk| {
            let path = delta.new_file().path().unwrap();
            if path.is_dir() {
                return true;
            }
            updated_file_paths.push(path.to_path_buf());

            true
        },
        None,
        None,
        None,
    )?;

    Ok(updated_file_paths)
}

fn filter_and_sort_result_files(updated_file_paths: &[PathBuf]) -> Vec<&PathBuf> {
    let re = regex::Regex::new(r"result_[0-9]{8}_[0-9]{6}\.json").unwrap();
    let mut result_file_paths = updated_file_paths
        .iter()
        .filter(|path| re.is_match(path.file_name().unwrap().to_str().unwrap()))
        .collect::<Vec<_>>();
    result_file_paths.sort_by(|a, b| b.file_name().unwrap().cmp(a.file_name().unwrap()));
    
    result_file_paths
}

fn commit_staged(repo: &Repository, message: &str) -> Result<()> {
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let signature = repo.signature()?;
    let parent_commit = repo.head()?.peel_to_commit()?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )?;
    Ok(())
}

fn read_exec_result(repo: &Repository, result_file_paths: Vec<&PathBuf>) -> Result<ExecResult> {
    let latest_file_path = repo.workdir().unwrap().join(result_file_paths[0]);
    let mut file = std::fs::File::open(&latest_file_path)?;
    let result: ExecResult = serde_json::from_reader(&mut file)?;
    Ok(result)
}

fn build_commit_message(args: &CommitArgs, result: &ExecResult) -> String {
    let avg_score = result.total_score as f64 / result.case_count as f64;
    let commit_message = format!("({:.2}) {}", avg_score, args.message);
    commit_message
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs::File;
    use std::path::Path;
    use tempfile::{tempdir, TempDir};

    #[test]
    fn test_list_updated_files() -> Result<()> {
        let dir = tempdir()?;
        let repo = Repository::init(&dir)?;

        create_dummy_commit(&dir, &repo)?;

        const STAGED_FILE_NAME: &str = "file.txt";
        let file_path = dir.path().join(STAGED_FILE_NAME);
        File::create(&file_path)?;
        let mut index = repo.index()?;
        index.add_path(Path::new(STAGED_FILE_NAME))?;
        index.write()?;

        const UNSTAGED_FILE_NAME: &str = "unstaged.txt";
        let file_path = dir.path().join(UNSTAGED_FILE_NAME);
        File::create(&file_path)?;

        let updated_files = list_updated_files(&repo)?;

        assert_eq!(updated_files.len(), 1);
        assert_eq!(updated_files[0], PathBuf::from(STAGED_FILE_NAME));

        Ok(())
    }

    fn create_dummy_commit(dir: &TempDir, repo: &Repository) -> Result<()> {
        const FILE_NAME: &str = ".gitkeep";
        let file_path = dir.path().join(FILE_NAME);
        File::create(&file_path)?;

        let mut index = repo.index()?;
        index.add_path(Path::new(FILE_NAME))?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = repo.signature()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;
        Ok(())
    }

    #[test]
    fn test_filter_result_files() {
        let file_path1 = "result_20210901_123456.json";
        let file_path2 = "pahcer/json/result_20210901_234567.json";
        let file_path3 = "unrelated.txt";

        let updated_files = vec![
            PathBuf::from(file_path1),
            PathBuf::from(file_path2),
            PathBuf::from(file_path3),
        ];
        let expected = vec![&updated_files[1], &updated_files[0]];

        let result_files = filter_and_sort_result_files(&updated_files);

        assert_eq!(result_files, expected);
    }

    #[test]
    fn test_build_commit_message() {
        let args = CommitArgs {
            message: "Test commit message".to_string(),
        };
        let result = ExecResult {
            case_count: 2,
            total_score: 10,
        };

        let commit_message = build_commit_message(&args, &result);

        assert_eq!(commit_message, "(5.00) Test commit message");
    }
}