// Dōjō · Constitution library — the predefined rule/template set a Dōjō starts
// from. Prevention over cure: pull proven principles, patterns, compliance
// controls, stack reviewers and design guardrails INTO the constitution rather
// than authoring every rule by hand (and re-encountering the same mistakes).
//
// Grouped by area (core principles · architecture & patterns · security ·
// compliance · language & stack · design system). Each pack lists individual
// rules the user cherry-picks; any rule can be marked NON-NEGOTIABLE (a hard
// guard that can't be relaxed downstream). Stack reviewers wire open-source
// checkers (qlty, eslint, ruff, clippy…) so lint / format / complexity smells
// are caught mechanically instead of in human review.
//
// Consumed two ways:
//   · DojoLibrary — the full browser (an artboard, and the target of
//     Governance → “Add from library”).
//   · DojoStarterConstitution — the condensed picker shown while creating a
//     Dōjō (pick a few core packs to seed the constitution on day one).
//
// Reuses DojoHead / DojoChip from dojo-shared.jsx. Token-only → theme-free.

const { useState: libS } = React;

/* level a rule can be pulled in at — cascades down the same ladder as authored governance */
const LIB_LEVELS = [
  { id: "org",   label: "Org",     kanji: "社" },
  { id: "team",  label: "Team",    kanji: "組" },
  { id: "proj",  label: "Project", kanji: "件" },
  { id: "stack", label: "Stack",   kanji: "技" },
];

/* categories (left rail) */
const LIB_CATS = [
  { id: "core",    kanji: "理", name: "Core principles",        sub: "YAGNI · DRY · SRP · testability" },
  { id: "arch",    kanji: "紋", name: "Architecture & patterns", sub: "GoF · patterns.dev · modularity" },
  { id: "sec",     kanji: "盾", name: "Security",                sub: "OWASP · secrets · authz" },
  { id: "comp",    kanji: "法", name: "Compliance",              sub: "HIPAA · PCI · SOC 2 · GDPR" },
  { id: "stack",   kanji: "技", name: "Language & stack",        sub: "reviewers · lint · format · complexity" },
  { id: "design",  kanji: "意", name: "Design system & UI",      sub: "tokens · atomic · no stray pixels" },
];

/* packs, grouped by category. defLevel = the level it naturally applies at;
   rec marks the ones we recommend for a fresh Dōjō; checkers = wired OSS tools. */
