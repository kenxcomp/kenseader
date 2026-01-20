# Keyboard Shortcuts (TUI)

## Navigation

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

## Actions

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

## Batch Selection (Yazi-style)

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

## Image & Link Navigation (Article Detail)

| Key | Action |
|-----|--------|
| `Tab` | Focus next image/link (in document order) |
| `Shift+Tab` | Focus previous image/link |
| `Enter` | Open fullscreen image viewer |
| `o` | Smart open: open focused link in browser, or focused image in external viewer |
| `b` | Smart open: open focused link in browser, or article's main URL if nothing focused |

Links in article content are displayed with blue underlined text. When focused, links are highlighted with a yellow background.

## Fullscreen Image Viewer

| Key | Action |
|-----|--------|
| `n` / `l` / `→` / `Space` | Next image |
| `p` / `h` / `←` | Previous image |
| `o` / `Enter` | Open image in external viewer |
| `q` / `Esc` | Exit fullscreen mode |

## Search

| Key | Action |
|-----|--------|
| `/` | Start forward search |
| `?` | Start backward search |
| `n` | Go to next match |
| `N` | Go to previous match |
| `Enter` | Confirm search |
| `Esc` | Cancel search |

## General

| Key | Action |
|-----|--------|
| `Esc` | Exit current mode |
| `q` | Quit application |

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
