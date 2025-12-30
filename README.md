# Kenseader

A high-performance terminal RSS reader with AI-powered summarization.

## Features

- **Terminal UI** - Beautiful TUI built with [ratatui](https://github.com/ratatui/ratatui)
- **Vim-Style Navigation** - Full vim keybindings for efficient navigation
- **AI Summarization** - Automatic article summaries via Claude CLI or OpenAI
- **Image Preview** - Inline image display in terminal (Sixel/Kitty/iTerm2)
- **Search** - Quick search with `/` and navigate matches with `n`/`N`
- **RSSHub Support** - Native `rsshub://` protocol for easy subscriptions
- **SQLite Storage** - Fast, local database for feeds and articles
- **Auto Mark-Read** - Articles automatically marked as read when viewed

## Installation

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
- Terminal with Sixel/Kitty/iTerm2 support (optional, for image preview)

## Usage

### Quick Start

```bash
# Subscribe to a feed
kenseader subscribe --url https://hnrss.org/frontpage --name "Hacker News"

# Or use shorthand
kenseader -s https://blog.rust-lang.org/feed.xml -n "Rust Blog"

# Refresh feeds
kenseader refresh

# Launch the TUI
kenseader run
```

### Commands

| Command | Description |
|---------|-------------|
| `run` | Start the TUI interface |
| `subscribe` | Subscribe to an RSS feed |
| `unsubscribe` | Unsubscribe from a feed |
| `list` | List all subscriptions |
| `refresh` | Refresh all feeds |
| `cleanup` | Clean up old articles |

## Keyboard Shortcuts (TUI)

### Navigation

| Key | Action |
|-----|--------|
| `h` / `←` | Move to left panel |
| `l` / `→` | Move to right panel |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `gg` | Jump to top (press `g` twice) |
| `G` | Jump to bottom |
| `Ctrl+d` | Scroll half page down |
| `Ctrl+u` | Scroll half page up |
| `Ctrl+f` | Scroll full page down |
| `Ctrl+b` | Scroll full page up |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Select / Open article |
| `b` | Open article in browser (in detail view) |
| `s` | Toggle saved/bookmark |
| `d` | Delete subscription (with confirmation) |
| `r` | Refresh feeds |
| `i` | Toggle unread-only mode |

### Search

| Key | Action |
|-----|--------|
| `/` | Start forward search |
| `?` | Start backward search |
| `n` | Go to next match |
| `N` | Go to previous match |
| `Enter` | Confirm search |
| `Esc` | Cancel search |

### General

| Key | Action |
|-----|--------|
| `Esc` | Exit current mode |
| `q` | Quit application |

## Configuration

Configuration file location: `~/.config/kenseader/config.toml`

```toml
[general]
article_retention_days = 3
log_level = "info"

[ai]
enabled = true
provider = "claude_cli"  # or "openai"
# openai_api_key = "sk-..."  # Required for OpenAI
# openai_model = "gpt-4o-mini"
max_summary_tokens = 150
concurrency = 2

[ui]
tick_rate_ms = 100
show_author = true
show_timestamps = true
image_preview = true

[sync]
refresh_interval_secs = 300
request_timeout_secs = 30
rate_limit_ms = 1000

[rsshub]
base_url = "https://rsshub.app"
```

### Image Preview

Kenseader supports inline image preview in terminals that support graphical protocols:

- **Sixel** - xterm, mlterm, foot, etc.
- **Kitty** - Kitty terminal
- **iTerm2** - iTerm2 on macOS

To enable/disable image preview:

```toml
[ui]
image_preview = true  # Set to false to disable
```

Images are automatically extracted from article content and displayed at the top of the article detail view.

### AI Providers

#### Claude CLI (Default)

Uses the Claude CLI for summarization. Requires [Claude CLI](https://github.com/anthropics/claude-cli) to be installed and authenticated.

#### OpenAI

Set `provider = "openai"` and provide your API key:

```toml
[ai]
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"
```

### RSSHub Protocol

Subscribe to RSSHub routes directly:

```bash
# These are equivalent:
kenseader -s rsshub://github/trending/daily -n "GitHub Trending"
kenseader -s https://rsshub.app/github/trending/daily -n "GitHub Trending"
```

Configure a custom RSSHub instance:

```toml
[rsshub]
base_url = "https://your-rsshub-instance.com"
```

## Project Structure

```
kenseader/
├── crates/
│   ├── kenseader-cli/    # CLI application
│   ├── kenseader-core/   # Core library (feed, storage, AI)
│   └── kenseader-tui/    # Terminal UI components
└── Cargo.toml            # Workspace configuration
```

## License

MIT
