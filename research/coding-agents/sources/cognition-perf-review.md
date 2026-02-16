# Devin's 2025 Performance Review: Learnings From 18 Months of Agents At Work

**Source:** https://cognition.ai/blog/devin-annual-performance-review-2025

## One-line Summary
After 18 months and hundreds of thousands of merged PRs, Devin is best characterized as an infinitely-parallelizable junior engineer excelling at clear-scoped tasks, with senior-level codebase understanding but junior-level execution—and it struggles with ambiguity, scope changes, and soft skills.

## Key Architectural Decisions

1. **"Fleet of Devins" model**: Parallel execution across hundreds of repos simultaneously. Not one agent doing many things—many agents each doing one thing.
2. **Playbook-driven execution**: Humans write playbooks (migration instructions, testing guides), then a fleet executes. Separation of planning (human) from execution (agent).
3. **DeepWiki for codebase understanding**: Separate product that generates comprehensive, always-updating documentation with system diagrams. Used as context source for the coding agent.
4. **AskDevin as planning interface**: Chat-based codebase exploration for humans planning work, separate from execution.
5. **Integration-first**: Slack, Teams, Jira, GitHub — Devin lives where engineers already work.

## What Worked

- **67% PR merge rate** (up from 34% a year ago) — 2x improvement in acceptance
- **4x faster problem solving, 2x more efficient resource consumption** year-over-year
- **Security vulnerability resolution**: 20x efficiency gain vs humans (1.5 min vs 30 min per vuln)
- **Migrations at scale**: 10-14x faster than human engineers (ETL files, Java version upgrades)
- **Test coverage**: Companies go from 50-60% to 80-90% using Devin
- **Data analysis surprise**: Unexpectedly good — companies ship 3x more data features (EightSleep example)
- **Documentation at massive scale**: 400,000+ repos documented for one bank, 5M lines of COBOL, 500GB repos
- **Devin eats its own dogfood**: Pushed ~1/3 of commits on Cognition's web app

## What Failed or Was Hard

- **Ambiguous requirements**: Like a junior, needs clear specs. Can't exercise senior judgment on vague tasks.
- **Visual design**: Needs specific component structure, colors, spacing — can't make aesthetic judgments.
- **Scope changes mid-task**: Performance degrades when requirements change after task starts. Worse than human juniors at iterative coaching.
- **Non-verifiable outcomes**: When you can't programmatically check if output is correct (code review, test logic), humans must still verify.
- **"Managing Devin" is a learned skill**: Engineers must adjust their workflow to scope work better upfront. This is a real adoption cost.
- **Soft skills**: Can't manage stakeholders, mentor reports, handle emotions. (Obvious but worth noting as a real limitation of the "team member" framing.)

## Novel Insights

1. **The competency matrix doesn't map to AI**: Traditional engineering leveling fails because Devin is senior at understanding but junior at execution, with infinite capacity but no soft skills. Need new evaluation frameworks.
2. **"Junior at infinite scale" is the killer use case**: The value isn't in doing hard things—it's in doing easy things across hundreds of repos simultaneously. Volume × reliability > individual brilliance.
3. **Only 20% of engineering is coding** (citing Microsoft research): The bigger opportunity is the other 80% — planning, reviewing, understanding. Devin's codebase understanding may be more valuable than its code generation.
4. **The bottleneck shifted**: From "writing code" to "reviewing/accepting code" and "scoping work for agents." This creates new organizational roles and skills.
5. **Data analysis is an unexpected wedge**: "@Devin in Slack, ask a data question" is a simpler entry point than full coding tasks. Lower stakes, faster feedback, builds trust.

## Practical Applications

- **Fleet model for repetitive tasks**: If your team has many repos or many similar codegen tasks (e.g., generating video processing pipelines), the "playbook + fleet" pattern is directly applicable.
- **Agent as data analyst**: Your team could use an agent for internal data questions ("what were yesterday's video generations by customer?") as a quick-win entry point before tackling harder coding tasks.
- **Documentation generation**: Before the agent writes code, it should deeply understand the codebase. DeepWiki-like capabilities (auto-generated docs) could be a prerequisite feature.
- **Clear scoping matters most**: The #1 determinant of agent success is quality of task specification. Your team should invest in UX for task scoping/requirements, not just execution.
- **Merge rate as north star metric**: 67% merge rate is their primary success metric. Your team should track acceptance/merge rate of agent-generated code.
- **Scope changes are poison**: Design the agent interaction to discourage mid-task pivots. Better to cancel and restart with new specs.

## Open Questions

- What does the "fleet of Devins" architecture look like? How do they manage state, dedup, and conflicts across parallel agents?
- How does the 67% merge rate break down by task type? (Migrations likely near 100%, greenfield features likely much lower)
- What does "playbook" authoring look like in practice? Is there a structured format?
- How much of the 4x speed improvement is model improvements vs. architecture improvements?
- What's the cost per PR? ROI calculation?

## Links to Related Work

- [Microsoft Research: Developer Productivity Study](https://www.microsoft.com/en-us/research/wp-content/uploads/2024/11/Time-Warp-Developer-Productivity-Study.pdf) — only 20% of eng time is coding
- [EightSleep + Devin case study](https://cognition.ai/blog/how-eight-sleep-uses-devin-as-a-data-analyst)
- [DeepWiki](https://deepwiki.com/) — codebase documentation product
- [Devin docs: testing use case](https://docs.devin.ai/use-cases/testing-refactoring)
