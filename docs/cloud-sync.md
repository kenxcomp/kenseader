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

## Technical Details

Kenseader uses several SQLite optimizations to enable reliable multi-device cloud sync:

### WAL Mode (Write-Ahead Logging)

The database uses WAL journal mode instead of the default DELETE mode. This provides:
- **Concurrent reads and writes**: Multiple devices can read simultaneously while one writes
- **Better performance**: Readers don't block writers and vice versa
- **Crash recovery**: Incomplete transactions are automatically rolled back

### Retry Mechanism

Write operations (marking articles read, toggling bookmarks, etc.) include automatic retry logic:
- On database busy/locked errors, operations retry up to 5 times
- Uses exponential backoff (200ms, 400ms, 800ms, 1600ms, 3200ms)
- Most concurrent access conflicts resolve within 1-2 retries

### Lock File Handling

On startup, the application checks for stale lock files (`.db-wal`, `.db-shm`) that may have been created by other devices and improperly synced. Files older than 30 seconds are automatically cleaned up.

### PRAGMA Configuration

The following SQLite settings optimize for cloud sync scenarios:
- `busy_timeout = 10000` - Wait up to 10 seconds for locks (increased for high-concurrency)
- `journal_mode = WAL` - Enable WAL mode
- `synchronous = NORMAL` - Balance between safety and performance
- `wal_autocheckpoint = 2000` - Periodic WAL checkpointing (~8MB)

### Connection Pool & Concurrency

To handle high-concurrency scenarios (daemon + TUI + cloud sync):
- **Connection pool**: 15 connections (up from 5) to handle parallel operations
- **IPC concurrency limit**: Maximum 10 concurrent requests processed simultaneously to prevent pool exhaustion
- **Batch operations**: Tags are inserted in batches to reduce database round trips

## Notes

- The config file (`~/.config/kenseader/config.toml`) is NOT synced - it stays local
- For future iOS development: The SQLite database can be read directly by iOS apps using libraries like GRDB.swift or SQLite.swift
