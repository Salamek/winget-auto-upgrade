use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Default, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    User,
    Machine,
    #[default]
    All,
}

#[derive(Debug, Clone)]
pub struct PackageEntry {
    pub id: String,
    pub source: String,
    pub scope: Scope,
    pub custom_args: Option<String>,
    pub override_args: Option<String>,
    pub force_architecture: Option<String>,
    pub force_locale: Option<String>,
    pub ignore_security_hash: bool,
    pub skip_depedencies: bool
}

#[derive(Deserialize)]
struct RawPackageEntry {
    id: String,
    source: Option<String>,
    scope: Option<Scope>,
    custom_args: Option<String>,
    override_args: Option<String>,
    force_architecture: Option<String>,
    force_locale: Option<String>,
    ignore_security_hash: Option<bool>,
    skip_depedencies: Option<bool>
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
            scope: e.scope.unwrap_or_else(|| Scope::default()),
            custom_args: e.custom_args,
            override_args: e.override_args,
            force_architecture: e.force_architecture,
            force_locale: e.force_locale,
            ignore_security_hash: e.ignore_security_hash.unwrap_or(false),
            skip_depedencies: e.skip_depedencies.unwrap_or(false)
        })
        .collect())
}
