// ─── Intake · the front door (S1 + S3 + S2/S4) ───────────────────────
//
// The only PROSPECTIVE surface in the Observatory: the user describes a
// chunk of work they're about to start, sensei classifies it on three
// axes (lifecycle · intent · risk) and recommends a playbook — a way of
// working proportional to the risk. Recommend-and-confirm, with
// auto-select for trusted low-risk chunks and forced explicit confirm
// for high-risk ones.
//
// Three in-content tabs (the SUBNAV pattern): Start · Playbooks · History.
// The structured twin of the CLI `/sensei:intake`.
//
// STYLING: semantic utility classes + var() geometry only. No raw hex.

const { useState: ikS } = React;

// ── Playbook catalog — the six built-ins (runtime-extensible) ──────────
const PLAYBOOKS = [
  { name: "vibe",         kanji: "空", title: "Vibe / spike",   when: "Greenfield, objective fuzzy — explore then extract learnings (discardable).", tone: "Explore fast and loose; capture what you learn, keep nothing you cannot justify." },
  { name: "mockup_first", kanji: "図", title: "Mockup-first",   when: "Greenfield, UX-heavy — design the surface before the spec.",                   tone: "Start from the mockup; let the UI shape the spec." },
  { name: "spec_driven",  kanji: "案", title: "Spec-driven",    when: "Clear objective + high blast-radius — force a deep design first.",            tone: "Slow down: write the design, enumerate edge cases, before any code." },
  { name: "gsd",          kanji: "進", title: "Get stuff done", when: "Known feature, low risk — lean plan then build.",                            tone: "Lean plan, then build; keep it tight." },
  { name: "change_flow",  kanji: "変", title: "Change-flow",    when: "Stable product enhancement — impact analysis then targeted design.",         tone: "Map impact first; design the smallest change that lands the value." },
  { name: "debug_flow",   kanji: "直", title: "Debug-flow",     when: "Stable product bug — reproduce, fix, add a regression test.",                tone: "Reproduce first; fix; lock it with a regression test." },
];
const PB = Object.fromEntries(PLAYBOOKS.map(p => [p.name, p]));

// ── The three axes (exact values from the contract) ────────────────────
const AXES = [
  { key: "lifecycle", label: "Lifecycle", values: ["greenfield", "stable"] },
  { key: "intent",    label: "Intent",    values: ["explore", "ux", "feature", "enhancement", "bug"] },
  { key: "risk",      label: "Risk",      values: ["low", "high"] },
];

// ── Example chunks — each demonstrates a different card variant ─────────
const EXAMPLES = [
  { text: "fix the null deref when the session token refreshes",
    match: ["deref", "null", "crash", "token refresh"],
    rec: { playbook: "debug_flow", lifecycle: "stable", intent: "bug", risk: "low",
           rationale: "A localized crash in a stable path — reproduce, fix, lock it.",
           auto_select: true, trust: { n: 12, ftr: 0.9 } } },
  { text: "prototype a new onboarding wizard, still figuring out the shape",
    match: ["onboarding", "prototype", "wizard"],
    rec: { playbook: "mockup_first", lifecycle: "greenfield", intent: "ux", risk: "low",
           rationale: "New UX surface with a fuzzy shape — design it before the spec.",
           auto_select: false, trust: { n: 0, ftr: null } } },
  { text: "rename the session store used across the app",
    match: ["rename", "session store", "across the app"],
    rec: { playbook: "spec_driven", lifecycle: "stable", intent: "enhancement", risk: "high",
           rationale: "Touches every caller of the store — high blast-radius, design first.",
           auto_select: false, trust: { n: 4, ftr: 0.82 } } },
];

