# Effective Harnesses for Long-Running Agents

**Source:** https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents
**Author:** Justin Young (Anthropic)
**Date Analyzed:** 2026-02-15

## One-Line Summary

A two-agent architecture (initializer + coding agent) solves the core problem of long-running agents: maintaining coherent progress across multiple context windows by using structured progress files, incremental feature work, and git-based state management.

## Key Architectural Decisions

1. **Two-agent split:** Separate "initializer agent" (first session only) from "coding agent" (every subsequent session). Same harness/tools, different user prompts.
2. **Structured feature list in JSON:** 200+ features marked pass/fail. JSON chosen over Markdown because the model is less likely to inappropriately modify JSON files.
3. **Progress file (`claude-progress.txt`):** Human-readable log of what agents have done, read at session start.
4. **Git as state management:** Descriptive commits enable reversion to working states. Git history + progress file = fast orientation for fresh context windows.
5. **`init.sh` script:** Written by initializer agent so coding agents don't waste tokens figuring out how to run the app.
6. **One feature at a time:** Explicit incremental approach—each session works on exactly one feature.
7. **End-of-session cleanup:** Code must be merge-ready (no bugs, documented, committed) before session ends.

## What Worked

- **Incremental feature work** was "critical" — eliminated the one-shotting failure mode entirely.
- **JSON feature lists with strongly-worded protection** ("It is unacceptable to remove or edit tests") prevented the model from gaming its own progress tracker.
- **Browser automation testing (Puppeteer MCP)** dramatically improved feature quality — agent caught bugs invisible from code alone.
- **Git-based recovery** — agents could revert bad changes and recover working states.
- **Boot sequence** (pwd → read progress → read features → git log → start dev server → basic test → work on next feature) saved tokens and caught broken states early.
- **"Clean state" discipline** — inspired by what effective human engineers do (leave code merge-ready).

## What Failed or Was Hard

- **One-shotting:** Without structure, even Opus 4.5 would try to build the entire app at once, run out of context mid-implementation, and leave undocumented half-finished work.
- **Premature victory declaration:** Later agent instances would see existing progress and declare the project complete.
- **Compaction alone insufficient:** Even with context compaction, instructions weren't passed clearly enough between sessions.
- **False test completion:** Claude would mark features done without proper end-to-end testing (e.g., only unit tests, not user-perspective testing).
- **Vision limitations:** Claude can't see browser-native alert modals through Puppeteer MCP, leading to buggier features relying on modals.
- **Context window boundaries:** Each new session starts with zero memory — the "shift worker with amnesia" problem.

## Novel Insights

1. **The shift-worker metaphor is precise:** The problem isn't intelligence—it's continuity. The solution mirrors real engineering practices (handoff notes, clean commits, documentation).
2. **JSON > Markdown for structured agent state:** Models are more disciplined with JSON, less likely to casually edit/delete structured data.
3. **Strongly-worded instructions matter for guardrails:** "Unacceptable" language prevents the model from taking shortcuts with its own tracking files.
4. **Testing as a human user would** is the key prompt for end-to-end verification. Without this explicit instruction, agents default to developer-perspective testing (unit tests, curl) which misses integration bugs.
5. **The initializer agent is doing prompt engineering for future agents** — it's essentially writing the context that will guide all subsequent work. Meta-prompting.
6. **Git is an underappreciated agent tool** — not just version control, but a recovery mechanism and orientation system.

## Practical Applications

- **Video generation pipeline as feature list:** Break complex video generation tasks into discrete, testable features (avatar rendering, lip sync, voice synthesis, scene transitions). Each feature gets pass/fail status.
- **Session handoff for long renders:** If a coding agent is building/debugging your team's video pipeline, the progress file + git pattern ensures continuity across sessions.
- **Browser automation for visual testing:** your team's output is visual — using browser automation or screenshot comparison to verify video output quality mirrors the Puppeteer testing approach.
- **Init script pattern:** For your team's likely complex dev environment (GPU servers, model servers, video processing pipelines), having an agent-written init script prevents wasted context on environment setup.
- **Incremental approach for complex systems:** Video AI has many interdependent components — the "one feature at a time, leave it clean" discipline would prevent cascading failures.
- **JSON feature tracking:** For a coding agent helping your engineers, maintaining a JSON-based task tracker prevents the agent from drifting or declaring premature victory.

## Open Questions

- **Multi-agent vs. single agent:** Article acknowledges this is unresolved — would specialized testing/QA/cleanup agents outperform the general-purpose coding agent?
- **Generalization beyond web apps:** Demo was optimized for full-stack web dev. How does this translate to ML/video pipelines?
- **Compaction quality:** How much information is lost during compaction? What's the fidelity ceiling?
- **Feature prioritization:** The agent picks "highest-priority feature" — who defines priority? How does the initializer agent sequence 200+ features?
- **Scale limits:** How many context windows can this sustain before progress stalls or quality degrades?
- **Cost:** No cost data provided. What's the token cost for the claude.ai clone demo?

## Links to Related Work

- [Claude Agent SDK](https://platform.claude.com/docs/en/agent-sdk/overview)
- [Claude 4 Prompting Guide — Multi-Context Window Workflows](https://docs.claude.com/en/docs/build-with-claude/prompt-engineering/claude-4-best-practices#multi-context-window-workflows)
- [Quickstart code examples](https://github.com/anthropics/claude-quickstarts/tree/main/autonomous-coding)
- [Building effective agents](https://www.anthropic.com/research/building-effective-agents) (predecessor post)
- [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (companion post)
