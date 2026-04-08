# .sensei — AI Assistant Environment

This folder contains configuration for the Sensei MCP server and setup instructions
so every contributor can reproduce the same AI-assisted development environment locally.

Each developer manages their own local Supabase instance — there is no shared cloud
component. See `docs/product/self-hosted-model.md` for the product rationale.

---

## What is Sensei?

Sensei is a codebase intelligence tool that makes AI coding agents more effective by:

- **Indexing** your repo's symbols, call edges, and embeddings into a personal Supabase instance
- **Serving** ranked context to agents via MCP tools (`get_session_context`, `search`, `context_pack`)
- **Collecting** session events (tool calls, FTR scores, token costs) from Claude hooks
- **Coaching** through skills that enforce design-before-code, zero-errors checkpoints, and pattern reuse

---

## Quick Setup

### 1. Install Claude Code

```sh
npm install -g @anthropic-ai/claude-code
```

### 2. Install Bun

```sh
curl -fsSL https://bun.sh/install | bash
```

### 3. Start a local Supabase instance

```sh
npx supabase start
# Copy the API URL and anon key from the output
```

Or use a personal project at supabase.com.

### 4. Configure Sensei

Copy `.sensei/config.yaml.example` to `.sensei/config.yaml` (not committed — contains your URL):

```yaml
supabase_url: http://localhost:54321
supabase_key: <your-anon-key>
repo_id: sensei   # matches the repo name in the index
```

### 5. Run `sensei init`

```sh
bun run cli init
# Scans the repo, indexes symbols into Supabase, installs Claude hooks
```

### 6. Verify

```sh
bun run test      # all tests must pass
bun run typecheck # 0 TypeScript errors
```

---

## Session Workflow

Every Claude Code session in this repo follows the MCP session protocol:

1. **Session start** — call `get_session_context(task_description="...")`. This loads
   interrupted work, returns repo orientation, and creates a tracked session.
2. **During work** — call `take_snapshot(progress_summary)` at key decision points.
   Use `context_pack(query)` to load only relevant symbols (saves tokens).
3. **Session end** — call `checkpoint(outcome, summary)`. This computes FTR score
   and closes the session.

See `CLAUDE.md` for the full protocol.

---

## MCP Server

The Sensei MCP server runs as a stdio transport. Claude Code connects to it via
the config in `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "sensei": {
      "command": "bun",
      "args": ["run", "/path/to/sensei/packages/server/src/index.ts"],
      "env": {
        "SUPABASE_URL": "http://localhost:54321",
        "SUPABASE_KEY": "<your-anon-key>"
      }
    },
    "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp@latest"]
    }
  }
}
```

Replace the path and credentials with your local values.

---

## Skills Used in This Project

| Skill | When to use |
| ----- | ----------- |
| `sensei:session` | Start of every session — calls `get_session_context` |
| `sensei:checkpoint` | Mid-session progress save |
| `sensei:pattern-based-development` | Before implementing any new feature |
| `sensei:working-smarter` | When building UI or starting any implementation task |
| `zero-errors-policy` | Enforces 0 lint + 0 TypeScript errors at checkpoints |
| `commit-commands:commit` | Runs zero-errors check then commits |

---

## Environment Variables

Add to `~/.zshrc` or `~/.bashrc`:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
export SUPABASE_URL=http://localhost:54321
export SUPABASE_KEY=<your-anon-key>
```

---

## Keeping This File Up to Date

If you change the MCP server config, add a required env var, or update the setup
steps, update this README so the next contributor picks it up automatically.
