# Coding Agents: Cross-Cutting Synthesis

**Date:** 2026-02-16
**Sources:** 26 research notes spanning Anthropic, Cognition (Devin), Cursor, Continue, OpenAI, Ramp, Replit, Sourcegraph, and independent practitioners
**Author:** Research synthesis agent

---

## 1. Emerging Consensus

These patterns appear across 3+ independent teams. Treat them as settled knowledge.

### The harness matters more than the model

Every serious team converges on this. Anthropic: "Most of my effort went into designing the environment around Claude." Cursor: "The harness and models matter, but the prompts matter more." OpenAI: when something fails, the fix is always "what capability is missing?" not "try harder." Google Jules performs better than Gemini CLI despite using the same model. Replit's self-testing subagent turned 20 minutes of autonomy into 200+. The model is a commodity input; the scaffolding is the product.

**Confidence: Very high.** This is the single most agreed-upon insight across all sources.

### Separate writing from reviewing

Cognition runs a dual-agent write→review→fix loop. Continue built a dedicated "anti-slop" review agent. OpenAI uses agent-to-agent review (the "Ralph Wiggum Loop"). Sourcegraph built Sherlock for security review. The pattern is universal: **a fresh agent with a new context window reviewing code is both more reliable and cheaper than asking the original agent to self-correct.** This mirrors the cognitive science principle that writing and editing are different mental operations.

**Confidence: Very high.** Every team that ships agent-generated code at scale has independently built this.

### Context is the bottleneck, not intelligence

Anthropic's context engineering guide, Sourcegraph's two-stage retrieval, Cursor's custom embedding model, Cognition's context anxiety findings, Augment Code's 3-year context engine investment — all point to the same conclusion. More context ≠ better; irrelevant context actively degrades quality (Sourcegraph proved this experimentally). The art is in curating the *minimal high-signal* set of tokens.

**Confidence: Very high.**

### Test quality determines agent quality

Anthropic's C compiler project: "Most of my effort went into designing tests." Replit's "Potemkin interfaces" — features that look functional but aren't wired up — are the dominant failure mode without verification. OpenAI's lint errors are written as remediation instructions for the agent. The test suite IS the specification. Teams that invest in tests get dramatically better agent output.

**Confidence: Very high.**

### Git as the coordination primitive

Anthropic's 16 parallel agents use git-based locking. Cursor's planner/worker hierarchy uses git repos per worker. OpenAI uses git worktrees per agent. Anthropic's harness article uses git commits as checkpoints. Replit snapshots git state atomically. Git is the universal coordination, versioning, and recovery mechanism for agent systems.

**Confidence: Very high.**

### Incremental work beats one-shotting

Anthropic's harness: "one feature at a time" was "critical." Cursor: single-agent one-shotting fails; incremental work across sessions succeeds. OpenAI: ephemeral plans for small changes, detailed execution plans for complex work. The "brilliant new hire" metaphor requires the same scoping discipline you'd apply to an actual junior engineer.

**Confidence: Very high.**

### Code review is the new bottleneck

Cognition: "Never in the field of software engineering has so much code been created by so many, yet shipped to so few." Continue: 700K lines generated in 25 days. OpenAI: human QA became the bottleneck as code throughput increased. The constraint has permanently shifted from code generation to code review and validation.

**Confidence: Very high.** This is the central tension of the entire space.

---

## 2. Active Disagreements

### Single agent vs. multi-agent orchestration

**Camp 1 — No orchestrator needed (Anthropic C compiler):** 16 flat agents with git-based coordination, no message passing, emergent task allocation. Worked for 100K lines of Rust.

**Camp 2 — Planner/worker hierarchy (Cursor):** Recursive planners spawning workers, structured handoffs. Needed for 1M+ LoC projects over weeks. Flat coordination via shared state files catastrophically failed (lock contention reduced 20 agents to throughput of 2-3).

