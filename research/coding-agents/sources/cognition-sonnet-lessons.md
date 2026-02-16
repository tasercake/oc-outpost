# Rebuilding Devin for Claude Sonnet 4.5: Lessons and Challenges

**Source:** https://cognition.ai/blog/devin-sonnet-4-5-lessons-and-challenges

## One-line Summary
Cognition rebuilt Devin's architecture for Sonnet 4.5, discovering that the model's context-window awareness, note-taking behavior, and parallelism required fundamental changes—not just a model swap—yielding 2x speed and 12% eval improvement.

## Key Architectural Decisions

1. **Context window deception trick**: Enable 1M token beta but cap at 200k. Model thinks it has runway, avoids "context anxiety" shortcuts. This is a hack around model behavior, not a principled solution.
2. **Dual-position prompting**: Reminders at both start AND end of prompt to prevent premature task wrap-up. Start-only wasn't enough.
3. **Kept proprietary memory management over model's native note-taking**: Sonnet 4.5 writes its own CHANGELOG.md/SUMMARY.md files, but these weren't comprehensive enough. Cognition's compaction/summarization systems outperformed the model's self-generated notes.
4. **Parallel tool execution**: Leveraged Sonnet 4.5's ability to run multiple bash commands/file reads simultaneously, but had to manage the context burn tradeoff.

## What Worked

- **Planning +18%, e2e evals +12%** — biggest leap since Sonnet 3.6 (GA model)
- **2x faster sessions** — parallelism + better model judgment
- **Model creates feedback loops**: Proactively writes/runs test scripts to verify its own work (e.g., fetching HTML of React app to check rendering)
- **Model self-verifies as it goes** — decent judgment about when to check work
- **Multi-hour sessions dramatically more reliable**

## What Failed or Was Hard

- **Context anxiety**: Model takes shortcuts when it *thinks* it's near context limit, even when it's not. "Very precise about these wrong estimates."
- **Note-taking can be counterproductive**: Model sometimes spends more tokens summarizing than solving. Shorter context → more summary tokens (inverse relationship).
- **Model doesn't know what it doesn't know**: When relying on model's own notes without Cognition's systems, performance degraded due to gaps in specific knowledge.
- **Overly creative workarounds**: When debugging, model sometimes builds elaborate scripts instead of fixing root cause (e.g., custom script instead of just killing a process on a port).
- **Parallelism burns context faster** → feeds back into context anxiety loop.

## Novel Insights

1. **Models are now context-window-aware** — this is a new axis of model behavior that agent builders must account for. The model's *belief* about remaining context changes its behavior.
2. **Context anxiety is inversely proportional to remaining window**: More cautious near limit, more parallel early on. The model appears trained to estimate output token cost of tool calls.
3. **Model's self-externalization of state** (writing files as memory) is a new paradigm from Anthropic's RL training — points toward future where agents communicate via filesystem artifacts.
4. **You can't just drop in a new model** — architectural assumptions about model behavior break across generations. Agent architectures are tightly coupled to model behavioral profiles.
5. **The model underestimates remaining tokens consistently** — systematic bias, not random error.

## Applicable to Tavus

- **Model migration is architecture work, not a config change**: When Tavus upgrades underlying models for their coding agent, they should expect to re-tune prompting, memory management, and execution strategies.
- **Context management is a first-class concern**: The 1M-beta-but-cap-at-200k trick is applicable to any agent that uses Claude models. Tavus should test for context anxiety in their agent.
- **Don't trust model's self-summarization for critical state**: Build proprietary compaction/memory systems. The model's notes are a starting point, not a replacement.
- **Feedback loops via test execution**: Having the agent write and run verification scripts is high-value. Tavus's agent should be encouraged to create self-checks.
- **Parallel tool calls**: If Tavus's agent runs multiple independent operations (e.g., reading multiple files, running multiple commands), enabling parallelism gives major speed wins but requires context budget management.

## Open Questions

- How does context anxiety scale with newer models? Will this be trained away or become more pronounced?
- Can you train a dedicated "context management model" that decides when to compact/summarize? Cognition hints at this.
- How well do model-generated notes work for subagent delegation vs. proprietary summarization?
- What's the optimal balance between parallelism and context conservation?
- Does the 1M-beta trick work with other providers/models, or is it Claude-specific?

## Links to Related Work

- [Don't Build Multi-Agents](https://cognition.ai/blog/dont-build-multi-agents) — Cognition's prior post on subagent complexity
- Anthropic's context window and RL training approaches
- Windsurf parallel tool call implementation
