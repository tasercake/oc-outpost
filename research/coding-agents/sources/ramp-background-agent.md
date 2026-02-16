# Ramp: Why We Built Our Background Agent (Inspect)

**Source:** https://builders.ramp.com/post/why-we-built-our-background-agent

## One-line summary
Ramp built "Inspect," a background coding agent running in sandboxed Modal VMs with full dev environment parity, achieving ~30% of all merged PRs within months by making sessions fast, multiplayer, and available everywhere (Slack, web, Chrome extension).

## Key architectural decisions

### Sandbox (Modal)
- Each session runs in a **sandboxed VM on Modal** with full dev environment: Vite, Postgres, Temporal, etc.
- **Image registry per repo**, rebuilt every 30 minutes via snapshot. Sessions start from the latest snapshot, so at most 30 min behind `main`.
- **Snapshot-based restore**: when agent finishes, snapshot is taken; if user sends follow-up after sandbox exits, restore from snapshot.
- Uses **Modal filesystem snapshots** for freeze/restore.

### Speed optimizations (critical insight)
- **Warm sandbox on keystroke**: start spinning up sandbox as soon as user begins typing, before they hit enter.
- **Allow reads before sync completes**: in large repos, unlikely the prompt touches files changed in last 30 min. Let agent read immediately, block writes until sync done.
- **Pool of pre-warmed sandboxes** for high-volume repos.
- **Move everything possible to build step**: even running app/tests once to generate caches.

### Agent framework
- Built on **OpenCode** (open-source coding agent) — chosen because:
  - Server-first architecture with typed SDK and plugin system
  - Agent can read OpenCode's own source code to understand behavior (underrated!)
  - Active maintainer collaboration

### API layer
- **Cloudflare Durable Objects**: each session gets its own SQLite DB. Scales to hundreds of concurrent sessions with no cross-session interference.
- **Cloudflare Agents SDK** for real-time WebSocket streaming between sandbox ↔ API ↔ clients.
- WebSocket Hibernation API keeps connections open without compute cost during idle.

### Multiplayer
- Any number of people can work in one session simultaneously. Each person's code changes are attributed to them.
- Nearly free to add if client sync is already built — just don't tie sessions to a single author.

### Authentication & PR creation
- **GitHub OAuth** for user tokens → PRs opened on behalf of the user (not the app), preventing self-approval of code.
- Sandbox pushes changes, sends event to API with branch name, API uses user's GitHub token to create PR.
- GitHub webhooks track PR lifecycle.

### Clients
1. **Slack**: Natural language (no syntax), classifier picks repo using fast model (GPT 5.2 no reasoning), virality loop as others see it used.
2. **Web**: Desktop + mobile, hosted VS Code in sandbox, streamed desktop view for visual verification, statistics page (merged PR rate, live "humans prompting" count).
3. **Chrome extension**: Visual element selection for React apps, uses DOM/React internals instead of screenshots (saves tokens), distributed via MDM policy (skip Chrome Web Store).

### Sub-agent spawning
- Agent can spawn child sessions for research or breaking large tasks into smaller PRs. "Don't be afraid of it spawning too many — frontier models contain themselves."

## What worked
- **~30% of all merged PRs** from Inspect within months, growing organically without mandating use.
- **Virality through public spaces**: Slack usage visible to others drove organic adoption.
- **Speed = adoption**: when background agents are fast, they're "strictly better than local."
- **Voice input** for casual use (e.g., describe a bug while winding down at night).
- **Follow-up queuing** (not interrupting): users can queue thoughts while agent works.

## What failed or was hard
- Not explicitly discussed — the article is more of a "build spec" than a retrospective. However:
- Implied: **repo classifier for Slack** needed tweaking initially.
- Implied: **image build optimization** required "thorough investigation" — getting full dev environments right is hard.
- The article is notably silent on failure modes, model errors, or code quality issues.

## Novel insights
1. **"Warm on keystroke"** — start sandbox provisioning before the user finishes typing. This is a UX trick that makes background agents feel as fast as local.
2. **OpenCode chosen specifically because agents can read its source** — self-legibility of tools matters when AI is the user.
3. **Chrome extension uses DOM tree, not screenshots** — dramatically reduces token usage for visual context.
4. **MDM policy distribution** for Chrome extensions — bypasses Chrome Web Store entirely.
5. **Multiplayer sessions** — not just collaboration, but enabling non-engineers (PMs, designers) to use the agent with engineering guardrails.
6. **Statistics page as growth lever** — showing merged PR rates and live usage counts drives competitive adoption.
7. **"It only has to work on your code"** — owning the tooling means you can optimize for your specific codebase, which will always beat general-purpose tools.
8. **Follow-ups are queued, not injected mid-execution** — simpler and lets users "think ahead" while agent works.

## Applicable to Tavus
- **Modal sandbox architecture** is directly replicable — Tavus could have agents with full video pipeline access (FFmpeg, ML inference runtimes, etc.).
- **Multiplayer sessions** could let product/design collaborate with engineering on video feature development.
- **Chrome extension approach** for visual verification is highly relevant — Tavus likely has React-based UIs for video configuration that could benefit from visual element selection.
- **Speed optimizations** (warm on keystroke, pre-built images) are essential for adoption — if it's slow, people won't use it.
- **Slack integration with repo classifier** is low-hanging fruit for any company with multiple repos.
- **Sub-agent spawning** for parallel approaches (try different models/prompts) is useful when exploring video processing algorithms.
- **Virality through visibility** — put the agent where people already work (Slack) and adoption follows.

## Open questions
- **Code quality**: No mention of how they ensure correctness beyond "the agent verifies its work." What's the review process? What's the defect rate?
- **Cost**: Modal VMs, Cloudflare Durable Objects, frontier model API calls — what's the per-session cost?
- **Model selection**: They mention supporting "all frontier models" but don't discuss which works best for what.
- **Failure recovery**: What happens when the agent produces bad code that passes tests? How do they handle hallucinated APIs?
- **Scale limits**: At what point does 30-min staleness become a problem (e.g., heavy merge conflicts)?
- **Security**: Full dev environment in a sandbox with access to Sentry, Datadog, LaunchDarkly — what's the blast radius if something goes wrong?

## Related work / Links
- [Modal](https://modal.com/) — cloud platform for sandboxes
- [Modal Sandboxes](https://modal.com/docs/guide/sandboxes) — instant-start sandboxes
- [Modal Sandbox Snapshots](https://modal.com/docs/guide/sandbox-snapshots) — freeze/restore
- [OpenCode](https://opencode.ai/) — open-source coding agent (server-first)
- [OpenCode SDK](https://opencode.ai/docs/sdk/)
- [OpenCode Plugins](https://opencode.ai/docs/plugins/)
- [Cloudflare Durable Objects](https://developers.cloudflare.com/durable-objects/)
- [Cloudflare Agents SDK](https://developers.cloudflare.com/agents/)
- [React Grab](https://www.react-grab.com/) — DOM inspection for React apps
- [Chrome Extension Update Server](https://developer.chrome.com/docs/extensions/how-to/distribute/host-on-linux#update)
- [code-server](https://github.com/coder/code-server) — hosted VS Code
