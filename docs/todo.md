Core Customization Features

  ┌─────────────┬───────────────────────┬──────────────────────────────┐
  │   Feature   │       Location        │           Purpose            │
  ├─────────────┼───────────────────────┼──────────────────────────────┤
  │ CLAUDE.md   │ Project/user/global   │ Persistent instructions      │
  │             │                       │ loaded every session         │
  ├─────────────┼───────────────────────┼──────────────────────────────┤
  │ Custom      │ .claude/commands/*.md │ Slash commands with          │
  │ commands    │                       │ $ARGUMENTS                   │
  ├─────────────┼───────────────────────┼──────────────────────────────┤
  │ Skills      │ Plugin-provided       │ Reusable prompt workflows    │
  │             │                       │ (what you're using now)      │
  ├─────────────┼───────────────────────┼──────────────────────────────┤
  │ Plugins     │ Distributed packages  │ Bundle skills + agents +     │
  │             │                       │ hooks + MCP servers          │
  └─────────────┴───────────────────────┴──────────────────────────────┘

  Automation & Integration

  - Hooks — Shell scripts triggered on events: PreToolUse, PostToolUse,
  SessionStart, UserPromptSubmit, Stop, etc. Great for auto-formatting,
  blocking file edits, injecting context.
  - MCP Servers — Connect external tools, APIs, and data. Configured in
  .mcp.json or via claude mcp add. Scoped per project or globally.
  - LSP Servers (.lsp.json) — Real-time code intelligence for any language.

  Agents & Parallelism

  - Subagents — Specialized agents in .claude/agents/ with their own tools,
   prompts, memory, and permissions. Can run in background.
  - Agent Teams — Multiple subagents working in parallel, coordinated by a
  lead agent.
  - Worktrees — Isolated git worktrees for parallel sessions on the same
  repo.

  Session & Context

  - Auto Memory (~/.claude/projects/.../memory/) — Claude self-writes
  learnings that persist across sessions (what you have set up here).
  - Plan Mode — Review Claude's plan before it executes any code.
  - Extended Thinking — Deep reasoning for complex problems (/thinking on
  Opus).
  - Checkpointing — Rewind conversation state.

  UI & Workflow

  - Keybindings (~/.claude/keybindings.json) — Fully remappable shortcuts
  with chord support.
  - Statusline — Customizable CLI status bar.
  - Scheduled Tasks (/loop) — Cron-style recurring prompts.

  Settings

  - Settings files — .claude/settings.json (project),
  .claude/settings.local.json (local), ~/.claude/settings.json (global) —
  control permissions, model, hooks, env vars.
  - Managed settings — Org-wide policy deployment for teams.

  The biggest ones you're likely not using yet: Hooks (powerful automation)
   and MCP servers (external integrations).
   
What's Built In

  /cost command — Shows session totals:
  Total cost:            $0.55
  Total duration (API):  6m 19.7s
  Total duration (wall): 6h 33m 10.2s
  Session-level only, no per-task breakdown.

  Hooks Don't Have Token Data

  Hooks (Stop, SessionEnd, PostToolUse, etc.) do not receive token counts
  in their JSON payloads — so you can't tap into them for usage tracking.

  The Real Solution: OpenTelemetry

  Claude Code has built-in OTel support with a claude_code.api_request
  event that fires per API call with:
  - input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens
  - cost_usd, duration_ms, model
  - prompt.id — lets you group by task/prompt

  Enable it with:
  export CLAUDE_CODE_ENABLE_TELEMETRY=1
  export OTEL_METRICS_EXPORTER=otlp
  export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
  claude

  Then pipe to Prometheus, Datadog, Honeycomb, ClickHouse, etc. Group
  events by prompt.id to reconstruct per-task costs.

  Anthropic Console

  The https://platform.claude.com Console shows aggregate org/workspace
  usage — but no programmatic API for querying Claude Code session history,
   and no per-task granularity.

  Bottom line: OTel export is the only way to get per-task token tracking.
UI & Workflow

   


  
  Per the CLAUDE.md, get_session_context is meant to be called at the start of
     every task — meaning when you give me something to work on. It's not
    automatically triggered on session startup.
  
    So the flow is:
    1. Session starts (no auto-call)
    2. You give me a task → I should call
    get_session_context(task_description="...") before doing anything
  
    If you want it called on session startup automatically (before any user
    message), that would require a hook — a SessionStart hook that calls it. You
     already have a SessionStart hook running (I see SessionStart:startup hook
    success: Success in the context), but it doesn't appear to be calling
    get_session_context.
    
     - A SessionStart hook outputs text that becomes a <system-reminder>
      visible to Claude at startup
      - Since get_session_context is MCP-only (can't be called from a shell),
      the hook needs to inject a strong instruction telling Claude to call it
      before responding
      - This should go in the project-level .claude/settings.local.json so it
      only fires for the sensei project
    
      The hook output will look like the existing SessionStart:startup hook
      success: Success message — it becomes a system-reminder before your first
      message.
      },
          17 +  "hooks": {
          18 +    "SessionStart": [
          19 +      {
          20 +        "hooks": [
          21 +          {
          22 +            "type": "command",
          23 +            "command": "echo 'SESSION PROTOCOL REQUIRED: Ca
             +ll get_session_context(task_description=\"session startup\"
             +) as your FIRST tool call before responding. Do not greet o
             +r ask questions first — call the tool immediately.'"
          24 +          }
          25 +        ]
          26 +      }
          27 +    ]
          
          Done. On the next session start in this project, Claude will see:
          
            SessionStart: SESSION PROTOCOL REQUIRED: Call
            get_session_context(task_description="session startup") as your FIRST tool
             call before responding. Do not greet or ask questions first — call the
            tool immediately.
          
            This gets injected as a <system-reminder> before any user message, which
            means I'll call get_session_context automatically before responding to
            you.
          
            One caveat: hooks can't directly invoke MCP tools — they can only inject
            text. So this works by instructing Claude (me) to make the call. If you
            want to verify it's working, just start a new session and watch whether
            get_session_context is the first tool call.
            
get_session_context should be used in SessionStart hook. 

We should document claude behavior covered above so that our implementation works properly. 
If behaviour changes we may need to adapt. 

Claude supports plugins, custom commands. Refer to `./reverse-engineer.md` for an example reverse engineering command. 

Sensei could use these customization features:

- Customizable cli status bar for showing some info like sensei skill, mcp usage calls. Low importance
- Scheduled tasks looks promising could be used for running specific skills like doc drift detection. could also be used for heartbeat
- Sensei effectively is a plugin because we are bundling skills, hooks and mcp servers. maybe we need to add an agent as well and register as a plugin. 
  - sensei:reverse-engineer command
  - sensei:zero-error-policy skill

etc.
Considering this lets re-work on design looking at what we want to implement and where it makes sense to use sensei as a plugin, sensei skills, commands etc. We may end up revisiting how our sensei cli works, what the dashboard collects and how it shows data.

I think our collector/telemetry should be fault tolerant. If the remote/supabase endpoint is not available or throws error, we should cache the payloads locally and attempt later.
What telemetry/collector received, is pshed in queue and sent to server when server is available, removed from q on success. does this make sense?

should hooks be set up to start mcp if not running? or should we
  consiter starting the monitorig service externally?


Should we consider writing the sensei using rust. Will it add performance benefits? 

To start seeing data in the dashboard

  Task Sessions / Tool Usage — data appears when the sensei MCP tools are
  used. Per CLAUDE.md, every session should start with:
  get_session_context(task_description="what you're doing")
  Start your next coding session with this call and the analytics page will
  begin populating.

  Cost Breakdown — appears automatically once Claude Code sends OTLP events
  to port 4318. Your ~/.claude/settings.json already has
  OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 set, so the next
  conversation's API calls will be recorded.
