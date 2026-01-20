# Cloud Sync (iCloud/Dropbox/etc.)

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

## Features

- **Tilde Expansion**: Paths support `~` for home directory (e.g., `~/Dropbox/kenseader`)
- **Auto Migration**: When you change `data_dir`, existing data is automatically migrated to the new location
- **Conflict Detection**: If the new path already has a database file, the daemon will report an error instead of overwriting

## What Gets Synced

| Item | Synced | Notes |
|------|--------|-------|
| Database (`kenseader.db`) | Yes | Contains feeds, articles, read status, summaries |
| Image Cache (`image_cache/`) | Yes | Cached article images |
| Socket File (`kenseader.sock`) | No | Local IPC only |
| PID File (`daemon.pid`) | No | Local process tracking |

## Read-Mode for Multi-Device Sync

When using cloud sync, you can run the TUI in **read-mode** to browse articles without a running daemon. This is useful when:
- You have the daemon running on one machine (e.g., desktop) but want to read on another (e.g., laptop)
- You want quick read-only access without starting the daemon
- You're using cloud sync and another device is handling feed updates

```bash
# Start TUI in read-mode (no daemon required)
kenseader run --read-mode
```

**Read-mode features:**
- Browses articles directly from the synced database
- Can toggle read/unread status (writes to database with retry on lock)
- Can save/bookmark articles
- Shows `[READ]` indicator in status bar and window title

**Read-mode limitations:**
- Cannot refresh feeds (daemon handles this)
- Cannot add/delete feed subscriptions
- Database writes may occasionally fail if another device is writing (retries automatically)

**Typical workflow:**
1. Run daemon on your main machine: `kenseader daemon start`
2. On other devices, just use read-mode: `kenseader run --read-mode`
3. Cloud sync keeps the database in sync across all devices

## Notes

- The config file (`~/.config/kenseader/config.toml`) is NOT synced - it stays local
- For future iOS development: The SQLite database can be read directly by iOS apps using libraries like GRDB.swift or SQLite.swift
