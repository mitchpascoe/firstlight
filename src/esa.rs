use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::NaiveDate;

const CDN_BASE: &str = "https://cdn.esawebb.org/archives/images";

#[derive(Clone)]
pub struct EsaImage {
    pub id: String,
    pub title: String,
    pub date: String,
    pub preview_url: String,
    pub wallpaper_url: String,
}

pub fn fetch_images() -> Result<Vec<EsaImage>> {
    let mut all = Vec::new();
    let mut seen = HashSet::new();

    for page in 1..=20 {
        let url = format!("https://esawebb.org/images/page/{page}/");
        let Ok(mut resp) = ureq::get(&url).call() else { break };
        let Ok(html) = resp.body_mut().read_to_string() else { break };

        let images = parse_page(&html);
        if images.is_empty() {
            break;
        }

        for img in images {
            if seen.insert(img.id.clone()) {
                all.push(img);
            }
        }
    }

    all.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(all)
}

fn parse_page(html: &str) -> Vec<EsaImage> {
    let mut images = Vec::new();

    for chunk in html.split("id: '").skip(1) {
        let Some(id) = chunk.splitn(2, '\'').next() else { continue };
        if id.is_empty() || id.contains('\n') {
            continue;
        }

        let title = extract_between(chunk, "title: '", "'")
            .unwrap_or(id)
            .to_string();
        let date = extract_between(chunk, "potw: '", "'")
            .and_then(|s| NaiveDate::parse_from_str(s, "%d %B %Y").ok())
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        images.push(EsaImage {
            preview_url: format!("{CDN_BASE}/thumb700x/{id}.jpg"),
            wallpaper_url: format!("{CDN_BASE}/large/{id}.jpg"),
            id: id.to_string(),
            title,
            date,
        });
    }

    images
}

fn extract_between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let s = text.find(start)? + start.len();
    let e = text[s..].find(end)?;
    Some(&text[s..s + e])
}

fn cache_dir() -> Result<PathBuf> {
    let dir = dirs::cache_dir()
        .context("No cache directory")?
        .join("firstlight");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn download(url: &str, prefix: &str, id: &str) -> Result<PathBuf> {
    let dest = cache_dir()?.join(format!("{prefix}_{id}.jpg"));

    if dest.exists() {
        return Ok(dest);
    }

    let mut reader = ureq::get(url)
        .call()
        .with_context(|| format!("Failed to download {url}"))?
        .into_body()
        .into_reader();

    let mut file = fs::File::create(&dest)?;
    io::copy(&mut reader, &mut file)?;

    Ok(dest)
}
