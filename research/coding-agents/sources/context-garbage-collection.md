# Context Garbage Collection & Memory Management in Multi-Agent Systems

> Research compiled 2026-02-16. Sources: MemGPT paper, Letta docs, Claude API docs, MongoDB engineering blog, LangMem docs, Meta Confucius paper, OpenClaw docs, Akka blog, various.

## The Core Problem

In a long-running system where N agents post to a shared queue, context grows unboundedly. Every message, tool result, and inter-agent communication accumulates. Without active management, you hit context limits, degrade model performance (context rot), and burn tokens re-explaining state.

MongoDB's engineering blog frames it well: "Most multi-agent AI systems fail not because agents can't communicate, but because they can't remember." Research by Cemri et al. found 40-80% failure rates in multi-agent frameworks, with 36.9% from inter-agent misalignment — agents operating on different versions of reality.

---

## 1. System-Level Context Window Management

### The Shared Resource Problem
Context windows in multi-agent systems are shared resources. Each agent's individual context fills with: system prompts, tool schemas, memory blocks, conversation history, and inter-agent messages. When agents share a queue, every agent's output becomes every other agent's input — O(N²) growth.

### Strategies

**Separation of concerns:** Don't share raw context. Each agent maintains its own context window. A coordinator/orchestrator decides what to route where. Anthropic's multi-agent researcher learned this the hard way: "Early agents made errors like spawning 50 subagents for simple queries... distracting each other with excessive updates."

**Message bus with selective consumption:** Rather than a shared queue where everyone reads everything, use pub/sub with topic-based routing. Agents subscribe to relevant topics only.

**Centralized state store + per-agent views:** A shared persistent store holds ground truth. Each agent queries only what it needs. Google ADK calls these "Artifacts" — discrete large objects stored outside the context, queried on demand. "5MB of noise in every prompt" becomes a precise, on-demand resource.

---

## 2. Summarization / Compaction Strategies

### Sliding Window
Simplest approach: keep the last K messages, drop the rest. Fast but lossy — important early context vanishes.

### Periodic Summarization
When context reaches a threshold, summarize older messages into a compact summary, keep recent messages verbatim.

**Claude API Compaction** (beta `compact-2026-01-12`):
- Monitors token usage per turn
- When input tokens exceed trigger threshold (default 150K, min 50K), generates a summary
- Creates a `compaction` block containing the summary
- All messages before the compaction block are dropped on next request
- Supports custom summarization instructions
- Can pause after compaction for inspection

**OpenClaw Compaction:**
- Auto-compaction triggers when session nears context window limit
- Summarizes older conversation, keeps recent messages intact
- Summary persisted in session's JSONL history
- **Pre-compaction memory flush**: before compacting, runs a silent turn to write durable notes to disk (prevents losing important details)
- Also has **session pruning** (separate from compaction): trims old tool results in-memory per-request
- Manual: `/compact` with optional focus instructions

### Hierarchical Memory (Recent Detail + Old Summaries)
The most sophisticated approach. Multiple tiers of compression:

1. **Recent messages** — full detail, in context window
2. **Session summaries** — compressed older conversation
3. **Cross-session knowledge** — distilled facts, preferences, patterns

This maps directly to the OS virtual memory metaphor that MemGPT pioneered.

---

## 3. Memory Architectures in Multi-Agent Systems

### MemGPT / Letta — Tiered Memory (OS-Inspired)

**Core insight:** Treat the LLM like a processor with limited registers (context window), and build a virtual memory system around it — just like an OS manages RAM vs. disk.

**Memory tiers:**
- **Main context (in-window):** System prompt, core memory blocks, recent conversation. The "registers."
- **Recall storage:** Full conversation history, searchable. The "RAM."
- **Archival storage:** Long-term knowledge base, vector-searchable. The "disk."

**Key mechanism:** The LLM itself manages memory via tool calls. It can:
- Read/write core memory blocks (always in context)
- Search recall storage (conversation history)
- Search/insert into archival storage (long-term)
- Use "heartbeats" for multi-step reasoning — request another turn to continue processing

**Memory blocks** can be shared across agents. Letta's "learned context" is written to memory blocks that multiple agents can read, enabling coordination without duplicating full context.

### LangMem (LangChain)

Three memory types modeled on human cognition:
- **Semantic memory:** Facts & knowledge (user preferences, knowledge triplets). Stored as collections or profiles.
- **Episodic memory:** Past experiences (few-shot examples, conversation summaries).
- **Procedural memory:** System behavior (personality, response patterns). Stored as prompt rules.

