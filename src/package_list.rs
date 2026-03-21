use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    User,
    Machine,
    #[default]
    None,
}


impl Scope {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "user"     => Some(Self::User),
            "machine" => Some(Self::Machine),
            "none"    => Some(Self::None),
            _         => Option::None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub id: String,
    pub source: String,
    pub scope: Scope
}

#[derive(Deserialize)]
struct RawPackageEntry {
    id: String,
    source: Option<String>,
    scope: Option<Scope>,
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
            scope: e.scope.unwrap_or(Scope::default()),
        })
        .collect())
}
