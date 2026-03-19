use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub id: String,
    pub source: String,
}

#[derive(Deserialize)]
struct RawPackageEntry {
    id: String,
    source: Option<String>,
}

#[derive(Deserialize)]
struct PackageList {
    #[serde(default)]
    packages: Vec<RawPackageEntry>,
}

pub fn load(url: &str, default_source: &str) -> Result<Vec<PackageEntry>> {
    let content = if let Some(path) = url.strip_prefix("file://") {
        match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(e).with_context(|| format!("Failed to read package list from {}", path)),
        }
    } else if url.starts_with("http://") || url.starts_with("https://") {
        reqwest::blocking::get(url)
            .with_context(|| format!("Failed to fetch package list from {}", url))?
            .text()
            .context("Failed to read response body")?
    } else {
        anyhow::bail!("Unsupported URL scheme in package list path: {}", url);
    };

    let list: PackageList = toml::from_str(&content)
        .with_context(|| format!("Failed to parse package list from {}", url))?;

    Ok(list.packages
        .into_iter()
        .map(|e| PackageEntry {
            id: e.id,
            source: e.source.unwrap_or_else(|| default_source.to_string()),
        })
        .collect())
}
