# Replit Snapshot Engine
> Source: https://blog.replit.com/inside-replits-snapshot-engine
> Date: Dec 17, 2025

## One-line summary
Replit built a Copy-on-Write block-device storage layer that enables instant filesystem forks, versioned databases, and isolated sandboxes—making AI agent actions fully reversible.

## Key architectural decisions

1. **Bottomless Storage via Network Block Device (NBD)**: Virtual block devices backed by Google Cloud Storage, lazily loaded and cached on co-located storage servers. Each device split into immutable 16 MiB chunks stored in GCS; a manifest maps chunks to a version.
2. **Copy-on-Write at block level**: Copying a disk = copying the manifest (constant-time, regardless of disk size). Two copies are fully independent and can diverge.
3. **Git for code versioning, storage-level backup for git itself**: Agent commits at "doneness" checkpoints. Git object graph is recoverable from prior storage versions. An append-only git remote on a separate volume provides belt-and-suspenders durability.
4. **Forkable PostgreSQL databases**: Unmodified Postgres running on a Bottomless-backed filesystem. Checkpoint = manifest copy. Restore = manifest swap. Dev/prod split so Agent only touches dev DB.
5. **Holistic checkpoints**: Each checkpoint = git commit + database manifest + agent state. Rollback restores all three atomically.

## What worked

- **Repurposed existing infra for AI safety**: Bottomless Storage was built for developer collaboration/remixing; it turned out to be perfect for agent sandboxing. Massive leverage from pre-existing investment.
- **Constant-time fork regardless of size**: The CoW manifest approach means spinning up parallel agent explorations is cheap.
- **Dev/prod DB split**: Simple but effective guardrail—Agent can't destroy production data.
- **Git is "in-distribution"**: LLMs already know git well, so the Agent can reason about history, diffs, and even restore refactored-away code on its own.

## What failed or was hard

- Not explicitly discussed, but implied challenges:
  - **Git state corruption by Agent**: They had to build recovery mechanisms because the Agent could corrupt git's own object graph.
  - **Coordinating checkpoint atomicity** across code + database + agent state is non-trivial.
  - The system is deeply tied to Replit's custom infrastructure—not portable.

## Novel insights

1. **"Transactional compute"**: The idea that an agent's entire exploration (code + DB + side effects) can be treated as a database transaction—commit or rollback atomically.
2. **Relaxed guardrails in sandboxes**: Because forks are disposable, you can let the agent do risky things (add log statements, install tools, mutate DB for debugging) that would be dangerous in the main environment. Then cherry-pick only the insight.
3. **Parallel sampling with infrastructure support**: Using LLM non-determinism + cheap forks to run N agents on the same problem simultaneously, pick the best result. They cite 72→80% on SWE-bench with this technique.
4. **Storage infra designed for humans also benefits agents**: The same primitives that make collaboration fast (remix/fork) make agent safety cheap.

## Practical Applications

- **Video pipeline state is analogous to database state**: If your team's coding agent modifies video processing configs, model weights, or pipeline parameters, having reversible snapshots prevents catastrophic changes.
- **Dev/prod isolation pattern**: Critical for any AI agent that touches production systems. Your team should ensure their agent operates in sandboxed environments with forkable state.
- **Parallel exploration**: Could run multiple agent attempts at solving a code task and pick the best—especially valuable for complex video processing optimizations.
- **Checkpoint = code + state**: If your team's agent modifies both code and system configuration, checkpoints need to capture both atomically.

## Open questions

- How do they handle long-running processes (e.g., a server) during snapshot/restore? Do they snapshot while Postgres is running (crash-consistent?) or do they quiesce first?
- What's the latency of a restore operation in practice?
- How do they select the "best" result from parallel agent runs? Automated evaluation or human review?
- Does the dev/prod DB split cause issues when agent-built features depend on production data patterns?

## Links to related work

- [Replit Bottomless Storage (2023)](https://blog.replit.com/replit-storage-the-next-generation)
- [Safe Vibe Coding](https://blog.replit.com/safe-vibe-coding)
- [Parallel Sampling / Inference-Time Scaling](https://arxiv.org/html/2504.00294v1)
- [Copy-on-Write (Wikipedia)](https://en.wikipedia.org/wiki/Copy-on-write)
- [Network Block Device (Wikipedia)](https://en.wikipedia.org/wiki/Network_block_device)
- [Claude 4 parallel sampling results](https://www.anthropic.com/news/claude-4)
