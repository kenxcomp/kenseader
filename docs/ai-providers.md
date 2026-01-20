# AI Providers

Kenseader supports multiple AI providers for article summarization. Choose between CLI-based providers (free, uses local CLI tools) or API-based providers (requires API key).

## CLI-Based Providers

CLI providers use locally installed AI CLI tools. They don't require API keys but need the respective CLI to be installed and authenticated.

| Provider | CLI Command | Installation |
|----------|-------------|--------------|
| `claude_cli` (Default) | `claude` | [Claude CLI](https://github.com/anthropics/claude-cli) |
| `gemini_cli` | `gemini` | [Gemini CLI](https://github.com/google/gemini-cli) |
| `codex_cli` | `codex` | [Codex CLI](https://github.com/openai/codex-cli) |

```toml
[ai]
provider = "claude_cli"  # or "gemini_cli" or "codex_cli"
summary_language = "Chinese"  # Summaries in Chinese
```

## API-Based Providers

API providers connect directly to AI services. They require an API key but offer more control and reliability.

| Provider | API Service | Model Examples |
|----------|-------------|----------------|
| `openai` | OpenAI API | gpt-4o, gpt-4o-mini |
| `gemini_api` | Google Gemini API | gemini-2.0-flash, gemini-1.5-pro |
| `claude_api` | Anthropic Claude API | claude-sonnet-4-20250514, claude-3-haiku |

```toml
[ai]
# OpenAI
provider = "openai"
openai_api_key = "sk-your-key-here"
openai_model = "gpt-4o-mini"

# Or Gemini API
provider = "gemini_api"
gemini_api_key = "AIza-your-key-here"
gemini_model = "gemini-2.0-flash"

# Or Claude API
provider = "claude_api"
claude_api_key = "sk-ant-your-key-here"
claude_model = "claude-sonnet-4-20250514"
```

## Summary Language

Configure the language for AI-generated summaries:

```toml
[ai]
summary_language = "English"   # Default
# summary_language = "Chinese"
# summary_language = "Japanese"
# summary_language = "Spanish"
# summary_language = "French"
```

## Batch Summarization

The background daemon uses intelligent batch summarization to process multiple articles in a single AI request, maximizing efficiency and reducing API costs.

### Dynamic Batch Processing

- **No Fixed Article Limit**: Articles are packed into batches dynamically based on content size
- **Token Limit**: Each batch targets ~100k tokens (~200k characters) for optimal API utilization
- **Content Truncation**: Long articles are automatically truncated to 4,000 characters to ensure more articles fit per batch
- **Smart Filtering**: Already-read articles are automatically excluded before each batch request to avoid wasting tokens

### Processing Flow

```
1. Fetch unread articles without summaries (up to 500 per cycle)
2. For each batch:
   a. Re-check article read status (skip if marked read)
   b. Pack articles until reaching ~100k token limit
   c. Send batch request to AI
   d. Save summaries to database
3. Continue until all articles are processed
```

### Efficiency Example

```
Found 235 articles to summarize
Batch 1: 72 articles, 197,442 chars
Batch 2: 60 articles, 198,694 chars
Batch 3: 93 articles, 197,918 chars
Batch 4: 10 articles, 18,283 chars
Summarized 235 articles in 4 batch(es)
```

## Smart Article Filtering

Kenseader includes AI-powered article filtering that automatically scores articles based on your reading interests and filters out low-relevance content.

### How It Works

1. **Interest Learning** - The system automatically tracks your reading behavior to learn your interests:
   - **Click events** - Recorded when you open an article (mark as read)
   - **Save events** - Recorded when you bookmark/save an article (high weight)
   - Tag affinities are computed from these events and used for scoring
   - Note: For new users with no history, all articles pass through (score 1.0)
2. **AI Scoring** - Articles are scored using a combination of:
   - **Profile Score (40%)** - Based on tag matching with your learned interests
   - **AI Score (60%)** - AI evaluates article relevance to your interests
3. **Auto-Filtering** - Articles below the relevance threshold are automatically marked as read (not deleted)

### Workflow

The filtering process runs in three stages:

**Stage 1: Summarization**
- Articles with 500+ characters get AI-generated summaries
- Shorter articles skip summarization

**Stage 2: Scoring & Filtering**
- Articles with summaries are scored using "title + summary"
- Short articles (< 500 chars) are scored using "title + content"
- Articles scoring below the threshold (default 0.3) are auto-filtered

**Stage 3: Style Classification**
- Summarized articles are classified by style (tutorial, news, opinion, analysis, review)
- Tone is detected (formal, casual, technical, humorous)
- Length category is assigned (short, medium, long)
- Style preferences are aggregated to learn your content style interests

### Configuration

```toml
[ai]
# Minimum content length for AI summarization (chars)
min_summarize_length = 500

# Relevance threshold (0.0 - 1.0)
# Articles scoring below this are auto-marked as read
relevance_threshold = 0.3

[sync]
# How often to run article filtering (seconds)
filter_interval_secs = 120
```

### Tips

- **Higher threshold** (0.5+) = More aggressive filtering, only highly relevant articles shown
- **Lower threshold** (0.2) = More permissive, shows most articles
- Filtered articles are marked as read, not deleted - toggle unread mode with `i` to see them
