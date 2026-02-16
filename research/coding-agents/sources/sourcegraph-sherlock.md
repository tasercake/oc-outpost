# Sourcegraph: Lessons from Building Sherlock — Automating Security Code Reviews

**Source:** https://sourcegraph.com/blog/lessons-from-building-sherlock-automating-security-code-reviews-with-sourcegraph

## One-line summary
Sourcegraph's security team built Sherlock, an internal LLM-powered tool that enriches PR diffs with security context (SAST alerts, threat models, related files) and uses Cody's API for automated security code review, saving ~30 min/day per security engineer and catching issues traditional scanners miss.

## Key architectural decisions
- **GitHub App integration** — triggered on PR open/update. Streams diffs, SAST alerts, and metadata into Sherlock. Event-driven, not manual.
- **Context enrichment before LLM** — adds references, related files, definitions BEFORE sending to Cody API. Reduces hallucination by grounding the LLM.
- **Custom security prompts per codebase** — tailored to their threat models and codebase specifics, not generic security rules.
- **SAST + LLM correlation** — feeds code scanner alerts INTO the LLM analysis. LLM can correlate scanner findings with broader context, reducing false positive noise.
- **Output to Slack + SIEM** — results go to a Slack channel for quick visibility AND logged in SIEM for broader alerting. Dual-channel output.
- **Hackathon origin** — started as an experiment, grew into a critical tool. Low-cost exploration → high-value production system.

## What worked
- **400 PRs scanned in 2 months** — found 3 high-severity, 4 medium-severity issues, plus 12 edge cases. Real, actionable findings.
- **Edge case detection** — LLMs excel at finding issues "outside standard pattern-matching approaches" that require understanding how code changes interact across the application.
- **30 min/day saved per security engineer** — concrete, measurable ROI from reduced manual triage.
- **Low-hanging fruit + subtle issues** — catches both obvious vulnerabilities scanners miss AND nuanced problems. Dual value.
- **SAST + LLM correlation** — using scanner output as input to LLM analysis is clever. Scanner provides structured findings, LLM provides contextual judgment.

## What failed or was hard
- **Hallucinations** — LLMs flag "non-existent vulnerabilities" and "point to unrelated files." Ongoing problem despite prompt refinement.
- **Generic best practices instead of specific findings** — LLMs tend to suggest security best practices rather than identifying actual vulnerabilities in the specific code. Had to customize prompts aggressively.
- **Code navigation limitations** — LLMs "don't navigate codebases the way humans do by mapping out symbols, references, and definitions across files." They can flag code as vulnerable without understanding broader context.
- **False positives from lack of cross-file understanding** — LLM sees a snippet as vulnerable but doesn't understand that input is already validated elsewhere.

## Novel insights
- **SAST output as LLM input** — feeding traditional scanner results INTO the LLM is an underappreciated pattern. The scanner provides structured signal; the LLM provides contextual reasoning to filter noise.
- **"Enumerating risks and edge cases" is the real value** — not replacing security review but augmenting it by systematically surfacing things humans might miss.
- **Future: code navigation APIs + LLM** — combining Sourcegraph's code intelligence (symbol resolution, references, definitions) with LLM analysis would address the cross-file understanding gap.
- **Internal tools as proving grounds** — Sherlock was built for internal use first. Low-stakes environment to iterate before productizing.
- **Security review is a perfect LLM use case** — bounded scope (PR diff), clear evaluation criteria (vulnerability found or not), and human-in-the-loop validation.

## Applicable to Tavus
- **PR-triggered security review agent** — directly applicable. Any company can build a Sherlock-equivalent. Especially valuable for video pipeline code handling user data.
- **SAST + LLM hybrid approach** — if Tavus uses static analysis tools, feeding their output into an LLM for contextual triage is a quick win.
- **Custom security prompts for domain** — Tavus handles video, potentially PII in video content, media processing. Domain-specific security prompts would be valuable.
- **Hackathon → production pipeline** — good model for introducing AI-assisted security review. Start with a hackathon, prove value on real PRs, scale.
- **Edge case enumeration for video processing** — LLMs could enumerate edge cases in video codec handling, stream processing, etc. that scanners can't detect.
- **Dual-output pattern (Slack + SIEM)** — good architecture for any AI-powered monitoring/review system.

## Open questions
- What's the false positive rate? They mention hallucinations but don't quantify.
- How do they handle Sherlock's findings that are wrong? Is there a feedback loop to improve prompts?
- What's the cost per PR scan?
- How does this scale to larger PRs (1000+ line diffs)?
- Could this approach work for non-security code quality issues?

## Links to related work
- [PurpleLlama cybersecurity benchmarks](https://github.com/meta-llama/PurpleLlama/tree/main/CybersecurityBenchmarks)
- [Sourcegraph Cody](https://cody.dev/)
- [Continue Anti-Slop agent](sources/continue-code-slop.md) — similar pattern for code quality
