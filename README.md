# Kenseader

A high-performance terminal RSS reader with AI-powered summarization and rich content display.

## Features

- **Terminal UI** - Beautiful TUI built with [ratatui](https://github.com/ratatui/ratatui)
- **Vim-Style Navigation** - Full vim keybindings for efficient navigation
- **AI Summarization** - Automatic article summaries via multiple AI providers (Claude, Gemini, OpenAI, Codex)
- **Background Scheduler** - Automatic feed refresh, article cleanup, and AI summarization in the background
- **Inline Image Display** - Images displayed at their original positions within article content
- **Rich Content Rendering** - Styled headings, quotes, code blocks, and lists
- **Protocol Auto-Detection** - Automatically selects best image protocol (Sixel/Kitty/iTerm2/Halfblocks)
- **Search** - Real-time search with `/` and navigate matches with `n`/`N`
- **RSSHub Support** - Native `rsshub://` protocol for easy subscriptions
- **SQLite Storage** - Fast, local database for feeds and articles
- **Auto Mark-Read** - Articles automatically marked as read when viewed

## Screenshots

```
┌─ Subscriptions ─┬─ Articles ──────────────────┬─ Article ─────────────────────┐
│ > Hacker News   │ ● Building a Rust CLI       │ Building a Rust CLI           │
│   Rust Blog     │   New features in 1.75      │                               │
│   GitHub Trend  │ ● Understanding async/await │ By John Doe | 2024-01-15      │
│                 │   Memory safety explained   │                               │
│                 │                             │ [Image displayed here]        │
│                 │                             │                               │
│                 │                             │ This article explains how to  │
│                 │                             │ build command-line tools...   │
├─────────────────┴─────────────────────────────┴───────────────────────────────┤
│ All | Subscriptions | 4 articles | q:quit h/l:panels j/k:move /:search        │
└───────────────────────────────────────────────────────────────────────────────┘
```

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
- Terminal with true color support (required for image display)

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
| `daemon start` | Start background daemon for auto-refresh and summarization |
| `daemon stop` | Stop the background daemon |
| `daemon status` | Check if daemon is running |

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
# Provider options: claude_cli, gemini_cli, codex_cli, openai, gemini_api, claude_api
provider = "claude_cli"
# Summary language (e.g., "English", "Chinese", "Japanese")
summary_language = "English"

# API keys (only needed for API-based providers)
# openai_api_key = "sk-..."
# gemini_api_key = "AIza..."
# claude_api_key = "sk-ant-..."

# Model names
openai_model = "gpt-4o-mini"
gemini_model = "gemini-2.0-flash"
claude_model = "claude-sonnet-4-20250514"

max_summary_tokens = 150
concurrency = 2

[ui]
tick_rate_ms = 100
show_author = true
show_timestamps = true
image_preview = true

[sync]
refresh_interval_secs = 300   # Auto-refresh interval (0 = disabled)
cleanup_interval_secs = 3600  # Old article cleanup interval
summarize_interval_secs = 60  # AI summarization interval
request_timeout_secs = 30
rate_limit_ms = 1000

[rsshub]
base_url = "https://rsshub.app"
```

## Image Display

Kenseader displays images inline within article content, at their original positions. The system automatically detects your terminal's capabilities and selects the best rendering method.

### Supported Protocols

| Protocol | Terminals | Quality |
|----------|-----------|---------|
| **Kitty Graphics** | Kitty | Highest |
| **iTerm2 Inline** | iTerm2 | High |
| **Sixel** | xterm, mlterm, foot, WezTerm, GNOME Terminal | High |
| **Halfblocks** | All terminals with true color | Medium |

### How It Works

1. **Auto-Detection** - Terminal capabilities are detected on startup
2. **Visible-First Loading** - Only images in the viewport are loaded first
3. **Async Download** - Images are downloaded in the background without blocking UI
4. **Dual Cache** - Memory cache for fast access + disk cache for persistence
5. **Fallback** - Graceful degradation to Unicode halfblock characters if no graphics protocol is available

### Configuration

```toml
[ui]
image_preview = true  # Set to false to disable images entirely
```

### Terminal Compatibility

For the best image quality, use a terminal with native graphics support:

- **macOS**: iTerm2, Kitty, WezTerm
- **Linux**: Kitty, foot, WezTerm, GNOME Terminal (with Sixel enabled)
- **Windows**: Windows Terminal (via WSL with Kitty/WezTerm)

For terminals without graphics support, images are rendered using Unicode halfblock characters (`▀`) with true colors. This works in any terminal supporting 24-bit color.

## Rich Content Rendering

Article content is parsed and rendered with formatting:

| Element | Display Style |
|---------|---------------|
| **Headings** | Bold, colored by level (H1: orange, H2: yellow, H3+: aqua) |
| **Quotes** | Italic with `|` prefix |
| **Code** | Green text with dark background |
| **Lists** | Bullet points with `•` prefix |
| **Links** | Displayed inline |
| **Images** | Rendered at original position |

## AI Providers

Kenseader supports multiple AI providers for article summarization. Choose between CLI-based providers (free, uses local CLI tools) or API-based providers (requires API key).

### CLI-Based Providers

CLI providers use locally installed AI CLI tools. They don't require API keys but need the respective CLI to be installed and authenticated.

| Provider | CLI Command | Installation |
|----------|-------------|--------------|
| `claude_cli` (Default) | `claude` | [Claude CLI](https://github.com/anthropics/claude-cli) |
| `gemini_cli` | `gemini` | [Gemini CLI](https://github.com/google/gemini-cli) |
| `codex_cli` | `codex` | [Codex CLI](https://github.com/openai/codex-cli) |

```toml
[ai]
provider = "claude_cli"  # or "gemini_cli" or "codex_cli"
summary_language = "Chinese"  # Summaries in Chinese
```

### API-Based Providers

API providers connect directly to AI services. They require an API key but offer more control and reliability.

| Provider | API Service | Model Examples |
|----------|-------------|----------------|
| `openai` | OpenAI API | gpt-4o, gpt-4o-mini |
| `gemini_api` | Google Gemini API | gemini-2.0-flash, gemini-1.5-pro |
| `claude_api` | Anthropic Claude API | claude-sonnet-4-20250514, claude-3-haiku |

```toml
[ai]
# OpenAI
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"

# Or Gemini API
provider = "gemini_api"
gemini_api_key = "AIza-your-key-here"
gemini_model = "gemini-2.0-flash"

# Or Claude API
provider = "claude_api"
claude_api_key = "sk-ant-your-key-here"
claude_model = "claude-sonnet-4-20250514"
```

### Summary Language

Configure the language for AI-generated summaries:

```toml
[ai]
summary_language = "English"   # Default
# summary_language = "Chinese"
# summary_language = "Japanese"
# summary_language = "Spanish"
# summary_language = "French"
```

### Batch Summarization

The background daemon uses batch summarization to process multiple articles in a single AI request, reducing API costs and improving efficiency.

- **Minimum Content Length**: Articles must have at least 1000 characters to be summarized
- **Batch Size Limits**: ~80,000 chars for Claude, ~100,000 chars for OpenAI/Gemini

## RSSHub Protocol

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
│   ├── kenseader-cli/    # CLI application and main entry point
│   ├── kenseader-core/   # Core library (feed parsing, storage, AI)
│   └── kenseader-tui/    # Terminal UI components
│       ├── app.rs        # Application state management
│       ├── rich_content.rs  # HTML parsing and image handling
│       └── widgets/      # UI widgets (article detail, list, etc.)
└── Cargo.toml            # Workspace configuration
```

## Performance

- **Lazy Loading** - Only visible images are loaded
- **Async I/O** - Non-blocking network and database operations
- **Memory Management** - Image cache limited to 20 images
- **Disk Cache** - Images cached at `~/.cache/kenseader/image_cache/`

## Background Daemon

Kenseader includes a background daemon that runs independently of the TUI to keep your feeds up-to-date.

### Starting the Daemon

```bash
# Start the background daemon
kenseader daemon start

# Check if daemon is running
kenseader daemon status

# Stop the daemon
kenseader daemon stop
```

### Scheduled Tasks

| Task | Default Interval | Description |
|------|------------------|-------------|
| **Feed Refresh** | 5 minutes | Fetches new articles from all subscribed feeds |
| **Article Cleanup** | 1 hour | Removes articles older than retention period |
| **AI Summarization** | 1 minute | Generates summaries for new articles |

### How It Works

1. **Independent Process** - Daemon runs separately from TUI, continues after TUI quits
2. **Graceful Shutdown** - Use `daemon stop` or Ctrl+C to stop cleanly
3. **PID File** - Tracks running daemon at `~/.local/share/kenseader/daemon.pid`
4. **Configurable Intervals** - Customize all intervals in the config file

### Configuration

```toml
[sync]
refresh_interval_secs = 300   # Feed refresh (0 = disabled)
cleanup_interval_secs = 3600  # Article cleanup
summarize_interval_secs = 60  # AI summarization
```

Set `refresh_interval_secs = 0` to disable the background scheduler entirely.

## Troubleshooting

### Images Not Displaying

1. Ensure `image_preview = true` in config
2. Check terminal supports true color: `echo $COLORTERM` should output `truecolor` or `24bit`
3. For best results, use iTerm2, Kitty, or WezTerm

### Slow Image Loading

1. Images are loaded asynchronously - scroll slowly to allow loading
2. Check network connection
3. Some websites block image hotlinking

### Memory Usage

If memory usage is high with many images:
- Images are automatically evicted from cache when limit is reached
- Restart the application to clear memory cache
- Disk cache persists between sessions

## License

MIT
