use assert_cmd::Command;
use std::fs;
use anyhow::Result;

const PRG: &str = "ahc";

#[test]
fn init() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config_file_path = temp_dir.path().join("ahc_tools.toml");

    let mut cmd = Command::cargo_bin("ahc")?;
    cmd.arg("init")
        .arg("test_project")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    assert!(config_file_path.exists());
    let content = fs::read_to_string(config_file_path)?;
    assert!(content.contains("test_project"));
    
    Ok(())
}

#[test]
fn download() -> Result<()> {
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

    let temp_dir = tempfile::tempdir()?;
    let config_file_path = temp_dir.path().join("ahc_tools.toml");
    let config = format!(
        r#"
                [general]
                name = "test_contest"
                problem_url = "{}"
            "#,
        server.url()
    );
    fs::write(&config_file_path, config)?;

    let mut cmd = Command::cargo_bin(PRG)?;
    cmd.arg("download")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let file_path = temp_dir.path().join("tools/mock.txt");
    assert!(file_path.exists());

    html_mock.assert();
    zip_mock.assert();
    
    Ok(())
}

#[test]
fn commit() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config_file_path = temp_dir.path().join("ahc_tools.toml");
    let config = r#"
        [general]
        name = "test_contest"
        problem_url = "https://example.net"
    "#;
    fs::write(&config_file_path, config)?;
    
    // copy test files to temp_dir
    let test_file_dir = fs::read_dir("tests/fixtures/e2e");
    copy_file_dir(test_file_dir?, temp_dir.path())?;
    
    // initialize git directory
    Command::new("git")
        .arg("init").current_dir(temp_dir.path()).assert().success();
    Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("test_user")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    
    // create initial commit, because ahc commit cannot handle initial commit
    Command::new("git")
        .arg("add")
        .arg("clean.sh")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    
    let mut cmd = Command::cargo_bin(PRG)?;
    cmd.arg("commit")
        .arg("test message")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    
    // check if the commit message is correct
    let output = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--pretty=%B")
        .current_dir(temp_dir.path())
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    assert_eq!(output.trim(), "(50890.50) test message");

    Ok(())
}

fn copy_file_dir(dir: fs::ReadDir, dest: &std::path::Path) -> Result<()> {
    for entry in dir {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let dir_name = entry.file_name();
            let new_dest = dest.join(dir_name);
            fs::create_dir(&new_dest)?;
            let new_dir = fs::read_dir(entry.path())?;
            copy_file_dir(new_dir, &new_dest)?;
        } else {
            let file_name = entry.file_name();
            let new_dest = dest.join(file_name);
            fs::copy(entry.path(), new_dest)?;
        }
    }
    Ok(())
}