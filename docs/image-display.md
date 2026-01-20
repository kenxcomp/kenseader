# Image Display

Kenseader displays images inline within article content, at their original positions. The system automatically detects your terminal's capabilities and selects the best rendering method.

## Supported Protocols

| Protocol | Terminals | Quality | Note |
|----------|-----------|---------|------|
| **Ueberzug++** | X11/Wayland terminals | Highest (native resolution) | Recommended for Linux |
| **Kitty Graphics** | Kitty | Highest | Native protocol |
| **iTerm2 Inline** | iTerm2, WezTerm | High | macOS native |
| **Sixel** | xterm, mlterm, foot, contour | High | Wide support |
| **Halfblocks** | All terminals with true color | Medium (uses `▀` characters) | Universal fallback |

## Ueberzug++ (Recommended for Linux)

For the best image quality on Linux, install [Ueberzug++](https://github.com/jstkdng/ueberzugpp):

```bash
# Arch Linux
sudo pacman -S ueberzugpp

# Fedora
sudo dnf install ueberzugpp

# Ubuntu/Debian (from source or AppImage)
# See: https://github.com/jstkdng/ueberzugpp#installation
```

Ueberzug++ renders images as native overlay windows, providing true high-resolution display regardless of terminal limitations. Kenseader automatically detects and uses it when available in X11/Wayland environments.

## How It Works

1. **Auto-Detection** - Terminal capabilities and environment are detected on startup
2. **Backend Selection** - The best available backend is chosen automatically:
   - On X11/Wayland with Ueberzug++: Native window overlay (highest quality)
   - On Kitty/iTerm2/WezTerm: Native terminal protocols
   - Fallback: Unicode halfblock characters (`▀`)
3. **Visible-First Loading** - Only images in the viewport are loaded first
4. **Async Download** - Images are downloaded in the background without blocking UI
5. **Dual Cache** - Memory cache for fast access + disk cache for persistence
6. **Graceful Fallback** - Degrades to halfblocks if no high-resolution option is available

## Fullscreen Image Viewer

When viewing an article, press `Enter` to open the fullscreen image viewer:

- **High-resolution display** - Uses the full terminal window for maximum image clarity
- **Navigate between images** - Use `n`/`p` or arrow keys to switch images
- **Open externally** - Press `o` to open the image in your system's default image viewer
- **Works on any terminal** - Even with halfblocks, fullscreen mode provides better resolution than inline display

## Configuration

```toml
[ui]
image_preview = true  # Set to false to disable images entirely
```

## Terminal Compatibility

For the best image quality, use a terminal with native graphics support:

- **macOS**: iTerm2, Kitty, WezTerm
- **Linux**: Kitty, foot, WezTerm, GNOME Terminal (with Sixel enabled)
- **Windows**: Windows Terminal (via WSL with Kitty/WezTerm)

For terminals without graphics support, images are rendered using Unicode halfblock characters (`▀`) with true colors. This works in any terminal supporting 24-bit color.

## Terminal Compatibility Matrix

| Terminal | macOS | Linux | Windows | Image Protocol |
|----------|-------|-------|---------|----------------|
| iTerm2   | ✅    | -     | -       | iTerm2 Inline  |
| Kitty    | ✅    | ✅    | -       | Kitty Graphics |
| WezTerm  | ✅    | ✅    | ✅      | iTerm2 Inline  |
| foot     | -     | ✅    | -       | Sixel          |
| GNOME Terminal | - | ✅  | -       | Sixel (if enabled) |
| Windows Terminal | - | - | ✅     | Halfblocks     |
| Others   | ✅    | ✅    | ✅      | Halfblocks     |

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

## Troubleshooting

### Images Not Displaying

1. Ensure `image_preview = true` in config
2. Check terminal supports true color: `echo $COLORTERM` should output `truecolor` or `24bit`
3. For best results on Linux, install Ueberzug++: `sudo pacman -S ueberzugpp` (Arch) or see installation guide
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
