# Discovery — Web Finds

Collected 2026-02-16 via web search. Focus: practitioner insights beyond the big-5 blog posts.

---

## 🔥 Top-Tier Finds

### 1. Mario Zechner — "What I learned building an opinionated and minimal coding agent"
- **URL:** https://mariozechner.at/posts/2025-11-30-pi-coding-agent/
- **Author:** Mario Zechner (libGDX creator, built Sitegeist browser-use agent)
- **Credentials:** Hands-on agent builder, years of LLM coding experience
- **Key insights:**
  - Built `pi-coding-agent` from scratch — unified LLM API, agent loop, TUI, coding CLI
  - Context engineering is paramount: "Exactly controlling what goes into the model's context yields better outputs"
  - Existing harnesses (Claude Code etc.) inject hidden context behind your back, breaking workflows
  - Minimal toolset philosophy: "if I don't need it, it won't be built"
  - No plan mode, no sub-agents, no MCP, no background bash — deliberate minimalism
  - Unified API across 4 provider APIs (OpenAI Completions, Responses, Anthropic Messages, Google GenAI)
  - Documents real provider quirks: Cerebras/xAI/Mistral incompatibilities with store field, max_tokens naming, developer role
  - YOLO mode by default (no permission prompts)
  - Structured split tool results for better context management
  - Cross-provider context handoffs
- **HN discussion:** https://news.ycombinator.com/item?id=46844822 (active, good comments on context management)
- **Quality: ⭐⭐⭐⭐⭐** — Deeply technical, from someone who actually built the thing

### 2. Lance Martin — "Agent Design Patterns" (Jan 2026)
- **URL:** https://rlancemartin.github.io/2026/01/09/agent_design/
- **Author:** Lance Martin (LangChain)
- **Credentials:** Works at LangChain, deeply embedded in agent tooling ecosystem
- **Key insights:**
  - Synthesizes patterns across Claude Code, Manus, Amp Code, Cursor Agent
  - **Give Agents a Computer:** Filesystem + shell as core primitives; "AI for your operating system"
  - **Multi-Layer Action Space:** Popular agents use surprisingly few tools (~12 for Claude Code, <20 for Manus). Push actions from tool-calling to computer via bash
  - **Progressive Disclosure:** Don't load all tool definitions upfront. Index and retrieve on demand. Cursor syncs MCP tool descriptions to a folder, agent reads only if needed
  - **Offload Context:** Write old tool results to files (Manus pattern). Apply summarization only when offloading has diminishing returns
  - CodeAct paper: agents chain actions by writing/executing code, saving tokens by not processing intermediate tool results
  - References Anthropic's "skills" standard (agentskills.io) for progressive disclosure
  - Context rot (Chroma research), context failure modes (Breunig)
  - METR: agent task length doubles every 7 months
- **Quality: ⭐⭐⭐⭐⭐** — Excellent synthesis, well-sourced

### 3. Lance Martin — "Context Engineering for Agents" (Jun 2025)
- **URL:** https://rlancemartin.github.io/2025/06/23/context_engineering/
- **Author:** Lance Martin (LangChain)
- **Key insights:**
  - Groups context engineering into 4 buckets: **Write, Select, Compress, Isolate**
  - **Write Context:** Scratchpads (persist info outside context window), long-term memories across sessions
  - **Select Context:** Retrieving relevant scratchpad/memory content back into context
  - Cognition quote: "Context engineering is effectively the #1 job of engineers building AI agents"
  - Drew Breunig's failure taxonomy: Context Poisoning, Distraction, Confusion, Clash
  - Reflexion paper: self-generated memories for re-use
  - Products with auto-generated memories: ChatGPT, Cursor, Windsurf
- **Quality: ⭐⭐⭐⭐⭐** — Foundational framework for thinking about context

### 4. James Grugett — "What I learned building an AI coding agent for a year"
- **URL:** https://jamesgrugett.com/p/what-i-learned-building-an-ai-coding
- **Author:** James Grugett (Manifold Markets co-founder)
- **Credentials:** Built a coding agent product (vibemode); hands-on for 1+ year
- **HN discussion:** https://news.ycombinator.com/item?id=44471832 (Jul 2025)
- **Quality: ⭐⭐⭐⭐** — Practitioner post-mortem (could not fetch full content)

