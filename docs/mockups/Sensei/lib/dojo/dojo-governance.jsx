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

/* scope ladder — what you can author against.
   Two ladders share this list:
     · personal (free) — Personal → Project → Stack, available to individuals
     · company (paid)  — Company → Client → Team → Project, the shared team plane
   Stack is shared/free. `paid` marks scopes that need the paid team plan. */
const GV_SCOPES = [
  { id: "personal", kanji: "己", name: "My projects", kind: "Personal", sub: "personal", free: true },
  { id: "org",     kanji: "社", name: "Company", kind: "Org",     sub: "company-wide", paid: true },
  { id: "client-globex", kanji: "客", name: "Globex", kind: "Client", sub: "client engagement", parent: "org", paid: true },
  { id: "team-pay",kanji: "組", name: "Payments",  kind: "Team",    sub: "team", parent: "org", paid: true },
  { id: "team-web",kanji: "組", name: "Web",       kind: "Team",    sub: "team", parent: "org", paid: true },
  { id: "proj-site",kanji:"件", name: "personal-site",kind: "Project", sub: "personal project", parent: "personal", free: true },
  { id: "proj-auth",kanji:"件", name: "lumen-auth",kind: "Project", sub: "project", parent: "team-pay" },
  { id: "proj-bill",kanji:"件", name: "billing-svc",kind:"Project", sub: "project", parent: "team-pay" },
  { id: "stack-rust",kanji:"技",name: "Rust",      kind: "Stack",   sub: "language" },
  { id: "stack-react",kanji:"技",name:"React",     kind: "Stack",   sub: "framework" },
];
// rail grouping order + labels; which kinds sit behind the paid team plan
const GV_KIND_ORDER = ["Personal", "Org", "Client", "Team", "Project", "Stack"];
const GV_KIND_LABEL = { Personal: "Personal", Org: "Company", Client: "Client", Team: "Team", Project: "Project", Stack: "Stack" };
const GV_PAID_KINDS = { Org: true, Client: true, Team: true };
// Company vs Client is derived per-user from membership: your employer Dōjō is
// “Company”; an org you’re engaged with (but don’t work for) is a “Client”.
const GV_KIND_CAPTION = { Org: "your employer", Client: "engagement · not your employer" };

/* authored knowledge, keyed by scope. Each scope shows what's defined *here*;
   the composed onboarding bundle also pulls everything inherited from parents. */
