# ClaudeKit

A polished, open-source desktop app that supercharges how developers interact with Claude Code.

ClaudeKit unifies session history browsing, real-time prompt quality feedback, AI-powered prompt rewriting, token/cost tracking, and raw session introspection into a single installable app.

## Features (Planned)

- **Session Browser** — Browse all your Claude Code sessions with syntax highlighting and full-text search
- **Insights Engine** — AI-powered analysis of your session history with actionable suggestions
- **Real-Time Prompt Feedback** — Live prompt quality scoring via Claude Code hook integration
- **Prompt Optimizer** — Rewrite any prompt using Claude Opus with extended thinking
- **Token Tracker & Raw Inspector** — Usage dashboards and full HTTP-level session inspection

## Tech Stack

- [Tauri v2](https://tauri.app/) — Rust backend + React/TypeScript frontend
- SQLite (via `rusqlite`) — Local session indexing
- React + Vite + shadcn/ui + Tailwind — UI
- Anthropic API — AI-powered features (user provides their own key)

## Status

Early development. See [`docs/PRD.md`](docs/PRD.md) for the full product requirements document.

## Development Phases

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | Session Browser | Planned |
| 2 | Insights Engine | Planned |
| 3 | Real-Time Prompt Feedback | Planned |
| 4 | Prompt Optimizer | Planned |
| 5 | Token Tracker & Raw Inspector | Planned |

## Philosophy

- **Local-first** — No cloud, no telemetry, no third-party servers
- **Your API key** — All AI features use your own Anthropic account
- **Fast** — SQLite indexing keeps the session browser snappy even with thousands of sessions
- **Open source** — Pre-built binaries on GitHub Releases, source always available

## License

MIT