### 5. Augment Code — "Software Agents You Can Trust"
- **URL:** https://www.augmentcode.com/blog/software-agents-you-can-trust
- **Date:** May 2025
- **Key insights:**
  - Context is "the missing ingredient" for agent quality at scale
  - Agents need: architectural consistency, code reuse, root cause analysis, end-to-end implementation, CI/CD troubleshooting
  - "For any meaningfully sized codebase, there is no hope of passing all of it as context" — must be judicious
  - 3 years of R&D on context engine: indexes entire codebase, documentation, tickets, conversations, UI designs
  - Adaptive learning: agents learn work habits over time via memories
  - Real-time index adjusts per-branch (different agents on different branches see different code)
  - Augment agent wrote majority of its own tooling integration code and tests (bootstrapping)
  - Auggie ranks #1 on SWE-Bench Pro
- **Quality: ⭐⭐⭐⭐** — From a real product company, useful architecture insights

---

## Sandboxing & Security

### 6. Docker — "A New Approach for Coding Agent Safety"
- **URL:** https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/
- **Date:** Dec 2025
- **Summary:** microVM-based isolation for Claude Code, Gemini, Codex, Kiro

### 7. NVIDIA — "Practical Security Guidance for Sandboxing Agentic Workflows"
- **URL:** https://developer.nvidia.com/blog/practical-security-guidance-for-sandboxing-agentic-workflows-and-managing-execution-risk
- **Date:** Feb 2026
- **Key insight:** Shared-kernel sandboxes (macOS Seatbelt, Bubblewrap, Docker) are insufficient for agentic code execution. Recommends full VM isolation (VMs, unikernels, Kata containers)

### 8. Kubernetes SIG — agent-sandbox
- **URL:** https://github.com/kubernetes-sigs/agent-sandbox
- **Summary:** K8s controller for isolated, stateful, singleton workloads for AI agent runtimes

### 9. INNOQ — "I sandboxed my coding agents. You should too."
- **URL:** https://www.innoq.com/en/blog/2025/12/dev-sandbox/
- **Date:** Dec 2025
- **Summary:** Practitioner walkthrough of sandboxing options. "A whole separate laptop would probably be the most secure."

### 10. Anthropic — Claude Code Sandboxing Engineering Post
- **URL:** https://www.anthropic.com/engineering/claude-code-sandboxing
- **Summary:** Filesystem and network isolation boundaries for Claude Code's bash tool

### 11. AISI (UK) — Inspect Sandboxing Toolkit
- **URL:** https://www.aisi.gov.uk/blog/the-inspect-sandboxing-toolkit-scalable-and-secure-ai-agent-evaluations
- **Summary:** Government AI safety institute's approach to sandboxed agent evaluations

### 12. awesome-sandbox (GitHub)
- **URL:** https://github.com/restyler/awesome-sandbox
- **Summary:** Curated list of code sandboxing solutions for AI

---

## Benchmarks & Evals

### 13. SWE-Bench Pro (Scale AI SEAL)
- **URL:** https://scale.com/leaderboard/swe_bench_pro_public
- **Summary:** Latest leaderboard; Resolve Rate metric with strict dual-condition evaluation

### 14. "Saving SWE-Bench" (arXiv, Oct 2025)
- **URL:** https://arxiv.org/abs/2510.08996
- **Summary:** Benchmark mutation approach for more realistic agent evaluation. Addresses contamination concerns.

### 15. Stephanie Jarmak — "Rethinking Coding Agent Benchmarks" (Jan 2026)
- **URL:** https://medium.com/@steph.jarmak/rethinking-coding-agent-benchmarks-5cde3c696e4a
- **Summary:** Evaluating agents across SWE-Bench Verified/Pro, HumanEval, DI-Bench, DependEval, RepoQA, Terminal-Bench. Designing new benchmarks where existing ones fall short.

### 16. Confucius Code Agent (Meta, arXiv)
- **URL:** https://arxiv.org/pdf/2512.10398
- **Summary:** Open-source coding agent for massive repos. 54.3% first-try fixes on SWE-Bench Pro. Layered memory, note-taking agent, failure notes for reuse.

### 17. ainativedev.io — "8 benchmarks shaping next-gen AI agents"
- **URL:** https://ainativedev.io/news/8-benchmarks-shaping-the-next-generation-of-ai-agents
- **Date:** Nov 2025
- **Summary:** Covers Context-Bench (Letta), and other emerging benchmarks

