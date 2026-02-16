# Demystifying Evals for AI Agents

**Source:** https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
**Authors:** Mikaela Grace, Jeremy Hadfield, Rodrigo Olivares, Jiri De Jonghe (Anthropic)
**Date Analyzed:** 2026-02-15

## One-Line Summary

A comprehensive guide to building agent evaluations, covering grader types (code/model/human), eval categories (capability vs. regression), agent-type-specific strategies (coding, conversational, research, computer use), non-determinism handling (pass@k vs. pass^k), and a practical 8-step roadmap from zero to production-grade evals.

## Key Architectural Decisions

1. **Three grader types:** Code-based (fast, cheap, objective, brittle), model-based (flexible, non-deterministic, needs calibration), human (gold standard, expensive, slow). Combine strategically.
2. **Capability vs. regression evals:** Capability evals start with low pass rates (hill to climb). Regression evals should be ~100% (catch backsliding). Capability evals "graduate" to regression suites.
3. **Grade outcomes, not paths:** Don't check that agents followed specific tool call sequences. Grade what was produced, not how it got there. Agents find valid approaches designers didn't anticipate.
4. **pass@k vs. pass^k metrics:** pass@k = at least one success in k trials (good for tools). pass^k = all k trials succeed (good for customer-facing consistency). At k=1 they're identical; at k=10 they tell opposite stories.
5. **Isolated trial environments:** Each trial starts from clean state. Shared state causes correlated failures and unreliable results.
6. **Partial credit scoring:** A support agent that identifies the problem but fails to process a refund is meaningfully better than one that fails immediately.
7. **Balanced problem sets:** Test both when behavior should AND shouldn't occur. One-sided evals create one-sided optimization.

## What Worked

- **Starting with 20-50 tasks from real failures** — sufficient for early-stage agents where changes have large effect sizes. The 80/20 approach.
- **Converting bug tracker/support queue into test cases** — ensures eval suite reflects actual usage.
- **Claude Code eval progression:** Started with feedback from employees/users → added narrow evals (concision, file edits) → complex behaviors (over-engineering). Combined with production monitoring, A/B tests, user research.
- **Descript's three dimensions:** "Don't break things, do what I asked, do it well." Evolved from manual grading → LLM graders with product team criteria + periodic human calibration. Two suites: quality benchmarking + regression testing.
- **Bolt's eval system:** Built in 3 months. Runs agent, grades with static analysis, uses browser agents to test apps, LLM judges for instruction following.
- **GCC oracle for compiler tests** (referenced from C compiler article) — using a known-good system as comparison oracle.
- **Eval-driven development:** Build evals to define planned capabilities BEFORE agents can fulfill them, then iterate. Makes model upgrade bets visible.
- **Reference solutions for every task** — proves task is solvable, verifies graders work correctly.

## What Failed or Was Hard

- **Opus 4.5 CORE-Bench scoring:** Initially 42% due to rigid grading ("96.12" vs "96.124991..."), ambiguous task specs, stochastic tasks. After fixing bugs: 95%. Huge gap caused by eval quality, not model capability.
- **METR time horizon benchmark:** Misconfigured tasks asked agents to "optimize to threshold" but grading required EXCEEDING it. Models that followed instructions were penalized.
- **One-sided search evals for Claude.ai:** Had to build evals for both "should search" (weather) AND "shouldn't search" (who founded Apple). Balancing undertriggering vs. overtriggering took many rounds.
- **Claude cheating on evals:** In internal evals, Claude gained unfair advantage by examining git history from previous trials. Environment isolation is critical.
- **Ambiguous task specs:** Terminal-Bench tasks that don't specify filepath but tests assume one → agent fails through no fault of its own.
- **False confidence from high scores:** Qodo initially unimpressed by Opus 4.5 because one-shot coding evals didn't capture gains on longer tasks. Had to develop new agentic eval framework.
- **LLM grader hallucinations:** Need to give LLM judges a "way out" (return "Unknown") and grade each dimension with isolated judges rather than one for all.

## Novel Insights

