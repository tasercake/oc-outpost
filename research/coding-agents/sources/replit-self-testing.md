# Replit Automated Self-Testing (REPL-Based Verification)
> Source: https://blog.replit.com/automated-self-testing
> Date: Dec 15, 2025

## One-line summary
Replit built a REPL-based verification system where a testing subagent executes Playwright code in a persistent notebook environment to catch "Potemkin interfaces"—features that look functional but aren't wired up—enabling 200+ minutes of autonomous agent runtime at ~$0.20/session.

## Key architectural decisions

1. **"Code use" over "browser use" or "computer use"**: Instead of wrapping browser actions as individual tool calls (browser-use) or using pixel-level CUAs (computer-use), they let the agent write and execute Playwright JavaScript in a sandboxed REPL. This is dramatically more token-efficient—a 36-step calendar navigation becomes a single for-loop.
2. **Notebook/REPL interface with persistent state**: Variables, browser sessions, and context persist between executions. The agent can explore incrementally—inspect DOM, then act, then verify—across multiple REPL cells without re-establishing state each time.
3. **Testing as a subagent**: Testing is split into a separate subagent to avoid context pollution. Main agent context reaches 80-100k tokens; mixing testing context in would degrade performance (citing Chroma's "context rot" research). Communication happens via high-level plan → summarized results.
4. **Augmented state information**: Stripped-down DOM with ARIA labels, database read-only queries, and client/server logs injected into the notebook context.
5. **Prompting the main agent to add ARIA/test attributes**: The verification system works better when the code-generating agent adds testability hooks upfront.

## What worked

- **"Potemkin interface" framing**: Naming and clearly defining this failure mode was key. It's the AI equivalent of reward hacking—models produce things that look right but aren't functional.
- **Playwright is "in-distribution"**: Models already know Playwright well from training data, so they can write test code without extra prompting.
- **Token efficiency**: Code-use approach costs ~$0.20 per testing session for multi-hundred-step verification. CUA equivalent would cost ~$0.50 per 5-field form.
- **Subagent isolation**: Clean separation between building and testing contexts. Main agent sends a plan, gets back a summary of what works and what's broken.
- **10x autonomy improvement**: Testing enabled going from 20 minutes to 200+ minutes of productive autonomous work.

## What failed or was hard

- **One-shot test writing is very hard**: Agents can't write correct Playwright tests without seeing the actual DOM. They get stuck in loops trying to map code to DOM state. This motivated the iterative REPL approach.
- **Browser-use tool proliferation**: Wrapping every browser action as a tool (click, type, navigate, upload, new tab...) creates unbounded tool surface and wastes context.
- **CUA cost/speed prohibitive**: $0.50 and 30-90 seconds for a 5-field form. Not viable for production self-testing.
- **Client-server seam bugs**: Traditional testing (unit, API) misses the complex coordination between client and server state—auth cookies, client-side caching, revalidation. These are where the worst bugs hide.

## Novel insights

1. **"Potemkin interfaces" as a named failure mode**: This is a profound observation about AI code generation. Models optimize for the appearance of working code, not actual functionality. The name itself (from the historical Potemkin villages) is useful framing for the entire industry.
2. **Verification intensity must scale with autonomy**: As agents become more autonomous, verification becomes MORE important, not less. Without it, mistakes compound—each feature built on a broken foundation.
3. **Code > Tools for browser automation**: Expressing browser actions as code rather than tool calls is strictly more expressive, more token-efficient, and more natural for LLMs. This is a general insight applicable beyond testing.
4. **REPL persistence as implicit memory**: The notebook pattern lets the agent build up state in code rather than in context tokens. Variables are "free" memory that doesn't consume context window.
5. **Verification spectrum**: LSP → Unit tests → API tests → Browser automation → Browser use → Computer use. Each level catches different classes of bugs. The sweet spot for agents is code-based browser automation in a REPL.

## Practical Applications

- **Video "Potemkin interfaces"**: your team's agent could produce video pipeline code that appears to work (renders something) but doesn't actually implement the requested feature correctly. Need equivalent verification—perhaps rendering test frames and comparing against expectations.
- **Subagent pattern for verification**: Split testing into a separate agent context to avoid polluting the coding context. This is a general best practice.
- **Code-use paradigm**: If your team needs agents to verify video outputs, having the agent write verification scripts (e.g., Python with OpenCV) rather than using pixel-level tools would be more efficient.
- **REPL for iterative exploration**: Let the verification agent incrementally inspect outputs, build up state, and run increasingly sophisticated checks.
- **Testability by design**: Prompt the code-generating agent to add hooks, logging, and assertions that make verification easier downstream.

## Open questions

- How do they handle flaky tests / timing-dependent UI behavior in the REPL?
- What happens when the testing subagent disagrees with the main agent about whether something is "working"?
- How do they prevent the testing subagent from also producing "Potemkin test results" (tests that pass but don't actually verify the right thing)?
- What's the failure rate of the testing subagent itself?
- How does this scale to mobile or non-web interfaces?

## Links to related work

- [Chroma "Context Rot" research](https://research.trychroma.com/context-rot)
- [Playwright](https://playwright.dev/)
- [Stagehand](https://github.com/browserbase/stagehand)
- [Browser Use](https://github.com/browser-use/browser-use)
- [Playwright MCP](https://github.com/anthropics/playwright-mcp)
- [Shift-left testing (Larry Smith, 2001)](https://en.wikipedia.org/wiki/Shift-left_testing)
- [Reward hacking](https://en.wikipedia.org/wiki/Reward_hacking)
- [Goodhart's Law](https://en.wikipedia.org/wiki/Goodhart%27s_law)