---

## Context & RAG for Code

### 18. "Retrieval-Augmented Code Generation: A Survey" (arXiv, Oct 2025)
- **URL:** https://arxiv.org/abs/2510.04905
- **Summary:** Comprehensive survey of RACG including sparse, dense, graph-based retrieval and agent-style pipelines for repository-level code generation

### 19. "Context Engineering for Multi-Agent LLM Code Assistants" (arXiv, Aug 2025)
- **URL:** https://arxiv.org/html/2508.08322v1
- **Summary:** In-context code + API docs yield significant gains; blindly retrieving similar examples can hurt performance

---

## Agent Observability & Tracing

### 20. Blaxel — "AI Observability for Coding Agents"
- **URL:** https://blaxel.ai/blog/ai-observability
- **Summary:** 360-degree visibility across agent, infrastructure, model. Observability data connected to exact sandbox state.

### 21. Arize — Agent Observability and Tracing
- **URL:** https://arize.com/ai-agents/agent-observability/
- **Summary:** Multi-agent tracing for visualizing agent interactions, delegation, and tool use in real-time

---

## Platform/Company Insights

### 22. Google Jules — Technical Overview
- **URL:** https://chandanadev.com/google-jules-ai-coding-agent
- **Key facts:** Runs on isolated Google Cloud VMs, uses Gemini 3 Pro, GitHub-native, agentic loop system. Beta: tens of thousands of tasks, 140K+ public code improvements.
- **InfoWorld comparison:** Jules performs better than Gemini CLI despite same model — the harness/scaffolding matters

### 23. Amazon Q Developer — Agentic Coding in IDE
- **URL:** https://aws.amazon.com/about-aws/whats-new/2025/05/amazon-q-developer-agentic-coding-experience-ide/
- **Key fact:** 200K-token context window

### 24. Windsurf/Codeium — Cascade Architecture
- **URL (leaked system prompt analysis):** https://deepwiki.com/hussainasghar/system-prompts-and-models-of-ai-tools/2.6-windsurf-agent-(cascade)
- **URL (Kevin Hou talk):** https://www.youtube.com/watch?v=bVNNvWq6dKo (AI Engineer Summit 2025)
- **Key concepts:** "Flow" state combining copilot + agent; full-repo awareness; real-time editor action tracking

---

## Hacker News Discussions (High Signal)

### 25. "Superpowers: How I'm using coding agents in October 2025" (Jesse Vincent)
- **URL:** https://news.ycombinator.com/item?id=45547344
- **Related:** https://github.com/obra/Superpowers, Simon Willison's notes
- **Summary:** Monthly practitioner reports on coding agent usage evolution

### 26. "What I learned building an AI coding agent for a year"
- **URL:** https://news.ycombinator.com/item?id=44471832
- **Summary:** Practitioner post-mortem discussion

### 27. "Things I learned from burning myself out with AI coding agents"
- **URL:** https://news.ycombinator.com/item?id=46678224
- **Summary:** Cautionary tale, good discussion on sustainable agent use

### 28. "Coding agents have replaced every framework I used"
- **URL:** https://news.ycombinator.com/item?id=46923543
- **Date:** Feb 2026
- **Summary:** Provocative claim with nuanced pushback in comments

---

## Other Notable Resources

### 29. Pragmatic Engineer — "Software engineering with LLMs in 2025: reality check"
- **URL:** https://newsletter.pragmaticengineer.com/p/software-engineering-with-llms-in-2025
- **Date:** Jul 2025
- **Summary:** Gergely Orosz's thorough assessment. "The agentic flow went from not useful at all to indispensable."

### 30. awesome-agentic-patterns (GitHub)
- **URL:** https://github.com/nibzard/awesome-agentic-patterns
- **Summary:** Curated catalogue of real-world agentic AI patterns, workflows, mini-architectures

### 31. Google ADK — "Developer's guide to multi-agent patterns"
- **URL:** https://developers.googleblog.com/developers-guide-to-multi-agent-patterns-in-adk/
- **Date:** Dec 2025
- **Summary:** 8 design patterns from Sequential Pipeline to Human-in-the-loop

### 32. Systems Security Foundations for Agentic Computing (ePrint)
- **URL:** https://eprint.iacr.org/2025/2173.pdf
- **Summary:** Academic treatment of security foundations for agentic systems
