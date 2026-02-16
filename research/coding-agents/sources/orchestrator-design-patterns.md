# Orchestrator Design Patterns in Multi-Agent Systems

Research compiled 2026-02-16. Sources: Cursor blog, Anthropic engineering blog, Microsoft Research, NVIDIA Research, arxiv papers, Google A2A docs.

---

## 1. Model Selection for Planning vs Execution

### The Key Finding: Planning ≠ Coding Ability

**Cursor's discovery** (Jan 2026, "Scaling Long-Running Autonomous Coding"):
- GPT-5.2 is a better **planner** than GPT-5.1-Codex, even though Codex is specifically trained for coding
- "We now use the model best suited for each role rather than one universal model"
- Opus 4.5 "tends to stop earlier and take shortcuts when convenient, yielding back control quickly" — bad for long autonomous runs
- Source: https://cursor.com/blog/scaling-agents

**NVIDIA Orchestrator-8B** (Dec 2025, ToolOrchestra):
- A fine-tuned **8B parameter model** outperformed GPT-5, Claude Opus 4.1, and all frontier models as an orchestrator
- On Humanity's Last Exam: Orchestrator-8B scored 37.1 vs GPT-5's 21.2 (with same tools)
- Cost: $9.20 vs $17.80 (GPT-5) or $52.50 (Claude Opus 4.1) per problem
- Key insight: "Small models are unburdened by excessive knowledge and trained to capture the essence of problem-solving due to their limited size"
- Trained with only 552 synthetic problems and 1,296 prompts via multi-objective RL
- Source: https://developer.nvidia.com/blog/train-small-orchestration-agents-to-solve-big-problems/

**Anthropic's Claude Research** (Jun 2025):
- Claude Opus 4 as lead agent + Claude Sonnet 4 as subagents outperformed single-agent Opus 4 by **90.2%**
- Lead agent handles planning/delegation; subagents handle parallel search/analysis
- Token usage alone explains **80%** of performance variance on BrowseComp
- Source: https://www.anthropic.com/engineering/multi-agent-research-system

**Difficulty-Aware Orchestration** (arxiv, Sep 2025):
- "Smaller LLMs can outperform larger models in specific domains while incurring lower inference costs"
- Heterogeneous model ensembles with proper orchestration achieve better cost-performance trade-offs than homogeneous systems
- Source: https://arxiv.org/html/2509.11079v1

### Summary: Does the Manager Need to Be the Strongest Model?

**No.** The evidence is surprisingly strong:
1. NVIDIA proved a fine-tuned 8B model orchestrates better than frontier models (with RL training)
2. Cursor found general reasoning (GPT-5.2) beats domain-specific coding models (Codex) at planning
3. Anthropic uses a stronger model as lead but cheaper models for execution — the architecture matters more than uniform model quality
4. Planning and routing are fundamentally different skills than execution. A model that's great at code generation may be poor at decomposing problems.

---

## 2. Single Point of Failure & Failure Handling

### Failure Modes of Centralized Orchestration

1. **Orchestrator hangs → system dies** (SBP protocol documentation explicitly calls this out)
2. **Bottleneck under load** — Cursor found that coordination mechanisms (locks, shared files) reduced 20 agents to effective throughput of 2-3
3. **Planning rigidity** — wrong initial decomposition cascades through all workers
4. **Risk aversion** — Cursor found flat/self-coordinating agents became risk-averse; nobody owned hard problems
5. **Infinite loops** — Anthropic found agents "scouring the web endlessly for nonexistent sources" and spawning 50 subagents for simple queries
6. **Context window exhaustion** — orchestrator accumulates context from all agents, hits limits first

### How Systems Handle Orchestrator Failure

**Cursor's approach**: No explicit failure recovery documented. They use a "judge agent" at the end of each cycle to determine whether to continue, then start the next iteration fresh. Periodic fresh starts combat drift. If the planner fails, workers have no work — but workers are designed to be self-contained once they have a task.

