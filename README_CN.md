# Kenseader

高性能终端 RSS 阅读器，支持 AI 智能摘要。

## 功能特性

- **终端界面** - 基于 [ratatui](https://github.com/ratatui/ratatui) 构建的精美 TUI
- **Vim 风格导航** - 完整的 vim 快捷键支持，高效浏览
- **AI 摘要** - 通过 Claude CLI 或 OpenAI 自动生成文章摘要
- **图片预览** - 终端内嵌图片显示（支持 Sixel/Kitty/iTerm2）
- **快速搜索** - 使用 `/` 搜索，`n`/`N` 导航匹配结果
- **RSSHub 支持** - 原生支持 `rsshub://` 协议，轻松订阅
- **SQLite 存储** - 快速本地数据库存储订阅源和文章
- **自动标记已读** - 查看文章时自动标记为已读

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
- 支持 Sixel/Kitty/iTerm2 的终端（可选，用于图片预览）

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

### 图片预览

Kenseader 支持在终端内显示图片，需要终端支持以下图形协议：

- **Sixel** - xterm, mlterm, foot 等
- **Kitty** - Kitty 终端
- **iTerm2** - macOS 上的 iTerm2

启用/禁用图片预览：

```toml
[ui]
image_preview = true  # 设为 false 禁用
```

图片会自动从文章内容中提取，并显示在文章详情视图的顶部。

### AI 提供商

#### Claude CLI（默认）

使用 Claude CLI 进行摘要。需要安装并认证 [Claude CLI](https://github.com/anthropics/claude-cli)。

#### OpenAI

设置 `provider = "openai"` 并提供 API 密钥：

```toml
[ai]
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"
```

### RSSHub 协议

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
│   ├── kenseader-cli/    # CLI 应用程序
│   ├── kenseader-core/   # 核心库（订阅源、存储、AI）
│   └── kenseader-tui/    # 终端 UI 组件
└── Cargo.toml            # 工作空间配置
```

## 许可证

MIT
