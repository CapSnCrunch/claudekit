# Phase 1 — Session Browser: Technical Design Document

**Version:** 1.0
**Status:** Draft
**Parent:** [Phase 1 PRD](Phase1_SessionBrowser_PRD.md)
**Last Updated:** 2026-02-28

---

## 1. Project Scaffold

### 1.1 Tech Stack

| Layer | Choice | Version |
|-------|--------|---------|
| App framework | Tauri | v2 |
| Backend language | Rust | stable (1.78+) |
| Frontend runtime | Bun | latest |
| Frontend framework | React | 18 |
| Frontend language | TypeScript | 5 |
| Frontend bundler | Vite | 5 |
| UI components | shadcn/ui | latest |
| Styling | Tailwind CSS | v3 |
| Charts | recharts | 2 |
| DB (Rust) | rusqlite | 0.31+ |
| File watching | notify | 6 |
| HTTP | reqwest | 0.12 (stub, used Phase 2+) |
| Serialization | serde / serde_json | 1 |

### 1.2 Directory Structure

```
claudekit/
├── docs/                          # PRD, TDD, and phase docs
├── src/                           # React/TypeScript frontend
│   ├── components/
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── MainPane.tsx
│   │   │   └── AppShell.tsx
│   │   ├── session/
│   │   │   ├── SessionList.tsx
│   │   │   ├── ConversationView.tsx
│   │   │   ├── MessageBubble.tsx
│   │   │   └── tool-blocks/
│   │   │       ├── BashBlock.tsx
│   │   │       ├── ReadBlock.tsx
│   │   │       ├── WriteBlock.tsx
│   │   │       ├── GlobBlock.tsx
│   │   │       ├── GrepBlock.tsx
│   │   │       ├── WebBlock.tsx
│   │   │       ├── TaskBlock.tsx
│   │   │       └── GenericToolBlock.tsx
│   │   ├── dashboard/
│   │   │   ├── Dashboard.tsx
│   │   │   ├── ActivityHeatmap.tsx
│   │   │   └── SummaryStats.tsx
│   │   └── ui/                    # shadcn/ui generated components
│   ├── hooks/
│   │   ├── useSessions.ts
│   │   ├── useProjects.ts
│   │   └── useConversation.ts
│   ├── lib/
│   │   ├── tauri.ts               # typed wrappers around invoke()
│   │   └── utils.ts
│   ├── types/
│   │   └── index.ts               # shared TS types mirroring Rust structs
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs          # SQLite schema creation / migrations
│   │   │   └── queries.rs         # typed query functions
│   │   ├── parser/
│   │   │   ├── mod.rs
│   │   │   ├── jsonl.rs           # JSONL line parsing
│   │   │   ├── message.rs         # message type deserialization
│   │   │   └── project.rs         # project path decoding
│   │   ├── indexer/
│   │   │   ├── mod.rs
│   │   │   └── walker.rs          # ~/.claude/projects/ walker + upsert logic
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── projects.rs        # list_projects, get_project
│   │       ├── sessions.rs        # list_sessions, get_session_messages
│   │       └── dashboard.rs       # get_dashboard_stats, get_heatmap_data
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/
│   └── workflows/
│       └── ci.yml
├── bunfig.toml
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.ts
└── components.json                # shadcn/ui config
```

### 1.3 Scaffold Commands

```bash
# From inside the claudekit/ directory:
bunx create-tauri-app . \
  --manager bun \
  --template react-ts \
  --identifier com.claudekit.app \
  --app-name ClaudeKit

# Install frontend deps
bun install

# Add shadcn/ui
bunx shadcn@latest init

# Add recharts
bun add recharts

# Verify dev build
bun tauri dev
```

---

## 2. JSONL Format Reference

