# Agent Triggering Mechanisms & Token-Efficient Polling

*Research compiled 2026-02-16*

## The Core Problem

In a multi-agent system with a shared message queue, how do you decide WHEN each agent should run inference? Every approach has a cost:
- **Polling** burns tokens on empty reads (N agents × M messages = N×M inference calls)
- **Event-driven** needs a router — which is itself a decision-maker
- **Hybrid** tries to get the best of both but adds complexity

---

## 1. Polling vs Event-Driven vs Hybrid Patterns

### Pure Polling (Round-Robin / Always-On)
Every agent reads every message and decides whether to act. Simple but expensive.

- **AutoGen 0.2 GroupChat** uses this conceptually: the `GroupChatManager` runs an LLM call after every message to decide who speaks next. The speaker selection strategies are: `round_robin`, `random`, `manual`, and `auto` (LLM-based). The `auto` mode literally asks an LLM "who should speak next?" after each turn — an LLM call per message.
- **Naive Discord bots**: Every message hits every bot's handler; each bot checks if it should respond (usually via keyword/mention matching, which is ~free computationally but would be expensive with LLM inference).

### Pure Event-Driven (Trigger-Based)
Agents only activate on explicit triggers: @mentions, keywords, tool calls, or structured handoffs.

- **Discord/Slack bots**: Classic pattern. Bot registers for specific events (mentions, keywords, slash commands). Zero cost when not triggered. The "router" is the platform's event system + simple string matching.
- **LangGraph conditional edges**: After a node executes, a deterministic function examines the state and routes to the next node. No LLM call needed for routing — it's a Python function checking state fields. Example: `if state["needs_search"]: goto search_agent else: goto summarizer`.
- **Tool-calling as routing**: The currently active agent calls a tool that IS another agent. The LLM's tool-call mechanism serves as the router. LangChain/LangGraph supervisor pattern: "the supervisor can be thought of as an agent whose tools are other agents."

### Hybrid (Cheap Filter → Expensive Inference)
Use a lightweight mechanism to filter, then run full LLM inference only when needed.

- **Semantic Router** (aurelio-labs): Pre-encode example utterances per intent into embedding space. At runtime, embed the incoming message and do nearest-neighbor lookup. Routes in <1ms with no LLM call. Only the selected agent runs inference.
- **Classifier cascade**: Small model (BERT/ModernBERT) classifies intent → routes to appropriate agent. Red Hat's vLLM Semantic Router uses this: "lightweight classifier to analyze query intent and complexity, routing simple queries to smaller models."
- **Keyword + embedding hybrid**: Check keywords first (near-zero cost), fall back to embedding similarity, fall back to LLM routing only for ambiguous cases.

---

## 2. Token-Efficient Agent Activation Techniques

### Tier 0: Zero-Token Filters (Deterministic)
- **@mention / slash commands**: User explicitly addresses an agent. Cost: 0 tokens.
- **Keyword matching**: Regex or string matching on message content. Cost: 0 tokens.
- **Channel/topic routing**: Messages in #billing go to billing agent. Cost: 0 tokens.
- **Structured handoffs**: Previous agent explicitly names next agent via tool call or state flag.

### Tier 1: Cheap Compute (~0.001× cost of LLM call)
- **Embedding similarity**: Embed incoming message, compare to pre-computed route embeddings via cosine similarity. ~100μs on CPU. Libraries: `semantic-router`, Zep's intent router.
- **Small classifier**: Fine-tuned BERT/DistilBERT for intent classification. A fine-tuned Qwen 2.5-0.5B achieved ~90% routing accuracy (per Medium tutorial on LLM-as-router).
- **TF-IDF / bag-of-words**: Classical ML classifier on message text. Nearly free.

### Tier 2: Lightweight LLM Call (~0.05× cost)
- **Small model routing**: Use a tiny model (GPT-4o-mini, Claude Haiku, or local 0.5B model) just for routing decisions. Input: message + list of agent descriptions. Output: agent name.
- **LangChain's LLMRouterChain** (deprecated but instructive): Prompted LLM to return `{"destination": "agent_name"}`. Works but "can be slow and expensive since every decision is an LLM call."

### Tier 3: Full LLM Inference
- **AutoGen's `auto` speaker selection**: Full LLM call with conversation history to select next speaker. Most flexible, most expensive.
- **Supervisor agent pattern**: A coordinator agent with full context decides routing. Accurate but costs as much as running an agent.

### The Practical Hierarchy
```
Message arrives
  → Tier 0: keyword/@mention match? → route directly
  → Tier 1: embedding similarity > threshold? → route to matched agent
  → Tier 2: ambiguous? small model classifies → route
  → Tier 3: truly complex? full LLM decides → route
  → Default: no agent activates (message ignored)
```

---

