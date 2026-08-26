# AI Coding Tool Transcript & Session Storage Locations

Cross-platform reference for where GitHub Copilot (CLI, VS Code, Visual Studio)
and Cursor store transcript, session, and chat data locally.

> **Purpose:** Share this with colleagues so they can locate and share their
> transcript files for analysis. All paths are per-user (not machine-wide).

---

## Table of Contents

1. [GitHub Copilot CLI](#1-github-copilot-cli)
2. [VS Code + GitHub Copilot Chat](#2-vs-code--github-copilot-chat)
3. [Visual Studio 2022+ GitHub Copilot](#3-visual-studio-2022-github-copilot)
4. [Cursor IDE](#4-cursor-ide)
5. [Quick Reference Table](#5-quick-reference-table)

---

## 1. GitHub Copilot CLI

### Home directory

The base directory for all Copilot CLI data is `~/.copilot/`. Override with the
`COPILOT_HOME` environment variable.

| Platform | Default path |
|---|---|
| **macOS** | `~/.copilot/` |
| **Linux** | `~/.copilot/` (or `$XDG_CONFIG_HOME/copilot/`) |
| **Windows** | `%USERPROFILE%\.copilot\` (e.g. `C:\Users\You\.copilot\`) |

### Session transcripts (events.jsonl)

Each CLI session produces a directory with a full event stream:

```
~/.copilot/session-state/<session-id>/
├── events.jsonl          ← primary transcript (JSONL, append-only)
├── workspace.yaml        ← session metadata (cwd, summary, name)
├── inuse.<PID>.lock      ← present while session is active
└── plan.md               ← implementation plan (if created)
```

**events.jsonl** is the richest source: user messages, assistant responses,
tool calls, tool results, model changes, token usage (at session shutdown).

| Platform | Path |
|---|---|
| **macOS** | `~/.copilot/session-state/` |
| **Linux** | `~/.copilot/session-state/` |
| **Windows** | `%USERPROFILE%\.copilot\session-state\` |

To find all active sessions (lock files):

```bash
# macOS/Linux
ls ~/.copilot/session-state/*/inuse.*.lock

# Windows PowerShell
Get-ChildItem "$env:USERPROFILE\.copilot\session-state\*\inuse.*.lock" |
    ForEach-Object { $_.Directory.Name }
```

### Session store (SQLite)

A single SQLite database with structured session + turn data:

```
~/.copilot/session-store.db
```

| Platform | Path |
|---|---|
| **macOS** | `~/.copilot/session-store.db` |
| **Linux** | `~/.copilot/session-store.db` |
| **Windows** | `%USERPROFILE%\.copilot\session-store.db` |

Tables: `sessions` (id, summary, repo, branch, timestamps), `turns`
(user_message, assistant_response), `checkpoints`, `session_files`,
`session_refs`, `search_index` (FTS5).

### Data store (SQLite)

Workspace-level aggregate data including token totals:

```
~/.copilot/data.db
```

| Platform | Path |
|---|---|
| **macOS** | `~/.copilot/data.db` |
| **Linux** | `~/.copilot/data.db` |
| **Windows** | `%USERPROFILE%\.copilot\data.db` |

### Other Copilot CLI paths

| Path | Contents |
|---|---|
| `~/.copilot/config.json` | Trusted folders, settings |
| `~/.copilot/command-history-state.json` | Prompt/command history |
| `~/.copilot/agents/*.md` | Custom agent definitions |
| `~/.copilot/logs/` | Debug/error logs |
| `~/.copilot/mcp-config.json` | MCP server configuration |

### Cache directory (not under ~/.copilot)

| Platform | Path |
|---|---|
| **macOS** | `~/Library/Caches/copilot` |
| **Linux** | `$XDG_CACHE_HOME/copilot` or `~/.cache/copilot` |
| **Windows** | `%LOCALAPPDATA%\copilot` |

---

## 2. VS Code + GitHub Copilot Chat

VS Code stores Copilot Chat data in multiple layers. All paths are under the
VS Code **User data directory**, which varies by OS and variant.

### VS Code variants

| Variant | macOS directory name | Linux directory name |
|---|---|---|
| VS Code (stable) | `Code` | `Code` |
| VS Code Insiders | `Code - Insiders` | `Code - Insiders` |
| VSCodium | `VSCodium` | `VSCodium` |
| Code - OSS | `Code - OSS` | `Code - OSS` |

### User data directory (base)

| Platform | Path |
|---|---|
| **macOS** | `~/Library/Application Support/<variant>/User/` |
| **Linux** | `~/.config/<variant>/User/` |
| **Windows** | `%APPDATA%\<variant>\User\` (e.g. `C:\Users\You\AppData\Roaming\Code\User\`) |

### Layer 1 — Chat session journals (primary)

Delta-serialized JSONL files. These are the main chat history.

```
<User-data>/workspaceStorage/<hash>/chatSessions/<session-id>.jsonl
<User-data>/globalStorage/emptyWindowChatSessions/<session-id>.jsonl
<User-data>/globalStorage/transferredChatSessions/<session-id>.jsonl
```

| Platform | Full path pattern |
|---|---|
| **macOS** | `~/Library/Application Support/Code/User/workspaceStorage/*/chatSessions/*.jsonl` |
| **Linux** | `~/.config/Code/User/workspaceStorage/*/chatSessions/*.jsonl` |
| **Windows** | `%APPDATA%\Code\User\workspaceStorage\*\chatSessions\*.jsonl` |

Each `workspaceStorage/<hash>/` has a `workspace.json` that maps the hash back
to the original project folder URI (`file:///path/to/project`).

### Layer 2 — Newer transcript format (event stream)

Typed event stream (session.start, user.message, assistant.message, tool calls):

```
<User-data>/workspaceStorage/<hash>/GitHub.copilot-chat/transcripts/<session-id>.jsonl
```

| Platform | Full path pattern |
|---|---|
| **macOS** | `~/Library/Application Support/Code/User/workspaceStorage/*/GitHub.copilot-chat/transcripts/*.jsonl` |
| **Linux** | `~/.config/Code/User/workspaceStorage/*/GitHub.copilot-chat/transcripts/*.jsonl` |
| **Windows** | `%APPDATA%\Code\User\workspaceStorage\*\GitHub.copilot-chat\transcripts\*.jsonl` |

### Layer 3 — OTel span store (richest token data)

SQLite database with real per-turn token counts. Requires opt-in:

```
<User-data>/globalStorage/github.copilot-chat/agent-traces.db
```

| Platform | Path |
|---|---|
| **macOS** | `~/Library/Application Support/Code/User/globalStorage/github.copilot-chat/agent-traces.db` |
| **Linux** | `~/.config/Code/User/globalStorage/github.copilot-chat/agent-traces.db` |
| **Windows** | `%APPDATA%\Code\User\globalStorage\github.copilot-chat\agent-traces.db` |

Enable via VS Code setting `github.copilot.chat.otel.dbSpanExporter.enabled`
or env var `COPILOT_OTEL_DB_SPAN_EXPORTER_ENABLED=true`.

### Layer 4 — Legacy SQLite state

Older chat metadata in the workspace state database:

```
<User-data>/workspaceStorage/<hash>/state.vscdb
```

Contains `memento/interactive-session` and `interactive.sessions` keys.

### VS Code Remote SSH

When using Remote SSH, VS Code server data lives on the **remote host**:

```
~/.vscode-server/data/User/workspaceStorage/<hash>/chatSessions/
~/.vscode-server/data/User/globalStorage/github.copilot-chat/
```

On the remote, replace `~/.config/Code/User/` with `~/.vscode-server/data/User/`.

### Workspace mapping

To find which project a chat belongs to, read the sibling `workspace.json`:

```bash
cat ~/Library/Application\ Support/Code/User/workspaceStorage/<hash>/workspace.json
# Returns: {"workspace":{"folder":{"$mid":1,"path":"/Users/you/project"}}}
```

---

## 3. Visual Studio 2022+ GitHub Copilot

Visual Studio (not VS Code) stores Copilot data differently.

### Chat logs (raw)

```
%TEMP%\VSGitHubCopilotLogs\<timestamp>_VSGitHubCopilot.chat.log
```

| Platform | Path |
|---|---|
| **Windows** | `C:\Users\<you>\AppData\Local\Temp\VSGitHubCopilotLogs\` |

These are raw log files with timestamps. Contains Q&A history, system prompts,
and analysis details. **Ephemeral** — stored in Temp, may be cleaned up.

### Persisted chat sessions

```
<solution-dir>\.vs\<solution-name>\copilot-chat\<hash>\sessions\
```

| Platform | Path |
|---|---|
| **Windows** | `C:\Users\<you>\source\<repo>\.<solution>\copilot-chat\<hash>\sessions\` |

These are per-solution persisted sessions.

### Agent mode planning data

```
%TEMP%\VisualStudio\copilot-vs\
```

| Platform | Path |
|---|---|
| **Windows** | `C:\Users\<you>\AppData\Local\Temp\VisualStudio\copilot-vs\` |

Contains JSON plan files for agent mode sessions.

### Copilot cache

```
%LOCALAPPDATA%\GitHub\Copilot
```

| Platform | Path |
|---|---|
| **Windows** | `C:\Users\<you>\AppData\Local\GitHub\Copilot` |

### Output window logs

In Visual Studio: **View > Output > GitHub Copilot** dropdown.

---

## 4. Cursor IDE

### Transcript root

```
~/.cursor/projects/
```

| Platform | Path |
|---|---|
| **macOS** | `~/.cursor/projects/` |
| **Linux** | `~/.cursor/projects/` |
| **Windows** | `%USERPROFILE%\.cursor\projects\` |

### Agent transcripts (JSONL)

Each project is a subdirectory named by its path hash. Sessions are UUIDs:

```
~/.cursor/projects/<project-hash>/agent-transcripts/<session-id>.jsonl
~/.cursor/projects/<project-hash>/agent-transcripts/<session-id>/<session-id>.jsonl
```

Some sessions use flat layout (file directly), others use nested (directory).
Both layouts exist — scan for `*.jsonl` recursively.

**Content:** User messages, assistant text responses, tool use events.
**Known limitation:** Tool results are NOT included (only tool inputs).

| Platform | Full path pattern |
|---|---|
| **macOS** | `~/.cursor/projects/*/agent-transcripts/**/*.jsonl` |
| **Linux** | `~/.cursor/projects/*/agent-transcripts/**/*.jsonl` |
| **Windows** | `%USERPROFILE%\.cursor\projects\*\agent-transcripts\*\*.jsonl` |

### Session metadata (SQLite)

```
~/.cursor/chats/<workspace-hash>/<session-id>/store.db
```

Contains `meta` + `blobs` tables. Carries `lastUsedModel`, `createdAt`,
`agentId`, `mode`.

| Platform | Path |
|---|---|
| **macOS** | `~/.cursor/chats/` |
| **Linux** | `~/.cursor/chats/` |
| **Windows** | `%USERPROFILE%\.cursor\chats\` |

### Other Cursor paths

| Path | Contents |
|---|---|
| `~/.cursor/rules/` | Global cursor rules |
| `~/.cursor/commands/` | Custom commands |
| `~/Library/Application Support/Cursor/User/skills/` | Downloaded skills (macOS) |
| `~/.config/cursor/User/skills/` | Downloaded skills (Linux) |
| `%APPDATA%\Cursor\User\skills\` | Downloaded skills (Windows) |

---

## 5. Quick Reference Table

### Copilot CLI

| What | macOS | Linux | Windows |
|---|---|---|---|
| Home | `~/.copilot/` | `~/.copilot/` | `%USERPROFILE%\.copilot\` |
| Events | `~/.copilot/session-state/<id>/events.jsonl` | same | `%USERPROFILE%\.copilot\session-state\<id>\events.jsonl` |
| Session DB | `~/.copilot/session-store.db` | same | `%USERPROFILE%\.copilot\session-store.db` |
| Data DB | `~/.copilot/data.db` | same | `%USERPROFILE%\.copilot\data.db` |
| Config | `~/.copilot/config.json` | same | `%USERPROFILE%\.copilot\config.json` |

### VS Code

| What | macOS | Linux | Windows |
|---|---|---|---|
| User data | `~/Library/Application Support/Code/User/` | `~/.config/Code/User/` | `%APPDATA%\Code\User\` |
| Chat journals | `<user-data>/workspaceStorage/*/chatSessions/*.jsonl` | same | same |
| Transcripts | `<user-data>/workspaceStorage/*/GitHub.copilot-chat/transcripts/*.jsonl` | same | same |
| OTel DB | `<user-data>/globalStorage/github.copilot-chat/agent-traces.db` | same | same |
| Empty window | `<user-data>/globalStorage/emptyWindowChatSessions/*.jsonl` | same | same |
| Workspace map | `<user-data>/workspaceStorage/*/workspace.json` | same | same |
| Remote SSH | `~/.vscode-server/data/User/...` | same | same |

### Visual Studio

| What | Windows |
|---|---|
| Chat logs | `%TEMP%\VSGitHubCopilotLogs\*VSGitHubCopilot.chat.log` |
| Sessions | `<solution>\.vs\<name>\copilot-chat\<hash>\sessions\` |
| Agent plans | `%TEMP%\VisualStudio\copilot-vs\` |
| Cache | `%LOCALAPPDATA%\GitHub\Copilot` |

### Cursor

| What | macOS | Linux | Windows |
|---|---|---|---|
| Root | `~/.cursor/projects/` | same | `%USERPROFILE%\.cursor\projects\` |
| Transcripts | `<root>/<hash>/agent-transcripts/**/*.jsonl` | same | same |
| Session DB | `~/.cursor/chats/<hash>/<id>/store.db` | same | `%USERPROFILE%\.cursor\chats\...` |

---

## Sharing Instructions

To share transcript data for analysis:

### Copilot CLI

```bash
# Create a portable archive of all session transcripts
cd ~/.copilot
tar czf ~/copilot-sessions.tar.gz session-state/ session-store.db data.db

# Or share a single session
tar czf ~/copilot-session-<id>.tar.gz session-state/<session-id>/
```

### VS Code

```bash
# Find workspace hash for your project
cat ~/Library/Application\ Support/Code/User/workspaceStorage/*/workspace.json | grep -l "your-project"

# Export all chat sessions for a workspace
cd ~/Library/Application\ Support/Code/User/workspaceStorage/<hash>
tar czf ~/vscode-chats.tar.gz chatSessions/ workspace.json

# Or export the full global storage
cd ~/Library/Application\ Support/Code/User
tar czf ~/vscode-global-chats.tar.gz globalStorage/emptyWindowChatSessions/
```

### Cursor

```bash
# Export all transcripts for a project
cd ~/.cursor/projects/<project-hash>
tar czf ~/cursor-transcripts.tar.gz agent-transcripts/
```

### Important notes

- **events.jsonl** (Copilot CLI) is append-only and can be large for long sessions
- **OTel DB** (VS Code) requires opt-in (`github.copilot.chat.otel.dbSpanExporter.enabled`)
- **Visual Studio** logs are ephemeral (Temp directory)
- All data is local — no server copies exist for recovery
- Transcripts may contain sensitive code and prompts — handle accordingly

---

## Validation notes (2026-08-26)

### What could and could not be checked

Checked against this machine. **None of the transcript layouts could be
confirmed** — only VS Code stable is installed, and it has no Copilot chat data:

| Tool | Present here | Notes |
|---|---|---|
| Copilot CLI (`~/.copilot`) | ✗ | not installed |
| Cursor (`~/.cursor`) | ✗ | not installed |
| VS Code stable | ✓ | `workspaceStorage` exists |
| VS Code Insiders / VSCodium / Code - OSS | ✗ | |
| `~/.vscode-server` | ✗ | |
| `chatSessions/*.jsonl` | ✗ | 0 files |
| `GitHub.copilot-chat/transcripts/*.jsonl` | ✗ | 0 files |
| `globalStorage/github.copilot-chat/agent-traces.db` | ✗ | absent |
| `globalStorage/emptyWindowChatSessions` | ✗ | absent |
| `workspaceStorage/*/state.vscdb` | ✓ | 1 file |

So treat every path below as RESEARCH, not as verified. That is exactly why we
are asking colleagues for samples — but the adapters should be written against
real files, not against this table.

### Correction: `workspace.json` is not always there

The doc states "Each `workspaceStorage/<hash>/` has a `workspace.json`". The one
such directory on this machine contains only `state.vscdb` and
`state.vscdb.backup` — no `workspace.json` — and it is named `1750350140069`, a
TIMESTAMP, not a hash.

This matters beyond the doc: the transcript-adapter plan resolves a chat's
project from that sibling file. If it is frequently absent, VS Code sessions
cannot be attributed to a project and land with empty `cwds`. Worth confirming
against a machine that actually has chat data before building on it.

### Corrections: the sharing commands do not work as written

**1. The workspace-hash lookup returns nothing useful.**

    cat ~/.../workspaceStorage/*/workspace.json | grep -l "your-project"

`grep -l` reading stdin prints `(standard input)` — it cannot name the file it
matched, which is the entire point. Verified. Use:

    grep -l "your-project" ~/Library/Application\ Support/Code/User/workspaceStorage/*/workspace.json

**2. `**/*.jsonl` does not recurse on macOS.**

macOS ships bash 3.2, which has no `globstar` — `**` degrades to a single `*`, so
`agent-transcripts/**/*.jsonl` silently matches only ONE level. Since Cursor uses
both a flat and a nested layout, a colleague following this would send half their
transcripts and neither of us would notice. Verified: `bash -O globstar` errors
with "invalid shell option name" on the system bash.

Use `find`, which needs no shell support:

    find ~/.cursor/projects -path '*/agent-transcripts/*' -name '*.jsonl'

**3. The Windows Cursor pattern misses the flat layout.**

`...\agent-transcripts\*\*.jsonl` only matches the NESTED form, while the
macOS/Linux row uses `**/*.jsonl` for both. Windows needs the two patterns
listed separately, or `Get-ChildItem -Recurse -Filter *.jsonl`.

### Before anyone runs the `tar` commands

This doc asks colleagues to send their data, so the privacy note at the bottom is
in the wrong place and too soft. What the archives actually contain:

* `session-state/**/events.jsonl` — every prompt, every assistant reply, tool
  inputs AND tool outputs. Tool outputs include file contents the agent read.
* `session-store.db` — the same conversations again, plus a `search_index` FTS5
  table that duplicates the message text. Deleting a row from `turns` does NOT
  remove it from the index.
* `data.db` — workspace-level aggregates, including repo paths.
* `plan.md` — implementation plans, often quoting source.

Deliberately NOT in the suggested archives, and they should stay out:
`~/.copilot/config.json` (trusted folder paths) and `~/.copilot/logs/` (may carry
tokens).

Suggested wording for whoever we ask: *"this archive contains the full text of
your AI conversations, including code the agent read from your machine. Review it
before sending, and do not send it if the sessions touched anything you cannot
share."*

A single-session export is a much better default than the tar-everything
recipes — smaller, reviewable, and usually enough to validate an adapter:

    tar czf ~/copilot-one-session.tar.gz -C ~/.copilot session-state/<session-id>/

### What we actually need from a sample

For adapter work, one session per tool is enough, and the small files matter more
than the databases:

| Tool | Minimum useful sample |
|---|---|
| Copilot CLI | one `session-state/<id>/` directory (`events.jsonl` + `workspace.yaml`) |
| VS Code | one `chatSessions/<uuid>.jsonl` **plus its sibling `workspace.json` if present** — the pairing is the thing in question |
| Cursor | one nested `<id>/<id>.jsonl` AND one flat `<id>.jsonl`, to confirm both layouts are real |

The SQLite databases can wait; they are large, contain the most, and every field
we want from them also appears in the JSONL.