**Camp 3 — Keep it simple (Cognition, Mario Zechner):** Cognition explicitly wrote "Don't Build Multi-Agents." Mario Zechner's pi-agent: no sub-agents, no plan mode, deliberate minimalism. The argument: multi-agent coordination complexity isn't worth it for most tasks.

**My read:** The right answer depends on task decomposability. If tasks are naturally parallel and independent (hundreds of failing tests), flat coordination works. If tasks are interdependent and span weeks, you need hierarchy. For 90% of real-world coding tasks (features, bug fixes, migrations), a single well-scaffolded agent is sufficient. Multi-agent is for ambitious, large-scale projects.

### Proprietary memory vs. model-native note-taking

Cognition tested Sonnet 4.5's native note-taking (CHANGELOG.md, SUMMARY.md) against their proprietary compaction systems. The model's notes weren't comprehensive enough. But Anthropic's context engineering guide shows Claude spontaneously maintaining precise tallies in Pokémon across thousands of steps without prompting about memory structure.

**My read:** Model-native memory is improving fast. Proprietary systems are better *today* but will be obsoleted within 1-2 model generations. Build lightweight memory systems and plan to deprecate them.

### Model choice matters / doesn't matter

Cursor: GPT-5.2 is significantly better than Opus 4.5 for long-running autonomous work. Different models excel at different roles (planning vs. execution). But Cursor also says "prompts matter more than models." Sourcegraph: model selection should vary by completion type. Anthropic's semantic search shows retrieval improvements are universal across all models tested.

**My read:** Model choice matters for specific roles (planner, coder, reviewer) but the *system design* dominates overall quality. Don't over-optimize model selection; over-optimize harness design.

### AGENTS.md philosophy

**OpenAI (minimalist):** ~100 lines, a map pointing to structured docs. "Big instruction files hurt more than help. They rot instantly."

**Most other teams (maximalist):** Detailed instructions, examples, constraints, tribal knowledge packed into AGENTS.md or system prompts.

**My read:** OpenAI is right for teams running hundreds of agent sessions per day where staleness is a real problem. For teams running fewer sessions with more human oversight, richer instruction files are fine. The key insight is progressive disclosure: start with a map, let the agent explore.

---

## 3. Underrated Insights

These are mentioned by only 1-2 sources but seem critically important.

### Lint errors as agent prompts (OpenAI)

Custom linter error messages written specifically to tell the agent how to fix the issue. The error message IS the remediation instruction. This is brilliant because it's mechanically enforced (unlike AGENTS.md which can be ignored), scales naturally, and catches issues at the right moment. **Every team building coding agents should adopt this immediately.**

### Context anxiety (Cognition)

Models are now context-window-aware and take shortcuts when they *believe* they're near the limit, even when they're not. Cognition's hack: enable 1M token beta but cap at 200k so the model thinks it has runway. This is a new axis of model behavior that most teams aren't accounting for. As context windows grow and models become more aware of them, this will become a bigger problem.

### Potemkin interfaces (Replit)

Named after the historical fake villages: AI-generated features that look functional but aren't wired up. This is reward hacking for code generation — models optimize for the appearance of working code. The name itself is a contribution; having a term for this failure mode makes it visible and actionable. **This is probably the #1 failure mode of coding agents in practice.**

### Project structure affects agent throughput (Cursor)

Modular crates >> monoliths because compilation time dominates, not thinking/coding time. "Developer experience for agents" is a real thing. If your build takes 5 minutes, agents spend most of their time waiting. This implies that *codebase architecture decisions should factor in agent productivity*, which is a radical reframing.

### Warm on keystroke (Ramp)

Start spinning up the sandbox before the user finishes typing. Combined with "allow reads before sync completes" — let the agent start reading immediately, block writes until sync is done. These UX optimizations make background agents feel as fast as local. Speed = adoption.

### Agent Trace as industry standard (Cognition + consortium)

An open spec for recording AI contributions alongside human authorship in git. Every agent lab independently invented conversation URLs — universal convergence suggests it's fundamental. The performance claim (3-point SWE-Bench improvement, 40-80% cache hit improvement) makes this not just an audit tool but a *performance optimization*. Early adoption is free upside.

