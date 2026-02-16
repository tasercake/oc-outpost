# Effective Context Engineering for AI Agents

**Source:** https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
**Authors:** Prithvi Rajasekaran, Ethan Dixon, Carly Ryan, Jeremy Hadfield (Anthropic Applied AI)
**Date Analyzed:** 2026-02-15

## One-Line Summary

Context engineering is the evolution of prompt engineering — it's the art of curating the smallest possible set of high-signal tokens across system prompts, tools, examples, and message history to maximize agent performance within the finite "attention budget" of LLMs.

## Key Architectural Decisions

1. **Context as finite resource with diminishing returns:** n² pairwise attention relationships mean more tokens = less focus per token. Treat context like working memory, not storage.
2. **"Right altitude" for prompts:** Neither brittle if-else hardcoding nor vague high-level guidance. Find the Goldilocks zone — specific enough to guide, flexible enough to provide heuristics.
3. **Just-in-time context retrieval:** Instead of pre-loading all relevant data, maintain lightweight identifiers (file paths, queries, links) and load dynamically at runtime via tools.
4. **Hybrid retrieval strategy:** Some data retrieved up front (CLAUDE.md files), other data explored autonomously (grep, glob). Best of both worlds.
5. **Progressive disclosure:** Let agents incrementally discover context through exploration rather than dumping everything upfront.
6. **Three techniques for long-horizon tasks:** Compaction, structured note-taking, and sub-agent architectures.

## What Worked

- **Just-in-time retrieval in Claude Code:** Model writes targeted queries, stores results, uses `head`/`tail` to analyze large databases without loading full data into context. Mirrors human cognition (bookmarks, file systems, inboxes).
- **File system metadata as implicit context:** Folder hierarchies, naming conventions, timestamps provide signals without consuming explicit tokens. `test_utils.py` in `tests/` vs `src/core_logic/` conveys different meanings.
- **Compaction in Claude Code:** Summarize message history, preserve architectural decisions/unresolved bugs/implementation details, discard redundant tool outputs. Continue with compressed context + 5 most recently accessed files.
- **Tool result clearing:** Safest, lightest form of compaction — once a tool has been called deep in history, the raw result is rarely needed again.
- **Structured note-taking (agentic memory):** Agent writes NOTES.md or to-do lists persisted outside context window. Claude playing Pokémon maintained precise tallies across thousands of game steps without any prompting about memory structure.
- **Sub-agent architectures:** Specialized sub-agents explore extensively (10k+ tokens), return condensed summaries (1-2k tokens). Clear separation of concerns. "Substantial improvement" over single-agent on complex research.
- **Minimal tool sets:** Tools should be self-contained, non-overlapping, descriptive. "If a human engineer can't definitively say which tool to use, an AI agent can't be expected to do better."

## What Failed or Was Hard

- **Context rot:** As token count increases, model's ability to recall information from context decreases. Emerges across ALL models, just at different rates.
- **Bloated tool sets:** Too much functionality or ambiguous decision points about which tool to use — one of the most common failure modes.
- **Edge case stuffing:** Teams stuff laundry lists of edge cases into prompts trying to cover every rule. Counterproductive — use diverse canonical examples instead.
- **Runtime exploration tradeoff:** Just-in-time retrieval is slower than pre-computed data. Without proper guidance, agents waste context chasing dead-ends.
- **Compaction information loss:** "Overly aggressive compaction can result in the loss of subtle but critical context whose importance only becomes apparent later." The art is in what to keep vs. discard.
- **Position encoding limitations:** Models handle longer sequences via interpolation adapted to originally trained smaller context, with some degradation in token position understanding.

## Novel Insights

1. **"Attention budget" as mental model:** LLMs have a fixed budget of attention that depletes with each token. Every token has an opportunity cost. This is the most useful framing for practitioners.
2. **Context engineering is iterative, not discrete:** Unlike writing a prompt once, context curation happens every inference turn. It's a continuous optimization problem.
3. **"Do the simplest thing that works" as evergreen advice:** Given how fast models improve, over-engineering context management today will be obsolete tomorrow. Simple > clever.
4. **Minimal ≠ short:** Minimal means the smallest set that FULLY outlines expected behavior. Sometimes that's quite long.
5. **Claude Pokémon example reveals emergent memory:** Without prompting about memory structure, Claude developed its own maps, strategic notes, and combat strategy tracking. The capability exists — you just need to give it persistence tools.
6. **Pictures > words for LLMs too:** "Examples are the 'pictures' worth a thousand words" — few-shot examples are more token-efficient than exhaustive rule lists.
7. **Performance gradient, not cliff:** Models don't suddenly fail at longer contexts. They show "reduced precision for information retrieval and long-range reasoning" — a gentle degradation.

## Applicable to Tavus

- **Attention budget for video context:** When a Tavus coding agent works on the video pipeline, it can't hold the entire codebase in context. Just-in-time retrieval (grep for relevant video processing code) is essential.
- **Hybrid retrieval for domain knowledge:** Pre-load Tavus-specific conventions (like CLAUDE.md) — video format specs, API contracts, model configurations. Let the agent explore implementation details on demand.
- **Compaction for long video debugging sessions:** Video pipeline debugging can span many turns. Compacting while preserving "architectural decisions, unresolved bugs, implementation details" is directly applicable.
- **Sub-agents for parallel exploration:** One sub-agent investigates a rendering bug, another reviews audio sync, a third checks encoding. Each returns a condensed summary to the coordinator.
- **Structured note-taking for complex pipelines:** A NOTES.md tracking which video components are working, which are broken, and what was tried prevents context loss across sessions.
- **Tool design matters:** Tavus's coding agent tools (file editing, test running, video preview) should be non-overlapping, well-described, and token-efficient in their output.
- **Progressive disclosure for large codebases:** Let the agent discover the video pipeline architecture layer by layer rather than dumping entire codebases upfront.

## Open Questions

- **Optimal compaction aggressiveness:** How do you tune the keep/discard threshold? What's the empirical impact of different compaction strategies?
- **When does just-in-time beat pre-retrieval?** The article says the boundary "depends on the task" but gives limited concrete guidance.
- **Sub-agent coordination overhead:** How much context is consumed by coordination vs. actual work? At what point do diminishing returns kick in?
- **Context rot quantification:** Can you measure context rot for a specific task/model? Is there a practical diagnostic?
- **Memory tool design:** The file-based memory tool (Sonnet 4.5 launch) — what's the optimal structure for agent-written notes? How do you prevent memory from becoming stale?

## Links to Related Work

- [Building effective agents](https://www.anthropic.com/research/building-effective-agents) — predecessor post
- [Writing tools for AI agents](https://www.anthropic.com/engineering/writing-tools-for-agents) — tool design principles
- [How we built our multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) — sub-agent architecture
- [Context rot research](https://research.trychroma.com/context-rot) — Chroma's work on degradation
- [Karpathy on context engineering](https://x.com/karpathy/status/1937902205765607626) — coining the term
- [Simon Willison's agent definition](https://simonwillison.net/2025/Sep/18/agents/) — "LLMs autonomously using tools in a loop"
- [Memory and context management cookbook](https://platform.claude.com/cookbook/tool-use-memory-cookbook)
- [Model Context Protocol (MCP)](https://modelcontextprotocol.io/docs/getting-started/intro)
- [Claude playing Pokémon](https://www.twitch.tv/claudeplayspokemon) — memory in non-coding domains
