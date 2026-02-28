# Phase 1 — Session Browser: Product Requirements Document

**Version:** 1.0
**Status:** Draft
**Parent:** [Master PRD](PRD.md)
**Last Updated:** 2026-02-28

---

## 1. Overview

Phase 1 delivers the **foundational session browsing experience** for ClaudeKit. It is the only phase that requires no Anthropic API key — it reads exclusively from Claude Code's local JSONL files. All later phases are built on top of this foundation.

By the end of Phase 1, a user should be able to install ClaudeKit, open it, and immediately have a beautiful, fast interface to browse every Claude Code session they've ever had — with full conversation rendering including tool use, code blocks, and an activity heatmap.

---

## 2. Goals

- Parse and index all local Claude Code session data from `~/.claude/projects/`
- Provide a browsable sidebar of projects and sessions
- Render full conversations including user/assistant messages and all tool interactions
- Display a GitHub-style activity heatmap on the dashboard
- Ship as a working, installable Tauri v2 desktop app on macOS (primary), with Windows/Linux parity targeted
- Establish the project scaffold, build pipeline, and CI foundation for all future phases

---

## 3. Out of Scope for Phase 1

The following are explicitly deferred to later phases:

- Full-text search across sessions (Phase 2+)
- Any Anthropic API calls or AI features
- Prompt quality scoring or insights
- Token/cost tracking (the data will be parsed and stored, but no UI for it yet)
- Real-time file watching (watch for new sessions while app is open — nice-to-have, deferred)
- Raw HTTP inspector view
- Settings UI (API key entry, preferences)
- Windows hook support

---

## 4. User Stories

### US-1: First Launch
> As a new user, I open ClaudeKit for the first time and it automatically finds my Claude Code sessions from `~/.claude/projects/` without any configuration.

**Acceptance criteria:**
- App reads `~/.claude/projects/` on startup without requiring user to set a path
- If the directory doesn't exist or is empty, a friendly empty state is shown
- Session count and project count visible on dashboard within 3 seconds of launch

---

### US-2: Browse Projects & Sessions
> As a user, I want to see all my Claude Code projects in a sidebar, expand them to see sessions, and click a session to read it.

**Acceptance criteria:**
- Left sidebar lists all projects (directory names decoded from their hashed paths, or raw if not resolvable)
- Each project shows the count of sessions it contains
- Sessions within a project are listed sorted by most-recent-first
- Session list items show: date, a truncated first user message as a title, and message count
- Clicking a session loads it in the main pane

---

### US-3: Read a Full Conversation
> As a user, I want to read a Claude Code session and see all messages — including tool calls and results — formatted clearly.

**Acceptance criteria:**
- User messages rendered with markdown support
- Assistant messages rendered with markdown and syntax-highlighted code blocks
- Tool use blocks rendered per tool type (see Section 5.3 for tool rendering spec)
- Tool results rendered inline, collapsible if long (>20 lines)
- Messages displayed in chronological order
- Timestamps shown on hover or in a subtle secondary style

---

### US-4: Activity Dashboard
> As a user, I want to open ClaudeKit and see at a glance how actively I've been using Claude Code over the past year.

**Acceptance criteria:**
- Dashboard shows a GitHub-style contribution heatmap (52-week grid, Mon–Sun columns)
- Each cell colored by session activity level (0, low, medium, high) with 4–5 intensity tiers
- Hovering a cell shows the date and session count in a tooltip
- Dashboard also shows: total sessions, total projects, most active project, activity this week vs last week

---

## 5. Feature Specification

### 5.1 Project & Session Discovery

**Project path resolution:**
Claude Code stores projects under paths like `~/.claude/projects/-Users-username-Code-myproject/`. The directory name is the absolute project path with `/` replaced by `-`. ClaudeKit should reverse this encoding to display human-readable project names (e.g., `~/Code/myproject`).

**Session JSONL files:**
Each `.jsonl` file in a project directory represents one session. The filename is the session UUID. Files are parsed line-by-line; each line is a JSON object.

**Indexing:**
On first launch (and on manual refresh), the Rust backend walks all project directories, parses all JSONL files, and upserts records into a local SQLite database. The database is the source of truth for the UI — the UI never reads JSONL files directly.

**Incremental updates:**
Phase 1 does full re-index on launch. File watching for live updates is deferred.

---

### 5.2 SQLite Schema

See the TDD for full schema. At a high level:

- `projects` — one row per `~/.claude/projects/<dir>` with decoded path and metadata
- `sessions` — one row per `.jsonl` file with session UUID, project FK, timestamps, message count, token totals
- `messages` — one row per JSONL line (message entry) with role, content (raw JSON), timestamp, parent UUID

Token totals are stored at the session level even though token UI is Phase 5 — the data is free to capture now.

---

### 5.3 Conversation Rendering

#### Message Types

| JSONL `type` field | Rendered as |
|-------------------|-------------|
| `human` / user role | User message bubble |
| `assistant` | Assistant message bubble |
| `system` | Collapsed "system prompt" disclosure |
| `tool_use` (in assistant content) | Tool call block (see below) |
| `tool_result` (in user content) | Tool result block (see below) |

