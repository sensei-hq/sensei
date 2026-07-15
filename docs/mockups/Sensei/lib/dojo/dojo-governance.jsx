// Dōjō · Governance — where the shared mind is *authored*, not just observed.
//
// An admin defines the governance model + shared knowledge at a scope
// (Org · Team · Project · Stack): the stance (autonomy, sharing, review,
// anonymization), the rules/guards/principles, shared skills, agents/personas,
// commands, and — for a project — its memory/learnings. Everything cascades
// down the scope ladder, so a developer who joins a project under the Dōjō
// inherits the composed set on day one.
//
// Reuses DojoHead / DojoChip from dojo-shared.jsx. Token-only → theme-free.

const { useState: gvS } = React;

/* scope ladder — what you can author against */
const GV_SCOPES = [
  { id: "org",     kanji: "社", name: "Company", kind: "Org",     sub: "company-wide" },
  { id: "team-pay",kanji: "組", name: "Payments",  kind: "Team",    sub: "team", parent: "org" },
  { id: "team-web",kanji: "組", name: "Web",       kind: "Team",    sub: "team", parent: "org" },
  { id: "proj-auth",kanji:"件", name: "lumen-auth",kind: "Project", sub: "project", parent: "team-pay" },
  { id: "proj-bill",kanji:"件", name: "billing-svc",kind:"Project", sub: "project", parent: "team-pay" },
  { id: "stack-rust",kanji:"技",name: "Rust",      kind: "Stack",   sub: "language" },
  { id: "stack-react",kanji:"技",name:"React",     kind: "Stack",   sub: "framework" },
];

/* authored knowledge, keyed by scope. Each scope shows what's defined *here*;
   the composed onboarding bundle also pulls everything inherited from parents. */
const GV_DATA = {
  org: {
    stance: { autonomy: 1, sharing: 2, review: 1, anon: 2 },
    rules: [
      { k: "守", type: "guard", t: "Never log refresh tokens, even at debug level", tone: "var(--accent)" },
      { k: "理", type: "principle", t: "Public APIs stay backward-compatible two minor versions", tone: "var(--ink-2)" },
      { k: "禁", type: "anti", t: "No secrets in source — use the vault, never .env in git", tone: "var(--warning)" },
    ],
    skills: [{ k: "技", t: "Write a conventional-commit message" }, { k: "技", t: "Draft an ADR from a decision" }],
    agents: [{ k: "問", t: "Reviewer — checks PRs against org guards" }],
    commands: [{ k: "令", t: "sensei ship — lint · test · changelog · tag" }],
    memory: [],
  },
  "team-pay": {
    stance: { autonomy: 0, sharing: 2, review: 2, anon: 2 },
    rules: [
      { k: "紋", type: "pattern", t: "Idempotency key on every money-moving mutation", tone: "var(--accent)" },
      { k: "禁", type: "anti", t: "Never retry a charge without an idempotency key", tone: "var(--warning)" },
    ],
    skills: [{ k: "技", t: "Reconcile a ledger discrepancy" }],
    agents: [{ k: "問", t: "Test author — integration tests for payment flows" }],
    commands: [{ k: "令", t: "sensei replay — re-run a webhook against staging" }],
    memory: [],
  },
  "proj-auth": {
    stance: { autonomy: 0, sharing: 1, review: 2, anon: 1 },
    rules: [
      { k: "紋", type: "pattern", t: "Rotate signing keys through the token-family table", tone: "var(--accent)" },
    ],
    skills: [{ k: "技", t: "Add an OAuth provider end-to-end" }],
    agents: [],
    commands: [{ k: "令", t: "sensei auth:migrate — run + verify a token migration" }],
    memory: [
      { k: "憶", t: "Refresh-token rotation was corrected 3× — the store must be transactional", tag: "3rd time" },
      { k: "憶", t: "Clock-skew tolerance of 30s settled the flaky session tests", tag: "settled" },
      { k: "憶", t: "Device-flow codes expire at 10m here, not the default 15m", tag: "gotcha" },
    ],
  },
  "stack-rust": {
    stance: { autonomy: 1, sharing: 2, review: 1, anon: 2 },
    rules: [{ k: "紋", type: "pattern", t: "Wrap third-party SDKs behind a trait for testability", tone: "var(--accent)" }],
    skills: [{ k: "技", t: "Explain a borrow-checker error" }],
    agents: [], commands: [], memory: [],
  },
};
const GV_EMPTY = { stance: { autonomy: 1, sharing: 1, review: 1, anon: 2 }, rules: [], skills: [], agents: [], commands: [], memory: [] };

/* stance dials — each posture is a 3-stop segmented choice */
const STANCE = [
  { id: "autonomy", kanji: "自", label: "Agent autonomy", help: "how far sensei goes before it gates to a human",
    stops: ["Gate often", "Balanced", "Run free"] },
  { id: "sharing", kanji: "共", label: "Sharing", help: "what leaves this scope, upward",
    stops: ["Manual", "Opt-in", "Automatic"] },
  { id: "review", kanji: "検", label: "Review", help: "approvals required before a lesson publishes",
    stops: ["1 approval", "2 approvals", "Consensus"] },
];

