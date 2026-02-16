# Replit Agent v2
> Source: https://blog.replit.com/agent-v2
> Date: ~Early 2025 (launched with Claude 3.7 Sonnet)

## One-line summary
Agent v2 introduced hypothesis-driven autonomy (search before edit), loop-avoidance via backtracking, real-time design preview, and improved UI generation—launched in partnership with Anthropic's Claude 3.7 Sonnet.

## Key architectural decisions

1. **Hypothesis-driven editing**: Agent forms a hypothesis, searches for relevant files, and only starts making changes when it has enough information. This is a "read-first" pattern that reduces blind edits.
2. **Loop detection and backtracking**: v2 knows when to "step back and rethink its approach" rather than getting stuck repeatedly trying the same fix. Implies some meta-cognitive scaffolding or trajectory analysis.
3. **Real-time design preview**: Industry-first live rendering of the UI as the agent builds it. Described as "watching time-lapse photography of your idea becoming real software."
4. **Guided ideation**: Agent guides users through the ideation process, recommending steps to take—shifting from pure execution to collaborative planning.
5. **Secure vibe coding environment**: Built-in security guardrails (references their "16 ways to vibe code securely" guide).

## What worked

- **Partnership with Anthropic**: Launching alongside Claude 3.7 Sonnet gave them access to better reasoning capabilities that powered the autonomy improvements.
- **Real-time preview as differentiator**: Showing the app being built live creates a compelling UX and helps users course-correct early.
- **Read-before-write**: Dramatically reduces wasted edits and hallucinated code changes.

## What failed or was hard

- **Explicitly marked as "not yet a finished product"**: Early access / alpha quality. Speed bumps acknowledged.
- **Limited to paid plans**: Gated behind Explorer Mode on paid plans, suggesting infrastructure costs are significant.
- Article is light on technical detail—mostly a marketing announcement.

## Novel insights

1. **Hypothesis-driven agent architecture**: The "form hypothesis → search → edit" loop is a pattern that mirrors how senior developers work. Most agent architectures jump straight to editing.
2. **Real-time preview as feedback loop**: Not just a UX feature—it could serve as an additional signal for the agent itself (visual verification during generation).
3. **Loop avoidance as a first-class feature**: Recognizing and breaking out of repetitive failure loops is one of the hardest problems in autonomous agents.

## Applicable to Tavus

- **Read-before-write pattern**: Any coding agent should understand the codebase before making changes. Especially important in a complex video processing codebase.
- **Real-time preview**: For Tavus, could mean showing video output or pipeline state as the agent modifies code—instant visual feedback.
- **Loop detection**: Critical for long-running autonomous tasks. Build explicit loop detection into the agent scaffolding.

## Open questions

- What specific mechanisms do they use for loop detection? Trajectory similarity? Token pattern matching?
- How does the real-time preview work technically? Hot module reloading? Separate rendering process?
- How does the hypothesis-driven approach interact with the testing subagent from Agent 3?

## Links to related work

- [Agent v2 docs](https://docs.replit.com/replitai/agent-v2)
- [16 Ways to Vibe Code Securely](https://blog.replit.com/16-ways-to-vibe-code-securely)
- [Claude 3.7 Sonnet](https://www.anthropic.com/news/claude-3-7-sonnet)
