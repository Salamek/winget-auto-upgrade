use anyhow::Result;
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

#[derive(Debug, Serialize)]
pub struct UpdateResult {
    pub updated: Vec<PackageUpgrade>,
    pub failed: Vec<String>,
}

pub trait PackageManager {
    fn list_upgrades(&self) -> Vec<PackageUpgrade>;
    fn upgrade_all(&self) -> Result<UpdateResult>;
    fn list(&self) -> 
    fn upgrade(&self, package: &Package) -> Result<PackageUpgrade>;
}

#[cfg(target_os = "windows")]
fn winget_exe() -> &'static str {
    "winget"
}

#[cfg(not(target_os = "windows"))]
fn winget_exe() -> &'static str {
    "winget-stub/winget.exe"
}

// Decode raw bytes from winget output (UTF-16LE).
fn decode_utf16le(data: &[u8]) -> String {
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    String::from_utf16_lossy(&words).to_owned()
}

// Strip ANSI escape sequences, C0/C1 control chars, and Unicode box-drawing /
// block elements that winget emits for its progress bar.
fn strip_garbage(text: String) -> String {
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
            // Box-drawing (U+2500–U+257F) and block elements (U+2580–U+25FF)
            '\u{2500}'..='\u{25FF}' => {}
            _ => out.push(c),
        }
    }
    out
}

// Parse a fixed-width winget table from raw stdout bytes.
// Returns one HashMap<column_name, value> per data row (column names are lowercased).
fn parse_table(data: &[u8]) -> Vec<HashMap<String, String>> {
    let text = strip_garbage(decode_utf16le(data));
        dbg!(&text);
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
    col_starts.push(header_chars.len() + 1);

    // Slice a line into column values by char ranges.
    let extract = |line: &str| -> Vec<String> {
        let chars: Vec<char> = line.chars().collect();
        col_starts.windows(2).map(|w| {
            let (s, e) = (w[0], w[1].min(chars.len()));
            if s >= chars.len() {
                String::new()
            } else {
                chars[s..e].iter().collect::<String>().trim().to_string()
            }
        }).collect()
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
        let row: HashMap<String, String> = headers.iter()
            .cloned()
            .zip(cols)
            .collect();
        rows.push(row);
    }
    rows
}

pub struct Winget {
    exe: &'static str,
}

impl Winget {
    pub fn new() -> Self {
        Winget { exe: winget_exe() }
    }
}

impl PackageManager for Winget {
    fn list_upgrades(&self) -> Vec<PackageUpgrade> {
        let output = match Command::new(self.exe).args(["upgrade"]).output() {
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
                if id.is_empty() { return None; }
                let name    = row.remove("name").unwrap_or_default();
                let source  = row.remove("source").unwrap_or_default();
                let version = row.remove("version").unwrap_or_default();
                let available = row.remove("available").unwrap_or_default();
                Some(PackageUpgrade {
                    from: Package { name: name.clone(), id: id.clone(), version, source: source.clone() },
                    to:   Package { name, id, version: available, source },
                })
            })
            .collect()
    }

    fn upgrade_all(&self) -> Result<UpdateResult> {
        let _output = Command::new(self.exe)
            .args([
                "upgrade",
                "--all",
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .output()?;

        Ok(UpdateResult { updated: vec![], failed: vec![] })
    }

    fn upgrade(&self, package: &Package) -> Result<PackageUpgrade> {
        let _output = Command::new(self.exe)
            .args([
                "upgrade",
                "--id",
                &package.id,
                "--silent",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ])
            .output()?;

        // TODO: parse output to determine new version
        Ok(PackageUpgrade {
            from: package.clone(),
            to: package.clone(),
        })
    }
}
