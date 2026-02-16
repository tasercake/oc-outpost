# Agent-to-Agent Communication Patterns & Addressing

*Research compiled 2026-02-16*

## The Core Question

When N agents share a communication channel (queue, bus, shared context), how do you prevent every agent from reading every message? What addressing and tagging patterns reduce noise and token waste?

---

## 1. Addressing Patterns

### @Mentions (Direct Addressing)
- **How it works:** Messages include explicit recipient identifiers — `@agent-name` or `to: agent-id`
- **Pros:** Zero ambiguity. Only addressed agents need to process the message. O(1) routing.
- **Cons:** Sender must know who to address. Breaks down when the right recipient is unknown.
- **Best for:** Known topologies, supervisor→worker delegation

### Topic Tags / Channels
- **How it works:** Messages tagged with topics (`#code-review`, `#test-results`). Agents subscribe to relevant topics.
- **Pros:** Decouples sender from receiver. New agents can subscribe without modifying senders.
- **Cons:** Topic proliferation. Agents may subscribe too broadly. Requires topic taxonomy design.
- **Best for:** Publish-subscribe architectures, loosely coupled systems

### Capability-Based Routing
- **How it works:** Messages describe what's needed (`needs: code-review`), router matches to agents that advertise that capability.
- **Pros:** Most flexible — sender doesn't need to know who or where, just what.
- **Cons:** Requires a capability registry/discovery mechanism. Matching can be fuzzy.
- **Best for:** Dynamic agent pools, Google A2A's "Agent Card" model

### Content-Based Filtering
- **How it works:** Agents define filters/predicates on message content. Only matching messages are delivered.
- **Pros:** Fine-grained. Agents get exactly what's relevant.
- **Cons:** Filter complexity. Expensive if filters are evaluated per-message. Hard to debug.
- **Best for:** High-volume event streams, ROS-style topic subscriptions

### Hybrid (Recommended)
In practice, combine: **topic channels for coarse routing** + **@mentions for direct addressing** + **capability discovery for dynamic dispatch**. This mirrors how Slack/Discord actually work (channels + mentions + integrations).

---

## 2. Agent-to-Agent Protocols

### Google A2A (Agent2Agent Protocol)
- **Launched:** April 2025 by Google, now under Linux Foundation. v0.3 (July 2025) added gRPC + signed security cards.
- **Architecture:** Client agent → HTTP/gRPC → Server (remote) agent
- **Key concepts:**
  - **Agent Card:** JSON metadata file advertising capabilities, skills, supported modalities, auth requirements. Acts as a "business card" for discovery.
  - **Task:** Unit of work with lifecycle states (submitted → working → input-required → completed → failed)
  - **Message:** Single exchange/turn in a conversation
  - **Artifact:** Output produced by task execution
  - **Part:** Content unit within messages (text, data, file references) with content-type negotiation
- **Noise reduction:** Point-to-point by design. Client discovers remote agent via Agent Card, sends task directly. No shared bus.
- **150+ supporting organizations** as of late 2025

### Anthropic MCP (Model Context Protocol)
- **Purpose:** Standardizes how AI applications connect to external tools, APIs, and data sources
- **Complementary to A2A:** MCP = agent↔tools, A2A = agent↔agent
- **Example:** Inventory agent uses MCP to query database, then A2A to notify supplier agent

### IBM ACP (Agent Communication Protocol) / BeeAI
- **Similar goals to A2A** — open standard for agent interoperability
- **Part of IBM's BeeAI framework**

### Key Insight: MCP + A2A Together
A2A handles inter-agent communication. MCP handles agent-to-tool communication. They're complementary layers, not competitors. A well-designed system uses both.

---

## 3. Noise Reduction Strategies

The fundamental problem: with N agents and M messages, naively every agent reads every message = O(N×M) token consumption.

### Strategy 1: Topic-Based Fan-Out Control
- Agents subscribe only to relevant topic channels
- Messages are routed only to subscribers
- **Reduction:** From O(N×M) to O(subscribers×M_relevant)

### Strategy 2: Interest-Based Filtering (Pub-Sub)
- "Publish-subscribe systems reduce noise through filtering but may miss relevant information" (Lyu 2025)
- Trade-off: too aggressive filtering → missed context; too loose → noise

### Strategy 3: Supervisor/Router Pattern
- A lightweight orchestrator reads all messages, dispatches only to relevant agents
- Orchestrator can be a cheap/fast model or rule-based router
- **Reduction:** Only orchestrator reads all; workers read only assigned tasks

### Strategy 4: Structured Envelopes
- Every message has a machine-readable envelope (to, from, topic, type, priority)
- Agents parse envelope only, skip body if not relevant
- **Reduction:** Envelope parsing is ~20 tokens vs full message body of 500+

### Strategy 5: Digest/Summary Pattern
- Instead of N agents reading raw messages, a summarizer agent periodically compiles digests
- Agents read compressed summaries
- **Reduction:** Amortizes token cost across time

