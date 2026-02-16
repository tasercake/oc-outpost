# Building a C Compiler with a Team of Parallel Claudes

**Source:** https://www.anthropic.com/engineering/building-c-compiler
**Author:** Nicholas Carlini (Anthropic Safeguards team)
**Date Analyzed:** 2026-02-15

## One-Line Summary

16 parallel Claude instances autonomously built a 100,000-line Rust-based C compiler capable of compiling the Linux kernel, using a bare-bones agent loop with git-based synchronization, task locking, and high-quality test suites — costing ~$20,000 over 2,000 sessions across two weeks.

## Key Architectural Decisions

1. **Infinite agent loop:** Simple bash `while true` loop that restarts Claude Code after each session. No orchestration agent. Each instance decides what to work on.
2. **Parallel agents via Docker containers:** Bare git repo as upstream. Each agent clones to `/workspace`, works, pushes back. 16 agents running simultaneously.
3. **File-based task locking:** Agents write to `current_tasks/` to claim work (e.g., `parse_if_statement.txt`). Git synchronization forces second agent to pick different task on conflict.
4. **No inter-agent communication:** No orchestrator, no message passing. Each agent independently picks "the next most obvious" problem. Emergent coordination.
5. **Specialized agent roles:** Some agents dedicated to: deduplicating code, performance optimization, output code efficiency, Rust code quality critique, documentation.
6. **GCC as oracle compiler:** For the Linux kernel (one giant task, not parallelizable), randomly compile most files with GCC, remaining with Claude's compiler. Binary search to find which files cause failures.
7. **Clean-room implementation:** No internet access during development. Depends only on Rust standard library.

## What Worked

- **Scale of output:** 100,000 lines of Rust, compiles Linux 6.9 on x86/ARM/RISC-V, plus QEMU, FFmpeg, SQLite, PostgreSQL, Redis. 99% pass rate on GCC torture tests. Can compile and run Doom.
- **Parallelism for independent tests:** When hundreds of distinct failing tests exist, parallelization is trivial — each agent picks a different test.
- **GCC oracle technique:** Brilliant solution for the "one giant task" problem. Random subsetting + binary search isolates which files cause kernel compilation failures.
- **Delta debugging:** After GCC oracle found individual file bugs, delta debugging found pairs of files that failed together but worked independently.
- **Agent-maintained documentation:** Agents instructed to maintain READMEs and progress files. When stuck on bugs, agents spontaneously maintained running docs of failed approaches.
- **Specialized roles:** Code deduplication agent prevented the LLM tendency to re-implement existing functionality. Quality critique agent improved overall code structure.
- **Deterministic fast test subset:** `--fast` flag runs 1-10% random sample, deterministic per-agent but random across VMs. Each agent covers all files but can identify regressions quickly.
- **Cost efficiency:** $20,000 total vs. what it would cost a human team to write a C compiler from scratch. Orders of magnitude cheaper.

## What Failed or Was Hard

- **Linux kernel as one task broke parallelism:** All 16 agents hit same bug, fixed it, overwrote each other. 16 agents = no benefit when task isn't decomposable.
- **New features breaking existing functionality:** Near end of project, every new feature frequently broke existing code. Had to build CI pipeline with stricter enforcement.
- **16-bit x86 code generator:** Opus couldn't implement it. Output was 60KB, far exceeding Linux's 32K code limit. Had to fall back to GCC for real-mode boot.
- **Generated code efficiency:** Even with all optimizations, outputs less efficient code than GCC with ALL optimizations disabled.
- **Rust code quality:** "Reasonable" but "nowhere near the quality of what an expert Rust programmer might produce."
- **Agent self-termination:** One instance accidentally ran `pkill -9 bash`, killing itself and the loop. Humorous but real.
- **Context window pollution from tests:** Test harness must NOT print thousands of useless bytes. Must limit output, log to files, pre-compute summaries.
- **Time blindness:** Claude can't tell time. Left alone, will spend hours running tests instead of making progress. Had to print progress infrequently and provide fast test subsets.
- **Not a drop-in compiler replacement:** Many projects still don't compile. Assembler and linker still buggy.