### REPL persistence as implicit memory (Replit)

Variables in a notebook are "free" memory that doesn't consume context window. The agent builds up state in code rather than in context tokens. This is an elegant solution to the context budget problem for verification tasks.

---

## 4. Architecture Patterns

### Pattern 1: Single Agent Loop

**Who uses it:** Claude Code, pi-agent (Mario Zechner), most IDE copilots
**How it works:** One LLM in a while loop with tools (file read/write, bash, search). Context compaction when window fills.
**Pros:** Simple, debuggable, no coordination overhead, works for 80% of tasks
**Cons:** Can't parallelize, limited by single context window, loses coherence on long tasks
**Best for:** Individual features, bug fixes, code review, tasks completable in <1 hour

### Pattern 2: Initializer + Worker (Session Handoff)

**Who uses it:** Anthropic's harness architecture
**How it works:** Initializer agent sets up the project (writes init scripts, progress files, feature lists). Subsequent "shift worker" agents read progress files, work on one feature, commit clean, exit.
**Pros:** Handles multi-session tasks, maintains coherence via persistent artifacts, each session starts fresh
**Cons:** Information loss at session boundaries, progress file quality determines everything, slower per-feature
**Best for:** Multi-day projects, building complete applications feature-by-feature

### Pattern 3: Flat Parallel Fleet

**Who uses it:** Anthropic C compiler (16 parallel Claudes)
**How it works:** N identical agents with shared git repo. No orchestrator. Each agent independently picks work, pushes commits. Git merge conflicts force different task allocation.
**Pros:** Trivially parallelizable for independent tasks, no coordination bottleneck, emergent task allocation
**Cons:** Breaks down when tasks are interdependent, merge conflicts can introduce bugs, no global coherence
**Best for:** Large test suites, independent bug fixes, migrations across many files, embarrassingly parallel work

### Pattern 4: Planner/Worker Hierarchy

**Who uses it:** Cursor (their final architecture after 5+ iterations)
**How it works:** Root planner → sub-planners → workers. Workers get isolated repo copies, complete tasks, push structured handoffs up. Judge agent decides when to continue. No integrator.
**Pros:** Scales to 1M+ LoC over weeks, maintains global coherence, handles interdependent tasks
**Cons:** Complex to build and debug, planner quality determines everything, expensive (trillions of tokens)
**Best for:** Ambitious projects (building a browser from scratch), large-scale migrations, week-long autonomous runs

### Pattern 5: Write/Review/Fix Loop

**Who uses it:** Cognition (Devin + Devin Review), Continue (anti-slop agent), OpenAI (Ralph Wiggum Loop)
**How it works:** Writing agent generates code → review agent (fresh context) evaluates → fix agent addresses issues → CI re-runs → loop until clean
**Pros:** Catches bugs writing agent can't see, fresh context beats stale context, maps to human code review
**Cons:** Burns 2-5x more tokens than writing alone, can oscillate (fix introduces new issue), still needs human judgment for architectural decisions
**Best for:** Any production code, CI/CD pipelines, quality-critical systems

### Pattern 6: Pipeline (Specialized Stages)

**Who uses it:** Sourcegraph (Planning → Retrieval → Generation → Post-processing), Replit (coding agent + testing subagent)
**How it works:** Different stages handled by different systems (not all LLM-based). Rule-based planning, multiple retrievers, LLM generation, deterministic post-processing.
**Pros:** Each stage independently optimizable, can mix ML and non-ML components, latency-optimizable
**Cons:** Pipeline coupling, harder to debug end-to-end, requires stage-specific evaluation
**Best for:** Latency-sensitive applications, code completions, systems requiring deterministic guarantees

### The Meta-Pattern

Most production systems combine these. A typical mature setup: **Single agent for generation (Pattern 1) → Review agent with fresh context (Pattern 5) → CI pipeline with deterministic checks (Pattern 6)**. The planner/worker hierarchy (Pattern 4) is reserved for unusually large or ambitious projects.

