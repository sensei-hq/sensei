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
  runs: [
    { id: "s-2891", project: "lumen-auth",   assistant: "claude-sonnet", state: "running",
      task: "refactor refresh-token rotation", elapsed: "38m", edits: 12, gate: true,
      corrections: 3, kanji: "観" },
    { id: "s-2890", project: "agency-monorepo", assistant: "claude-opus", state: "waiting",
      task: "wire initech billing webhook", elapsed: "12m", edits: 4, gate: true,
      corrections: 0, kanji: "観" },
    { id: "s-2887", project: "ledger-core",   assistant: "claude-sonnet", state: "running",
      task: "add idempotency keys to ledger writes", elapsed: "1h 4m", edits: 27, gate: false,
      corrections: 1, kanji: "観" },
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
      count: 8, adopted: true, note: "token redaction, signature checks, secret scanning" },
    { id: "p2", kanji: "紋", name: "Payments patterns", by: "Acme · payments",
      count: 6, adopted: true, note: "idempotency, ledger writes, reconciliation" },
    { id: "p3", kanji: "技", name: "React · TypeScript baseline", by: "Rust Guild",
      count: 11, adopted: false, note: "exports, query layer, suspense boundaries" },
    { id: "p4", kanji: "盾", name: "Client engagement shield", by: "Globex · lead",
      count: 5, adopted: true, note: "dereferencing, webhook verification, audit trail" },
    { id: "p5", kanji: "理", name: "API compatibility", by: "Acme · platform",
      count: 4, adopted: false, note: "deprecation windows, versioning, changelog gates" },
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
      context: "4 sessions · dereferenced · confidence 0.91", age: "1h" },
    { id: "d2", project: "lumen-auth", kanji: "決",
      title: "promote ‘idempotency key’ from Project to Company",
      options: ["promote to Company", "keep at Project", "decline"],
      context: "adopted in 6 repos · no conflicts", age: "5h" },
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