const LIB_PACKS = {
  core: [
    { id: "core-first", name: "First principles", source: "sensei core", defLevel: "org", rec: true,
      rules: [
        { t: "YAGNI — build only what a current requirement needs" },
        { t: "DRY — a single source of truth for every piece of knowledge" },
        { t: "KISS — prefer the simplest design that works" },
        { t: "Single Responsibility — one reason to change per unit" },
        { t: "Composition over inheritance" },
      ] },
    { id: "core-solid", name: "SOLID", source: "R. Martin", defLevel: "org", rec: true,
      rules: [
        { t: "Single Responsibility Principle" },
        { t: "Open/Closed — open to extension, closed to modification" },
        { t: "Liskov Substitution — subtypes honour their base contract" },
        { t: "Interface Segregation — no client depends on unused methods" },
        { t: "Dependency Inversion — depend on abstractions, not concretions" },
      ] },
    { id: "core-test", name: "Testability & quality", source: "sensei core", defLevel: "org",
      rules: [
        { t: "Every behaviour change ships with a test that would fail without it" },
        { t: "Pure functions for logic; push side-effects to the edges" },
        { t: "No untested error path on a money- or auth-touching flow", hard: true },
        { t: "Fast unit suite (<10s) gates every commit" },
      ] },
  ],
  arch: [
    { id: "arch-modular", name: "Modularity & boundaries", source: "sensei core", defLevel: "org", rec: true,
      rules: [
        { t: "Feature-sliced modules — cohesive inside, thin contracts between" },
        { t: "Dependencies point inward; domain has no framework imports" },
        { t: "Wrap third-party SDKs behind an owned interface for testability" },
        { t: "No circular dependencies between modules", hard: true },
      ] },
    { id: "arch-gof", name: "Design patterns · OO (GoF)", source: "Gang of Four", defLevel: "team",
      rules: [
        { t: "Prefer Strategy over branching on type" },
        { t: "Factory / Builder for multi-step or conditional construction" },
        { t: "Adapter at every third-party boundary" },
        { t: "Observer / pub-sub for cross-module events, not direct calls" },
      ] },
    { id: "arch-patternsdev", name: "Web patterns", source: "patterns.dev", defLevel: "stack",
      rules: [
        { t: "Container/Presentational split for view logic" },
        { t: "Code-split at the route; lazy-load below the fold" },
        { t: "Debounce / throttle expensive event handlers" },
        { t: "Memoize derived data, not whole trees" },
      ] },
  ],
  sec: [
    { id: "sec-owasp", name: "OWASP baseline", source: "OWASP ASVS", defLevel: "org", rec: true,
      rules: [
        { t: "No secrets in source — use the vault, never .env in git", hard: true },
        { t: "Never log tokens or PII, even at debug level", hard: true },
        { t: "Parameterised queries only — no string-built SQL", hard: true },
        { t: "Validate & encode every external input at the boundary" },
        { t: "Authorize on every request, server-side, deny by default" },
      ] },
    { id: "sec-supply", name: "Supply chain", source: "SLSA", defLevel: "org",
      rules: [
        { t: "Pin dependency versions; no floating majors" },
        { t: "Block a merge on a high/critical advisory", hard: true },
        { t: "Generate an SBOM on every release build" },
      ] },
  ],
  comp: [
    { id: "comp-hipaa", name: "HIPAA", source: "45 CFR §164", defLevel: "org", regulated: true,
      rules: [
        { t: "Encrypt PHI at rest and in transit", hard: true },
        { t: "Immutable audit log of every PHI access", hard: true },
        { t: "Minimum-necessary access — least privilege on PHI" },
        { t: "Signed BAA before any PHI leaves the boundary", hard: true },
      ] },
    { id: "comp-pci", name: "PCI DSS", source: "PCI SSC v4", defLevel: "org", regulated: true,
      rules: [
        { t: "Never store CVV; tokenize the PAN", hard: true },
        { t: "Cardholder data isolated to a segmented scope" },
        { t: "Quarterly access review of the CDE" },
      ] },
    { id: "comp-soc2", name: "SOC 2", source: "AICPA TSC", defLevel: "org", regulated: true,
      rules: [
        { t: "Change management — every prod change is reviewed & traceable", hard: true },
        { t: "Access granted by role, revoked on offboard within 24h" },
        { t: "Encrypted backups, restore tested quarterly" },
      ] },
    { id: "comp-gdpr", name: "GDPR", source: "EU 2016/679", defLevel: "org", regulated: true,
      rules: [
        { t: "Lawful basis recorded for every category of personal data", hard: true },
        { t: "Honour erasure & export requests within 30 days" },
        { t: "Data-minimisation — collect only what the purpose needs" },
        { t: "Privacy by design on new data flows" },
      ] },
  ],
  stack: [
    { id: "stk-ts", name: "TypeScript / JavaScript", source: "reviewer", defLevel: "stack", rec: true,
      checkers: ["eslint", "prettier", "qlty"],
      rules: [
        { t: "strict: true — no implicit any", hard: true },
        { t: "No unused exports, vars or imports (eslint)" },
        { t: "Format on save (prettier) — no style diffs in review" },
        { t: "Cyclomatic complexity ≤ 10 per function (qlty)" },
      ] },
    { id: "stk-py", name: "Python", source: "reviewer", defLevel: "stack",
      checkers: ["ruff", "black", "mypy", "qlty"],
      rules: [
        { t: "ruff lint clean — pyflakes + isort + pyupgrade" },
        { t: "black formatting enforced in CI" },
        { t: "Type hints on public functions; mypy strict on core" },
        { t: "No function over 50 lines / complexity 10 (qlty)" },
      ] },
    { id: "stk-rust", name: "Rust", source: "reviewer", defLevel: "stack",
      checkers: ["clippy", "rustfmt"],
      rules: [
        { t: "clippy clean at deny(warnings)", hard: true },
        { t: "rustfmt enforced; no manual formatting" },
        { t: "No unwrap() on a fallible path outside tests" },
      ] },
    { id: "stk-go", name: "Go", source: "reviewer", defLevel: "stack",
      checkers: ["golangci-lint", "gofmt", "qlty"],
      rules: [
        { t: "golangci-lint clean before merge" },
        { t: "gofmt / goimports enforced" },
        { t: "Errors wrapped with context, never discarded" },
      ] },
  ],
  design: [
    { id: "dsn-tokens", name: "Semantic tokens", source: "design system", defLevel: "org", rec: true,
      rules: [
        { t: "Color from semantic tokens only — no raw #hex / rgba / oklch", hard: true },
        { t: "Type from the fixed scale — never a literal font-size", hard: true },
        { t: "Spacing from the 4px grid — no stray 18px / 5px values" },
        { t: "Never redefine tokens locally — link the canonical stylesheet", hard: true },
      ] },
    { id: "dsn-atomic", name: "Atomic components", source: "design system", defLevel: "team",
      rules: [
        { t: "Reuse the shared component; never hand-roll a duplicate" },
        { t: "One missing variant is added to the component, not forked inline" },
        { t: "Geometry-only inline styles; color/space/type via classes" },
        { t: "Compose screens from primitives, not bespoke markup" },
      ] },
    { id: "dsn-a11y", name: "Accessibility", source: "WCAG 2.2 AA", defLevel: "org",
      rules: [
        { t: "4.5:1 text contrast; 3:1 for large text & UI", hard: true },
        { t: "Every interactive target ≥ 44px and keyboard-reachable" },
        { t: "Semantic landmarks & labels on every control" },
      ] },
  ],
};

