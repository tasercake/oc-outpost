# Devin Review: AI to Stop Slop

**Source:** https://cognition.ai/blog/devin-review

## One-line Summary
Cognition built Devin Review, a code review tool that uses AI to intelligently organize diffs, detect bugs, and provide codebase-aware chat—addressing the bottleneck shift from code generation to code review in the age of coding agents.

## Key Architectural Decisions

1. **Intelligent diff organization**: Instead of GitHub's alphabetical file ordering, groups logically connected changes and orders hunks for top-to-bottom review. "As if a smart colleague was walking you through the PR."
2. **Move/rename detection**: Detects code that was copied/moved rather than showing as full delete + full write. Noise reduction.
3. **Inline codebase-aware chat**: Pipes diffs into an AskDevin session with full codebase understanding. Review without leaving the diff view.
4. **Tiered bug detection**: Red (probable bugs), Yellow (warnings), Gray (FYI/commentary). Addresses the "spammy and low signal" problem of existing AI review tools.
5. **Zero-friction access**: Replace `github.com` with `devinreview.com` in any PR URL. No login for public PRs. Also: `npx devin-review <url>`.

## What Worked

- **URL-swap UX is genius**: `github.com` → `devinreview.com` is frictionless adoption. No setup, no config, instant value.
- **Addresses real pain point**: "Code review, not code generation, is now the bottleneck" — validated by customer feedback.
- **Severity categorization**: Red/Yellow/Gray prevents the alert fatigue that plagues other AI review tools.
- **Free during early release**: Removes adoption barrier entirely.

## What Failed or Was Hard

- **"The great mystery of our time"**: Extreme productivity with coding agents in prototypes vs. disappointing useful output in orgs. They identify the problem but don't fully solve it—review is one piece.
- **Lazy LGTM problem**: Large PRs get rubber-stamped. AI review helps but doesn't fully solve the human incentive problem.
- **Early stage**: "This is a starting point" — feature set is minimal compared to the vision.

## Novel Insights

1. **"Never in the field of software engineering has so much code been created by so many, yet shipped to so few"** — brilliant framing of the productivity paradox. Agents create code but the shipping pipeline is bottlenecked.
2. **GitHub's PR review hasn't meaningfully changed in 15 years**: The diff-review UX is from 2011. This is a massive design space waiting to be explored.
3. **Diff ordering matters enormously**: Alphabetical file ordering is nonsensical for understanding changes. Logical grouping + explanations transforms the review experience.
4. **AI review tools fail due to being "spammy and low signal"**: The severity tiering is the key differentiator. Most AI review tools fail because they flag everything equally.
5. **The agent ecosystem creates its own demand for review tools**: More agent-generated code → more review needed → review agent becomes essential. Self-reinforcing loop.

## Applicable to Tavus

- **Build review into the agent workflow**: If Tavus's agent generates PRs, pair it with review capabilities. The write→review loop is essential for quality.
- **Diff organization as UX**: If Tavus shows agent-generated code changes to users, organizing diffs logically (not alphabetically) dramatically improves comprehension.
- **Severity tiering for any AI-generated output**: The Red/Yellow/Gray pattern applies beyond code review—any AI output validation could use confidence-based categorization.
- **Zero-friction access pattern**: The URL-swap pattern (`github.com` → `devinreview.com`) is a brilliant distribution strategy. Consider similar frictionless entry points.
- **"Review is the bottleneck" applies to video too**: As AI generates more video content/code, the quality review step becomes the constraint. Invest in review tooling.

## Open Questions

- What's the false positive rate on bug detection? (The "spammy" problem they cite for competitors)
- How deep is the codebase understanding? Full repo or just PR context?
- How does it handle PRs that span multiple concerns (refactor + feature + fix)?
- What's the path from free to paid? What's the business model?
- Can the review agent's findings feed back into improving the coding agent?

## Links to Related Work

- [GitHub's original PR diff comments (2011)](https://github.blog/news-insights/the-library/pull-request-diff-comments/)
- [Closing the Agent Loop](https://cognition.ai/blog/closing-the-agent-loop-devin-autofixes-review-comments) — auto-fixing review comments
- [Devin Review docs](https://docs.devin.ai/work-with-devin/devin-review)
