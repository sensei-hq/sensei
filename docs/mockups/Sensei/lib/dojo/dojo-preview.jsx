// Dōjō · Effective-constitution preview for a project.
// Answers the question a developer actually asks: "before I start, what rules
// will apply HERE?" It resolves the whole scope ladder for one project and
// shows the composed constitution with conflicts resolved.
//
// The ladder, broad → specific: Company → Client → Personal → Project → Stack.
//   · A project on your EMPLOYER's own product resolves without a Client rung
//     (Company · Personal · Project · Stack).
//   · A project that is NOT your employer's — bound to another org's Dōjō —
//     resolves as a CLIENT engagement: the Client rung switches on and
//     Client + Company BOTH apply.
//   · A purely personal project (no Dōjō) resolves on the free personal ladder
//     alone (Personal · Project · Stack).
//
// Baseline (true everywhere, not a rung): everything sensei records is DERIVED
// and anonymous — no code, no source, no references — and it stays on your
// machine. There is no non-anonymous path, so a classification change alters
// which RULES apply, never what "leaves."
//
// Conflict resolution, stated plainly:
//   1. A non-negotiable (★) rule locks; no narrower scope can relax it.
//   2. Otherwise the more specific scope refines the broader one (Stack > Project
//      > Personal > Client > Company).
//
// Reuses DojoHead / DojoChip from dojo-shared.jsx. Token-only → theme-free.

const { useState: pvS } = React;

/* the three projects the preview can resolve — one per lifecycle case */
const PV_PROJECTS = [
  { id: "auth",   kanji: "件", name: "lumen-auth",    repo: "acme/lumen-auth",   kind: "company",
    dojo: "Acme Corp", team: "Payments", stack: ["React", "TypeScript"],
    why: "This repo belongs to Acme — your employer’s own product. No client rung: Company · Personal · Project · Stack." },
  { id: "globex", kanji: "件", name: "globex-portal", repo: "globex/portal",     kind: "client",
    dojo: "Globex", team: null, stack: ["React", "TypeScript"],
    why: "This repo is Globex’s, not your employer’s — it’s bound to the Globex Dōjō. So it resolves as a CLIENT engagement: the Client rung switches on, and Client + Company both apply." },
  { id: "site",   kanji: "件", name: "personal-site", repo: "rin/personal-site", kind: "personal",
    dojo: null, team: null, stack: ["React"],
    why: "No Dōjō is bound — this is yours alone. It resolves on the free personal ladder: Personal · Project · Stack. Nothing company or client applies." },
  { id: "mono",   kanji: "件", name: "agency-monorepo", repo: "studio/monorepo", kind: "client",
    dojo: "Studio", team: null, stack: ["React", "TypeScript"], clients: ["Globex", "Initech"],
    why: "One repo, work for two clients (packages/globex-∗, packages/initech-∗). It isn't your employer's, so it resolves as a CLIENT engagement — and because it's bound to two clients, BOTH client rungs apply over the shared Company base, each keeping its own derived-learning context." },
];

/* rule tone by rule-family kanji */
const PV_TONE = { 守: "var(--accent)", 法: "var(--accent)", 盾: "var(--accent)", 検: "var(--ink-soft)",
  理: "var(--ink-soft)", 紋: "var(--ink-soft)", 憶: "var(--ink-soft)", 己: "var(--ink-mute)", 技: "var(--ink-soft)" };

/* rungs, broad → specific. `rules` each: {k, t, hard?}. hard = non-negotiable.
   Rungs are included per project by a `when` predicate on project kind. */