## 3. The Router/Dispatcher Problem (Infinite Regress?)

**The question**: If you need something to decide which agents to wake up, isn't that just another agent? How do you avoid infinite regress?

**The answer**: The router does NOT need to be an LLM agent. The regress stops when you hit a deterministic layer.

### Levels of Router Intelligence

1. **Rules engine** (no regress): `if "billing" in message: wake billing_agent`. Finite, deterministic, zero cost. This is how Discord/Slack bots work.

2. **Embedding lookup** (no regress): Pre-computed vectors, cosine similarity, threshold. Deterministic once trained. The `semantic-router` library does exactly this.

3. **Small classifier** (no regress): A trained model that outputs probabilities over agents. Not an "agent" — no autonomy, no tool use, no conversation. Just classification.

4. **LLM-as-router** (potential regress): An LLM that decides routing IS another agent. But you can break the regress by:
   - Making it a **single-turn classifier** (no tools, no memory, no recursion)
   - Using a **much cheaper model** than the task agents
   - Having a **fixed prompt** that only outputs a routing decision

5. **Self-routing** (no separate router): Each agent receives the message with a cheap pre-prompt: "Should you handle this? Reply YES/NO." But this is just polling with a cheap filter — still N calls per message.

### LangGraph's Elegant Solution
LangGraph avoids the regress entirely for most cases: **conditional edges are Python functions, not agents**. The routing logic is:
```python
def route(state):
    if state["last_output"].tool_calls:
        return "tool_executor"
    return "end"

graph.add_conditional_edges("agent", route)
```
No LLM call. No token cost. The "router" is compiled into the graph structure.

### AutoGen's Approach (Accepts the Cost)
AutoGen's GroupChatManager explicitly uses an LLM call to select the next speaker. It accepts the token cost of the routing call as worthwhile for flexibility. The speaker selection prompt includes agent descriptions and conversation history.

---

## 4. Existing Implementations

### Discord/Slack Bot Architectures
- **Event-driven, keyword/mention triggers**
- Bot registers handlers for specific patterns; platform filters events
- Cost: 0 tokens for non-matching messages
- Limitation: Can't handle nuanced intent ("I need help with my bill" without keyword "billing")

### LangGraph (LangChain)
- **Graph-based, conditional edges**
- Nodes = agents/tools, edges = deterministic routing functions
- Supervisor pattern: supervisor agent's tool calls route to sub-agents
- "An agent is more likely to succeed on a focused task than if it has to select from dozens of tools"
- Cost model: Only the active path executes; unused agents cost nothing

### AutoGen Group Chat
- **LLM-based speaker selection**
- `speaker_selection_method`: `auto` (LLM), `round_robin`, `random`, `manual`, or custom function
- `auto` mode: sends conversation + agent descriptions to LLM, asks who should speak next
- Custom function: `def custom_speaker_selection_func(last_speaker, groupchat) -> Agent | str | None`
- `allowed_or_disallowed_speaker_transitions`: constrains which agent can follow which (reduces LLM confusion)
- In 0.4 (AG2): `SelectorGroupChat` — "uses a model to select the next speaker based on conversation context"

### Anthropic's Multi-Agent Research System
- Distributes work across agents with separate context windows
- "Token usage by itself explains 80% of the variance" in cost
- Architecture focuses on parallel execution to add capacity
- Microsoft reports multi-agent systems use **~15× tokens of single chat session**

### Semantic Router (aurelio-labs)
- **Embedding-based, zero-LLM routing**
- Pre-encode utterance examples per route
- Runtime: embed query → cosine similarity → route
- "Rather than waiting for slow LLM generations to make tool-use decisions, we use semantic vector space"
- Sub-millisecond routing decisions

### vLLM Semantic Router (Red Hat)
- **Classifier-based model routing**
- Uses ModernBERT or similar to classify query complexity
- Routes simple queries to small/fast models, complex to large/reasoning models
- "Ensuring every token generated adds value"

---

## 5. Cost Modeling

### The Naive Polling Multiplier

**Scenario**: 10 agents, shared message queue, every agent polls every message.

| Component | Per Message | Notes |
|-----------|------------|-------|
| Input tokens per agent poll | ~500-2000 | System prompt + message + context |
| Output tokens per agent poll | ~10-50 | "Not relevant" / routing decision |
| **Total per message (10 agents)** | **5K-20K input + 100-500 output** | **10× multiplier on token cost** |
| With conversation history | 10K-100K input per agent | History grows linearly |

**At scale**: If agents see 100 messages/day with ~1K tokens average context:
- Naive polling: 10 agents × 100 msgs × 1K tokens = **1M input tokens/day** just for "should I act?" decisions
- With GPT-4o at $2.50/M input tokens: **$2.50/day just for polling**
- With conversation history (10K tokens avg): **$25/day**

