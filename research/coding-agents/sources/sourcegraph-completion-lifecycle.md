# Sourcegraph: The Lifecycle of a Code AI Completion

**Source:** https://sourcegraph.com/blog/the-lifecycle-of-a-code-ai-completion

## One-line summary
Sourcegraph details the full four-stage pipeline (Planning → Retrieval → Generation → Post-processing) behind Cody's code completions, with deep technical lessons on latency optimization, Tree-sitter integration, context retrieval, and the critical importance of post-processing.

## Key architectural decisions
- **Four-stage pipeline:** Planning → Retrieval → Generation → Post-processing. Each stage independently optimizable.
- **Rule-based planning (no AI)** — planning step uses heuristics and Tree-sitter, NOT LLMs. Fast and deterministic. Like a database query planner.
- **Tree-sitter for syntax understanding** — WASM bindings for incremental parsing. Used for: detecting single vs. multi-line, categorizing syntactic context (function body, docstring, method call), truncating completions, and scoring quality.
- **Different models for different completion types** — more complex/expensive model for multi-line (user expects to wait), faster/smaller model for single-line.
- **Jaccard similarity for retrieval** — sliding window over recently-viewed files, using Jaccard similarity against lines above cursor. Simple but effective for the latency budget.
- **Moved from Claude Instant to StarCoder** — general-purpose model → use-case-specific model for better latency and fill-in-the-middle support.
- **Streaming with early termination** — don't wait for full response; terminate when completion starts generating unwanted content (e.g., second function).

## What worked
- **Tree-sitter everywhere** — used in planning (trigger detection), post-processing (truncation), and quality scoring (syntax error detection). Single tool, multiple high-value uses.
- **XML tags over markdown for Claude prompts** — significant quality improvement from following Anthropic's prompting guidance. "It pays off to read the docs!"
- **Trimming trailing whitespace from prompts** — dramatically reduced empty responses. Tiny change, massive impact.
- **"Laying words in Claude's mouth"** — pre-filling the assistant response with the prefix to guide generation. Clever prompt engineering.
- **Suggestion widget integration** — using VS Code's IntelliSense selections to steer LLM completions. "Absolutely magical."
- **Recycling prior completion requests** — if user pauses mid-typing and resumes, reuse the earlier request's result if it still applies. ~10% of requests benefit.
- **TCP connection reuse** — every hop (client → server → LLM) needed persistent connections. Different HTTP clients have different defaults (!).

## What failed or was hard
- **Fill-in-the-middle with Claude Instant** — naive implementations caused LLM to repeat suffix content instead of generating new code. Required careful XML tagging and leveraging Claude's reasoning.
- **Latency was the #1 UX problem** — 1.8s p75 for single-line was unacceptable. Required months of optimization across every pipeline stage.
- **Mentioning "comment" in prompts increased comment generation** — prompt sensitivity is extreme. Slight wording changes cause dramatic quality shifts.
- **Whitespace at end of prompt caused empty responses** — non-obvious failure mode. LLMs are sensitive to trailing whitespace.
- **Embedding-based retrieval didn't pan out for autocomplete** — accuracy issues and caching complexity led them to move away from it.
- **Irrelevant context makes quality worse** — experimentally validated. Over-retrieval actively degrades completions.
- **Synchronous BigQuery logging on hot path** — discovered during latency investigation. "Whoops!"
- **Multi-line completions: LLMs keep generating** — without truncation, LLM generates function after function after function. Requires sophisticated stop logic.
- **Parallel request strategy added latency** — requesting 3 completions and picking best → latency = max(3 requests). Reduced to single request for single-line.

## Novel insights
- **"The LLM interaction is important, but only a small piece of a much larger AI engineering system"** — the pipeline around the LLM matters as much or more than the LLM itself.
- **Planning as query planning** — analogizing to database query planners is brilliant. Divide problem space into categories, optimize each independently.
- **Latency budget allocation is an engineering art** — sub-1s total: network + retrieval + inference + post-processing. Every millisecond matters.
- **Model selection should vary by completion type** — this is underappreciated. Not every request needs the same model.
- **Prompt sensitivity is extreme** — whitespace, word choice, tag format all cause dramatic quality shifts. Treat prompts as fragile engineering artifacts.
- **Recycling prior requests** — elegant optimization that exploits typing patterns. Users often provide partial info that's sufficient.
- **LLM probabilities as quality signal** — with open models (StarCoder), you can use token probabilities to score completion confidence. Not available with Claude.
- **"If we err on the side of not showing completions, users think the product doesn't work"** — showing a mediocre completion is better than showing nothing. Users prefer imperfect to absent.

## Applicable to Tavus
- **Four-stage pipeline pattern** — directly applicable architecture for any AI completion/generation system, including video generation pipelines.
- **Rule-based planning before expensive AI** — pre-classify requests with fast heuristics before invoking LLMs. Saves cost and latency.
- **Tree-sitter equivalent for video/media** — find a "Tree-sitter" for your domain: fast, incremental, error-tolerant parsing of domain-specific structures.
- **Latency obsession** — for any real-time AI product, the latency optimization catalog here is a goldmine: streaming, connection reuse, stop words, early termination.
- **Model routing by task** — use different models for different complexity levels. Cheaper/faster for simple tasks, expensive for complex.
- **Post-processing is critical** — never trust raw LLM output. Build robust post-processing to clean, truncate, and validate.
- **"Showing nothing is worse than showing something mediocre"** — UX insight applicable to any AI product.

## Open questions
- How does the planning step evolve? Will it eventually use a small LLM for classification?
- What's the acceptance rate by language? Some languages must be harder than others.
- How do they handle multi-file completions?
- What's the cost breakdown per completion across the pipeline?
- How does StarCoder compare to newer code models (DeepSeek Coder, etc.)?

## Links to related work
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/)
- [Jaccard similarity](https://philippspiess.com/note/engineering/ml/jaccard-similarity)
- [StarCoder (HuggingFace)](https://huggingface.co/bigcode/starcoder)
- [Cody context architecture](https://about.sourcegraph.com/whitepaper/cody-context-architecture.pdf)
- [Ollama local inference](https://github.com/sourcegraph/cody/pull/905)
