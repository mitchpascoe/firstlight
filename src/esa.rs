use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

const FEED_URL: &str = "https://esawebb.org/images/feed/";
const CDN_BASE: &str = "https://cdn.esawebb.org/archives/images";

#[derive(Clone)]
pub struct EsaImage {
    pub id: String,
    pub title: String,
    pub date: String,
    pub preview_url: String,
    pub wallpaper_url: String,
}

#[derive(Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(default)]
    item: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    title: Option<String>,
    #[serde(rename = "pubDate")]
    pub_date: Option<String>,
    enclosure: Option<Enclosure>,
}

#[derive(Deserialize)]
struct Enclosure {
    #[serde(rename = "@url")]
    url: String,
}

pub fn fetch_images() -> Result<Vec<EsaImage>> {
    let xml = ureq::get(FEED_URL)
        .call()
        .context("Failed to fetch ESA feed")?
        .body_mut()
        .read_to_string()
        .context("Failed to read feed body")?;

    let rss: Rss = quick_xml::de::from_str(&xml).context("Failed to parse RSS feed")?;

    let images = rss
        .channel
        .item
        .into_iter()
        .filter_map(|item| {
            let enclosure = item.enclosure?;
            let filename = enclosure.url.rsplit('/').next()?;
            let id = filename.strip_suffix(".jpg").unwrap_or(filename).to_string();
            let title = item.title.unwrap_or_else(|| id.clone());
            let date = item.pub_date.unwrap_or_default();

            Some(EsaImage {
                preview_url: format!("{CDN_BASE}/thumb700x/{id}.jpg"),
                wallpaper_url: format!("{CDN_BASE}/large/{id}.jpg"),
                id,
                title,
                date,
            })
        })
        .collect();

    Ok(images)
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
