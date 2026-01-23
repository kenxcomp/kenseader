# AI Summary and Scoring Logic

This document describes the background AI processing pipeline for article summarization, scoring, and classification.

## Architecture Overview

The background AI processing uses a **three-stage pipeline** architecture:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Stage 1:       │ ──► │  Stage 2:       │ ──► │  Stage 3:       │
│  Summarization  │     │  Scoring        │     │  Classification │
│  (60s cycle)    │     │  (120s cycle)   │     │  (after scoring)│
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## Stage 1: Article Summarization

**Trigger**: `scheduler.summarize_interval_secs` (default: 60s)

**Source**: `kenseader-core/src/scheduler/tasks.rs:108-241`

### Process Flow

1. **Query unsummarized articles**: `list_unsummarized(limit=500, min_length=500)`
   - Conditions: unread + no summary + content_text >= min_length

2. **Truncate content**: Limit to 4000 characters

3. **Create batches**: Based on `batch_char_limit` (~200k characters)

4. **Batch summarize**: Call `summarizer.batch_summarize()` → AI generates summaries

5. **Store summary**: `article_repo.update_summary()`
   - Updates: `summary` + `summary_generated_at` fields

6. **Extract tags**: `summarizer.extract_tags()` → Store in `article_tags` table

### Key Features

- **Dynamic re-check**: Before each batch, verify articles haven't been marked as read
- **Batch processing**: Process multiple articles in a single API call
- **Error recovery**: Single article failure doesn't interrupt the entire batch

## Stage 2: Scoring and Filtering

**Trigger**: `scheduler.filter_interval_secs` (default: 120s)

**Source**: `kenseader-core/src/scheduler/tasks.rs:291-449`

### Process Flow

1. **Compute user preferences**: `analyzer.compute_preferences()`
   - Based on `behavior_events` table

2. **Get top interests**: `analyzer.get_top_tags(Last30Days, limit=10)`

3. **Build candidate articles**: content = title + summary (or title + content_text for short articles)

4. **Create scoring batches**: Based on `batch_char_limit`

5. **Batch score**: `summarizer.batch_score_relevance(batch, interests)` → Returns 0.0-1.0

6. **Store score**: `article_repo.update_relevance_score()`

7. **Auto-filter**: If `score < relevance_threshold`, automatically mark as read

### User Interest Weights

The system calculates user interests based on behavior events with the following weights:

| Event Type | Weight |
|-----------|--------|
| exposure | 0.1 |
| click | 1.0 |
| read_start | 1.5 |
| read_complete | 3.0 |
| save | 5.0 |
| view_repeat | 4.0 |

**Source**: `kenseader-core/src/profile/analyzer.rs:58-67`

## Stage 3: Style Classification

**Trigger**: Immediately after Stage 2 completes

**Source**: `kenseader-core/src/scheduler/tasks.rs:489-547`

### Process Flow

1. **Get unclassified articles**: `style_repo.list_unclassified(batch_size=10)`
   - Conditions: has summary but no style classification

2. **Classify**: `summarizer.classify_style()` → Returns:
   - `style_type`: tutorial / news / opinion / analysis / review
   - `tone`: formal / casual / technical / humorous
   - `length_category`: short / medium / long

3. **Store classification**: `style_repo.upsert()`

## Core Components

| Component | File | Responsibility |
|-----------|------|----------------|
| `Summarizer` | `ai/summarizer.rs:15-127` | AI call wrapper, concurrency control |
| `AiProvider` trait | `ai/providers/mod.rs:68` | Unified provider interface |
| `OpenAiProvider` | `ai/providers/openai.rs` | OpenAI API implementation |
| `ClaudeApiProvider` | `ai/providers/claude_api.rs` | Claude API implementation |
| `GeminiApiProvider` | `ai/providers/gemini_api.rs` | Gemini API implementation |
| `ProfileAnalyzer` | `profile/analyzer.rs:7` | User interest analysis |
| `ArticleRepository` | `storage/article_repo.rs:63` | Database operations |

## Data Structures

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
    pub content: String,  // title + summary or title + content
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

## Database Schema

### articles table

| Field | Type | Description |
|-------|------|-------------|
| `summary` | TEXT | AI-generated summary |
| `summary_generated_at` | DATETIME | Summary generation timestamp |
| `relevance_score` | REAL | Relevance score (0.0-1.0) |

### article_tags table

| Field | Type | Description |
|-------|------|-------------|
| `article_id` | TEXT | Article reference |
| `tag` | TEXT | AI-extracted tag |
| `source` | TEXT | Source identifier ('ai') |

### article_styles table

| Field | Type | Description |
|-------|------|-------------|
| `article_id` | TEXT | Article reference |
| `style_type` | TEXT | Article style type |
| `tone` | TEXT | Writing tone |
| `length_category` | TEXT | Content length category |

