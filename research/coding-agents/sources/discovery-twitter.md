# Discovery — X/Twitter Finds

Collected 2026-02-16 via web search snippets. Full threads not fetchable — treat as pointers, not full context.

---

## High-Signal Threads

### 1. Rohan Paul (@rohanpaul_ai) — Confucius Code Agent breakdown
- **URL:** https://x.com/rohanpaul_ai/status/2000006574396321870
- **Credentials:** AI/ML content creator, summarizes papers. ⚠️ Commentator, not builder.
- **Content:** Detailed breakdown of Meta's Confucius agent architecture:
  - Orchestrator loop: code search → file edits → test → feedback
  - Layered memory with compression: goals, decisions, errors, open tasks
  - Separate note-taking agent writes failure notes for reuse across runs
  - 54.3% first-try on SWE-Bench Pro

### 2. Rohan Paul (@rohanpaul_ai) — Professional developers don't vibe code
- **URL:** https://x.com/rohanpaul_ai/status/2005472842922570157
- **Content:** References paper (13 observed sessions, 99 survey responses):
  - Professionals don't trust agents enough to stop reviewing
  - Agents struggle with design decisions
  - "Agents are treated like fast assistants, software quality still depends on human judgment"

### 3. Pat Grady (@gradypb) — RL vs Agent Harnesses
- **URL:** https://x.com/gradypb/status/2011491957730918510
- **Credentials:** Sequoia Capital partner. ⚠️ Investor, not builder.
- **Content:** "Two different technical approaches seem to both be working: reinforcement learning and agent harnesses. The former teaches a model to stay on track. The latter designs specific scaffolding around limitations (memory hand-offs, compaction, and more)."
- **Insight:** Useful framing of the two competing paradigms

### 4. Malte Ubl (@cramforce) — Vercel Agent code review architecture
- **URL:** https://x.com/cramforce/status/1970222579026927946
- **Credentials:** CTO of Vercel. ✅ Builder.
- **Content:** "Vercel Agent is good at code review because it uses a coding agent architecture where the agent sees your entire repo and can explore it similarly to Claude Code rather than just seeing the diff"

### 5. Andrew Ng (@AndrewYNg) — Agentic testing priorities
- **URL:** https://x.com/AndrewYNg/status/1968710001079501303
- **Credentials:** DeepLearning.AI founder. ✅ Builder/educator.
- **Content:** "I rarely write extensive tests for front-end code. If there's a bug, hopefully it will be easy to see. Prioritizing where to test helps make agents more reliable."

### 6. Maxime Rivest (@MaximeRivest) — Agent frameworks debate
- **URL:** https://x.com/MaximeRivest/status/1964721030636937490
- **Credentials:** Independent developer. Appears hands-on.
- **Content:** "Coding agents can—and should—be simple. An agent should be able to use Python functions as tools and maintain state." Argues both pro- and anti-framework camps are mostly wrong.

### 7. Vinh Nguyen (@vinhnx) — "Don't Build Agents, Build Skills Instead"
- **URL:** https://x.com/vinhnx/status/2000241735545565495
- **Credentials:** Developer. ⚠️ Sharing Anthropic talk, not original insight.
- **Content:** References Barry Zhang & Mahesh Murag (Anthropic) talk on skills > agents pattern. "In the past year, we've seen rapid advancement of model intelligence and convergence on agent scaffolding."

### 8. Yohei Nakajima (@yoheinakajima) — Codex early impressions
- **URL:** https://x.com/yoheinakajima/status/1923587179613388820
- **Credentials:** BabyAGI creator. ✅ Builder.
- **Content:** Practical workflow: PR from Codex → merge in GitHub → pull and run in Replit. Notes the friction of feeding errors back. "Love the parallel tasks. UI is clean, mobile works well."

### 9. Chris Wood (@C_H_Wood) — "Inside an AI Agent - System Prompts"
- **URL:** https://x.com/C_H_Wood/status/1912243933469680050
- **Credentials:** Developer/content creator. Hands-on.
- **Content:** Analysis of Claude Code system prompts, references Sourcegraph tutorial on building agents from scratch. "There's a lot of room to play around with how this stuff works."

### 10. Aaron Levie (@levie) — Coding agents as proxy for knowledge work
- **URL:** https://x.com/levie/status/2005095077518201221
- **Credentials:** Box CEO. ⚠️ Executive commentator, not builder.
- **Content:** "Skills will matter more than ever. You'll just do different stuff with them." Argues AI impact is felt most by domain experts.

### 11. Ryan Carson (@ryancarson) — "Don't outsource your thinking to the agent"
- **URL:** https://x.com/ryancarson/status/1974112545427632191
- **Content:** "There is a failure mode where you outsource the thinking and not the typing."

### 12. Fran Algaba (@franalgaba_) — Hooks as underused agent capability
- **URL:** https://x.com/franalgaba_/status/2005300393119563777
- **Credentials:** Appears to be hands-on developer.
- **Content:** "Something really under-used imo are hooks. So much potential to adapt any agent to specific needs and constraints."

---

## Signal Assessment

| Author | Role | Builder? | Signal |
|--------|------|----------|--------|
| Malte Ubl | Vercel CTO | ✅ Yes | High — architecture insight |
| Andrew Ng | DeepLearning.AI | ✅ Yes | High — practical testing wisdom |
| Yohei Nakajima | BabyAGI creator | ✅ Yes | High — workflow friction |
| Mario Zechner (via web) | libGDX/pi-agent | ✅ Yes | Highest — deep technical |
| Pat Grady | Sequoia | ❌ Investor | Medium — good framing |
| Rohan Paul | Content creator | ❌ Summarizer | Medium — good paper surfacing |
| Aaron Levie | Box CEO | ❌ Executive | Low — general commentary |
