# Kenseader

[![CI](https://github.com/kenxcomp/kenseader/actions/workflows/release.yml/badge.svg)](https://github.com/kenxcomp/kenseader/actions)
[![Release](https://img.shields.io/github/v/release/kenxcomp/kenseader)](https://github.com/kenxcomp/kenseader/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-orange)](https://github.com/kenxcomp/homebrew-tap)

高性能终端 RSS 阅读器，支持 AI 智能摘要和富文本内容显示。

![正常模式](src/normal%20mode.png)

## 快速开始

```bash
# 通过 Homebrew 安装（macOS/Linux）
brew tap kenxcomp/tap && brew install kenseader

# 启动守护进程和 TUI
brew services start kenseader
kenseader run
```

## 功能特性

- 🖥️ **终端界面** - 基于 [ratatui](https://github.com/ratatui/ratatui) 构建的精美 TUI
- ⌨️ **Vim 风格导航** - 完整的 vim 快捷键支持，高效浏览
- 🤖 **AI 摘要** - 通过 Claude、Gemini、OpenAI 自动生成文章摘要
- 🎯 **智能过滤** - 基于用户兴趣的 AI 相关性评分
- 🏷️ **风格分类** - AI 分类文章风格、语气和篇幅
- 🖼️ **嵌入式图片** - 图片在原始位置显示（Sixel/Kitty/iTerm2/半块字符）
- 🔍 **实时搜索** - `/` 搜索，`n`/`N` 导航匹配结果
- 📦 **RSSHub 支持** - 原生 `rsshub://` 协议轻松订阅
- 📋 **批量选择** - Yazi 风格，`Space` 切换选择，`v` Visual 模式
- 📚 **阅读历史** - `u` 返回，`Ctrl+r` 前进
- 🔄 **后台调度** - 自动刷新、清理和 AI 处理
- 💾 **SQLite 存储** - 快速本地数据库
- ✨ **平滑滚动** - nvim 风格的平滑滚动动画，支持可配置的缓动函数

## 界面预览

### 正常模式
![正常模式](src/normal%20mode.png)

### 仅未读模式
![仅未读模式](src/unread-only%20mode.png)

## 终端兼容性

| 终端 | macOS | Linux | Windows | 图片协议 |
|------|-------|-------|---------|----------|
| iTerm2   | ✅    | -     | -       | iTerm2 Inline  |
| Kitty    | ✅    | ✅    | -       | Kitty Graphics |
| WezTerm  | ✅    | ✅    | ✅      | iTerm2 Inline  |
| foot     | -     | ✅    | -       | Sixel          |
| 其他   | ✅    | ✅    | ✅      | 半块字符     |

<details>
<summary>📦 安装（更多选项）</summary>

### Homebrew（macOS/Linux）

```bash
# 添加 tap
brew tap kenxcomp/tap

# 安装 kenseader
brew install kenseader

# 将守护进程作为后台服务启动（推荐）
brew services start kenseader

# 或手动启动
kenseader daemon start

# 运行终端界面
kenseader run
```

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/kenxcomp/kenseader.git
cd kenseader

# 编译发布版本
cargo build --release

# 二进制文件位于 ./target/release/kenseader
```

### 依赖要求

- Rust 1.70+
- SQLite（通过 sqlx 内置）
- 支持真彩色的终端（图片显示必需）

</details>

## 架构

Kenseader 采用**客户端-服务器架构**，TUI 和守护进程作为独立进程运行：

```
┌─────────────────┐         Unix Socket         ┌─────────────────────┐
│  kenseader run  │  ◄────────────────────────► │  kenseader daemon   │
│   (纯前端 TUI)   │      JSON-RPC 协议          │   (后端服务)         │
└─────────────────┘                             └─────────────────────┘
                                                         │
                                                         ▼
                                                ┌─────────────────────┐
                                                │      SQLite DB      │
                                                └─────────────────────┘
```

- **守护进程** (`kenseader daemon start`)：处理所有后端操作 - 订阅源刷新、文章清理、AI 摘要、数据库访问
- **TUI** (`kenseader run`)：纯前端，通过 IPC 与守护进程通信
- **IPC Socket**：`~/.local/share/kenseader/kenseader.sock`（Unix socket）

## 使用方法

```bash
# 1. 订阅 RSS 源（不需要守护进程）
kenseader subscribe --url https://hnrss.org/frontpage --name "Hacker News"
kenseader -s https://blog.rust-lang.org/feed.xml -n "Rust 博客"

# 2. 启动守护进程（运行 TUI 前必须启动）
kenseader daemon start

# 3. 启动终端界面
kenseader run

# 4. 完成后停止守护进程
kenseader daemon stop
```

> **重要**：TUI 需要守护进程在运行中。如果没有先启动守护进程就运行 `kenseader run`，你会看到错误信息。

### 命令列表

| 命令 | 描述 |
|------|------|
| `run` | 启动终端界面 |
| `run --read-mode` | 以只读模式启动 TUI（直接访问数据库，无需守护进程） |
| `subscribe` | 订阅 RSS 源 |
| `unsubscribe` | 取消订阅 |
| `import` | 从 OPML 文件导入订阅 |
| `list` | 列出所有订阅 |
| `refresh` | 刷新所有订阅源 |
| `cleanup` | 清理旧文章 |
| `daemon start` | 启动后台守护进程 |
| `daemon stop` | 停止后台守护进程 |
| `daemon status` | 检查守护进程状态 |

## 快捷键

| 按键 | 操作 |
|------|------|
| `h/j/k/l` | Vim 风格导航 |
| `gg` / `G` | 跳转到顶部/底部 |
| `Enter` | 选择文章 / 打开全屏图片 |
| `b` | 在浏览器中打开 |
| `s` | 切换收藏/书签 |
| `d` | 切换已读/未读 |
| `r` | 刷新订阅源 |
| `i` | 切换仅显示未读模式 |
| `/` | 搜索 |
| `q` | 退出 |

查看[完整快捷键文档](docs/keybindings_CN.md)了解所有快捷键。

## 配置

配置文件：`~/.config/kenseader/config.toml`

```toml
[ai]
enabled = true
provider = "claude_cli"  # claude_cli, gemini_cli, openai, gemini_api, claude_api
summary_language = "Chinese"

[ui]
image_preview = true

[ui.scroll]
smooth_enabled = true        # 启用平滑滚动（默认：true）
animation_duration_ms = 150  # 动画时长（毫秒）
easing = "cubic"             # 缓动函数：none, linear, cubic, quintic, easeout

[sync]
refresh_interval_secs = 3600
```

查看[完整配置文档](docs/configuration_CN.md)了解所有选项。

## 文档

| 主题 | 描述 |
|------|------|
| [配置](docs/configuration_CN.md) | 完整配置参考、快捷键自定义、RSSHub 设置 |
| [快捷键](docs/keybindings_CN.md) | 完整快捷键参考 |
| [图片显示](docs/image-display_CN.md) | 图片协议、终端兼容性、故障排除 |
| [AI 提供商](docs/ai-providers_CN.md) | CLI/API 提供商、批量摘要、智能过滤 |
| [后台守护进程](docs/daemon_CN.md) | 定时任务、IPC API、配置 |
| [云同步](docs/cloud-sync_CN.md) | iCloud/Dropbox 同步、多设备只读模式 |

## 项目结构

```
kenseader/
├── crates/
│   ├── kenseader-cli/    # CLI 应用程序和主入口
│   ├── kenseader-core/   # 核心库（订阅源解析、存储、AI）
│   └── kenseader-tui/    # 终端 UI 组件
└── Cargo.toml            # 工作空间配置
```

## 贡献

欢迎贡献！请参阅 [CONTRIBUTING.md](CONTRIBUTING.md) 了解指南。

- 🐛 [报告 Bug](https://github.com/kenxcomp/kenseader/issues)
- 💡 [功能建议](https://github.com/kenxcomp/kenseader/issues)
- 🔧 [提交 PR](https://github.com/kenxcomp/kenseader/pulls)

## 许可证

MIT