function GvSection({ kanji, title, count, addLabel, children, empty }) {
  return (
    <div style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, padding: "12px 16px", borderBottom: "var(--hairline)" }}>
        <span className="kanji" style={{ fontSize: 15, color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>{title}</span>
        <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{count}</span>
        <span style={{ flex: 1 }} />
        <button style={{ display: "inline-flex", alignItems: "center", gap: 6, background: "var(--paper)", border: "var(--hairline)",
          borderRadius: 7, padding: "5px 11px", cursor: "pointer", fontFamily: "inherit", fontSize: 12, color: "var(--ink-2)" }}>
          <span style={{ color: "var(--accent)" }}>+</span> {addLabel}
        </button>
      </div>
      {count === 0
        ? <div style={{ padding: "18px 16px", fontSize: 12.5, color: "var(--ink-4)", fontStyle: "italic" }}>{empty}</div>
        : <div>{children}</div>}
    </div>
  );
}
function GvRow({ item, last, showTag }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: 12, alignItems: "center",
                  padding: "11px 16px", borderBottom: last ? "none" : "1px solid var(--edge)" }}>
      <span className="kanji" style={{ fontSize: 16, color: item.tone || "var(--ink-3)", width: 20, textAlign: "center" }}>{item.k}</span>
      <span style={{ fontSize: 13.5, color: "var(--ink)" }}>{item.t}</span>
      {showTag && item.tag
        ? <span className="mono" style={{ fontSize: 10.5, color: "var(--accent)" }}>{item.tag}</span>
        : item.type
          ? <DojoChip>{item.type === "anti" ? "anti-pattern" : item.type}</DojoChip>
          : <span style={{ fontSize: 12, color: "var(--ink-4)" }}>edit</span>}
    </div>
  );
}

