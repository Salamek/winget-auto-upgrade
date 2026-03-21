# Winget Auto Upgrade

Automated package updater for [winget](https://github.com/microsoft/winget-cli) written in Rust. Designed as a lightweight, configurable alternative/companion to [WAU](https://github.com/Romanitho/Winget-AutoUpdate) with full compatibility with its registry configuration (in sense of implemented functionality).

## Features

- Automatically lists and installs available winget upgrades
- Allow list, block list, and per-package override options
- Scope-aware filtering — separate behaviour for user and SYSTEM context
- Skips updates on metered connections
- Pre/post update hooks — run arbitrary executables around each upgrade
- Layered configuration: `config.toml` → WAU registry → Group Policy registry
- Native Windows toast notifications with configurable level
- Logs to file and terminal simultaneously

## Installation

```
cargo build --release
```

Copy the resulting binary to a location of your choice. To run automatically, register it as a Scheduled Task (recommended: one task running as SYSTEM for machine-scope updates, one running as the logged-in user for user-scope updates).

## Configuration

All configuration is optional — the binary runs with sensible defaults if no config file is present.

### Priority order (highest wins)

1. `HKLM\Software\Policies\Romanitho\Winget-AutoUpdate` (Group Policy)
2. `HKLM\SOFTWARE\Romanitho\Winget-AutoUpdate` (WAU registry settings)
3. `config.toml` (next to the binary)
4. Built-in defaults

### config.toml

```toml
# Path to the log file
log_path = "winget-update.log"

# Default package source when not specified in a list
default_source = "winget"

# Package lists — file:// or https:// URLs
allow_list_path    = "file://allow_list.toml"
block_list_path    = "file://block_list.toml"
override_list_path = "file://override_list.toml"

# Skip packages whose current version is reported as "Unknown"
skip_unknown_version = true

# Allow updates to run on a metered (cellular/limited) connection
run_on_metered_connection = false

# Notification verbosity: "all" | "success" | "error" | "none"
notification_level = "all"

# Hooks — paths to executables run before/after each package upgrade
# pre_update_hook  = "C:\\Scripts\\pre.bat"
# post_update_hook = "C:\\Scripts\\post.bat"

# Arguments passed to hooks — supports template variables (see Hooks section)
hook_args_template = "{id} {source} {version} {available_version}"
```

## Package Lists

Allow, block, and override lists share the same TOML format. Lists can be loaded from a local file (`file://`) or a remote URL (`https://`).

### Allow list

If non-empty, only packages present in the list are upgraded. An empty or missing allow list means all packages are eligible (subject to the block list).

```toml
[[packages]]
id = "Mozilla.Firefox"
source = "winget"   # optional, defaults to config default_source
scope = "all"       # optional: "user" | "machine" | "all" (default)

[[packages]]
id = "7zip.7zip"
```

### Block list

Packages listed here are never upgraded regardless of the allow list.

```toml
[[packages]]
id = "Microsoft.Teams"

[[packages]]
id = "Company.LegacyApp"
scope = "user"   # only block in user context
```

### Override list

Per-package upgrade options. The first entry matching the package identity and current scope is used.

```toml
[[packages]]
id = "Mozilla.Firefox"
source = "winget"
scope = "machine"
force_architecture = "x64"
force_locale = "en-US"

[[packages]]
id = "Company.CustomApp"
override_args = "/quiet /norestart"
ignore_security_hash = true
skip_dependencies = true
```

#### Package entry fields

| Field | Type | Description |
|---|---|---|
| `id` | string | **Required.** Winget package identifier |
| `source` | string | Package source. Defaults to `default_source` |
| `scope` | string | `"user"`, `"machine"`, or `"all"` (default) |
| `custom_args` | string | Extra arguments appended to the winget command |
| `override_args` | string | Passed to winget `--override` (replaces installer args) |
| `force_architecture` | string | Passed to winget `--architecture` |
| `force_locale` | string | Passed to winget `--locale` |
| `ignore_security_hash` | bool | Adds `--ignore-security-hash` |
| `skip_dependencies` | bool | Adds `--skip-dependencies` |

### Scope

Scope controls in which execution context a list entry is active:

| Scope | Active when |
|---|---|
| `all` | Always (default) |
| `machine` | Running as SYSTEM |
| `user` | Running as a regular user |

This allows a single set of list files to serve both Scheduled Task contexts without duplication.

## Hooks

Hooks are arbitrary executables (scripts, binaries) called before and after each individual package upgrade. Hook failures are logged as warnings but do not abort the upgrade.

### Hook variables

The `hook_args_template` config value is interpolated for each package:

| Variable | Description |
|---|---|
| `{id}` | Package identifier |
| `{name}` | Package display name |
| `{source}` | Package source |
| `{scope}` | Current execution scope (`user` or `machine`) |
| `{version}` | Installed version (pre-update hook: current; post-update hook: newly installed) |
| `{available_version}` | Version being upgraded to |

### Example

```toml
pre_update_hook    = "C:\\Scripts\\close-app.exe"
post_update_hook   = "C:\\Scripts\\notify-team.ps1"
hook_args_template = "{id} {version} {available_version}"
```

`close-app.exe` is called with e.g. `Microsoft.Teams 25017.203 26043.201` before the upgrade, and `notify-team.ps1` is called with the actual installed version after.

## WAU Compatibility

The registry key layout follows [Winget-AutoUpdate](https://github.com/Romanitho/Winget-AutoUpdate). The following WAU registry values are read:

| Registry value | Config field |
|---|---|
| `WAU_WingetSourceCustom` | `default_source` |
| `WAU_DoNotRunOnMetered` | `run_on_metered_connection` |
| `WAU_NotificationLevel` | `notification_level` |

WAU notification level names (`Full`, `Success only`, `Errors only`, `None`) are mapped to the internal enum automatically.

## Winget detection (Windows)

The binary resolves the winget executable in this order:

1. `%ProgramFiles%\WindowsApps\Microsoft.DesktopAppInstaller_*_8wekyb3d8bbwe\winget.exe` — newest version wins
2. `%LOCALAPPDATA%\Microsoft\WindowsApps\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\winget.exe`
3. `winget` via `PATH`

## License

GPL — see [LICENSE](LICENSE).
