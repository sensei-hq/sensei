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
      { k: "理", type: "principle", t: "Public APIs stay backward-compatible two minor versions", tone: "var(--ink-soft)" },
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
    stops: ["Gate often", "Balanced", "Run free"],
    conseq: ["Every prod-touching or irreversible step pauses for approval.", "Routine steps run free; risky or irreversible ones gate to a human.", "Only explicitly-flagged commands gate — everything else runs."] },
  { id: "sharing", kanji: "共", label: "Sharing", help: "what leaves this scope, upward",
    stops: ["Manual", "Opt-in", "Automatic"],
    conseq: ["Nothing leaves unless someone shares it by hand.", "Lessons queue in ready-to-share; you confirm each batch.", "Lessons past the confidence bar are sent up on a cadence."] },
  { id: "review", kanji: "検", label: "Review", help: "approvals required before a lesson publishes",
    stops: ["1 approval", "2 approvals", "Consensus"],
    conseq: ["One named maintainer decision publishes a lesson.", "High-impact items wait for a second maintainer.", "All scope owners must sign off before anything publishes."] },
];

/* S5 · playbook learning review — the loop attributes each recorded chunk's
   outcome (did the first turn land?) back to a lifecycle·intent·risk combo and
   proposes a re-weighting. A maintainer accepts (→ becomes a governance rule at
   this scope) or dismisses. This is the governance half of the front-door loop;
   a developer's own per-combo FTR lives in the Sensei app's Intake · History. */
const GV_PB = { vibe: "Vibe / spike", mockup_first: "Mockup-first", spec_driven: "Spec-driven",
  gsd: "Get stuff done", change_flow: "Change-flow", debug_flow: "Debug-flow" };
