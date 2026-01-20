# Configuration

Configuration file location: `~/.config/kenseader/config.toml` (same on all platforms)

> **Note**: The `config/default.toml` file in the project directory is just a template. The application reads configuration from `~/.config/kenseader/config.toml`. If the config file doesn't exist, default values are used. To customize settings, copy the template to the correct location:
>
> ```bash
> mkdir -p ~/.config/kenseader
> cp config/default.toml ~/.config/kenseader/config.toml
> ```

## Full Configuration Reference

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

## Customizing Keybindings

All keybindings can be customized in `config.toml` using Vim-style notation:

```toml
[keymap]
# Navigation (Colemak example)
move_down = "n"           # was: j
move_up = "e"             # was: k
focus_left = "m"          # was: h
focus_right = "i"         # was: l
next_article = "<C-n>"    # was: <C-j>
prev_article = "<C-e>"    # was: <C-k>

# Use <C-x> for Ctrl+x, <S-x> for Shift+x
# Special keys: <CR>, <Enter>, <Esc>, <Tab>, <Space>, <Left>, <Right>, <Up>, <Down>
```

See `config/default.toml` for the complete list of configurable keybindings.

## RSSHub Configuration

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
