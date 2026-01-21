# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Kenseader is a high-performance terminal RSS reader written in Rust with AI-powered summarization and rich content display. It uses a client-server architecture where the TUI frontend communicates with a background daemon via Unix socket IPC.

## Build Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test -p kenseader-core   # Test specific crate
cargo fmt                      # Format code
cargo clippy                   # Lint checks
```

## Development Workflow

```bash
# Terminal 1: Start daemon with debug logging
RUST_LOG=debug ./target/debug/kenseader daemon start

# Terminal 2: Run TUI
./target/debug/kenseader run
```

## Architecture

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

**IPC Socket:** `~/.local/share/kenseader/kenseader.sock`

## Read-Mode Design

Read-mode (`kenseader run --read-mode`) enables multi-device usage with cloud-synced storage (iCloud, Dropbox, etc.). When the database is stored in a cloud directory, only one device needs to run the daemon for feed fetching and AI processing, while other devices can use read-mode for a lightweight frontend-only experience.

Read-mode directly accesses the SQLite database (bypassing IPC) and supports these write operations to keep read status synchronized across devices:
- Mark article as read
- Undo read status
- Visual mode batch read operations

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `kenseader-cli` | Entry point, CLI commands (`main.rs`, `commands/`) |
| `kenseader-core` | Business logic: feed parsing, storage, AI, IPC, scheduling |
| `kenseader-tui` | Terminal UI: widgets, themes, event handling, image rendering |

### kenseader-core Key Modules

- `feed/` - Feed fetching (`fetcher.rs`), parsing (`parser.rs`), OPML import
- `storage/` - SQLite database layer (`database.rs`, `*_repo.rs`)
- `ai/` - AI summarization with multiple providers (`providers/`)
- `ipc/` - Client-server communication (`client.rs`, `server.rs`, `protocol.rs`)
- `scheduler/` - Background task scheduling (`service.rs`, `tasks.rs`)
- `profile/` - User interest tracking and article filtering

### kenseader-tui Key Modules

- `app.rs` - Application state management
- `widgets/` - UI components (article_detail, article_list, subscriptions, popup, status_bar)
- `themes/` - 24 built-in color themes (catppuccin, gruvbox, dracula, nord, etc.)
- `image_renderer/` - Terminal image protocols (Kitty, Sixel, iTerm2, halfblocks)
- `rich_content.rs` - HTML parsing and inline image handling
- `keymap.rs` - Configurable vim-style keybindings

## AI Provider Integration

Supports both CLI-based and API-based AI providers in `kenseader-core/src/ai/providers/`:

- **CLI-based:** `claude_cli.rs`, `gemini_cli.rs`, `cli_base.rs` (requires CLI tool installed)
- **API-based:** `openai.rs`, `gemini_api.rs`, `claude_api.rs` (requires API key)

## Configuration

Config file: `~/.config/kenseader/config.toml`

Reference implementation: `config/default.toml`

Key sections: `[general]`, `[ai]`, `[ui]`, `[sync]`, `[rsshub]`, `[keymap]`

## Dependency Highlights

- **Async runtime:** tokio (full features)
- **TUI:** ratatui, crossterm, ratatui-image
- **Database:** sqlx (SQLite, async)
- **HTTP:** reqwest (rustls-tls, proxy support)
- **Feed parsing:** feed-rs
- **AI:** async-openai (for OpenAI API)
