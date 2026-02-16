# Closing the Agent Loop: Devin Autofixes Review Comments

**Source:** https://cognition.ai/blog/closing-the-agent-loop-devin-autofixes-review-comments

## One-line Summary
Cognition shipped auto-fix for PR review comments—when any GitHub bot (linter, CI, security scanner, Devin Review) flags an issue, Devin automatically fixes it, creating a write→catch→fix→merge loop that removes humans from mechanical fixes.

## Key Architectural Decisions

1. **Bot-trigger architecture**: Devin listens for GitHub bot comments on PRs and auto-responds with fixes. Generic—works with any bot that comments (linters, CI, security scanners, dependency managers).
2. **Separation of writing and reviewing agents**: One agent writes code, a different agent (Devin Review) pressure-tests it. "One agent writes, the other pressure-tests, and this continues in a loop."
3. **Configurable bot triggers**: Users choose which bots Devin should respond to in settings. Not all-or-nothing.
4. **Loop until clean**: The system iterates — write, review catches issue, fix, CI re-runs — until PR is clean for human review.

## What Worked

- **"Massively increased internal token spend"** — they can't go back, which means the quality improvement justifies the cost. Strong signal.
- **Humans reduced to judgment calls**: Architecture decisions, product direction, edge cases with domain knowledge. Everything mechanical is automated.
- **Works with existing ecosystem**: Any GitHub bot that comments. No special integration needed per-tool.
- **"Systems compound, tools don't"**: The insight that a coding agent alone is a tool, but coding agent + review agent + auto-fix is a system.

## What Failed or Was Hard

- **Token cost exploded**: "Massively increased internal token spend" — the loop burns tokens because fix attempts trigger re-reviews.
- **Gap still exists**: Running the app, clicking through flows, writing unit tests aren't yet in the loop. They acknowledge this.
- **User workflow friction**: Before this, users were copy-pasting between coding agent and review agent. The autofix closes this, but the fact it existed shows how agent interop is hard.

## Novel Insights

1. **"Code review, not code generation, is now the bottleneck"**: As agents generate more code faster, the human review bottleneck becomes the constraint. This is a key insight for the industry.
2. **Dual-agent loop is more effective than single-agent self-review**: The writing agent focuses on solving the problem; the review agent spends dedicated reasoning on the diff. Different "mindsets" for different tasks.
3. **"Even the best engineers don't catch everything on first pass"**: The argument for review isn't just about AI slop—it's about the fundamental nature of writing vs. reviewing being different cognitive tasks.
4. **The feedback loop is the product**: Not the individual agent capabilities, but the closed loop between them. This is a systems thinking insight.

## Applicable to Tavus

- **Build the review loop early**: If Tavus's coding agent generates PRs, adding automated review + autofix creates a compound system. Don't ship a write-only agent.
- **Review bottleneck applies to video pipelines too**: As AI generates more video processing code, the bottleneck shifts to validating that code works correctly. Automated testing/validation loops are essential.
- **Token cost is a real tradeoff**: The loop burns tokens. Tavus should budget for 2-5x the generation cost for review+fix iterations.
- **Ecosystem integration via bot comments**: A generic "listen for bot comments, auto-fix" pattern is reusable across any GitHub-based workflow.

## Open Questions

- What's the average number of fix iterations before a PR is clean?
- How do they prevent infinite loops (review flags issue → fix introduces new issue → review flags again)?
- What percentage of review comments are auto-fixable vs. requiring human judgment?
- How does token cost scale with PR complexity?
- When will the "running the app" and "unit test" gaps be closed?

## Links to Related Work

- [Devin Review](https://cognition.ai/blog/devin-review) — the review agent
- [devinreview.com](https://devinreview.com) — the product