/* ── small pieces ─────────────────────────────────────────── */
function LibCheckbox({ on }) {
  return (
    <span style={{ width: 18, height: 18, borderRadius: "var(--radius-sm)", flexShrink: 0,
      border: on ? "none" : "2px solid var(--paper-edge)", background: on ? "var(--ink)" : "transparent",
      display: "flex", alignItems: "center", justifyContent: "center" }}>
      {on && <span style={{ color: "var(--paper)", fontSize: "var(--text-xs)", lineHeight: 1 }}>✓</span>}
    </span>
  );
}
function LibStar({ on, onClick, title }) {
  return (
    <button onClick={onClick} title={title} style={{ background: "none", border: "none", cursor: "pointer", padding: 0, lineHeight: 1,
      color: on ? "var(--warning)" : "var(--ink-faint)", fontSize: "var(--text-base)", flexShrink: 0 }}>
      {on ? "★" : "☆"}
    </button>
  );
}
function LevelPills({ value, onChange }) {
  return (
    <div style={{ display: "inline-flex", background: "var(--paper-mute)", borderRadius: "var(--radius)", padding: "2px", gap: "2px" }}>
      {LIB_LEVELS.map(l => {
        const on = l.id === value;
        return (
          <button key={l.id} onClick={() => onChange(l.id)} title={"Apply at " + l.label}
            style={{ display: "inline-flex", alignItems: "center", gap: "3px", borderRadius: "var(--radius-sm)", cursor: "pointer",
              padding: "2px var(--space-2)", fontSize: "var(--text-xs)", fontFamily: "inherit", border: "none",
              background: on ? "var(--paper)" : "transparent", color: on ? "var(--ink)" : "var(--ink-mute)",
              boxShadow: on ? "var(--shadow-sm)" : "none", fontWeight: on ? 600 : 400 }}>
            <span className="kanji" style={{ fontSize: "var(--text-xs)", color: on ? "var(--accent)" : "var(--ink-faint)" }}>{l.kanji}</span>{l.label}
          </button>
        );
      })}
    </div>
  );
}

/* checker chip — the wired open-source tool */
function CheckerChip({ name }) {
  const oss = name !== "mypy";
  return (
    <span className="mono" style={{ display: "inline-flex", alignItems: "center", gap: "3px", fontSize: "var(--text-xs)",
      color: "var(--ink-soft)", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius-full)",
      padding: "2px var(--space-2)", whiteSpace: "nowrap" }}>
      <span style={{ width: 5, height: 5, borderRadius: "50%", background: oss ? "var(--success)" : "var(--ink-faint)" }} />{name}
    </span>
  );
}

