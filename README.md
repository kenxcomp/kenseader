# Kenseader

A high-performance terminal RSS reader with AI-powered summarization and rich content display.

## Features

- **Terminal UI** - Beautiful TUI built with [ratatui](https://github.com/ratatui/ratatui)
- **Vim-Style Navigation** - Full vim keybindings for efficient navigation
- **AI Summarization** - Automatic article summaries via multiple AI providers (Claude, Gemini, OpenAI, Codex)
- **Smart Article Filtering** - AI-powered relevance scoring based on user interests, auto-filters low-relevance articles
- **Article Style Classification** - AI classifies articles by style (tutorial, news, opinion, analysis, review), tone, and length
- **Background Scheduler** - Automatic feed refresh, article cleanup, AI summarization, and filtering in the background
- **Inline Image Display** - Images displayed at their original positions within article content
- **Rich Content Rendering** - Styled headings, quotes, code blocks, lists, and hyperlinks
- **Link Navigation** - Tab through URLs and images in article content, open links in browser
- **Protocol Auto-Detection** - Automatically selects best image protocol (Sixel/Kitty/iTerm2/Halfblocks)
- **Search** - Real-time search with `/` and navigate matches with `n`/`N`, with highlighting
- **Batch Selection** - Yazi-style batch selection with `Space` and Visual mode with `v` for bulk operations
- **Reading History** - Navigate through reading history with `u` (back) and `Ctrl+r` (forward)
- **OPML Import** - Import subscriptions from OPML files for easy migration
- **Loading Indicators** - Animated spinner during feed refresh operations
- **Error Display** - Feeds with fetch errors are highlighted in red
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

### Quick Start

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

> **Important**: The TUI requires the daemon to be running. If you try to run `kenseader run` without starting the daemon first, you'll see:
> ```
> Daemon is not running.
> Please start the daemon first with:
>   kenseader daemon start
> ```

### Commands

| Command | Description |
|---------|-------------|
| `run` | Start the TUI interface |
| `subscribe` | Subscribe to an RSS feed |
| `unsubscribe` | Unsubscribe from a feed |
| `import` | Import subscriptions from OPML file |
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
| `Ctrl+j` | Next article (in detail view, respects unread-only mode) |
| `Ctrl+k` | Previous article (in detail view, respects unread-only mode) |

### Actions

| Key | Action |
|-----|--------|
| `Enter` | Select article / Open fullscreen image viewer (in detail view) |
| `b` | Open article in browser (article list/detail view) |
| `s` | Toggle saved/bookmark |
| `d` | Toggle read/unread (article list) / Delete subscription (feed list, with confirmation) |
| `r` | Refresh feeds (async, non-blocking) |
| `i` | Toggle unread-only mode |
| `u` | Go back in reading history |
| `Ctrl+r` | Go forward in reading history |

### Batch Selection (Yazi-style)

| Key | Action |
|-----|--------|
| `Space` | Toggle selection and move to next item |
| `v` | Enter Visual mode for range selection |
| `Esc` | Exit Visual mode / Clear selection |
| `d` | Batch toggle read (articles) / Delete selected (feeds) |

Visual mode tips:
- Use `gg` then `v` then `G` to select all items
- Selected items show ✓ marker with purple background
- Status bar shows `VISUAL` mode and selection count

### Image & Link Navigation (Article Detail)

| Key | Action |
|-----|--------|
| `Tab` | Focus next image/link (in document order) |
| `Shift+Tab` | Focus previous image/link |
| `Enter` | Open fullscreen image viewer |
| `o` | Smart open: open focused link in browser, or focused image in external viewer |
| `b` | Smart open: open focused link in browser, or article's main URL if nothing focused |

Links in article content are displayed with blue underlined text. When focused, links are highlighted with a yellow background.

### Fullscreen Image Viewer

| Key | Action |
|-----|--------|
| `n` / `l` / `→` / `Space` | Next image |
| `p` / `h` / `←` | Previous image |
| `o` / `Enter` | Open image in external viewer |
| `q` / `Esc` | Exit fullscreen mode |

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

Configuration file location: `~/.config/kenseader/config.toml` (same on all platforms)

> **Note**: The `config/default.toml` file in the project directory is just a template. The application reads configuration from `~/.config/kenseader/config.toml`. If the config file doesn't exist, default values are used. To customize settings, copy the template to the correct location:
>
> ```bash
> mkdir -p ~/.config/kenseader
> cp config/default.toml ~/.config/kenseader/config.toml
> ```

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

# Article filtering settings
min_summarize_length = 500    # Minimum chars for AI summarization
max_summary_length = 150      # Maximum summary output length
relevance_threshold = 0.3     # Articles below this score are auto-filtered (0.0-1.0)

[ui]
tick_rate_ms = 100
show_author = true
show_timestamps = true
image_preview = true

[sync]
refresh_interval_secs = 3600  # Scheduler check interval (0 = disabled)
feed_refresh_interval_secs = 43200  # Per-feed refresh interval (12 hours)
cleanup_interval_secs = 3600  # Old article cleanup interval
summarize_interval_secs = 60  # AI summarization interval
filter_interval_secs = 120    # Article filtering interval
request_timeout_secs = 30
rate_limit_ms = 1000
# proxy_url = "http://127.0.0.1:7890"  # HTTP/SOCKS5 proxy for feed fetching

[rsshub]
base_url = "https://hub.slarker.me"  # Default (rsshub.app is Cloudflare protected)
# access_key = "your_access_key"  # For instances requiring authentication
```

## Image Display

Kenseader displays images inline within article content, at their original positions. The system automatically detects your terminal's capabilities and selects the best rendering method.

### Supported Protocols

| Protocol | Terminals | Quality | Note |
|----------|-----------|---------|------|
| **Üeberzug++** | X11/Wayland terminals | Highest (native resolution) | Recommended for Linux |
| **Kitty Graphics** | Kitty | Highest | Native protocol |
| **iTerm2 Inline** | iTerm2, WezTerm | High | macOS native |
| **Sixel** | xterm, mlterm, foot, contour | High | Wide support |
| **Halfblocks** | All terminals with true color | Medium (uses `▀` characters) | Universal fallback |

### Üeberzug++ (Recommended for Linux)

For the best image quality on Linux, install [Üeberzug++](https://github.com/jstkdng/ueberzugpp):

```bash
# Arch Linux
sudo pacman -S ueberzugpp

# Fedora
sudo dnf install ueberzugpp

# Ubuntu/Debian (from source or AppImage)
# See: https://github.com/jstkdng/ueberzugpp#installation
```

Üeberzug++ renders images as native overlay windows, providing true high-resolution display regardless of terminal limitations. Kenseader automatically detects and uses it when available in X11/Wayland environments.

### How It Works

1. **Auto-Detection** - Terminal capabilities and environment are detected on startup
2. **Backend Selection** - The best available backend is chosen automatically:
   - On X11/Wayland with Üeberzug++: Native window overlay (highest quality)
   - On Kitty/iTerm2/WezTerm: Native terminal protocols
   - Fallback: Unicode halfblock characters (`▀`)
3. **Visible-First Loading** - Only images in the viewport are loaded first
4. **Async Download** - Images are downloaded in the background without blocking UI
5. **Dual Cache** - Memory cache for fast access + disk cache for persistence
6. **Graceful Fallback** - Degrades to halfblocks if no high-resolution option is available

### Fullscreen Image Viewer

When viewing an article, press `Enter` to open the fullscreen image viewer:

- **High-resolution display** - Uses the full terminal window for maximum image clarity
- **Navigate between images** - Use `n`/`p` or arrow keys to switch images
- **Open externally** - Press `o` to open the image in your system's default image viewer
- **Works on any terminal** - Even with halfblocks, fullscreen mode provides better resolution than inline display

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

The background daemon uses intelligent batch summarization to process multiple articles in a single AI request, maximizing efficiency and reducing API costs.

#### Dynamic Batch Processing

- **No Fixed Article Limit**: Articles are packed into batches dynamically based on content size
- **Token Limit**: Each batch targets ~100k tokens (~200k characters) for optimal API utilization
- **Content Truncation**: Long articles are automatically truncated to 4,000 characters to ensure more articles fit per batch
- **Smart Filtering**: Already-read articles are automatically excluded before each batch request to avoid wasting tokens

#### Processing Flow

```
1. Fetch unread articles without summaries (up to 500 per cycle)
2. For each batch:
   a. Re-check article read status (skip if marked read)
   b. Pack articles until reaching ~100k token limit
   c. Send batch request to AI
   d. Save summaries to database
3. Continue until all articles are processed
```

#### Configuration

```toml
[ai]
# Minimum content length for AI summarization (chars)
min_summarize_length = 500
```

#### Efficiency Example

```
Found 235 articles to summarize
Batch 1: 72 articles, 197,442 chars
Batch 2: 60 articles, 198,694 chars
Batch 3: 93 articles, 197,918 chars
Batch 4: 10 articles, 18,283 chars
Summarized 235 articles in 4 batch(es)
```

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
base_url = "https://hub.slarker.me"  # Default instance
# Alternative instances:
#   https://rsshub.rssforever.com
#   https://rsshub.ktachibana.party
#   https://rsshub.qufy.me
```

> **Note**: The official `rsshub.app` is protected by Cloudflare and will return 403 errors. Kenseader defaults to `hub.slarker.me` which works without protection. If you experience issues, try switching to another public instance from the list above, or [deploy your own](https://docs.rsshub.app/deploy/).

Sources: [Public RSSHub Instances](https://github.com/AboutRSS/ALL-about-RSS#rsshub)

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

- **Image Preloading** - Images for nearby articles (±2) are preloaded in the background while browsing the article list, making them appear instantly when entering article detail view
- **Lazy Loading** - Only visible images are loaded first (visible-first strategy with 20-line lookahead)
- **Async I/O** - Non-blocking network and database operations
- **Memory Management** - Image cache limited to 50 entries with LRU eviction
- **Disk Cache** - Images cached at `~/.local/share/kenseader/image_cache/` for persistence across sessions
- **Resized Image Cache** - Pre-resized images are cached to avoid expensive resize operations on every frame

## Background Daemon

The daemon is the **core backend service** that handles all data operations. The TUI is a pure frontend that communicates with the daemon via Unix socket IPC.

### Starting the Daemon

```bash
# Start the background daemon
kenseader daemon start

# Check if daemon is running
kenseader daemon status

# Stop the daemon
kenseader daemon stop
```

### Daemon Output

When the daemon starts, you'll see:
```
Starting kenseader daemon...
Daemon started (PID: 12345). Press Ctrl+C or run 'kenseader daemon stop' to stop.
  Refresh interval: 300 seconds
  Cleanup interval: 3600 seconds
  Summarize interval: 60 seconds
  IPC socket: /Users/you/.local/share/kenseader/kenseader.sock
```

### Scheduled Tasks

| Task | Default Interval | Description |
|------|------------------|-------------|
| **Feed Refresh** | 1 hour (scheduler) | Smart refresh: only fetches feeds older than per-feed interval |
| **Article Cleanup** | 1 hour | Removes articles older than retention period |
| **AI Summarization** | 1 minute | Generates summaries for new articles |
| **Article Filtering** | 2 minutes | Scores articles by relevance and auto-filters low-relevance ones |
| **Style Classification** | 2 minutes | Classifies article style, tone, and length (runs with filtering) |

### Smart Feed Refresh

The scheduler uses intelligent per-feed refresh intervals to reduce unnecessary network requests:

- **Scheduler Interval** (`refresh_interval_secs`): How often the scheduler checks for feeds to refresh (default: 1 hour)
- **Per-Feed Interval** (`feed_refresh_interval_secs`): Minimum time between refreshes for each feed (default: 12 hours)

Each feed is only refreshed if its `last_fetched_at` is older than the per-feed interval. New feeds (never fetched) are refreshed immediately.

### IPC API

The daemon exposes these operations via Unix socket:

| Method | Description |
|--------|-------------|
| `ping` | Health check |
| `status` | Get daemon status and uptime |
| `feed.list` | List all feeds with unread counts |
| `feed.add` | Add a new feed subscription |
| `feed.delete` | Delete a feed |
| `feed.refresh` | Trigger feed refresh |
| `article.list` | List articles (with filters) |
| `article.get` | Get single article by ID |
| `article.mark_read` | Mark article as read |
| `article.mark_unread` | Mark article as unread |
| `article.toggle_saved` | Toggle saved/bookmark status |
| `article.search` | Search articles |

### How It Works

1. **Required for TUI** - The daemon must be running before starting the TUI
2. **Independent Process** - Daemon runs separately from TUI, continues after TUI quits
3. **Graceful Shutdown** - Use `daemon stop` or Ctrl+C to stop cleanly
4. **PID File** - Tracks running daemon at `~/.local/share/kenseader/daemon.pid`
5. **IPC Socket** - Unix socket at `~/.local/share/kenseader/kenseader.sock`
6. **Configurable Intervals** - Customize all intervals in the config file

### Configuration

```toml
[sync]
refresh_interval_secs = 3600        # Scheduler check interval (0 = disabled)
feed_refresh_interval_secs = 43200  # Per-feed interval (12 hours)
cleanup_interval_secs = 3600        # Article cleanup
summarize_interval_secs = 60        # AI summarization
filter_interval_secs = 120          # Article filtering
```

Set `refresh_interval_secs = 0` to disable the background scheduler entirely.
Set `feed_refresh_interval_secs = 0` to refresh all feeds every scheduler cycle.

## Development

### Running for Development

```bash
# Clone and build
git clone https://github.com/kenxcomp/kenseader.git
cd kenseader
cargo build

# Terminal 1: Start daemon with debug logging
RUST_LOG=debug ./target/debug/kenseader daemon start

# Terminal 2: Run TUI
./target/debug/kenseader run
```

### Running for Production

```bash
# Build release version
cargo build --release

# Start daemon (can run in background or as a service)
./target/release/kenseader daemon start &

# Run TUI
./target/release/kenseader run

# Stop daemon when done
./target/release/kenseader daemon stop
```

### Viewing Logs

```bash
# Run with specific log level
RUST_LOG=info ./target/release/kenseader daemon start

# Available levels: error, warn, info, debug, trace
RUST_LOG=debug ./target/release/kenseader daemon start

# Redirect logs to file
RUST_LOG=info ./target/release/kenseader daemon start 2> /tmp/kenseader.log
```

### Testing IPC Connection

You can test the IPC connection with a simple Python script:

```python
import socket
import json
import uuid

socket_path = "~/.local/share/kenseader/kenseader.sock"
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(socket_path)

# Send ping request
request = {"id": str(uuid.uuid4()), "method": "ping", "params": None}
sock.sendall((json.dumps(request) + "\n").encode())
print(sock.recv(4096).decode())  # {"id":"...","result":{"ok":true}}
```

## Smart Article Filtering

Kenseader includes AI-powered article filtering that automatically scores articles based on your reading interests and filters out low-relevance content.

### How It Works

1. **Interest Learning** - The system automatically tracks your reading behavior to learn your interests:
   - **Click events** - Recorded when you open an article (mark as read)
   - **Save events** - Recorded when you bookmark/save an article (high weight)
   - Tag affinities are computed from these events and used for scoring
   - Note: For new users with no history, all articles pass through (score 1.0)
2. **AI Scoring** - Articles are scored using a combination of:
   - **Profile Score (40%)** - Based on tag matching with your learned interests
   - **AI Score (60%)** - AI evaluates article relevance to your interests
3. **Auto-Filtering** - Articles below the relevance threshold are automatically marked as read (not deleted)

### Workflow

The filtering process runs in three stages:

**Stage 1: Summarization**
- Articles with 500+ characters get AI-generated summaries
- Shorter articles skip summarization

**Stage 2: Scoring & Filtering**
- Articles with summaries are scored using "title + summary"
- Short articles (< 500 chars) are scored using "title + content"
- Articles scoring below the threshold (default 0.3) are auto-filtered

**Stage 3: Style Classification**
- Summarized articles are classified by style (tutorial, news, opinion, analysis, review)
- Tone is detected (formal, casual, technical, humorous)
- Length category is assigned (short, medium, long)
- Style preferences are aggregated to learn your content style interests

### Configuration

```toml
[ai]
# Minimum content length for AI summarization (chars)
min_summarize_length = 500

# Relevance threshold (0.0 - 1.0)
# Articles scoring below this are auto-marked as read
relevance_threshold = 0.3

[sync]
# How often to run article filtering (seconds)
filter_interval_secs = 120
```

### Tips

- **Higher threshold** (0.5+) = More aggressive filtering, only highly relevant articles shown
- **Lower threshold** (0.2) = More permissive, shows most articles
- Filtered articles are marked as read, not deleted - toggle unread mode with `i` to see them

## Troubleshooting

### Images Not Displaying

1. Ensure `image_preview = true` in config
2. Check terminal supports true color: `echo $COLORTERM` should output `truecolor` or `24bit`
3. For best results on Linux, install Üeberzug++: `sudo pacman -S ueberzugpp` (Arch) or see installation guide
4. For macOS, use iTerm2, Kitty, or WezTerm for native image protocols

### Slow Image Loading

1. Images are loaded asynchronously - scroll slowly to allow loading
2. Check network connection
3. Some websites block image hotlinking

### Memory Usage

If memory usage is high with many images:
- Images are automatically evicted from cache when limit is reached
- Restart the application to clear memory cache
- Disk cache persists between sessions

## Cloud Sync (iCloud/Dropbox/etc.)

To sync your RSS data across devices (e.g., Mac + future iOS app):

1. Edit `~/.config/kenseader/config.toml`
2. Set `data_dir` to your cloud storage path:

   ```toml
   [general]
   # iCloud (macOS)
   data_dir = "~/Library/Mobile Documents/com~apple~CloudDocs/kenseader"

   # Or Dropbox
   # data_dir = "~/Dropbox/kenseader"
   ```

3. Restart the daemon: `kenseader daemon stop && kenseader daemon start`

### Features

- **Tilde Expansion**: Paths support `~` for home directory (e.g., `~/Dropbox/kenseader`)
- **Auto Migration**: When you change `data_dir`, existing data is automatically migrated to the new location
- **Conflict Detection**: If the new path already has a database file, the daemon will report an error instead of overwriting

### What Gets Synced

| Item | Synced | Notes |
|------|--------|-------|
| Database (`kenseader.db`) | Yes | Contains feeds, articles, read status, summaries |
| Image Cache (`image_cache/`) | Yes | Cached article images |
| Socket File (`kenseader.sock`) | No | Local IPC only |
| PID File (`daemon.pid`) | No | Local process tracking |

### Notes

- The config file (`~/.config/kenseader/config.toml`) is NOT synced - it stays local
- For future iOS development: The SQLite database can be read directly by iOS apps using libraries like GRDB.swift or SQLite.swift

## License

MIT