**Anthropic's approach**: Checkpointing, retry logic, and "rainbow deployments" for safety. The lead agent saves its plan to Memory early (persisting context against truncation). If context window exceeds 200K tokens, it's truncated but the plan survives in memory.

**Magentic-One (Microsoft)**: The Orchestrator has two loops:
- **Outer loop**: Task Ledger (facts, guesses, plan) — if inner loop stalls, outer loop creates a new plan
- **Inner loop**: Progress Ledger (tracks progress, assigns subtasks)
- If progress isn't being made for enough steps, it triggers re-planning
- This is self-healing within a single orchestrator, but the orchestrator itself is still a SPOF

**Distributed systems patterns** (classical):
- **Leader election** (Raft, Paxos): multiple potential orchestrators, one active. On failure, others elect a new leader. Not yet applied to LLM agents in published work.
- **Consensus protocols**: All agents agree on next action. Expensive in tokens but eliminates SPOF.
- **Supervisor trees** (Erlang/OTP): Hierarchical supervision where a failed component is restarted by its supervisor. Maps well to planner-worker where planners supervise workers.

---

## 3. Orchestrator-Free Architectures

### Stigmergy (Ant Colony Coordination)

**SBP — Stigmergic Blackboard Protocol** (Feb 2026, open-sourced):
- Agents coordinate through environment-based signals ("digital pheromones") on a shared blackboard
- Signal intensity + decay curves prevent stale data accumulation
- "Agents simply 'sense' the blackboard and react based on their own internal logic"
- Designed as complement to MCP: "MCP defines capabilities (tools); SBP defines awareness (collective task state)"
- Used in production for "non-linear workflows where agents need to pick up where others left off"
- Source: https://github.com/AdviceNXT/sbp

**Cursor's failed flat coordination**: They tried stigmergy-like approaches first (shared file, agents self-coordinate). It failed because:
- Lock contention / forgotten locks
- Risk aversion — no agent owned hard problems
- Work churning without progress
- They moved TO hierarchy (planner-worker) as a solution

### Blackboard Systems

**LbMAS — Blackboard-based LLM Multi-Agent System** (arxiv, Jul 2025):
- Classic blackboard architecture applied to LLM agents
- Three components: control unit, blackboard (shared memory), group of role-based agents
- Control unit (itself an LLM) selects which agents participate each round based on blackboard state
- Blackboard divided into public space + private spaces (for debates/verification)
- Agents communicate ONLY through the blackboard — replaces per-agent memory
- Results: competitive with SOTA while spending **fewer tokens**
- Key advantage: agents see ALL historical context, enabling dynamic collaboration without predefined workflows
- Source: https://arxiv.org/abs/2507.01701

### Tuple Spaces (Linda)

No published LLM-agent systems using tuple spaces directly, but the pattern maps naturally:
- Shared associative memory where agents post and retrieve typed tuples
- Agents are decoupled — they don't need to know about each other
- Natural fit for task queues (post task tuple, worker takes matching tuple)
- SBP and blackboard systems are effectively modern incarnations of this pattern

### Assessment: Can Orchestrator-Free Work?

**For LLM agents specifically, the evidence says: partially.**
- Stigmergy/flat coordination failed for Cursor at scale (hundreds of agents) — hierarchy was necessary
- Blackboard systems show promise for reasoning/math tasks (competitive + token-efficient)
- The hybrid approach (blackboard for communication, lightweight control unit for selection) may be the sweet spot
- Pure peer-to-peer breaks down because LLM agents lack the reliable, deterministic behavior that makes ant colonies work

---

## 4. Cost of Orchestration

### Published Data Points

