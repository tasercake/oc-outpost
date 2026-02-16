# Agent Trace: Capturing the Context Graph of Code

**Source:** https://cognition.ai/blog/agent-trace

## One-line Summary
Cognition co-launched Agent Trace, an open vendor-neutral spec for recording AI contributions alongside human authorship in git—shifting the precious resource from "lines of code" to "context" by linking every code change back to the conversation that created it.

## Key Architectural Decisions

1. **Open spec, not proprietary**: Joined Cursor, Cloudflare, Vercel, Google Jules, Amp, OpenCode, git-ai. Industry-wide standard rather than lock-in.
2. **Conversation URL as primary identifier**: Each change links to a chat/conversation/trajectory URL where full context (potentially long, multimodal) lives. Avoids bloating git with prompts.
3. **PII separation by design**: Sensitive info stays in the conversation store, not in the trace. Top-level access only for compatible agents.
4. **Granularity below commits**: Traces can be associated with sub-commit changes (specific line ranges), not just full commits.
5. **Context retrieval for agents**: Agent Traces progressively expose context to agents that need it, triggered by code — not just for humans to read.

## What Worked

- **Industry coalition**: Getting Cursor, Google Jules, Vercel, Cloudflare etc. all on the same spec is a major coordination achievement.
- **Internal tooling already built**: File viewer with AI/human blame, PR-level breakdown of feature development, PR review with full development context.
- **Performance claim**: Including reasoning artifacts and prior tool calls improves SWE-Bench by ~3 points; cache hit rates improve 40-80%.

## What Failed or Was Hard

- **Still early**: Showed mock data, not real dashboards. The spec exists but ecosystem adoption is nascent.
- **Chicken-and-egg**: Value increases with adoption, but adoption requires value. Classic standards problem.

## Novel Insights

1. **"Context is the new precious resource"**: Lines of code are commodity. The value is in understanding *why* code was written. This is a paradigm shift from code-centric to context-centric development.
2. **Git was designed for bandwidth-constrained era (2005)**; we're now in a **context-constrained era**. The primitives need to change.
3. **Agent Traces improve agent performance, not just human understanding**: When agents can retrieve the context that created existing code, they make better decisions. This is a compounding feedback loop.
4. **Every agent lab independently invented conversation URLs**: Universal convergence on the same pattern suggests it's fundamental, not incidental.
5. **Foundation Capital's "Context Graphs" framing**: "A living record of decision traces stitched across entities and time so precedent becomes searchable." Agent Trace is an implementation of this.
6. **"AI Engineers will spend majority of time crafting and reading context more than code"**: This redefines what an AI engineer does.

## Practical Applications

- **Adopt Agent Trace early**: If your team's coding agent generates code, tracing provenance is critical for debugging, auditing, and improving the agent over time.
- **Context retrieval as agent capability**: your team's agent should be able to query "why was this code written this way?" from traces. This dramatically reduces re-investigation time.
- **Eng management dashboards**: Track AI vs human contributions, understand where the agent adds value, identify where it struggles. Data-driven agent improvement.
- **Video pipeline provenance**: For AI-generated video processing code, traces could track which customer requirement or video feature led to each code change.
- **Cache hit optimization**: If agent traces improve cache hit rates 40-80%, this directly reduces inference cost.

## Open Questions

- How does Agent Trace handle multi-agent scenarios where multiple agents contribute to the same code?
- What's the storage overhead of maintaining traces at scale?
- How do traces interact with code review workflows?
- Will traces become a security/compliance requirement (auditing AI-generated code)?
- How do you handle traces when code is manually edited after agent generation?

## Links to Related Work

- [Agent Trace spec](https://agent-trace.dev/)
- [Foundation Capital: Context Graphs](https://foundationcapital.com/context-graphs-ais-trillion-dollar-opportunity/)
- [OpenAI reasoning items SWE-Bench improvement](https://cookbook.openai.com/examples/responses_api/reasoning_items)
- [Latent Space: Agent Labs](https://www.latent.space/p/agent-labs)
- [Devin Review](https://cognition.ai/blog/devin-review)
