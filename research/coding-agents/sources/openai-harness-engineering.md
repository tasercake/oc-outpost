# OpenAI: Harness Engineering — Leveraging Codex in an Agent-First World

**Source:** https://openai.com/index/harness-engineering/

## One-line summary
OpenAI built a real product with **zero manually-written code** over 5 months using Codex agents, producing ~1M lines of code across ~1,500 PRs with 3-7 engineers, and shares hard-won lessons about how engineering discipline shifts from writing code to designing environments, scaffolding, and feedback loops.

## Key architectural decisions

### Zero human-written code (hard constraint)
- Intentional constraint: no engineer writes code directly. All code — app logic, tests, CI, docs, tooling, dashboards — written by Codex.
- Humans steer through prompts, acceptance criteria, and environment design.
- Estimated **10x speed improvement** over manual coding.

### AGENTS.md as table of contents, not encyclopedia (~100 lines)
- **Critical insight**: big instruction files hurt more than help. They crowd out task context, rot quickly, and can't be mechanically verified.
- AGENTS.md is a **map** (~100 lines) pointing to structured `docs/` directory.
- **Progressive disclosure**: agents start with small stable entry point, taught where to look next.

### Repository as system of record
- All knowledge lives in-repo as versioned artifacts (markdown, schemas, code, execution plans).
- Slack discussions, Google Docs, tribal knowledge = **illegible to agents** = effectively doesn't exist.
- "If it isn't discoverable to the agent, it's illegible."
- Design docs catalogued with verification status.
- Plans treated as first-class artifacts — ephemeral for small changes, full execution plans with progress/decision logs for complex work.

