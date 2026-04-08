# Local Environment Setup Checklist

Use this as a personal checklist when setting up a new machine or onboarding to this project.

---

## Prerequisites

- [ ] **Bun** installed (`curl -fsSL https://bun.sh/install | bash`)
- [ ] **Node.js 20+** installed (required by some tooling)
- [ ] **Docker** installed (for local Supabase)
- [ ] **Claude Code** installed (`npm i -g @anthropic-ai/claude-code`)
- [ ] **ANTHROPIC_API_KEY** set in shell profile

---

## Repository Setup

```sh
git clone https://github.com/jerrythomas/sensei
cd sensei
bun install
```

Verify:

```sh
bun run test      # should pass
bun run typecheck # 0 errors
```

---

## Supabase Setup

```sh
# Start local Supabase (first run downloads Docker images — takes a few minutes)
npx supabase start

# Output includes:
#   API URL: http://localhost:54321
#   anon key: eyJ...
```

Run migrations:

```sh
npx supabase db push
```

---

## Sensei Config

Create `.sensei/config.yaml` (gitignored — each developer has their own):

```yaml
supabase_url: http://localhost:54321
supabase_key: <anon-key-from-supabase-start>
repo_id: sensei
```

---

## Claude Code MCP Config

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "sensei": {
      "command": "bun",
      "args": ["run", "<absolute-path-to-repo>/packages/server/src/index.ts"],
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

---

## First Index

```sh
bun run cli init
# or if already initialised:
bun run cli index
```

This scans the repo and populates your local Supabase with symbols and embeddings.

---

## Shell Profile

Add to `~/.zshrc` or `~/.bashrc`:

```sh
export ANTHROPIC_API_KEY=sk-ant-...
export SUPABASE_URL=http://localhost:54321
export SUPABASE_KEY=<your-anon-key>
```

---

## Done

Open Claude Code in the repo root:

```sh
claude
```

Start your session with:

```
get_session_context(task_description="<what you're about to do>")
```
