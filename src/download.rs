use crate::Config;
use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use clap::Args;
use std::io::Cursor;
use zip::ZipArchive;

#[derive(Args)]
pub(crate) struct DownloadArgs {
    #[arg(short, long)]
    output_path: Option<String>,
    #[arg(short, long)]
    url: Option<String>,
    #[arg(short, long)]
    zip_url: Option<String>,
}

pub(crate) fn download(args: DownloadArgs, config: Config) -> Result<()> {
    let zip_url = if let Some(zip_url) = args.zip_url {
        zip_url
    } else {
        let url = if let Some(url) = args.url {
            url
        } else {
            config.general.problem_url
        };

        let html = fetch_html(&url)?;
        find_tool_url(&html)?
    };

    let cursor = fetch_zip(&zip_url)?;
    let output_path = args.output_path.as_deref().unwrap_or(".");

    unzip_file(cursor, output_path)?;

    Ok(())
}

fn fetch_html(url: &String) -> Result<String> {
    let html = reqwest::blocking::get(url)
        .context(format!("Failed to fetch HTML from URL: {}", url))?
        .text()
        .context("Failed to get HTML text")?;
    Ok(html)
}

fn find_tool_url(html: &str) -> Result<String> {
    let document = scraper::Html::parse_document(html);
    let selector =
        scraper::Selector::parse("a").map_err(|_| anyhow!("Failed to parse selector: a"))?;
    let mut tools = vec![];
    for element in document.select(&selector) {
        if element.text().any(|text| text.contains("ローカル版")) {
            if let Some(href) = element.value().attr("href") {
                tools.push(href);
            }
        }
    }

    eprintln!("Found {} tool links:", tools.len());
    for tool in &tools {
        eprintln!(" - {}", tool);
    }

    if tools.len() != 1 {
        return Err(anyhow!("Found {} tool links, expected 1", tools.len()));
    }
    Ok(tools[0].into())
}

fn fetch_zip(zip_url: &String) -> Result<Cursor<Bytes>> {
    eprintln!("Downloading tools from: {}", zip_url);
    let zip_bytes = reqwest::blocking::get(zip_url)
        .context(format!("Failed to fetch zip file from URL: {}", zip_url))?
        .bytes()?;
    let cursor = Cursor::new(zip_bytes);
    Ok(cursor)
}

fn unzip_file<R>(data: R, output_path: &str) -> Result<()>
where
    R: std::io::Read + std::io::Seek,
{
    eprintln!("Unzipping tools to: {}", output_path);
    // unzip file
    let mut zip = ZipArchive::new(data).context("Failed to parse zip file")?;
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .context(format!("Failed to get file by index: {}", i))?;

        let file_path = match file.enclosed_name() {
            None => continue,
            Some(path) => path,
        };
        let out_path = std::path::Path::new(output_path).join(file_path);

        if file.is_dir() {
            std::fs::create_dir_all(out_path).context(format!(
                "Failed to create directory: {:?}",
                file.enclosed_name().unwrap()
            ))?;
        } else {
            let mut output_file = std::fs::File::create(out_path).context(format!(
                "Failed to create file: {:?}",
                file.enclosed_name().unwrap()
            ))?;
            std::io::copy(&mut file, &mut output_file)
                .context(format!("Failed to copy file: {:?}", output_file))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn test_fetch_html() {
        let mut server = mockito::Server::new();

        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("content")
            .create();
        let html = fetch_html(&server.url()).unwrap();
        assert_eq!(html, "content");
        mock.assert();
    }

    #[test]
    fn test_find_tool_url() {
        // read file from test directory
        let html = include_str!("tests/fixtures/atcoder_mock.html");
        let url = find_tool_url(html).unwrap();
        assert_eq!(url, "https://example.net/tools.zip");
    }

    #[test]
    fn test_unzip_file() {
        let data = include_bytes!("tests/fixtures/test_archive.zip");
        let cursor = Cursor::new(data.as_ref());
        let dir = tempdir().unwrap();
        let output_path = dir.path().to_str().unwrap();

        unzip_file(cursor, output_path).unwrap();

        let file_path = dir.path().join("tools/mock.txt");
        assert!(file_path.exists());

        let mut file = std::fs::File::open(file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "content\n");

        let dir_path = dir.path().join("tools/in");
        assert!(dir_path.exists());
        let mut file = std::fs::File::open(dir_path.join("0000.txt")).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "1000\n");
    }
}