**Key features:**
- **Hot path** responsiveness + **background enrichment** (async memory processing)
- Memory enrichment process balances creation vs. consolidation
- Relevance = semantic similarity × importance × strength (recency/frequency)
- Prompt optimization: memories can modify the agent's system prompt over time

**Memory lifecycle:** Accept conversations + current memory state → LLM determines how to expand/consolidate → return updated state.

### Mem0

Recently integrated with OpenClaw. Automatically captures facts from conversations and recalls relevant context on every turn. Persists across restarts and compaction events.

### Agentic Memory (Jan 2026 paper, arxiv 2601.01885)
Proposes unified LTM+STM management for LLM agents, arguing existing methods handle them as separate components with heuristics. Aims for a learned, unified approach.

---

## 4. Event Sourcing Parallels

Traditional event sourcing: **append-only event log + periodic snapshots**. To reconstruct state, load latest snapshot + replay events since.

### Direct mapping to agent context:

| Event Sourcing | Agent Context |
|---|---|
| Event log | Full conversation/message history |
| Snapshot | Compaction summary / memory state |
| Replay from snapshot | Load summary + recent messages |
| Event store | Session JSONL / persistent storage |
| Projection/read model | Per-agent context view |
| CQRS (read/write separation) | Separate write (log everything) from read (selective retrieval) |

### Key insight from Akka blog:
"Agents are stateful and capable of making decisions." Event sourcing provides the durable state backbone. Agents receive context from environment (events), make decisions (commands), and produce new events.

### The compaction-as-snapshot pattern:
OpenClaw's approach is essentially event sourcing: append all messages to JSONL (event log), periodically compact (snapshot), continue from snapshot + recent events. The pre-compaction memory flush is like promoting important events to a separate "facts" store before snapshotting.

**OpenClaw's formulation** (from binds.ch analysis): "Treat the LLM context as a cache and treat disk memory as the source of truth. Then add a compactor to keep the cache bounded and a retriever to page state back in."

---

## 5. Relevance Scoring

### Per-Agent Relevance Filtering
Not every message matters to every agent. Approaches:

**Embedding-based similarity:** Embed each message + agent's current task/role. Score by cosine similarity. Only inject messages above threshold. Used by RAG systems universally.

**TF-IDF / keyword matching:** Cheaper, good for topic-based filtering. Less semantic understanding.

**LLM-based scoring:** Ask a small model "is this message relevant to agent X's current task?" Most accurate but most expensive.

**Recency × relevance × importance:** LangMem's approach. Relevance isn't just semantic similarity — also factor in how recent the memory is, how frequently it's been accessed, and its marked importance.

### Selective Context Sharing (MongoDB blog)
"Selective context sharing propagates relevant information between agents without overwhelming their individual context windows." Combined with "memory block coordination" for synchronized access to shared state.

### Google ADK Approach
"Learned context" — a sleep-time agent processes conversations offline, extracts relevant knowledge, writes it to memory blocks. At runtime, only the distilled knowledge enters context, not raw conversation.

---

## 6. Published System Approaches

### MemGPT / Letta
- **Architecture:** OS-inspired virtual memory. Main context ↔ recall storage ↔ archival storage.
- **GC mechanism:** LLM self-manages memory via tools. Decides what to keep in context, what to page out.
- **Shared memory:** Memory blocks can be shared across agents.
- **Strength:** Elegant abstraction. Agent has agency over its own memory.
- **Weakness:** Relies on LLM's judgment for memory management — can be unreliable.

### Cognition Devin
- **Architecture:** Explicit memory systems with model-driven state externalization.
- **GC mechanism:** Agent writes files (CHANGELOG.md, SUMMARY.md) to externalize state. File system as memory.
- **Key finding:** Model generates more summary tokens when context window is shorter. Useful but "not a reliable replacement for compacted memory systems."
- **Knowledge base:** Cross-session instructions/rules persist as a knowledge base component.
- **Lesson:** Explicit, structured memory systems outperform model's natural tendency to self-summarize.

### Claude Code
- **Architecture:** Linear conversation with auto-compaction.
- **GC mechanism:** Auto-compact at ~95% context usage. Summarizes older turns, keeps recent ones.
- **Manual control:** `/compact [instructions]` for targeted summarization. `Esc+Esc` or `/rewind` for partial compaction from a checkpoint.
- **Since v2.0.64:** Compaction is instant.
- **Strength:** Simple, works well for single-agent coding sessions.
- **Limitation:** No cross-session memory (relies on CLAUDE.md files for persistent context).

