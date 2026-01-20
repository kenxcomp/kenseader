# Kenseader

高性能终端 RSS 阅读器，支持 AI 智能摘要和富文本内容显示。

## 功能特性

- **终端界面** - 基于 [ratatui](https://github.com/ratatui/ratatui) 构建的精美 TUI
- **Vim 风格导航** - 完整的 vim 快捷键支持，高效浏览
- **AI 摘要** - 通过多种 AI 提供商自动生成文章摘要（Claude、Gemini、OpenAI、Codex）
- **智能文章过滤** - 基于用户兴趣的 AI 相关性评分，自动过滤低相关性文章
- **文章风格分类** - AI 分类文章风格（教程、新闻、观点、分析、评测）、语气和篇幅
- **后台调度器** - 自动刷新订阅源、清理旧文章、生成 AI 摘要、智能过滤
- **嵌入式图片显示** - 图片在文章正文的原始位置显示
- **富文本渲染** - 支持标题、引用、代码块、列表、超链接等样式化显示
- **链接导航** - Tab 键在文章中的 URL 和图片间切换，打开链接到浏览器
- **协议自动检测** - 自动选择最佳图片协议（Sixel/Kitty/iTerm2/半块字符）
- **实时搜索** - 使用 `/` 搜索，`n`/`N` 导航匹配结果，支持高亮显示
- **批量选择** - Yazi 风格批量选择，`Space` 切换选择，`v` 进入 Visual 模式批量操作
- **阅读历史** - `u` 返回上一篇，`Ctrl+r` 前进到下一篇
- **OPML 导入** - 支持从 OPML 文件批量导入订阅，方便迁移
- **加载动画** - 刷新订阅源时显示动画指示器
- **错误提示** - 有抓取错误的订阅源以红色高亮显示
- **RSSHub 支持** - 原生支持 `rsshub://` 协议，轻松订阅
- **SQLite 存储** - 快速本地数据库存储订阅源和文章
- **自动标记已读** - 查看文章时自动标记为已读

## 界面预览

### 正常模式
![正常模式](src/normal%20mode.png)

### 仅未读模式
![仅未读模式](src/unread-only%20mode.png)

## 安装

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

### 快速开始

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

> **重要**：TUI 需要守护进程在运行中。如果没有先启动守护进程就运行 `kenseader run`，你会看到：
> ```
> Daemon is not running.
> Please start the daemon first with:
>   kenseader daemon start
> ```

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
| `daemon start` | 启动后台守护进程（自动刷新和摘要） |
| `daemon stop` | 停止后台守护进程 |
| `daemon status` | 检查守护进程状态 |

## 快捷键（TUI）

### 导航

| 按键 | 操作 |
|------|------|
| `h` / `←` | 移动到左侧面板 |
| `l` / `→` | 移动到右侧面板 |
| `j` / `↓` | 向下移动 |
| `k` / `↑` | 向上移动 |
| `gg` | 跳转到顶部（按两次 `g`） |
| `G` | 跳转到底部 |
| `Ctrl+d` | 向下滚动半页 |
| `Ctrl+u` | 向上滚动半页 |
| `Ctrl+f` | 向下滚动整页 |
| `Ctrl+b` | 向上滚动整页 |
| `Ctrl+j` | 下一篇文章（详情视图，未读模式下仅跳转未读文章） |
| `Ctrl+k` | 上一篇文章（详情视图，未读模式下仅跳转未读文章） |

### 操作

| 按键 | 操作 |
|------|------|
| `Enter` | 选择文章 / 打开全屏图片查看器（详情视图） |
| `b` | 在浏览器中打开文章（文章列表/详情视图） |
| `s` | 切换收藏/书签 |
| `d` | 切换已读/未读（文章列表） / 删除订阅（订阅源列表，需确认） |
| `r` | 刷新订阅源（异步，非阻塞） |
| `i` | 切换仅显示未读模式 |
| `u` | 返回上一篇阅读历史 |
| `Ctrl+r` | 前进到下一篇阅读历史 |

### 批量选择（Yazi 风格）

| 按键 | 操作 |
|------|------|
| `Space` | 切换选择并移动到下一项 |
| `v` | 进入 Visual 模式进行范围选择 |
| `Esc` | 退出 Visual 模式 / 清除选择 |
| `d` | 批量切换已读（文章） / 删除选中项（订阅源） |

Visual 模式技巧：
- 使用 `gg` 然后 `v` 然后 `G` 来全选所有项目
- 选中项显示 ✓ 标记和紫色背景
- 状态栏显示 `VISUAL` 模式和选中数量

### 图片和链接导航（文章详情）

| 按键 | 操作 |
|------|------|
| `Tab` | 聚焦下一个可聚焦项（图片/链接） |
| `Shift+Tab` | 聚焦上一个可聚焦项 |
| `Enter` | 打开全屏图片查看器（聚焦图片时） |
| `o` | 智能打开 - 在浏览器中打开链接，或在外部查看器中打开图片 |
| `b` | 智能打开 - 在浏览器中打开聚焦的链接，未聚焦时打开文章 URL |

### 全屏图片查看器

| 按键 | 操作 |
|------|------|
| `n` / `l` / `→` / `空格` | 下一张图片 |
| `p` / `h` / `←` | 上一张图片 |
| `o` / `Enter` | 在外部查看器中打开图片 |
| `q` / `Esc` | 退出全屏模式 |

### 搜索

| 按键 | 操作 |
|------|------|
| `/` | 开始正向搜索 |
| `?` | 开始反向搜索 |
| `n` | 跳转到下一个匹配 |
| `N` | 跳转到上一个匹配 |
| `Enter` | 确认搜索 |
| `Esc` | 取消搜索 |

### 通用

| 按键 | 操作 |
|------|------|
| `Esc` | 退出当前模式 |
| `q` | 退出程序 |

### 自定义快捷键

所有快捷键都可以在 `config.toml` 中使用 Vim 风格表示法自定义：

```toml
[keymap]
# 导航（Colemak 布局示例）
move_down = "n"           # 原: j
move_up = "e"             # 原: k
focus_left = "m"          # 原: h
focus_right = "i"         # 原: l
next_article = "<C-n>"    # 原: <C-j>
prev_article = "<C-e>"    # 原: <C-k>

# 使用 <C-x> 表示 Ctrl+x，<S-x> 表示 Shift+x
# 特殊键：<CR>, <Enter>, <Esc>, <Tab>, <Space>, <Left>, <Right>, <Up>, <Down>
```

完整的可配置快捷键列表请参见 `config/default.toml`。

## 配置

配置文件位置：`~/.config/kenseader/config.toml`（所有平台通用）

> **注意**：项目目录中的 `config/default.toml` 只是模板文件。应用程序从 `~/.config/kenseader/config.toml` 读取配置。如果配置文件不存在，将使用默认值。要自定义设置，请将模板复制到正确位置：
>
> ```bash
> mkdir -p ~/.config/kenseader
> cp config/default.toml ~/.config/kenseader/config.toml
> ```

```toml
[general]
article_retention_days = 3  # 文章保留天数
log_level = "info"          # 日志级别

[ai]
enabled = true              # 启用 AI 摘要
# 提供商选项: claude_cli, gemini_cli, codex_cli, openai, gemini_api, claude_api
provider = "claude_cli"
# 摘要语言（如 "English", "Chinese", "Japanese"）
summary_language = "Chinese"

# API 密钥（仅 API 提供商需要）
# openai_api_key = "sk-..."
# gemini_api_key = "AIza..."
# claude_api_key = "sk-ant-..."

# 模型名称
openai_model = "gpt-4o-mini"
gemini_model = "gemini-2.0-flash"
claude_model = "claude-sonnet-4-20250514"

max_summary_tokens = 150    # 摘要最大 token 数
concurrency = 2             # 并发摘要任务数

# 文章过滤设置
min_summarize_length = 500    # AI 摘要的最小字符数
max_summary_length = 150      # 摘要最大输出长度
relevance_threshold = 0.3     # 低于此分数的文章将被自动过滤（0.0-1.0）

[ui]
tick_rate_ms = 100          # 刷新率（毫秒）
show_author = true          # 显示作者
show_timestamps = true      # 显示时间戳
image_preview = true        # 图片预览

[sync]
refresh_interval_secs = 3600  # 调度器检查间隔（秒），0 = 禁用
feed_refresh_interval_secs = 43200  # 单个订阅源刷新间隔（12 小时）
cleanup_interval_secs = 3600  # 旧文章清理间隔（秒）
summarize_interval_secs = 60  # AI 摘要生成间隔（秒）
filter_interval_secs = 120    # 文章过滤间隔（秒）
request_timeout_secs = 30     # 请求超时（秒）
rate_limit_ms = 1000          # 请求频率限制（毫秒）
# proxy_url = "http://127.0.0.1:7890"  # HTTP/SOCKS5 代理

[rsshub]
base_url = "https://hub.slarker.me"  # 默认实例（rsshub.app 被 Cloudflare 保护）
# access_key = "your_access_key"  # 访问密钥（用于需要认证的实例）
```

## 图片显示

Kenseader 在文章正文中嵌入式显示图片，图片出现在其原始位置。系统自动检测终端能力并选择最佳渲染方式。

### 支持的协议

| 协议 | 终端 | 质量 | 备注 |
|------|------|------|------|
| **Üeberzug++** | X11/Wayland 终端 | 最高（原生分辨率） | Linux 推荐 |
| **Kitty Graphics** | Kitty | 最高 | 原生协议 |
| **iTerm2 Inline** | iTerm2, WezTerm | 高 | macOS 原生 |
| **Sixel** | xterm, mlterm, foot, contour | 高 | 广泛支持 |
| **半块字符** | 所有支持真彩色的终端 | 中等（使用 `▀` 字符） | 通用回退 |

### Üeberzug++（Linux 推荐）

在 Linux 上获得最佳图片质量，请安装 [Üeberzug++](https://github.com/jstkdng/ueberzugpp)：

```bash
# Arch Linux
sudo pacman -S ueberzugpp

# Fedora
sudo dnf install ueberzugpp

# Ubuntu/Debian（从源码编译或使用 AppImage）
# 参见: https://github.com/jstkdng/ueberzugpp#installation
```

Üeberzug++ 将图片渲染为原生覆盖窗口，无论终端限制如何都能提供真正的高分辨率显示。Kenseader 在 X11/Wayland 环境中会自动检测并使用它。

### 工作原理

1. **自动检测** - 启动时自动检测终端能力和环境
2. **后端选择** - 自动选择最佳可用后端：
   - X11/Wayland + Üeberzug++：原生窗口覆盖（最高质量）
   - Kitty/iTerm2/WezTerm：原生终端协议
   - 回退：Unicode 半块字符（`▀`）
3. **可见优先加载** - 优先加载视口内的图片
4. **异步下载** - 图片在后台下载，不阻塞界面
5. **双层缓存** - 内存缓存快速访问 + 磁盘缓存持久化
6. **优雅降级** - 不支持高分辨率选项时自动回退到半块字符

### 配置选项

```toml
[ui]
image_preview = true  # 设为 false 完全禁用图片
```

### 终端兼容性

推荐使用支持原生图形协议的终端以获得最佳图片质量：

- **macOS**: iTerm2, Kitty, WezTerm
- **Linux**: Kitty, foot, WezTerm, GNOME Terminal（需启用 Sixel）
- **Windows**: Windows Terminal（通过 WSL 配合 Kitty/WezTerm）

对于不支持图形协议的终端，图片使用 Unicode 半块字符（`▀`）配合真彩色渲染。这在任何支持 24 位颜色的终端中都可以工作。

## 富文本渲染

文章内容经过解析后以格式化方式显示：

| 元素 | 显示样式 |
|------|----------|
| **标题** | 加粗，按级别着色（H1: 橙色, H2: 黄色, H3+: 青色） |
| **引用** | 斜体，带 `|` 前缀 |
| **代码** | 绿色文字，深色背景 |
| **列表** | 带 `•` 符号前缀 |
| **链接** | 内联显示 |
| **图片** | 在原始位置渲染 |

## AI 提供商

Kenseader 支持多种 AI 提供商进行文章摘要。可选择 CLI 提供商（免费，使用本地 CLI 工具）或 API 提供商（需要 API 密钥）。

### CLI 提供商

CLI 提供商使用本地安装的 AI CLI 工具。不需要 API 密钥，但需要安装并认证相应的 CLI。

| 提供商 | CLI 命令 | 安装链接 |
|--------|----------|----------|
| `claude_cli`（默认） | `claude` | [Claude CLI](https://github.com/anthropics/claude-cli) |
| `gemini_cli` | `gemini` | [Gemini CLI](https://github.com/google/gemini-cli) |
| `codex_cli` | `codex` | [Codex CLI](https://github.com/openai/codex-cli) |

```toml
[ai]
provider = "claude_cli"  # 或 "gemini_cli" 或 "codex_cli"
summary_language = "Chinese"  # 中文摘要
```

### API 提供商

API 提供商直接连接 AI 服务。需要 API 密钥，但提供更好的控制和可靠性。

| 提供商 | API 服务 | 模型示例 |
|--------|----------|----------|
| `openai` | OpenAI API | gpt-4o, gpt-4o-mini |
| `gemini_api` | Google Gemini API | gemini-2.0-flash, gemini-1.5-pro |
| `claude_api` | Anthropic Claude API | claude-sonnet-4-20250514, claude-3-haiku |

```toml
[ai]
# OpenAI
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"

# 或 Gemini API
provider = "gemini_api"
gemini_api_key = "AIza-your-key-here"
gemini_model = "gemini-2.0-flash"

# 或 Claude API
provider = "claude_api"
claude_api_key = "sk-ant-your-key-here"
claude_model = "claude-sonnet-4-20250514"
```

### 摘要语言

配置 AI 生成摘要的语言：

```toml
[ai]
summary_language = "Chinese"  # 中文（推荐中文用户使用）
# summary_language = "English"   # 英文
# summary_language = "Japanese"  # 日文
# summary_language = "Spanish"   # 西班牙文
```

### 批量摘要

后台守护进程使用智能批量摘要功能，在单个 AI 请求中处理多篇文章，最大化效率并降低 API 成本。

#### 动态批处理

- **无固定文章数限制**：根据内容大小动态打包文章到批次中
- **Token 限制**：每个批次目标约 100k tokens（约 200k 字符），以优化 API 利用率
- **内容截断**：长文章自动截断为 4,000 字符，确保每批次能容纳更多文章
- **智能过滤**：每次批量请求前自动排除已读文章，避免浪费 token

#### 处理流程

```
1. 获取未读且无摘要的文章（每轮最多 500 篇）
2. 对每个批次：
   a. 重新检查文章已读状态（已读则跳过）
   b. 打包文章直到达到约 100k token 限制
   c. 发送批量请求到 AI
   d. 保存摘要到数据库
3. 继续处理直到所有文章完成
```

#### 配置选项

```toml
[ai]
# AI 摘要的最小内容长度（字符）
min_summarize_length = 500
```

#### 效率示例

```
发现 235 篇文章需要摘要
批次 1: 72 篇文章, 197,442 字符
批次 2: 60 篇文章, 198,694 字符
批次 3: 93 篇文章, 197,918 字符
批次 4: 10 篇文章, 18,283 字符
共计 235 篇文章，4 个批次完成摘要
```

## RSSHub 协议

直接订阅 RSSHub 路由：

```bash
# 以下两种方式等效：
kenseader -s rsshub://github/trending/daily -n "GitHub 趋势"
kenseader -s https://rsshub.app/github/trending/daily -n "GitHub 趋势"
```

配置自定义 RSSHub 实例：

```toml
[rsshub]
base_url = "https://hub.slarker.me"  # 默认实例
# 备选实例：
#   https://rsshub.rssforever.com
#   https://rsshub.ktachibana.party
#   https://rsshub.qufy.me
```

> **注意**：官方 `rsshub.app` 被 Cloudflare 保护，会返回 403 错误。Kenseader 默认使用 `hub.slarker.me`，无需特殊配置即可使用。如遇问题，可尝试切换到上方列出的其他公共实例，或[部署自己的实例](https://docs.rsshub.app/deploy/)。

来源：[公共 RSSHub 实例列表](https://github.com/AboutRSS/ALL-about-RSS#rsshub)

## 项目结构

```
kenseader/
├── crates/
│   ├── kenseader-cli/    # CLI 应用程序和主入口
│   ├── kenseader-core/   # 核心库（订阅源解析、存储、AI）
│   └── kenseader-tui/    # 终端 UI 组件
│       ├── app.rs        # 应用状态管理
│       ├── rich_content.rs  # HTML 解析和图片处理
│       └── widgets/      # UI 组件（文章详情、列表等）
└── Cargo.toml            # 工作空间配置
```

## 性能优化

- **图片预加载** - 浏览文章列表时，后台预加载光标前后各 2 篇文章的图片，进入文章详情时图片即时显示
- **懒加载** - 优先加载可见区域的图片（可见优先策略，带 20 行预加载边距）
- **异步 I/O** - 非阻塞的网络和数据库操作
- **内存管理** - 图片缓存限制为 50 条，使用 LRU 淘汰策略
- **磁盘缓存** - 图片缓存于 `~/.local/share/kenseader/image_cache/`，跨会话持久化
- **预调整尺寸缓存** - 缓存预调整尺寸后的图片，避免每帧重复进行昂贵的缩放操作

## 后台守护进程

守护进程是**核心后端服务**，处理所有数据操作。TUI 是纯前端，通过 Unix socket IPC 与守护进程通信。

### 启动守护进程

```bash
# 启动后台守护进程
kenseader daemon start

# 检查守护进程状态
kenseader daemon status

# 停止守护进程
kenseader daemon stop
```

### 守护进程输出

启动守护进程后，你会看到：
```
Starting kenseader daemon...
Daemon started (PID: 12345). Press Ctrl+C or run 'kenseader daemon stop' to stop.
  Refresh interval: 300 seconds
  Cleanup interval: 3600 seconds
  Summarize interval: 60 seconds
  IPC socket: /Users/you/.local/share/kenseader/kenseader.sock
```

### 定时任务

| 任务 | 默认间隔 | 描述 |
|------|----------|------|
| **订阅源刷新** | 1 小时（调度器） | 智能刷新：仅获取超过单源间隔的订阅源 |
| **旧文章清理** | 1 小时 | 删除超过保留期限的文章 |
| **AI 摘要生成** | 1 分钟 | 为新文章生成摘要 |
| **文章过滤** | 2 分钟 | 评估文章相关性并自动过滤低相关性文章 |
| **风格分类** | 2 分钟 | 分类文章风格、语气和篇幅（与过滤同时运行） |

### 智能订阅源刷新

调度器使用智能的单源刷新间隔来减少不必要的网络请求：

- **调度器间隔** (`refresh_interval_secs`)：调度器检查需要刷新的订阅源的频率（默认：1 小时）
- **单源间隔** (`feed_refresh_interval_secs`)：每个订阅源两次刷新之间的最小时间（默认：12 小时）

只有当订阅源的 `last_fetched_at` 超过单源间隔时才会刷新。新订阅（从未获取过）会立即刷新。

### IPC API

守护进程通过 Unix socket 暴露以下操作：

| 方法 | 描述 |
|------|------|
| `ping` | 健康检查 |
| `status` | 获取守护进程状态和运行时间 |
| `feed.list` | 获取所有订阅源及未读数 |
| `feed.add` | 添加新订阅源 |
| `feed.delete` | 删除订阅源 |
| `feed.refresh` | 触发订阅源刷新 |
| `article.list` | 获取文章列表（支持过滤） |
| `article.get` | 通过 ID 获取单篇文章 |
| `article.mark_read` | 标记文章为已读 |
| `article.mark_unread` | 标记文章为未读 |
| `article.toggle_saved` | 切换收藏/书签状态 |
| `article.search` | 搜索文章 |

### 工作原理

1. **TUI 必需** - 启动 TUI 前必须先运行守护进程
2. **独立进程** - 守护进程与 TUI 分离运行，退出 TUI 后继续运行
3. **优雅退出** - 使用 `daemon stop` 或 Ctrl+C 正常停止
4. **PID 文件** - 守护进程 PID 保存在 `~/.local/share/kenseader/daemon.pid`
5. **IPC Socket** - Unix socket 位于 `~/.local/share/kenseader/kenseader.sock`
6. **可配置间隔** - 所有间隔都可在配置文件中自定义

### 配置选项

```toml
[sync]
refresh_interval_secs = 3600        # 调度器检查间隔（0 = 禁用）
feed_refresh_interval_secs = 43200  # 单源刷新间隔（12 小时）
cleanup_interval_secs = 3600        # 旧文章清理间隔
summarize_interval_secs = 60        # AI 摘要生成间隔
filter_interval_secs = 120          # 文章过滤间隔
```

设置 `refresh_interval_secs = 0` 可完全禁用后台调度器。
设置 `feed_refresh_interval_secs = 0` 则每次调度器运行时刷新所有订阅源。

## 开发

### 开发环境运行

```bash
# 克隆并编译
git clone https://github.com/kenxcomp/kenseader.git
cd kenseader
cargo build

# 终端 1：启动守护进程并开启调试日志
RUST_LOG=debug ./target/debug/kenseader daemon start

# 终端 2：运行 TUI
./target/debug/kenseader run
```

### 生产环境运行

```bash
# 编译发布版本
cargo build --release

# 启动守护进程（可在后台运行或作为服务）
./target/release/kenseader daemon start &

# 运行 TUI
./target/release/kenseader run

# 完成后停止守护进程
./target/release/kenseader daemon stop
```

### 查看日志

```bash
# 以指定日志级别运行
RUST_LOG=info ./target/release/kenseader daemon start

# 可用级别：error, warn, info, debug, trace
RUST_LOG=debug ./target/release/kenseader daemon start

# 重定向日志到文件
RUST_LOG=info ./target/release/kenseader daemon start 2> /tmp/kenseader.log
```

### 测试 IPC 连接

可以用简单的 Python 脚本测试 IPC 连接：

```python
import socket
import json
import uuid

socket_path = "~/.local/share/kenseader/kenseader.sock"
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(socket_path)

# 发送 ping 请求
request = {"id": str(uuid.uuid4()), "method": "ping", "params": None}
sock.sendall((json.dumps(request) + "\n").encode())
print(sock.recv(4096).decode())  # {"id":"...","result":{"ok":true}}
```

## 智能文章过滤

Kenseader 包含 AI 驱动的文章过滤功能，根据您的阅读兴趣自动评估文章相关性并过滤低相关性内容。

### 工作原理

1. **兴趣学习** - 系统自动追踪您的阅读行为来学习您的兴趣：
   - **点击事件** - 当您打开文章时记录（标记为已读）
   - **收藏事件** - 当您收藏/保存文章时记录（高权重）
   - 基于这些事件计算标签偏好并用于评分
   - 注意：对于没有历史记录的新用户，所有文章都会通过（评分 1.0）
2. **AI 评分** - 文章通过以下两种方式综合评分：
   - **用户画像评分（40%）** - 基于标签与您学习到的兴趣的匹配度
   - **AI 评分（60%）** - AI 评估文章与您兴趣的相关性
3. **自动过滤** - 低于相关性阈值的文章会被自动标记为已读（不会删除）

### 工作流程

过滤过程分三个阶段进行：

**阶段 1：摘要生成**
- 500 字符以上的文章会生成 AI 摘要
- 较短的文章跳过摘要生成

**阶段 2：评分与过滤**
- 有摘要的文章使用「标题 + 摘要」进行评分
- 短文章（< 500 字符）使用「标题 + 正文」进行评分
- 评分低于阈值（默认 0.3）的文章会被自动过滤

**阶段 3：风格分类**
- 已生成摘要的文章会进行风格分类（教程、新闻、观点、分析、评测）
- 检测文章语气（正式、随意、技术、幽默）
- 分配篇幅类别（短、中、长）
- 聚合风格偏好以学习您的内容风格兴趣

### 配置选项

```toml
[ai]
# AI 摘要的最小内容长度（字符）
min_summarize_length = 500

# 相关性阈值（0.0 - 1.0）
# 低于此分数的文章会被自动标记为已读
relevance_threshold = 0.3

[sync]
# 文章过滤运行间隔（秒）
filter_interval_secs = 120
```

### 使用建议

- **较高阈值**（0.5+）= 更激进的过滤，只显示高度相关的文章
- **较低阈值**（0.2）= 更宽松的过滤，显示大部分文章
- 被过滤的文章只是标记为已读，并非删除 - 按 `i` 键切换未读模式即可查看

## 常见问题

### 图片不显示

1. 确保配置中 `image_preview = true`
2. 检查终端是否支持真彩色：`echo $COLORTERM` 应输出 `truecolor` 或 `24bit`
3. Linux 用户建议安装 Üeberzug++：`sudo pacman -S ueberzugpp`（Arch）或参见安装指南
4. macOS 用户建议使用 iTerm2、Kitty 或 WezTerm 以获得原生协议支持

### 图片加载慢

1. 图片是异步加载的，滚动时请稍等片刻
2. 检查网络连接
3. 部分网站会阻止图片外链

### 内存占用高

如果图片较多导致内存占用高：
- 图片缓存达到上限时会自动清理
- 重启应用可清空内存缓存
- 磁盘缓存会在会话间保留

## 云同步（iCloud/Dropbox 等）

将 RSS 数据同步到多设备（如 Mac + 未来的 iOS 应用）：

1. 编辑 `~/.config/kenseader/config.toml`
2. 设置 `data_dir` 为云存储路径：

   ```toml
   [general]
   # iCloud (macOS)
   data_dir = "~/Library/Mobile Documents/com~apple~CloudDocs/kenseader"

   # 或 Dropbox
   # data_dir = "~/Dropbox/kenseader"
   ```

3. 重启守护进程：`kenseader daemon stop && kenseader daemon start`

### 功能特性

- **波浪线展开**：路径支持 `~` 表示用户主目录（如 `~/Dropbox/kenseader`）
- **自动迁移**：修改 `data_dir` 时，现有数据会自动迁移到新位置
- **冲突检测**：如果新路径已存在数据库文件，守护进程会报错而非覆盖

### 同步内容

| 项目 | 是否同步 | 备注 |
|------|----------|------|
| 数据库 (`kenseader.db`) | 是 | 包含订阅源、文章、阅读状态、摘要等 |
| 图片缓存 (`image_cache/`) | 是 | 缓存的文章图片 |
| Socket 文件 (`kenseader.sock`) | 否 | 仅用于本地 IPC |
| PID 文件 (`daemon.pid`) | 否 | 本地进程跟踪 |

### 多设备同步的只读模式

使用云同步时，可以在**只读模式**下运行 TUI 来浏览文章，无需运行守护进程。适用场景：
- 在一台设备（如台式机）上运行守护进程，在另一台设备（如笔记本）上阅读
- 快速只读访问，无需启动守护进程
- 使用云同步时，另一台设备负责更新订阅源

```bash
# 以只读模式启动 TUI（无需守护进程）
kenseader run --read-mode
```

**只读模式功能：**
- 直接从同步的数据库浏览文章
- 可切换已读/未读状态（数据库锁定时自动重试写入）
- 可收藏/书签文章
- 状态栏和窗口标题显示 `[READ]` 指示器

**只读模式限制：**
- 无法刷新订阅源（由守护进程处理）
- 无法添加/删除订阅
- 数据库写入可能偶尔失败（如另一台设备正在写入时），会自动重试

**典型工作流：**
1. 在主设备上运行守护进程：`kenseader daemon start`
2. 在其他设备上使用只读模式：`kenseader run --read-mode`

### 注意事项

- 配置文件（`~/.config/kenseader/config.toml`）不会被同步，保持本地独立
- 未来 iOS 开发：SQLite 数据库可以直接被 iOS 应用读取（使用 GRDB.swift 或 SQLite.swift 等库）

## 许可证

MIT