| System | Orchestrator Overhead | Notes |
|--------|----------------------|-------|
| Anthropic Research | Multi-agent uses ~15× more tokens than chat, ~4× more than single agent | Lead agent + subagent architecture |
| NVIDIA Orchestrator-8B | $9.20/problem total (orchestrator is 8B, tiny cost) | vs $52.50 for Claude Opus 4.1 monolithic |
| Kore.ai analysis | Patterns vary by >200% in token usage | Depends on reasoning iterations |
| Microsoft (Azure guide) | "Magentic orchestrations are the most variable" — iterative planning makes cost hard to predict | Recommends monitoring per-agent token consumption |
| LbMAS (Blackboard) | "Token-economical" — competitive accuracy with fewer tokens | Blackboard as shared memory reduces redundant context |

### Key Insight on Cost

The orchestrator's token cost is less important than **whether it reduces total system tokens**. A good orchestrator:
- Prevents duplicate work
- Stops agents from spinning on impossible tasks
- Routes to cheaper models when appropriate
- NVIDIA showed an 8B orchestrator can cut total cost by 3-6× vs frontier models doing everything

Microsoft Azure's guidance: "Not every agent requires the most capable model. Agents that perform classification, extraction, or formatting can often use smaller, less expensive models."

---

## 5. How Existing Systems Handle the Coordinator Role

### Cursor: Planner-Worker Hierarchy

- **Architecture**: Planners explore codebase → create tasks. Workers execute tasks. Judge evaluates cycles.
- **Planners can spawn sub-planners** — planning itself is parallel and recursive
- **On planner failure**: Not explicitly handled. Workers are self-contained once assigned. Periodic fresh starts provide implicit recovery.
- **Key learning**: "The right amount of structure is somewhere in the middle. Too little → conflict, duplication, drift. Too much → fragility."
- **Prompts > Architecture**: "A surprising amount of the system's behavior comes down to how we prompt the agents"
- They removed complexity: killed the "integrator" role for quality control — it created more bottlenecks than it solved

### Anthropic: Orchestrator-Worker (NOT No-Orchestrator)

Despite some characterizations, Anthropic DOES use an orchestrator:
- **Lead Researcher agent** (Opus 4) decomposes query, spawns subagents (Sonnet 4)
- Subagents search in parallel with independent context windows
- Lead agent synthesizes results, decides if more research needed, can spawn more subagents
- **CitationAgent** as final post-processing step
- **Key engineering**: Lead agent must give detailed task descriptions to subagents. Vague instructions ("research the semiconductor shortage") led to duplicated/missed work.
- **Failure handling**: Checkpointing, retry logic, rainbow deployments. Plan saved to persistent memory early.
- **What Anthropic learned**: "Teaching the orchestrator how to delegate" was critical. Subagents need objective, output format, tool guidance, and clear task boundaries.

### AutoGen Group Chat Manager

