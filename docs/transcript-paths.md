# Sharing AI coding transcripts

We're analysing how AI coding sessions actually go — where time is spent, where
work gets redone, where the tool helps and where it gets in the way. The output
is a set of productivity metrics and concrete suggestions back to you for your
own workflow.

Find your tool below, run the commands for your OS, and send the archive.

If you use more than one tool, send one archive per tool.

---

## GitHub Copilot CLI

### macOS / Linux

```bash
tar czf ~/copilot-cli-transcripts.tar.gz -C ~/.copilot session-state session-store.db
```

### Windows (PowerShell)

```powershell
Compress-Archive -Path "$env:USERPROFILE\.copilot\session-state",
                       "$env:USERPROFILE\.copilot\session-store.db" `
                 -DestinationPath "$env:USERPROFILE\copilot-cli-transcripts.zip"
```

### One session only

If you'd rather send a single session than everything:

```bash
ls -lt ~/.copilot/session-state/        # newest at the top

tar czf ~/copilot-cli-one-session.tar.gz -C ~/.copilot session-state/<session-id>
```

---

## VS Code (GitHub Copilot Chat)

Replace `Code` with `Code - Insiders`, `VSCodium`, or `Code - OSS` if that's the
build you use.

### macOS

```bash
cd ~/Library/Application\ Support/Code/User
tar czf ~/vscode-copilot-transcripts.tar.gz workspaceStorage globalStorage
```

### Linux

```bash
cd ~/.config/Code/User
tar czf ~/vscode-copilot-transcripts.tar.gz workspaceStorage globalStorage
```

### Windows (PowerShell)

```powershell
Compress-Archive -Path "$env:APPDATA\Code\User\workspaceStorage",
                       "$env:APPDATA\Code\User\globalStorage" `
                 -DestinationPath "$env:USERPROFILE\vscode-copilot-transcripts.zip"
```

### If that archive comes out too large

`workspaceStorage` holds state for every folder you've ever opened. To send just
the chat files:

```bash
cd ~/Library/Application\ Support/Code/User    # macOS
# cd ~/.config/Code/User                       # Linux

find workspaceStorage globalStorage \
     \( -name '*.jsonl' -o -name 'workspace.json' -o -name 'agent-traces.db' \) \
     -print0 | tar czf ~/vscode-copilot-chats.tar.gz --null -T -
```

Keep the `workspace.json` files — they're what maps a chat back to the project it
belongs to.

### Remote SSH

If you work over Remote SSH, the data is on the **remote host**, not your laptop.
Run the Linux commands there, starting from:

```bash
cd ~/.vscode-server/data/User
```

---

## Cursor

### macOS / Linux

```bash
tar czf ~/cursor-transcripts.tar.gz -C ~/.cursor projects chats
```

### Windows (PowerShell)

```powershell
Compress-Archive -Path "$env:USERPROFILE\.cursor\projects",
                       "$env:USERPROFILE\.cursor\chats" `
                 -DestinationPath "$env:USERPROFILE\cursor-transcripts.zip"
```

Cursor stores some sessions as a single file and others as a folder. The commands
above capture both — no need to hunt for individual sessions.

---

## Claude Code

One JSONL file per session, grouped by project.

### macOS / Linux / Windows (Git Bash / WSL)

```bash
tar czf ~/claude-transcripts.tar.gz -C ~/.claude projects
```

Only `projects` — the rest of `~/.claude/` holds settings and credentials.

### Windows (PowerShell)

```powershell
Compress-Archive -Path "$env:USERPROFILE\.claude\projects" `
                 -DestinationPath "$env:USERPROFILE\claude-transcripts.zip"
```

### One project only

Project folders are named after their path, with `/` replaced by `-`:

```bash
ls ~/.claude/projects/                  # find yours

tar czf ~/claude-one-project.tar.gz -C ~/.claude projects/<folder-name>
```

---

## Zed

A single SQLite database of threads.

### macOS

```bash
tar czf ~/zed-transcripts.tar.gz -C ~/Library/Application\ Support/Zed threads
```

### Linux

```bash
tar czf ~/zed-transcripts.tar.gz -C ~/.local/share/zed threads
```

If that path doesn't exist, search for it and send whatever it finds:

```bash
find ~ -name 'threads.db' -path '*[Zz]ed*' 2>/dev/null
```

---

## OpenCode

A single SQLite database.

### macOS / Linux

```bash
cd ~/.local/share/opencode
tar czf ~/opencode-transcripts.tar.gz opencode.db opencode.db-wal opencode.db-shm
```

Include the `-wal` and `-shm` files if they're there — recent sessions may not
have been written into the main file yet, and without them you'd send a stale
copy. If they don't exist, just send `opencode.db`.

Send only those files — `~/.local/share/opencode/` also holds `auth.json`.

### Windows (PowerShell)

