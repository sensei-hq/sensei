// Personal-first mock data for the Dōjō web-app redesign (work-first IA).
// The viewer lands on THEIR work — projects in flight, a "needs you" band,
// live runs — with dōjō membership as an optional, secondary layer.
//
// Two planes throughout: governance (rules + the constitution ladder) and
// relay (watch · approve · decide · chat).
//
// Shapes are intentionally flat and self-describing so the kit reads them
// without a schema. Token-only, theme-free. window.DOJO2.

window.DOJO2 = {
  /* ── the viewer ─────────────────────────────────────────── */
  me: { name: "Rin Saito", handle: "rin", avatar: "R" },

  /* ── my dōjōs · additive roles ──────────────────────────── */
  // role: developer (read-mostly) · maintainer (governance) · lead (clients)
  //       · admin (member roles + policies). Roles are cumulative.
  roles: {
    developer:  { kanji: "士", label: "developer",  note: "read-mostly · watches & chats" },
    maintainer: { kanji: "掟", label: "maintainer", note: "authors governance · triages" },
    lead:       { kanji: "客", label: "lead",       note: "client engagements & audit" },
    admin:      { kanji: "任", label: "admin",      note: "member roles & policies" },
  },
  dojos: [
    { slug: "acme",    kanji: "社", name: "Acme Corp",   kind: "employer",  role: "admin",
      route: "sensei-hq.com/acme",    members: 48, projects: 9, needs: 4, since: "2y" },
    { slug: "globex",  kanji: "客", name: "Globex",      kind: "client",    role: "lead",
      route: "sensei-hq.com/globex",  members: 12, projects: 3, needs: 2, since: "7mo" },
    { slug: "initech", kanji: "客", name: "Initech",     kind: "client",    role: "maintainer",
      route: "sensei-hq.com/initech", members: 21, projects: 4, needs: 1, since: "1y" },
    { slug: "rustco",  kanji: "群", name: "Rust Guild",  kind: "community", role: "developer",
      route: "sensei-hq.com/rust-guild", members: 340, projects: 18, needs: 0, since: "3mo" },
  ],

  /* ── my work · projects in flight (across every dōjō + solo) ─ */
  // classification: company · client · personal · community
  // phase: watch → notice → adopt
  projects: [
    { id: "auth",   kanji: "件", name: "lumen-auth",     repo: "acme/lumen-auth",
      dojo: "acme",   dojoName: "Acme Corp",   classification: "company",
      phase: "notice", role: "maintainer", lastRun: "8m", runsWeek: 14,
      spark: [3, 5, 4, 8, 6, 11, 14], needs: 2, locks: 3, levels: 4,
      note: "3 patterns surfacing in payments paths" },
    { id: "globex", kanji: "件", name: "globex-portal",  repo: "globex/portal",
      dojo: "globex", dojoName: "Globex",      classification: "client",
      phase: "watch",  role: "lead", lastRun: "31m", runsWeek: 6,
      spark: [1, 2, 2, 4, 3, 5, 6], needs: 1, locks: 5, levels: 4,
      note: "engagement · sources dereferenced" },
    { id: "ledger", kanji: "件", name: "ledger-core",    repo: "acme/ledger-core",
      dojo: "acme",   dojoName: "Acme Corp",   classification: "company",
      phase: "adopt",  role: "developer", lastRun: "2h", runsWeek: 9,
      spark: [6, 7, 5, 9, 8, 9, 9], needs: 0, locks: 4, levels: 3,
      note: "idempotency pattern adopted org-wide" },
    { id: "site",   kanji: "件", name: "personal-site",  repo: "rin/personal-site",
      dojo: null,     dojoName: null,          classification: "personal",
      phase: "watch",  role: null, lastRun: "1d", runsWeek: 3,
      spark: [2, 1, 3, 2, 4, 2, 3], needs: 0, locks: 0, levels: 3,
      note: "no dōjō · your ladder alone" },
    { id: "mono",   kanji: "件", name: "agency-monorepo", repo: "studio/monorepo",
      dojo: "initech", dojoName: "Initech",    classification: "client",
      phase: "notice", role: "maintainer", lastRun: "4h", runsWeek: 7,
      spark: [4, 3, 6, 5, 7, 6, 7], needs: 1, locks: 6, levels: 4,
      note: "two client rungs resolve here" },
  ],

  /* ── needs-you band · things waiting on the viewer ──────── */
  // kind: gate (approve a command) · conflict (settle a rule clash)
  //       · decision (sign off) · review (second approval)
  needsYou: [
    { id: "n1", kind: "gate", kanji: "令", project: "lumen-auth", dojo: "Acme Corp",
      title: "run migration against staging db",
      cmd: "pnpm db:migrate --env=staging", age: "3m",
      why: "touches an auth-boundary schema · guarded" },
    { id: "n2", kind: "conflict", kanji: "争", project: "agency-monorepo", dojo: "Initech",
      title: "retry policy clashes with idempotency rule",
      age: "26m", why: "Company ‘retry freely’ vs Team ‘idempotency key required’" },
    { id: "n3", kind: "decision", kanji: "決", project: "globex-portal", dojo: "Globex",
      title: "adopt ‘verify webhook signature’ as a client guard",
      age: "1h", why: "4 sessions · dereferenced · confidence 0.91" },
    { id: "n4", kind: "review", kanji: "承", project: "lumen-auth", dojo: "Acme Corp",
      title: "second approval · ‘never log refresh tokens’",
      age: "2h", why: "Keiko approved · needs one more maintainer" },
  ],

  /* ── live runs · sessions in progress right now ─────────── */
  // A run is a *plan graph*: ordered stages, each holding tasks that run either
  // in parallel (all start together) or in sequence (each waits on the one
  // before). Node state ∈ done · running · gate · queued · blocked · failed.
  // A run's plan_graph: { goal, phases: [{ title, tasks: [...] }] }. A task is
  // { id, title, agent, model, spec_ref, summary, state, deps, is_gate,
  //   gate_severity }. state ∈ pending · active · done · skipped · failed ·
  //   blocked · needs_review. deps hold task ids — the ready-set (and therefore
  //   what runs in parallel) is derived, never authored.
  runs: [
    { id: "s-2891", project: "lumen-auth",   assistant: "claude-sonnet", state: "running",
      task: "refactor refresh-token rotation", elapsed: "38m", edits: 12, gate: true,
      corrections: 3, kanji: "観", last: "2m",
      feed: [
        { icon: "pen-new-square", tone: "var(--ink-mute)", text: "Wrote redaction-sink.ts · 38 lines", at: "14:08" },
        { icon: "shield-warning", tone: "var(--accent)", text: "Paused for approval · staging migration", at: "14:04" },
        { icon: "test-tube", tone: "var(--success)", text: "12 tests pass · 0 fail", at: "13:58" },
        { icon: "document-text", tone: "var(--ink-mute)", text: "Read auth/rotation.ts · 6 call sites", at: "13:41" },
      ],
      plan: {
        goal: "Rotate refresh tokens without leaking them to the logger",
        phases: [
          { title: "Survey", tasks: [
            { id: "t1", title: "Read auth module", agent: "general-purpose", model: "haiku", spec_ref: "specs/auth-rotation.md#surface", summary: "2m", state: "done", deps: [] },
            { id: "t2", title: "Map refresh call sites", agent: "general-purpose", model: "haiku", spec_ref: "specs/auth-rotation.md#surface", summary: "3m · 14 sites", state: "done", deps: [] },
          ]},
          { title: "Plan", tasks: [
            { id: "t3", title: "Draft rotation strategy", agent: "architect", model: "opus", spec_ref: "specs/auth-rotation.md#strategy", summary: "4m", state: "done", deps: ["t1", "t2"] },
            { id: "t4", title: "Check against auth guards", agent: "rule-checker", model: "sonnet", spec_ref: "constitution/company.md#auth", summary: "1m · 8 rules", state: "done", deps: ["t3"] },
          ]},
          { title: "Implement", tasks: [
            { id: "t5", title: "Rotate issuer + store", agent: "general-purpose", model: "sonnet", spec_ref: "specs/auth-rotation.md#issuer", summary: "9m · 6 edits", state: "done", deps: ["t4"] },
            { id: "t6", title: "Route debug line to redaction sink", agent: "general-purpose", model: "sonnet", spec_ref: "specs/auth-rotation.md#logging", summary: "4m · 3 edits", state: "active", deps: ["t4"] },
            { id: "t7", title: "Update device-code flow", agent: "general-purpose", model: "sonnet", spec_ref: "specs/auth-rotation.md#device-code", summary: "6m · 3 edits", state: "active", deps: ["t4"] },
          ]},
          { title: "Verify", tasks: [
            { id: "t8", title: "Unit tests", agent: "test-runner", model: "haiku", spec_ref: "specs/auth-rotation.md#tests", state: "pending", deps: ["t5", "t6", "t7"] },
            { id: "t9", title: "Integration test — device code", agent: "test-runner", model: "sonnet", spec_ref: "constitution/project.md#refresh-flow", summary: "project rung asks for it", state: "pending", deps: ["t7"] },
            { id: "t10", title: "Lint + types", agent: "test-runner", model: "haiku", state: "pending", deps: ["t5", "t6", "t7"] },
          ]},
          { title: "Ship", tasks: [
            { id: "t11", title: "Staging migration", agent: "migrator", model: "sonnet", spec_ref: "specs/auth-rotation.md#migration", summary: "needs your approval", state: "blocked", deps: ["t8", "t9", "t10"], is_gate: true, gate_severity: "blocking" },
            { id: "t12", title: "Write session note", agent: "scribe", model: "haiku", state: "pending", deps: ["t11"] },
          ]},
        ],
      } },
    { id: "s-2890", project: "agency-monorepo", assistant: "claude-opus", state: "waiting",
      task: "wire initech billing webhook", elapsed: "12m", edits: 4, gate: true,
      corrections: 0, kanji: "観", last: "9m",
      feed: [
        { icon: "shield-warning", tone: "var(--accent)", text: "Signature verification needs client sign-off", at: "11:52" },
        { icon: "pen-new-square", tone: "var(--ink-mute)", text: "Wrote webhook/receiver.ts · 61 lines", at: "11:47" },
        { icon: "link-round", tone: "var(--ink-mute)", text: "Read billing adapter · initech scope", at: "11:40" },
      ],
      plan: {
        goal: "Receive Initech billing webhooks safely inside the client scope",
        phases: [
          { title: "Survey", tasks: [
            { id: "t1", title: "Read billing adapter", agent: "general-purpose", model: "haiku", spec_ref: "specs/billing-webhook.md#adapter", summary: "3m", state: "done", deps: [] },
          ]},
          { title: "Implement", tasks: [
            { id: "t2", title: "Webhook receiver", agent: "general-purpose", model: "sonnet", spec_ref: "specs/billing-webhook.md#receiver", summary: "5m · 4 edits", state: "done", deps: ["t1"] },
            { id: "t3", title: "Signature verification", agent: "general-purpose", model: "opus", spec_ref: "constitution/client.md#webhook-signature", summary: "client guard — needs sign-off", state: "needs_review", deps: ["t1"], is_gate: true, gate_severity: "blocking" },
            { id: "t4", title: "Retry + backoff", agent: "general-purpose", model: "sonnet", spec_ref: "specs/billing-webhook.md#retry", summary: "waits on signature verification", state: "blocked", deps: ["t3"] },
          ]},
          { title: "Verify", tasks: [
            { id: "t5", title: "Replay fixture suite", agent: "test-runner", model: "haiku", state: "pending", deps: ["t4"] },
            { id: "t6", title: "Confidentiality check", agent: "rule-checker", model: "sonnet", spec_ref: "constitution/client.md#identifiers", summary: "client scope", state: "pending", deps: ["t4"] },
          ]},
        ],
      } },
    { id: "s-2887", project: "ledger-core",   assistant: "claude-sonnet", state: "running",
      task: "add idempotency keys to ledger writes", elapsed: "1h 4m", edits: 27, gate: false,
      corrections: 1, kanji: "観", last: "6m",
      feed: [
        { icon: "close-circle", tone: "var(--danger)", text: "Retry handler failed · double-submit conflict", at: "14:02" },
        { icon: "pen-new-square", tone: "var(--ink-mute)", text: "Wrote ledger/refunds.ts · 5 edits", at: "13:55" },
        { icon: "database", tone: "var(--success)", text: "Backfill complete · 1.2M rows", at: "13:31" },
        { icon: "database", tone: "var(--ink-mute)", text: "Added idempotency_key column", at: "13:09" },
      ],
      plan: {
        goal: "Every money-moving write carries an idempotency key",
        phases: [
          { title: "Survey", tasks: [
            { id: "t1", title: "Map ledger write paths", agent: "general-purpose", model: "haiku", spec_ref: "specs/idempotency.md#paths", summary: "6m · 9 paths", state: "done", deps: [] },
            { id: "t2", title: "Read payments pack", agent: "rule-checker", model: "haiku", spec_ref: "packs/payments.md", summary: "2m · 6 rules", state: "done", deps: [] },
          ]},
          { title: "Migrate", tasks: [
            { id: "t3", title: "Add key column", agent: "migrator", model: "sonnet", spec_ref: "specs/idempotency.md#schema", summary: "8m", state: "done", deps: ["t1"] },
            { id: "t4", title: "Backfill existing rows", agent: "migrator", model: "sonnet", spec_ref: "specs/idempotency.md#backfill", summary: "21m · 1.2M rows", state: "done", deps: ["t3"] },
            { id: "t5", title: "Add unique constraint", agent: "migrator", model: "sonnet", spec_ref: "specs/idempotency.md#constraint", summary: "4m", state: "done", deps: ["t4"] },
          ]},
          { title: "Implement", tasks: [
            { id: "t6", title: "Write path — charges", agent: "general-purpose", model: "sonnet", spec_ref: "specs/idempotency.md#charges", summary: "11m · 8 edits", state: "done", deps: ["t5"] },
            { id: "t7", title: "Write path — refunds", agent: "general-purpose", model: "sonnet", spec_ref: "specs/idempotency.md#refunds", summary: "7m · 5 edits", state: "active", deps: ["t5"] },
            { id: "t8", title: "Retry handler", agent: "general-purpose", model: "sonnet", spec_ref: "specs/idempotency.md#retry", summary: "conflict on double-submit", state: "failed", deps: ["t5"] },
          ]},
          { title: "Verify", tasks: [
            { id: "t9", title: "Ledger reconciliation", agent: "test-runner", model: "sonnet", summary: "waits on retry handler", state: "blocked", deps: ["t8"] },
            { id: "t10", title: "Load replay", agent: "test-runner", model: "haiku", state: "pending", deps: ["t9"] },
          ]},
        ],
      } },
    { id: "s-2884", project: "globex-portal", assistant: "claude-sonnet", state: "waiting",
      task: "harden client webhook intake", elapsed: "2h 11m", edits: 9, gate: false,
      corrections: 0, kanji: "観", last: "1h",
      feed: [
        { icon: "checklist-minimalistic", tone: "var(--accent)", text: "Candidate rule waiting on your sign-off", at: "13:10" },
        { icon: "pen-new-square", tone: "var(--ink-mute)", text: "Wrote intake/verify.ts · 9 edits", at: "12:58" },
      ],
      plan: {
        goal: "Make the client intake path verify before it parses",
        phases: [
          { title: "Survey", tasks: [
            { id: "t1", title: "Read client rung", agent: "rule-checker", model: "haiku", spec_ref: "constitution/client.md", summary: "2m · 2 rules", state: "done", deps: [] },
          ]},
          { title: "Implement", tasks: [
            { id: "t2", title: "Signature verify helper", agent: "general-purpose", model: "sonnet", spec_ref: "specs/intake.md#verify", summary: "14m · 9 edits", state: "done", deps: ["t1"] },
            { id: "t3", title: "Intake replay guard", agent: "general-purpose", model: "sonnet", spec_ref: "specs/intake.md#replay", state: "pending", deps: ["t2"] },
          ]},
          { title: "Adopt", tasks: [
            { id: "t4", title: "Promote guard to Client rung", agent: "rule-checker", model: "opus", spec_ref: "constitution/client.md#webhook-signature", summary: "needs your decision", state: "needs_review", deps: ["t2"], is_gate: true, gate_severity: "advisory" },
          ]},
        ],
      } },
    { id: "s-2882", project: "api-gateway", assistant: "claude-opus", state: "stalled",
      task: "split rate limiter per tenant", elapsed: "3h 02m", edits: 6, gate: false,
      corrections: 0, kanji: "観", last: "47m", stale: true,
      feed: [
        { icon: "hourglass", tone: "var(--warning)", text: "No heartbeat for 47m — the assistant went quiet", at: "12:20" },
        { icon: "pen-new-square", tone: "var(--ink-mute)", text: "Wrote gateway/limiter.ts · 6 edits", at: "11:33" },
      ],
      plan: {
        goal: "One rate-limit bucket per tenant, not per gateway",
        phases: [
          { title: "Survey", tasks: [
            { id: "t1", title: "Map limiter call sites", agent: "general-purpose", model: "haiku", spec_ref: "specs/limiter.md#sites", summary: "5m · 7 sites", state: "done", deps: [] },
            { id: "t2", title: "Read platform pack", agent: "rule-checker", model: "haiku", spec_ref: "packs/platform.md", summary: "2m", state: "done", deps: [] },
          ]},
          { title: "Implement", tasks: [
            { id: "t3", title: "Per-tenant bucket store", agent: "general-purpose", model: "opus", spec_ref: "specs/limiter.md#store", summary: "quiet since 12:20", state: "active", deps: ["t1", "t2"] },
            { id: "t4", title: "Burst window", agent: "general-purpose", model: "sonnet", spec_ref: "specs/limiter.md#burst", state: "pending", deps: ["t3"] },
          ]},
        ],
      } },
    { id: "s-2879", project: "acme-web", assistant: "claude-sonnet", state: "done",
      task: "move settings page to the new form kit", elapsed: "52m", edits: 31, gate: false,
      corrections: 0, kanji: "観", last: "yesterday",
      feed: [
        { icon: "check-circle", tone: "var(--success)", text: "Session note written · 2 learnings offered", at: "17:41" },
        { icon: "test-tube", tone: "var(--success)", text: "24 tests pass · 0 fail", at: "17:28" },
      ],
      plan: {
        goal: "Settings page runs on the shared form kit",
        phases: [
          { title: "Implement", tasks: [
            { id: "t1", title: "Port settings form", agent: "general-purpose", model: "sonnet", spec_ref: "specs/form-kit.md#settings", summary: "31m · 24 edits", state: "done", deps: [] },
            { id: "t2", title: "Port validation", agent: "general-purpose", model: "sonnet", spec_ref: "specs/form-kit.md#validation", summary: "9m · 7 edits", state: "done", deps: [] },
            { id: "t3", title: "Drop legacy field wrapper", agent: "general-purpose", model: "haiku", summary: "superseded by the kit", state: "skipped", deps: ["t1"] },
          ]},
          { title: "Verify", tasks: [
            { id: "t4", title: "Unit tests", agent: "test-runner", model: "haiku", summary: "24 pass", state: "done", deps: ["t1", "t2"] },
            { id: "t5", title: "Write session note", agent: "scribe", model: "haiku", state: "done", deps: ["t4"] },
          ]},
        ],
      } },
  ],

  /* ── governance · the constitution ladder ───────────────── */
  // Rungs broad → specific. Each rule: {kanji, text, level, hard?} where
  // level is the rung it entered from; hard = ★ non-negotiable (locks).
  ladder: [
    { id: "company", kanji: "社", scope: "Company", name: "Acme Corp",
      caption: "your employer · every project", tone: "ink",
      rules: [
        { kanji: "守", text: "No secrets in source — vault only, never .env in git", hard: true },
        { kanji: "守", text: "Never log tokens or PII, even at debug level", hard: true },
        { kanji: "理", text: "Public APIs stay backward-compatible two minor versions" },
        { kanji: "検", text: "Coverage ≥ 80% on money- or auth-touching paths", hard: true },
      ] },
    { id: "client", kanji: "客", scope: "Client", name: "Globex",
      caption: "engagement rung · switches on for client repos", tone: "accent",
      rules: [
        { kanji: "盾", text: "Verify webhook signatures before parsing the body", hard: true },
        { kanji: "盾", text: "Client identifiers never leave the machine — derived only" },
      ] },
    { id: "personal", kanji: "己", scope: "Personal", name: "Rin Saito",
      caption: "your standing preferences · every project", tone: "ink",
      rules: [
        { kanji: "己", text: "Explain the plan before editing more than three files" },
        { kanji: "己", text: "Prefer small, reviewable commits over one large diff" },
      ] },
    { id: "project", kanji: "件", scope: "Project", name: "lumen-auth",
      caption: "this repo only", tone: "ink",
      rules: [
        { kanji: "紋", text: "Every money-moving mutation carries an idempotency key", hard: true },
        { kanji: "検", text: "Integration test required for any refresh-flow change" },
      ] },
    { id: "stack", kanji: "技", scope: "Stack", name: "React · TypeScript",
      caption: "most specific · refines everything above", tone: "ink",
      rules: [
        { kanji: "技", text: "No default exports in shared packages" },
        { kanji: "技", text: "Server state through the query layer, never in a store" },
      ] },
  ],

  /* ── conflicts settled by the ladder (topic · winner · why) ─ */
  conflicts: [
    { id: "cf1", topic: "retry behaviour on money-moving calls",
      loser: { level: "Company", text: "retry freely on transient failure" },
      winner: { level: "Project", text: "idempotency key required before retry" },
      why: "More specific scope refines the broader one — Project > Company.", locked: false },
    { id: "cf2", topic: "logging verbosity in auth boundary",
      loser: { level: "Stack", text: "debug-log request/response bodies" },
      winner: { level: "Company", text: "never log tokens or PII (★)" },
      why: "A non-negotiable locks — no narrower scope can relax it.", locked: true },
  ],

  /* ── stance dial · three axes the viewer sets per scope ──── */
  stance: [
    { id: "autonomy", kanji: "任", label: "autonomy",
      caption: "how far a session runs before it asks",
      levels: ["ask always", "ask on guarded", "ask on risky", "run freely"], value: 1 },
    { id: "sharing", kanji: "共", label: "sharing",
      caption: "what surfaces to the dōjō",
      levels: ["private", "patterns only", "patterns + prompts", "everything derived"], value: 1 },
    { id: "review", kanji: "検", label: "review",
      caption: "who signs off before a rule adopts",
      levels: ["me alone", "one maintainer", "two maintainers", "quorum"], value: 2 },
  ],

  /* ── rule packs · adoptable bundles (NOT ‘library’) ─────── */
  rulePacks: [
    { id: "p1", kanji: "守", name: "Auth boundary guards", by: "Acme · platform",
      count: 8, adopted: true, note: "token redaction, signature checks, secret scanning",
      rules: [
        { title: "Never log tokens, refresh tokens, or PII", tone: "guard" },
        { title: "Verify request signatures before handling any payload", tone: "guard" },
        { title: "Scan diffs for hardcoded secrets before commit", tone: "guard" },
        { title: "Rotate signing keys on a fixed schedule", tone: "norm" },
        { title: "Enforce short-lived access tokens with refresh", tone: "norm" },
        { title: "Reject unsigned or expired webhooks", tone: "guard" },
        { title: "Redact tokens in error traces and crash reports", tone: "guard" },
        { title: "Store secrets in the vault, never in env files", tone: "norm" },
      ] },
    { id: "p2", kanji: "紋", name: "Payments patterns", by: "Acme · payments",
      count: 6, adopted: true, note: "idempotency, ledger writes, reconciliation",
      rules: [
        { title: "Require an idempotency key before any retry", tone: "guard" },
        { title: "Write ledger entries in a single transaction", tone: "guard" },
        { title: "Reconcile against the processor daily", tone: "norm" },
        { title: "Never mutate a settled charge — issue a reversal", tone: "guard" },
        { title: "Record currency and amount in minor units", tone: "norm" },
        { title: "Emit an audit event on every state change", tone: "norm" },
      ] },
    { id: "p3", kanji: "技", name: "React · TypeScript baseline", by: "Rust Guild",
      count: 11, adopted: false, note: "exports, query layer, suspense boundaries",
      rules: [
        { title: "Prefer named exports over default exports", tone: "norm" },
        { title: "No any — narrow or use unknown", tone: "norm" },
        { title: "Data fetching goes through the query layer", tone: "norm" },
        { title: "Wrap async views in a suspense boundary", tone: "norm" },
        { title: "Co-locate types with the component that owns them", tone: "norm" },
        { title: "No inline styles for theme values — use tokens", tone: "norm" },
        { title: "Derive state; don't duplicate server data in state", tone: "norm" },
        { title: "Every effect has a cleanup or a comment why not", tone: "norm" },
        { title: "Keys on lists are stable ids, never the index", tone: "guard" },
        { title: "Props are readonly — no mutation in children", tone: "norm" },
        { title: "Error boundaries wrap every route root", tone: "norm" },
      ] },
    { id: "p4", kanji: "盾", name: "Client engagement shield", by: "Globex · lead",
      count: 5, adopted: true, note: "dereferencing, webhook verification, audit trail",
      rules: [
        { title: "De-reference client PII before it leaves the boundary", tone: "guard" },
        { title: "Verify inbound webhooks against the shared secret", tone: "guard" },
        { title: "Keep an append-only audit trail per engagement", tone: "guard" },
        { title: "Scope credentials to a single client, never shared", tone: "guard" },
        { title: "Purge client data on engagement close", tone: "norm" },
      ] },
    { id: "p5", kanji: "理", name: "API compatibility", by: "Acme · platform",
      count: 4, adopted: false, note: "deprecation windows, versioning, changelog gates",
      rules: [
        { title: "Ship a 90-day deprecation window on breaking changes", tone: "norm" },
        { title: "Version the API in the path, never the header only", tone: "norm" },
        { title: "A changelog entry gates every public-surface change", tone: "guard" },
        { title: "Additive changes only within a major version", tone: "norm" },
      ] },
  ],

  /* ── asks · what a running session can't decide alone ───── */
  // One per blocking question raised by a run. kind: approval (a guarded
  // action needs your yes) · choice (a fork the model won't pick) · recovery
  // (the run stopped and needs a direction). Each names the task it blocks.
  asks: [
    { id: "a1", run: "s-2891", task: "t11", taskTitle: "Staging migration", kind: "approval", severity: "blocking", age: "3m",
      question: "Run the staging migration?",
      context: "pnpm db:migrate --env=staging · touches an auth-boundary schema · the company rung requires a human yes",
      options: ["Run it", "Dry-run first", "Skip the migration"] },
    { id: "a2", run: "s-2891", task: "t6", taskTitle: "Route debug line to redaction sink", kind: "choice", severity: "blocking", age: "11m",
      question: "Two debug lines still print the refresh token. What should happen to them?",
      context: "Company rung forbids logging tokens; sensei won't pick between redacting and deleting your logging.",
      options: ["Route through the redaction sink", "Delete the lines", "Keep them — they are dev-only"] },
    { id: "a3", run: "s-2890", task: "t3", taskTitle: "Signature verification", kind: "choice", severity: "blocking", age: "9m",
      question: "Where does the Initech signing secret come from?",
      context: "No secret is in scope for this repo, and the client rung forbids inventing one.",
      options: ["Vault · initech/webhook-secret", "Ask the client for it", "Stub it — local only"] },
    { id: "a4", run: "s-2884", task: "t3", taskTitle: "Intake replay guard", kind: "choice", severity: "advisory", age: "1h",
      question: "How long should the replay window be?",
      context: "The client's spec doesn't say. sensei will not guess a security window.",
      options: ["5 minutes", "1 hour", "Follow the client doc I paste"] },
    { id: "a5", run: "s-2887", task: "t8", taskTitle: "Retry handler", kind: "recovery", severity: "blocking", age: "6m",
      question: "The retry handler conflicts on double-submit. How should retries behave?",
      context: "Two writes with the same idempotency key arrived 40ms apart; both paths are defensible.",
      options: ["Dedupe on the key", "Fail fast and surface", "Queue and retry later"] },
    { id: "a6", run: "s-2882", task: "t3", taskTitle: "Per-tenant bucket store", kind: "recovery", severity: "blocking", age: "47m",
      question: "No heartbeat for 47 minutes. What should sensei do with this run?",
      context: "The assistant went quiet mid-task; nothing has been written since 12:20.",
      options: ["Resume where it stopped", "Restart the phase", "Stop the run"] },
    { id: "a7", run: "s-2891", task: "t7", taskTitle: "Update device-code flow", kind: "clarification", severity: "advisory", age: "18m",
      question: "Found a second refresh-token store in the device-code flow. Is it in scope?",
      context: "Undocumented, written by the CLI path, and it rotates on its own schedule — nothing in the plan covers it.",
      options: ["Include it in this change", "Leave it — open a follow-up", "Stop and let me look"] },
    { id: "a8", run: "s-2884", task: "t2", taskTitle: "Signature verify helper", kind: "clarification", severity: "advisory", age: "40m",
      question: "The client sends two signature headers with different algorithms. Which one is authoritative?",
      context: "Both verify against the shared secret; the docs you gave sensei mention only one.",
      options: ["The v2 header", "The legacy header", "Accept either for now"] },
  ],

  /* ── relay · runs / gates / decisions / chat ────────────── */
  gates: [
    { id: "g1", project: "lumen-auth", cmd: "pnpm db:migrate --env=staging",
      kind: "command", risk: "guarded", why: "touches an auth-boundary schema",
      session: "s-2891", age: "3m" },
    { id: "g2", project: "agency-monorepo", cmd: "gh secret set STRIPE_KEY",
      kind: "secret", risk: "high", why: "writes a secret — company guard requires approval",
      session: "s-2890", age: "1m" },
  ],
  decisions: [
    { id: "d1", project: "globex-portal", kanji: "決",
      title: "adopt ‘verify webhook signature’ as a client guard",
      options: ["adopt to Client rung", "keep as project note", "decline"],
      context: "4 sessions · dereferenced · confidence 0.91", age: "1h", session: "s-2884" },
    { id: "d2", project: "lumen-auth", kanji: "決",
      title: "promote ‘idempotency key’ from Project to Company",
      options: ["promote to Company", "keep at Project", "decline"],
      context: "adopted in 6 repos · no conflicts", age: "5h", session: "s-2891" },
  ],
  chat: {
    project: "lumen-auth", session: "s-2891",
    thread: [
      { who: "sensei", kanji: "先", text: "Noticed the refresh-token rotation touches the logger. There is a company guard against logging tokens — want me to route the debug line through the redaction sink?", when: "38m" },
      { who: "rin", text: "Yes. And add the integration test the project rung asks for.", when: "36m" },
      { who: "sensei", kanji: "先", text: "Drafting the test against the device-code flow. One command needs approval to run the staging migration — it is in your needs-you band.", when: "35m" },
      { who: "rin", text: "Approving now.", when: "3m" },
    ],
  },

  /* ── org context · projects in a dōjō's jurisdiction ────── */
  orgProjects: {
    acme: [
      { id: "auth",   kanji: "件", name: "lumen-auth",  team: "Payments", classification: "company", phase: "notice", maintainers: 3, runsWeek: 14, needs: 2 },
      { id: "ledger", kanji: "件", name: "ledger-core", team: "Payments", classification: "company", phase: "adopt",  maintainers: 2, runsWeek: 9,  needs: 0 },
      { id: "gw",     kanji: "件", name: "api-gateway", team: "Platform", classification: "company", phase: "watch",  maintainers: 4, runsWeek: 5,  needs: 1 },
      { id: "web",    kanji: "件", name: "acme-web",    team: "Web",      classification: "company", phase: "notice", maintainers: 2, runsWeek: 8,  needs: 1 },
    ],
  },

  /* ── org · the dōjō's OWN authored constitution, by section ─
     A dōjō authors rules at the scopes it owns: company-wide, per team,
     per stack (stacks also adopt rule packs). This is NOT the resolution
     ladder — that only appears at project preview time. */
  orgConstitution: {
    acme: [
      { id: "company", kanji: "社", scope: "Company-wide", group: "Company", caption: "every project in the dōjō",
        rules: [
          { kanji: "守", text: "No secrets in source — vault only, never .env in git", hard: true },
          { kanji: "守", text: "Never log tokens or PII, even at debug level", hard: true },
          { kanji: "理", text: "Public APIs stay backward-compatible two minor versions" },
          { kanji: "検", text: "Coverage ≥ 80% on money- or auth-touching paths", hard: true },
        ] },
      { id: "team-pay", kanji: "組", scope: "Payments", group: "Teams", caption: "payments · ledger repos",
        rules: [
          { kanji: "紋", text: "Every money-moving mutation carries an idempotency key", hard: true },
          { kanji: "検", text: "Reconciliation job runs before any ledger migration" },
        ] },
      { id: "team-plat", kanji: "組", scope: "Platform", group: "Teams", caption: "platform · API · gateway",
        rules: [
          { kanji: "理", text: "Every public endpoint carries a deprecation policy" },
          { kanji: "守", text: "Rate-limit and auth-check at the gateway, not the service" },
        ] },
      { id: "stack-react", kanji: "技", scope: "React · TypeScript", group: "Stacks", caption: "adopted packs + rules",
        packs: ["React · TypeScript baseline"],
        rules: [
          { kanji: "技", text: "No default exports in shared packages" },
          { kanji: "技", text: "Server state through the query layer, never in a store" },
        ] },
      { id: "stack-pg", kanji: "技", scope: "Postgres", group: "Stacks", caption: "no pack adopted yet",
        packs: [],
        rules: [
          { kanji: "技", text: "Every migration is reversible or ships a documented backout" },
        ] },
    ],
  },

  /* ── org · teams · the collective view ───────────────────
     A team is a group of developers working the same projects. Each dev runs
     sensei locally; what they SEND up (learnings · project memory · instruments
     · first-try counts) is what the dōjō can see. Metrics roll up per team and
     drill down per person. Nothing arrives that a developer didn't send. */
  teams: {
    acme: [
      { id: "payments", kanji: "組", name: "Payments", caption: "lumen-auth · ledger-core",
        lead: "Marco Diaz", members: 9, projects: 2,
        ftr: 78, ftrDelta: 6, sessions: 41, corrections: 1.4, correctionsDelta: -0.3, governed: 92,
        spark: [58, 61, 64, 62, 69, 71, 74, 78],
        memory: { learnings: 34, memories: 118, rules: 12, instruments: 6 },
        inflow: { sent: 23, triage: 4, adopted: 17, declined: 2 },
        people: [
          { name: "Marco Diaz",    role: "maintainer", sessions: 11, ftr: 84, corrections: 0.9, sent: 7, adopted: 6, last: "12m" },
          { name: "Rin Saito",     role: "maintainer", sessions: 9,  ftr: 81, corrections: 1.1, sent: 5, adopted: 4, last: "now", you: true },
          { name: "Aiko Nakamura", role: "developer",  sessions: 8,  ftr: 76, corrections: 1.5, sent: 6, adopted: 4, last: "1h" },
          { name: "Ben Osei",      role: "developer",  sessions: 7,  ftr: 71, corrections: 1.9, sent: 3, adopted: 2, last: "4h" },
          { name: "Tom Becker",    role: "developer",  sessions: 6,  ftr: 68, corrections: 2.2, sent: 2, adopted: 1, last: "5d" },
        ] },
      { id: "platform", kanji: "組", name: "Platform", caption: "api-gateway · shared infra",
        lead: "Sven Karlsson", members: 7, projects: 1,
        ftr: 71, ftrDelta: 2, sessions: 28, corrections: 1.8, correctionsDelta: -0.1, governed: 74,
        spark: [64, 62, 66, 65, 68, 67, 70, 71],
        memory: { learnings: 21, memories: 74, rules: 9, instruments: 4 },
        inflow: { sent: 14, triage: 3, adopted: 9, declined: 2 },
        people: [
          { name: "Sven Karlsson", role: "maintainer", sessions: 9, ftr: 79, corrections: 1.2, sent: 6, adopted: 5, last: "3h" },
          { name: "Priya Raman",   role: "developer",  sessions: 8, ftr: 73, corrections: 1.6, sent: 4, adopted: 3, last: "2h" },
          { name: "Jonas Weber",   role: "developer",  sessions: 6, ftr: 66, corrections: 2.3, sent: 3, adopted: 1, last: "1d" },
          { name: "Lena Fischer",  role: "developer",  sessions: 5, ftr: 64, corrections: 2.4, sent: 1, adopted: 0, last: "2d" },
        ] },
      { id: "web", kanji: "組", name: "Web", caption: "acme-web",
        lead: "Rin Saito", members: 5, projects: 1,
        ftr: 83, ftrDelta: 9, sessions: 22, corrections: 0.8, correctionsDelta: -0.6, governed: 96,
        spark: [66, 68, 71, 74, 76, 79, 81, 83],
        memory: { learnings: 27, memories: 91, rules: 7, instruments: 8 },
        inflow: { sent: 19, triage: 1, adopted: 16, declined: 2 },
        people: [
          { name: "Rin Saito",   role: "maintainer", sessions: 8, ftr: 88, corrections: 0.5, sent: 8, adopted: 7, last: "now", you: true },
          { name: "Hana Kim",    role: "developer",  sessions: 7, ftr: 84, corrections: 0.7, sent: 6, adopted: 5, last: "40m" },
          { name: "Diego Ortiz", role: "developer",  sessions: 7, ftr: 78, corrections: 1.2, sent: 5, adopted: 4, last: "6h" },
        ] },
      { id: "data", kanji: "組", name: "Data", caption: "no project bound yet",
        lead: null, members: 4, projects: 0,
        ftr: null, ftrDelta: null, sessions: 0, corrections: null, correctionsDelta: null, governed: 0,
        spark: null,
        memory: { learnings: 0, memories: 0, rules: 0, instruments: 0 },
        inflow: { sent: 0, triage: 0, adopted: 0, declined: 0 },
        people: [] },
    ],
  },

  /* ── org · what arrived from local sensei installs ────────
     kind: learning (a noticed pattern, as a candidate rule) · memory (a
     decision or gotcha kept with the repo) · instrument (a skill, agent or
     command a dev built) · metric (counts only — never code or prompts). */
  teamInflow: [
    { id: "f1", kanji: "紋", kind: "learning", team: "Payments", project: "ledger-core",
      title: "Every money-moving mutation carries an idempotency key", by: "Marco Diaz",
      state: "adopted", when: "2h", note: "seen in 6 sessions · now a Payments rule" },
    { id: "f2", kanji: "覚", kind: "memory", team: "Payments", project: "lumen-auth",
      title: "Refresh tokens rotate on the CLI path too — undocumented store", by: "Rin Saito",
      state: "triage", when: "3h", note: "kept with the repo · offered to the team" },
    { id: "f3", kanji: "具", kind: "instrument", team: "Web", project: "acme-web",
      title: "Skill · port a form to the shared form kit", by: "Hana Kim",
      state: "adopted", when: "1d", note: "used 14 times since · +9pp first-try" },
    { id: "f4", kanji: "盾", kind: "learning", team: "Platform", project: "api-gateway",
      title: "Rate-limit at the gateway, never in the service", by: "Sven Karlsson",
      state: "triage", when: "5h", note: "conflicts with one Platform rule · owner Sven K." },
    { id: "f5", kanji: "数", kind: "metric", team: "Payments", project: "—",
      title: "First-try counts · 41 sessions this week", by: "9 developers",
      state: "counted", when: "live", note: "counts only · no code, prompts or diffs leave the machine" },
    { id: "f6", kanji: "紋", kind: "learning", team: "Platform", project: "api-gateway",
      title: "Prefer a bucket store per tenant over a global limiter", by: "Priya Raman",
      state: "declined", when: "2d", note: "already covered by an adopted pack" },
    { id: "f7", kanji: "覚", kind: "memory", team: "Web", project: "acme-web",
      title: "Legacy field wrapper is superseded — do not extend it", by: "Diego Ortiz",
      state: "adopted", when: "1d", note: "kept with the repo · 5 devs read it" },
  ],

  /* ── what a local sensei sends up (the developer's switch) ── */
  teamSharing: [
    { kanji: "紋", label: "Learnings", note: "patterns sensei noticed, as candidate rules", on: true },
    { kanji: "覚", label: "Project memory", note: "decisions and gotchas kept with the repo", on: true },
    { kanji: "具", label: "Instruments", note: "skills, agents and commands you built", on: true },
    { kanji: "数", label: "First-try counts", note: "counts only — never code, prompts or diffs", on: true },
    { kanji: "刻", label: "Session transcripts", note: "the full conversation — off by default, always", on: false },
  ],

  /* ── org · members / roles for the admin surface ────────── */
  members: [
    { name: "Keiko Tanaka",  git: "Org owner",   role: "admin",      scopes: "all",              active: "now" },
    { name: "Marco Diaz",    git: "Repo admin",  role: "maintainer", scopes: "Payments · Ledger", active: "12m" },
    { name: "Rin Saito",     git: "Repo admin",  role: "maintainer", scopes: "Web · Auth",        active: "now", you: true },
    { name: "Sven Karlsson", git: "Repo admin",  role: "maintainer", scopes: "Platform · API",    active: "3h" },
    { name: "Aiko Nakamura", git: "Write",       role: "developer",  scopes: "Web · Auth",        active: "1h" },
    { name: "Tom Becker",    git: "Read",        role: "developer",  scopes: "—",                 active: "5d" },
  ],

  /* ── contributions · what you've shared upstream + its fate ─
     You propose; a maintainer decides. Client work auto-anonymizes. */
  contributions: {
    mine: [
      { kanji: "紋", title: "Adapter wraps a third-party SDK behind a trait", dest: "Acme Corp", scope: "Stack · Rust", status: "approved", when: "2d", note: "published · +7pp first-try rate" },
      { kanji: "直", title: "Prefer $state(...) over let in Svelte 5 components", dest: "Rust Guild", scope: "Stack · Svelte", status: "pending", when: "6h", note: "in triage · owner Sven K." },
      { kanji: "盾", title: "Verify webhook signature before parsing the body", dest: "Globex", scope: "Client · anonymized", status: "approved", when: "1d", note: "anonymized · shared safely", client: true },
      { kanji: "問", title: "Persona: integration-test author for auth flows", dest: "Acme Corp", scope: "Stack · React", status: "declined", when: "3d", note: "merged into an existing persona" },
    ],
    downstream: [
      { kanji: "守", title: "Never log refresh tokens, even at debug level", from: "Acme Corp", scope: "Company", when: "8m", adopted: false, kind: "guard" },
      { kanji: "紋", title: "Idempotency key on money-moving mutations", from: "Acme Corp", scope: "Team · Payments", when: "4h", adopted: true, kind: "pattern" },
      { kanji: "技", title: "Skill: explain a slow query plan", from: "Rust Guild", scope: "Stack · Postgres", when: "1d", adopted: false, kind: "skill" },
    ],
    stat: { approved: 2, pending: 1, helped: 612 },
  },

  /* ── org · scope ownership · who triages which queue ─────── */
  scopeOwners: {
    acme: [
      { scope: "Company-wide", group: "Company", owner: "Keiko Tanaka", role: "admin", queue: 3, sla: "24h" },
      { scope: "Payments", group: "Teams", owner: "Marco Diaz", role: "maintainer", queue: 5, sla: "12h" },
      { scope: "Platform", group: "Teams", owner: "Sven Karlsson", role: "maintainer", queue: 2, sla: "24h" },
      { scope: "React · TypeScript", group: "Stacks", owner: "Rin Saito", role: "maintainer", queue: 4, sla: "48h" },
      { scope: "Postgres", group: "Stacks", owner: null, role: null, queue: 1, sla: "fallback" },
    ],
  },

  /* ── org · plan & billing (the business model) ──────────── */
  billing: {
    plan: "Team · private", perSeat: 12, seatsActive: 34, seatsReadonly: 14, renews: "Aug 1",
    tiers: [
      { id: "free", kanji: "無", name: "Free", price: "Free", sub: "public · OSS · personal",
        lines: ["Public / open-source or personal solo dōjō", "Unlimited members · full governance authoring", "Relay for your own projects — watch · approve · decide · chat", "Fair use: 1 active machine · standard realtime"] },
      { id: "team", kanji: "組", name: "Team", price: "Per seat", sub: "/ mo · active contributor", current: true,
        lines: ["Private, shared scopes for a company or team", "Role consoles · client engagements · audit", "Relay across the team — shared inbox, presence, priority realtime", "Read-only members always free"] },
      { id: "ent", kanji: "企", name: "Enterprise", price: "Contract", sub: "custom", dark: true,
        lines: ["Self-hosted / VPC · SSO (OIDC / SAML) + SCIM", "Audit retention & export · air-gapped bundle", "Self-hosted relay · SSO on mobile", "SLA & priority support"] },
    ],
    relayRows: [
      { label: "Relay on your own projects — watch · approve · decide · chat", free: true },
      { label: "One active machine · standard realtime · native push", free: true },
      { label: "Shared team inbox & queue · presence (who's handling this)", free: false },
      { label: "Higher concurrency · priority realtime · approval audit trail", free: false },
    ],
    invoices: [
      { d: "Jul 1, 2026", amt: "$408.00", s: "paid" },
      { d: "Jun 1, 2026", amt: "$396.00", s: "paid" },
      { d: "May 1, 2026", amt: "$372.00", s: "paid" },
    ],
  },

  /* ── org role-console data (ported into the dojo2 IA) ───── */
  // Govern (maintainer) · Clients (lead) · Admin
  consoles: {
    // 1 · Triage — candidate learnings awaiting a maintainer decision
    triage: [
      { scope: "Payments", items: [
        { id: "t1", kanji: "紋", title: "Idempotency key on every money-moving mutation", origin: "6 sessions · 3 repos", conf: 0.91, conflicts: 1, dups: 0, impact: "high" },
        { id: "t2", kanji: "検", title: "Reconcile before any ledger migration", origin: "s-2887 · ledger-core", conf: 0.78, conflicts: 0, dups: 2, impact: "normal" },
      ] },
      { scope: "React · TypeScript", items: [
        { id: "t3", kanji: "技", title: "Server state through the query layer, never a store", origin: "11 sessions", conf: 0.86, conflicts: 0, dups: 1, impact: "normal" },
        { id: "t4", kanji: "直", title: "Prefer $state(...) over let in Svelte 5", origin: "Rust Guild mirror", conf: 0.64, conflicts: 0, dups: 0, impact: "low" },
      ] },
      { scope: "Auth boundary", items: [
        { id: "t5", kanji: "守", title: "Never log refresh tokens, even at debug level", origin: "s-2891 · lumen-auth", conf: 0.95, conflicts: 0, dups: 0, impact: "high" },
      ] },
    ],
    // candidate detail (for the selected row)
    candidateDetail: {
      learning: "Every money-moving mutation must carry an idempotency key before retry.",
      cause: "Two sessions retried a charge on a transient 500 and double-posted to the ledger.",
      context: "Surfaced in payments-service across lumen-auth, ledger-core and globex-portal.",
      evidence: ["s-2887 · double-post caught in reconciliation", "s-2871 · manual rollback, 40 min", "3 more sessions"],
      conflict: { loser: "Company · retry freely on transient failure", winner: "Project · idempotency key required" },
      dupOf: null,
      scopes: ["Company", "Team · Payments", "Stack · Node"],
    },
    // 2 · Approvals — second-approval queue for high-impact candidates
    approvals: [
      { id: "a1", kanji: "守", title: "Never log refresh tokens, even at debug level", scope: "Company", first: "Keiko Tanaka", when: "2h", impact: "safety" },
      { id: "a2", kanji: "紋", title: "Promote idempotency key from Project to Company", scope: "Company", first: "Marco Diaz", when: "5h", impact: "high" },
    ],
    // 3 · Knowledge — published library + prune policy; catalog of extensions
    knowledge: {
      prunePolicy: "Prune after 90 days unused",
      active: [
        { kanji: "紋", title: "Idempotency key on money-moving mutations", scope: "Team · Payments", adopted: "6 repos", age: "adopted 3mo" },
        { kanji: "守", title: "Verify webhook signature before parsing", scope: "Client guard", adopted: "3 repos", age: "adopted 1mo" },
        { kanji: "技", title: "No default exports in shared packages", scope: "Stack · React", adopted: "9 repos", age: "adopted 5mo" },
      ],
      pending: [
        { kanji: "理", title: "Deprecation window of two minor versions", scope: "Company", age: "unused 84d" },
      ],
      catalog: [
        { kanji: "問", title: "integration-test author", kind: "agent", scope: "Stack · React" },
        { kanji: "令", title: "explain a slow query plan", kind: "command", scope: "Stack · Postgres" },
        { kanji: "技", title: "auth-boundary reviewer", kind: "skill", scope: "Company" },
      ],
    },
    // 4 · Engagements — client confidentiality
    engagements: [
      { id: "e1", kanji: "客", client: "Globex", projects: "globex-portal · billing", lessons: 86, dropped: 214, since: "7mo", status: "active" },
      { id: "e2", kanji: "客", client: "Initech", projects: "agency-monorepo", lessons: 41, dropped: 97, since: "1y", status: "active" },
    ],
    confidentiality: {
      kept: ["The lesson — a pattern, a guard, a skill", "Anonymized code shape", "Confidence & impact"],
      dropped: ["Client & repo identifiers", "Endpoints, hostnames, secrets", "Literal source & data"],
      example: { raw: "await stripe.charges.create({ idempotencyKey })", stripped: "await <payment-sdk>.<mutation>({ idempotencyKey })" },
    },
    // 5 · Incidents — confidentiality containment
    incidents: [
      { id: "i1", kanji: "盾", title: "Near-leak: client hostname in a shared prompt", client: "Globex", state: "contained", when: "3d", severity: "high" },
      { id: "i2", kanji: "盾", title: "Raw stack trace queued to Collective", client: "Initech", state: "resolved", when: "2w", severity: "medium" },
    ],
    // 6 · Client audit — immutable confidentiality ledger
    clientAudit: [
      { t: "10:42", kanji: "共", event: "Lesson shared upstream", detail: "idempotency pattern · anonymized", client: "Globex", ok: true },
      { t: "10:41", kanji: "盾", event: "Stripped 2 identifiers", detail: "hostname, repo slug", client: "Globex", ok: true },
      { t: "09:18", kanji: "却", event: "Blocked contribution", detail: "raw source detected · held", client: "Initech", ok: false },
      { t: "Yesterday", kanji: "共", event: "Lesson shared upstream", detail: "webhook guard · anonymized", client: "Globex", ok: true },
    ],
    // 7 · Identity & SSO — admin
    identity: {
      idp: { name: "Okta", protocol: "OIDC", status: "connected", domain: "acme.okta.com" },
      scim: true,
      mappings: [
        { source: "GitHub org · acme", to: "auto-join · role from repo access", count: 41 },
        { source: "Magic link · @acme.com", to: "developer by default", count: 5 },
        { source: "Device code", to: "read-only", count: 2 },
      ],
    },
    // 8 · Health / Monitor — admin
    health: {
      signals: [
        { kanji: "観", label: "Sessions this week", n: "312", sub: "↑ 14%", tone: "var(--accent)" },
        { kanji: "覚", label: "Adoption rate", n: "68%", sub: "of approved", tone: "var(--success)" },
        { kanji: "盾", label: "Leak-guard blocks", n: "3", sub: "all contained", tone: "var(--warning)" },
        { kanji: "門", label: "Queue age · median", n: "6h", sub: "within SLA", tone: "var(--ink)" },
      ],
      contribVsApprove: [
        { wk: "W1", c: 18, a: 12 }, { wk: "W2", c: 22, a: 15 }, { wk: "W3", c: 19, a: 17 }, { wk: "W4", c: 26, a: 20 },
      ],
      alerts: [
        { kanji: "盾", title: "Leak-guard held a raw stack trace", detail: "Initech · auto-contained · no data left", when: "2h", sev: "resolved" },
        { kanji: "門", title: "Postgres scope queue has no owner", detail: "1 candidate routed to fallback", when: "1d", sev: "warning" },
      ],
    },
  },
};