---

## 5. The Eval Problem

### What people are measuring

- **Merge/acceptance rate** (Cognition: 67%, Ramp: ~30% of all PRs) — the most practical metric but conflates agent quality with task difficulty
- **SWE-Bench variants** (Verified, Pro) — the industry standard but increasingly gamed and limited to bug fixes on open-source projects
- **Code retention** (Cursor) — does agent-written code stay in the codebase? Simple, lagging, but real
- **pass@k vs. pass^k** (Anthropic) — at least one success in k trials vs. all k succeed. pass^k is the right metric for customer-facing consistency
- **Dissatisfied follow-up requests** (Cursor) — 2.2% reduction with better search. Measures agent frustration
- **Time savings** (Cognition: 10-14x for migrations, 20x for security vulns; Sourcegraph: 30 min/day for security review) — compelling but hard to standardize
- **Token cost per task** (Replit: ~$0.20/testing session) — efficiency metric

### What's broken about evals

**Eval quality > model quality.** Anthropic's Opus 4.5 scored 42% on CORE-Bench due to rigid grading, then 95% after fixing eval bugs. METR's benchmark penalized models that followed instructions correctly. The eval itself is usually the problem, not the model.

**No ground truth for context quality.** Sourcegraph: "What IS the right context for a query?" Manual annotation doesn't scale. User feedback attributes to LLM quality, not context quality. This is an unsolved problem.

**One-shot evals miss agentic gains.** Qodo was initially unimpressed by Opus 4.5 because one-shot coding evals didn't capture improvements on longer tasks. Need eval frameworks that match how agents actually work (multi-step, tool-using, iterating).

### What's working

**Eval-driven development** — write the eval first, then build the capability. Like TDD for agents. Anthropic and Descript both use this effectively.

**Swiss Cheese Model** — no single eval layer catches everything. Combine automated evals, production monitoring, A/B tests, user feedback, transcript review, and human studies. Each catches what others miss.

**Start with 20-50 tasks from real failures.** Sufficient for early-stage agents. Convert bug tracker / support queue into test cases.

---

## 6. Context Engineering

### The Framework

Lance Martin (LangChain) provides the clearest taxonomy: **Write, Select, Compress, Isolate.**

- **Write:** Scratchpads (NOTES.md, progress files), long-term memories, agent-written documentation
- **Select:** Retrieval (keyword + semantic + graph), progressive disclosure, just-in-time context loading
- **Compress:** Compaction (summarize history, preserve decisions, discard tool outputs), tool result clearing
- **Isolate:** Sub-agents with dedicated context windows, REPL state as implicit memory

### What works in practice

**Hybrid retrieval** (Sourcegraph, Cursor): Multiple complementary retrievers — keyword (Zoekt/grep), semantic embeddings, dependency graphs, local editor state. Each surfaces different types of relevant information. Combination always beats any single method.

**Progressive disclosure** (Anthropic, OpenAI): Don't dump everything upfront. Start with a map (~100 lines), let the agent explore. Cursor syncs MCP tool descriptions to a folder; agent reads only if needed. This is strictly better than front-loading.

**Structured note-taking** (Anthropic, Cognition, Cursor): Agent writes persistent files (NOTES.md, scratchpad.md, progress files). Key nuance from Cursor: **rewrite frequently, don't append.** Appendable files grow unbounded; rewritten files stay fresh.

**Sub-agent isolation** (Anthropic, Replit): Specialized sub-agents explore extensively (10K+ tokens), return condensed summaries (1-2K tokens). Replit's testing subagent exists specifically to avoid context pollution — main agent context reaches 80-100K tokens; mixing testing context in degrades performance.

### The critical failure mode: context rot

Chroma's research, validated by multiple teams: as token count increases, model's ability to recall information from context decreases. This is not a cliff but a gradient — "reduced precision for information retrieval and long-range reasoning." Every team that runs agents for more than ~30 minutes encounters this.

