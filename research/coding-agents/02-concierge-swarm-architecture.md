# Concierge-Swarm Architecture: Design Document

A multi-agent coding system where a fast concierge handles user interaction while a manager orchestrates a dynamic swarm of specialized workers.

## 1. Core Architecture

```
                         ┌──────────┐
                         │   User   │
                         └────┬─────┘
                              │
                    ┌─────────┴─────────┐
                    │ (concurrent)      │
                    ▼                   ▼
            ┌─────────────┐    ┌──────────────┐
            │  Concierge  │    │   Manager    │
            │ (fast model │    │  (steering)  │
            │  read-only) │    └──────┬───────┘
            └──────┬──────┘           │
                   │                  │ spawns workers;
                   │ reads            │ reads/writes queue
                   │                  │
                   │           ┌──────▼───────────────────┐
                   └──────────►│      Central Queue       │
                               │                          │◄──┐
                               └──────────┬───────────────┘   │
                                          │                    │
                                    ┌─────▼─────┐      ┌──────┴────┐
                                    │ Worker A  │      │ Worker N  │
                                    │ (dynamic) │      │ (dynamic) │
                                    └─────┬─────┘      └─────┬─────┘
                                          │                   │
                                    ┌─────▼─────┐      ┌─────▼─────┐
                                    │ Sub-agent │      │ Sub-agent │
                                    └───────────┘      └───────────┘
```

Key topology:
- **User → Concierge + Manager concurrently** (fan-out on input)
- **Manager → Queue** (spawns workers, then steers them *indirectly* via addressed messages on the queue — no direct control connection)
- **Workers ↔ Queue** (read addressed messages + write status/results; this is how they receive steering from the manager)
- **Concierge → Queue** (read-only, for status reporting to user)
- **Workers → Sub-agents** (explicit management; sub-agents are NOT connected to the central queue, only to their parent worker)
- **Queue is the sole communication backbone** between manager, workers, and concierge

### Components

**Concierge** — Fast, conversational model (e.g., Haiku-class, Gemini Flash). Never participates in the core work loop. Responsibilities:
- Acknowledge user requests immediately ("On it — breaking this down now")
- Read the central queue and report status at milestones
- Answer user questions about what's happening
- Optionally: read streaming outputs from other agents for semi-realtime reporting (see §7)