### Strict architectural enforcement
- **Rigid layered architecture**: Types → Config → Repo → Service → Runtime → UI per business domain.
- Cross-cutting concerns (auth, telemetry, feature flags) enter through single explicit interface: **Providers**.
- Enforced via **custom linters** (themselves Codex-generated) and structural tests.
- Lint error messages are written to inject **remediation instructions into agent context** — the error message IS the prompt.
- "Parse, don't validate" at boundaries (Zod emerged naturally, wasn't prescribed).
- File size limits, naming conventions, structured logging — all enforced mechanically.
- Philosophy: **enforce boundaries centrally, allow autonomy locally**.

### Application legibility for agents
- App bootable per **git worktree** — Codex launches one instance per change.
- **Chrome DevTools Protocol** wired into agent runtime — DOM snapshots, screenshots, navigation.
- **Local observability stack** (ephemeral per worktree): LogQL for logs, PromQL for metrics.
- Enables prompts like "ensure startup < 800ms" or "no span in critical journeys > 2s."

### Agent-to-agent review loop
- Codex reviews its own changes locally, then requests additional agent reviews (local + cloud).
- Iterates until all agent reviewers satisfied — a "[Ralph Wiggum Loop](https://ghuntley.com/loop/)."
- Humans may review but **aren't required to**.
- Over time, almost all review pushed to agent-to-agent.

### Entropy management ("garbage collection")
- Initially: team spent **every Friday (20% of week) cleaning up "AI slop."** Didn't scale.
- Solution: **"Golden principles"** encoded in repo + recurring background Codex tasks that scan for deviations, update quality grades, open targeted refactoring PRs.
- Most cleanup PRs reviewable in <1 minute, often automerged.
- "Technical debt is like a high-interest loan — pay it down continuously."

### Merge philosophy
- **Minimal blocking merge gates**. PRs are short-lived.
- Test flakes addressed with follow-up runs, not blocking.
- "Corrections are cheap, waiting is expensive" — only viable in high-throughput environment.

## What worked
- **~1M lines of code, ~1,500 PRs, 3.5 PRs/engineer/day** — throughput increased as team grew from 3→7.
- **Single Codex runs working 6+ hours** on complex tasks (often overnight).
- Product has **daily internal users** and external alpha testers — real, shipped software.
- **"Boring" technology choices** work best — composable, stable APIs, well-represented in training data.
- **Reimplementing over importing** sometimes cheaper — e.g., own `map-with-concurrency` helper instead of `p-limit`, tightly integrated with OTel, 100% test coverage.
- **Doc-gardening agent** automatically detects stale docs and opens fix-up PRs.
- **End-to-end autonomy achieved**: single prompt → validate → reproduce bug → record video → fix → validate → record resolution video → open PR → respond to feedback → detect build failures → escalate only when judgment needed → merge.

## What failed or was hard
- **Early progress slower than expected** — not because Codex was incapable, but because the **environment was underspecified**. Agent lacked tools, abstractions, internal structure.
- **"Try harder" never works** — when something fails, the fix is always "what capability is missing?"
- **Human QA became the bottleneck** as code throughput increased — had to invest heavily in making UI/logs/metrics legible to agents.
- **20% of engineering time on cleanup** initially — "AI slop" was a real problem before they systematized it.
- **Context management** is one of the biggest challenges — too much context is as bad as too little.
- **Giant AGENTS.md files** actively harm performance — they "rot instantly" and "become an attractive nuisance."
- **Architectural drift** is inevitable — agents replicate existing patterns including bad ones.
- They **don't yet know** how architectural coherence evolves over years.

## Novel insights
1. **"Give Codex a map, not a 1,000-page manual"** — AGENTS.md should be ~100 lines, a table of contents with pointers. This contradicts the common pattern of stuffing everything into AGENTS.md.
2. **Lint error messages as agent prompts** — custom linter errors are written specifically to tell the agent how to fix the issue. The error message IS the remediation instruction.
3. **"Boring" tech is agent-friendly** — composable, stable, well-trained-on technologies are easier for agents to model. This is a concrete argument for tech conservatism.
4. **Reimplement > import** in some cases — when you need tight integration and full legibility, writing your own (via agent) can be cheaper than wrapping opaque libraries.
5. **"Garbage collection" as a recurring agent process** — not a human chore but an automated, continuous system. Quality grades tracked per domain over time.
6. **"Ralph Wiggum Loop"** — agent reviews its own work, requests agent reviews, iterates until satisfied. Humans optional.
7. **The fix is never "try harder"** — always "what capability is missing?" This is a fundamental reframe of debugging agent failures.
8. **Corrections are cheap, waiting is expensive** — in high-throughput agent environments, conventional merge gates become counterproductive.
9. **Ephemeral observability stacks per worktree** — each agent gets isolated logs/metrics that are torn down after the task. Enables performance-oriented prompts.
10. **Plans as first-class versioned artifacts** — execution plans with progress and decision logs checked into the repo, not in project management tools.

## Applicable to Tavus
- **"Map, not manual" for AGENTS.md** — directly applicable. Keep it short, point to structured docs.
- **Lint errors as agent instructions** — Tavus could write custom linters for video pipeline code that tell the agent exactly how to fix issues (e.g., "use our VideoEncoder abstraction, not raw ffmpeg calls").
- **Ephemeral environments per worktree** — critical for Tavus where video processing might need GPU resources, specific model checkpoints, etc. Each agent run gets its own isolated stack.
- **"Boring tech" preference** — for a video AI company, this means being deliberate about which ML frameworks/tools are exposed to agents vs. abstracted behind stable interfaces.
- **Quality grades per domain** — Tavus could track agent-generated code quality across video pipeline, API, frontend, ML inference separately.
- **Garbage collection process** — automated cleanup agents scanning for code drift, especially important in fast-moving ML codebases.
- **Agent-legible observability** — expose video processing metrics (latency, quality scores, resource usage) to agents via queryable interfaces.
- **Reimplement vs import** — for video-specific utilities, having agent-written, fully-tested, tightly-integrated helpers may be better than wrapping complex video libraries.
- **Progressive disclosure of codebase knowledge** — essential for large codebases with complex domain logic (video encoding, lip sync, avatar generation).

## Open questions
- **What product did they build?** Never named or described — just "internal beta with daily users." Hard to assess complexity.
- **Model costs**: 1,500 PRs with 6-hour single runs — what's the API spend?
- **What does "human review optional" actually look like?** What's the defect rate? Any production incidents from unreviewed agent PRs?
- **How do they handle security-sensitive code?** Auth, payments, PII — all agent-generated with optional review?
- **Test coverage and quality**: They mention 100% coverage on one utility — is that the norm or the exception?
- **How does this work with external dependencies?** Library upgrades, security patches, breaking API changes.
- **Team composition**: 3-7 engineers — what seniority? Could junior engineers steer effectively?
- **Reproducibility**: Can they reproduce any historical state of the codebase from the versioned plans + code?

## Related work / Links
- [Ralph Wiggum Loop](https://ghuntley.com/loop/) — agent self-review loop pattern
- [Execution Plans](https://cookbook.openai.com/articles/codex_exec_plans) — OpenAI cookbook on planning
- [ARCHITECTURE.md](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html) — matklad's architecture docs pattern
- [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/) — boundary validation philosophy
- [AI is forcing us to write good code](https://bits.logic.inc/p/ai-is-forcing-us-to-write-good-code) — strict structure for agents
- [Aardvark](https://openai.com/index/introducing-aardvark/) — OpenAI's other agent working on codebases
- [Codex](https://openai.com/codex/) — OpenAI's coding agent
