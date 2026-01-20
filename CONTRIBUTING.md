# Contributing to Kenseader

Thank you for your interest in contributing to Kenseader! We welcome contributions of all kinds.

## Ways to Contribute

- 🐛 **Report Bugs** - Found a bug? [Open an issue](https://github.com/kenxcomp/kenseader/issues)
- 💡 **Request Features** - Have an idea? [Open an issue](https://github.com/kenxcomp/kenseader/issues)
- 🔧 **Submit Code** - Fix bugs or add features via [Pull Requests](https://github.com/kenxcomp/kenseader/pulls)
- 📖 **Improve Docs** - Help improve documentation
- 🌍 **Translations** - Help translate to other languages

## Development Setup

### Prerequisites

- Rust 1.70+
- SQLite (bundled via sqlx)
- Terminal with true color support

### Building from Source

```bash
# Clone the repository
git clone https://github.com/kenxcomp/kenseader.git
cd kenseader

# Build debug version
cargo build

# Build release version
cargo build --release
```

### Running for Development

```bash
# Terminal 1: Start daemon with debug logging
RUST_LOG=debug ./target/debug/kenseader daemon start

# Terminal 2: Run TUI
./target/debug/kenseader run
```

### Running Tests

```bash
cargo test
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

## Code Style

- Follow Rust conventions and idioms
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Write clear commit messages

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** for your changes (`git checkout -b feature/my-feature`)
3. **Make your changes** and commit them
4. **Test** your changes thoroughly
5. **Submit a PR** with a clear description of your changes

### PR Guidelines

- Keep changes focused and atomic
- Include tests for new functionality
- Update documentation if needed
- Make sure all tests pass

## Commit Messages

- Use clear, descriptive commit messages
- Start with a verb (Add, Fix, Update, Remove, etc.)
- Reference issues when applicable (e.g., "Fix #123: ...")

Examples:
```
Add RSSHub protocol support
Fix image loading in Kitty terminal
Update documentation for AI providers
```

## Reporting Issues

When reporting bugs, please include:

- Kenseader version (`kenseader --version`)
- Operating system and version
- Terminal emulator and version
- Steps to reproduce the issue
- Expected vs actual behavior
- Relevant logs or error messages

## Questions?

If you have questions, feel free to:
- Open a [GitHub Discussion](https://github.com/kenxcomp/kenseader/discussions)
- Open an [issue](https://github.com/kenxcomp/kenseader/issues)

Thank you for contributing! 🎉