## Novel Insights

1. **No orchestrator needed (at this scale):** Emergent coordination via git locking + independent decision-making worked for 16 agents. This is counterintuitive — most multi-agent papers assume you need a coordinator.
2. **The oracle compiler pattern is generalizable:** Use a known-good reference implementation to create parallelizable test decomposition for any "one big task." Applicable to any domain with a reference oracle.
3. **Test quality > agent quality for autonomous work:** "Most of my effort went into designing the environment around Claude — the tests, the environment, the feedback." The harness IS the product.
4. **Design for Claude, not for humans:** Test output formatting, error messages (`ERROR` keyword on same line for grep), log structure — all optimized for LLM consumption, not human readability.
5. **LLMs re-implement existing functionality constantly:** This is a fundamental behavioral pattern, not a bug. Dedicate a specialized agent to combat it.
6. **Capability benchmarking by pushing to limits:** "The best way to understand what language models can do is to push them to their limits, and then study where they start to break down."
7. **Cost curve is dramatic:** $20K for a C compiler that would cost a team of engineers months/years. Even if quality is lower, the speed/cost tradeoff is unprecedented.
8. **Agent teams change the ambition level:** "Allows us, as users of these tools, to become more ambitious with our goals." This isn't incremental — it's a phase change in what's achievable.

## Applicable to Tavus

- **Parallel agents for video pipeline components:** Different agents working on different video processing stages simultaneously — one on avatar rendering, one on audio processing, one on encoding. Git-based coordination.
- **Oracle pattern for video quality:** Use an existing high-quality renderer as oracle. Compare Claude's output against reference. Binary search to find which code changes degrade quality.
- **Test harness design for video agents:** Tavus's test output should be optimized for LLM consumption — not dumping raw video processing logs but summarized metrics (frame quality scores, sync offsets, render times).
- **Specialized agent roles:** Dedicated agents for: code quality, documentation, performance optimization, test writing. The deduplication agent role is especially relevant for large codebases.
- **Fast test subsets:** For video processing tests (which are slow), implement `--fast` flag with deterministic random sampling. Each agent session runs a subset, coverage across all agents.
- **CI pipeline for autonomous work:** As the project matures, stricter CI enforcement prevents agents from breaking existing functionality — critical for production video systems.
- **Cost-benefit analysis:** If Tavus can parallelize coding agent work, the cost-to-value ratio may justify significant API spend for complex features that would otherwise require weeks of engineering.
- **Time blindness mitigation:** Video processing involves long-running operations. Agents need progress reporting and time limits to prevent getting stuck in test loops.

## Open Questions

- **Scaling beyond 16 agents:** What's the practical limit? Does coordination overhead grow linearly, quadratically?
- **When does an orchestrator become necessary?** The "no orchestrator" approach worked here but might not for more interdependent tasks.
- **Quality ceiling:** Is the quality gap (vs. expert human code) fundamental or will it shrink with better models?
- **Merge conflict resolution quality:** How well does Claude actually handle merge conflicts? What percentage of merges introduce bugs?
- **Reproducibility:** Can this be reproduced? The article mentions Opus 4.6 specifically — would different model versions produce dramatically different results?
- **Safety implications:** Author explicitly notes unease. "Programmers deploying software they've never personally verified is a real concern." How do you audit 100K lines of agent-written code?
- **Task decomposition at runtime:** Agents self-decompose tasks. How reliable is this? What percentage of time is wasted on poor decomposition choices?

## Links to Related Work

- [Claude's C Compiler source code](https://github.com/anthropics/claudes-c-compiler) — the actual artifact
- [GCC torture test suite](https://gcc.gnu.org/onlinedocs/gccint/Torture-Tests.html) — benchmark used
- [Long-running agent harness](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) — related Anthropic work
- Ralph-loop — referenced as similar prior art for agent loops
- [Claude Code](https://www.anthropic.com/claude-code) — underlying agent harness used