function pvLadder(proj) {
  const company = {
    id: "company", kanji: "社", name: "Company", scope: "Acme Corp", kind: "Company",
    caption: "your employer · company-wide", tone: "var(--ink-soft)",
    rules: [
      { k: "守", t: "No secrets in source — use the vault, never .env in git", hard: true },
      { k: "守", t: "Never log tokens or PII, even at debug level", hard: true },
      { k: "法", t: "Encrypt PII at rest and in transit", hard: true, src: "compliance" },
      { k: "理", t: "Public APIs stay backward-compatible two minor versions" },
      { k: "検", t: "Test coverage ≥ 80% on money- or auth-touching paths", hard: true },
    ],
  };
  const client = {
    id: "client", kanji: "客", name: "Client", scope: proj.dojo, kind: "Client",
    caption: "engagement · not your employer", tone: "var(--accent)",
    rules: [
      { k: "盾", t: "This engagement's derived learnings stay in its own context — never merged into the company pool", hard: true },
      { k: "法", t: "Signed DPA governs any PII in the codebase", hard: true, src: "compliance" },
      { k: "検", t: "Two maintainer approvals before a lesson publishes to the Dōjō", hard: true },
    ],
  };
  const personal = {
    id: "personal", kanji: "己", name: "Personal", scope: "You", kind: "Personal",
    caption: "your own preferences", tone: "var(--ink-mute)", free: true,
    rules: [
      { k: "己", t: "Prefer run-free autonomy on scratch work" },
      { k: "己", t: "Conventional-commit messages" },
    ],
  };
  const project = {
    id: "project", kanji: "件", name: "Project", scope: proj.name, kind: "Project",
    caption: "this repo", tone: "var(--ink-soft)", free: proj.kind === "personal",
    rules: proj.kind === "personal"
      ? [{ k: "紋", t: "Static export — no server-side code" }]
      : [
          { k: "紋", t: "Rotate signing keys through the token-family table" },
          { k: "憶", t: "Device-flow codes expire at 10m here, not the default 15m" },
          { k: "検", t: "Relax coverage to ≥ 60% for this repo", relax: "Test coverage ≥ 80% on money- or auth-touching paths" },
        ],
  };
  const stack = {
    id: "stack", kanji: "技", name: "Stack", scope: proj.stack.join(" · "), kind: "Stack",
    caption: "language & framework reviewers", tone: "var(--ink-soft)", free: true,
    checkers: ["eslint", "prettier", "qlty"],
    rules: [
      { k: "技", t: "strict: true — no implicit any (eslint)", hard: true },
      { k: "技", t: "Format on save (prettier) — no style diffs in review" },
      { k: "技", t: "Cyclomatic complexity ≤ 10 per function (qlty)" },
    ],
  };
  const rungs = [];
  if (proj.kind !== "personal") rungs.push(company);
  if (proj.clients && proj.clients.length) {
    proj.clients.forEach(cn => rungs.push({ ...client, id: "client-" + cn, scope: cn,
      rules: client.rules.map(r => r.k === "盾" ? { ...r, t: cn + "'s derived learnings stay in its own context — never merged with the other client's or the company pool" } : r) }));
  } else if (proj.kind === "client") {
    rungs.push(client);
  }
  rungs.push(personal, project, stack);
  return rungs;
}

/* the resolved conflicts to narrate per project kind. Each: what two scopes
   collided, who won, and why. */
function pvConflicts(proj) {
  if (proj.kind === "personal") {
    return [
      { topic: "Autonomy", winner: "Personal", lost: null,
        detail: "No company or client stance to override you — run-free applies." },
    ];
  }
  const base = [
    { topic: "Autonomy on prod-touching steps", winner: "Company", winScope: proj.kind === "client" ? "Client + Company" : "Company",
      lost: "Personal · “run free”", detail: "Company gates irreversible steps and marks it non-negotiable, so the Personal preference can’t relax it here." },
    { topic: "Test coverage", winner: "Company", winScope: "Company",
      lost: "Project · “relax to ≥ 60%”", detail: "Coverage ≥ 80% is a non-negotiable (★) from Company — a narrower scope cannot lower a locked bar. The project override is ignored." },
  ];
  if (proj.kind === "client") {
    base.push({ topic: "Approvals to publish", winner: "Client", winScope: "Client",
      lost: "Company · “1 approval”", detail: "The Client engagement requires two approvals; the stricter, more-specific rule wins." });
  }
  if (proj.clients && proj.clients.length > 1) {
    base.push({ topic: "Cross-client isolation", winner: "Client", winScope: "each client",
      lost: "one shared pool across clients", detail: "Both client rungs apply, but each keeps its own derived-learning context — a lesson from one client's work is never carried into the other's." });
  }
  return base;
}