#### Tool Call Rendering

Each tool gets a distinct visual treatment:

| Tool name | Render style |
|-----------|-------------|
| `Bash` | Dark terminal block with command line, output in monospace |
| `Read` | File path header + syntax-highlighted content block |
| `Write` / `Edit` | Diff view (before/after) with line-level highlighting |
| `Glob` | File path list |
| `Grep` | Search query + match list with file:line references |
| `WebFetch` / `WebSearch` | URL pill + response summary |
| `Task` | Nested indented block labeled "Subagent task" |
| Unknown tools | Generic collapsible JSON block |

Tool results longer than 20 lines are collapsed by default with a "Show all" expander.

#### Code Blocks

All fenced code blocks in assistant messages use syntax highlighting. Language is auto-detected if not specified. Inline `code` uses a distinct monospace style.

---

### 5.4 Activity Heatmap

- **Layout:** 52 columns (weeks) × 7 rows (Mon–Sun), most recent week on the right
- **Color scale:** 5 tiers — `0 sessions` (background), `1`, `2–4`, `5–9`, `10+`
- **Color theme:** Matches app theme (green tones in light mode, teal/emerald in dark)
- **Tooltip:** Hover shows `{date}: {n} sessions`
- **Year selector:** If user has data spanning multiple years, allow switching years
- **Scope:** Counts sessions started (by session `created_at` timestamp)

---

### 5.5 Dashboard Summary Stats

Displayed above or alongside the heatmap:

- **Total sessions** (all time)
- **Total projects**
- **Sessions this week** vs last week (with up/down delta indicator)
- **Most active project** (by session count, all time)
- **Longest session** (by message count)

---

## 6. Non-Functional Requirements

| Requirement | Target |
|------------|--------|
| Cold start to interactive | < 3 seconds for up to 1,000 sessions |
| Session load time (any session) | < 500ms |
| Indexing speed | > 500 sessions/second |
| App binary size | < 20 MB (Tauri target, no Chromium) |
| Memory usage at rest | < 150 MB |
| macOS minimum version | macOS 12 (Monterey) |
| Windows minimum version | Windows 10 (1903+) |

---

## 7. UI Layout

```
┌─────────────────────────────────────────────────────────────┐
│  ClaudeKit                                        [─][□][×]  │
├──────────────┬──────────────────────────────────────────────┤
│              │                                              │
│  Dashboard   │   [Dashboard pane]                          │
│  ──────────  │   Activity heatmap + summary stats          │
│  Projects    │                                              │
│  ──────────  │                                              │
│  > myproject │                                              │
│    ○ Session │                                              │
│    ○ Session │                                              │
│  > project2  │                                              │
│    ○ Session │                                              │
│              │                                              │
│              │                                              │
└──────────────┴──────────────────────────────────────────────┘
```

When a session is selected:

```
┌─────────────────────────────────────────────────────────────┐
│  ClaudeKit                                        [─][□][×]  │
├──────────────┬──────────────────────────────────────────────┤
│              │  myproject / 2026-02-14                      │
│  Dashboard   │  ─────────────────────────────────────────  │
│  ──────────  │                                              │
│  Projects    │  [User]  Fix the auth bug in login.ts        │
│  ──────────  │                                              │
│  > myproject │  [Claude]  I'll look at login.ts...          │
│  ● Session ◄ │  ┌─ Bash ──────────────────────────────┐    │
│    ○ Session │  │ $ cat src/auth/login.ts              │    │
│    ○ Session │  └─────────────────────────────────────┘    │
│  > project2  │  ┌─ Result (collapsed) ────── [Show] ──┐    │
│              │  └─────────────────────────────────────┘    │
│              │                                              │
└──────────────┴──────────────────────────────────────────────┘
```

---

## 8. Phase 1 Deliverables Checklist

- [ ] `claudekit` Tauri v2 project scaffolded and building locally
- [ ] Rust backend: JSONL parser
- [ ] Rust backend: SQLite indexer
- [ ] Rust backend: Tauri commands exposed to frontend
- [ ] React frontend: project/session sidebar
- [ ] React frontend: conversation renderer (all tool types)
- [ ] React frontend: dashboard with heatmap
- [ ] GitHub Actions CI: build check on push for macOS
- [ ] GitHub Actions CI: build artifacts for macOS `.dmg`

---

## 9. Success Criteria

Phase 1 is complete when:

1. A fresh install on macOS reads `~/.claude/projects/` and displays all sessions without configuration
2. Clicking any session renders the full conversation including tool use blocks
3. The activity heatmap correctly reflects the user's session history
4. Cold start time is under 3 seconds with a representative dataset (tested with 200+ sessions)
5. The app builds and runs on macOS via `bun tauri build`

---

*Next: [Phase1_SessionBrowser_TDD.md](Phase1_SessionBrowser_TDD.md)*
