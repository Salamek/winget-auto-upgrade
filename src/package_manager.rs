use anyhow::{Result, anyhow};
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Serialize, Clone)]
pub struct Package {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct PackageUpgrade {
    pub from: Package,
    pub to: Package,
}

#[derive(Debug, Default)]
pub struct UpgradeOptions {
    pub custom_args: Option<String>,
    pub override_args: Option<String>,
    pub force_architecture: Option<String>,
    pub force_locale: Option<String>,
    pub ignore_security_hash: bool,
    pub skip_dependencies: bool,
}

pub trait PackageManager {
    fn list_upgrades(&self) -> Vec<PackageUpgrade>;
    fn list(&self) -> Vec<Package>;
    fn upgrade(&self, package: &Package, options: &UpgradeOptions) -> Result<Package>;
}

#[cfg(target_os = "windows")]
fn winget_exe() -> String {
    use std::path::PathBuf;

    fn dir_version(path: &PathBuf) -> Vec<u32> {
        // Directory name: Microsoft.DesktopAppInstaller_1.2.3.4_x64_8wekyb3d8bbwe
        // Version is the second underscore-delimited segment
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.split('_').nth(1))
            .map(|v| v.split('.').filter_map(|p| p.parse().ok()).collect())
            .unwrap_or_default()
    }

    // Try system path first — pick highest version if multiple installs exist
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        let windows_apps = PathBuf::from(program_files).join("WindowsApps");
        if let Ok(entries) = std::fs::read_dir(&windows_apps) {
            let mut candidates: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| {
                            n.starts_with("Microsoft.DesktopAppInstaller_")
                                && n.ends_with("_8wekyb3d8bbwe")
                        })
                        .unwrap_or(false)
                })
                .map(|p| p.join("winget.exe"))
                .filter(|p| p.exists())
                .collect();

            candidates.sort_by(|a, b| {
                let va = dir_version(&a.parent().unwrap().to_path_buf());
                let vb = dir_version(&b.parent().unwrap().to_path_buf());
                vb.cmp(&va) // descending
            });

            if let Some(path) = candidates.first() {
                return path.to_string_lossy().into_owned();
            }
        }
    }

    // Fall back to user-scoped install
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let user_path = PathBuf::from(local_app_data)
            .join("Microsoft")
            .join("WindowsApps")
            .join("Microsoft.DesktopAppInstaller_8wekyb3d8bbwe")
            .join("winget.exe");
        if user_path.exists() {
            return user_path.to_string_lossy().into_owned();
        }
    }

    // Last resort: rely on PATH
    "winget".to_string()
}

#[cfg(not(target_os = "windows"))]
fn winget_exe() -> String {
    "winget-stub/winget.exe".to_string()
}

// Decode raw bytes from winget output.
// Winget writes UTF-16LE (with BOM) when talking to a console, but switches to
// UTF-8 when stdout is redirected to a pipe (i.e. Command::output()).
fn decode_output(data: &[u8]) -> String {
    if data.starts_with(&[0xFF, 0xFE]) {
        // UTF-16LE with BOM
        let words: Vec<u16> = data[2..]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16_lossy(&words).to_owned()
    } else {
        // UTF-8 (piped output)
        String::from_utf8_lossy(data).into_owned()
    }
}

// Simulate terminal carriage-return behaviour: \r resets to the start of the
// current line so the next characters overwrite what was written before.
// This removes winget's spinner lines which use \r to overwrite themselves.
fn apply_carriage_returns(text: &str) -> String {
    // Normalize \r\n → \n first so actual line endings don't clear their own content.
    // After that, lone \r simulates terminal overwrite (spinner lines reset to col 0).
    let text = text.replace("\r\n", "\n");
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '\r' => current.clear(),
            '\n' => {
                lines.push(current.clone());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.join("\n")
}

// Strip ANSI escape sequences, C0/C1 control chars, and Unicode box-drawing /
// block elements that winget emits for its progress bar.
fn strip_garbage(text: String) -> String {
    let text = apply_carriage_returns(&text);
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // ANSI CSI escape: ESC [ <params> <letter>
            '\x1B' if chars.peek() == Some(&'[') => {
                chars.next(); // consume '['
                for ch in chars.by_ref() {
                    if ch.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // C0 control chars (keep \n \t space), C1 control chars
            '\x00'..='\x08' | '\x0B'..='\x1F' | '\x7F'..='\u{9F}' => {}
            // Box-drawing horizontal line (U+2500 ─) used as table separator → keep as ASCII dash
            '\u{2500}' => out.push('-'),
            // Other box-drawing (U+2501–U+257F) and block elements (U+2580–U+25FF) — drop
            '\u{2501}'..='\u{25FF}' => {}
            _ => out.push(c),
        }
    }
    out
}

// Parse a fixed-width winget table from raw stdout bytes.
// Returns one HashMap<column_name, value> per data row (column names are lowercased).
fn parse_table(data: &[u8]) -> Vec<HashMap<String, String>> {
    let text = strip_garbage(decode_output(data));

    let lines: Vec<&str> = text.lines().collect();
    // Find the separator line: trimmed content is all dashes (10+)
    let sep_idx = match lines.iter().position(|l| {
        let t = l.trim();
        t.len() >= 10 && t.chars().all(|c| c == '-')
    }) {
        Some(i) if i > 0 => i,
        _ => return vec![],
    };

    let header_line = lines[sep_idx - 1];

    // Determine column start positions (char indices) from the header.
    // A gap of 2+ consecutive spaces marks a column boundary.
    let header_chars: Vec<char> = header_line.chars().collect();

    let mut col_starts: Vec<usize> = vec![0];
    let mut i = 0;
    while i < header_chars.len() {
        if header_chars[i] == ' ' {
            let gap_start = i;
            while i < header_chars.len() && header_chars[i] == ' ' {
                i += 1;
            }
            if i - gap_start >= 2 {
                col_starts.push(i);
            }
        } else {
            i += 1;
        }
    }
    col_starts.push(usize::MAX); // last column: read to end of each data line

    // Slice a line into column values by char ranges.
    let extract = |line: &str| -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        col_starts
            .windows(2)
            .map(|w| {
                let (s, e) = (w[0], w[1].min(chars.len()));
                if s >= chars.len() {
                    String::new()
                } else {
                    chars[s..e].iter().collect::<String>().trim().to_string()
                }
            })
            .collect()
    };

    let headers: Vec<String> = extract(header_line)
        .into_iter()
        .map(|h| h.to_lowercase())
        .collect();

    let mut rows = vec![];
    for line in &lines[sep_idx + 1..] {
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }
        // Summary line like "2 upgrades available." starts with a digit
        if stripped.starts_with(|c: char| c.is_ascii_digit()) {
            break;
        }
        let cols = extract(line);
        let row: HashMap<String, String> = headers.iter().cloned().zip(cols).collect();
        rows.push(row);
    }
    rows
}