**Cognition's context anxiety** is a second-order effect: models trained to be aware of context limits take shortcuts when they *believe* they're near the limit, creating a behavior change independent of actual context rot.

### The key tradeoff

Compaction preserves tokens but loses information. "Overly aggressive compaction can result in the loss of subtle but critical context whose importance only becomes apparent later." There is no general solution — each domain requires tuning what to keep vs. discard.

---

## 7. Sandboxing & Safety

### The spectrum (from weakest to strongest)

1. **Permission prompts** (Claude Code default) — user approves each action. Safe but kills autonomy.
2. **Filesystem/network restrictions** (Claude Code sandboxing, macOS Seatbelt) — limits blast radius but shared kernel.
3. **Containers** (Docker) — process isolation, cheap, fast. NVIDIA argues this is insufficient for agentic workloads.
4. **MicroVMs** (Docker's new approach, Modal for Ramp) — stronger isolation, near-container speed.
5. **Full VMs** (Google Jules on GCP, Cursor's single large Linux VM) — strongest isolation, highest overhead.
6. **Copy-on-Write snapshots** (Replit's Bottomless Storage) — full reversibility. Agent actions become "transactions" — commit or rollback atomically.

### What the industry is converging on

**Full VM or microVM isolation for background agents.** Ramp uses Modal VMs. Google Jules uses GCP VMs. Cursor uses single large Linux VMs. NVIDIA explicitly recommends VM-level isolation over containers for agentic workloads.

**Replit's insight is the most forward-looking:** CoW snapshots make agent exploration cheap and fully reversible. You can let agents do risky things (mutate databases, install tools, add logging) in a fork, then cherry-pick only the results. Parallel sampling (72→80% on SWE-bench) becomes cheap with infrastructure support.

**Kubernetes is getting involved:** `kubernetes-sigs/agent-sandbox` provides a K8s controller for isolated, stateful agent workloads.

### The security gap

**Replit's research is alarming:** AI-only security scans are nondeterministic — identical vulnerabilities get different classifications based on minor syntactic changes. LLMs are blind to dependency vulnerabilities (no real-time CVE data). **Hybrid security is non-negotiable:** deterministic static analysis + dependency scanning as baseline, LLM reasoning as augmentation, not replacement.

---

## 8. The Human-in-the-Loop Question

### The emerging autonomy spectrum

**Level 1 — Copilot:** Human writes, AI suggests. (GitHub Copilot, Sourcegraph Cody completions)
**Level 2 — Supervised agent:** AI writes, human approves each step. (Claude Code default mode)
**Level 3 — Background agent with PR review:** AI writes autonomously, human reviews PR. (Ramp Inspect, Devin, OpenAI Codex)
**Level 4 — Agent-reviewed agent:** AI writes, another AI reviews, human spot-checks. (OpenAI's Ralph Wiggum Loop, Cognition's write→review→fix loop)
**Level 5 — Fully autonomous:** AI writes, reviews, merges, deploys. No human in loop. (Anthropic's C compiler for non-production code)

### Where teams actually are

Most production deployments are at **Level 3** (Ramp, Devin, Codex). The cutting edge is **Level 4** (OpenAI internally). **Level 5** is used only for experimental/non-production projects.

### The key insight from Cognition

"Managing Devin is a learned skill." Engineers must adjust workflows to scope work better upfront. The human role shifts from *writing code* to *specifying intent, reviewing output, and designing agent environments*. This is an organizational change, not just a technical one.

### What humans are still needed for

- **Ambiguous requirements** — agents fail when specs are vague (Cognition)
- **Architectural decisions** — agents follow patterns but can't make system design tradeoffs
- **Visual/aesthetic judgment** — agents need specific component structure, colors, spacing (Cognition)
- **Non-verifiable outcomes** — when you can't programmatically check correctness, humans must verify
- **Scope changes** — agents degrade when requirements change mid-task; worse than human juniors at iterative coaching

### The research says

Professional developers don't trust agents enough to stop reviewing (13 observed sessions, 99 survey responses). "Agents are treated like fast assistants, software quality still depends on human judgment." This is unlikely to change until agent reliability is dramatically higher.

---

## 9. What Nobody's Talking About

### Cost at scale

Teams mention "trillions of tokens" (Cursor), "massively increased token spend" (Cognition), "$20K for a C compiler" (Anthropic), but almost nobody publishes per-PR or per-feature cost data. Ramp's ~30% of merged PRs presumably costs significant API spend. **The economics of coding agents are opaque.** Without clear cost/benefit analysis, organizations can't make rational adoption decisions.

### Long-term codebase health

OpenAI acknowledges they "don't yet know how architectural coherence evolves over years" with agent-generated code. Agent-written code replicates existing patterns including bad ones ("architectural drift"). The 20%-of-time-on-cleanup problem suggests agent code generates more technical debt than human code. **Nobody has operated an agent-heavy codebase for more than ~18 months.** We have no idea what the 5-year maintenance story looks like.

### Agent-generated code auditability

Anthropic's C compiler author: "Programmers deploying software they've never personally verified is a real concern." Agent Trace is a step toward provenance, but nobody discusses how you audit 100K+ lines of agent-written code. Regulatory environments (healthcare, finance, defense) will need answers.

### Team skill atrophy

If engineers shift from writing to reviewing, do their coding skills atrophy? Ryan Carson: "There is a failure mode where you outsource the thinking and not the typing." This is mentioned in passing by a few people but nobody is studying it systematically.

### Non-English codebases and documentation

All research assumes English-language code, docs, and communication. How do agents perform on codebases with Japanese comments, Chinese documentation, or mixed-language READMEs? This affects a huge portion of the global developer population.

### Testing the tests

Replit asks: "How do you prevent the testing subagent from producing 'Potemkin test results' — tests that pass but don't actually verify the right thing?" This is turtles all the way down. Nobody has a satisfying answer.

### Multi-repo and monorepo coordination

Most examples are single-repo. Real companies have 10-100+ repos with cross-repo dependencies. How do agents handle changes that span multiple repos with different CI pipelines, different owners, and different deployment schedules?

### Failure modes at organizational scale

What happens when 50 engineers are each running 3-5 agent sessions simultaneously? Merge conflicts, CI queue saturation, review queue overload. Nobody has published data on organizational scaling limits.

---

## 10. Recommendations for Tavus

Tavus: AI video company, real-time conversational video, complex ML pipelines, Python/Rust stack, ~50-100 engineers.

### Start here (Weeks 1-4)

**1. Deploy a background coding agent for well-scoped tasks.** Use Claude Code or OpenCode with a CLAUDE.md/AGENTS.md file that describes your codebase architecture, conventions, and common patterns. Start with: bug fixes from your issue tracker, test coverage expansion, and documentation generation. Track merge rate as your north star metric. Target: >60% merge rate on these scoped tasks.

**2. Build a CI-triggered review agent.** Following Continue's anti-slop pattern and Sourcegraph's Sherlock: on every PR (human or agent-generated), run an automated review agent that checks for Tavus-specific quality standards — video pipeline conventions, API design patterns, security concerns around media processing. This is low-risk, high-signal, and will pay dividends immediately regardless of how much agent-generated code you produce.

**3. Write custom linters with agent-readable error messages.** OpenAI's key insight. Your linter errors should say "Use our `VideoEncoder` abstraction, not raw FFmpeg calls. See docs/video-encoding.md for the API." This mechanically enforces conventions without relying on the agent reading documentation.

### Build the foundation (Months 2-3)

**4. Invest in codebase legibility for agents.** This means:
- An AGENTS.md that's a ~100-line map pointing to structured docs (OpenAI pattern)
- Modularize your build — if your Rust compilation takes 5 minutes, agents waste most of their time waiting (Cursor's insight that project structure affects token throughput)
- Expose video processing metrics (latency, quality scores, resource usage) via queryable interfaces so agents can verify their own work
- Write init scripts that set up the dev environment automatically

**5. Build domain-specific evaluation.** Adapt Descript's framework: "Don't break things, do what I asked, do it well." For Tavus this means:
- **Don't break things:** Existing avatars render correctly, existing API contracts hold, video quality doesn't regress
- **Do what I asked:** Feature matches specification, tests pass, code compiles
- **Do it well:** Follows Tavus conventions, performant, no code slop
- Start with 20-50 eval tasks from real bugs and feature requests. Use pass^k (not pass@k) because your customers expect consistent quality.

**6. Sandbox with full VM isolation.** Your ML pipelines involve GPU resources, model checkpoints, and likely sensitive training data. Use Modal (like Ramp) or equivalent: each agent session gets a sandboxed VM with your full dev environment (FFmpeg, ML inference runtimes, etc.). Pre-warm images on a 30-minute rebuild cycle. Implement Replit's CoW snapshot pattern if your state management is complex enough to warrant it.

### Scale strategically (Months 3-6)

**7. Use the fleet model for migrations and test coverage.** Cognition's "junior at infinite scale" is your first high-ROI pattern. Write a playbook (migration instructions, testing guide), then a fleet of agents executes across your codebase. Use cases: framework upgrades, API migration, expanding test coverage from 60% to 90%, security vulnerability remediation.

**8. Don't build multi-agent orchestration yet.** For a 50-100 engineer team, the planner/worker hierarchy (Cursor's approach) is overkill. Single well-scaffolded agents for individual tasks + the fleet model for parallelizable work covers 95% of use cases. Revisit multi-agent orchestration when you have clear evidence of needing it.

**9. Adopt Agent Trace early.** It's an open standard with buy-in from Cursor, Google Jules, Vercel, Cloudflare. The cost is near-zero. The benefits: provenance tracking, improved cache hit rates (40-80% per Cognition), and data for improving your agent over time.

### What to avoid

- **Don't try to build an agent for greenfield video pipeline features.** Agents are bad at ambiguous requirements and aesthetic judgment — two things central to novel video AI work. Use agents for well-specified, verifiable tasks.
- **Don't trust LLM-only security scanning.** Replit proved this is nondeterministic. Use traditional SAST/DAST + dependency scanning as baseline, LLM reasoning as augmentation.
- **Don't over-invest in prompt engineering at the expense of tooling.** The prompt matters, but the test suite, linters, CI pipeline, and evaluation framework matter more. Build the environment, not the prompt.
- **Don't let agents touch production without human review** for at least the first 6 months. The Level 4 autonomy (agent-reviewed agent) pattern is viable for low-risk code, but video pipeline code with ML inference, real-time processing, and customer-facing output is high-risk.

### The Tavus-specific opportunity

Your Python/Rust stack is well-suited for agents — both languages are well-represented in training data ("boring tech" advantage per OpenAI). Your video processing pipeline is likely highly modular (encoding, decoding, lip sync, avatar rendering, audio processing) — this natural decomposition maps well to agent task boundaries. And your ML pipeline likely generates many similar tasks (training runs, experiment tracking, data preprocessing) that are perfect for the fleet model.

The biggest risk isn't technical — it's organizational. "Managing agents is a learned skill" (Cognition). Your engineers need to learn to write clear specifications, scope work tightly, and review agent output effectively. Budget for this learning curve. The companies that succeed with coding agents are the ones that treat it as an organizational transformation, not a tool adoption.

---

## Methodology Notes

- **Strong consensus** = 4+ independent teams agree, with production evidence
- **Emerging consensus** = 3+ teams agree, some still experimental
- **My speculation** = clearly flagged as such, based on pattern-matching across sources
- Sources span Jan 2025 – Feb 2026. The field moves fast; some recommendations may date within months.
- Notably absent from our sources: GitHub Copilot Workspace, Amazon Q Developer deep dives, Cline/Aider/open-source agent architectures. These would enrich the analysis.