**Manager** — Strong planning model, not necessarily the largest. Sole agent with full visibility into all agents and the complete queue. Responsibilities:
- Decompose user requests into tasks
- Spawn top-level workers (the only direct control action)
- Steer workers **indirectly** by posting addressed messages to the queue (e.g., reprioritize, reassign, provide context)
- Maintain agent registry (who exists, what they're doing)
- Resolve conflicts when workers flag disagreements
- Selectively introduce agents to each other when collaboration is needed

The manager has **no direct runtime connection to workers**. All steering flows through the queue. This means workers can operate even if the manager is temporarily unavailable — they just won't receive new instructions until it's back.

**Workers** — Specialized agents spawned by the manager. Connected to the central queue for receiving instructions and communicating with peers. Each worker:
- Reads messages tagged to it + a compacted summary of the broader queue
- Posts status updates and results to the queue
- Can tag other workers it knows about
- Has a dynamic role tied to its current task, not a fixed archetype
- Can spawn and **directly manage** sub-agents (sub-agents are NOT connected to the central queue — they only communicate with their parent worker)
- Recursive sub-agent spawning up to a configurable depth limit

**Central Queue** — The shared communication backbone (see §3 for semantics).

## 2. Design Principles

### 2.1 Never block the fast path
The concierge must always be able to respond, regardless of manager or worker state. It reads the queue; it never writes to it (except user-facing responses). If the manager is busy decomposing, the concierge can still say "your request is being broken down." This is the EPIC principle applied at the system level.

### 2.2 Human-like interaction cadence
A human given a task doesn't disappear for 20 minutes. They say "got it," give periodic updates, and flag blockers. The concierge mimics this:
- **Immediate**: "Got it, looking into this"
- **Decomposition**: "Breaking this into 3 parts: X, Y, Z"
- **Progress**: "X is done, Y is 60% through, Z is blocked on..."
- **Completion**: "Here's what we built. Agent A handled X, Agent B handled Y..."

### 2.3 Event-driven, not polling
Agents do nothing when there are no relevant messages. Activation is triggered by new messages matching relevance criteria, not periodic polling. This is critical for cost control (see §4).

### 2.4 Hierarchy with selective visibility
Not every agent needs to know about every other agent. The manager maintains the full topology; workers know about collaborators the manager explicitly introduces, plus any agents they discover through queue topic matching.

## 3. Queue Semantics

### Recommended: Hybrid filtered queue

A flat, append-only queue is the source of truth. But agents don't read the raw queue — they get **filtered views**:

| Agent | What it sees |
|-------|-------------|
| Manager | Full queue (all messages, unfiltered) |
| Concierge | Full queue (read-only, for status reporting) |
| Workers | Messages tagged to them (full) + periodic compacted summary of broader queue |

This gives you:
- **No information loss** — the full queue always exists
- **Cost control** — workers only pay full context cost for relevant messages
- **Ambient awareness** — the compacted summary lets workers notice relevant activity without reading every message

### Message format

Structured envelope for cheap filtering, natural language body for flexibility:

```json
{
  "id": "msg-uuid",
  "from": "agent-id",
  "to": ["agent-id", ...] | "*",      // explicit targets or broadcast
  "topic": ["#auth", "#frontend"],     // semantic tags for discovery
  "type": "task|status|question|result|conflict",
  "priority": "normal|urgent",
  "body": "Natural language content...",
  "refs": ["msg-uuid-1", ...]         // reply-to / thread
}
```

Agents can filter on envelope fields (~20 tokens) without reading the body. The `topic` field enables discovery — a worker on `#auth` can notice another worker posting `#auth`-related updates without the manager introducing them.

### Threading

Task-scoped threads (via `refs`) keep conversations organized. A thread about "refactor the payment module" generates 1 notification for non-participants vs N messages in a flat stream. Workers subscribe to threads they're participating in.

## 4. Agent Activation

### Three-tier activation (zero to minimal LLM cost)

When a new message hits the queue:

1. **Deterministic filter** (zero cost): Is this agent explicitly tagged in `to`? Is the message `type` one this agent handles? → If yes, activate.
2. **Embedding similarity** (near-zero cost): Compare message embedding against agent's capability embedding using semantic-router. Sub-millisecond, no LLM tokens. → If similarity > threshold, activate.
3. **Manager routing** (LLM cost, rare): For ambiguous messages that don't match any agent's filter or embedding, the manager decides who should handle it.

This gives ~10× cost reduction over naive "every agent reads every message."

### Cost model

| Approach | Token multiplier (10 agents, 100 messages) |
|----------|-------------------------------------------|
| Naive broadcast | 10× per message, O(N×M²) total |
| Tag-only filtering | ~2-3× (most messages target 1-2 agents) |
| Three-tier activation | ~1.5-2× (deterministic handles 80%+) |

## 5. Agent Awareness & Discovery

### Manager as directory

The manager maintains an agent registry (not an LLM — a data structure):

```json
{
  "agents": {
    "worker-a": {
      "role": "Implementing auth module",
      "topics": ["#auth", "#backend", "#security"],
      "status": "active",
      "spawned_at": "...",
      "last_activity": "..."
    }
  }
}
```

### Discovery mechanisms

1. **Explicit introduction**: Manager tells Worker A "Worker B is handling the API layer, here's how to reach them." Used for known collaborations.
2. **Topic-based discovery**: Worker A posts a `#database` question. Worker C, subscribed to `#database`, sees it and can respond. No manager involvement.
3. **Registry query**: Any agent can query the registry ("who's working on auth?") without an LLM call — it's a data lookup.

### Dynamic roles

Agent roles are tied to tasks, not archetypes. When the manager spawns a worker, it assigns:
- A task description (what to do)
- Topic tags (for discovery)
- Known collaborators (explicit introductions)

As work evolves, agents update their own topic tags. The registry reflects current state, not initial assignment.

## 6. Manager Design

### Model selection

Research finding: **planning ability ≠ model size**. NVIDIA's fine-tuned 8B orchestrator beat GPT-5 and Claude Opus 4.1 at routing tasks at 1/6th the cost. Cursor found GPT-5.2 (general) plans better than GPT-5.1-Codex (coding-specialized).

Recommendation: Use the strongest model *for planning and decomposition*, which may not be the largest or most expensive model. Consider fine-tuning a smaller model for routing decisions specifically.

### Failure modes and mitigation

| Failure | Impact | Mitigation |
|---------|--------|------------|
| Manager produces bad decomposition | Swarm works on wrong things | Workers can flag "this doesn't make sense given X" — need enough queue visibility to push back |
| Manager goes down mid-task | No new work spawned, in-flight work continues | Workers finish current tasks. Concierge reports status. Queue persists. Manager restarts and reads queue to recover state. |
| Manager is slow | User waiting for decomposition | Concierge acknowledges immediately. Manager decomposes async. "Warm on keystroke" — start provisioning worker slots while manager plans. |
| Manager makes conflicting assignments | Two workers collide | Workers detect conflicts via queue (same file/topic) and escalate to manager. Git-level conflicts caught mechanically. |

### The blackboard advantage

The central queue effectively creates a **blackboard architecture** — shared state that any agent can read. This means the system degrades gracefully:
- Manager down → workers see each other's updates, can self-coordinate for simple cases
- Worker down → manager sees missing status updates, can reassign
- Concierge down → user loses visibility but work continues

This is significantly more resilient than hub-and-spoke (where coordinator death = full stop).

## 7. Concierge: Streaming Innovation

### The novel idea

Most LLM APIs support streaming output. The concierge can potentially:
1. Read the streaming token output of the manager and workers in near-realtime
2. Maintain a running mental model of "what's happening right now"
3. Report to the user as milestones occur, without waiting for agents to finish

This is inspired by vLLM's streaming/realtime work and Pipecat's approach to concurrent inference.

### Implementation considerations

- Requires infrastructure that exposes agent output streams (not just final results)
- Concierge context grows with every streamed token it observes — need aggressive summarization
- The concierge needs to distinguish between "agent is still thinking" and "agent has produced a result"
- Latency budget: concierge response should be <1s from user query to first token

### Fallback

If streaming observation is too complex or expensive, the concierge falls back to:
- Reading completed messages from the queue (event-driven, slight delay)
- Checking agent status fields in the registry (near-instant, less detailed)

## 8. Context Management & Garbage Collection

### Per-agent context budget

Each agent has a context window. The queue can outgrow any single agent's window. Strategies:

1. **Compacted queue summary**: A non-LLM process periodically summarizes old messages into a compressed form. All agents get the summary; only recent messages are full-text.
2. **Thread-scoped context**: Workers only load full context for threads they're in. Other threads exist as one-line summaries.
3. **External memory**: Agents write important findings/decisions to files (like Devin's note-taking). The queue is ephemeral; files are persistent.
4. **Dedicated memory agent**: A background agent (cheap model) that watches the queue and maintains a structured knowledge base. Other agents query it instead of re-reading old messages.

### Queue lifecycle

```
New message → Active queue (full text, ~last N messages)
                    ↓ (age/token threshold)
             Compacted summary (1-2 lines per message)
                    ↓ (age threshold)
             Archive (on disk, queryable but not in any agent's context)
```

## 9. Conflict Resolution

An unsolved problem across the industry. Three approaches, from simplest to most robust:

### Mechanical (git-level)
Two agents modify the same file → git merge conflict → whoever committed second must resolve. Simple, catches syntactic conflicts, misses semantic ones.

### Agent-detected
Workers monitor the queue for topic overlap. If Worker A sees Worker B posting about the same module, it flags a potential conflict to the manager before proceeding. Requires ambient awareness (the compacted queue summary).

### Manager-arbitrated
When a conflict is flagged (by git or by agents), the manager:
1. Reads both agents' recent work and reasoning
2. Decides who has the right approach (or synthesizes)
3. Directs one agent to adjust

This is expensive (manager reads a lot of context) but only triggers on actual conflicts, which should be rare if the manager's initial decomposition was good.

## 10. Cold Start & Warm-Up

The latency from "user sends request" to "first meaningful work" is:

```
User message → Concierge acknowledges (~200ms)
            → Manager reads & decomposes (~5-15s)
            → Workers provisioned & context loaded (~5-10s)
            → First worker begins execution (~20-30s total)
```

### Optimizations

1. **Warm on keystroke** (Ramp): Pre-provision worker slots when the user starts typing
2. **Concurrent decomposition**: Manager starts decomposing while concierge acknowledges
3. **Speculative workers**: For common task patterns, spawn likely workers before decomposition completes
4. **Worker pool**: Keep N warm workers with base codebase context loaded, assign tasks on demand

## 11. Open Questions

1. **Recursive depth limit**: How many levels of sub-agents before coordination overhead > value? Practical sweet spot seems to be 2-3 levels.
2. **Queue infrastructure**: In-memory (fast, volatile) vs persistent (durable, slower)? Likely need both — in-memory for active work, persistent for audit/recovery.
3. **Model heterogeneity**: Should different workers use different models based on task type? Research says yes (Cursor's finding), but adds operational complexity.
4. **Cost ceiling**: What's the acceptable token cost per user request? Need to establish this early — it constrains how many agents and how much queue context each can consume.
5. **Testing**: How do you evaluate a multi-agent system? Per-agent evals + end-to-end evals + cost tracking. The eval problem is harder than for single-agent systems.
6. **Graceful scaling**: System should work with 1 manager + 1 worker (minimum viable) and scale to 1 manager + N workers without architectural changes.
7. **Security**: If agents can spawn sub-agents and communicate freely, how do you prevent prompt injection from propagating through the swarm? One compromised agent could potentially steer others.

## 12. Relationship to Existing Work

| Pattern | Relationship |
|---------|-------------|
| Cursor planner/worker | Manager ≈ planner, but with persistent queue + concierge layer |
| EPIC (Pipecat) | Concierge ≈ voice agent (fast, never blocked). Workers ≈ UI agent. Queue ≈ shared context. |
| Cognition fleet | Workers ≈ fleet, but with dynamic roles instead of playbook-driven |
| Anthropic parallel Claudes | Similar "no inter-agent communication" for independent tasks, but this architecture adds explicit communication for interdependent work |
| Continue IDE/Cloud split | Concierge ≈ IDE agent, Workers ≈ Cloud agents |
| Blackboard systems (classic CS) | Queue ≈ blackboard, Manager ≈ control unit, Workers ≈ knowledge sources |

## 13. Recommended Implementation Order

1. **MVP**: Manager + 1 worker + concierge. Flat queue. No filtering. Prove the interaction model works.
2. **Add workers**: Dynamic spawning, recursive sub-agents. Discover where coordination breaks down.
3. **Add filtering**: Structured envelopes, tag-based routing, compacted summaries. Measure token savings.
4. **Add discovery**: Agent registry, topic-based discovery, semantic routing.
5. **Add streaming**: Concierge reads worker output streams. Measure user experience improvement.
6. **Optimize**: Fine-tune routing model, tune GC thresholds, add conflict resolution.