pub struct Winget {
    exe: String,
}

impl Winget {
    pub fn new() -> Self {
        Winget { exe: winget_exe() }
    }
}

impl PackageManager for Winget {
    fn list_upgrades(&self) -> Vec<PackageUpgrade> {
        let output = match Command::new(&self.exe).args(["upgrade"]).output() {
            Ok(o) => o,
            Err(e) => {
                log::error!("Failed to run {}: {}", self.exe, e);
                return vec![];
            }
        };

        parse_table(&output.stdout)
            .into_iter()
            .filter_map(|mut row| {
                let id = row.remove("id")?;
                if id.is_empty() {
                    return None;
                }
                let name = row.remove("name").unwrap_or_default();
                let source = row.remove("source").unwrap_or_default();
                let version = row.remove("version").unwrap_or_default();
                let available = row.remove("available").unwrap_or_default();
                Some(PackageUpgrade {
                    from: Package {
                        name: name.clone(),
                        id: id.clone(),
                        version,
                        source: source.clone(),
                    },
                    to: Package {
                        name,
                        id,
                        version: available,
                        source,
                    },
                })
            })
            .collect()
    }

    fn list(&self) -> Vec<Package> {
        let output = match Command::new(&self.exe).args(["list"]).output() {
            Ok(o) => o,
            Err(e) => {
                log::error!("Failed to run {}: {}", self.exe, e);
                return vec![];
            }
        };

        parse_table(&output.stdout)
            .into_iter()
            .filter_map(|mut row| {
                let id = row.remove("id")?;
                if id.is_empty() {
                    return None;
                }
                let name = row.remove("name").unwrap_or_default();
                let source = row.remove("source").unwrap_or_default();
                let version = row.remove("version").unwrap_or_default();
                Some(Package {
                    name: name,
                    id: id,
                    version: version,
                    source: source,
                })
            })
            .collect()
    }

    fn upgrade(&self, package: &Package, options: &UpgradeOptions) -> Result<Package> {
        let mut args = vec![
            "upgrade".to_string(),
            "--id".to_string(),
            package.id.clone(),
            "--source".to_string(),
            package.source.clone(),
            "--silent".to_string(),
            "--accept-package-agreements".to_string(),
            "--accept-source-agreements".to_string(),
        ];

        if let Some(arch) = &options.force_architecture {
            args.extend(["--architecture".to_string(), arch.clone()]);
        }
        if let Some(locale) = &options.force_locale {
            args.extend(["--locale".to_string(), locale.clone()]);
        }
        if let Some(override_args) = &options.override_args {
            args.extend(["--override".to_string(), override_args.clone()]);
        }
        if let Some(custom_args) = &options.custom_args {
            args.extend(["--custom".to_string(), custom_args.clone()]);
        }
        if options.ignore_security_hash {
            args.push("--ignore-security-hash".to_string());
        }
        if options.skip_dependencies {
            args.push("--skip-dependencies".to_string());
        }

        let output = Command::new(&self.exe).args(&args).output()?;

        let stdout = decode_output(&output.stdout);
        if !stdout.contains("Successfully installed") {
            return Err(anyhow!("upgrade failed for {}", package.id));
        }

        let version = stdout
            .lines()
            .find_map(|line| line.split("Version ").nth(1))
            .unwrap_or_default()
            .trim()
            .to_string();

        Ok(Package {
            name: package.name.clone(),
            id: package.id.clone(),
            source: package.source.clone(),
            version: version,
        })
    }
}