function PvRuleRow({ r, last }) {
  const tone = PV_TONE[r.k] || "var(--ink-soft)";
  return (
    <div style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
      padding: "var(--space-2) 0", borderBottom: last ? "none" : "1px solid var(--paper-edge)" }}>
      <span className="kanji" style={{ fontSize: "var(--text-base)", color: tone, width: 20, textAlign: "center" }}>{r.k}</span>
      <span style={{ fontSize: "var(--text-sm)", color: r.relax ? "var(--ink-faint)" : "var(--ink)", textDecoration: r.relax ? "line-through" : "none", lineHeight: 1.4 }}>
        {r.t}
      </span>
      <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)", justifySelf: "end", flexShrink: 0 }}>
        {r.src === "compliance" && <DojoChip tone="var(--accent)" soft="var(--accent-soft)" border="1px solid var(--accent-edge)">法</DojoChip>}
        {r.relax
          ? <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--warning)" }}>overridden ↑</span>
          : r.hard
            ? <span style={{ display: "inline-flex", alignItems: "center", gap: "3px", fontSize: "var(--text-xs)", color: "var(--warning)" }}>★ non-negotiable</span>
            : <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>negotiable</span>}
      </span>
    </div>
  );
}

function DojoRulePreview({ mobile = false, initial = "globex", onOpenLibrary, onExit }) {
  const [pid, setPid] = pvS(initial);
  const [override, setOverride] = pvS({});
  const [libFor, setLibFor] = pvS(null);      // which rung opened the library ("personal" | "project")
  const baseProj = PV_PROJECTS.find(p => p.id === pid) || PV_PROJECTS[0];
  const effKind = override[baseProj.id] || baseProj.kind;
  const isOverridden = effKind !== baseProj.kind;
  const proj = isOverridden ? { ...baseProj, kind: effKind, clients: effKind === "client" ? baseProj.clients : undefined } : baseProj;
  const rungs = pvLadder(proj);
  const conflicts = pvConflicts(proj);
  const total = rungs.reduce((n, r) => n + r.rules.filter(x => !x.relax).length, 0);
  const locked = rungs.reduce((n, r) => n + r.rules.filter(x => x.hard).length, 0);

  const chooseKind = (k) => setOverride(o => ({ ...o, [baseProj.id]: k }));
  const openLibrary = (rungId, scope) => { setLibFor(rungId); if (onOpenLibrary) onOpenLibrary(rungId, scope); };

  return (
    <div className="sensei" data-screen-label="Dōjō · effective-constitution preview" style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      {onExit && (
        <div style={{ flexShrink: 0, display: "flex", alignItems: "center", gap: "var(--space-2)", padding: "var(--space-2) var(--space-4)", borderBottom: "var(--hairline)", background: "var(--paper-soft)" }}>
          <button onClick={onExit} className="zs-btn zs-btn-sm zs-btn-ghost border-1px" title="Back to your work">
            <span className="text-ink-mute">←</span><span className="kanji text-accent">携</span>Your work
          </button>
          <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>you · project rules</span>
        </div>
      )}
      <DojoHead mobile={mobile} kanji="序" eyebrow="Project · effective constitution" title="What governs this project"
        sub="The composed constitution for a project — every rule that resolves onto it, down the ladder, with conflicts already settled. See exactly what governs your work before the first commit, and manage your own personal guidelines and guardrails, which layer in below."
        right={<React.Fragment>
          <DojoChip tone="var(--ink-mute)" soft="var(--paper-soft)" border="var(--hairline)">場 {proj.repo}</DojoChip>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">{total} rules · {locked} locked</DojoChip>
        </React.Fragment>} />

      {/* project picker */}
      <div style={{ flexShrink: 0, display: "flex", gap: "var(--space-2)", flexWrap: "wrap", padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-5)", borderBottom: "var(--hairline)", background: "var(--paper-soft)" }}>
        {PV_PROJECTS.map(p => {
          const on = p.id === pid;
          const kindTone = p.kind === "client" ? "var(--accent)" : p.kind === "personal" ? "var(--ink-mute)" : "var(--ink-soft)";
          return (
            <button key={p.id} onClick={() => setPid(p.id)} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)", cursor: "pointer", fontFamily: "inherit",
              background: on ? "var(--paper)" : "transparent", border: on ? "1px solid var(--accent)" : "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-3)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: kindTone }}>{p.kanji}</span>
              <span className="mono" style={{ fontSize: "var(--text-sm)", color: on ? "var(--ink)" : "var(--ink-soft)" }}>{p.name}</span>
              <DojoChip tone={kindTone} soft={p.kind === "client" ? "var(--accent-soft)" : "var(--paper-mute)"}>
                {p.kind === "company" ? "社 company" : p.kind === "client" ? "客 client" : "己 personal"}
              </DojoChip>
            </button>
          );
        })}
      </div>

      {/* why this classification + override */}
      <div style={{ flexShrink: 0, display: "flex", flexDirection: "column", gap: "var(--space-2)", padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-5)",
        background: proj.kind === "client" ? "var(--accent-soft)" : "var(--paper)", borderBottom: "var(--hairline)" }}>
        <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-base)", color: proj.kind === "client" ? "var(--accent)" : "var(--ink-mute)", flexShrink: 0 }}>問</span>
          <span style={{ fontSize: "var(--text-xs)", color: proj.kind === "client" ? "var(--ink-soft)" : "var(--ink-mute)", lineHeight: 1.5 }}>
            <b style={{ fontWeight: 600, color: "var(--ink)" }} className="mono">{proj.repo}</b> — {proj.why}
          </span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap", paddingLeft: "calc(var(--text-base) + var(--space-2))" }}>
          <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>Classified</span>
          <DojoChip tone={effKind === "client" ? "var(--accent)" : effKind === "personal" ? "var(--ink-mute)" : "var(--ink-soft)"} soft={effKind === "client" ? "var(--accent-soft)" : "var(--paper-mute)"}>
            {effKind === "company" ? "社 company" : effKind === "client" ? "客 client" : "己 personal"}
          </DojoChip>
          {isOverridden && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--warning)" }}>overridden</span>}
          {baseProj.dojo ? (
            <React.Fragment>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>· not right?</span>
              {["client", "company"].map(k => (
                <button key={k} onClick={() => chooseKind(k)}
                  style={{ cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-xs)", borderRadius: "var(--radius)", padding: "2px var(--space-2)",
                    border: effKind === k ? "1px solid var(--accent)" : "var(--hairline)", background: effKind === k ? "var(--paper)" : "transparent", color: effKind === k ? "var(--ink)" : "var(--ink-soft)" }}>
                  {k === "client" ? "Client" : "Company"}
                </button>
              ))}
              {isOverridden && <button onClick={() => setOverride(o => { const n = { ...o }; delete n[baseProj.id]; return n; })}
                style={{ cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-xs)", border: "none", background: "none", color: "var(--ink-faint)", textDecoration: "underline" }}>reset</button>}
            </React.Fragment>
          ) : (
            <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>· bind a Dōjō to govern this as company or client work</span>
          )}
        </div>
      </div>

      <div style={mobile ? { flex: 1, display: "flex", flexDirection: "column", minHeight: 0, overflow: "auto" } : { flex: 1, display: "grid", gridTemplateColumns: "minmax(0,0.92fr) minmax(0,1.08fr)", minHeight: 0 }}>
        {/* left · the ladder */}
        <div style={{ borderRight: mobile ? "none" : "var(--hairline)", borderBottom: mobile ? "var(--hairline)" : "none", overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>The ladder</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{rungs.length} scopes</span>
          </div>
          <p style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5, margin: "0 0 var(--space-3)" }}>Numbered <b style={{ fontWeight: 600, color: "var(--ink)" }}>broadest (1) → most specific</b>. Each rung <b style={{ fontWeight: 600, color: "var(--ink)" }}>refines the one above</b>; a ★ non-negotiable locks so no narrower rung can relax it. On a client engagement <b style={{ fontWeight: 600, color: "var(--accent)" }}>Company and Client both apply</b> — the client rung sits on top of your company base.</p>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
            {rungs.map((rg, i) => (
              <div key={rg.id} style={{ position: "relative", background: "var(--paper-soft)", border: "var(--hairline)", borderLeft: "3px solid " + rg.tone,
                borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginLeft: 0 }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap", marginBottom: "var(--space-2)" }}>
                  <span className="mono" style={{ flexShrink: 0, width: 20, height: 20, borderRadius: "var(--radius-full)", background: "var(--paper-mute)", color: "var(--ink-mute)", fontSize: "var(--text-xs)", display: "inline-flex", alignItems: "center", justifyContent: "center" }}>{i + 1}</span>
                  <span className="kanji" style={{ fontSize: "var(--text-lg)", color: rg.tone }}>{rg.kanji}</span>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>{rg.name}</span>
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)" }}>{rg.scope}</span>
                  {rg.free && <DojoChip tone="var(--success)" soft="var(--success-soft)">free</DojoChip>}
                  {rg.id === "personal" && <DojoChip tone="var(--ink-mute)" soft="var(--paper-mute)">yours · editable</DojoChip>}
                  <span style={{ flex: 1 }} />
                  {(rg.id === "personal" || rg.id === "project") && (
                    <button onClick={() => openLibrary(rg.id, rg.scope)} title="Add rules from the library" style={{ cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-xs)", color: libFor === rg.id ? "var(--ink)" : "var(--ink-soft)",
                      background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "2px var(--space-2)", display: "inline-flex", alignItems: "center", gap: "3px", whiteSpace: "nowrap" }}>
                      <span style={{ color: "var(--accent)" }}>＋</span> library
                    </button>
                  )}
                  <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{rg.caption}</span>
                </div>
                {libFor === rg.id && (
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-soft)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-3)" }}>
                    <span className="kanji" style={{ color: "var(--accent)", flexShrink: 0 }}>庫</span>
                    <span style={{ lineHeight: 1.5 }}>Library open — pick packs to add to <b className="mono" style={{ fontWeight: 600 }}>{rg.scope}</b>.</span>
                    <span style={{ flex: 1 }} />
                    <button onClick={() => setLibFor(null)} style={{ cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-xs)", border: "none", background: "none", color: "var(--ink-faint)", textDecoration: "underline" }}>close</button>
                  </div>
                )}
                {rg.checkers && (
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--space-1)", flexWrap: "wrap", marginBottom: "var(--space-2)" }}>
                    <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>caught by</span>
                    {rg.checkers.map(c => (
                      <span key={c} className="mono" style={{ display: "inline-flex", alignItems: "center", gap: "3px", fontSize: "var(--text-xs)", color: "var(--ink-soft)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-full)", padding: "1px var(--space-2)" }}>
                        <span style={{ width: 5, height: 5, borderRadius: "50%", background: "var(--success)" }} />{c}
                      </span>
                    ))}
                    <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>→ surfaced in your review lane</span>
                  </div>
                )}
                <div style={{ background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "0 var(--space-3)" }}>
                  {rg.rules.map((r, j) => <PvRuleRow key={j} r={r} last={j === rg.rules.length - 1} />)}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* right · resolved constitution + conflicts */}
        <div style={{ overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)", background: "var(--paper)" }}>
          {/* conflict resolution */}
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--warning)" }}>衝</span>
            <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>Conflicts, resolved</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{conflicts.length}</span>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)", marginBottom: "var(--space-5)" }}>
            {conflicts.map((c, i) => (
              <div key={i} style={{ background: "var(--paper-soft)", border: "var(--hairline)", borderLeft: "3px solid var(--warning)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap", marginBottom: "var(--space-1)" }}>
                  <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>{c.topic}</span>
                  <span style={{ flex: 1 }} />
                  <DojoChip tone="var(--success)" soft="var(--success-soft)" border="1px solid var(--success-edge)">{c.winScope || c.winner} wins</DojoChip>
                </div>
                {c.lost && <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginBottom: "var(--space-1)" }}>over <span style={{ textDecoration: "line-through" }}>{c.lost}</span></div>}
                <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>{c.detail}</div>
              </div>
            ))}
          </div>

          {/* the resolution rule */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-1)", background: "var(--paper-soft)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)", marginBottom: "var(--space-5)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-1)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>序</span>
              <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>How it resolves</span>
            </div>
            {[
              "Everything sensei records is derived and anonymous, and stays on your machine — so classification changes which rules apply, never what leaves.",
              "A non-negotiable (★) locks — no narrower scope can relax it.",
              "Otherwise the more specific scope refines the broader (Stack → Project → Personal → Client → Company).",
            ].map((t, i) => (
              <div key={i} style={{ display: "flex", gap: "var(--space-2)", fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.5 }}>
                <span className="mono" style={{ color: "var(--ink-faint)", flexShrink: 0 }}>{i + 1}</span>{t}
              </div>
            ))}
          </div>

          {/* what a developer starts with */}
          <div style={{ background: "var(--ink)", borderRadius: "var(--radius-lg)", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
            <div style={{ display: "flex", alignItems: "baseline", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
              <span className="display" style={{ fontSize: "var(--text-3xl)", fontWeight: 300, color: "var(--paper)", lineHeight: 1 }}>{total}</span>
              <span style={{ fontSize: "var(--text-sm)", color: "var(--on-primary-soft, rgba(255,255,255,0.72))" }}>rules govern this project on day one</span>
            </div>
            <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", marginBottom: "var(--space-3)" }}>
              <span style={{ display: "inline-flex", alignItems: "center", gap: "3px", fontSize: "var(--text-xs)", color: "var(--warning)", background: "rgba(255,255,255,0.06)", borderRadius: "var(--radius-full)", padding: "2px var(--space-2)" }}>★ {locked} non-negotiable</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)", background: "rgba(255,255,255,0.06)", borderRadius: "var(--radius-full)", padding: "2px var(--space-2)" }}>盾 derived · anonymous · stays on your machine</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--on-primary-mute, rgba(255,255,255,0.5))", background: "rgba(255,255,255,0.06)", borderRadius: "var(--radius-full)", padding: "2px var(--space-2)" }}>{rungs.length} scopes composed</span>
            </div>
            <div style={{ fontSize: "var(--text-xs)", color: "var(--on-primary-soft, rgba(255,255,255,0.72))", lineHeight: 1.55 }}>
              Inherited automatically — nothing to copy in. Sensei enforces the locked rules, the reviewers catch style and complexity mechanically, and the rest guide as you go. <span style={{ fontStyle: "italic" }}>Still listening.</span>
            </div>
            <div style={{ marginTop: "var(--space-3)", paddingTop: "var(--space-3)", borderTop: "1px solid rgba(255,255,255,0.1)", fontSize: "var(--text-xs)", color: "var(--on-primary-mute, rgba(255,255,255,0.5))", lineHeight: 1.5 }}>
              This is the same view a teammate sees when they join a Dōjō — <span style={{ fontStyle: "italic" }}>here's what you'll follow.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { DojoRulePreview, PV_PROJECTS });
