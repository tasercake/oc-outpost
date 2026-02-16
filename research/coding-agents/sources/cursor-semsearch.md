# Cursor: Improving Agent with Semantic Search

**Source:** https://cursor.com/blog/semsearch
**Date researched:** 2026-02-15

## One-line summary
Cursor trained a custom embedding model using agent session traces as training data, achieving 12.5% higher accuracy in codebase Q&A and measurable improvements in code retention and user satisfaction via A/B tests.

## Key architectural decisions

1. **Custom embedding model** trained specifically for code search, not off-the-shelf embeddings
2. **Training data from agent traces:** When agents work through tasks, they search and open files before finding the right code. An LLM analyzes these traces retrospectively to rank what *should* have been retrieved earlier. The embedding model is trained to align similarity scores with these rankings.
3. **Semantic search + grep combined** — not either/or. The combination produces best outcomes.
4. **Both offline evals and online A/B tests** to validate impact

## What worked

- **12.5% average accuracy improvement** on Cursor Context Bench (range: 6.5%–23.5% depending on model)
- **Code retention increase:** +0.3% overall, +2.6% on large codebases (1,000+ files)
- **2.2% reduction in dissatisfied follow-up requests** when semantic search available
- **Works across all frontier models tested** — universal improvement, not model-specific
- **Agent trace training loop:** Elegant self-improving system where agent behavior generates training data for better retrieval, which improves future agent behavior

## What failed or was hard

- Not much disclosed about failures, but implicit challenges:
  - Effect sizes in A/B tests are modest (0.3% code retention) because not all queries require search
  - Building and maintaining evaluation datasets (Cursor Context Bench) is ongoing work
  - Indexing pipelines for fast retrieval at scale need continuous engineering

## Novel insights

1. **Agent traces as training data for retrieval.** Instead of synthetic pairs or human labels, use the agent's actual search→find→use behavior as signal. The LLM retrospectively identifies what should have been surfaced earlier. This is a self-improving flywheel.

2. **Semantic search helps more on larger codebases.** 2.6% code retention improvement on 1K+ file repos vs 0.3% overall. This makes intuitive sense—grep is sufficient for small codebases where you can enumerate, but semantic search shines when the search space is vast.

3. **Code retention as a proxy metric for agent quality.** Clever: if agent-written code stays in the codebase, it was probably good. If users immediately rewrite/delete it, it wasn't. This is a lagging but very real signal.

4. **"Dissatisfied follow-up requests"** as a metric — detecting when users have to correct or re-prompt indicates the agent's first attempt was insufficient.

5. **Universal improvement across models** suggests retrieval quality is a bottleneck independent of reasoning capability. Even the best models produce better results with better context.

## Practical Applications

- **Custom embedding model for domain-specific retrieval:** Your team could train embeddings on video pipeline code patterns, making an agent better at navigating their specific codebase
- **Agent trace → training data pipeline:** If your team builds internal coding agents, logging agent sessions and mining them for retrieval training data is high-ROI
- **Semantic search for large codebases:** Essential for any codebase with 1K+ files. If your team's monorepo is large, this is table stakes.
- **Code retention as eval metric:** Simple, measurable, meaningful. Your team could use this to evaluate their coding agent's output quality.
- **Grep + semantic search combination:** Don't replace grep, augment it. Both tools serve different retrieval needs.
- **Evaluation methodology:** Both offline benchmarks (controlled) and online A/B tests (real-world) — good pattern for any AI feature.

## Open questions

- What embedding model architecture? Transformer size? Code-specific pretraining?
- What's the indexing latency? How quickly does the index update as the codebase changes?
- How does semantic search perform on non-code files (docs, configs, READMEs)?
- What's the retrieval latency vs grep? Does it add meaningful time to agent loops?
- How many traces needed to train a useful model? Cold start problem?
- Does the embedding model generalize across languages/frameworks, or is it codebase-specific?
- What chunk size / granularity is used for indexing?

## Links to related work

- [Cursor Context Bench](https://cursor.com/blog/semsearch) — their evaluation dataset (mentioned but not publicly released)
- Related: Sourcegraph's code intelligence, GitHub code search
- Related: [Sourcegraph lessons on context retrieval](https://sourcegraph.com/blog/lessons-from-building-ai-coding-assistants-context-retrieval-and-evaluation)
- Conceptually: RLHF-style training but for retrieval (using LLM judgments as reward signal)
- [Scaling Agents](https://cursor.com/blog/scaling-agents) and [Self-Driving Codebases](https://cursor.com/blog/self-driving-codebases) — the agents that benefit from this search
