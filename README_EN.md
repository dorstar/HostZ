# HostZ

> Lightweight system hosts manager — Rust + Tauri v2 rewrite of SwitchHosts

[![Rust](https://img.shields.io/badge/rust-1.95+-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-2.11-blue.svg)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## Features

- **Ultra Lightweight**: 10 MB binary, < 50 MB memory, < 1s startup (vs SwitchHosts Electron ~200 MB memory, HostZ uses only 1/20)
- **Three Entry Types**: Local (editable), Remote (HTTP fetch), Group (aggregate multiple entries)
- **Append & Overwrite Modes**: Append mode preserves existing system hosts content
- **Syntax Highlighting**: Comments (green), IPs (blue) in read-only mode
- **Find & Replace**: Regex support, cross-entry search
- **Import & Export**: Native file dialogs, one-click config migration
- **Dark Theme**: 35+ CSS variables with full coverage
- **Bilingual (zh-CN / en)**: Switch in settings — tray, toolbar, dialogs all follow
- **Trash Can**: Soft delete + restore + permanent delete
- **System Tray**: Close minimizes to tray, right-click menu for quick actions
- **Write History**: Auto-record each system hosts write, with history viewer

## Installation

Download `HostZ_1.0.0_x64-setup.exe` from [Releases](../../releases) (~3 MB NSIS installer).

Or run `hostz.exe` directly (portable, no installation needed).

**Requirement**: Windows 10+ (WebView2 included), macOS / Linux need WebView2 installed separately.

## Build

```bash
# Requires: Rust 1.95+
cd src-tauri
cargo tauri build

# Output: src-tauri/target/release/bundle/nsis/HostZ_1.0.0_x64-setup.exe
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Shell | Tauri v2 |
| Frontend | Pure HTML/CSS/JS (no framework, zero dependencies) |
| Icons | Octicons 16px SVG (CSS mask-image) |
| Database | SQLite (rusqlite bundled) |
| HTTP | ureq |
| Scheduler | std::thread |

## Project Structure

```
HostZ/
├── dist/                  # Frontend static assets
│   ├── index.html
│   ├── style.css
│   ├── app.js
│   └── icons/             # Octicons SVG icons
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri entry + 27 commands
│   │   ├── models/        # Data models
│   │   ├── db/            # SQLite data layer
│   │   ├── core/          # Core business logic
│   │   ├── services/      # Tray / Scheduler / Notifications / Hotkeys
│   │   └── utils/         # Platform / i18n
│   └── tests/             # Integration tests (19)
└── docs/                  # Documentation
```

## Data Storage

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\HostZ\data\hostz.db` |
| macOS | `~/Library/Application Support/HostZ/data/hostz.db` |
| Linux | `~/.config/HostZ/data/hostz.db` |

## License

MIT