const GV_PROPOSALS = [
  { id: "p1", lifecycle: "stable", intent: "enhancement", risk: "low", prefer: "change_flow", over: "gsd",
    ftrFrom: 0.71, ftrTo: 0.90, n: 14, why: "Impact-first runs landed cleaner than lean-plan on stable enhancements." },
  { id: "p2", lifecycle: "greenfield", intent: "explore", risk: "low", prefer: "vibe", over: "mockup_first",
    ftrFrom: 0.60, ftrTo: 0.82, n: 9, why: "Discardable spikes beat premature mockups when the objective is fuzzy." },
  { id: "p3", lifecycle: "stable", intent: "bug", risk: "high", prefer: "spec_driven", over: "debug_flow",
    ftrFrom: 0.55, ftrTo: 0.78, n: 6, why: "High-blast bugs did better with a design pass before the fix." },
];
function GvAxisChip({ label, value, high }) {
  return (
    <span className="mono" style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)",
      borderRadius: "var(--radius-full)", padding: "2px var(--space-2)", fontSize: "var(--text-xs)", whiteSpace: "nowrap",
      background: high ? "var(--warning-soft)" : "var(--paper)", border: high ? "1px solid var(--warning-edge)" : "var(--hairline)",
      color: high ? "var(--warning)" : "var(--ink-soft)" }}>
      <span style={{ color: "var(--ink-faint)" }}>{label}</span><span style={{ fontWeight: 600 }}>{value}</span>
    </span>
  );
}
function PlaybookReview({ scope, mobile }) {
  const [status, setStatus] = gvS({});  // { id: 'accepted' | 'dismissed' }
  const pending = GV_PROPOSALS.filter(p => !status[p.id]);
  return (
    <div style={{ marginTop: "var(--space-3)" }}>
      <GvSection kanji="覚" title="Playbook learning · proposed rules"
        count={pending.length} addLabel="New rule"
        empty="Nothing proposed — the loop is still gathering outcomes.">
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) var(--space-4)",
          borderBottom: "1px solid var(--paper-edge)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.45 }}>
          <span className="kanji" style={{ color: "var(--accent)", fontSize: "var(--text-sm)" }}>察</span>
          Sensei attributes first-turn resolution back to each combo and proposes a re-weighting. Accept to make it a rule at
          <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}> {scope.name}</b>; it then cascades to everyone below.
        </div>
        {GV_PROPOSALS.map((p, i) => {
          const st = status[p.id];
          const last = i === GV_PROPOSALS.length - 1;
          return (
            <div key={p.id} style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr auto", gap: "var(--space-3)",
              alignItems: "center", padding: "var(--space-3) var(--space-4)", borderBottom: last ? "none" : "1px solid var(--paper-edge)",
              opacity: st === "dismissed" ? 0.5 : 1 }}>
              <div style={{ minWidth: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap", marginBottom: "var(--space-1)" }}>
                  <GvAxisChip label="lifecycle" value={p.lifecycle} />
                  <GvAxisChip label="intent" value={p.intent} />
                  <GvAxisChip label="risk" value={p.risk} high={p.risk === "high"} />
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)" }}>
                    → prefer <b style={{ fontWeight: 600 }}>{GV_PB[p.prefer]}</b>
                    <span style={{ color: "var(--ink-faint)" }}> over {GV_PB[p.over]}</span>
                  </span>
                </div>
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.45 }}>
                  {p.why}{" "}
                  <span className="mono" style={{ color: "var(--success)" }}>
                    FTR {p.ftrFrom.toFixed(2)} → {p.ftrTo.toFixed(2)}
                  </span>
                  <span className="mono" style={{ color: "var(--ink-faint)" }}> · {p.n} runs</span>
                </div>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", justifySelf: mobile ? "start" : "end" }}>
                {st === "accepted" ? (
                  <span className="mono" style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)",
                    fontSize: "var(--text-xs)", color: "var(--success)", background: "var(--success-soft)",
                    border: "1px solid var(--success-edge)", borderRadius: "var(--radius-full)", padding: "3px var(--space-3)" }}>
                    ✓ rule at {scope.name}
                  </span>
                ) : st === "dismissed" ? (
                  <button onClick={() => setStatus(s => ({ ...s, [p.id]: null }))} className="mono"
                    style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none", cursor: "pointer" }}>
                    dismissed · undo
                  </button>
                ) : (
                  <>
                    <button onClick={() => setStatus(s => ({ ...s, [p.id]: "dismissed" }))}
                      style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", background: "var(--paper)", border: "var(--hairline)",
                        borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-3)", cursor: "pointer", fontFamily: "inherit" }}>
                      Dismiss
                    </button>
                    <button onClick={() => setStatus(s => ({ ...s, [p.id]: "accepted" }))}
                      style={{ fontSize: "var(--text-sm)", color: "var(--paper)", background: "var(--ink)", border: "none",
                        borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-4)", cursor: "pointer", fontFamily: "inherit", fontWeight: 500 }}>
                      Accept
                    </button>
                  </>
                )}
              </div>
            </div>
          );
        })}
      </GvSection>
    </div>
  );
}

function GvSection({ kanji, title, count, addLabel, children, empty, inheritedCount = 0 }) {
  return (
    <div style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>{kanji}</span>
        <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{title}</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{count} here{inheritedCount > 0 ? ` · ${inheritedCount} inherited` : ""}</span>
        <span style={{ flex: 1 }} />
        <button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", background: "var(--paper)", border: "var(--hairline)",
          borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-3)", cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>
          <span style={{ color: "var(--accent)" }}>+</span> {addLabel}
        </button>
      </div>
      {count === 0 && inheritedCount === 0
        ? <div style={{ padding: "var(--space-4) var(--space-4)", fontSize: "var(--text-sm)", color: "var(--ink-faint)", fontStyle: "italic" }}>{empty}</div>
        : <div>{children}</div>}
    </div>
  );
}
function GvRow({ item, last, showTag, inherited }) {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
                  padding: "var(--space-3) var(--space-4)", borderBottom: last ? "none" : "1px solid var(--paper-edge)", opacity: inherited ? 0.62 : 1 }}>
      <span className="kanji" style={{ fontSize: "var(--text-base)", color: inherited ? "var(--ink-faint)" : (item.tone || "var(--ink-mute)"), width: 20, textAlign: "center" }}>{item.k}</span>
      <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", textDecoration: item._overridden ? "line-through" : "none", textDecorationColor: "var(--ink-faint)", opacity: item._overridden ? 0.75 : 1 }}>{item.t}</span>
      {inherited
        ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: item._overridden ? "var(--danger)" : "var(--ink-faint)", whiteSpace: "nowrap" }}>{item._overridden ? "overridden here" : "↑ " + item._from}</span>
        : item._overrides
          ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", whiteSpace: "nowrap" }}>overrides ↑ {item._overrides}</span>
        : showTag && item.tag
          ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>{item.tag}</span>
          : item.type
            ? <DojoChip>{item.type === "anti" ? "anti-pattern" : item.type}</DojoChip>
            : <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>edit</span>}
    </div>
  );
}