/* ── one pack card ────────────────────────────────────────── */
function LibPackCard({ pack, sel, setSel, nonNeg, setNonNeg, level, setLevel }) {
  const chosen = pack.rules.filter(r => sel[r.t]).length;
  const allOn = chosen === pack.rules.length;
  const toggleAll = () => {
    setSel(s => {
      const next = { ...s };
      pack.rules.forEach(r => { next[r.t] = !allOn; });
      return next;
    });
  };
  return (
    <div style={{ background: "var(--paper-soft)", border: chosen > 0 ? "1px solid var(--accent-edge)" : "var(--hairline)",
      borderRadius: "var(--radius-lg)", overflow: "hidden" }}>
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)", borderBottom: "var(--hairline)" }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>{pack.name}</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{pack.source}</span>
            {pack.regulated && <DojoChip tone="var(--accent)" soft="var(--accent-soft)" border="1px solid var(--accent-edge)">法 regulated</DojoChip>}
            {pack.rec && <DojoChip tone="var(--success)" soft="var(--success-soft)" border="1px solid var(--success-edge)">recommended</DojoChip>}
          </div>
          {pack.checkers && (
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-1)", flexWrap: "wrap", marginTop: "var(--space-2)" }}>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)" }}>caught by</span>
              {pack.checkers.map(c => <CheckerChip key={c} name={c} />)}
            </div>
          )}
        </div>
        <LevelPills value={level[pack.id] || pack.defLevel} onChange={l => setLevel(v => ({ ...v, [pack.id]: l }))} />
        <button onClick={toggleAll} style={{ fontSize: "var(--text-xs)", color: allOn ? "var(--ink-mute)" : "var(--accent)",
          background: "none", border: "none", cursor: "pointer", fontFamily: "inherit", whiteSpace: "nowrap" }}>
          {allOn ? "clear all" : "add all"}
        </button>
      </div>
      <div>
        {pack.rules.map((r, i) => {
          const on = !!sel[r.t];
          const nn = !!nonNeg[r.t] || (on && r.hard);
          return (
            <div key={i} style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", gap: "var(--space-3)", alignItems: "center",
              padding: "var(--space-2) var(--space-4)", borderBottom: i < pack.rules.length - 1 ? "1px solid var(--paper-edge)" : "none" }}>
              <button onClick={() => setSel(s => ({ ...s, [r.t]: !on }))}
                style={{ display: "flex", alignItems: "center", gap: "var(--space-3)", gridColumn: "1 / 3", background: "none", border: "none",
                  cursor: "pointer", fontFamily: "inherit", textAlign: "left", padding: "var(--space-1) 0", minWidth: 0 }}>
                <LibCheckbox on={on} />
                <span style={{ fontSize: "var(--text-sm)", color: on ? "var(--ink)" : "var(--ink-soft)" }}>{r.t}</span>
                {r.hard && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", whiteSpace: "nowrap" }}>hard guard</span>}
              </button>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", justifySelf: "end" }}>
                {on && (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)" }}>
                    <LibStar on={nn} onClick={() => setNonNeg(s => ({ ...s, [r.t]: !nn }))}
                      title={nn ? "Non-negotiable — click to relax" : "Mark non-negotiable"} />
                    <span style={{ fontSize: "var(--text-xs)", color: nn ? "var(--warning)" : "var(--ink-faint)", whiteSpace: "nowrap" }}>
                      {nn ? "non-negotiable" : "negotiable"}
                    </span>
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ══ FULL BROWSER ═════════════════════════════════════════════ */
function DojoLibrary({ mobile = false, scopeName = "Acme Corp" }) {
  const [cat, setCat] = libS("core");
  const [sel, setSel] = libS({});          // { ruleText: true }
  const [nonNeg, setNonNeg] = libS({});    // { ruleText: true }
  const [level, setLevel] = libS({});      // { packId: levelId }
  const [custom, setCustom] = libS([]);    // [{ t, cat, level, hard }] — user-authored rules
  const [draft, setDraft] = libS("");
  const [draftLevel, setDraftLevel] = libS("org");
  const [draftNN, setDraftNN] = libS(false);
  const packs = LIB_PACKS[cat] || [];
  const chosenCount = Object.values(sel).filter(Boolean).length + custom.length;
  // count non-negotiable across everything selected (hard guards + starred + authored)
  const allRules = Object.values(LIB_PACKS).flat().flatMap(p => p.rules);
  const nnTotal = Object.entries(sel).filter(([t, on]) => on && (nonNeg[t] || allRules.some(r => r.t === t && r.hard))).length + custom.filter(c => c.hard).length;

  const rail = (
    <div style={mobile
      ? { flexShrink: 0, display: "flex", flexWrap: "wrap", gap: "var(--space-1)", padding: "var(--space-2) var(--space-3)", borderBottom: "var(--hairline)", background: "var(--paper-soft)" }
      : { borderRight: "var(--hairline)", background: "var(--paper-soft)", overflow: "auto", padding: "var(--space-4) var(--space-3)", width: 250, flexShrink: 0 }}>
      {!mobile && <div style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-faint)", fontWeight: 600, padding: "0 var(--space-2)", marginBottom: "var(--space-2)" }}>Areas</div>}
      {LIB_CATS.map(c => {
        const on = c.id === cat;
        const packN = (LIB_PACKS[c.id] || []).length;
        if (mobile) return (
          <button key={c.id} onClick={() => setCat(c.id)} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)",
            borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-3)", fontSize: "var(--text-xs)", cursor: "pointer", fontFamily: "inherit",
            background: on ? "var(--ink)" : "transparent", color: on ? "var(--paper)" : "var(--ink-soft)", border: on ? "1px solid var(--ink)" : "var(--hairline)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-sm)", color: on ? "var(--paper)" : "var(--accent)" }}>{c.kanji}</span>{c.name}
          </button>
        );
        return (
          <button key={c.id} onClick={() => setCat(c.id)} style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "var(--space-3)", alignItems: "center",
            width: "100%", textAlign: "left", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-3)", cursor: "pointer", fontFamily: "inherit",
            marginBottom: "var(--space-1)", background: on ? "var(--paper)" : "transparent", border: on ? "var(--hairline)" : "1px solid transparent" }}>
            <span className="kanji" style={{ fontSize: "var(--text-lg)", color: on ? "var(--accent)" : "var(--ink-mute)", width: 22, textAlign: "center" }}>{c.kanji}</span>
            <div style={{ minWidth: 0 }}>
              <div style={{ fontSize: "var(--text-sm)", color: on ? "var(--ink)" : "var(--ink-soft)", fontWeight: on ? 600 : 400 }}>{c.name}</div>
              <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)", marginTop: "1px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{c.sub}</div>
            </div>
          </button>
        );
      })}
    </div>
  );

  const catMeta = LIB_CATS.find(c => c.id === cat);

  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <DojoHead mobile={mobile} kanji="蔵" eyebrow="Govern · start from proven rules" title="Constitution library"
        sub="Pull principles, patterns, compliance controls, stack reviewers and design guardrails straight into the constitution — prevention is cheaper than rework. Cherry-pick rules, set the level they apply at, and mark the ones that are non-negotiable."
        right={<DojoChip tone="var(--ink-soft)" soft="var(--paper-soft)" border="var(--hairline)">→ {scopeName}</DojoChip>} />

      <div style={mobile ? { flex: 1, display: "flex", flexDirection: "column", minHeight: 0 } : { flex: 1, display: "flex", minHeight: 0 }}>
        {rail}
        <main style={{ flex: 1, minWidth: 0, overflow: "auto", padding: mobile ? "var(--space-4)" : "var(--space-5)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-3)" }}>
            <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>{catMeta.kanji}</span>
            <span style={{ fontSize: "var(--text-lg)", color: "var(--ink)", fontWeight: 600 }} className="display">{catMeta.name}</span>
            <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>{packs.length} packs</span>
          </div>
          {/* author your own rule — classified into this area */}
          <div style={{ background: "var(--paper-soft)", border: "1px dashed var(--ink-faint)", borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)", flexWrap: "wrap" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)" }}>筆</span>
              <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 600 }}>Write your own rule</span>
              <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>classified under {catMeta.name}</span>
            </div>
            <div style={{ display: "flex", gap: "var(--space-2)", flexWrap: "wrap", alignItems: "center" }}>
              <input value={draft} onChange={e => setDraft(e.target.value)} placeholder="e.g. Feature flags removed within two releases"
                style={{ flex: 1, minWidth: 220, boxSizing: "border-box", background: "var(--paper)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-3)", fontSize: "var(--text-sm)", fontFamily: "inherit", color: "var(--ink)" }} />
              <LevelPills value={draftLevel} onChange={setDraftLevel} />
              <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)" }}>
                <LibStar on={draftNN} onClick={() => setDraftNN(v => !v)} title={draftNN ? "Non-negotiable — click to relax" : "Mark non-negotiable"} />
                <span style={{ fontSize: "var(--text-xs)", color: draftNN ? "var(--warning)" : "var(--ink-faint)", whiteSpace: "nowrap" }}>{draftNN ? "non-negotiable" : "negotiable"}</span>
              </span>
              <button disabled={!draft.trim()} onClick={() => { if (!draft.trim()) return; setCustom(cs => [...cs, { t: draft.trim(), cat, level: draftLevel, hard: draftNN }]); setDraft(""); setDraftNN(false); }}
                style={{ border: "none", borderRadius: "var(--radius)", padding: "var(--space-2) var(--space-4)", cursor: draft.trim() ? "pointer" : "default", fontFamily: "inherit", fontSize: "var(--text-sm)", fontWeight: 500,
                  background: draft.trim() ? "var(--ink)" : "var(--paper-mute)", color: draft.trim() ? "var(--paper)" : "var(--ink-faint)" }}>Add rule</button>
            </div>
            {custom.filter(c => c.cat === cat).length > 0 && (
              <div style={{ marginTop: "var(--space-3)", borderTop: "var(--hairline)", paddingTop: "var(--space-2)", display: "flex", flexDirection: "column", gap: "var(--space-1)" }}>
                {custom.filter(c => c.cat === cat).map((c, i) => (
                  <div key={i} style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", fontSize: "var(--text-sm)", color: "var(--ink)" }}>
                    <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>己</span>
                    <span style={{ flex: 1 }}>{c.t}</span>
                    <DojoChip tone="var(--ink-soft)" soft="var(--paper-mute)">{(LIB_LEVELS.find(l => l.id === c.level) || {}).label}</DojoChip>
                    {c.hard && <span style={{ fontSize: "var(--text-xs)", color: "var(--warning)" }}>★</span>}
                    <button onClick={() => setCustom(cs => cs.filter(x => x !== c))} title="Remove" style={{ background: "none", border: "none", cursor: "pointer", color: "var(--ink-faint)", fontSize: "var(--text-base)", lineHeight: 1 }}>×</button>
                  </div>
                ))}
              </div>
            )}
          </div>
          {cat === "stack" && (
            <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", background: "var(--paper-soft)", border: "var(--hairline)",
              borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", flexShrink: 0 }}>検</span>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
                Each stack has a <b style={{ fontWeight: 600, color: "var(--ink-soft)" }}>reviewer</b> wired to open-source checkers — <span className="mono">qlty</span>, <span className="mono">eslint</span>, <span className="mono">ruff</span>, <span className="mono">clippy</span> and friends. Lint, format and complexity smells are caught mechanically before a human ever reads the diff. No duplication, no bikeshedding.
              </span>
            </div>
          )}
          {cat === "comp" && (
            <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", background: "var(--accent-soft)", border: "1px solid var(--accent-edge)",
              borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", flexShrink: 0 }}>法</span>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-soft)", lineHeight: 1.5 }}>
                Compliance controls come pre-written from the framework. Cherry-pick exactly the ones in scope, then <b style={{ fontWeight: 600 }}>mark them non-negotiable</b> (★) so no scope below can relax them. These are the wedge for regulated and agency work.
              </span>
            </div>
          )}
          {cat === "design" && (
            <div style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-2)", background: "var(--paper-soft)", border: "var(--hairline)",
              borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)", marginBottom: "var(--space-4)" }}>
              <span className="kanji" style={{ fontSize: "var(--text-base)", color: "var(--accent)", flexShrink: 0 }}>意</span>
              <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", lineHeight: 1.5 }}>
                These guards catch drift at the source — a raw <span className="mono">#hex</span>, a literal <span className="mono">font-size</span>, a locally redefined token. The exact mistakes that quietly grow a design system into thirty font sizes and a broken color ramp. Caught on write, not in a later audit.
              </span>
            </div>
          )}
          <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(auto-fit, minmax(420px, 1fr))", gap: "var(--space-3)", paddingBottom: "var(--space-6)" }}>
            {packs.map(p => (
              <LibPackCard key={p.id} pack={p} sel={sel} setSel={setSel} nonNeg={nonNeg} setNonNeg={setNonNeg} level={level} setLevel={setLevel} />
            ))}
          </div>
        </main>
      </div>

      {/* sticky selection footer */}
      <div style={{ flexShrink: 0, borderTop: "var(--hairline)", background: "var(--paper-soft)",
        display: "flex", alignItems: "center", gap: "var(--space-4)", padding: mobile ? "var(--space-3) var(--space-4)" : "var(--space-3) var(--space-6)", flexWrap: "wrap" }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: "var(--space-2)" }}>
          <span className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, color: "var(--ink)", lineHeight: 1 }}>{chosenCount}</span>
          <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>rules selected</span>
        </div>
        {nnTotal > 0 && (
          <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-1)", fontSize: "var(--text-xs)", color: "var(--warning)",
            background: "var(--warning-soft)", border: "1px solid var(--warning-edge)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-3)" }}>
            ★ {nnTotal} non-negotiable
          </span>
        )}
        <span style={{ flex: 1 }} />
        <span style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }} className="mono">cascades to every scope below</span>
        <button disabled={chosenCount === 0} style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)",
          background: chosenCount === 0 ? "var(--paper-mute)" : "var(--ink)", color: chosenCount === 0 ? "var(--ink-faint)" : "var(--paper)",
          border: "none", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-5)", cursor: chosenCount === 0 ? "default" : "pointer",
          fontFamily: "inherit", fontSize: "var(--text-sm)", fontWeight: 500 }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: chosenCount === 0 ? "var(--ink-faint)" : "var(--accent)" }}>掟</span>
          Add {chosenCount || ""} to constitution
        </button>
      </div>
    </div>
  );
}

