# Cursor: Scaling Long-Running Autonomous Coding

**Source:** https://cursor.com/blog/scaling-agents
**Date researched:** 2026-02-15

## One-line summary
Cursor ran hundreds of concurrent coding agents for weeks on ambitious projects (building a browser from scratch), discovering that a planner/worker hierarchy with loose coordination beats flat self-coordination or rigid planning.

## Key architectural decisions

1. **Planner-worker hierarchy over flat coordination.** Flat peer agents with shared state files failed—agents became risk-averse, avoided hard tasks, and lock contention killed throughput. Separating planners (who create tasks) from workers (who execute them) solved this.

2. **Recursive planning.** Planners can spawn sub-planners for specific areas, making planning itself parallel. This prevents any single planner from getting tunnel vision.

3. **Judge agent for iteration control.** A judge determines whether to continue after each cycle, then the next iteration starts fresh.

4. **Removed the integrator role.** Initially built a centralized quality control/conflict resolution agent—it became a bottleneck. Workers handled conflicts themselves.

5. **Accepted imperfect commits.** Rather than requiring 100% correctness before every commit, they allowed a small error rate and let other agents fix issues naturally.

## What worked

- **Scale:** Hundreds of agents working concurrently on a single codebase for weeks, producing 1M+ LoC
- **Real projects delivered:** Browser from scratch, Solid→React migration in Cursor codebase (+266K/-193K edits), 25x video rendering speedup in Rust (merged to production)
- **Removing complexity** was often better than adding it—the integrator role, complex locking, etc.
- **Prompts matter more than harness.** "The harness and models matter, but the prompts matter more."
- GPT-5.2 significantly better than Opus 4.5 for long-running autonomous work (Opus stops early, takes shortcuts)

## What failed or was hard

- **Self-coordination via shared files:** Lock contention reduced 20 agents to throughput of 2-3. Agents forgot to release locks, held them too long, or updated files without locks.
- **Optimistic concurrency control:** Simpler but didn't fix the deeper problem of no ownership.
- **Flat hierarchy → risk aversion:** Without hierarchy, no agent took responsibility for hard problems. All made small safe changes.
- **Model-specific issues:** Opus 4.5 "tends to stop earlier and take shortcuts when convenient, yielding back control quickly"
- **Drift over long periods** required periodic fresh starts

## Novel insights

1. **Different models excel at different roles.** GPT-5.2 is a better planner than GPT-5.1-Codex despite the latter being coding-specialized. Use the best model per role, not one universal model.
2. **The right structure is in the middle.** Too little → conflicts and drift. Too much → fragility. This mirrors real engineering org design.
3. **Removing complexity > adding complexity** for multi-agent systems. Counter-intuitive when systems are failing.
4. **Prompts dominate system behavior** at scale—more than the harness architecture or model choice.
5. **Resemblance to human org structures is emergent**, not designed. The planner/worker pattern mirrors how software teams actually operate.

## Applicable to Tavus

- **Multi-agent for large migrations:** The Solid→React migration pattern directly applies to any large codebase refactor Tavus might need
- **Role-specialized models:** If building a coding agent, consider using different models for planning vs execution vs review
- **Accept imperfection, fix forward:** For internal tooling or CI-gated pipelines, allowing agents to produce imperfect code that gets iteratively fixed may be more efficient than demanding perfection
- **Prompt engineering at scale:** Investment in prompt quality has outsized returns when agents run for hours/days
- **Video rendering optimization example:** Agents achieved 25x perf improvement in Rust video rendering—directly relevant to Tavus's video processing pipeline

## Open questions

- How do you specify intent well enough for week-long autonomous runs? The blog hints this is hard but doesn't give a framework.
- What's the token cost for these runs? "Trillions of tokens" is mentioned but no cost analysis.
- How do you handle security/secret management with hundreds of concurrent agents?
- What's the failure rate? How often do week-long runs produce nothing useful?
- How does the judge agent decide when to stop vs continue?

## Links to related work

- [Self-Driving Codebases (companion post)](https://cursor.com/blog/self-driving-codebases) — deeper technical details on the harness
- [FastRender browser source](https://github.com/wilsonzlin/fastrender) — the browser built by agents
- [Java LSP](https://github.com/wilson-anysphere/indonesia), [Windows 7 emulator](https://github.com/wilsonzlin/aero), [Excel](https://github.com/wilson-anysphere/formula) — other multi-agent projects