This section documents the Claude Code JSONL format. The parsing approach is informed by [claude-code-history-viewer](https://github.com/nicobailon/claude-code-history-viewer) and extended to cover all observed message shapes.

### 2.1 File Layout

```
~/.claude/
└── projects/
    └── -Users-username-Code-myproject/   # project dir (path-encoded)
        ├── <session-uuid>.jsonl           # one file per session
        └── <session-uuid>.jsonl
```

**Project path decoding:** The directory name is the absolute path with all `/` replaced by `-`. To decode:
```
"-Users-username-Code-myproject" → "/Users/username/Code/myproject"
```
Edge case: the leading `-` represents the leading `/`.

### 2.2 JSONL Entry Schema

Each line in a `.jsonl` file is a JSON object. The top-level shape:

```typescript
interface JournalEntry {
  type: "user" | "assistant" | "system" | "summary";
  uuid: string;                    // unique ID for this entry
  parent_uuid: string | null;      // for conversation threading
  session_id: string;
  timestamp: string;               // ISO 8601
  message: AnthropicMessage;       // API-format message object
  cost_usd?: number;               // present on assistant entries
  usage?: {
    input_tokens: number;
    output_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
  };
}
```

The `message` field is a standard Anthropic API message object:

```typescript
interface AnthropicMessage {
  id?: string;
  role: "user" | "assistant";
  content: ContentBlock[];
  model?: string;
  stop_reason?: string;
}

type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content: string | ContentBlock[]; is_error?: boolean }
  | { type: "image"; source: { type: "base64"; media_type: string; data: string } };
```

### 2.3 Session Metadata Inference

There is no separate session metadata file — metadata is inferred:

| Property | Source |
|---------|--------|
| `session_id` | `session_id` field on any entry; `.jsonl` filename |
| `created_at` | `timestamp` of first entry |
| `updated_at` | `timestamp` of last entry |
| `title` | Text content of first `user` role entry (truncated to 80 chars) |
| `message_count` | Count of non-`summary` entries |
| `total_input_tokens` | Sum of `usage.input_tokens` across all entries |
| `total_output_tokens` | Sum of `usage.output_tokens` across all entries |

### 2.4 Edge Cases

- **Empty files:** Skip silently.
- **Malformed lines:** Log a warning, skip the line, continue parsing.
- **`summary` type entries:** Store but don't display as messages (used internally by Claude Code for context compaction). Mark in DB as `is_summary = true`.
- **Branching conversations:** Entries have `parent_uuid` forming a tree. For Phase 1, display as a linear sequence (follow the longest chain). Threading UI is deferred.
- **Large files:** Parse line-by-line (streaming), never load entire file into memory.

---

## 3. SQLite Schema

Database location: `~/.claudekit/db.sqlite` (macOS/Linux) or `%APPDATA%\claudekit\db.sqlite` (Windows).

```sql
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
  version   INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

-- Projects
CREATE TABLE IF NOT EXISTS projects (
  id            TEXT PRIMARY KEY,  -- the raw directory name (e.g. "-Users-...")
  decoded_path  TEXT NOT NULL,     -- human-readable path (e.g. "/Users/...")
  display_name  TEXT NOT NULL,     -- last path component (e.g. "myproject")
  session_count INTEGER NOT NULL DEFAULT 0,
  last_active   TEXT,              -- ISO 8601, max updated_at of child sessions
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
  id                   TEXT PRIMARY KEY,  -- session UUID (JSONL filename stem)
  project_id           TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  title                TEXT,              -- first user message, truncated
  message_count        INTEGER NOT NULL DEFAULT 0,
  total_input_tokens   INTEGER NOT NULL DEFAULT 0,
  total_output_tokens  INTEGER NOT NULL DEFAULT 0,
  total_cost_usd       REAL,
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL,
  indexed_at           TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);

-- Messages
CREATE TABLE IF NOT EXISTS messages (
  id            TEXT PRIMARY KEY,  -- entry uuid
  session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  parent_id     TEXT,              -- parent_uuid for threading
  role          TEXT NOT NULL,     -- "user" | "assistant" | "system"
  is_summary    INTEGER NOT NULL DEFAULT 0,
  content_json  TEXT NOT NULL,     -- raw JSON of content block array
  input_tokens  INTEGER,
  output_tokens INTEGER,
  cost_usd      REAL,
  model         TEXT,
  timestamp     TEXT NOT NULL,
  ordinal       INTEGER NOT NULL   -- insertion order within session (for display)
);
CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp  ON messages(timestamp);
```

### 3.1 Schema Migration Strategy

- Schema version stored in `schema_migrations` table
- On app start, Rust runs pending migrations in order
- Migrations are embedded as `&str` constants in `db/schema.rs`
- No migration framework — simple sequential versioned SQL strings

---

## 4. Rust Backend

### 4.1 Indexer (`src-tauri/src/indexer/`)

```rust
// High-level flow
pub async fn run_full_index(db: &Connection, claude_dir: &Path) -> Result<IndexStats> {
    let projects_dir = claude_dir.join("projects");
    for project_dir in read_project_dirs(&projects_dir)? {
        let project = upsert_project(db, &project_dir)?;
        for jsonl_path in read_session_files(&project_dir)? {
            index_session(db, &project.id, &jsonl_path)?;
        }
    }
}

pub fn index_session(db: &Connection, project_id: &str, path: &Path) -> Result<()> {
    // 1. Check if file mtime > sessions.indexed_at → skip if unchanged
    // 2. Open file, read lines
    // 3. Parse each line → JournalEntry
    // 4. Upsert session row (compute title, counts, timestamps)
    // 5. Upsert message rows
}
```

**Optimization:** Before parsing a JSONL file, compare the file's `mtime` against `sessions.indexed_at` in the DB. Skip if unchanged. This makes subsequent launches fast.

### 4.2 Tauri Commands

All commands are `async` and return `Result<T, String>` (errors are serialized as strings for the frontend).

```rust
// commands/projects.rs
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, String>

#[tauri::command]
pub async fn get_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDetail, String>

// commands/sessions.rs
#[tauri::command]
pub async fn list_sessions(
    state: State<'_, AppState>,
    project_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<SessionSummary>, String>

#[tauri::command]
pub async fn get_session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<MessageRecord>, String>

// commands/dashboard.rs
#[tauri::command]
pub async fn get_dashboard_stats(
    state: State<'_, AppState>,
) -> Result<DashboardStats, String>

#[tauri::command]
pub async fn get_heatmap_data(
    state: State<'_, AppState>,
    year: Option<i32>,   // defaults to current year
) -> Result<Vec<HeatmapDay>, String>

// commands/indexer.rs
#[tauri::command]
pub async fn run_index(
    state: State<'_, AppState>,
) -> Result<IndexStats, String>
```

### 4.3 Shared State

```rust
pub struct AppState {
    pub db: Mutex<Connection>,
    pub claude_dir: PathBuf,
}
```

Initialized in `main.rs` before the Tauri builder runs. `claude_dir` defaults to `~/.claude`.

### 4.4 Serializable Structs (serde)

```rust
#[derive(Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub decoded_path: String,
    pub display_name: String,
    pub session_count: i64,
    pub last_active: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub message_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub is_summary: bool,
    pub content_json: String,  // raw JSON, parsed by frontend
    pub timestamp: String,
    pub ordinal: i64,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct HeatmapDay {
    pub date: String,       // "YYYY-MM-DD"
    pub count: i64,         // number of sessions that day
}

#[derive(Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_sessions: i64,
    pub total_projects: i64,
    pub sessions_this_week: i64,
    pub sessions_last_week: i64,
    pub most_active_project: Option<ProjectSummary>,
    pub longest_session: Option<SessionSummary>,
}

#[derive(Serialize, Deserialize)]
pub struct IndexStats {
    pub projects_indexed: usize,
    pub sessions_indexed: usize,
    pub messages_indexed: usize,
    pub duration_ms: u64,
}
```

---

## 5. Frontend

### 5.1 TypeScript Types

Mirror the Rust structs exactly in `src/types/index.ts`:

```typescript
export interface ProjectSummary {
  id: string;
  decodedPath: string;
  displayName: string;
  sessionCount: number;
  lastActive: string | null;
}

export interface SessionSummary {
  id: string;
  projectId: string;
  title: string | null;
  messageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface MessageRecord {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  isSummary: boolean;
  contentJson: string;  // JSON.parse() → ContentBlock[]
  timestamp: string;
  ordinal: number;
  model: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

// Anthropic content block types
export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content: string | ContentBlock[]; is_error?: boolean }
  | { type: "image"; source: { type: "base64"; media_type: string; data: string } };
```

Note: Tauri serializes Rust `snake_case` fields as `camelCase` by default when using `#[serde(rename_all = "camelCase")]`. Ensure this is set on all Rust structs.

### 5.2 Tauri Invoke Wrappers (`src/lib/tauri.ts`)

```typescript
import { invoke } from "@tauri-apps/api/core";
import type { ProjectSummary, SessionSummary, MessageRecord, DashboardStats, HeatmapDay, IndexStats } from "../types";

export const api = {
  listProjects: () =>
    invoke<ProjectSummary[]>("list_projects"),

  listSessions: (projectId: string, limit?: number, offset?: number) =>
    invoke<SessionSummary[]>("list_sessions", { projectId, limit, offset }),

  getSessionMessages: (sessionId: string) =>
    invoke<MessageRecord[]>("get_session_messages", { sessionId }),

  getDashboardStats: () =>
    invoke<DashboardStats>("get_dashboard_stats"),

  getHeatmapData: (year?: number) =>
    invoke<HeatmapDay[]>("get_heatmap_data", { year }),

  runIndex: () =>
    invoke<IndexStats>("run_index"),
};
```

### 5.3 State Management

Use React's built-in hooks — no external state library in Phase 1.

- **App-level state:** `selectedProjectId`, `selectedSessionId` in `App.tsx` via `useState`
- **Data fetching:** Custom hooks (`useSessions`, `useProjects`, `useConversation`) using `useEffect` + `useState` wrapping `api.*` calls
- **Indexing state:** Managed in `App.tsx`, surfaced via a loading indicator and refresh button

### 5.4 Component Breakdown

#### `App.tsx`
- Runs `api.runIndex()` on mount (with loading state)
- Holds `selectedProjectId` and `selectedSessionId`
- Renders `<AppShell>` passing down selection state and setters

#### `AppShell.tsx`
- Two-column layout: `<Sidebar>` (fixed width, ~260px) + `<MainPane>` (flex-grow)

#### `Sidebar.tsx`
- "Dashboard" link at top
- List of projects via `useProjects()`
- Each project is collapsible, shows `<SessionList>` when open
- Active project/session highlighted

#### `SessionList.tsx`
- Receives `projectId`, fetches sessions via `useSessions(projectId)`
- Each row: date (relative, e.g. "3 days ago"), truncated title, message count badge
- Clicking a row calls `onSelectSession(sessionId)`

#### `MainPane.tsx`
- If no session selected: renders `<Dashboard>`
- If session selected: renders `<ConversationView sessionId={...}>`

#### `ConversationView.tsx`
- Fetches messages via `useConversation(sessionId)`
- Scrollable, virtualized if message count > 100 (use `@tanstack/react-virtual`)
- Renders a `<MessageBubble>` per message (skipping `isSummary = true`)

#### `MessageBubble.tsx`
- User messages: right-aligned bubble with markdown rendering (`react-markdown` + `rehype-highlight`)
- Assistant messages: left-aligned, same rendering
- Iterates `contentJson` (parsed to `ContentBlock[]`), delegates tool blocks to appropriate component
- `type: "text"` → inline markdown
- `type: "tool_use"` → `<ToolCallDispatcher name={name} input={input} />`
- `type: "tool_result"` → `<ToolResultBlock content={content} isError={isError} />`

#### Tool Block Components

All tool blocks share a common wrapper with:
- Colored left border (per tool category)
- Tool name badge
- Collapsible body if content > 20 lines

| Component | Renders |
|-----------|---------|
| `BashBlock` | Dark bg terminal, `$ {command}` header |
| `ReadBlock` | File path chip + syntax-highlighted content |
| `WriteBlock` / `EditBlock` | Unified diff view |
| `GlobBlock` | File path list with icons |
| `GrepBlock` | Query + match list `file:line: content` |
| `WebBlock` | URL pill + response body |
| `TaskBlock` | Indented "Subagent task" label + description |
| `GenericToolBlock` | Formatted JSON of `input` |

#### `Dashboard.tsx`
- Renders `<SummaryStats>` + `<ActivityHeatmap>`

#### `ActivityHeatmap.tsx`
- Built with `recharts` (custom cell renderer) or a lightweight custom SVG grid
- 52 × 7 grid
- Color computed from `count` → one of 5 CSS custom property colors
- Tooltip on hover via Radix `Tooltip`
- Year selector (dropdown) if data spans multiple years

#### `SummaryStats.tsx`
- 4–5 stat cards in a responsive grid
- Pulls from `DashboardStats`

---

## 6. Build & CI

### 6.1 Local Dev Commands

```bash
# Start frontend dev server + Tauri in dev mode
bun tauri dev

# Build production app (macOS)
bun tauri build

# Type-check only
bunx tsc --noEmit

# Format
bunx biome format --write .  # (or prettier if preferred)
```

### 6.2 GitHub Actions CI (`.github/workflows/ci.yml`)

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: "./src-tauri -> target"
      - run: bun install
      - run: bun tauri build
      - uses: actions/upload-artifact@v4
        with:
          name: ClaudeKit-macos
          path: src-tauri/target/release/bundle/dmg/*.dmg
```

---

## 7. Dependency List (`Cargo.toml`)

```toml
[dependencies]
tauri = { version = "2", features = ["shell-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
walkdir = "2"
dirs = "5"
thiserror = "1"
log = "0.4"
env_logger = "0.11"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

`rusqlite` with `features = ["bundled"]` statically links SQLite — no system dependency required.

---

## 8. Known Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|-----------|
| Claude Code changes JSONL schema | Medium | Parse defensively with `Option<>` fields; log unknown keys |
| Project path decoding edge cases (spaces, special chars) | Medium | URL-decode as fallback; show raw path if decode fails |
| Very large sessions (10k+ messages) | Low | Lazy-load messages, paginate, use virtual list in React |
| `~/.claude` doesn't exist on first launch | High | Show friendly empty state with instructions to use Claude Code first |
| Windows path encoding differences | Medium | Use `dirs` crate for cross-platform home dir; normalize separators |

---

## 9. Implementation Order

Recommended build order within Phase 1:

1. **Scaffold** — `create-tauri-app`, directory structure, CI
2. **DB schema** — `schema.rs`, migration runner
3. **JSONL parser** — parse a single `.jsonl` file to `Vec<JournalEntry>`
4. **Indexer** — walk `~/.claude/projects/`, upsert to SQLite
5. **Tauri commands** — expose `list_projects`, `list_sessions`, `get_session_messages`
6. **Sidebar** — projects + sessions list, selection state
7. **Conversation renderer** — `MessageBubble` + text content only first
8. **Tool blocks** — `BashBlock` first (most common), then others
9. **Dashboard stats** — `DashboardStats` query + `SummaryStats` component
10. **Activity heatmap** — `get_heatmap_data` query + `ActivityHeatmap` component
11. **Polish** — empty states, loading states, error handling, dark mode

---

*Previous: [Phase1_SessionBrowser_PRD.md](Phase1_SessionBrowser_PRD.md)*
*Next: Phase 2 (Insights Engine) — coming after Phase 1 ships*
