# 配置

配置文件位置：`~/.config/kenseader/config.toml`（所有平台通用）

> **注意**：项目目录中的 `config/default.toml` 只是模板文件。应用程序从 `~/.config/kenseader/config.toml` 读取配置。如果配置文件不存在，将使用默认值。要自定义设置，请将模板复制到正确位置：
>
> ```bash
> mkdir -p ~/.config/kenseader
> cp config/default.toml ~/.config/kenseader/config.toml
> ```

## 完整配置参考

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

## 自定义快捷键

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

## RSSHub 配置

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