function DojoGovernance({ mobile = false }) {
  const [scopeId, setScopeId] = gvS("proj-auth");
  const [stanceOv, setStanceOv] = gvS({});   // { scopeId: { autonomy: n, … } }
  const [showOnboard, setShowOnboard] = gvS(false);
  const scope = GV_SCOPES.find(s => s.id === scopeId) || GV_SCOPES[0];
  const d = GV_DATA[scopeId] || GV_EMPTY;
  const isProject = scope.kind === "Project";

  // inheritance chain (self → parents) for the "inherits" composition
  const chain = [];
  let cur = scope;
  while (cur) { chain.push(cur); cur = cur.parent ? GV_SCOPES.find(s => s.id === cur.parent) : null; }
  // A project also inherits from any Stack scope (React/Rust/…), not just its parent ladder.
  if (scope.kind === "Project" || scope.kind === "Team") {
    GV_SCOPES.filter(s => s.kind === "Stack").forEach(s => { if (!chain.some(c => c.id === s.id)) chain.push(s); });
  }
  const inherited = chain.slice(1);
  const sum = (key) => {
    const seen = new Set();
    chain.forEach(s => ((GV_DATA[s.id] || GV_EMPTY)[key] || []).forEach(it => seen.add((it.t || it.name || JSON.stringify(it)).toLowerCase())));
    return seen.size;
  };

  const grouped = [
    { kanji: "掟", title: "Rules · guards · principles", key: "rules", add: "Add rule", empty: "No rules defined at this scope yet." },
    { kanji: "技", title: "Shared skills", key: "skills", add: "Add skill", empty: "No skills shared here yet." },
    { kanji: "問", title: "Agents & personas", key: "agents", add: "Add agent", empty: "No agents defined here yet." },
    { kanji: "令", title: "Commands", key: "commands", add: "Add command", empty: "No commands defined here yet." },
  ];

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="掟" eyebrow="Govern · define" title="Governance & shared knowledge"
        sub="Define the stance and the shared skills, agents, commands, rules — and a project's memory — at each scope. Everything cascades down the ladder, so a developer who joins inherits it on day one."
        right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">{scope.kanji} {scope.name} · {scope.kind}</DojoChip>} />

      <div style={mobile
          ? { flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }
          : { flex: 1, display: "grid", gridTemplateColumns: "minmax(150px, 230px) 1fr", minHeight: 0 }}>
        {/* scope picker — rail on desktop, wrapping pill row on mobile */}
        {mobile ? (
          <div style={{ flexShrink: 0, display: "flex", flexWrap: "wrap", gap: "var(--space-1)", padding: "var(--space-2) var(--space-3)", borderBottom: "var(--hairline)", background: "var(--paper-soft)" }}>
            {GV_SCOPES.map(s => {
              const on = s.id === scopeId;
              return (
                <button key={s.id} onClick={() => setScopeId(s.id)} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)",
                  borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)", cursor: "pointer",
                  background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)",
                  border: on ? "1px solid var(--ink)" : "var(--hairline)" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: on ? "var(--paper)" : "var(--accent)" }}>{s.kanji}</span>{s.name}
                </button>
              );
            })}
          </div>
        ) : (
        <aside style={{ borderRight: "var(--hairline)", background: "var(--paper-soft)", overflow: "auto", padding: "var(--space-4) var(--space-3)" }}>
          {["Org", "Team", "Project", "Stack"].map(kind => (
            <div key={kind} style={{ marginBottom: "var(--space-3)" }}>
              <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, padding: "0 var(--space-2)", marginBottom: "var(--space-1)" }}>{kind}</div>
              <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
                {GV_SCOPES.filter(s => s.kind === kind).map(s => {
                  const on = s.id === scopeId;
                  const n = ((GV_DATA[s.id] || GV_EMPTY).rules || []).length + ((GV_DATA[s.id] || GV_EMPTY).skills || []).length;
                  return (
                    <button key={s.id} onClick={() => setScopeId(s.id)} style={{
                      display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center", gap: "var(--space-2)", width: "100%", textAlign: "left",
                      borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-2)", cursor: "pointer", fontSize: "var(--text-sm)",
                      background: on ? "var(--paper)" : "transparent", border: on ? "var(--hairline)" : "1px solid transparent",
                      color: on ? "var(--ink)" : "var(--ink-soft)", fontFamily: "inherit",
                    }}>
                      <span className="kanji" style={{ fontSize: "var(--text-sm)", width: 16, textAlign: "center", color: on ? "var(--accent)" : "var(--ink-mute)" }}>{s.kanji}</span>
                      <span style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{s.name}</span>
                      {n > 0 && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{n}</span>}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </aside>
        )}

        {/* authoring main */}
        <main style={{ overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", flex: mobile ? 1 : undefined, minHeight: mobile ? 0 : undefined }}>
          {/* stance */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Governance stance</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{scope.name}</span>
            <span style={{ flex: 1 }} />
            <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>盾</span>
              Client anonymization · always on
            </span>
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "var(--space-3)", marginBottom: "var(--space-5)" }}>
            {STANCE.map(st => {
              const val = (stanceOv[scopeId] || {})[st.id] ?? d.stance[st.id] ?? 1;
              return (
                <div key={st.id} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{st.kanji}</span>
                    <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 500 }}>{st.label}</span>
                  </div>
                  <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.4, marginBottom: "var(--space-2)" }}>{st.help}</div>
                  <div style={{ display: "flex", background: "var(--paper-mute)", borderRadius: "var(--radius-lg)", padding: "var(--space-1)", gap: "var(--space-1)" }}>
                    {st.stops.map((stop, i) => {
                      const on = i === val;
                      return (
                        <div key={i} onClick={() => setStanceOv(o => ({ ...o, [scopeId]: { ...(o[scopeId] || {}), [st.id]: i } }))}
                          style={{ flex: 1, textAlign: "center", borderRadius: "var(--radius)", padding: "var(--space-1) var(--space-1)", fontSize: "var(--text-xs)",
                          fontWeight: on ? 600 : 400, cursor: "pointer",
                          background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
                          boxShadow: on ? "var(--shadow-sm)" : "none" }}>{stop}</div>
                      );
                    })}
                  </div>
                  <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-1)", marginTop: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.45 }}>
                    <span style={{ color: "var(--accent)", flexShrink: 0 }}>→</span>{st.conseq[val]}
                  </div>
                </div>
              );
            })}
          </div>

          {/* authored knowledge — defined here + inherited (greyed, source-tagged) */}
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))", gap: "var(--space-3)" }}>
            {grouped.map(g => {
              const items = d[g.key] || [];
              const inheritedTitles = {};
              inherited.forEach(s => ((GV_DATA[s.id] || GV_EMPTY)[g.key] || []).forEach(it => {
                const key = (it.t || "").toLowerCase();
                if (!(key in inheritedTitles)) inheritedTitles[key] = s.name;
              }));
              const localTitles = new Set(items.map(it => (it.t || "").toLowerCase()));
              // Mark local rules that shadow an inherited one, rather than hiding the collision.
              const localItems = items.map(it => {
                const key = (it.t || "").toLowerCase();
                return inheritedTitles[key] ? { ...it, _overrides: inheritedTitles[key] } : it;
              });
              const inheritedItems = [];
              inherited.forEach(s => ((GV_DATA[s.id] || GV_EMPTY)[g.key] || []).forEach(it => {
                const key = (it.t || "").toLowerCase();
                if (!inheritedItems.some(x => (x.t || "").toLowerCase() === key))
                  inheritedItems.push({ ...it, _from: s.name, _overridden: localTitles.has(key) });
              }));
              return (
                <GvSection key={g.key} kanji={g.kanji} title={g.title} count={items.length} addLabel={g.add} empty={g.empty}
                  inheritedCount={inheritedItems.length}>
                  {localItems.map((it, i) => <GvRow key={"l" + i} item={it} last={i === localItems.length - 1 && inheritedItems.length === 0} />)}
                  {inheritedItems.map((it, i) => <GvRow key={"i" + i} item={it} last={i === inheritedItems.length - 1} inherited />)}
                </GvSection>
              );
            })}
          </div>

          {/* memory — project scope only */}
          {isProject && (
            <div style={{ marginTop: "var(--space-3)" }}>
              <GvSection kanji="憶" title="Memory & learnings · this project" count={(d.memory || []).length} addLabel="Add learning"
                empty="Nothing learned here yet — sensei fills this as the team works.">
                {(d.memory || []).map((it, i) => <GvRow key={i} item={it} last={i === d.memory.length - 1} showTag />)}
              </GvSection>
            </div>
          )}

          {/* S5 · playbook learning — accept/reject proposed re-weightings */}
          <PlaybookReview scope={scope} mobile={mobile} />

          {/* what a new developer inherits */}
          <div style={{ marginTop: "var(--space-4)", background: "var(--paper-soft)", border: "1px solid var(--accent)", borderRadius: "var(--radius-lg)", padding: "var(--space-4) var(--space-4)" }}>
            <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)", flexWrap: mobile ? "wrap" : "nowrap" }}>
              <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: "var(--accent)", lineHeight: 1 }}>迎</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontSize: "var(--text-base)", color: "var(--ink)", fontWeight: 600 }}>What a new developer on {scope.name} inherits</div>
                <div style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, marginTop: "var(--space-1)" }}>
                  Composed down the ladder{inherited.length > 0 && <> — this scope plus {inherited.map((s, i) => <span key={s.id}><b style={{ fontWeight: 600 }}>{s.name}</b>{i < inherited.length - 1 ? " · " : ""}</span>)}</>}. Delivered to their Observatory the moment they connect.
                </div>
                <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", marginTop: "var(--space-3)" }}>
                  <DojoChip tone="var(--accent)" soft="var(--accent-soft)">{sum("rules")} rules</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("skills")} skills</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("agents")} agents</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("commands")} commands</DojoChip>
                  {isProject && <DojoChip tone="var(--success)" soft="var(--success-soft)">{sum("memory")} learnings</DojoChip>}
                </div>
              </div>
              <button onClick={() => setShowOnboard(v => !v)} style={{ flexShrink: 0, alignSelf: "center", display: "inline-flex", alignItems: "center", gap: "var(--space-2)", background: "var(--ink)", color: "var(--paper)",
                border: "none", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-4)", cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-sm)", fontWeight: 500 }}>
                <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>観</span> {showOnboard ? "Hide preview" : "Preview onboarding"}
              </button>
            </div>
            {showOnboard && (
              <div style={{ marginTop: "var(--space-3)", borderTop: "1px solid var(--paper-edge)", paddingTop: "var(--space-3)" }}>
                <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".12em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600, marginBottom: "var(--space-2)" }}>What lands in their Observatory on connect</div>
                {chain.map(s => {
                  const sd = GV_DATA[s.id] || GV_EMPTY;
                  const parts = [["rules", sd.rules], ["skills", sd.skills], ["agents", sd.agents], ["commands", sd.commands], ["learnings", sd.memory]]
                    .filter(([, arr]) => (arr || []).length).map(([n, arr]) => `${arr.length} ${n}`).join(" · ");
                  if (!parts) return null;
                  return (
                    <div key={s.id} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-1) 0", fontSize: "var(--text-sm)", color: "var(--ink-soft)" }}>
                      <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)", width: 18, textAlign: "center" }}>{s.kanji}</span>
                      <span style={{ color: "var(--ink)", minWidth: 110 }}>{s.name}</span>
                      <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>{parts}</span>
                    </div>
                  );
                })}
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "var(--space-1)" }}>Composed by specificity — a more specific scope's rule wins on collision.</div>
              </div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}

window.DojoGovernance = DojoGovernance;