- GroupChat with a manager agent that selects next speaker
- Manager uses LLM to decide which agent speaks next based on chat history
- **Orchestrator is pluggable** — can swap selection strategy (round-robin, random, LLM-based, custom)
- **Failure modes**: Manager model can misroute, select wrong agent, or loop. No built-in failure recovery — relies on termination conditions and max-turn limits.
- Community requests for customizable orchestrator with error handling/retry (GitHub issue #2695)

### Microsoft Magentic-One

- **Orchestrator + 4 specialized agents**: WebSurfer, FileSurfer, Coder, ComputerTerminal
- **Dual-loop architecture**:
  - Outer loop: Task Ledger (facts, guesses, plan) — strategic level
  - Inner loop: Progress Ledger (current progress, agent assignments) — tactical level
- **Self-healing**: If inner loop stalls, outer loop re-plans. Orchestrator tracks whether progress is being made.
- **Limitation**: Orchestrator is still SPOF. If orchestrator model fails/hallucinates, entire system fails.
- **Modular design**: Agents can be added/removed without reworking the system (vs monolithic approaches)
- Built on AutoGen framework

### Google A2A (Agent-to-Agent Protocol)

- **Not an orchestration framework** — it's an interoperability protocol
- Enables agents to discover, communicate, and delegate across organizational boundaries
- **Client-server model**: One agent (client) sends tasks to another (server). Server returns results asynchronously.
- **Agent Cards**: JSON metadata describing agent capabilities, used for discovery
- **No built-in coordinator**: A2A is deliberately agnostic about orchestration. Any agent can be a client (coordinator) or server (worker).
- **Complementary to MCP**: MCP = tool access; A2A = agent-to-agent communication
- **Implications**: In A2A, the "manager" is just another agent that happens to delegate. You could have multiple managers, or agents that both delegate and execute.
- Source: https://a2a-protocol.org/latest/

---

## 6. Synthesis: Answering the Original Questions

### Q1: Does the manager need to be the strongest model?

**No.** Three independent findings converge:
1. **NVIDIA**: Fine-tuned 8B model beats frontier models as orchestrator (37.1 vs 21.2 on HLE)
2. **Cursor**: Best planner (GPT-5.2) ≠ best coder (Codex). Planning is a different skill.
3. **Anthropic**: Stronger model as lead + cheaper subagents is the winning combo

The orchestrator needs to be good at **decomposition, delegation, and progress tracking** — not necessarily at execution. A well-prompted or fine-tuned smaller model can excel at these meta-cognitive tasks. The key is matching model capability to role requirements.

### Q2: What are the failure modes? How do systems handle orchestrator failure?

**Failure modes**: Hangs, bottlenecks, bad decomposition cascading, infinite loops, context exhaustion, risk aversion in flat structures.

**Current handling is primitive**:
- Re-planning on stall detection (Magentic-One)
- Periodic fresh starts (Cursor)
- Checkpointing + retry (Anthropic)
- Nobody does leader election or hot standby for the orchestrator

**This is an open problem.** No published multi-agent LLM system has robust orchestrator failover.

### Q3: Can you have graceful degradation?

**Yes, in theory. Partially demonstrated in practice:**
- Cursor showed workers can function independently once assigned tasks (degraded mode = workers finish current tasks, no new planning)
- Blackboard systems inherently degrade gracefully — if control unit fails, agents could still read/write the blackboard
- A2A's decentralized design means any agent can step into a coordinator role
- **Concierge-as-degraded-manager** pattern: feasible if the concierge has enough context about the system's state. The key requirement is persistent shared state (blackboard, task ledger) that any agent can read.

---

## Sources

1. Cursor, "Scaling Long-Running Autonomous Coding" (Jan 2026) — https://cursor.com/blog/scaling-agents
2. Anthropic, "How We Built Our Multi-Agent Research System" (Jun 2025) — https://www.anthropic.com/engineering/multi-agent-research-system
3. Microsoft Research, "Magentic-One" (Nov 2024) — https://www.microsoft.com/en-us/research/articles/magentic-one-a-generalist-multi-agent-system-for-solving-complex-tasks/
4. NVIDIA, "Train Small Orchestration Agents to Solve Big Problems" (Dec 2025) — https://developer.nvidia.com/blog/train-small-orchestration-agents-to-solve-big-problems/
5. Han & Zhang, "Exploring Advanced LLM Multi-Agent Systems Based on Blackboard Architecture" (Jul 2025) — https://arxiv.org/abs/2507.01701
6. SBP Protocol, "Stigmergic Blackboard Protocol" (Feb 2026) — https://github.com/AdviceNXT/sbp
7. Google, "A2A Protocol" — https://a2a-protocol.org/latest/
8. Microsoft Azure, "AI Agent Orchestration Patterns" — https://learn.microsoft.com/en-us/azure/architecture/ai-ml/guide/ai-agent-design-patterns
9. Kore.ai, "Choosing the Right Orchestration Pattern" (Feb 2026) — https://www.kore.ai/blog/choosing-the-right-orchestration-pattern-for-multi-agent-systems
