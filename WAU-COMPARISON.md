# Feature Parity: winget-autoupgrade vs Winget-AutoUpdate (WAU)

> **Design differences to keep in mind:**
> - WAU is PowerShell-based; this project is a native Rust binary
> - WAU manages its own scheduled tasks; this project is a plain executable invoked by a scheduled task created by the MSI installer
> - WAU has a built-in mod/extension system; this project deliberately excludes it — external tools should be called via pre/post hooks instead (separate project planned)
> - WAU self-updates; this project is distributed/updated via MSI

---

## Core Update Logic

| Feature | WAU | This project | Notes |
|---|---|---|---|
| `winget upgrade` detection | ✅ | ✅ | |
| Allow list (whitelist mode) | ✅ | ✅ | WAU: `included_apps.txt`; this project: `allow_list.toml` |
| Block list (blacklist mode) | ✅ | ✅ | WAU: `excluded_apps.txt`; this project: `block_list.toml` |
| Override list (per-package options) | ✅ | ✅ | WAU: mods system; this project: `override_list.toml` |
| Skip packages with unknown version | ✅ | ✅ | |
| Force architecture per package | ✅ | ✅ | |
| Force locale per package | ✅ | ✅ | |
| Custom/override installer args | ✅ | ✅ | |
| Ignore security hash per package | ✅ | ✅ | |
| Skip dependencies per package | ✅ | ✅ | |
| Machine vs user scope filtering | ✅ | ✅ | WAU: `WAU_UserContext`; this project: scope field in package lists |
| Allow user context to bypass system lists | ✅ | ✅ | WAU: `WAU_BypassListForUsers` (global toggle); this project: `scope` field per entry — more granular |
| Metered connection detection | ✅ | ✅ | WAU: `WAU_DoNotRunOnMetered` |
| Pre-update hook | ✅ | ✅ | WAU: mods (`_pre.ps1`); this project: `pre_update_hook` config |
| Post-update hook | ✅ | ✅ | WAU: mods (`_notify.ps1`); this project: `post_update_hook` config |
| Mods system (download & exec scripts) | ✅ | ❌ | **Deliberately excluded** — security risk; use hooks + separate project instead |

---

## Notifications

| Feature | WAU | This project | Notes |
|---|---|---|---|
| Toast notifications | ✅ | ✅ | |
| Notification levels (Full/SuccessOnly/ErrorsOnly/None) | ✅ | ✅ | `WAU_NotificationLevel` |
| Per-notification icons (info/success/warning/error) | ✅ | ✅ | |
| Single updating notification (tag-based replace) | ✅ | ✅ | |
| Notify logged-in user when running as SYSTEM | ✅ | ❌ | WAU saves XML and triggers `Winget-AutoUpdate-Notify` scheduled task to display in user context. This project drops the notification when running as SYSTEM. Needs a relay mechanism or separate notify helper exe. |
| Multi-user notification (all logged-in users) | ✅ | ❌ | Depends on the SYSTEM relay above |

---

## Registry Configuration

| WAU Key | WAU | This project | Notes |
|---|---|---|---|
| `WAU_NotificationLevel` | ✅ | ✅ | Values differ slightly: WAU uses "SuccessOnly"/"ErrorsOnly"; code maps them |
| `WAU_DoNotRunOnMetered` | ✅ | ✅ | |
| `WAU_WingetSourceCustom` | ✅ | ✅ | Maps to `default_source` |
| `WAU_UseWhiteList` | ✅ | ✅ | Implicit: non-empty allow list = whitelist mode |
| `WAU_ListPath` | ✅ | ✅ | Equivalent: `allow_list_path` / `block_list_path` / `override_list_path` in config |
| `WAU_UserContext` | ✅ | ✅ | Equivalent: scope-based filtering in package lists |
| `WAU_MaxLogFiles` | ✅ | ❌ | Log rotation not implemented |
| `WAU_MaxLogSize` | ✅ | ❌ | Log rotation not implemented |
| `WAU_UpdatesAtLogon` | ✅ | ❌ | Scheduling is MSI/Task Scheduler responsibility |
| `WAU_UpdatesAtTime` | ✅ | ❌ | Scheduling is MSI/Task Scheduler responsibility |
| `WAU_UpdatesTimeDelay` | ✅ | ❌ | Scheduling is MSI/Task Scheduler responsibility |
| `WAU_UpdatesInterval` | ✅ | ❌ | Scheduling is MSI/Task Scheduler responsibility |
| `WAU_DisableAutoUpdate` | ✅ | ❌ | WAU self-update only; not applicable |
| `WAU_UpdatePrerelease` | ✅ | ❌ | WAU self-update only; not applicable |
| `WAU_ModsPath` | ✅ | ❌ | Mods deliberately excluded |
| `WAU_AzureBlobSASURL` | ✅ | ❌ | WAU uses this for mods auth; list paths already support HTTPS URLs |
| `WAU_BypassListForUsers` | ✅ | ✅ | Superseded by `scope` field in package lists — per-entry machine/user/all control is more granular than a global bypass toggle |

Group Policy overrides (`HKLM\Software\Policies\Romanitho\Winget-AutoUpdate`) are supported for the three keys this project reads.

---

## Package Lists

| Feature | WAU | This project | Notes |
|---|---|---|---|
| Local file lists | ✅ | ✅ | |
| Remote HTTPS lists | ✅ | ✅ | |
| Azure Blob Storage lists | ✅ | ✅ | Via HTTPS URL |
| SharePoint lists | ✅ | ✅ | Via HTTPS URL |
| Format | Plain text (one ID per line) | TOML (structured, with per-entry options) | |
| Per-entry scope (machine/user/all) | ✅ | ✅ | |
| Per-entry upgrade options | via mods | ✅ | This project folds mods-style options into the override list |

---

## Scheduling & Deployment

| Feature | WAU | This project | Notes |
|---|---|---|---|
| Scheduled task — SYSTEM context | ✅ | 🔲 MSI | To be created by WiX MSI installer |
| Scheduled task — user context at logon | ✅ | 🔲 MSI | Optional; for per-user updates |
| Scheduled task — user notification relay | ✅ | ❌ | `Winget-AutoUpdate-Notify` task; see Notifications gap above |
| MSI installer | ✅ | 🔲 planned | |
| GPO ADMX template | ✅ | ❌ | Not planned |
| Self-update | ✅ | ❌ | Intentionally excluded; use MSI |

---

## Logging

| Feature | WAU | This project | Notes |
|---|---|---|---|
| File logging | ✅ | ✅ | |
| Console/terminal logging | ✅ | ✅ | |
| Log rotation by file count (`WAU_MaxLogFiles`) | ✅ | ❌ | Not implemented |
| Log rotation by file size (`WAU_MaxLogSize`) | ✅ | ❌ | Not implemented |
| Per-package log files | ✅ | ❌ | WAU writes a separate log per package; this project uses a single log |

---

## Summary

**At parity:** core update loop, all package list types and filtering, all per-package upgrade options, hooks, notifications with icons and level filtering, metered connection detection, machine/user scope, WAU registry + group policy config layers.

**Intentionally out of scope:** mods system, self-update, ADMX, scheduling (MSI responsibility).

**Gaps worth addressing:**
1. **SYSTEM → user notification relay** — most impactful; without it the binary is silent when run as SYSTEM.
2. **Log rotation** — `WAU_MaxLogFiles` / `WAU_MaxLogSize` equivalents.