// pick the recommendation for a description (nearest example, else lean gsd)
function classify(text) {
  const t = (text || "").toLowerCase();
  if (/\b(fail|error|crash|throw)\b/.test(t) && /demo-?error/.test(t)) return "__error__";
  for (const ex of EXAMPLES) {
    if (ex.match.some(m => t.includes(m))) return ex.rec;
  }
  if (/\b(bug|crash|null|deref|broken|regression)\b/.test(t))
    return { playbook: "debug_flow", lifecycle: "stable", intent: "bug", risk: "low",
             rationale: "Reads as a bug in a stable path — reproduce, fix, add a regression test.",
             auto_select: false, trust: { n: 6, ftr: 0.83 } };
  if (/\b(rewrite|rename|migrate|refactor|across|every)\b/.test(t))
    return { playbook: "spec_driven", lifecycle: "stable", intent: "enhancement", risk: "high",
             rationale: "Wide blast-radius — design and enumerate edge cases before code.",
             auto_select: false, trust: { n: 0, ftr: null } };
  if (/\b(explore|spike|try|experiment|prototype|figuring)\b/.test(t))
    return { playbook: "vibe", lifecycle: "greenfield", intent: "explore", risk: "low",
             rationale: "Objective still fuzzy — explore, then extract what's worth keeping.",
             auto_select: false, trust: { n: 0, ftr: null } };
  return { playbook: "gsd", lifecycle: "stable", intent: "feature", risk: "low",
           rationale: "A known feature at low risk — lean plan, then build.",
           auto_select: false, trust: { n: 3, ftr: 0.78 } };
}

// re-derive a playbook when the user overrides an axis (challengeable read)
function playbookFor({ lifecycle, intent, risk }) {
  if (intent === "bug") return "debug_flow";
  if (risk === "high") return "spec_driven";
  if (lifecycle === "greenfield" && intent === "ux") return "mockup_first";
  if (lifecycle === "greenfield" && intent === "explore") return "vibe";
  if (lifecycle === "stable" && intent === "enhancement") return "change_flow";
  return "gsd";
}

// ── Axis chip — value shown back, with an edit affordance ──────────────
function AxisChip({ axis, value, onChange }) {
  const [open, setOpen] = ikS(false);
  const high = axis.key === "risk" && value === "high";
  const skin = high
    ? "text-warning bg-warning-soft border border-warning-edge"
    : "text-ink-soft bg-paper border border-paper-edge";
  return (
    <span className="relative inline-flex" >
      <button onClick={() => setOpen(o => !o)}
        className={"inline-flex items-center mono text-xs rounded-full " + skin}
        style={{ gap: 6, padding: "4px 8px 4px 10px", cursor: "pointer", whiteSpace: "nowrap" }}
        title={"sensei's read — click to correct"}>
        <span className="text-ink-faint" style={{ letterSpacing: ".04em" }}>{axis.label.toLowerCase()}</span>
        <span className="font-semibold" >{value}</span>
        <svg width="9" height="9" viewBox="0 0 16 16" style={{ opacity: 0.5 }}>
          <path d="M4 6 L8 10 L12 6" fill="none" stroke="currentColor" strokeWidth="2"
                strokeLinecap="round" strokeLinejoin="round"/>
        </svg>
      </button>
      {open && (
        <div className="bg-paper border border-paper-edge rounded shadow-lg absolute flex flex-col"
 style={{ top: "calc(100% + 4px)", left: 0, zIndex: 20,
 minWidth: 130, padding: 4, gap: 2 }}>
          {axis.values.map(v => (
            <button key={v} onClick={() => { onChange(v); setOpen(false); }}
              className={"mono text-xs rounded " + (v === value ? "text-accent bg-paper-mute" : "text-ink-soft")}
              style={{ textAlign: "left", padding: "5px 8px", cursor: "pointer",
                       border: "none", background: v === value ? "var(--paper-3)" : "transparent" }}>
              {v}
            </button>
          ))}
        </div>
      )}
    </span>
  );
}