### Strategy 6: Pull vs Push
- **Push (broadcast):** Messages delivered to all subscribers → higher noise
- **Pull (polling):** Agents query for messages matching their criteria → lower noise, higher latency
- **Best:** Push with topic filtering (push relevant, skip irrelevant)

---

## 4. Structured vs Unstructured Messages

### Structured (JSON/Schema)
```json
{
  "from": "code-agent",
  "to": "review-agent",
  "type": "review_request",
  "topic": "code-review",
  "payload": {
    "file": "src/auth.py",
    "diff_summary": "Added OAuth2 flow",
    "priority": "high"
  }
}
```
**Pros:**
- Machine-parseable envelope enables filtering without reading body
- Type field enables routing without content inspection
- Schema validation catches malformed messages
- Deterministic parsing — no LLM call needed to understand structure

**Cons:**
- Rigid — adding new message types requires schema updates
- Verbose for simple exchanges
- Agents must agree on schema upfront

### Unstructured (Natural Language)
```
@review-agent Please review the OAuth2 changes in src/auth.py. High priority.
```
**Pros:**
- Flexible — any agent can understand any message (if it has an LLM)
- Low coordination cost — no schema design needed
- Natural for LLM agents (it's their native format)

**Cons:**
- Requires LLM call to parse → expensive at scale
- Ambiguous — "high priority" means different things to different agents
- No machine filtering — every agent must read and interpret every message

### Recommended: Structured Envelope + Unstructured Body
```json
{
  "from": "code-agent",
  "to": "review-agent", 
  "type": "review_request",
  "topic": "code-review",
  "priority": "high",
  "body": "Please review the OAuth2 changes in src/auth.py. I added the authorization code flow with PKCE."
}
```
- Envelope is machine-parseable (routing, filtering, prioritization)
- Body is natural language (flexibility, LLM-native)
- Agents can filter on envelope fields without reading body
- **This is essentially how email works** (headers + body), and it's proven at scale

---

## 5. Real-World Communication Patterns

### Slack/Discord (Human-Scale, Proven Patterns)
| Pattern | Mechanism | Agent Analogy |
|---------|-----------|---------------|
| Channels | Topic-based grouping | Topic subscriptions |
| Threads | Scoped sub-conversations | Task-scoped context (reduces noise for non-participants) |
| @mentions | Direct addressing | Point-to-point messages |
| Reactions | Lightweight acknowledgment | Status signals without message overhead |
| Channel muting | Interest-based filtering | Agent unsubscription |

**Key insight:** Threads are the killer feature for noise reduction. A 50-message thread generates 1 notification for non-participants vs 50 in a flat channel. **Agent systems should use task-scoped threads, not flat queues.**

### Microservice Architectures (Service Mesh / Message Brokers)
| Pattern | Examples | Agent Analogy |
|---------|----------|---------------|
| Message queues | RabbitMQ, SQS | Task queues with competing consumers |
| Pub-sub topics | Kafka, SNS | Interest-based routing |
| Service mesh | Istio, Envoy | Capability-based routing with sidecar proxies |
| Dead letter queues | DLQ | Failed task handling |
| Content-based routing | Apache Camel | Message body inspection for routing |

**Key patterns that transfer:**
- **Competing consumers:** Multiple agents subscribe to same queue, each message processed by exactly one. Prevents duplicate work.
- **Fan-out/fan-in:** One message triggers N parallel workers, results aggregated. Map-reduce for agents.
- **Circuit breaker:** Stop sending to agents that are failing/slow. Prevents cascade failures.
- **Back-pressure:** Slow consumers signal producers to throttle. Prevents queue overflow.

### ROS (Robot Operating System) Topics
- **Topics:** Named buses for typed messages (e.g., `/camera/image`, `/motor/velocity`)
- **Nodes subscribe** to topics they care about
- **Message types** are strictly defined (like protobuf schemas)
- **QoS profiles:** reliability, durability, deadline, liveliness — agents can specify delivery guarantees
- **Key insight:** ROS separates the communication graph from the computation graph. Any node can publish/subscribe to any topic. This decoupling is powerful for dynamic agent systems.

### Actor Model (Erlang/Akka)
- **Mailbox per actor:** Each actor has its own message queue. Messages are point-to-point.
- **PID addressing:** Every actor has a unique process ID. Messages are sent to specific PIDs.
- **Pattern matching on receive:** Actors can selectively process messages from their mailbox based on pattern matching. Unmatched messages stay in queue.
- **No shared state:** All communication is via message passing. No shared memory.
- **Supervision trees:** Parent actors manage child actor lifecycle. Failed actors are restarted by supervisors.

**Key insight for agent systems:** The actor model's **mailbox + selective receive** is ideal. Each agent has a private inbox. Messages are addressed to specific agents. Agents can pattern-match on message type to process in priority order. This is the cleanest model for noise reduction — you literally cannot receive messages not addressed to you.

**Why actor model > shared queue for agents:**
- Shared queue: N agents × M messages = O(NM) reads
- Actor mailbox: Each message read exactly once by its recipient = O(M) reads total

---

## 6. Shared Context Observation vs Explicit Messaging

### The Two Paradigms

**Shared Context / Blackboard Pattern:**
- All agents observe a shared state (document, database, scratchpad)
- Agents notice changes relevant to them and react
- Communication is implicit — through state changes
- Examples: Google Docs-style collaboration, shared memory in multi-agent systems, Pipecat's frame pipeline

**Explicit Messaging:**
- Agents send directed messages to each other
- Communication is explicit — sender actively chooses recipient
- Examples: A2A protocol, actor mailboxes, microservice API calls

### When Each Works

| Dimension | Shared Context | Explicit Messaging |
|-----------|---------------|-------------------|
| **Coordination overhead** | Low (just read state) | Higher (must address, route) |
| **Token cost** | High if context is large (every agent reads full state) | Low (only relevant messages) |
| **Coupling** | Loose (agents don't know about each other) | Tighter (sender must know recipient) |
| **Scalability** | Poor — O(N × state_size) per observation | Good — O(messages_per_agent) |
| **Emergent behavior** | Natural (agents react to patterns) | Must be designed |
| **Debugging** | Hard (who changed what?) | Easier (explicit message trail) |
| **Best N agents** | 2-5 | 5-100+ |

### The Pipecat Model: Frame Pipelines
Pipecat uses a **stream-of-frames** architecture where data flows through processors in a pipeline. Each processor observes and transforms frames relevant to it:
- AudioFrame, TextFrame, etc. are typed
- Processors only handle frame types they care about
- This is effectively **content-type-based filtering on a shared stream**

This works well for **linear pipelines** (audio → STT → LLM → TTS → audio) but doesn't map directly to multi-agent collaboration where agents need peer-to-peer communication.

### Hybrid Recommendation
- **Small teams (2-5 agents):** Shared context works. Token cost is manageable. Low coordination overhead wins.
- **Medium teams (5-20 agents):** Shared context + topic-scoped views. Each agent sees a filtered projection of shared state.
- **Large systems (20+ agents):** Explicit messaging with capability discovery. Shared context doesn't scale.
- **Mixed:** Shared context for **state** (what's true now) + explicit messages for **events** (what just happened / what needs to happen).

---

## 7. Synthesis: Recommended Pattern for Agent Communication

For a system like OpenClaw where agents communicate via a shared mechanism:

### Architecture
```
┌─────────────────────────────────────────────┐
│              Message Bus / Queue              │
│  (topics: #tasks, #results, #status, ...)    │
├─────────────────────────────────────────────┤
│  Envelope: { from, to, type, topic, priority}│
│  Body: natural language or structured payload │
└──────────┬──────────┬──────────┬────────────┘
           │          │          │
      ┌────▼───┐ ┌────▼───┐ ┌───▼────┐
      │Agent A │ │Agent B │ │Agent C │
      │sub: #X │ │sub: #Y │ │sub: #X,Y│
      └────────┘ └────────┘ └────────┘
```

### Rules
1. **Structured envelope, unstructured body** — filter on envelope, read body only when relevant
2. **Topic channels** for coarse routing — agents subscribe to what they care about  
3. **@mentions** for direct addressing within channels
4. **Task-scoped threads** — conversation about task X stays in thread X, doesn't pollute the main channel
5. **Competing consumers** for task queues — one agent picks up each task, no duplicate processing
6. **Capability-based discovery** for dynamic routing — agents advertise what they can do (à la A2A Agent Cards)
7. **Supervisor pattern** for orchestration — cheap router reads all, dispatches to specialists

### Token Budget Impact
| Pattern | Token Cost per Message | Notes |
|---------|----------------------|-------|
| Broadcast to all | N × message_tokens | Worst case |
| Topic-filtered | subscribers × message_tokens | 2-5x better |
| Envelope-only filtering | N × ~20 tokens + 1 × message_tokens | 10-50x better |
| Direct addressing | 1 × message_tokens | Optimal |
| Supervisor routing | 1 × (envelope + summary) + 1 × message_tokens | Near-optimal |

---

## Sources

1. Google A2A Protocol — https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/
2. A2A v0.3 Update — https://cloud.google.com/blog/products/ai-machine-learning/agent2agent-protocol-is-getting-an-upgrade
3. IBM on A2A — https://www.ibm.com/think/topics/agent2agent-protocol
4. A2A GitHub — https://github.com/a2aproject/A2A
5. LLMs for Multi-Agent Cooperation (Lyu 2025) — https://xue-guang.com/post/llm-marl/
6. Communication Topologies in LLM-MAS (Shen et al. 2025) — https://arxiv.org/abs/2505.23352
7. Communication Protocols for LLM Agents (APXML) — https://apxml.com/courses/agentic-llm-memory-architectures/chapter-5-multi-agent-systems/communication-protocols-llm-agents
8. AI Agents Meet Real-Time Data (StreamNative 2025) — https://streamnative.io/blog/ai-agents-real-time-data-bridge
9. Pipecat Framework — https://github.com/pipecat-ai/pipecat
10. A2A Adoption Guide — https://www.apono.io/blog/what-is-agent2agent-a2a-protocol-and-how-to-adopt-it/
