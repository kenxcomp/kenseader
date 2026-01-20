# AI 提供商

Kenseader 支持多种 AI 提供商进行文章摘要。可选择 CLI 提供商（免费，使用本地 CLI 工具）或 API 提供商（需要 API 密钥）。

## CLI 提供商

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

## API 提供商

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

## 摘要语言

配置 AI 生成摘要的语言：

```toml
[ai]
summary_language = "Chinese"  # 中文（推荐中文用户使用）
# summary_language = "English"   # 英文
# summary_language = "Japanese"  # 日文
# summary_language = "Spanish"   # 西班牙文
```

## 批量摘要

后台守护进程使用智能批量摘要功能，在单个 AI 请求中处理多篇文章，最大化效率并降低 API 成本。

### 动态批处理

- **无固定文章数限制**：根据内容大小动态打包文章到批次中
- **Token 限制**：每个批次目标约 100k tokens（约 200k 字符），以优化 API 利用率
- **内容截断**：长文章自动截断为 4,000 字符，确保每批次能容纳更多文章
- **智能过滤**：每次批量请求前自动排除已读文章，避免浪费 token

### 处理流程

```
1. 获取未读且无摘要的文章（每轮最多 500 篇）
2. 对每个批次：
   a. 重新检查文章已读状态（已读则跳过）
   b. 打包文章直到达到约 100k token 限制
   c. 发送批量请求到 AI
   d. 保存摘要到数据库
3. 继续处理直到所有文章完成
```

### 效率示例

```
发现 235 篇文章需要摘要
批次 1: 72 篇文章, 197,442 字符
批次 2: 60 篇文章, 198,694 字符
批次 3: 93 篇文章, 197,918 字符
批次 4: 10 篇文章, 18,283 字符
共计 235 篇文章，4 个批次完成摘要
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
