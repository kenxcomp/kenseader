# Kenseader

高性能终端 RSS 阅读器，支持 AI 智能摘要。

## 功能特性

- **终端界面** - 基于 [ratatui](https://github.com/ratatui/ratatui) 构建的精美 TUI
- **AI 摘要** - 通过 Claude CLI 或 OpenAI 自动生成文章摘要
- **RSSHub 支持** - 原生支持 `rsshub://` 协议，轻松订阅
- **SQLite 存储** - 快速本地数据库存储订阅源和文章
- **键盘驱动** - 无需离开终端即可高效浏览

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

### 快捷键（TUI）

| 按键 | 操作 |
|------|------|
| `j` / `↓` | 向下移动 |
| `k` / `↑` | 向上移动 |
| `Enter` | 选择 / 打开 |
| `o` | 在浏览器中打开文章 |
| `r` | 刷新订阅源 |
| `s` | AI 摘要文章 |
| `q` | 退出 |

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
