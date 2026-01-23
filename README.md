# Kenseader

[![CI](https://github.com/kenxcomp/kenseader/actions/workflows/release.yml/badge.svg)](https://github.com/kenxcomp/kenseader/actions)
[![Release](https://img.shields.io/github/v/release/kenxcomp/kenseader)](https://github.com/kenxcomp/kenseader/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-orange)](https://github.com/kenxcomp/homebrew-tap)

A high-performance terminal RSS reader with AI-powered summarization and rich content display.

![Normal Mode](src/normal%20mode.png)

## Quick Start

```bash
# Install via Homebrew (macOS/Linux)
brew tap kenxcomp/tap && brew install kenseader

# Start daemon & TUI
brew services start kenseader
kenseader run
```

## Features

- 🖥️ **Terminal UI** - Beautiful TUI built with [ratatui](https://github.com/ratatui/ratatui)
- ⌨️ **Vim-Style Navigation** - Full vim keybindings for efficient navigation
- 🤖 **AI Summarization** - Automatic article summaries via Claude, Gemini, OpenAI
- 🎯 **Smart Filtering** - AI-powered relevance scoring based on your interests
- 🏷️ **Style Classification** - AI classifies articles by style, tone, and length
- 🖼️ **Inline Images** - Images displayed at original positions (Sixel/Kitty/iTerm2/Halfblocks)
- 🔍 **Real-time Search** - `/` to search, `n`/`N` to navigate matches
- 📦 **RSSHub Support** - Native `rsshub://` protocol for easy subscriptions
- 📋 **Batch Selection** - Yazi-style selection with `Space` and Visual mode with `v`
- 📚 **Reading History** - Navigate history with `u` (back) and `Ctrl+r` (forward)
- 🔄 **Background Scheduler** - Auto-refresh, cleanup, and AI processing
- 💾 **SQLite Storage** - Fast, local database for feeds and articles
- ✨ **Smooth Scrolling** - nvim-like smooth scroll animations with configurable easing

## Screenshots

### Normal Mode
![Normal Mode](src/normal%20mode.png)

### Unread-Only Mode
![Unread-Only Mode](src/unread-only%20mode.png)

## Terminal Compatibility

| Terminal | macOS | Linux | Windows | Image Protocol |
|----------|-------|-------|---------|----------------|
| iTerm2   | ✅    | -     | -       | iTerm2 Inline  |
| Kitty    | ✅    | ✅    | -       | Kitty Graphics |
| WezTerm  | ✅    | ✅    | ✅      | iTerm2 Inline  |
| foot     | -     | ✅    | -       | Sixel          |
| Others   | ✅    | ✅    | ✅      | Halfblocks     |

<details>
<summary>📦 Installation (more options)</summary>

### Homebrew (macOS/Linux)

```bash
# Add the tap
brew tap kenxcomp/tap

# Install kenseader
brew install kenseader

# Start the daemon as a background service (recommended)
brew services start kenseader

# Or start manually
kenseader daemon start

# Run the TUI
kenseader run
```

### From Source

```bash
# Clone the repository
git clone https://github.com/kenxcomp/kenseader.git
cd kenseader

# Build release binary
cargo build --release

# Binary will be at ./target/release/kenseader
```

### Requirements

- Rust 1.70+
- SQLite (bundled via sqlx)
- Terminal with true color support (required for image display)

</details>

## Architecture

Kenseader uses a **client-server architecture** with the TUI and daemon running as separate processes:

```
┌─────────────────┐         Unix Socket         ┌─────────────────────┐
│  kenseader run  │  ◄────────────────────────► │  kenseader daemon   │
│   (Pure TUI)    │      JSON-RPC Protocol      │   (Backend Service) │
└─────────────────┘                             └─────────────────────┘
                                                         │
                                                         ▼
                                                ┌─────────────────────┐
                                                │      SQLite DB      │
                                                └─────────────────────┘
```

- **Daemon** (`kenseader daemon start`): Handles all backend operations - feed refresh, article cleanup, AI summarization, database access
- **TUI** (`kenseader run`): Pure frontend that communicates with daemon via IPC
- **IPC Socket**: `~/.local/share/kenseader/kenseader.sock` (Unix socket)

## Usage

```bash
# 1. Subscribe to feeds (can be done without daemon)
kenseader subscribe --url https://hnrss.org/frontpage --name "Hacker News"
kenseader -s https://blog.rust-lang.org/feed.xml -n "Rust Blog"

# 2. Start the daemon (REQUIRED before running TUI)
kenseader daemon start

# 3. Launch the TUI
kenseader run

# 4. When done, stop the daemon
kenseader daemon stop
```

> **Important**: The TUI requires the daemon to be running. If you try to run `kenseader run` without starting the daemon first, you'll see an error message.

### Commands

| Command | Description |
|---------|-------------|
| `run` | Start the TUI interface |
| `run --read-mode` | Start TUI in read-mode (direct database access, no daemon required) |
| `subscribe` | Subscribe to an RSS feed |
| `unsubscribe` | Unsubscribe from a feed |
| `import` | Import subscriptions from OPML file |
| `list` | List all subscriptions |
| `refresh` | Refresh all feeds |
| `cleanup` | Clean up old articles |
| `daemon start` | Start background daemon for auto-refresh and summarization |
| `daemon stop` | Stop the background daemon |
| `daemon status` | Check if daemon is running |

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `h/j/k/l` | Vim-style navigation |
| `gg` / `G` | Jump to top/bottom |
| `Enter` | Select article / Open fullscreen image |
| `b` | Open in browser |
| `s` | Toggle saved/bookmark |
| `d` | Toggle read/unread |
| `r` | Refresh feeds |
| `i` | Toggle unread-only mode |
| `/` | Search |
| `q` | Quit |

See [full keybindings documentation](docs/keybindings.md) for all shortcuts.

## Configuration

Configuration file: `~/.config/kenseader/config.toml`

```toml
[ai]
enabled = true
provider = "claude_cli"  # claude_cli, gemini_cli, openai, gemini_api, claude_api
summary_language = "English"

[ui]
image_preview = true

[ui.scroll]
smooth_enabled = true        # Enable smooth scrolling (default: true)
animation_duration_ms = 150  # Animation duration in milliseconds
easing = "cubic"             # Easing: none, linear, cubic, quintic, easeout

[sync]
refresh_interval_secs = 3600
```

See [full configuration documentation](docs/configuration.md) for all options.

## Documentation

| Topic | Description |
|-------|-------------|
| [Configuration](docs/configuration.md) | Full config reference, keybinding customization, RSSHub setup |
| [Keybindings](docs/keybindings.md) | Complete keyboard shortcuts reference |
| [Image Display](docs/image-display.md) | Image protocols, terminal compatibility, troubleshooting |
| [AI Providers](docs/ai-providers.md) | CLI/API providers, batch summarization, smart filtering |
| [Background Daemon](docs/daemon.md) | Scheduled tasks, IPC API, configuration |
| [Cloud Sync](docs/cloud-sync.md) | iCloud/Dropbox sync, read-mode for multi-device |

## Project Structure

```
kenseader/
├── crates/
│   ├── kenseader-cli/    # CLI application and main entry point
│   ├── kenseader-core/   # Core library (feed parsing, storage, AI)
│   └── kenseader-tui/    # Terminal UI components
└── Cargo.toml            # Workspace configuration
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

- 🐛 [Report bugs](https://github.com/kenxcomp/kenseader/issues)
- 💡 [Request features](https://github.com/kenxcomp/kenseader/issues)
- 🔧 [Submit PRs](https://github.com/kenxcomp/kenseader/pulls)

## License

MIT
