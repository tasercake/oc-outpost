# Source Index

## Status Key
- 🔴 Not started
- 🟡 In progress
- 🟢 Done

## Batch 1 — Seed Articles

### Ramp
- 🟢 [Why We Built Our Background Agent](https://builders.ramp.com/post/why-we-built-our-background-agent)

### OpenAI
- 🟢 [Harness Engineering](https://openai.com/index/harness-engineering/)

### Anthropic
- 🟢 [Effective Harnesses for Long-Running Agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- 🟢 [Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- 🟢 [Demystifying Evals for AI Agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- 🟢 [Building C Compiler](https://www.anthropic.com/engineering/building-c-compiler)

### Cursor
- 🟢 [Scaling Agents](https://cursor.com/blog/scaling-agents)
- 🟢 [Self-Driving Codebases](https://cursor.com/blog/self-driving-codebases)
- 🟢 [Semantic Search](https://cursor.com/blog/semsearch)

### Replit
- 🟢 [Inside Replit's Snapshot Engine](https://blog.replit.com/inside-replits-snapshot-engine)
- 🟢 [Automated Self-Testing](https://blog.replit.com/automated-self-testing)
- 🟢 [Agent v2](https://blog.replit.com/agent-v2)
- 🟢 [Introducing Replit Agent](https://blog.replit.com/introducing-replit-agent)
- 🟢 [Securing AI-Generated Code](https://blog.replit.com/securing-ai-generated-code)

### Continue
- 🟢 [Introducing Workflows](https://blog.continue.dev/introducing-workflows-run-continuous-ai-in-the-background/) → [notes](sources/continue-workflows.md)
- 🟢 [Fight Code Slop with Continuous AI](https://blog.continue.dev/fight-code-slop-with-continuous-ai/) → [notes](sources/continue-code-slop.md)

### Sourcegraph
- 🟢 [Lessons: Context Retrieval and Evaluation](https://sourcegraph.com/blog/lessons-from-building-ai-coding-assistants-context-retrieval-and-evaluation) → [notes](sources/sourcegraph-context-retrieval.md)
- 🟢 [Lifecycle of a Code AI Completion](https://sourcegraph.com/blog/the-lifecycle-of-a-code-ai-completion) → [notes](sources/sourcegraph-completion-lifecycle.md)
- 🟢 [Sherlock: Automating Security Code Reviews](https://sourcegraph.com/blog/lessons-from-building-sherlock-automating-security-code-reviews-with-sourcegraph) → [notes](sources/sourcegraph-sherlock.md)

### Cognition (Devin)
- 🟢 [Devin Sonnet 4.5 Lessons](https://cognition.ai/blog/devin-sonnet-4-5-lessons-and-challenges) → [notes](sources/cognition-sonnet-lessons.md)
- 🟢 [Devin Annual Performance Review 2025](https://cognition.ai/blog/devin-annual-performance-review-2025) → [notes](sources/cognition-perf-review.md)
- 🟢 [Closing the Agent Loop](https://cognition.ai/blog/closing-the-agent-loop-devin-autofixes-review-comments) → [notes](sources/cognition-closing-loop.md)
- 🟢 [Agent Trace](https://cognition.ai/blog/agent-trace) → [notes](sources/cognition-agent-trace.md)
- 🟢 [Devin Review](https://cognition.ai/blog/devin-review) → [notes](sources/cognition-devin-review.md)

## Batch 2 — Discovery (from X/Twitter, HN, etc.)

### Architecture & Design Patterns
- 🟢 [Mario Zechner: What I learned building an opinionated and minimal coding agent](https://mariozechner.at/posts/2025-11-30-pi-coding-agent/) — Deep practitioner post-mortem on building pi-coding-agent from scratch; context engineering, minimal toolset philosophy
- 🟢 [Lance Martin: Agent Design Patterns (Jan 2026)](https://rlancemartin.github.io/2026/01/09/agent_design/) — Synthesis of patterns across Claude Code, Manus, Amp, Cursor: multi-layer action space, progressive disclosure, context offloading
- 🟢 [Lance Martin: Context Engineering for Agents](https://rlancemartin.github.io/2025/06/23/context_engineering/) — Framework: Write/Select/Compress/Isolate context; scratchpads, memories, failure modes
- 🟢 [Meta: Confucius Code Agent (arXiv)](https://arxiv.org/pdf/2512.10398) — Open-source scalable agent scaffold; layered memory, note-taking agent, 54.3% on SWE-Bench Pro

### Practitioner Post-Mortems
- 🟢 [James Grugett: What I learned building an AI coding agent for a year](https://jamesgrugett.com/p/what-i-learned-building-an-ai-coding) — Year-long retrospective from Manifold Markets co-founder
- 🟢 [Pragmatic Engineer: Software engineering with LLMs in 2025](https://newsletter.pragmaticengineer.com/p/software-engineering-with-llms-in-2025) — Gergely Orosz reality check; "agentic flow went from not useful to indispensable"
- 🟢 [HN: Things I learned from burning myself out with AI coding agents](https://news.ycombinator.com/item?id=46678224) — Cautionary tale and sustainability discussion

### Sandboxing & Security
- 🟢 [Docker: A New Approach for Coding Agent Safety](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/) — microVM-based isolation for coding agents
- 🟢 [NVIDIA: Practical Security for Sandboxing Agentic Workflows](https://developer.nvidia.com/blog/practical-security-guidance-for-sandboxing-agentic-workflows-and-managing-execution-risk) — Full VM isolation recommended over shared-kernel sandboxes
- 🟢 [Kubernetes SIG: agent-sandbox](https://github.com/kubernetes-sigs/agent-sandbox) — K8s controller for isolated agent runtimes
- 🟢 [Anthropic: Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) — Filesystem and network isolation for bash tool

### Benchmarks & Evals
- 🟢 [SWE-Bench Pro Leaderboard (Scale AI SEAL)](https://scale.com/leaderboard/swe_bench_pro_public) — Latest benchmark results with strict evaluation
- 🟢 [Saving SWE-Bench (arXiv)](https://arxiv.org/abs/2510.08996) — Benchmark mutation for more realistic eval; addresses contamination
- 🟢 [Stephanie Jarmak: Rethinking Coding Agent Benchmarks](https://medium.com/@steph.jarmak/rethinking-coding-agent-benchmarks-5cde3c696e4a) — Cross-benchmark evaluation and gap analysis

### Context & Retrieval
- 🟢 [Retrieval-Augmented Code Generation Survey (arXiv)](https://arxiv.org/abs/2510.04905) — Comprehensive survey of RACG for repository-level code
- 🟢 [Context Engineering for Multi-Agent Code Assistants (arXiv)](https://arxiv.org/html/2508.08322v1) — In-context code + API docs help; blind retrieval can hurt

### Company/Product Insights
- 🟢 [Augment Code: Software Agents You Can Trust](https://www.augmentcode.com/blog/software-agents-you-can-trust) — Context engine architecture, per-branch real-time indexing, adaptive learning
- 🟢 [Windsurf Cascade Architecture (DeepWiki)](https://deepwiki.com/hussainasghar/system-prompts-and-models-of-ai-tools/2.6-windsurf-agent-(cascade)) — Leaked system prompt analysis of Windsurf's agent
- 🟢 [Kevin Hou (Windsurf): How Windsurf writes 90% of your code](https://www.youtube.com/watch?v=bVNNvWq6dKo) — AI Engineer Summit 2025 talk on agentic IDE architecture

### X/Twitter Threads (snippets only)
- 🟢 [Malte Ubl: Vercel Agent code review architecture](https://x.com/cramforce/status/1970222579026927946) — CTO insight: agent sees entire repo, not just diff
- 🟢 [Andrew Ng: Agentic testing priorities](https://x.com/AndrewYNg/status/1968710001079501303) — Prioritize where to test; skip front-end, focus on backend
- 🟢 [Pat Grady: RL vs Agent Harnesses](https://x.com/gradypb/status/2011491957730918510) — Two competing paradigms for long-running agents
