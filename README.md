# Kenseader

A high-performance terminal RSS reader with AI-powered summarization.

## Features

- **Terminal UI** - Beautiful TUI built with [ratatui](https://github.com/ratatui/ratatui)
- **AI Summarization** - Automatic article summaries via Claude CLI or OpenAI
- **RSSHub Support** - Native `rsshub://` protocol for easy subscriptions
- **SQLite Storage** - Fast, local database for feeds and articles
- **Keyboard-Driven** - Efficient navigation without leaving the terminal

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

### Keyboard Shortcuts (TUI)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Select / Open |
| `o` | Open article in browser |
| `r` | Refresh feeds |
| `s` | Summarize article (AI) |
| `q` | Quit |

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