// ── Recommendation card (S3) ───────────────────────────────────────────
function RecommendationCard({ rec, recorded, onAxisChange, onConfirm, onReset }) {
  const pb = PB[rec.playbook];
  const highRisk = rec.risk === "high";
  const trusted = rec.trust && rec.trust.n > 0 && rec.trust.ftr != null;
  const auto = rec.auto_select && !highRisk;

  return (
    <div className={"flex flex-col gap-4 rounded-lg border border-paper-edge "
                    + (highRisk ? "bg-warning-soft border-warning-edge" : "bg-paper-soft")}
         style={{ padding: "var(--space-6)", animation: "ikRise var(--dur-slow) var(--ease) both" }}>

      {/* Header — playbook identity + trust/auto badges */}
      <div className="flex items-start gap-3">
        <span className="kanji text-accent shrink-0" style={{ fontSize: 28, lineHeight: 1, width: 32 }}>{pb.kanji}</span>
        <div className="flex-1 min-w-0" >
          <div className="flex items-center gap-2 flex-wrap" >
            <span className="text-lg font-semibold text-ink">{pb.title}</span>
            <span className="mono text-xs text-ink-faint">{pb.name}</span>
            {trusted && (
              <span className="mono text-xs rounded-full text-success bg-success-soft border border-success-edge whitespace-nowrap"
 style={{ padding: "2px 9px" }}>
                FTR {rec.trust.ftr.toFixed(2)} · {rec.trust.n} runs
              </span>
            )}
            {auto && (
              <span className="mono text-xs rounded-full text-accent bg-paper-mute border border-paper-edge"
                    style={{ padding: "2px 9px" }}>auto-selected</span>
            )}
          </div>
          <div className="text-sm text-ink-soft" style={{ marginTop: 4, lineHeight: 1.5 }}>{rec.rationale}</div>
        </div>
      </div>

      {/* Sensei's read — the three axes, challengeable */}
      <div>
        <div className="text-xs text-ink-faint uppercase mb-2" style={{ letterSpacing: ".14em" }}>sensei read</div>
        <div className="flex items-center gap-2 flex-wrap" >
          {AXES.map(ax => (
            <AxisChip key={ax.key} axis={ax} value={rec[ax.key]}
                      onChange={recorded ? () => {} : (v) => onAxisChange(ax.key, v)}/>
          ))}
        </div>
      </div>

      {/* Opening tone — the posture sensei adopts once chosen */}
      <div className="flex items-start gap-2 border-t pt-3" >
        <span className="kanji text-ink-faint shrink-0" style={{ fontSize: 13, lineHeight: 1.4 }}>静</span>
        <span className="text-sm text-ink-mute italic" style={{ lineHeight: 1.5 }}>“{pb.tone}”</span>
      </div>

      {/* Confirm / recorded footer */}
      {recorded ? (
        <div className="flex items-center gap-2 border-t pt-4" >
          <span className="inline-flex items-center justify-center rounded-full text-success bg-success-soft border border-success-edge shrink-0"
 style={{ width: 22, height: 22 }}>
            <svg width="12" height="12" viewBox="0 0 16 16"><path d="M3.5 8.5 L6.5 11.5 L12.5 4.5"
              fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round"/></svg>
          </span>
          <span className="text-sm text-ink-soft flex-1">
            {auto ? "Auto-selected and recorded — say “change” to pick another." : "Recorded. Go work in this playbook."}
          </span>
          <button onClick={onReset} className="zs-btn zs-btn-ghost zs-btn-sm">New intake</button>
        </div>
      ) : (
        <div className="flex flex-col gap-2 border-t pt-4" >
          {highRisk && (
            <div className="text-sm text-warning" style={{ lineHeight: 1.45 }}>
              High blast-radius — sensei won’t auto-select this. Confirm to proceed.
            </div>
          )}
          <div className="flex items-center gap-3">
            <button onClick={onConfirm} className="zs-btn zs-btn-primary">Use this playbook</button>
            <button onClick={onReset} className="zs-btn zs-btn-ghost">Start over</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Start tab — describe → recommend → confirm ─────────────────────────
function IntakeStart() {
  const [phase, setPhase]   = ikS("describe");   // describe · loading · recommended · recorded · error
  const [text, setText]     = ikS("");
  const [rec, setRec]       = ikS(null);
  const [err, setErr]       = ikS(null);

  const runRecommend = () => {
    setPhase("loading");
    setTimeout(() => {
      const r = classify(text);
      if (r === "__error__") {
        setErr("daemon did not respond — the classifier is offline.");
        setPhase("error");
        return;
      }
      setRec(r);
      // trusted low-risk → auto-select opens already recorded (never for high risk)
      setPhase(r.auto_select && r.risk !== "high" ? "recorded" : "recommended");
    }, 850);
  };

  const onAxisChange = (key, value) => {
    const next = { ...rec, [key]: value };
    next.playbook = playbookFor(next);
    // an override is the human's read, not a trusted track record → no auto
    next.auto_select = false;
    setRec(next);
  };

  const reset = () => { setPhase("describe"); setText(""); setRec(null); setErr(null); };

  return (
    <div style={{ maxWidth: 720 }} className="flex flex-col gap-6 mx-auto w-full">
      {/* Frame — the grounding prompt */}
      <div className="flex items-start gap-3">
        <span className="kanji text-accent shrink-0" style={{ fontSize: 22, lineHeight: 1.2 }}>門</span>
        <div>
          <div className="text-base text-ink">Describe the work you’re about to start.</div>
          <div className="text-sm text-ink-mute" style={{ marginTop: 2 }}>
            Sensei reads the chunk, classifies it, and recommends a way of working. You confirm before work begins.
          </div>
        </div>
      </div>

      {/* Freeform input */}
      <div className="flex flex-col gap-3">
        <textarea
 value={text}
 onChange={e => setText(e.target.value)}
 disabled={phase === "loading"}
 placeholder="e.g. fix the crash when the token refreshes…"
 className="text-base text-ink bg-paper border border-paper-edge rounded-lg w-full p-4"
 style={{ minHeight: 96, resize: "vertical",
 lineHeight: 1.5, fontFamily: "inherit", outline: "none" }}/>

        {/* Example chunks — quick fills that show different reads */}
        {(phase === "describe") && (
          <div className="flex items-center gap-2 flex-wrap" >
            <span className="text-xs text-ink-faint" style={{ marginRight: 2 }}>try:</span>
            {EXAMPLES.map(ex => (
              <button key={ex.text} onClick={() => setText(ex.text)}
 className="text-xs text-ink-soft bg-paper-soft border border-paper-edge rounded-full cursor-pointer text-left"
 style={{ padding: "5px 12px" }}>
                {ex.text.length > 42 ? ex.text.slice(0, 40) + "…" : ex.text}
              </button>
            ))}
          </div>
        )}

        <div className="flex items-center gap-3">
          <button onClick={runRecommend}
                  disabled={!text.trim() || phase === "loading"}
                  className="zs-btn zs-btn-primary"
                  style={{ opacity: (!text.trim() || phase === "loading") ? 0.5 : 1 }}>
            {phase === "loading"
              ? <span className="inline-flex items-center gap-2">
                  <svg className="zs-spin" width="13" height="13" viewBox="0 0 16 16">
                    <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="2" strokeOpacity="0.25"/>
                    <path d="M8 2 a6 6 0 0 1 6 6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                  </svg>
                  reading the chunk…
                </span>
              : "Recommend a playbook"}
          </button>
          {phase === "loading" &&
            <span className="text-sm text-ink-mute italic" >Still listening.</span>}
        </div>
      </div>

      {/* Recommendation / recorded */}
      {(phase === "recommended" || phase === "recorded") && rec && (
        <RecommendationCard rec={rec} recorded={phase === "recorded"}
          onAxisChange={onAxisChange}
          onConfirm={() => setPhase("recorded")}
          onReset={reset}/>
      )}

      {/* Error — actionable */}
      {phase === "error" && (
        <div className="flex items-start gap-3 rounded-lg bg-danger-soft border border-danger-edge p-4"
 >
          <span className="text-danger shrink-0" style={{ marginTop: 1 }}>
            <svg width="14" height="14" viewBox="0 0 16 16"><path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5"
              fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"/></svg>
          </span>
          <div className="flex-1 text-sm text-ink-soft" style={{ lineHeight: 1.5 }}>
            Couldn’t classify — <span className="mono text-danger">{err}</span>
          </div>
          <button onClick={runRecommend} className="zs-btn zs-btn-secondary zs-btn-sm">Retry</button>
        </div>
      )}
    </div>
  );
}

// ── Playbooks tab (S2) — browse the six methods ────────────────────────
function IntakePlaybooks() {
  return (
    <div style={{ maxWidth: 860 }} className="flex flex-col gap-4 mx-auto w-full">
      <div className="text-sm text-ink-mute" style={{ lineHeight: 1.5 }}>
        The ways of working sensei can pick from. Depth follows risk — the catalog grows over time.
      </div>
      <div className="grid gap-4" style={{ gridTemplateColumns: "1fr 1fr" }}>
        {PLAYBOOKS.map(pb => (
          <div key={pb.name} className="flex flex-col gap-3 rounded-lg border border-paper-edge bg-paper-soft p-6"
 >
            <div className="flex items-center gap-3">
              <span className="kanji text-accent shrink-0" style={{ fontSize: 24, lineHeight: 1, width: 28 }}>{pb.kanji}</span>
              <span className="text-base font-semibold text-ink flex-1">{pb.title}</span>
              <span className="mono text-xs text-ink-faint">{pb.name}</span>
            </div>
            <div className="text-sm text-ink-soft" style={{ lineHeight: 1.5 }}>{pb.when}</div>
            <div className="text-sm text-ink-mute italic border-t pt-3" style={{ lineHeight: 1.5 }}>“{pb.tone}”</div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── History tab (S4) — recorded chunks + per-combo FTR (your own) ──────
const IK_HISTORY = [
  { chunk: "fix the null deref when the session token refreshes", pb: "debug_flow",  lifecycle: "stable",     intent: "bug",         risk: "low",  when: "2h ago",       ftr: 1 },
  { chunk: "add a per-project FTR sparkline to Today",            pb: "gsd",         lifecycle: "stable",     intent: "feature",     risk: "low",  when: "yesterday",    ftr: 1 },
  { chunk: "map impact of moving the capture buffer off-thread",  pb: "change_flow", lifecycle: "stable",     intent: "enhancement", risk: "low",  when: "yesterday",    ftr: 0 },
  { chunk: "prototype the relay phone view",                      pb: "mockup_first",lifecycle: "greenfield", intent: "ux",          risk: "low",  when: "3 days ago",   ftr: 1 },
  { chunk: "rename the session store used across the app",        pb: "spec_driven", lifecycle: "stable",     intent: "enhancement", risk: "high", when: "last week",    ftr: null },
];
function IntakeHistory() {
  // per-combo rollup (your own stats — the accept/reject of learned rules lives in Dōjō)
  const combos = {};
  IK_HISTORY.forEach(h => {
    const k = h.pb;
    (combos[k] = combos[k] || { n: 0, landed: 0 });
    combos[k].n++;
    if (h.ftr === 1) combos[k].landed++;
  });
  return (
    <div style={{ maxWidth: 820 }} className="flex flex-col gap-6 mx-auto w-full">
      {/* per-playbook FTR (your own) */}
      <div className="flex flex-col gap-2">
        <div className="text-xs text-ink-faint uppercase" style={{ letterSpacing: ".14em" }}>your reliability</div>
        <div className="flex items-center gap-2 flex-wrap" >
          {Object.entries(combos).map(([name, c]) => (
            <span key={name} className="inline-flex items-center gap-2 mono text-xs bg-paper-soft border border-paper-edge rounded-full"
                  style={{ padding: "5px 12px" }}>
              <span className="kanji text-accent" style={{ fontSize: 12 }}>{PB[name].kanji}</span>
              <span className="text-ink-soft">{PB[name].title}</span>
              <span className="text-ink-faint">FTR {c.n ? (c.landed / c.n).toFixed(2) : "—"} · {c.n}</span>
            </span>
          ))}
        </div>
        <div className="text-xs text-ink-mute" style={{ marginTop: 2 }}>
          These are your own stats. Proposed rule changes are reviewed in the Dōjō.
        </div>
      </div>

      {/* recorded chunks */}
      <div className="rounded-lg border border-paper-edge bg-paper-soft overflow-hidden" >
        {IK_HISTORY.map((h, i) => (
          <div key={i} className="flex items-center gap-3 py-3 px-4"
 style={{
 borderBottom: i < IK_HISTORY.length - 1 ? "var(--hairline)" : "none" }}>
            <span className="kanji text-accent shrink-0" style={{ fontSize: 15, width: 18 }}>{PB[h.pb].kanji}</span>
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{h.chunk}</div>
              <div className="mono text-xs text-ink-faint">
                {PB[h.pb].title} · {h.lifecycle} · {h.intent} · {h.risk}
              </div>
            </div>
            <span className="mono text-xs text-ink-faint shrink-0" >{h.when}</span>
            <span className="shrink-0 text-right" style={{ width: 74 }}>
              {h.ftr == null
                ? <span className="mono text-xs text-ink-faint">pending</span>
                : h.ftr === 1
                  ? <span className="mono text-xs text-success">first-try</span>
                  : <span className="mono text-xs text-warning">corrected</span>}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── The screen — header + tab strip + body ─────────────────────────────
const IK_TABS = [["start", "Start"], ["playbooks", "Playbooks"], ["history", "History"]];
function IntakeScreen() {
  const [tab, setTab] = ikS("start");
  return (
    <div className="w-full h-full flex flex-col overflow-hidden bg-paper" >
      {/* header */}
      <div className="border-b shrink-0" style={{ padding: "var(--space-6) var(--space-8) var(--space-4)" }}>
        <div className="text-xs text-ink-faint uppercase" style={{ letterSpacing: ".18em" }}>先生 · front door</div>
        <div className="flex items-baseline gap-3" style={{ marginTop: 4 }}>
          <span className="zs-h2 text-ink m-0" >Start a chunk</span>
          <span className="text-sm text-ink-mute">sensei recommends a way of working — proportional to the risk.</span>
        </div>
      </div>
      {/* tab strip — SUBNAV pattern */}
      <div className="flex items-center gap-1 py-3 px-8 border-b bg-paper shrink-0" >
        {IK_TABS.map(([id, label]) => {
          const on = tab === id;
          return (
            <button key={id} onClick={() => setTab(id)}
              className={"text-sm rounded " + (on ? "text-ink" : "text-ink-mute")}
              style={{ padding: "6px 12px", cursor: "pointer", border: "none",
                       fontFamily: "inherit", fontWeight: on ? 600 : 400,
                       background: on ? "var(--paper-3)" : "transparent" }}>
              {label}
            </button>
          );
        })}
      </div>
      {/* body */}
      <div className="flex-1 min-h-0 overflow-auto p-8" >
        {tab === "start"     && <IntakeStart/>}
        {tab === "playbooks" && <IntakePlaybooks/>}
        {tab === "history"   && <IntakeHistory/>}
      </div>
    </div>
  );
}

if (!document.getElementById("ik-kf")) {
  const s = document.createElement("style");
  s.id = "ik-kf";
  s.textContent = "@keyframes ikRise{from{opacity:0;transform:translateY(4px)}to{opacity:1;transform:none}}";
  document.head.appendChild(s);
}

Object.assign(window, { IntakeScreen, INTAKE_PLAYBOOKS: PLAYBOOKS });