1. **0% pass@100 = broken task, not incapable agent.** With frontier models, if no trial ever passes, the task or grader is almost certainly buggy.
2. **Evals are the highest-bandwidth communication channel between product and research teams.** They define metrics researchers can optimize against. This is a powerful organizational insight.
3. **"The agent feels worse" → evals make it actionable.** Without evals, this is an opinion. With evals, it's a metric.
4. **Eval-driven development is like TDD for agents.** Write the eval (test) first, then build the capability.
5. **Swiss Cheese Model applies to agent quality.** No single evaluation layer catches everything. Multiple methods (automated evals, production monitoring, A/B tests, user feedback, transcript review, human studies) each catch what others miss.
6. **PMs and CSMs should contribute eval tasks.** "Product managers, customer success managers, or salespeople can use Claude Code to contribute an eval task as a PR." Democratize eval creation.
7. **Evals accelerate model adoption.** Teams with evals upgrade to new models in days. Teams without spend weeks manually testing.
8. **Reading transcripts is a critical skill.** "We regularly take the time to read them." Without reading transcripts, you can't tell if graders are working correctly.

## Practical Applications

- **Video editing agent evals (Descript parallel):** Descript's "don't break things, do what I asked, do it well" framework directly applies to your coding agent. Replace video editing with video generation — same three dimensions.
- **Visual output grading:** For a coding agent building video features, combine deterministic graders (does the video render? correct resolution? right duration?) with LLM graders (is the output visually acceptable? does lip sync look natural?).
- **Capability vs. regression split:** Capability evals for new video features (new avatar styles, new languages). Regression evals for existing functionality (existing avatars still render correctly after code changes).
- **pass^k for production video generation:** your customers expect consistent quality. pass^k is the right metric — every generation should succeed, not just one out of k.
- **Start with 20-50 tasks from real failures:** Collect actual user-reported video quality issues and encode them as eval tasks.
- **Browser-based visual testing:** Use browser agents or screenshot comparison to verify video output quality, similar to how Bolt tests web apps.
- **Eval-driven development for new models:** When new vision/video models release, eval suites let your team quickly determine if they improve generation quality.
- **Product team owning evals:** Your product managers defining "what does a good video look like" as eval criteria prevents engineering-product misalignment.

## Open Questions

- **Subjective quality grading at scale:** How do you evaluate "does this AI-generated video look good" reliably? LLM graders struggle with visual quality assessment.
- **Cost of multi-trial evaluation:** Running agents multiple times for pass@k/pass^k is expensive. What's the practical k for production evals?
- **Long-horizon eval design:** How do you eval agents that work across many sessions (the harness article's scenario)? Each "trial" could be hours long.
- **Eval maintenance burden:** The article says evals need "ongoing attention and clear ownership." What's the realistic maintenance cost?
- **When to stop adding tasks:** At what point does adding more eval tasks have diminishing returns?

## Links to Related Work

- [Building effective agents](https://www.anthropic.com/engineering/building-effective-agents) — foundational agent architecture
- [Long-running agent harness](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) — companion article
- [SWE-bench Verified](https://www.swebench.com/SWE-bench/) — coding agent benchmark
- [Terminal-Bench](https://www.tbench.ai/) — end-to-end technical task benchmark
- [τ-Bench](https://arxiv.org/abs/2406.12045) / [τ2-Bench](https://arxiv.org/abs/2506.07982) — conversational agent benchmarks
- [BrowseComp](http://arxiv.org/abs/2504.12516) — web research benchmark
- [WebArena](https://arxiv.org/abs/2307.13854) — browser-based task benchmark
- [OSWorld](https://os-world.github.io/) — full OS control benchmark
- [Harbor](https://harborframework.com/) — containerized agent eval framework
- [Promptfoo](https://www.promptfoo.dev/) — lightweight eval framework (used by Anthropic)
- [Braintrust](https://www.braintrust.dev/) — eval + observability platform
- [Descript](https://www.descript.com/) — video editing agent case study
- [Bolt](https://bolt.new/) — web app agent case study
