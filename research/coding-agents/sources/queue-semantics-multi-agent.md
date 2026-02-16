# Message Queue Semantics for Multi-Agent Systems

Research compiled 2026-02-16. Sources: web search, framework docs, blog posts, GitHub issues.

---

## 1. Queue Design Patterns in Multi-Agent AI Systems

Three dominant patterns exist for inter-agent message passing, each with distinct tradeoffs:

### A. Append-Only Shared Log (Chat History)

**How it works:** All agents read from and append to a single ordered message list. Every agent sees the full conversation history on each turn.

**Who uses it:**
- **AutoGen GroupChat** — The canonical example. All agents in a group chat share one `chat_history` list. The GroupChatManager selects the next speaker, sends a `RequestToSpeak`, and the agent publishes a `GroupChatMessage` back to the shared topic. Every agent's `_chat_history` accumulates all messages. ([AutoGen docs](https://microsoft.github.io/autogen/stable//user-guide/core-user-guide/design-patterns/group-chat.html))
- **OpenAI Swarm** — Stateless between calls; the entire `messages` array is passed on every `client.run()` call. No persistent state — every handoff must include all context the next agent needs. ([github.com/openai/swarm](https://github.com/openai/swarm))

**Tradeoffs:**
| Pro | Con |
|-----|-----|
| Simple mental model | Token cost grows O(N × M) where N = agents, M = messages |
| Full auditability & replay | Every agent pays to read irrelevant messages |
| No message loss | Context window limits become a hard ceiling |
| Natural for LLMs (chat format) | No filtering — can't subscribe to subset |

### B. Topic-Based Routing / Directed Messaging

**How it works:** Messages are routed to specific agents or groups via topics/channels. Agents only receive messages addressed to their topic.

**Who uses it:**
- **AutoGen Core (v0.4+)** — Uses `TopicId` and `TypeSubscription` for pub-sub routing. Agents can subscribe to specific topic types and only receive relevant messages. The GroupChat example uses `DefaultTopicId` for broadcast, but the API supports selective routing.
- **LangGraph** — Nodes communicate via state updates along graph edges. Each node receives only the state passed to it via its incoming edges. Uses `Command` objects to route to specific nodes with `goto="agent_name"` and can pass selective state updates. ([LangGraph docs](https://langchain-ai.github.io/langgraphjs/concepts/low_level/))
- **CrewAI** — Uses structured, schema-validated message-passing with envelope-encapsulated JSON (sender, receiver, task_id, performative, payload). Task-based routing where agents receive task outputs from their dependencies. ([emergentmind.com](https://www.emergentmind.com/topics/crewai-framework))

**Tradeoffs:**
| Pro | Con |
|-----|-----|
| Token-efficient — agents only see relevant messages | More complex orchestration logic |
| Scales better with agent count | Harder to debug (no single conversation view) |
| Natural for specialized agents | Risk of lost context if routing is wrong |
| Supports parallel execution | Requires explicit routing decisions |

### C. Pub-Sub / Event Bus

**How it works:** Agents publish events to topics; other agents subscribe to topics of interest. Decoupled — publishers don't know subscribers.

**Who uses it:**
- **AutoGen Core** — Built on pub-sub primitives. `RoutedAgent` with `TypeSubscription` and `TopicId`. Agents subscribe to message types and topics, receive only matching messages.
- **Pipecat** — Frame-based pipeline architecture. Processors in a pipeline receive frames flowing through. `ParallelPipeline` creates branches where each branch receives all upstream frames. Not traditional pub-sub but similar fan-out semantics. ([docs.pipecat.ai](https://docs.pipecat.ai/guides/learn/pipeline))
- **AgentLayer** — Infrastructure layer bridging agents across frameworks (LangChain, AutoGPT, CrewAI) with relay, bridge, and router components. ([agentlayer.link](https://agentlayer.link/))

**Tradeoffs:**
| Pro | Con |
|-----|-----|
| Maximum decoupling | Message ordering harder to guarantee |
| Easy to add/remove agents | Debugging fan-out side effects |
| Natural for event-driven architectures | Agents may miss messages if subscription is wrong |
| Scales horizontally | Overhead of subscription management |

---

## 2. Context Cost Scaling

### The Quadratic Cost Problem

The most concrete analysis comes from [exe.dev's "Expensively Quadratic" post](https://blog.exe.dev/expensively-quadratic) (Feb 2026):

- In a single-agent coding loop, **cache reads dominate cost by ~27,500 tokens** into the conversation
- By conversation end, **cache reads are 87% of total cost**
- A sample conversation cost **$12.93 total** (using Anthropic pricing)
- The cost structure is effectively **O(T × C)** where T = total tokens and C = number of LLM calls
- Anthropic pricing: input = x, cache write = 1.25x, output = 5x, cache read = x/10 (where x = $5/MTok for Opus)

### Multi-Agent Scaling

For N agents sharing a log of M messages:

```
Single agent:  Cost ∝ M²/2  (each call reads all previous messages)
N agents:      Cost ∝ N × M²/2  (each agent reads full history on each turn)
```

**Concrete numbers from AutoGen issue #1070** (GitHub): Users report that in GroupChat, every agent receives the entire conversation history when called. A user asked: "do agents receive the whole history of conversation when they are called? Is there a way to restrict the context received by an agent to the last message?" Answer: yes, they receive everything. No built-in filtering.

### Mitigation Strategies

1. **Sliding window** — Include last N messages in full + summary of older messages (common recommendation from [medium/@klaushofenbitzer](https://medium.com/@klaushofenbitzer/token-cost-trap-why-your-ai-agents-roi-breaks-at-scale-and-how-to-fix-it-4e4a9f6f5b9a))
2. **Summary compression** — AutoGen supports `summary_method='reflection_with_llm'` to compress chat history at handoff points
3. **Selective context** — LangGraph's approach: only pass relevant state fields along edges, not entire history
4. **Memory layers** — External memory (e.g., [Mem0](https://mem0.ai)) to store/retrieve cross-session context without stuffing the context window
5. **Hierarchical decomposition** — Nest group chats; inner groups share detailed context, outer groups share summaries

---

## 3. Filtering/Subscription Mechanisms from Infrastructure

### How Traditional Message Brokers Map to Agent Queues

| Broker | Pattern | Agent Relevance |
|--------|---------|----------------|
| **Apache Kafka** | Append-only log, consumer groups, topic partitions | Best for: replay, audit, event sourcing. Agents can replay history. Partitions by key enable per-user agent affinity. No per-message filtering without consumers reading everything. |
| **NATS** | Lightweight pub-sub, subject-based routing with wildcards, JetStream for persistence | Best for: low-latency agent coordination, hierarchical topic routing (`agents.team1.researcher`). Supports optimistic concurrency control (useful for event sourcing). |
| **RabbitMQ** | Queue + exchange routing (direct, topic, fanout, headers) | Best for: task distribution, complex routing rules, dead-letter queues for failed agent tasks |
| **Apache Pulsar** | Unified stream + queue semantics, multiple subscription modes | Best for: mixed workloads. Shared subscription = work queue (one agent handles each task). Exclusive = broadcast. **Key_Shared** = order per key with parallel consumers (e.g., all messages for user X go to same agent). ([javapro.io](https://javapro.io/2025/11/06/why-ai-agents-need-a-protocol-flexible-event-bus/)) |

### Key Insight: Pulsar's Dual Semantics

From the JAVAPRO article on AI agents and event buses:
> "An agent platform can use one unified event bus for everything. If you have an agent that needs to both listen to some streams and handle directed tasks, you don't have to integrate Kafka and a separate queue system."

Pulsar's subscription modes mapped to agent patterns:
- **Exclusive subscription** → Each agent sees all messages (shared log)
- **Shared subscription** → Work distribution across agent pool (task queue)
- **Key_Shared** → Per-user/per-session agent affinity with parallelism

### Event Sourcing for Agent State

Event sourcing (storing state as sequence of events) is naturally suited for agent systems:
- **Replay**: reconstruct any agent's state by replaying events
- **Branching**: fork a conversation by replaying up to point N then diverging
- **Audit**: complete history of all agent decisions
- Kafka natively supports this; NATS JetStream supports it with optimistic concurrency control

---

## 4. Existing Implementation Deep Dives

### OpenAI Swarm (now → Agents SDK)

- **Architecture**: Stateless client-side orchestration. No server state between calls.
- **Message passing**: Full `messages` array passed on every `client.run()` call via Chat Completions API
- **Handoffs**: Agent returns another Agent object from a function call. The Swarm loop switches `agent` and continues with same message history.
- **Context variables**: Mutable dict passed through the loop, accessible in agent functions. Lightweight shared state.
- **Key design choice**: "Every handoff must include all context the next agent needs—no hidden variables, no magical memory"
- **No queue**: There is no queue. It's a synchronous loop with full-history replay.
- **Replaced by**: OpenAI Agents SDK (production-ready evolution)

**Source:** [github.com/openai/swarm](https://github.com/openai/swarm), [Galileo analysis](https://galileo.ai/blog/openai-swarm-framework-multi-agents)

### LangGraph

- **Architecture**: Directed graph where nodes = agents/functions, edges = control flow. Built on Pregel-style message passing ("super-steps").
- **State management**: Typed `State` objects (typically `MessagesState` with a `messages` list). State flows along edges. Reducers merge updates (e.g., `add_messages` appends).
- **Message passing**: "When a Node completes its operation, it sends messages along one or more edges to other node(s)." Each node receives current state and returns updated state.
- **Selective routing**: `Command` objects with `goto="agent_name"` and `update={...}` for targeted state updates. Conditional edges for dynamic routing.
- **Multi-agent patterns**: Network (agents as nodes with edges), supervisor (one agent routes to others), hierarchical (nested graphs with supervisor at each level).
- **Key advantage**: Agents can see only the state fields relevant to them if you design the graph that way. Not forced into shared-everything.
- **Built-in persistence**: Checkpointing for replay, branching, time-travel debugging.

**Source:** [LangGraph glossary](https://langchain-ai.github.io/langgraphjs/concepts/low_level/), [LangChain blog](https://blog.langchain.com/langgraph-multi-agent-workflows/)

### CrewAI

- **Architecture**: Role-based agents organized into Crews with Tasks. Pure Python, standalone (no LangChain dependency as of recent versions).
- **Message passing**: Structured, schema-validated message envelopes (sender, receiver, task_id, performative, payload). Event-based architecture internally.
- **Task flow**: Sequential or hierarchical. Task outputs flow to dependent tasks. Agents receive task context, not full conversation history.
- **Communication**: "Flexible communication channels allowing agents to exchange information seamlessly" — but primarily task-output-driven, not free-form chat.
- **Process types**: Sequential (linear chain), hierarchical (manager delegates), and consensual (planned).
- **Key design**: Task-centric rather than conversation-centric. Agents communicate through task results, reducing irrelevant context.

**Source:** [docs.crewai.com](https://docs.crewai.com/en/concepts/agents), [emergentmind.com](https://www.emergentmind.com/topics/crewai-framework)

### AutoGen (Microsoft)

- **Architecture**: Event-driven agents with pub-sub messaging (Core API) or conversation-based (AgentChat API).
- **GroupChat**: Shared message log. All agents subscribe to same topic. GroupChatManager selects next speaker (round-robin or LLM-based selector). Sequential — one agent at a time.
- **Core API (v0.4)**: True pub-sub with `TopicId`, `TypeSubscription`, `RoutedAgent`. Agents subscribe to specific message types on specific topics. Much more flexible than the high-level GroupChat.
- **Cost problem**: Confirmed by GitHub issue #1070 — agents receive entire conversation history. No built-in way to restrict context to last message only. Users report high token costs in group chat sessions.
- **Mitigations**: `summary_method='reflection_with_llm'` compresses history at transitions. `max_consecutive_auto_reply` limits rounds. Token tracking built in.
- **Termination**: Max rounds, explicit "DONE" tokens, or satisfaction checks prevent infinite loops.

**Source:** [AutoGen Group Chat docs](https://microsoft.github.io/autogen/stable//user-guide/core-user-guide/design-patterns/group-chat.html), [GitHub issue #1070](https://github.com/microsoft/autogen/issues/1070)

### Pipecat

- **Architecture**: Pipeline/dataflow model for real-time voice and multimodal AI. Frame-based processing.
- **Not a message queue**: Frames flow through a pipeline of processors sequentially. More like Unix pipes than message queues.
- **ParallelPipeline**: Creates branches where each branch receives all upstream frames (fan-out).
- **Shared context**: Processors operate on the same data stream. Context is the frame flowing through, not a shared state object.
- **Multi-agent**: Multiple LLM processors can be chained in a pipeline, each adding to the conversation. But it's pipeline-sequential, not free-form multi-agent.
- **Key difference**: Optimized for real-time (ultra-low latency), not for complex multi-agent reasoning workflows.

**Source:** [docs.pipecat.ai](https://docs.pipecat.ai/guides/learn/pipeline), [github.com/pipecat-ai/pipecat](https://github.com/pipecat-ai/pipecat)

---

## 5. Synthesis: Design Decision Matrix

| Design Decision | Shared Log | Topic Routing | Pub-Sub Event Bus |
|----------------|-----------|--------------|-------------------|
| **Token cost** | O(N×M²) — worst | O(M_relevant²) — better | O(M_subscribed²) — depends on subscriptions |
| **Complexity** | Low | Medium | High |
| **Debugging** | Easy (single timeline) | Medium | Hard (distributed) |
| **Agent coupling** | Tight (all see everything) | Medium (explicit routing) | Loose (topic-based) |
| **Scalability (agents)** | Poor (linear cost per agent) | Good | Best |
| **Best for** | Small teams (2-5 agents), simple tasks | Specialized pipelines, handoffs | Large-scale, event-driven, mixed workloads |
| **Example framework** | AutoGen GroupChat, Swarm | LangGraph, CrewAI | AutoGen Core, Pulsar-backed systems |

### Recommendations by Use Case

1. **Simple multi-agent chat** (2-3 agents, short conversations): Shared log (AutoGen GroupChat / Swarm style). Cost is manageable, simplicity wins.

2. **Specialized pipeline** (each agent has distinct role, linear flow): Topic routing (LangGraph / CrewAI style). Pass only relevant state between stages.

3. **Dynamic multi-agent with scaling** (many agents, long-running, mixed broadcast + task): Pub-sub with infrastructure backing (Pulsar/NATS). Use Key_Shared for session affinity, shared subscriptions for work distribution.

4. **Cost-sensitive at scale**: Always add summarization/compression at boundaries. Never pass full history to every agent. Consider hierarchical decomposition (team-level summaries, not raw messages).

---

## Sources

- [exe.dev — "Expensively Quadratic: the LLM Agent Cost Curve"](https://blog.exe.dev/expensively-quadratic) (Feb 2026)
- [AutoGen Group Chat docs](https://microsoft.github.io/autogen/stable//user-guide/core-user-guide/design-patterns/group-chat.html)
- [AutoGen GitHub issue #1070 — cost calculation](https://github.com/microsoft/autogen/issues/1070)
- [OpenAI Swarm README](https://github.com/openai/swarm)
- [Galileo — OpenAI Swarm Framework Guide](https://galileo.ai/blog/openai-swarm-framework-multi-agents)
- [LangGraph Glossary (low-level concepts)](https://langchain-ai.github.io/langgraphjs/concepts/low_level/)
- [LangChain Blog — LangGraph Multi-Agent Workflows](https://blog.langchain.com/langgraph-multi-agent-workflows/)
- [CrewAI Framework analysis — EmergentMind](https://www.emergentmind.com/topics/crewai-framework)
- [JAVAPRO — Why AI Agents Need a Protocol-Flexible Event Bus](https://javapro.io/2025/11/06/why-ai-agents-need-a-protocol-flexible-event-bus/)
- [Pipecat Pipeline docs](https://docs.pipecat.ai/guides/learn/pipeline)
- [Mem0 — Agentic Frameworks Guide](https://mem0.ai/blog/agentic-frameworks-ai-agents)
- [Token Cost Trap (Medium)](https://medium.com/@klaushofenbitzer/token-cost-trap-why-your-ai-agents-roi-breaks-at-scale-and-how-to-fix-it-4e4a9f6f5b9a)
