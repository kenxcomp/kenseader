# AI 摘要与评分逻辑

本文档描述了文章摘要、评分和分类的后台 AI 处理流水线。

## 架构概览

后台 AI 处理采用**三阶段流水线**架构：

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  阶段 1:        │ ──► │  阶段 2:        │ ──► │  阶段 3:        │
│  文章摘要       │     │  评分过滤       │     │  风格分类       │
│  (60秒周期)     │     │  (120秒周期)    │     │  (评分后立即)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## 阶段 1：文章摘要

**触发条件**: `scheduler.summarize_interval_secs` (默认: 60秒)

**代码位置**: `kenseader-core/src/scheduler/tasks.rs:108-241`

### 处理流程

1. **查询未摘要文章**: `list_unsummarized(limit=500, min_length=500)`
   - 条件: 未读 + 无摘要 + content_text >= min_length

2. **截断内容**: 限制为 4000 字符

3. **创建批次**: 基于 `batch_char_limit` (~200k 字符)

4. **批量摘要**: 调用 `summarizer.batch_summarize()` → AI 生成摘要

5. **存储摘要**: `article_repo.update_summary()`
   - 更新字段: `summary` + `summary_generated_at`

6. **提取标签**: `summarizer.extract_tags()` → 存入 `article_tags` 表

### 关键特性

- **动态重检查**: 每个批次前验证文章是否已被标记为已读
- **批处理**: 在单个 API 调用中处理多篇文章
- **错误恢复**: 单篇文章失败不会中断整个批次

## 阶段 2：评分与过滤

**触发条件**: `scheduler.filter_interval_secs` (默认: 120秒)

**代码位置**: `kenseader-core/src/scheduler/tasks.rs:291-449`

### 处理流程

1. **计算用户偏好**: `analyzer.compute_preferences()`
   - 基于 `behavior_events` 表

2. **获取顶部兴趣**: `analyzer.get_top_tags(Last30Days, limit=10)`

3. **构建候选文章**: content = title + summary (或短文章使用 title + content_text)

4. **创建评分批次**: 基于 `batch_char_limit`

5. **批量评分**: `summarizer.batch_score_relevance(batch, interests)` → 返回 0.0-1.0

6. **存储评分**: `article_repo.update_relevance_score()`

7. **自动过滤**: 如果 `score < relevance_threshold`，自动标记为已读

### 用户兴趣权重

系统根据行为事件计算用户兴趣，权重如下：

| 事件类型 | 权重 |
|---------|------|
| exposure (曝光) | 0.1 |
| click (点击) | 1.0 |
| read_start (开始阅读) | 1.5 |
| read_complete (完成阅读) | 3.0 |
| save (保存) | 5.0 |
| view_repeat (重复查看) | 4.0 |

**代码位置**: `kenseader-core/src/profile/analyzer.rs:58-67`

## 阶段 3：风格分类

**触发条件**: 阶段 2 完成后立即执行

**代码位置**: `kenseader-core/src/scheduler/tasks.rs:489-547`

### 处理流程

1. **获取未分类文章**: `style_repo.list_unclassified(batch_size=10)`
   - 条件: 有摘要但无风格分类

2. **分类**: `summarizer.classify_style()` → 返回:
   - `style_type`: tutorial / news / opinion / analysis / review
   - `tone`: formal / casual / technical / humorous
   - `length_category`: short / medium / long

3. **存储分类**: `style_repo.upsert()`

## 核心组件

| 组件 | 文件 | 职责 |
|-----|------|-----|
| `Summarizer` | `ai/summarizer.rs:15-127` | AI 调用封装、并发控制 |
| `AiProvider` trait | `ai/providers/mod.rs:68` | 统一 Provider 接口 |
| `OpenAiProvider` | `ai/providers/openai.rs` | OpenAI API 实现 |
| `ClaudeApiProvider` | `ai/providers/claude_api.rs` | Claude API 实现 |
| `GeminiApiProvider` | `ai/providers/gemini_api.rs` | Gemini API 实现 |
| `ProfileAnalyzer` | `profile/analyzer.rs:7` | 用户兴趣分析 |
| `ArticleRepository` | `storage/article_repo.rs:63` | 数据库操作 |

## 数据结构

### ArticleForSummary

```rust
pub struct ArticleForSummary {
    pub id: String,
    pub title: String,
    pub content: String,
}
```

### ArticleForScoring

```rust
pub struct ArticleForScoring {
    pub id: String,
    pub content: String,  // title + summary 或 title + content
}
```

### ArticleStyleResult

```rust
pub struct ArticleStyleResult {
    pub style_type: String,      // tutorial|news|opinion|analysis|review
    pub tone: String,            // formal|casual|technical|humorous
    pub length_category: String, // short|medium|long
}
```

## 数据库模式

### articles 表

| 字段 | 类型 | 描述 |
|-----|------|-----|
| `summary` | TEXT | AI 生成的摘要 |
| `summary_generated_at` | DATETIME | 摘要生成时间戳 |
| `relevance_score` | REAL | 相关度评分 (0.0-1.0) |

### article_tags 表