### Meta Confucius Code Agent (CCA)
- **Architecture:** Unified orchestrator + hierarchical working memory + persistent note-taking.
- **GC mechanism — Hierarchical Working Memory:** When conversations grow too large, an "Architect" agent summarizes earlier turns into structured plans. Compression preserves key decisions and error traces while keeping recent interactions verbatim.
- **GC mechanism — Note-Taking Agent:** Converts interaction traces into persistent Markdown notes with lightweight tags. Notes are typed memory nodes with tools to search, read, write, edit, delete, import.
- **Cross-session:** Notes persist across sessions for continual learning.
- **Meta-agent:** Automates agent configuration through build-test-improve cycles.
- **Key innovation:** Separating the "what to remember" decision into a dedicated sub-agent rather than relying on the main agent.

### OpenClaw
- **Architecture:** Session-based with JSONL history + file-based persistent memory.
- **GC mechanism:** Auto-compaction when approaching context limits. Pre-compaction memory flush writes durable notes to disk.
- **Persistent memory:** Markdown files (MEMORY.md, daily notes). Agent reads on session start.
- **Session pruning:** Trims old tool results in-memory (separate from compaction).
- **Compaction modes:** Configurable via `openclaw.json` (mode, target tokens, etc.).
- **Philosophy** (per binds.ch): "Context = cache, disk = source of truth. Compactor keeps cache bounded, retriever pages state back in."
- **Mem0 integration:** Plugin for automatic fact extraction and cross-session recall.

---

## Synthesis: Design Patterns for Context GC

### Pattern 1: Compaction-as-Snapshot (Event Sourcing)
Append all messages → periodically summarize → continue from summary + recent. Used by Claude Code, OpenClaw. Simple, effective for single-agent.

### Pattern 2: Tiered Virtual Memory (OS-Inspired)
Multiple storage tiers with explicit paging between them. Agent controls what's in-window. Used by MemGPT/Letta. Best for long-lived agents needing fine-grained memory control.

### Pattern 3: External Note-Taking (File System as Memory)
Agent writes structured notes to files. Notes survive compaction/restarts. Used by Confucius CCA, Devin, OpenClaw. Good for cross-session persistence.

### Pattern 4: Selective Routing (Pub/Sub)
Don't share everything with everyone. Route messages by relevance to each agent. Used in multi-agent orchestrators. Essential for N-agent scaling.

### Pattern 5: Background Memory Processing
Async "sleep-time" agents process conversations offline, extract knowledge, update memory stores. Used by Google ADK, LangMem. Separates real-time work from memory maintenance.

### Who Decides What's Stale?

Three approaches:
1. **Mechanical/threshold-based:** Token count triggers compaction. Simple, predictable. (Claude, OpenClaw)
2. **Agent self-management:** LLM decides what to keep/discard via memory tools. Flexible but unreliable. (MemGPT)
3. **Dedicated memory agent:** Separate agent/process responsible for memory curation. Most robust. (Confucius Architect, Google sleep-time agent, LangMem background processing)

### The Unbounded Growth Problem — Solved?

No system truly "solves" it. All approaches are lossy. The question is what to lose:
- **Sliding window:** Loses old context entirely
- **Summarization:** Loses detail, keeps gist
- **Hierarchical:** Loses detail at older tiers, keeps recent detail
- **External storage + retrieval:** Keeps everything but retrieval is imperfect

The practical answer: **hierarchical memory with multiple persistence layers.** Recent context in-window (full detail), session summaries (compacted), extracted facts in persistent storage (durable), with relevance-based retrieval to page things back in when needed.

---

## Sources

- MemGPT paper: https://arxiv.org/abs/2310.08560
- Letta docs: https://docs.letta.com
- Claude Compaction API: https://platform.claude.com/docs/en/build-with-claude/compaction
- MongoDB Memory Engineering: https://www.mongodb.com/company/blog/technical/why-multi-agent-systems-need-memory-engineering
- LangMem conceptual guide: https://langchain-ai.github.io/langmem/concepts/conceptual_guide/
- Confucius Code Agent: https://arxiv.org/abs/2512.10398
- OpenClaw compaction docs: https://docs.openclaw.ai/concepts/compaction
- OpenClaw architecture analysis: https://binds.ch/blog/openclaw-systems-analysis/
- Akka event sourcing + AI: https://akka.io/blog/event-sourcing-the-backbone-of-agentic-ai
- Agentic Memory paper: https://arxiv.org/abs/2601.01885
- Context Engineering for Agents: https://rlancemartin.github.io/2025/06/23/context_engineering/
- Cognition Devin Sonnet 4.5 blog: https://cognition.ai/blog/devin-sonnet-4-5-lessons-and-challenges
- Chroma context rot research: https://research.trychroma.com/context-rot
- Google ADK blog: https://developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/
