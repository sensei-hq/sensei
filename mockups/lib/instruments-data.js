// Instruments data — consolidates tools per MCP for the revised Playground.
//
// "MCP" = an installed server. "Tools" = what that server exposes.
// Each tool is either:
//   · action  — does something (writes, side-effect)
//   · query   — returns information
//
// Sensei's own MCP draws from window.MCP_TOOLS; we annotate each sensei
// tool with its kind (action/query) via a lookup table.
//
// Third-party MCPs (Postgres, Stripe, GitHub, Sentry) get representative
// tool lists so the Playground has real content when scope switches.

window.INSTRUMENTS = (function () {
  // ── annotate sensei tools with action/query kind ────────────
  const SENSEI_KIND = {
    "project.explain-structure":  "query",
    "project.find-hotspots":      "query",
    "project.find-duplication":   "query",
    "library.explain":            "query",
    "library.find-usage":         "query",
    "library.suggest-rule":       "query",
    "library.doc-drift":          "query",
    "pattern.list-followed":      "query",
    "pattern.list-antipatterns":  "query",
    "pattern.promote":            "action",     // writes a rule file
    "session.summarize":          "query"
  };

  const senseiTools = (window.MCP_TOOLS?.tools || []).map(t => ({
    ...t,
    mcp: "sensei",
    kind: SENSEI_KIND[t.id] || "query"
  }));

  // ── third-party MCPs ─────────────────────────────────────────
  const thirdParty = [
    // ═════════ Postgres MCP ═════════
    {
      mcp: "postgres",
      id: "postgres.list-tables",
      name: "postgres.list_tables",
      kind: "query",
      summary: "List tables in a schema with row counts and size.",
      inputs: [
        { key: "schema", kind: "text", required: false, label: "Schema",
          default: "public" }
      ],
      example: {
        schema: "public",
        response: `12 tables in public:

  users                 24,182 rows · 18 MB
  sessions             412,309 rows · 124 MB
  refresh_tokens        88,771 rows · 11 MB
  orgs                      84 rows · 32 kB
  ... +8 more`
      }
    },
    {
      mcp: "postgres",
      id: "postgres.describe-table",
      name: "postgres.describe_table",
      kind: "query",
      summary: "Return the columns, indexes, and constraints of a table.",
      inputs: [
        { key: "schema", kind: "text", required: false, label: "Schema", default: "public" },
        { key: "table",  kind: "text", required: true,  label: "Table",
          placeholder: "sessions" }
      ],
      example: {
        schema: "public", table: "sessions",
        response: `sessions · 8 columns, 3 indexes

  id              uuid          PK
  user_id         uuid          FK users.id
  issued_at       timestamptz   not null
  expires_at      timestamptz   not null
  revoked_at      timestamptz   nullable
  device_fp       text
  ip              inet
  created_at      timestamptz   default now()

Indexes:
  • sessions_pkey            (id)
  • sessions_user_id_idx     (user_id)
  • sessions_expires_at_idx  (expires_at) · WHERE revoked_at IS NULL`
      }
    },
    {
      mcp: "postgres",
      id: "postgres.explain",
      name: "postgres.explain",
      kind: "query",
      summary: "Return the planner's EXPLAIN output for a query (read-only).",
      inputs: [
        { key: "sql",     kind: "text", required: true, label: "SQL",
          placeholder: "SELECT ..." },
        { key: "analyze", kind: "enum", required: false, label: "Analyze",
          options: ["plan", "analyze"], default: "plan" }
      ],
      example: {
        sql: "SELECT * FROM sessions WHERE user_id=$1 AND revoked_at IS NULL",
        analyze: "analyze",
        response: `Bitmap Heap Scan on sessions  (cost=4.12..124.8 rows=18 width=124)
  Recheck Cond: (user_id = $1)
  Filter: (revoked_at IS NULL)
  Rows Removed by Filter: 2
  ->  Bitmap Index Scan on sessions_user_id_idx  (cost=0..4.11 rows=20)
        Index Cond: (user_id = $1)

Planning time: 0.182 ms
Execution time: 0.241 ms`
      }
    },
    {
      mcp: "postgres",
      id: "postgres.run-migration",
      name: "postgres.run_migration",
      kind: "action",
      summary: "Apply a migration file to the local dev database.",
      inputs: [
        { key: "file",   kind: "text", required: true, label: "Migration file",
          placeholder: "migrations/2026_01_18_add_fingerprint.sql" },
        { key: "dry",    kind: "enum", required: false, label: "Mode",
          options: ["dry-run", "apply"], default: "dry-run" }
      ],
      example: {
        file: "migrations/2026_01_18_add_fingerprint.sql", dry: "dry-run",
        response: `DRY-RUN · migrations/2026_01_18_add_fingerprint.sql

  ALTER TABLE sessions ADD COLUMN device_fp text;
  CREATE INDEX sessions_device_fp_idx ON sessions(device_fp);

Would affect: 1 table · 0 rows rewritten.
No issues detected. Re-run with mode=apply to commit.`
      }
    },

    // ═════════ Stripe MCP ═════════
    {
      mcp: "stripe",
      id: "stripe.list-prices",
      name: "stripe.list_prices",
      kind: "query",
      summary: "List active prices with product + interval.",
      inputs: [
        { key: "product", kind: "text", required: false, label: "Product id",
          placeholder: "prod_..." }
      ],
      example: {
        product: "prod_LumenPro",
        response: `4 prices on prod_LumenPro:

  price_1O5Zx   $19/mo    usd · monthly
  price_1O5Zy   $190/yr   usd · yearly
  price_1O5Zz   €19/mo    eur · monthly (test)
  price_1O5ZA   £16/mo    gbp · monthly (test)`
      }
    },
    {
      mcp: "stripe",
      id: "stripe.get-customer",
      name: "stripe.get_customer",
      kind: "query",
      summary: "Retrieve a customer, their subscriptions, and recent invoices.",
      inputs: [
        { key: "id", kind: "text", required: true, label: "Customer id",
          placeholder: "cus_..." }
      ],
      example: {
        id: "cus_NXv8Tq",
        response: `cus_NXv8Tq · aiko@studio.co  ·  created 2024-11-02

Subscription:
  sub_1P3fTt  · LumenPro  · active · renews 2026-02-03  · $19/mo

Recent invoices:
  in_1P8k…   paid   $19.00  2026-01-03
  in_1P5f…   paid   $19.00  2025-12-03
  in_1P2…    paid   $19.00  2025-11-03`
      }
    },
    {
      mcp: "stripe",
      id: "stripe.replay-webhook",
      name: "stripe.replay_webhook",
      kind: "action",
      summary: "Re-send a webhook event to your local listener.",
      inputs: [
        { key: "event", kind: "text", required: true, label: "Event id",
          placeholder: "evt_..." },
        { key: "url",   kind: "text", required: false, label: "Target URL",
          default: "http://localhost:4242/stripe/webhook" }
      ],
      example: {
        event: "evt_1P8kX9", url: "http://localhost:4242/stripe/webhook",
        response: `Replayed evt_1P8kX9 (invoice.payment_succeeded)
  → http://localhost:4242/stripe/webhook
  ← 200 OK · 42ms

Local handler logged:
  [webhook] invoice.payment_succeeded for cus_NXv8Tq · $19.00`
      }
    },

    // ═════════ GitHub MCP ═════════
    {
      mcp: "github",
      id: "github.search-code",
      name: "github.search_code",
      kind: "query",
      summary: "Search code across repos you have access to.",
      inputs: [
        { key: "query", kind: "text", required: true,  label: "Query",
          placeholder: "repo:lumen/api axum middleware" },
        { key: "limit", kind: "number", required: false, label: "Limit", default: 10 }
      ],
      example: {
        query: "repo:lumen/api from_fn", limit: 10,
        response: `6 matches in lumen/api:

  src/middleware.ts:22   use(from_fn(trace))
  src/middleware.ts:38   use(from_fn(auth))
  src/router.ts:19       use(from_fn(rate_limit))
  src/handlers/sync.ts:41
  tests/middleware.test.ts:8
  tests/router.test.ts:42`
      }
    },
    {
      mcp: "github",
      id: "github.list-prs",
      name: "github.list_prs",
      kind: "query",
      summary: "List pull requests in a repo, filtered by state.",
      inputs: [
        { key: "repo",  kind: "text", required: true, label: "Repo",
          placeholder: "lumen/api" },
        { key: "state", kind: "enum", required: false, label: "State",
          options: ["open", "closed", "merged", "all"], default: "open" }
      ],
      example: {
        repo: "lumen/api", state: "open",
        response: `4 open PRs in lumen/api:

  #418  Fix refresh-token rotation skew         aiko · 2 reviews
  #416  Extract auth middleware                 ren  · 1 review · CI green
  #411  Add device fingerprint to sessions      mika · 0 reviews · CI failing
  #407  Retry with exponential backoff          aiko · approved · ready to merge`
      }
    },
    {
      mcp: "github",
      id: "github.create-issue",
      name: "github.create_issue",
      kind: "action",
      summary: "Open a new issue in a repo.",
      inputs: [
        { key: "repo",  kind: "text", required: true, label: "Repo",
          placeholder: "lumen/api" },
        { key: "title", kind: "text", required: true, label: "Title" },
        { key: "body",  kind: "text", required: false, label: "Body" },
        { key: "labels", kind: "text", required: false, label: "Labels (comma)" }
      ],
      example: {
        repo: "lumen/api", title: "Retry logic is duplicated across 3 handlers",
        labels: "refactor, pattern",
        response: `Opened issue #420 in lumen/api
  → https://github.com/lumen/api/issues/420

Labels: refactor, pattern
Assigned: (none)`
      }
    },

    // ═════════ Sentry MCP ═════════
    {
      mcp: "sentry",
      id: "sentry.recent-errors",
      name: "sentry.recent_errors",
      kind: "query",
      summary: "Recent unresolved errors grouped by fingerprint.",
      inputs: [
        { key: "project", kind: "text", required: true, label: "Sentry project",
          placeholder: "lumen-api" },
        { key: "since",   kind: "since", required: false, label: "Since",
          options: ["1h", "24h", "7d"], default: "24h" }
      ],
      example: {
        project: "lumen-api", since: "24h",
        response: `3 groups (24h) · 41 events total

  TokenExpiredError             28 events · auth/refresh.ts:94
  SessionNotFoundError           9 events · api/handlers/sessions.ts:112
  PoolTimeoutError               4 events · db/pool.ts:22 (rare)

Top release: 2026.01.17-a7f3c2  (12% regression in TokenExpiredError)`
      }
    }
  ];

  // ── MCP index ────────────────────────────────────────────────
  const mcps = [
    { id: "sensei",   name: "Sensei",       kanji: "先",
      tagline: "Your codebase's private expert.",
      publisher: "local", verified: true, installed: true },
    { id: "postgres", name: "Postgres",     kanji: "庫",
      tagline: "Local Postgres, introspected.",
      publisher: "pgMCP", verified: true, installed: true },
    { id: "stripe",   name: "Stripe",       kanji: "銀",
      tagline: "Live dashboard data, from the CLI.",
      publisher: "stripe", verified: true, installed: true },
    { id: "github",   name: "GitHub",       kanji: "貢",
      tagline: "Repos, PRs, issues.",
      publisher: "github", verified: true, installed: false, recommended: true },
    { id: "sentry",   name: "Sentry",       kanji: "哨",
      tagline: "Recent errors, grouped.",
      publisher: "sentry.io", verified: true, installed: false, recommended: false }
  ];

  // Merge tools, attach counts back onto mcps
  const allTools = [...senseiTools, ...thirdParty];
  mcps.forEach(m => {
    m.toolCount = allTools.filter(t => t.mcp === m.id).length;
    m.actionCount = allTools.filter(t => t.mcp === m.id && t.kind === "action").length;
    m.queryCount  = allTools.filter(t => t.mcp === m.id && t.kind === "query").length;
  });

  return { mcps, tools: allTools };
})();
