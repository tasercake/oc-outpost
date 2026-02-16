# Cursor: Towards Self-Driving Codebases

**Source:** https://cursor.com/blog/self-driving-codebases
**Date researched:** 2026-02-15

## One-line summary
Deep technical post-mortem on Cursor's multi-agent harness evolution through 5+ design iterations—from flat self-coordination to recursive planner/worker hierarchy—with hard-won lessons on prompting, infrastructure, intent specification, and accepting imperfection at scale.

## Key architectural decisions

### System evolution (in order)
1. **Single agent** → Failed. Lost track, proclaimed false success, couldn't handle complexity.
2. **Manual multi-agent with dependency graph** → Better throughput, but agents couldn't communicate or provide feedback.
3. **Self-coordinating peers (shared state file)** → Failed. Lock contention, confusion, risk aversion.
4. **Planner → Executor → Workers + Judge** → Better coordination but bottlenecked on slowest worker. Too rigid—upfront planning couldn't adapt.
5. **Continuous executor (planner+executor merged)** → Too many roles, pathological behaviors (sleeping randomly, refusing to spawn tasks, premature completion).
6. **Final: Recursive planner hierarchy + independent workers** → Root planner → subplanners → workers. Workers isolated, push handoffs up. No integrator.

### Infrastructure choices
- **Single large Linux VM** per run (not distributed) to avoid premature complexity
- **Extensive observability:** all agent messages, system actions, command outputs logged with timestamps
- **Workers get their own copy of the repo** — isolation over shared state
- **Handoff mechanism:** workers write structured summaries (what was done + notes/concerns/deviations/feedback) that propagate up to planners

### Freshness mechanisms
- `scratchpad.md` frequently rewritten (not appended)
- Auto-summarization at context limits
- Self-reflection and alignment reminders in system prompts
- Encouraged to pivot and challenge assumptions

## What worked

- **Recursive subplanners** for fan-out: rapidly spawning workers while maintaining ownership hierarchy
- **Handoff-based communication** instead of shared state: workers complete tasks and push structured summaries up. No cross-talk between workers.
- **Removing the integrator:** was "red tape"—hundreds of workers, one gate. Workers capable of handling conflicts themselves.
- **Accepting turbulence:** multiple agents touching same files → let it naturally converge rather than over-engineering coordination
- **Accepting a stable error rate:** requiring 100% correctness before commits caused serialization, agents going out of scope to fix irrelevant things, trampling each other. Small steady error rate + final fixup pass is better.
- **~1,000 commits/hour** across 10M tool calls over one week, no human intervention needed
- **Broken state recovery:** after major restructuring into many crates, system converged from heavily broken state to working code in days
- **Modular crate structure** dramatically improved throughput (less compilation wait time)

## What failed or was hard

- **Self-coordination:** locks fundamentally don't work for LLM agents. They don't understand the significance of holding a lock.
- **Continuous executor (merged planner+executor):** too many simultaneous roles overwhelmed the agent. Slept randomly, stopped spawning, claimed premature completion.
- **100% commit correctness requirement:** caused cascading failures, agents going out of scope, piling on same issues
- **Monolith project structure:** hundreds of agents compiling simultaneously → many GB/s disk I/O, significant throughput impact. "Project structure and developer experience affect token throughput."
- **Git and Cargo shared locks:** designed for single-user, became bottlenecks at multi-agent scale
- **Vague instructions amplified at scale:** "spec implementation" led agents into obscure rarely-used features. "Generate many tasks" → conservative small output.
- **Missing implicit requirements:** performance expectations, dependency philosophy, resource management all needed explicit specification
- **First architecture was unfit:** initial browser architecture couldn't evolve into full browser. "Failure of the initial specification."

## Novel insights

1. **Agents follow instructions to the end, good or bad.** At scale, bad instructions become catastrophic. The harness "merely followed our instructions exactly."

2. **Constraints > instructions for prompting.** "No TODOs, no partial implementations" works better than "remember to finish implementations." Models do good things by default; constraints define boundaries.

3. **Don't instruct what the model already knows.** Only instruct what it doesn't know (multi-agent collaboration) or domain-specific things (how to run tests). "Treat the model like a brilliant new hire."

4. **Give concrete numbers for scope.** "Generate many tasks" → small output. "Generate 20-100 tasks" → dramatically different behavior. Numbers convey ambition.

5. **Avoid checkbox mentality.** Listing specific items to do deprioritizes unlisted things and narrows focus. Better to describe intent and let the model use judgment.

6. **Project structure affects agent throughput.** Modular crates >> monolith because compilation time dominates, not thinking/coding time. This is a form of "developer experience for agents."

7. **Copy-on-write and deduplication for multi-agent repos** could bring huge efficiency gains—most files identical across agent copies.

8. **Anti-fragility as design principle.** More agents = higher probability of individual failure. System must withstand failures and let others recover.

9. **Empirical over assumption-driven.** Don't assume human org patterns or distributed systems patterns will work for agents. Observe and iterate.

10. **The "ideal efficient system" accepts some error rate** with a final reconciliation pass, rather than perfect code 100% of the time.

## Practical Applications

- **Agent developer experience matters:** If your team's codebase has long build times, monolith structure, or complex setup, agents will be slower. Modularizing specifically for agent productivity is a real lever.
- **Handoff-based architecture:** For a coding agent product, structured handoffs (what was done + concerns + deviations) is a pattern to adopt for multi-step workflows.
- **Prompting patterns directly applicable:**
  - Constraints > instructions
  - Concrete numbers for scope
  - Don't instruct what models already know
  - Describe intent, not checkboxes
- **Freshness mechanisms** (scratchpad rewriting, auto-summarization, self-reflection) critical for any long-running agent your team builds
- **Single VM simplicity:** Don't prematurely distribute. A single large machine with good observability can handle hundreds of agents.
- **Observability-first:** Log everything, replay sessions, use Cursor itself to analyze agent logs. Essential for iterating on agent behavior.
- **Specification quality scales with compute:** If your team runs expensive agent workflows, investing in specification/intent quality has outsized ROI.

## Open questions

- How exactly are worker repo copies managed? Shallow clones? Full copies? What's the disk overhead?
- What's the cost per run? Trillions of tokens × API pricing = ?
- How do planners decide when to spawn subplanners vs workers directly?
- What happens when the recursive planner hierarchy gets too deep?
- How is the handoff format structured exactly? Is it free-form or templated?
- Could this architecture work for non-coding tasks (e.g., video pipeline orchestration)?
- What's the model context window utilization pattern over week-long runs?

## Links to related work

- [Scaling Agents (companion post)](https://cursor.com/blog/scaling-agents) — higher-level summary
- [FastRender browser source](https://github.com/wilsonzlin/fastrender)
- [Cursor Semantic Search](https://cursor.com/blog/semsearch) — retrieval for agents
- Conceptually related: MapReduce (fan-out/fan-in), actor model (isolated workers with message passing)
- "Virtuous AI loop" — AI used to develop AI, accelerating feedback cycle
