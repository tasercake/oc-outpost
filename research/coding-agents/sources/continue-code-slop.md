# Continue: Fight Code Slop with Continuous AI

**Source:** https://blog.continue.dev/fight-code-slop-with-continuous-ai/

## One-line summary
Continue's "Anti-Slop" agent runs in CI on every PR to detect and fix AI-generated code quality issues (duplicated logic, bad abstractions, security vulnerabilities) using team-specific rules — essentially encoding senior engineer review into an automated agent.

## Key architectural decisions
- **Separate agent for review, not inline** — deliberately NOT trying to make the coding agent clean up after itself. A fresh agent with a new context window is "more reliable and cheaper" than asking the original agent to self-correct.
- **CI-triggered, not IDE-triggered** — runs on PR open, not during coding. This guarantees coverage regardless of dev tool or workflow.
- **Narrow focus per agent** — "looking at code through a narrow lens, one concern at a time, is more manageable." Each agent handles one quality dimension.
- **Team-owned prompts** — the slop definitions are codebase-specific (e.g., "n+1 queries with TypeORM", "duplicated methods across shared packages"). Not generic linting.
- **Single file maximum** — constrains blast radius per run.

## What worked
- **700K lines generated in 25 days** — real-world scale that makes the problem visceral. You physically cannot review that much code.
- **Separation of concerns** — coding agent generates, review agent validates. Like how compilers have separate optimization passes.
- **Codebase-specific rules** — generic linters already exist. The value is encoding tribal knowledge that only your team has.

## What failed or was hard
- Implicit: the coding agents produce "slop" at scale despite being "the smartest, most expensive agents." Quality is inherently limited.
- Context window limits mean you can't front-load all quality requirements into the coding agent prompt.
- Human review is still needed — the anti-slop agent handles "tedious, nitpicky feedback" so humans can focus on architecture. Not a replacement.

## Novel insights
- **"A fresh agent with a new context window is both more reliable and cheaper than asking your coding agent to remember to clean up after itself"** — this is a key architectural insight. Multi-pass with fresh context beats single-pass with overloaded context.
- **AI code review shifts from bug-catching to knowledge encoding** — traditional review catches bugs; AI-era review encodes team standards and tribal knowledge.
- **The review bottleneck is the new problem** — with 700K lines of generated code, the constraint has shifted entirely from writing to reviewing.
- **Signals → agents pattern** — "A PR opens, an agent reviews it. An alert fires, an agent triages it." Event-driven agent architecture.

## Practical Applications
- **Multi-pass agent architecture** — for a coding agent: don't try to make one agent do everything. Have a generation pass and separate quality/review passes.
- **Encoding team standards** — Your team could encode video pipeline quality standards, API design patterns, or security requirements as review agents.
- **CI integration pattern** — running agents in CI is immediately applicable. Could add domain-specific agents for video codec compliance, performance regression checks, etc.
- **Fresh context > stale context** — when building long-running agents, consider breaking work into passes with fresh contexts rather than one continuous session.

## Open questions
- How effective is the single-file limit? Does slop often span multiple files?
- What's the false positive rate? How often does the anti-slop agent make changes that humans revert?
- How do you handle disagreements between the coding agent and the review agent (oscillating fixes)?
- Could you run the anti-slop pass BEFORE the PR (as a pre-commit hook) to avoid the PR noise?

## Links to related work
- [Continuous AI guide](https://docs.continue.dev/guides/continuous-ai)
- [What is Continuous AI](https://blog.continue.dev/what-is-continuous-ai-a-developers-guide/)
- [Anti-Slop agent template](https://www.continue.dev/agents/share/4289f235-dec2-41c4-8505-61801346711e)
