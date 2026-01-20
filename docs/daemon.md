# Background Daemon

The daemon is the **core backend service** that handles all data operations. The TUI is a pure frontend that communicates with the daemon via Unix socket IPC.

## Starting the Daemon

```bash
# Start the background daemon
kenseader daemon start

# Check if daemon is running
kenseader daemon status

# Stop the daemon
kenseader daemon stop
```

## Daemon Output

When the daemon starts, you'll see:
```
Starting kenseader daemon...
Daemon started (PID: 12345). Press Ctrl+C or run 'kenseader daemon stop' to stop.
  Refresh interval: 300 seconds
  Cleanup interval: 3600 seconds
  Summarize interval: 60 seconds
  IPC socket: /Users/you/.local/share/kenseader/kenseader.sock
```

## Scheduled Tasks

| Task | Default Interval | Description |
|------|------------------|-------------|
| **Feed Refresh** | 1 hour (scheduler) | Smart refresh: only fetches feeds older than per-feed interval |
| **Article Cleanup** | 1 hour | Removes articles older than retention period |
| **AI Summarization** | 1 minute | Generates summaries for new articles |
| **Article Filtering** | 2 minutes | Scores articles by relevance and auto-filters low-relevance ones |
| **Style Classification** | 2 minutes | Classifies article style, tone, and length (runs with filtering) |

## Smart Feed Refresh

The scheduler uses intelligent per-feed refresh intervals to reduce unnecessary network requests:

- **Scheduler Interval** (`refresh_interval_secs`): How often the scheduler checks for feeds to refresh (default: 1 hour)
- **Per-Feed Interval** (`feed_refresh_interval_secs`): Minimum time between refreshes for each feed (default: 12 hours)

Each feed is only refreshed if its `last_fetched_at` is older than the per-feed interval. New feeds (never fetched) are refreshed immediately.

## IPC API

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

## How It Works

1. **Required for TUI** - The daemon must be running before starting the TUI
2. **Independent Process** - Daemon runs separately from TUI, continues after TUI quits
3. **Graceful Shutdown** - Use `daemon stop` or Ctrl+C to stop cleanly
4. **PID File** - Tracks running daemon at `~/.local/share/kenseader/daemon.pid`
5. **IPC Socket** - Unix socket at `~/.local/share/kenseader/kenseader.sock`
6. **Configurable Intervals** - Customize all intervals in the config file

## Configuration

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

## Testing IPC Connection

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
