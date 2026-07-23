// Mock data for the Dōjō console (the company-hosted governance site).
// Shapes mirror the model in the UX audit §3: memberships, the contribute→
// triage→approve→distribute lifecycle, git-derived roles, and the
// source-dereferencing confidentiality model.

window.DOJO = {
  // The orgs this user belongs to — the multi-membership model. The console
  // is scoped to one at a time via the org switcher.
  memberships: [
    { id: "acme",   kanji: "社", name: "Acme Corp",        kind: "employer",  current: true, role: "admin" },
    { id: "globex", kanji: "客", name: "Globex",           kind: "client", role: "maintainer" },
    { id: "initech",kanji: "客", name: "Initech",          kind: "client", role: "lead" },
    { id: "rustco", kanji: "群", name: "Rust Guild",       kind: "community", role: "readonly" },
    { id: "self",   kanji: "己", name: "Personal",         kind: "personal", role: "owner" },
  ],

  org: {
    name: "Acme Corp", kanji: "社", domain: "dojo.acme.internal",
    members: 48, scopes: 12, repos: 134,
  },

  metrics: {
    pendingTriage: 7,
    contribWeek: 23,
    contribSpark: [9, 12, 8, 14, 11, 17, 23],
    approvedWeek: 18,
    adoptionLift: 0.14,           // +14pp FTR across adopting scopes
    ftrSpark: [0.61, 0.63, 0.62, 0.66, 0.69, 0.71, 0.74],
    dereferenced: 142,            // client lessons auto-dereferenced this week
    incidents: 0,
  },

  // Triage queue — candidates awaiting a maintainer decision. Grouped by scope
  // in the UI; each carries origin (drives attribution), confidence, conflicts.
  queue: [
    { id: "c1", kanji: "守", type: "guard", scope: "Company", scopeKanji: "社",
      title: "Never log refresh tokens, even at debug level",
      origin: "employer", by: "Keiko T.", confidence: 0.94, conflicts: 0, dups: 1,
      age: "2h", evidence: 6, impact: "high",
      cause: "Three sessions leaked tokens into debug logs in lumen-auth; caught in review, not by a guard.",
      context: "Applies to any logger call inside an auth boundary. Generalises across services.",
      learning: "Refresh tokens must never reach a log sink. Add a redaction guard at the logger, not the call site." },
    { id: "c2", kanji: "紋", type: "pattern", scope: "Team · Payments", scopeKanji: "組",
      title: "Idempotency key on every money-moving mutation",
      origin: "employer", by: "Marco D.", confidence: 0.88, conflicts: 1, dups: 0,
      age: "5h", evidence: 9, impact: "high",
      cause: "A retried charge double-billed once; the pattern that prevents it was tribal knowledge.",
      context: "Payments service and anything calling the ledger. Conflicts with an older 'retry freely' note.",
      learning: "Every mutation that moves money carries a client-supplied idempotency key, checked before commit." },
    { id: "c3", kanji: "盾", type: "guard", scope: "Client · Globex", scopeKanji: "客",
      title: "Validate webhook signatures before parsing the body",
      origin: "client", by: "dereferenced", confidence: 0.91, conflicts: 0, dups: 0,
      age: "1d", evidence: 4, impact: "med",
      cause: "An unsigned webhook was parsed and acted on during an engagement; the source has been dropped.",
      context: "Any inbound webhook handler. The lesson stands without the client's repo or identifiers.",
      learning: "Verify the signature header against the shared secret before the payload is ever deserialized.",
      dereferenced: ["client name", "repo path", "endpoint URLs"] },
    { id: "c4", kanji: "問", type: "prompt", scope: "Stack · React", scopeKanji: "技",
      title: "Persona: an integration-test author for auth flows",
      origin: "employer", by: "Aiko N.", confidence: 0.79, conflicts: 0, dups: 2,
      age: "1d", evidence: 5, impact: "med",
      cause: "Auth changes repeatedly shipped without integration tests; a persona gets the assistant there faster.",
      context: "Front-end auth work on React. Two near-duplicate personas already exist — candidate to merge.",
      learning: "A reusable persona that drafts integration tests for refresh + device-code flows before edits land." },
    { id: "c5", kanji: "理", type: "principle", scope: "Company", scopeKanji: "社",
      title: "Public APIs stay backward-compatible for two minor versions",
      origin: "employer", by: "Sven K.", confidence: 0.83, conflicts: 0, dups: 0,
      age: "2d", evidence: 7, impact: "high",
      cause: "A breaking change to a public endpoint broke three internal consumers in one release.",
      context: "Every public-facing API surface across the org.",
      learning: "A removed or changed public field is deprecated for two minor versions before it disappears." },
    { id: "c6", kanji: "技", type: "skill", scope: "Stack · Postgres", scopeKanji: "技",
      title: "Skill: explain a slow query plan and suggest an index",
      origin: "community", by: "Rust Guild", confidence: 0.72, conflicts: 0, dups: 0,
      age: "3d", evidence: 3, impact: "low",
      cause: "Recurring slow-query triage that follows the same steps each time.",
      context: "Any Postgres-backed service. Pulled from a community contribution.",
      learning: "A skill that reads EXPLAIN ANALYZE output, names the bottleneck, and proposes a candidate index." },
  ],

  // Members — roles derived from git, fine-tuned in Dōjō.
  members: [
    { name: "Keiko Tanaka",  git: "Org owner",            dojo: "Org admin",   scopes: "all",            active: "now" },
    { name: "Marco Diaz",    git: "Repo admin / Maintain",dojo: "Maintainer",  scopes: "Payments · Ledger", active: "12m" },
    { name: "Aiko Nakamura", git: "Write",                dojo: "Contributor", scopes: "Web · Auth",     active: "1h" },
    { name: "Sven Karlsson", git: "Repo admin / Maintain",dojo: "Maintainer",  scopes: "Platform · API", active: "3h" },
    { name: "Priya Rao",     git: "Write",                dojo: "Contributor", scopes: "Data",           active: "1d" },
    { name: "Tom Becker",    git: "Read",                 dojo: "Read-only",   scopes: "—",              active: "5d" },
  ],

  activity: [
    { kanji: "決", text: "Keiko approved “Never log refresh tokens” → Company", when: "8m", tone: "success" },
    { kanji: "共", text: "Aiko shared a persona from web-auth", when: "1h", tone: "ink" },
    { kanji: "盾", text: "1 client lesson auto-dereferenced from Globex", when: "2h", tone: "accent" },
    { kanji: "配", text: "“Idempotency key” distributed to Payments (6 repos)", when: "4h", tone: "ink" },
    { kanji: "改", text: "Sven revised “API compatibility” before publishing", when: "1d", tone: "ink" },
  ],
};