## Configuration

### AI Settings (`config/default.toml`)

```toml
[ai]
enabled = true
provider = "claude_cli"           # or openai, gemini_api, claude_api
summary_language = "Chinese"
max_summary_tokens = 150
concurrency = 2                   # Concurrency limit
min_summarize_length = 500        # Minimum content length for summarization
relevance_threshold = 0.3         # Score threshold for auto-filtering
```

### Scheduler Settings

```toml
[sync]
summarize_interval_secs = 60      # Summarization cycle
filter_interval_secs = 120        # Scoring and filtering cycle
```

## Complete Call Chain

```
T=0s: Daemon Start
├─ Initialize Database
├─ Create Summarizer (if ai.enabled)
├─ Create SchedulerService
└─ Start periodic tasks

T=+60s: First Summarize Cycle
├─ summarize_pending_articles()
│  ├─ Query: list_unsummarized(500, min_length=500)
│  ├─ Create batches
│  ├─ FOR EACH batch:
│  │  ├─ summarizer.batch_summarize(batch)
│  │  ├─ article_repo.update_summary(id, summary)
│  │  ├─ summarizer.extract_tags(content)
│  │  └─ article_repo.add_tags(id, tags, "ai")
│  └─ Emit SchedulerEvent::ArticlesSummarized

T=+120s: First Filter Cycle
├─ score_and_filter_articles()
│  ├─ analyzer.compute_preferences()
│  ├─ analyzer.get_top_tags(Last30Days, 10)
│  ├─ article_repo.list_unread_summarized()
│  ├─ Create scoring batches
│  ├─ FOR EACH batch:
│  │  ├─ summarizer.batch_score_relevance(batch, interests)
│  │  ├─ article_repo.update_relevance_score(id, score)
│  │  └─ IF score < threshold:
│  │     └─ article_repo.mark_read(id)
│  └─ Emit SchedulerEvent::ArticlesFiltered
└─ classify_pending_articles()
   ├─ style_repo.list_unclassified(10)
   ├─ FOR EACH article:
   │  ├─ summarizer.classify_style(content)
   │  └─ style_repo.upsert(id, classification)
   └─ Emit SchedulerEvent::ArticlesClassified

T=+180s: Second Summarize Cycle
└─ [Repeat]
```

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      RSS Feed Sources                           │
└─────────────────┬──────────────────────────────────────────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │  Fetch & Parse      │ ◄─ refresh_all_feeds()
        │  (FeedFetcher)      │
        └─────────┬───────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │   SQLite Database   │
        │   (articles table)  │ ◄─ new articles
        └─────────┬───────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  Stage 1: Summarization          │
        │  - Unread articles               │
        │  - No summary                    │
        │  - content >= min_length         │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │   AI Provider (OpenAI/Claude)    │
        │   ├─ Generate summary            │
        │   └─ Extract tags                │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  Stage 2: Scoring & Filtering    │
        │  - Analyze user behavior         │
        │  - Compute preferences           │
        │  - Score relevance               │
        │  - Auto-filter low scores        │
        └─────────┬────────────────────────┘
                  │
        ┌─────────▼────────────────────────┐
        │  Stage 3: Style Classification   │
        │  - Classify style/tone/length    │
        └─────────┬────────────────────────┘
                  │
                  ▼
        ┌─────────────────────┐
        │   TUI Frontend      │
        │   (Display to user) │
        └─────────────────────┘
```

## File Reference

| Function | File Path | Key Function/Line |
|----------|-----------|-------------------|
| Daemon Start | `crates/kenseader-cli/src/commands/daemon.rs` | `start()` (L215) |
| Scheduler | `crates/kenseader-core/src/scheduler/service.rs` | `SchedulerService::run()` (L72) |
| Summarization | `crates/kenseader-core/src/scheduler/tasks.rs` | `summarize_pending_articles()` (L108) |
| Scoring | `crates/kenseader-core/src/scheduler/tasks.rs` | `score_and_filter_articles()` (L291) |
| Classification | `crates/kenseader-core/src/scheduler/tasks.rs` | `classify_pending_articles()` (L489) |
| Summarizer | `crates/kenseader-core/src/ai/summarizer.rs` | `Summarizer` (L15) |
| Provider Interface | `crates/kenseader-core/src/ai/providers/mod.rs` | `AiProvider` trait (L68) |
| Storage | `crates/kenseader-core/src/storage/article_repo.rs` | `ArticleRepository` (L63) |
| User Analysis | `crates/kenseader-core/src/profile/analyzer.rs` | `ProfileAnalyzer` (L7) |
| Configuration | `config/default.toml` | `[ai]`, `[sync]` sections |