### Anthropic's Data Point
- Multi-agent systems use **~15× tokens** of equivalent single-agent session
- Token usage explains **80%** of cost variance
- This is for active collaboration, not idle polling — polling would be worse

### Cost Reduction by Tier

| Strategy | Tokens per routing decision | Cost relative to full LLM |
|----------|---------------------------|--------------------------|
| Keyword match | 0 | 0× |
| Embedding similarity | 0 (compute only) | ~0.001× |
| Small classifier (BERT) | 0 (compute only) | ~0.001× |
| Tiny LLM router (0.5B) | ~100 | ~0.01× |
| GPT-4o-mini routing | ~200-500 | ~0.05× |
| Full LLM routing | ~1000-5000 | 1× |

### The Break-Even Analysis
If you have N agents and M messages:
- **Naive polling cost**: N × M × (full inference cost)
- **Smart routing cost**: M × (router cost) + (activated agents) × (full inference cost)
- **Break-even**: Smart routing wins when `N × full_cost > router_cost + avg_activations × full_cost`
- With 10 agents and 1-2 activating per message: smart routing is **5-10× cheaper**

---

## 6. Key Takeaways & Recommendations

1. **Never poll with full LLM inference**. Use a tiered activation strategy.

2. **Embedding-based routing is the sweet spot** for most multi-agent systems. Near-zero cost, sub-millisecond latency, surprisingly accurate with good example utterances. Use `semantic-router` or similar.

3. **LangGraph's approach is architecturally cleanest**: routing via deterministic conditional edges (Python functions) means zero token cost for routing. Agents only run when the graph explicitly activates them.

4. **The router doesn't need to be an agent**. A classifier, embedding lookup, or rules engine breaks the infinite regress. Reserve LLM-based routing for genuinely ambiguous cases.

5. **AutoGen's LLM-based speaker selection is flexible but expensive**. Fine for small agent counts and high-value conversations. For high-throughput systems, use custom `speaker_selection_method` with deterministic logic.

6. **For our use case** (OpenClaw multi-agent with shared message queue): Consider a hybrid — keyword/mention matching first, then embedding similarity for ambiguous messages, with full LLM routing as a rare fallback. This would reduce polling cost by ~10× compared to every-agent-reads-every-message.

---

## Sources

- [AutoGen 0.2 Conversation Patterns](https://microsoft.github.io/autogen/0.2/docs/tutorial/conversation-patterns/)
- [AutoGen GroupChat Customized Speaker Selection](https://microsoft.github.io/autogen/0.2/docs/notebooks/agentchat_groupchat_customized/)
- [AutoGen SelectorGroupChat](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/selector-group-chat.html)
- [Intent Recognition and Auto-Routing in Multi-Agent Systems](https://gist.github.com/mkbctrl/a35764e99fe0c8e8c00b2358f55cd7fa)
- [LangGraph Multi-Agent Orchestration Guide (Latenode)](https://latenode.com/blog/ai-frameworks-technical-infrastructure/langgraph-multi-agent-orchestration/)
- [LangChain Multi-Agent Docs](https://docs.langchain.com/oss/python/langchain/multi-agent)
- [How Agent Handoffs Work (Towards Data Science)](https://towardsdatascience.com/how-agent-handoffs-work-in-multi-agent-systems/)
- [Semantic Router (aurelio-labs)](https://github.com/aurelio-labs/semantic-router)
- [Semantic Similarity as Intent Router (Zep)](https://blog.getzep.com/building-an-intent-router-with-langchain-and-zep/)
- [vLLM Semantic Router (Red Hat)](https://www.redhat.com/en/blog/bringing-intelligent-efficient-routing-open-source-ai-vllm-semantic-router)
- [LLM as a Router: Fine-Tuning for Intent Workflows (Medium)](https://medium.com/@vanshkhaneja/llm-as-a-router-how-to-fine-tune-models-for-intent-based-workflows-6d272eab55d1)
- [FusionRoute: Token-Level Multi-LLM Collaboration (arXiv)](https://arxiv.org/pdf/2601.05106)
- [Anthropic: How We Built Our Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Microsoft: Build Multi-Agent AI Systems (~15× token usage)](https://techcommunity.microsoft.com/blog/azuredevcommunityblog/build-multi%E2%80%91agent-ai-systems-with-microsoft/4454510)
- [AWS: Build Multi-Agent Systems with LangGraph and Bedrock](https://aws.amazon.com/blogs/machine-learning/build-multi-agent-systems-with-langgraph-and-amazon-bedrock/)
- [Managing LLM Agent Costs (APXML)](https://apxml.com/courses/multi-agent-llm-systems-design-implementation/chapter-6-system-evaluation-debugging-tuning/managing-llm-agent-costs)
