# Research Tooling: Gaps & Fixes

## Current Capabilities
| Tool | Works for | Fails for |
|------|-----------|-----------|
| `web_fetch` | Static HTML, simple blogs | JS-heavy SPAs, X/Twitter, paywalled content |
| `web_search` (Brave) | General search, finding URLs | Deep X/Twitter thread discovery |
| `browser` (headless Chrome) | Cookie walls, JS rendering, screenshots | Logged-in content (no persistent auth) |
| `summarize` | YouTube transcripts ✅, JS sites (better than web_fetch), PDFs | X/Twitter (needs `bird` which is deprecated) |
| `yt-dlp` | Just installed — transcript extraction backup | — |
| `blogwatcher` | RSS/Atom feed monitoring | Not installed yet |

## Blockers & Fixes

### 1. X/Twitter Access (HIGH PRIORITY)
**Problem:** Can't fetch tweets. `web_fetch` returns nothing. `bird` CLI is deprecated/removed. Nitter instances are dead or 403.
**Fix options:**
- Browser automation with a logged-in session (Krishna would need to log in once via VNC)
- X API access (paid, $100/mo basic tier)
- Use Brave Search with `site:x.com` queries to find tweet content in search snippets (partial, but free)
- Firecrawl API key for `summarize` might help

### 2. Persistent Browser Auth (MEDIUM)
**Problem:** Every browser session starts fresh. Can't access paywalled Substacks, logged-in X, etc.
**Fix:** Chrome profile data persists across sessions (the `openclaw` profile). If Krishna logs into X once via VNC on that Chrome profile, subsequent headless sessions should retain cookies.

### 3. RSS/Feed Monitoring for New Content (MEDIUM)
**Problem:** Research is point-in-time. New blog posts from Cursor, Anthropic, etc. won't be caught.
**Fix:** Install `blogwatcher`, add feeds for all key sources. Set up a cron job to check weekly.

### 4. Video Content (SOLVED ✅)
**Problem:** YouTube transcripts were inaccessible.
**Fix:** `summarize --youtube auto --extract-only` works perfectly. `yt-dlp` installed as backup.

### 5. Exec Timeouts (LIVABLE)
**Problem:** Long-running commands get SIGKILL'd after ~60s.
**Fix:** Use `nohup` + background for anything >30s. Check back with polling. Already adapted to this.

## Action Items
- [ ] Get X/Twitter access sorted (discuss with Krishna)
- [ ] Install blogwatcher and add feeds for: Anthropic eng blog, Cursor blog, Replit blog, Cognition blog, Sourcegraph blog, Continue blog, OpenAI blog
- [ ] Consider Firecrawl API key for summarize (handles JS + anti-bot better)
- [ ] Test browser cookie persistence for authenticated research