```powershell
Compress-Archive -Path "$env:USERPROFILE\.local\share\opencode\opencode.db*" `
                 -DestinationPath "$env:USERPROFILE\opencode-transcripts.zip"
```

---

## Visual Studio 2022+

Visual Studio keeps chat history in two places, and one of them is temporary.

### Persisted sessions, per solution

```powershell
# Run from your solution folder
Compress-Archive -Path ".vs\*\copilot-chat" `
                 -DestinationPath "$env:USERPROFILE\vs-copilot-sessions.zip"
```

### Chat logs — temporary, so grab these before they're cleaned up

```powershell
Compress-Archive -Path "$env:TEMP\VSGitHubCopilotLogs" `
                 -DestinationPath "$env:USERPROFILE\vs-copilot-logs.zip"
```

### Agent mode plans

```powershell
Compress-Archive -Path "$env:TEMP\VisualStudio\copilot-vs" `
                 -DestinationPath "$env:USERPROFILE\vs-copilot-plans.zip"
```

---

## Reference — where it lives and what's in it

### GitHub Copilot CLI

Base directory is `~/.copilot/` (`%USERPROFILE%\.copilot\` on Windows), or
wherever `COPILOT_HOME` points.

| Path | Contents |
|---|---|
| `session-state/<id>/events.jsonl` | The full session: prompts, replies, tool calls, tool results, model changes, token usage at shutdown |
| `session-state/<id>/workspace.yaml` | Working directory, session name, summary |
| `session-state/<id>/plan.md` | Implementation plan, when one was created |
| `session-store.db` | SQLite — `sessions` (summary, repo, branch, timestamps), `turns`, `checkpoints`, `session_files` |
| `data.db` | SQLite — workspace-level totals including token counts |

Not requested, and not picked up by the commands above: `config.json`, `logs/`,
`mcp-config.json`.

### VS Code

Base directory:

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Code/User/` |
| Linux | `~/.config/Code/User/` |
| Windows | `%APPDATA%\Code\User\` |
| Remote SSH | `~/.vscode-server/data/User/` on the remote host |

| Path | Contents |
|---|---|
| `workspaceStorage/<id>/chatSessions/*.jsonl` | Chat history, stored as a series of deltas |
| `workspaceStorage/<id>/workspace.json` | Maps that folder back to your project path |
| `workspaceStorage/<id>/GitHub.copilot-chat/transcripts/*.jsonl` | Newer format — a typed event stream |
| `globalStorage/emptyWindowChatSessions/*.jsonl` | Chats started without a folder open |
| `globalStorage/github.copilot-chat/agent-traces.db` | SQLite with real per-turn token counts. Present only if tracing was enabled |
| `workspaceStorage/<id>/state.vscdb` | Older chat metadata |

Which of these exist depends on your VS Code and Copilot Chat versions — it's
normal for only some to be present.

### Cursor

| Path | Contents |
|---|---|
| `~/.cursor/projects/<hash>/agent-transcripts/<id>.jsonl` | Session transcript, single-file layout |
| `~/.cursor/projects/<hash>/agent-transcripts/<id>/<id>.jsonl` | Same, folder layout |
| `~/.cursor/chats/<hash>/<id>/store.db` | SQLite — model used, created-at, mode |

Cursor records tool *inputs* but not tool *results*.

### Claude Code

| Path | Contents |
|---|---|
| `~/.claude/projects/<path-slug>/*.jsonl` | One file per session — prompts, replies, tool calls and tool results. The slug is the project path with `/` replaced by `-` |

The rest of `~/.claude/` (`settings.json`, `mcp.json`, `plugins/`, caches) is not
requested.

### Zed

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/Zed/threads/threads.db` |
| Linux | `~/.local/share/zed/threads/threads.db` |

SQLite, `threads` table — one row per conversation.

### OpenCode

| Path | Contents |
|---|---|
| `~/.local/share/opencode/opencode.db` | SQLite — sessions and messages |
| `~/.local/share/opencode/opencode.db-wal`, `-shm` | Write-ahead log. Recent activity may live here rather than in the main file |

`auth.json` in the same folder is not requested.

### Visual Studio

| Path | Contents |
|---|---|
| `<solution>\.vs\<name>\copilot-chat\<hash>\sessions\` | Persisted chat sessions |
| `%TEMP%\VSGitHubCopilotLogs\*.chat.log` | Raw chat logs — temporary, cleaned up periodically |
| `%TEMP%\VisualStudio\copilot-vs\` | Agent mode plan files |

### Path confirmation

Claude Code, Zed and OpenCode were confirmed on a real machine. The Copilot and
Cursor paths come from documentation and may differ on your setup — if a command
finds nothing, say so and send whatever the tool's data folder does contain.

### What's in the archives

Your prompts, the assistant's replies, and the tool activity in between —
including file contents the assistant read while working. Worth a look before
sending if any of those sessions touched something that shouldn't leave your
machine.
