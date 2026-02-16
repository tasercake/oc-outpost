# Sourcegraph: Lessons from Building AI Coding Assistants — Context Retrieval and Evaluation

**Source:** https://sourcegraph.com/blog/lessons-from-building-ai-coding-assistants-context-retrieval-and-evaluation
**Paper:** [RecSys '24](https://arxiv.org/abs/2408.05344)

## One-line summary
Sourcegraph details their two-stage context engine (retrieval → ranking) that feeds relevant code snippets to LLMs, and the surprisingly hard problem of evaluating whether the right context was selected.

## Key architectural decisions
- **Two-stage retrieval + ranking** — industry-standard pattern (Spotify, YouTube, Facebook all use it). Retrieval optimizes for recall (cast wide net), ranking optimizes for precision (fit token budget).
- **Multiple complementary retrievers:**
  - **Keyword (Zoekt)** — trigram-based search, fast exact matches
  - **Embedding-based** — semantic/conceptual similarity via code embedding models
  - **Graph-based** — static analysis dependency graphs (callers, implementations)
  - **Local context** — editor state, open files, cursor position, git history
- **Pointwise ranking with transformer encoder** — trained to predict relevance of context item to query. Simple but effective.
- **Token budget as knapsack problem** — ranking isn't about ordering (order doesn't matter much for LLM context); it's about selecting the optimal SET of items within budget.
- **Latency SLA + token budget constraints** — hard engineering constraints that bound the system.
- **MCP for extending beyond code** — using Model Context Protocol to pull from GitHub issues, wikis, etc.

## What worked
- **Multi-retriever complementarity** — each retriever surfaces different types of relevant info. Keyword finds exact references, semantic finds conceptual matches, graph finds dependencies.
- **Zoekt for keyword search** — "blazingly fast trigram-based search engine." Speed matters when you're in the hot path.
- **Treating ranking as set selection (knapsack) rather than ordering** — insight that for LLM context, the SET matters more than the ORDER.
- **Separation of retrieval and ranking** — allows independent optimization of each stage.

## What failed or was hard
- **No ground truth for "good context"** — the fundamental evaluation challenge. What IS the right context for a query? Manual annotation is expensive and doesn't scale.
- **User feedback loop is broken** — users interact with LLM responses, not context items. If a response is bad, was it bad context or bad LLM reasoning? Hard to decompose.
- **Irrelevant context actively hurts** — not just wasted tokens; it can confuse the LLM and degrade response quality. Over-retrieval is worse than under-retrieval.
- **Embedding accuracy issues** — they moved away from embeddings for autocomplete due to accuracy problems and caching complexity.
- **Latency vs quality tradeoff** — sophisticated retrieval is better but slower. Hard constraint on retrieval time since it blocks generation.

## Novel insights
- **"Adding irrelevant context can make the response quality worse"** — this is critical. More context ≠ better. There's an optimal amount and exceeding it degrades quality.
- **Knapsack framing of context selection** — ranking for LLM context is fundamentally different from ranking for display (search results). Order matters less; set composition matters more.
- **Evaluation requires component-specific AND end-to-end** — you need to evaluate retrieval, ranking, and final response separately AND together. Each can fail independently.
- **Synthetic datasets as evaluation workaround** — when human annotation is too expensive and open-source benchmarks are low quality, synthetic data fills the gap.
- **The context engine is a "specialized search tool within the broader AI assistant architecture"** — framing context retrieval as a tool the agent uses (like a human searching docs).

## Practical Applications
- **Two-stage retrieval + ranking is the pattern** — if your team builds code search/context for their coding agent, this architecture is proven at scale.
- **Multiple retriever types** — don't rely on just embeddings or just keyword search. Complementary retrievers are key.
- **Token budget management** — critical for any LLM-based system. Treat it as a knapsack optimization problem.
- **Evaluation methodology** — the lack of ground truth for context quality will also affect any team. Consider synthetic datasets early.
- **Irrelevant context degrades quality** — when building RAG for video pipeline code, be aggressive about filtering. Less is more.
- **Latency constraints** — if building IDE integrations, retrieval must be sub-second. Design for this from the start.

## Open questions
- How well does the pointwise ranking model generalize across different codebases and languages?
- What's the optimal token budget for different types of queries (simple questions vs. architectural discussions)?
- How do you handle stale context when code changes frequently?
- What's the latency breakdown between retrieval and ranking?
- How effective is the graph-based retriever compared to keyword and embedding approaches?

## Links to related work
- [RecSys '24 paper (arXiv)](https://arxiv.org/abs/2408.05344)
- [Zoekt search engine](https://github.com/sourcegraph/zoekt/)
- [Anatomy of a coding assistant](https://sourcegraph.com/blog/anatomy-of-a-coding-assistant)
- [Model Context Protocol](https://modelcontextprotocol.io/introduction)
- [In-context learning paper](https://arxiv.org/pdf/2301.00234)
- [Cody context architecture whitepaper](https://about.sourcegraph.com/whitepaper/cody-context-architecture.pdf)