/* ══ STARTER PICKER (during Dōjō create) ═════════════════════ */
// Condensed: pick whole packs to seed the constitution. Grouped-pack toggles,
// recommended pre-checked. A tighter surface than the full browser.
function DojoStarterConstitution({ mobile = false }) {
  const starter = [
    { cat: "core",   packs: ["core-first", "core-solid", "core-test"] },
    { cat: "arch",   packs: ["arch-modular", "arch-gof"] },
    { cat: "sec",    packs: ["sec-owasp", "sec-supply"] },
    { cat: "comp",   packs: ["comp-hipaa", "comp-pci", "comp-soc2", "comp-gdpr"] },
    { cat: "stack",  packs: ["stk-ts", "stk-py", "stk-rust", "stk-go"] },
    { cat: "design", packs: ["dsn-tokens", "dsn-atomic", "dsn-a11y"] },
  ];
  const byId = {}; Object.values(LIB_PACKS).flat().forEach(p => { byId[p.id] = p; });
  const [on, setOn] = libS(() => {
    const init = {}; Object.values(LIB_PACKS).flat().forEach(p => { if (p.rec) init[p.id] = true; });
    return init;
  });
  const total = Object.values(LIB_PACKS).flat().filter(p => on[p.id]).reduce((n, p) => n + p.rules.length, 0);

  return (
    <div className="sensei" data-screen-label="Create · starter constitution" style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <div style={{ height: 54, flexShrink: 0, display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "0 var(--space-5)", borderBottom: "var(--hairline)" }}>
        <span className="kanji" style={{ fontSize: "var(--text-xl)", color: "var(--accent)", lineHeight: 1 }}>結</span>
        <span className="display" style={{ fontSize: "var(--text-lg)", letterSpacing: "-0.01em" }}>Dōjō</span>
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-faint)" }}>create · step 2 of 2</span>
        <span style={{ flex: 1 }} />
        <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", background: "none", border: "none" }}>skip — start empty</span>
      </div>
      <div style={{ flex: 1, overflow: "auto", padding: mobile ? "var(--space-5) 0" : "var(--space-6) 0" }}>
        <div style={{ maxWidth: 720, margin: "0 auto", padding: mobile ? "0 var(--space-4)" : "0 var(--space-6)" }}>
          <span className="kanji" style={{ fontSize: "var(--text-2xl)", color: "var(--accent)", lineHeight: 1 }}>蔵</span>
          <h1 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: "var(--space-4) 0 0", lineHeight: 1.1 }}>Seed the constitution</h1>
          <p style={{ fontSize: "var(--text-sm)", color: "var(--ink-soft)", lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-5)" }}>
            Start from proven rules instead of a blank page — prevention is cheaper than rework. Pick the packs that fit; refine, add compliance controls and mark non-negotiables anytime in Governance.
          </p>
          {starter.map(grp => {
            const c = LIB_CATS.find(x => x.id === grp.cat);
            return (
              <div key={grp.cat} style={{ marginBottom: "var(--space-5)" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", marginBottom: "var(--space-2)" }}>
                  <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>{c.kanji}</span>
                  <span style={{ fontSize: "var(--text-xs)", letterSpacing: ".14em", textTransform: "uppercase", color: "var(--ink-mute)", fontWeight: 600 }}>{c.name}</span>
                </div>
                <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: "var(--space-2)" }}>
                  {grp.packs.map(pid => {
                    const p = byId[pid]; if (!p) return null;
                    const active = !!on[pid];
                    return (
                      <button key={pid} onClick={() => setOn(s => ({ ...s, [pid]: !active }))}
                        style={{ display: "flex", alignItems: "flex-start", gap: "var(--space-3)", textAlign: "left", cursor: "pointer", fontFamily: "inherit",
                          background: active ? "var(--paper-soft)" : "var(--paper)", border: active ? "1px solid var(--accent)" : "var(--hairline)",
                          borderRadius: "var(--radius-lg)", padding: "var(--space-3) var(--space-4)" }}>
                        <LibCheckbox on={active} />
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", flexWrap: "wrap" }}>
                            <span style={{ fontSize: "var(--text-sm)", color: "var(--ink)", fontWeight: 500 }}>{p.name}</span>
                            {p.regulated && <span className="mono" style={{ fontSize: "var(--text-xs)", color: "var(--accent)" }}>法</span>}
                          </div>
                          <div style={{ fontSize: "var(--text-xs)", color: "var(--ink-mute)", marginTop: "1px" }}>
                            {p.rules.length} rules · <span className="mono">{p.source}</span>
                            {p.checkers && <span> · {p.checkers.slice(0, 3).join(" ")}</span>}
                          </div>
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </div>
      <div style={{ flexShrink: 0, borderTop: "var(--hairline)", background: "var(--paper-soft)", display: "flex", alignItems: "center", gap: "var(--space-3)", padding: "var(--space-3) var(--space-6)" }}>
        <span style={{ fontSize: "var(--text-sm)", color: "var(--ink-mute)" }}>
          <b style={{ fontWeight: 600, color: "var(--ink)" }}>{total}</b> rules will seed the constitution
        </span>
        <span style={{ flex: 1 }} />
        <button style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)", background: "var(--ink)", color: "var(--paper)",
          border: "none", borderRadius: "var(--radius-lg)", padding: "var(--space-2) var(--space-5)", cursor: "pointer", fontFamily: "inherit", fontSize: "var(--text-sm)", fontWeight: 500 }}>
          <span className="kanji" style={{ fontSize: "var(--text-sm)", color: "var(--accent)" }}>結</span> Create Dōjō with these
        </button>
      </div>
    </div>
  );
}

Object.assign(window, { DojoLibrary, DojoStarterConstitution, LIB_PACKS, LIB_CATS });
