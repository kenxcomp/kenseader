# Kenseader

高性能终端 RSS 阅读器，支持 AI 智能摘要和富文本内容显示。

## 功能特性

- **终端界面** - 基于 [ratatui](https://github.com/ratatui/ratatui) 构建的精美 TUI
- **Vim 风格导航** - 完整的 vim 快捷键支持，高效浏览
- **AI 摘要** - 通过 Claude CLI 或 OpenAI 自动生成文章摘要
- **嵌入式图片显示** - 图片在文章正文的原始位置显示
- **富文本渲染** - 支持标题、引用、代码块、列表等样式化显示
- **协议自动检测** - 自动选择最佳图片协议（Sixel/Kitty/iTerm2/半块字符）
- **实时搜索** - 使用 `/` 搜索，`n`/`N` 导航匹配结果
- **RSSHub 支持** - 原生支持 `rsshub://` 协议，轻松订阅
- **SQLite 存储** - 快速本地数据库存储订阅源和文章
- **自动标记已读** - 查看文章时自动标记为已读

## 界面预览

```
┌─ 订阅源 ────────┬─ 文章列表 ────────────────┬─ 文章详情 ─────────────────────┐
│ > Hacker News   │ ● 构建 Rust CLI 工具      │ 构建 Rust CLI 工具             │
│   Rust 博客     │   1.75 版本新特性         │                               │
│   GitHub 趋势   │ ● 理解 async/await        │ 作者 John Doe | 2024-01-15    │
│                 │   内存安全详解            │                               │
│                 │                           │ [图片在此显示]                 │
│                 │                           │                               │
│                 │                           │ 本文介绍如何构建命令行工具... │
├─────────────────┴───────────────────────────┴───────────────────────────────┤
│ 全部 | 订阅源 | 4 篇文章 | q:退出 h/l:切换面板 j/k:移动 /:搜索              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 安装

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

## 使用方法

### 快速开始

```bash
# 订阅一个 RSS 源
kenseader subscribe --url https://hnrss.org/frontpage --name "Hacker News"

# 或使用简写
kenseader -s https://blog.rust-lang.org/feed.xml -n "Rust 博客"

# 刷新订阅源
kenseader refresh

# 启动终端界面
kenseader run
```

### 命令列表

| 命令 | 描述 |
|------|------|
| `run` | 启动终端界面 |
| `subscribe` | 订阅 RSS 源 |
| `unsubscribe` | 取消订阅 |
| `list` | 列出所有订阅 |
| `refresh` | 刷新所有订阅源 |
| `cleanup` | 清理旧文章 |

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

### 操作

| 按键 | 操作 |
|------|------|
| `Enter` | 选择 / 打开文章 |
| `b` | 在浏览器中打开文章（详情视图） |
| `s` | 切换收藏/书签 |
| `d` | 删除订阅（需确认） |
| `r` | 刷新订阅源 |
| `i` | 切换仅显示未读模式 |

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

## 配置

配置文件位置：`~/.config/kenseader/config.toml`

```toml
[general]
article_retention_days = 3  # 文章保留天数
log_level = "info"          # 日志级别

[ai]
enabled = true              # 启用 AI 摘要
provider = "claude_cli"     # 或 "openai"
# openai_api_key = "sk-..."  # OpenAI 必填
# openai_model = "gpt-4o-mini"
max_summary_tokens = 150    # 摘要最大 token 数
concurrency = 2             # 并发摘要任务数

[ui]
tick_rate_ms = 100          # 刷新率（毫秒）
show_author = true          # 显示作者
show_timestamps = true      # 显示时间戳
image_preview = true        # 图片预览

[sync]
refresh_interval_secs = 300 # 自动刷新间隔（秒）
request_timeout_secs = 30   # 请求超时（秒）
rate_limit_ms = 1000        # 请求频率限制（毫秒）

[rsshub]
base_url = "https://rsshub.app"  # RSSHub 服务地址
```

## 图片显示

Kenseader 在文章正文中嵌入式显示图片，图片出现在其原始位置。系统自动检测终端能力并选择最佳渲染方式。

### 支持的协议

| 协议 | 终端 | 质量 |
|------|------|------|
| **Kitty Graphics** | Kitty | 最高 |
| **iTerm2 Inline** | iTerm2 | 高 |
| **Sixel** | xterm, mlterm, foot, WezTerm, GNOME Terminal | 高 |
| **半块字符** | 所有支持真彩色的终端 | 中等 |

### 工作原理

1. **自动检测** - 启动时自动检测终端图形能力
2. **可见优先加载** - 优先加载视口内的图片
3. **异步下载** - 图片在后台下载，不阻塞界面
4. **双层缓存** - 内存缓存快速访问 + 磁盘缓存持久化
5. **优雅降级** - 不支持图形协议时自动回退到 Unicode 半块字符

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

### Claude CLI（默认）

使用 Claude CLI 进行摘要。需要安装并认证 [Claude CLI](https://github.com/anthropics/claude-cli)。

### OpenAI

设置 `provider = "openai"` 并提供 API 密钥：

```toml
[ai]
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"
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
base_url = "https://your-rsshub-instance.com"
```

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

- **懒加载** - 仅加载可见区域的图片
- **异步 I/O** - 非阻塞的网络和数据库操作
- **内存管理** - 图片缓存限制为 20 张
- **磁盘缓存** - 图片缓存于 `~/.cache/kenseader/image_cache/`

## 常见问题

### 图片不显示

1. 确保配置中 `image_preview = true`
2. 检查终端是否支持真彩色：`echo $COLORTERM` 应输出 `truecolor` 或 `24bit`
3. 建议使用 iTerm2、Kitty 或 WezTerm 以获得最佳效果

### 图片加载慢

1. 图片是异步加载的，滚动时请稍等片刻
2. 检查网络连接
3. 部分网站会阻止图片外链

### 内存占用高

如果图片较多导致内存占用高：
- 图片缓存达到上限时会自动清理
- 重启应用可清空内存缓存
- 磁盘缓存会在会话间保留

## 许可证

MIT