| 字段 | 类型 | 描述 |
|-----|------|-----|
| `article_id` | TEXT | 文章引用 |
| `tag` | TEXT | AI 提取的标签 |
| `source` | TEXT | 来源标识 ('ai') |

### article_styles 表

| 字段 | 类型 | 描述 |
|-----|------|-----|
| `article_id` | TEXT | 文章引用 |
| `style_type` | TEXT | 文章风格类型 |
| `tone` | TEXT | 写作语气 |
| `length_category` | TEXT | 内容长度分类 |

## 配置说明

### AI 设置 (`config/default.toml`)

```toml
[ai]
enabled = true
provider = "claude_cli"           # 或 openai, gemini_api, claude_api
summary_language = "Chinese"
max_summary_tokens = 150
concurrency = 2                   # 并发限制
min_summarize_length = 500        # 摘要最小内容长度
relevance_threshold = 0.3         # 自动过滤的评分阈值
```

### 调度器设置

```toml
[sync]
summarize_interval_secs = 60      # 摘要周期
filter_interval_secs = 120        # 评分和过滤周期
```

## 完整调用链

```
T=0s: Daemon 启动
├─ 初始化数据库
├─ 创建 Summarizer (如果 ai.enabled)
├─ 创建 SchedulerService
└─ 启动定时任务

T=+60s: 第一个摘要周期
├─ summarize_pending_articles()
│  ├─ 查询: list_unsummarized(500, min_length=500)
│  ├─ 创建批次
│  ├─ 对每个批次:
│  │  ├─ summarizer.batch_summarize(batch)
│  │  ├─ article_repo.update_summary(id, summary)
│  │  ├─ summarizer.extract_tags(content)
│  │  └─ article_repo.add_tags(id, tags, "ai")
│  └─ 发送 SchedulerEvent::ArticlesSummarized

T=+120s: 第一个过滤周期
├─ score_and_filter_articles()
│  ├─ analyzer.compute_preferences()
│  ├─ analyzer.get_top_tags(Last30Days, 10)
│  ├─ article_repo.list_unread_summarized()
│  ├─ 创建评分批次
│  ├─ 对每个批次:
│  │  ├─ summarizer.batch_score_relevance(batch, interests)
│  │  ├─ article_repo.update_relevance_score(id, score)
│  │  └─ 如果 score < threshold:
│  │     └─ article_repo.mark_read(id)
│  └─ 发送 SchedulerEvent::ArticlesFiltered
└─ classify_pending_articles()
   ├─ style_repo.list_unclassified(10)
   ├─ 对每篇文章:
   │  ├─ summarizer.classify_style(content)
   │  └─ style_repo.upsert(id, classification)
   └─ 发送 SchedulerEvent::ArticlesClassified

T=+180s: 第二个摘要周期
└─ [循环重复]
```

## 数据流图

```
┌─────────────────────────────────────────────────────────────────┐
│                      RSS Feed 源                                │
└─────────────────┬──────────────────────────────────────────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │  获取和解析         │ ◄─ refresh_all_feeds()
        │  (FeedFetcher)      │
        └─────────┬───────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │   SQLite 数据库     │
        │   (articles 表)     │ ◄─ 新文章
        └─────────┬───────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  阶段 1: 摘要生成                │
        │  - 未读文章                      │
        │  - 无摘要                        │
        │  - content >= min_length         │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │   AI Provider (OpenAI/Claude)    │
        │   ├─ 生成摘要                    │
        │   └─ 提取标签                    │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  阶段 2: 评分与过滤              │
        │  - 分析用户行为                  │
        │  - 计算偏好                      │
        │  - 评估相关度                    │
        │  - 自动过滤低分文章              │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  阶段 3: 风格分类                │
        │  - 分类风格/语气/长度            │
        └─────────┬────────────────────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │   TUI 前端          │
        │   (展示给用户)      │
        └─────────────────────┘
```

## 文件参考

| 功能 | 文件路径 | 关键函数/行号 |
|-----|---------|-------------|
| Daemon 启动 | `crates/kenseader-cli/src/commands/daemon.rs` | `start()` (L215) |
| 调度器 | `crates/kenseader-core/src/scheduler/service.rs` | `SchedulerService::run()` (L72) |
| 摘要任务 | `crates/kenseader-core/src/scheduler/tasks.rs` | `summarize_pending_articles()` (L108) |
| 评分任务 | `crates/kenseader-core/src/scheduler/tasks.rs` | `score_and_filter_articles()` (L291) |
| 分类任务 | `crates/kenseader-core/src/scheduler/tasks.rs` | `classify_pending_articles()` (L489) |
| Summarizer | `crates/kenseader-core/src/ai/summarizer.rs` | `Summarizer` (L15) |
| Provider 接口 | `crates/kenseader-core/src/ai/providers/mod.rs` | `AiProvider` trait (L68) |
| 存储层 | `crates/kenseader-core/src/storage/article_repo.rs` | `ArticleRepository` (L63) |
| 用户分析 | `crates/kenseader-core/src/profile/analyzer.rs` | `ProfileAnalyzer` (L7) |
| 配置 | `config/default.toml` | `[ai]`, `[sync]` 部分 |
