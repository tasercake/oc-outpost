# Continue: Introducing Agents (Workflows)

**Source:** https://blog.continue.dev/introducing-workflows-run-continuous-ai-in-the-background/

## One-line summary
Continue ships cloud-based "Agents" that run asynchronously in the background, shifting AI coding from request-response to continuous, event-driven workflows across the development lifecycle.

## Key architectural decisions
- **IDE agents vs. Cloud agents** — clear separation: IDE agents are synchronous/interactive ("copilot"), cloud agents are async/autonomous ("background worker"). This mirrors the local dev vs CI distinction.
- **CLI-first testing** — agents share the same runtime as the Continue CLI TUI mode, so you can test prompts locally before deploying to cloud. Reduces the feedback loop.
- **MCP integration** — agents can use MCP servers, making them composable with arbitrary tool integrations (security scanning, docs, etc.).
- **GitHub-centric output** — agents produce PRs as their primary artifact, fitting into existing code review workflows.

## What worked
- Reusing the CLI runtime for both local and cloud agents — single codebase, consistent behavior.
- Starting with simple, well-scoped tasks (fix a bug, write boilerplate) builds user confidence before scaling ambition.
- The "invest in thorough prompts" guidance acknowledges that agent quality is prompt-quality-bound.

## What failed or was hard
- Article is light on failures — it's a launch post. But they implicitly acknowledge:
  - Agents "might not succeed on the first try" — reliability is not yet high.
  - Higher PR volume from agents strains existing code review practices. Teams must adapt.
  - The "embrace the workflow shift" language suggests organizational friction is real.

## Novel insights
- **"The future of development is maintaining prompts, managing agents, and orchestrating AI workflows"** — reframes the developer role from code-writer to agent-orchestrator.
- **Continuous AI as a paradigm** — AI shouldn't just respond to queries; it should run continuously like CI/CD. Tests run in background, why not AI?
- **PR volume as a scaling problem** — when agents produce many PRs, the bottleneck shifts from code generation to code review. This is under-discussed elsewhere.

## Applicable to Tavus
- **Background agents for video pipeline QA** — continuous agents could monitor video generation outputs, flag quality regressions, or auto-fix template issues.
- **Prompt-as-config pattern** — if Tavus builds a coding agent, the idea of testing prompts locally (CLI) before deploying to production is directly applicable.
- **Agent orchestration** — multiple specialized agents (security, quality, docs) running in parallel on every PR is a pattern worth adopting.
- **Review bottleneck** — Tavus should plan for how humans review agent-generated code at scale.

## Open questions
- How do they handle agent failures? Retry logic? Human escalation?
- What's the cost model for cloud agents running "continuously"?
- How do they prevent agents from conflicting with each other (e.g., two agents editing the same file)?
- What observability/tracing exists for debugging agent runs?

## Links to related work
- [Continue CLI docs](https://docs.continue.dev/guides/cli)
- [MCP Cookbooks](https://docs.continue.dev/guides/overview#mcp-integration-cookbooks)
- [Continuous AI guide](https://docs.continue.dev/guides/continuous-ai)
- [Hub agents](https://hub.continue.dev/agents)
