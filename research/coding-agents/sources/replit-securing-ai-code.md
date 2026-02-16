# Securing AI-Generated Code (Replit White Paper)
> Source: https://blog.replit.com/securing-ai-generated-code
> Date: Jan 14, 2026

## One-line summary
Replit's research shows that AI-only security scans are nondeterministic and miss dependency vulnerabilities; a hybrid architecture combining deterministic static analysis with LLM-based reasoning is essential for securing AI-generated code.

## Key architectural decisions

1. **Hybrid security architecture**: Combine rule-based static analysis + dependency scanning (deterministic baseline) with LLM-powered reasoning (intent-level, business logic issues).
2. **Controlled experiments on React apps**: Tested with realistic vulnerability variants to compare AI-only vs. hybrid approaches.
3. **Deterministic tools for deterministic problems**: Static analysis for known patterns (hardcoded secrets, SQL injection); LLMs for novel/contextual issues.
4. **Continuous vulnerability feeds**: Dependency scanning requires real-time CVE databases that LLMs don't have access to.

## What worked

- **Static analysis provides consistency**: Rule-based scanners deliver deterministic, repeatable detection across all code variations. No prompt sensitivity, no nondeterminism.
- **LLMs add value for intent-level reasoning**: LLMs can reason about business logic flaws and contextual security issues that static tools miss.
- **Hybrid outperforms either alone**: The combination catches more than either approach independently.

## What failed or was hard

- **AI-only scans are nondeterministic**: Identical vulnerabilities get different classifications based on minor syntactic changes or variable naming. This is a fundamental reliability problem.
- **Prompt sensitivity**: Detection depends on what security issues are explicitly mentioned in the prompt. Shifts responsibility from tool to user—dangerous.
- **Functionally equivalent code assessed differently**: Same vulnerability, different syntax → different security verdict. E.g., hardcoded secrets detected in one representation, missed in another.
- **Dependency vulnerabilities invisible to LLMs**: Without real-time CVE feeds, LLMs can't identify version-specific vulnerabilities. Supply-chain risks are completely missed.

## Novel insights

1. **Nondeterminism as a security liability**: In security scanning, inconsistency is worse than missing things consistently. If a scan sometimes catches a bug and sometimes doesn't, you can't trust it at all.
2. **Prompt sensitivity creates a false sense of security**: Users who don't know to ask about specific vulnerability classes won't get those classes checked. The tool's coverage depends on user expertise—defeating the purpose.
3. **Syntactic sensitivity reveals shallow understanding**: LLMs matching patterns rather than understanding semantics. `const password = "secret"` detected; `const credential = "secret"` missed.
4. **Supply-chain attacks are LLM-blind**: This is a critical gap. Dependency vulnerabilities are one of the most common attack vectors, and LLMs fundamentally can't address them without external data.

## Applicable to Tavus

- **Don't trust LLM-only code review**: If Tavus's coding agent generates code, don't rely on another LLM to verify its security. Use traditional SAST/DAST tools as baseline.
- **Dependency scanning is non-negotiable**: AI agents install packages; those packages need CVE checking via Snyk, Dependabot, or similar. LLMs won't catch this.
- **Hybrid verification pipeline**: Static analysis → LLM reasoning → human review for high-risk changes. Layer defenses.
- **Video processing security**: Video codecs, FFmpeg commands, and media processing have their own vulnerability classes (buffer overflows, path traversal). Static analysis tools specific to these domains would be important.

## Open questions

- How do they integrate the security scanning into the agent loop? Does it block commits? Run asynchronously?
- What's the false positive rate of the hybrid approach? Too many false positives could slow the agent down.
- How do they handle the tension between security scanning and agent autonomy? (Stopping to fix security issues vs. moving forward)
- Can LLM security reasoning improve with fine-tuning on security-specific datasets?

## Links to related work

- [Full white paper](https://securing-ai-generated-code.replit.app/)
- [Decision-Time Guidance blog post](https://blog.replit.com/decision-time-guidance) — related post on keeping agents reliable
- [16 Ways to Vibe Code Securely](https://blog.replit.com/16-ways-to-vibe-code-securely)