const GV_DATA = {
  "personal": {
    stance: { autonomy: 1, sharing: 0, review: 1, anon: 2 },
    rules: [
      { k: "理", type: "principle", t: "Keep it simple — solo project, no ceremony", tone: "var(--ink-soft)" },
      { k: "守", type: "guard", t: "No secrets in source — use the vault", tone: "var(--accent)" },
    ],
    skills: [{ k: "技", t: "Write a conventional-commit message" }],
    agents: [], commands: [], memory: [],
  },
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
    <span className="mono inline-flex items-center gap-1 rounded-full text-xs whitespace-nowrap" style={{ padding: "2px var(--space-2)",
 background: high ? "var(--warning-soft)" : "var(--paper)", border: high ? "1px solid var(--warning-edge)" : "var(--hairline)",
 color: high ? "var(--warning)" : "var(--ink-soft)" }}>
      <span className="text-ink-faint" >{label}</span><span className="font-semibold" >{value}</span>
    </span>
  );
}
function PlaybookReview({ scope, mobile }) {
  const [status, setStatus] = gvS({});  // { id: 'accepted' | 'dismissed' }
  const pending = GV_PROPOSALS.filter(p => !status[p.id]);
  return (
    <div className="mt-3" >
      <GvSection kanji="覚" title="Playbook learning · proposed rules"
        count={pending.length} addLabel="New rule"
        empty="Nothing proposed — the loop is still gathering outcomes.">
        <div className="flex items-center gap-2 py-2 px-4 text-xs text-ink-mute" style={{
 borderBottom: "1px solid var(--paper-edge)", lineHeight: 1.45 }}>
          <span className="kanji text-accent text-sm" >察</span>
          Sensei attributes first-turn resolution back to each combo and proposes a re-weighting. Accept to make it a rule at
          <b className="font-semibold text-ink-soft" > {scope.name}</b>; it then cascades to everyone below.
        </div>
        {GV_PROPOSALS.map((p, i) => {
          const st = status[p.id];
          const last = i === GV_PROPOSALS.length - 1;
          return (
            <div className="grid gap-3 items-center py-3 px-4" key={p.id} style={{ gridTemplateColumns: mobile ? "1fr" : "1fr auto", borderBottom: last ? "none" : "1px solid var(--paper-edge)",
 opacity: st === "dismissed" ? 0.5 : 1 }}>
              <div className="min-w-0" >
                <div className="flex items-center gap-2 flex-wrap mb-1" >
                  <GvAxisChip label="lifecycle" value={p.lifecycle} />
                  <GvAxisChip label="intent" value={p.intent} />
                  <GvAxisChip label="risk" value={p.risk} high={p.risk === "high"} />
                  <span className="text-sm text-ink" >
                    → prefer <b className="font-semibold" >{GV_PB[p.prefer]}</b>
                    <span className="text-ink-faint" > over {GV_PB[p.over]}</span>
                  </span>
                </div>
                <div className="text-xs text-ink-mute" style={{ lineHeight: 1.45 }}>
                  {p.why}{" "}
                  <span className="mono text-success" >
                    FTR {p.ftrFrom.toFixed(2)} → {p.ftrTo.toFixed(2)}
                  </span>
                  <span className="mono text-ink-faint" > · {p.n} runs</span>
                </div>
              </div>
              <div className="flex items-center gap-2" style={{ justifySelf: mobile ? "start" : "end" }}>
                {st === "accepted" ? (
                  <span className="mono inline-flex items-center gap-1 text-xs text-success bg-success-soft rounded-full" style={{
 border: "1px solid var(--success-edge)", padding: "3px var(--space-3)" }}>
                    ✓ rule at {scope.name}
                  </span>
                ) : st === "dismissed" ? (
                  <button onClick={() => setStatus(s => ({ ...s, [p.id]: null }))} className="mono text-xs text-ink-mute border-0 cursor-pointer"
 style={{ background: "none" }}>
                    dismissed · undo
                  </button>
                ) : (
                  <>
                    <button className="text-sm text-ink-soft bg-paper border border-paper-edge rounded py-1 px-3 cursor-pointer" onClick={() => setStatus(s => ({ ...s, [p.id]: "dismissed" }))}
 style={{ fontFamily: "inherit" }}>
                      Dismiss
                    </button>
                    <button className="text-sm text-paper bg-ink border-0 rounded py-1 px-4 cursor-pointer font-medium" onClick={() => setStatus(s => ({ ...s, [p.id]: "accepted" }))}
 style={{ fontFamily: "inherit" }}>
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
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      <div className="flex items-center gap-2 py-3 px-4 border-b" >
        <span className="kanji text-base text-accent" >{kanji}</span>
        <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>{title}</span>
        <span className="mono text-xs text-ink-faint" >{count} here{inheritedCount > 0 ? ` · ${inheritedCount} inherited` : ""}</span>
        <span className="flex-1" />
        <button className="inline-flex items-center gap-1 bg-paper border border-paper-edge rounded py-1 px-3 cursor-pointer text-xs text-ink-soft" style={{ fontFamily: "inherit" }}>
          <span className="text-accent" >+</span> {addLabel}
        </button>
      </div>
      {count === 0 && inheritedCount === 0
        ? <div className="py-4 px-4 text-sm text-ink-faint italic" >{empty}</div>
        : <div>{children}</div>}
    </div>
  );
}
function GvRow({ item, last, showTag, inherited }) {
  return (
    <div className="grid gap-3 items-center py-3 px-4" style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: last ? "none" : "1px solid var(--paper-edge)", opacity: inherited ? 0.62 : 1 }}>
      <span className="kanji text-base text-center" style={{ color: inherited ? "var(--ink-faint)" : (item.tone || "var(--ink-mute)"), width: 20 }}>{item.k}</span>
      <span className="text-sm text-ink" style={{ textDecoration: item._overridden ? "line-through" : "none", textDecorationColor: "var(--ink-faint)", opacity: item._overridden ? 0.75 : 1 }}>{item.t}</span>
      {inherited
        ? <span className="mono text-xs whitespace-nowrap" style={{ color: item._overridden ? "var(--danger)" : "var(--ink-faint)" }}>{item._overridden ? "overridden here" : "↑ " + item._from}</span>
        : item._overrides
          ? <span className="mono text-xs text-accent whitespace-nowrap" >overrides ↑ {item._overrides}</span>
        : showTag && item.tag
          ? <span className="mono text-xs text-accent" >{item.tag}</span>
          : item.type
            ? <DojoChip>{item.type === "anti" ? "anti-pattern" : item.type}</DojoChip>
            : <span className="text-xs text-ink-faint" >edit</span>}
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
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      <DojoHead mobile={mobile} kanji="掟" eyebrow="Govern · define" title="Governance & shared knowledge"
        sub="Define the stance and the shared skills, agents, commands, rules — and a project's memory — at each scope. Everything cascades down the ladder, so a developer who joins inherits it on day one."
        right={<><DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">{scope.kanji} {scope.name} · {scope.kind}</DojoChip>
          <button className="inline-flex items-center gap-1 bg-paper border border-paper-edge rounded py-1 px-3 cursor-pointer text-xs text-ink-soft whitespace-nowrap" style={{ fontFamily: "inherit" }}>
            <span className="kanji text-accent" >蔵</span> Add from library
          </button></>} />

      <div style={mobile
          ? { flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }
          : { flex: 1, display: "grid", gridTemplateColumns: "minmax(150px, 230px) 1fr", minHeight: 0 }}>
        {/* scope picker — rail on desktop, wrapping pill row on mobile */}
        {mobile ? (
          <div className="shrink-0 flex flex-wrap gap-1 py-2 px-3 border-b bg-paper-soft" >
            {GV_SCOPES.map(s => {
              const on = s.id === scopeId;
              return (
                <button className="inline-flex items-center gap-1 rounded-full py-1 px-3 text-xs cursor-pointer" key={s.id} onClick={() => setScopeId(s.id)} style={{
 background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)",
 border: on ? "1px solid var(--ink)" : "var(--hairline)" }}>
                  <span className="kanji text-sm" style={{ color: on ? "var(--paper)" : "var(--accent)" }}>{s.kanji}</span>{s.name}
                </button>
              );
            })}
          </div>
        ) : (
        <aside className="border-r bg-paper-soft overflow-auto py-4 px-3" >
          <div className="flex items-start gap-1 py-0 px-2 mb-3 text-xs text-ink-mute" style={{ lineHeight: 1.4 }}>
            <span className="kanji text-accent shrink-0" >己</span>
            <span>Governance is <b className="font-semibold text-ink-soft" >free</b> on your personal ladder. The shared company ladder is on the paid team plan — sensei knows which Dōjō is <b className="font-semibold text-ink-soft" >your company</b> and which is a <b className="font-semibold text-ink-soft" >client</b> from your membership.</span>
          </div>
          {GV_KIND_ORDER.map(kind => {
            const list = GV_SCOPES.filter(s => s.kind === kind);
            if (!list.length) return null;
            const paid = GV_PAID_KINDS[kind];
            return (
            <div className="mb-3" key={kind} >
              <div className="flex items-center gap-2 py-0 px-2 mb-1" >
                <span className="text-xs uppercase text-ink-faint font-semibold" style={{ letterSpacing: ".14em" }}>{GV_KIND_LABEL[kind]}</span>
                <span className="flex-1" />
                {paid
                  ? <span className="mono text-xs text-accent" title="Paid team plan" style={{ letterSpacing: ".04em" }}>鍵 paid</span>
                  : <span className="mono text-xs text-success" title="Free for individuals" style={{ letterSpacing: ".04em" }}>free</span>}
              </div>
              {GV_KIND_CAPTION[kind] && <div className="text-xs text-ink-faint py-0 px-2 mb-1" style={{ marginTop: "-2px" }}>{GV_KIND_CAPTION[kind]}</div>}
              <div className="flex flex-col gap-1" >
                {GV_SCOPES.filter(s => s.kind === kind).map(s => {
                  const on = s.id === scopeId;
                  const n = ((GV_DATA[s.id] || GV_EMPTY).rules || []).length + ((GV_DATA[s.id] || GV_EMPTY).skills || []).length;
                  return (
                    <button className="grid items-center gap-2 w-full text-left rounded py-2 px-2 cursor-pointer text-sm" key={s.id} onClick={() => setScopeId(s.id)} style={{ gridTemplateColumns: "auto 1fr auto",
 background: on ? "var(--paper)" : "transparent", border: on ? "var(--hairline)" : "1px solid transparent",
 color: on ? "var(--ink)" : "var(--ink-soft)", fontFamily: "inherit" }}>
                      <span className="kanji text-sm text-center" style={{ width: 16, color: on ? "var(--accent)" : "var(--ink-mute)" }}>{s.kanji}</span>
                      <span className="min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{s.name}</span>
                      {n > 0 && <span className="mono text-xs text-ink-faint" >{n}</span>}
                    </button>
                  );
                })}
              </div>
            </div>
            );
          })}
        </aside>
        )}

        {/* authoring main */}
        <main className="overflow-auto" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)", flex: mobile ? 1 : undefined, minHeight: mobile ? 0 : undefined }}>
          {/* stance */}
          <div className="flex items-center gap-2 mb-3" >
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Governance stance</span>
            <span className="mono text-xs text-ink-faint" >{scope.name}</span>
            <span className="flex-1" />
            <span className="inline-flex items-center gap-1 text-xs text-ink-mute" >
              <span className="kanji text-sm text-accent" >盾</span>
              Client anonymization · always on
            </span>
          </div>
          <div className="grid gap-3 mb-6" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))" }}>
            {STANCE.map(st => {
              const val = (stanceOv[scopeId] || {})[st.id] ?? d.stance[st.id] ?? 1;
              return (
                <div className="bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" key={st.id} >
                  <div className="flex items-center gap-2 mb-1" >
                    <span className="kanji text-sm text-accent" >{st.kanji}</span>
                    <span className="text-sm text-ink font-medium" >{st.label}</span>
                  </div>
                  <div className="text-xs text-ink-mute mb-2" style={{ lineHeight: 1.4 }}>{st.help}</div>
                  <div className="flex bg-paper-mute rounded-lg p-1 gap-1" >
                    {st.stops.map((stop, i) => {
                      const on = i === val;
                      return (
                        <div className="flex-1 text-center rounded py-1 px-1 text-xs cursor-pointer" key={i} onClick={() => setStanceOv(o => ({ ...o, [scopeId]: { ...(o[scopeId] || {}), [st.id]: i } }))}
 style={{
 fontWeight: on ? 600 : 400,
 background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
 boxShadow: on ? "var(--shadow-sm)" : "none" }}>{stop}</div>
                      );
                    })}
                  </div>
                  <div className="flex items-start gap-1 mt-2 text-xs text-ink-mute" style={{ lineHeight: 1.45 }}>
                    <span className="text-accent shrink-0" >→</span>{st.conseq[val]}
                  </div>
                </div>
              );
            })}
          </div>

          {/* authored knowledge — defined here + inherited (greyed, source-tagged) */}
          <div className="flex items-center gap-2 bg-paper-soft border border-paper-edge rounded-lg py-2 px-4 mb-3" >
            <span className="kanji text-base text-accent shrink-0" >蔵</span>
            <span className="text-xs text-ink-mute flex-1" style={{ lineHeight: 1.45 }}>
              Don’t start from a blank page — pull proven principles, patterns, compliance controls and stack reviewers from the <b className="font-semibold text-ink-soft" >constitution library</b>. Prevention is cheaper than rework.
            </span>
            <button className="inline-flex items-center gap-1 bg-ink border-0 rounded py-1 px-3 cursor-pointer text-xs text-paper whitespace-nowrap shrink-0" style={{ fontFamily: "inherit" }}>
              Browse library →
            </button>
          </div>
          <div className="grid gap-3" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(320px, 1fr))" }}>
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
            <div className="mt-3" >
              <GvSection kanji="憶" title="Memory & learnings · this project" count={(d.memory || []).length} addLabel="Add learning"
                empty="Nothing learned here yet — sensei fills this as the team works.">
                {(d.memory || []).map((it, i) => <GvRow key={i} item={it} last={i === d.memory.length - 1} showTag />)}
              </GvSection>
            </div>
          )}

          {/* S5 · playbook learning — accept/reject proposed re-weightings */}
          <PlaybookReview scope={scope} mobile={mobile} />

          {/* what a new developer inherits */}
          <div className="mt-4 bg-paper-soft rounded-lg py-4 px-4" style={{ border: "1px solid var(--accent)" }}>
            <div className="flex items-start gap-3" style={{ flexWrap: mobile ? "wrap" : "nowrap" }}>
              <span className="kanji text-2xl text-accent" style={{ lineHeight: 1 }}>迎</span>
              <div className="flex-1 min-w-0" >
                <div className="text-base text-ink font-semibold" >What a new developer on {scope.name} inherits</div>
                <div className="text-sm text-ink-soft mt-1" style={{ lineHeight: 1.55 }}>
                  Composed down the ladder{inherited.length > 0 && <> — this scope plus {inherited.map((s, i) => <span key={s.id}><b className="font-semibold" >{s.name}</b>{i < inherited.length - 1 ? " · " : ""}</span>)}</>}. Delivered to their Observatory the moment they connect.
                </div>
                <div className="flex gap-2 flex-wrap mt-3" >
                  <DojoChip tone="var(--accent)" soft="var(--accent-soft)">{sum("rules")} rules</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("skills")} skills</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("agents")} agents</DojoChip>
                  <DojoChip tone="var(--ink-soft)">{sum("commands")} commands</DojoChip>
                  {isProject && <DojoChip tone="var(--success)" soft="var(--success-soft)">{sum("memory")} learnings</DojoChip>}
                </div>
              </div>
              <button className="shrink-0 self-center inline-flex items-center gap-2 bg-ink text-paper border-0 rounded-lg py-2 px-4 cursor-pointer text-sm font-medium" onClick={() => setShowOnboard(v => !v)} style={{ fontFamily: "inherit" }}>
                <span className="kanji text-sm text-accent" >観</span> {showOnboard ? "Hide preview" : "Preview onboarding"}
              </button>
            </div>
            {showOnboard && (
              <div className="mt-3 pt-3" style={{ borderTop: "1px solid var(--paper-edge)" }}>
                <div className="text-xs uppercase text-ink-mute font-semibold mb-2" style={{ letterSpacing: ".12em" }}>What lands in their Observatory on connect</div>
                {chain.map(s => {
                  const sd = GV_DATA[s.id] || GV_EMPTY;
                  const parts = [["rules", sd.rules], ["skills", sd.skills], ["agents", sd.agents], ["commands", sd.commands], ["learnings", sd.memory]]
                    .filter(([, arr]) => (arr || []).length).map(([n, arr]) => `${arr.length} ${n}`).join(" · ");
                  if (!parts) return null;
                  return (
                    <div className="flex items-center gap-2 py-1 px-0 text-sm text-ink-soft" key={s.id} >
                      <span className="kanji text-sm text-ink-mute text-center" style={{ width: 18 }}>{s.kanji}</span>
                      <span className="text-ink" style={{ minWidth: 110 }}>{s.name}</span>
                      <span className="mono text-xs text-ink-mute" >{parts}</span>
                    </div>
                  );
                })}
                <div className="text-xs text-ink-faint mt-1" >Composed by specificity — a more specific scope's rule wins on collision.</div>
              </div>
            )}
          </div>
        </main>
      </div>
    </div>
  );
}

window.DojoGovernance = DojoGovernance;
