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
    <div className="grid gap-3 items-center py-2 px-0" style={{ gridTemplateColumns: "auto 1fr auto", borderBottom: last ? "none" : "1px solid var(--paper-edge)" }}>
      <span className="kanji text-base text-center" style={{ color: tone, width: 20 }}>{r.k}</span>
      <span className="text-sm" style={{ color: r.relax ? "var(--ink-faint)" : "var(--ink)", textDecoration: r.relax ? "line-through" : "none", lineHeight: 1.4 }}>
        {r.t}
      </span>
      <span className="inline-flex items-center gap-2 shrink-0" style={{ justifySelf: "end" }}>
        {r.src === "compliance" && <DojoChip tone="var(--accent)" soft="var(--accent-soft)" border="1px solid var(--accent-edge)">法</DojoChip>}
        {r.relax
          ? <span className="mono text-xs text-warning" >overridden ↑</span>
          : r.hard
            ? <span className="inline-flex items-center text-xs text-warning" style={{ gap: "3px" }}>★ non-negotiable</span>
            : <span className="mono text-xs text-ink-faint" >negotiable</span>}
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
    <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label="Dōjō · effective-constitution preview" >
      {onExit && (
        <div className="shrink-0 flex items-center gap-2 py-2 px-4 border-b bg-paper-soft" >
          <button onClick={onExit} className="zs-btn zs-btn-sm zs-btn-ghost border border-paper-edge" title="Back to your work">
            <span className="text-ink-mute">←</span><span className="kanji text-accent">携</span>Your work
          </button>
          <span className="mono text-xs text-ink-faint" >you · project rules</span>
        </div>
      )}
      <DojoHead mobile={mobile} kanji="序" eyebrow="Project · effective constitution" title="What governs this project"
        sub="The composed constitution for a project — every rule that resolves onto it, down the ladder, with conflicts already settled. See exactly what governs your work before the first commit, and manage your own personal guidelines and guardrails, which layer in below."
        right={<React.Fragment>
          <DojoChip tone="var(--ink-mute)" soft="var(--paper-soft)" border="var(--hairline)">場 {proj.repo}</DojoChip>
          <DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">{total} rules · {locked} locked</DojoChip>
        </React.Fragment>} />

      {/* project picker */}
      <div className="shrink-0 flex gap-2 flex-wrap border-b bg-paper-soft" style={{ padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-6)" }}>
        {PV_PROJECTS.map(p => {
          const on = p.id === pid;
          const kindTone = p.kind === "client" ? "var(--accent)" : p.kind === "personal" ? "var(--ink-mute)" : "var(--ink-soft)";
          return (
            <button className="inline-flex items-center gap-2 cursor-pointer rounded-lg py-2 px-3" key={p.id} onClick={() => setPid(p.id)} style={{ fontFamily: "inherit",
 background: on ? "var(--paper)" : "transparent", border: on ? "1px solid var(--accent)" : "var(--hairline)" }}>
              <span className="kanji text-base" style={{ color: kindTone }}>{p.kanji}</span>
              <span className="mono text-sm" style={{ color: on ? "var(--ink)" : "var(--ink-soft)" }}>{p.name}</span>
              <DojoChip tone={kindTone} soft={p.kind === "client" ? "var(--accent-soft)" : "var(--paper-mute)"}>
                {p.kind === "company" ? "社 company" : p.kind === "client" ? "客 client" : "己 personal"}
              </DojoChip>
            </button>
          );
        })}
      </div>

      {/* why this classification + override */}
      <div className="shrink-0 flex flex-col gap-2 border-b" style={{ padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-6)",
 background: proj.kind === "client" ? "var(--accent-soft)" : "var(--paper)" }}>
        <div className="flex items-start gap-2" >
          <span className="kanji text-base shrink-0" style={{ color: proj.kind === "client" ? "var(--accent)" : "var(--ink-mute)" }}>問</span>
          <span className="text-xs" style={{ color: proj.kind === "client" ? "var(--ink-soft)" : "var(--ink-mute)", lineHeight: 1.5 }}>
            <b className="mono font-semibold text-ink">{proj.repo}</b> — {proj.why}
          </span>
        </div>
        <div className="flex items-center gap-2 flex-wrap" style={{ paddingLeft: "calc(var(--text-base) + var(--space-2))" }}>
          <span className="text-xs text-ink-mute" >Classified</span>
          <DojoChip tone={effKind === "client" ? "var(--accent)" : effKind === "personal" ? "var(--ink-mute)" : "var(--ink-soft)"} soft={effKind === "client" ? "var(--accent-soft)" : "var(--paper-mute)"}>
            {effKind === "company" ? "社 company" : effKind === "client" ? "客 client" : "己 personal"}
          </DojoChip>
          {isOverridden && <span className="mono text-xs text-warning" >overridden</span>}
          {baseProj.dojo ? (
            <React.Fragment>
              <span className="text-xs text-ink-faint" >· not right?</span>
              {["client", "company"].map(k => (
                <button className="cursor-pointer text-xs rounded" key={k} onClick={() => chooseKind(k)}
 style={{ fontFamily: "inherit", padding: "2px var(--space-2)",
 border: effKind === k ? "1px solid var(--accent)" : "var(--hairline)", background: effKind === k ? "var(--paper)" : "transparent", color: effKind === k ? "var(--ink)" : "var(--ink-soft)" }}>
                  {k === "client" ? "Client" : "Company"}
                </button>
              ))}
              {isOverridden && <button className="cursor-pointer text-xs border-0 text-ink-faint underline" onClick={() => setOverride(o => { const n = { ...o }; delete n[baseProj.id]; return n; })}
 style={{ fontFamily: "inherit", background: "none" }}>reset</button>}
            </React.Fragment>
          ) : (
            <span className="text-xs text-ink-faint" >· bind a Dōjō to govern this as company or client work</span>
          )}
        </div>
      </div>

      <div style={mobile ? { flex: 1, display: "flex", flexDirection: "column", minHeight: 0, overflow: "auto" } : { flex: 1, display: "grid", gridTemplateColumns: "minmax(0,0.92fr) minmax(0,1.08fr)", minHeight: 0 }}>
        {/* left · the ladder */}
        <div className="overflow-auto" style={{ borderRight: mobile ? "none" : "var(--hairline)", borderBottom: mobile ? "var(--hairline)" : "none", padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
          <div className="flex items-center gap-2 mb-1" >
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>The ladder</span>
            <span className="mono text-xs text-ink-faint" >{rungs.length} scopes</span>
          </div>
          <p className="text-xs text-ink-mute" style={{ lineHeight: 1.5, margin: "0 0 var(--space-3)" }}>Numbered <b className="font-semibold text-ink" >broadest (1) → most specific</b>. Each rung <b className="font-semibold text-ink" >refines the one above</b>; a ★ non-negotiable locks so no narrower rung can relax it. On a client engagement <b className="font-semibold text-accent" >Company and Client both apply</b> — the client rung sits on top of your company base.</p>
          <div className="flex flex-col gap-2" >
            {rungs.map((rg, i) => (
              <div className="relative bg-paper-soft border border-paper-edge rounded-lg py-3 px-4 ml-0" key={rg.id} style={{ borderLeft: "3px solid " + rg.tone }}>
                <div className="flex items-center gap-2 flex-wrap mb-2" >
                  <span className="mono shrink-0 rounded-full bg-paper-mute text-ink-mute text-xs inline-flex items-center justify-center" style={{ width: 20, height: 20 }}>{i + 1}</span>
                  <span className="kanji text-lg" style={{ color: rg.tone }}>{rg.kanji}</span>
                  <span className="text-sm text-ink font-semibold" >{rg.name}</span>
                  <span className="mono text-xs text-ink-soft" >{rg.scope}</span>
                  {rg.free && <DojoChip tone="var(--success)" soft="var(--success-soft)">free</DojoChip>}
                  {rg.id === "personal" && <DojoChip tone="var(--ink-mute)" soft="var(--paper-mute)">yours · editable</DojoChip>}
                  <span className="flex-1" />
                  {(rg.id === "personal" || rg.id === "project") && (
                    <button className="cursor-pointer text-xs bg-paper border border-paper-edge rounded inline-flex items-center whitespace-nowrap" onClick={() => openLibrary(rg.id, rg.scope)} title="Add rules from the library" style={{ fontFamily: "inherit", color: libFor === rg.id ? "var(--ink)" : "var(--ink-soft)", padding: "2px var(--space-2)", gap: "3px" }}>
                      <span className="text-accent" >＋</span> library
                    </button>
                  )}
                  <span className="mono text-xs text-ink-faint" >{rg.caption}</span>
                </div>
                {libFor === rg.id && (
                  <div className="flex items-center gap-2 mb-2 text-xs text-ink-soft bg-accent-soft rounded py-2 px-3" style={{ border: "1px solid var(--accent-edge)" }}>
                    <span className="kanji text-accent shrink-0" >庫</span>
                    <span style={{ lineHeight: 1.5 }}>Library open — pick packs to add to <b className="mono font-semibold" >{rg.scope}</b>.</span>
                    <span className="flex-1" />
                    <button className="cursor-pointer text-xs border-0 text-ink-faint underline" onClick={() => setLibFor(null)} style={{ fontFamily: "inherit", background: "none" }}>close</button>
                  </div>
                )}
                {rg.checkers && (
                  <div className="flex items-center gap-1 flex-wrap mb-2" >
                    <span className="text-xs text-ink-mute" >caught by</span>
                    {rg.checkers.map(c => (
                      <span key={c} className="mono inline-flex items-center text-xs text-ink-soft bg-paper border border-paper-edge rounded-full" style={{ gap: "3px", padding: "1px var(--space-2)" }}>
                        <span className="rounded-full bg-success" style={{ width: 5, height: 5 }} />{c}
                      </span>
                    ))}
                    <span className="text-xs text-ink-faint" >→ surfaced in your review lane</span>
                  </div>
                )}
                <div className="bg-paper border border-paper-edge rounded py-0 px-3" >
                  {rg.rules.map((r, j) => <PvRuleRow key={j} r={r} last={j === rg.rules.length - 1} />)}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* right · resolved constitution + conflicts */}
        <div className="overflow-auto bg-paper" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
          {/* conflict resolution */}
          <div className="flex items-center gap-2 mb-3" >
            <span className="kanji text-sm text-warning" >衝</span>
            <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>Conflicts, resolved</span>
            <span className="mono text-xs text-ink-faint" >{conflicts.length}</span>
          </div>
          <div className="flex flex-col gap-2 mb-6" >
            {conflicts.map((c, i) => (
              <div className="bg-paper-soft border border-paper-edge rounded-lg py-3 px-4" key={i} style={{ borderLeft: "3px solid var(--warning)" }}>
                <div className="flex items-center gap-2 flex-wrap mb-1" >
                  <span className="text-sm text-ink font-semibold" >{c.topic}</span>
                  <span className="flex-1" />
                  <DojoChip tone="var(--success)" soft="var(--success-soft)" border="1px solid var(--success-edge)">{c.winScope || c.winner} wins</DojoChip>
                </div>
                {c.lost && <div className="text-xs text-ink-faint mb-1" >over <span style={{ textDecoration: "line-through" }}>{c.lost}</span></div>}
                <div className="text-xs text-ink-mute" style={{ lineHeight: 1.5 }}>{c.detail}</div>
              </div>
            ))}
          </div>

          {/* the resolution rule */}
          <div className="flex flex-col gap-1 bg-paper-soft border border-paper-edge rounded-lg p-4 mb-6" >
            <div className="flex items-center gap-2 mb-1" >
              <span className="kanji text-sm text-accent" >序</span>
              <span className="text-xs uppercase text-ink-mute font-semibold" style={{ letterSpacing: ".14em" }}>How it resolves</span>
            </div>
            {[
              "Everything sensei records is derived and anonymous, and stays on your machine — so classification changes which rules apply, never what leaves.",
              "A non-negotiable (★) locks — no narrower scope can relax it.",
              "Otherwise the more specific scope refines the broader (Stack → Project → Personal → Client → Company).",
            ].map((t, i) => (
              <div className="flex gap-2 text-xs text-ink-soft" key={i} style={{ lineHeight: 1.5 }}>
                <span className="mono text-ink-faint shrink-0" >{i + 1}</span>{t}
              </div>
            ))}
          </div>

          {/* what a developer starts with */}
          <div className="bg-ink rounded-lg" style={{ padding: mobile ? "var(--space-4)" : "var(--space-6)" }}>
            <div className="flex items-baseline gap-2 mb-2" >
              <span className="display text-3xl font-light text-paper" style={{ lineHeight: 1 }}>{total}</span>
              <span className="text-sm" style={{ color: "var(--on-primary-soft, rgba(255,255,255,0.72))" }}>rules govern this project on day one</span>
            </div>
            <div className="flex gap-2 flex-wrap mb-3" >
              <span className="inline-flex items-center text-xs text-warning rounded-full" style={{ gap: "3px", background: "rgba(255,255,255,0.06)", padding: "2px var(--space-2)" }}>★ {locked} non-negotiable</span>
              <span className="mono text-xs text-accent rounded-full" style={{ background: "rgba(255,255,255,0.06)", padding: "2px var(--space-2)" }}>盾 derived · anonymous · stays on your machine</span>
              <span className="mono text-xs rounded-full" style={{ color: "var(--on-primary-mute, rgba(255,255,255,0.5))", background: "rgba(255,255,255,0.06)", padding: "2px var(--space-2)" }}>{rungs.length} scopes composed</span>
            </div>
            <div className="text-xs" style={{ color: "var(--on-primary-soft, rgba(255,255,255,0.72))", lineHeight: 1.55 }}>
              Inherited automatically — nothing to copy in. Sensei enforces the locked rules, the reviewers catch style and complexity mechanically, and the rest guide as you go. <span className="italic" >Still listening.</span>
            </div>
            <div className="mt-3 pt-3 text-xs" style={{ borderTop: "1px solid rgba(255,255,255,0.1)", color: "var(--on-primary-mute, rgba(255,255,255,0.5))", lineHeight: 1.5 }}>
              This is the same view a teammate sees when they join a Dōjō — <span className="italic" >here's what you'll follow.</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { DojoRulePreview, PV_PROJECTS });