function DojoGovernance() {
  const [scopeId, setScopeId] = gvS("proj-auth");
  const scope = GV_SCOPES.find(s => s.id === scopeId) || GV_SCOPES[0];
  const d = GV_DATA[scopeId] || GV_EMPTY;
  const isProject = scope.kind === "Project";

  // inheritance chain (self → parents) for the "inherits" composition
  const chain = [];
  let cur = scope;
  while (cur) { chain.push(cur); cur = cur.parent ? GV_SCOPES.find(s => s.id === cur.parent) : null; }
  const inherited = chain.slice(1);
  const sum = (key) => chain.reduce((n, s) => n + ((GV_DATA[s.id] || GV_EMPTY)[key] || []).length, 0);

  const grouped = [
    { kanji: "掟", title: "Rules · guards · principles", key: "rules", add: "Add rule", empty: "No rules defined at this scope yet." },
    { kanji: "技", title: "Shared skills", key: "skills", add: "Add skill", empty: "No skills shared here yet." },
    { kanji: "問", title: "Agents & personas", key: "agents", add: "Add agent", empty: "No agents defined here yet." },
    { kanji: "令", title: "Commands", key: "commands", add: "Add command", empty: "No commands defined here yet." },
  ];

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead kanji="掟" eyebrow="Govern · define" title="Governance & shared knowledge"
        sub="Define the stance and the shared skills, agents, commands, rules — and a project's memory — at each scope. Everything cascades down the ladder, so a developer who joins inherits it on day one."
        right={<DojoChip tone="var(--ink-2)" soft="var(--paper-2)" border="var(--hairline)">{scope.kanji} {scope.name} · {scope.kind}</DojoChip>} />

      <div style={{ flex: 1, display: "grid", gridTemplateColumns: "230px 1fr", minHeight: 0 }}>
        {/* scope picker */}
        <aside style={{ borderRight: "var(--hairline)", background: "var(--paper-2)", overflow: "auto", padding: "16px 12px" }}>
          {["Org", "Team", "Project", "Stack"].map(kind => (
            <div key={kind} style={{ marginBottom: 14 }}>
              <div style={{ fontSize: 9.5, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-4)", fontWeight: 600, padding: "0 8px", marginBottom: 6 }}>{kind}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {GV_SCOPES.filter(s => s.kind === kind).map(s => {
                  const on = s.id === scopeId;
                  const n = ((GV_DATA[s.id] || GV_EMPTY).rules || []).length + ((GV_DATA[s.id] || GV_EMPTY).skills || []).length;
                  return (
                    <button key={s.id} onClick={() => setScopeId(s.id)} style={{
                      display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center", gap: 9, width: "100%", textAlign: "left",
                      borderRadius: 7, padding: "8px 9px", cursor: "pointer", fontSize: 13,
                      background: on ? "var(--paper)" : "transparent", border: on ? "var(--hairline)" : "1px solid transparent",
                      color: on ? "var(--ink)" : "var(--ink-2)", fontFamily: "inherit",
                    }}>
                      <span className="kanji" style={{ fontSize: 14, width: 16, textAlign: "center", color: on ? "var(--accent)" : "var(--ink-3)" }}>{s.kanji}</span>
                      <span style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.name}</span>
                      {n > 0 && <span className="mono" style={{ fontSize: 10, color: "var(--ink-4)" }}>{n}</span>}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </aside>

        {/* authoring main */}
        <main style={{ overflow: "auto", padding: 24 }}>
          {/* stance */}
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
            <span style={{ fontSize: 11, letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 600 }}>Governance stance</span>
            <span className="mono" style={{ fontSize: 11, color: "var(--ink-4)" }}>{scope.name}</span>
            <span style={{ flex: 1 }} />
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 11, color: "var(--ink-3)" }}>
              <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>盾</span>
              Client anonymization · always on
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 12, marginBottom: 24 }}>
            {STANCE.map(st => {
              const val = d.stance[st.id] ?? 1;
              return (
                <div key={st.id} style={{ background: "var(--paper-2)", border: "var(--hairline)", borderRadius: 12, padding: "13px 15px" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 3 }}>
                    <span className="kanji" style={{ fontSize: 14, color: "var(--accent)" }}>{st.kanji}</span>
                    <span style={{ fontSize: 13, color: "var(--ink)", fontWeight: 500 }}>{st.label}</span>
                  </div>
                  <div style={{ fontSize: 11.5, color: "var(--ink-3)", lineHeight: 1.4, marginBottom: 10 }}>{st.help}</div>
                  <div style={{ display: "flex", background: "var(--paper-3)", borderRadius: 8, padding: 3, gap: 2 }}>
                    {st.stops.map((stop, i) => {
                      const on = i === val;
                      return (
                        <div key={i} style={{ flex: 1, textAlign: "center", borderRadius: 6, padding: "6px 4px", fontSize: 11.5,
                          fontWeight: on ? 600 : 400, cursor: "pointer",
                          background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-3)",
                          boxShadow: on ? "0 1px 2px rgba(0,0,0,0.08)" : "none" }}>{stop}</div>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>

          {/* authored knowledge */}
          <div style={{ display: "grid", gridTemplateColumns: isProject ? "1fr 1fr" : "1fr 1fr", gap: 14 }}>
            {grouped.map(g => {
              const items = d[g.key] || [];
              return (
                <GvSection key={g.key} kanji={g.kanji} title={g.title} count={items.length} addLabel={g.add} empty={g.empty}>
                  {items.map((it, i) => <GvRow key={i} item={it} last={i === items.length - 1} />)}
                </GvSection>
              );
            })}
          </div>

          {/* memory — project scope only */}
          {isProject && (
            <div style={{ marginTop: 14 }}>
              <GvSection kanji="憶" title="Memory & learnings · this project" count={(d.memory || []).length} addLabel="Add learning"
                empty="Nothing learned here yet — sensei fills this as the team works.">
                {(d.memory || []).map((it, i) => <GvRow key={i} item={it} last={i === d.memory.length - 1} showTag />)}
              </GvSection>
            </div>
          )}

          {/* what a new developer inherits */}
          <div style={{ marginTop: 20, background: "var(--paper-2)", border: "1px solid var(--accent)", borderRadius: 12, padding: "16px 18px" }}>
            <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
              <span className="kanji" style={{ fontSize: 26, color: "var(--accent)", lineHeight: 1 }}>迎</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: 14.5, color: "var(--ink)", fontWeight: 600 }}>What a new developer on {scope.name} inherits</div>
                <div style={{ fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.55, marginTop: 3 }}>
                  Composed down the ladder{inherited.length > 0 && <> — this scope plus {inherited.map((s, i) => <span key={s.id}><b style={{ fontWeight: 600 }}>{s.name}</b>{i < inherited.length - 1 ? " · " : ""}</span>)}</>}. Delivered to their Observatory the moment they connect.
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 11 }}>
                  <DojoChip tone="var(--accent)" soft="var(--accent-soft)">{sum("rules")} rules</DojoChip>
                  <DojoChip tone="var(--ink-2)">{sum("skills")} skills</DojoChip>
                  <DojoChip tone="var(--ink-2)">{sum("agents")} agents</DojoChip>
                  <DojoChip tone="var(--ink-2)">{sum("commands")} commands</DojoChip>
                  {isProject && <DojoChip tone="var(--success)" soft="var(--success-soft)">{sum("memory")} learnings</DojoChip>}
                </div>
              </div>
              <button style={{ flexShrink: 0, alignSelf: "center", display: "inline-flex", alignItems: "center", gap: 7, background: "var(--ink)", color: "var(--paper)",
                border: "none", borderRadius: 8, padding: "10px 16px", cursor: "pointer", fontFamily: "inherit", fontSize: 13, fontWeight: 500 }}>
                <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>観</span> Preview onboarding
              </button>
            </div>
          </div>
        </main>
      </div>
    </div>
  );
}

window.DojoGovernance = DojoGovernance;
