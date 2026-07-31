// Sensei — Setup Wizard (10 stages) + Empty Observatory shell.
// Full-bleed flow; reuses primitives. Hybrid layout:
//   - left rail: stepper (completed steps show as collapsed "chips", current expanded)
//   - main area: current stage content
//   - bottom bar: primary/secondary actions + terse progress

const { useState: useS, useEffect: useE, useRef: useR, useMemo: useM } = React;

// ─────────────────────────────────────────────────────────────
// Settings persistence helpers — used by the Preferences stage and read by
// the observatory chrome (welcome toast, sensei tone, sharing schedule…).
// Stored as a single JSON blob in localStorage; cross-window updates emit a
// "sensei-settings-changed" CustomEvent so listeners refresh in place.
function readSenseiSettings() {
  try {
    const raw = localStorage.getItem("sensei-settings");
    return raw ? JSON.parse(raw) : {};
  } catch { return {}; }
}
function writeSenseiSettings(patch) {
  const next = { ...readSenseiSettings(), ...patch };
  try { localStorage.setItem("sensei-settings", JSON.stringify(next)); } catch {}
  window.dispatchEvent(new CustomEvent("sensei-settings-changed"));
  return next;
}

// ─────────────────────────────────────────────────────────────
// Stage list (order matters)
// Stage list (order matters)
//
// Each stage's `n` is a *meaning* kanji, not a counter — the glyph
// reinforces what the step is *about*, and survives reordering without
// going stale. The numeric "01 / 11" lives in the bottom bar for a sense
// of progression. Mapping:
//   礼 welcome     · the teacher's bow / receiving you
//   名 preferences · "name" — the input itself
//   連 assistants  · "connect / link" the agents in
//   庵 folders     · "hermitage / dwelling" — where your work lives
//   観 scan        · "observe / watch" — sensei's core verb
//   組 projects    · "assemble / group" repos into a project
//   書 libraries   · "writing / book" — the libs sensei wraps
//   器 instruments · "vessel / instrument" — sensei's tools
//   路 routers     · "route / path" — where models come from
//   任 assignments · "entrust / assign" roles to models
//   入 done        · "enter" — the door at the end
// Rail subs are written in the same voice as the page taglines —
// short complete sentences, lowercase sensei, second-person, periods.
const WIZ_STAGES = [
  { id: "welcome",     n: "礼",  title: "Welcome",         sub: "A quiet observer. Nothing more." },
  { id: "preferences", n: "名",  title: "Profile",          sub: "Your name, and how forward sensei is." },
  { id: "acps",        n: "連",  title: "Assistants",      sub: "Connect the AI tools you already use." },
  { id: "folders",     n: "庵",  title: "Folders",         sub: "Where your work lives." },
  { id: "scan",        n: "観",  title: "Scan",            sub: "Workers recurse. Repos surface." },
  { id: "projects",    n: "組",  title: "Projects",        sub: "Each project, one or more repos." },
  { id: "libraries",   n: "書",  title: "Libraries",       sub: "What sensei should wrap." },
  { id: "registry",    n: "器",  title: "Instruments",     sub: "Tools sensei can reach for." },
  { id: "inference",   n: "路",  title: "Routers",          sub: "Where models come from — local and cloud." },
  { id: "assignments", n: "任",  title: "Assignments",     sub: "Which model handles which role." },
  { id: "done",        n: "入",  title: "Enter",           sub: "The observatory is ready." }
];

// Preferences mode reorders the same stages so Scan sits first ("two ways
// to reach the scan: on first load, and here when changes are needed") and
// drops the onboarding-only bookends (Welcome / Enter). Everything else is
// the same screens — just reachable for editing, not as a linear gate.
const PREF_ORDER = ["scan", "folders", "projects", "acps",
                    "libraries", "registry", "inference", "assignments", "preferences"];

// ─────────────────────────────────────────────────────────────────
// Root wizard. mode:
//   "setup"        — linear first-time gate (legacy / reference)
//   "preferences"  — settings surface; scan first, free navigation, no gate
function SetupWizard({ onDone, onExit, mode = "setup" }) {
  const D = window.SENSEI_SETUP;
  const isPrefs = mode === "preferences";
  const stages = isPrefs
    ? PREF_ORDER.map(id => WIZ_STAGES.find(s => s.id === id)).filter(Boolean)
    : WIZ_STAGES;
  const [stageIdx, setStageIdx] = useS(0);
  const stage = stages[stageIdx];

  // accumulated state — realistic enough to read as "a thing being configured"
  const [state, setState] = useS({
    components: { variant: "partial", acting: {} }, // acting[id] = "installing" | "done"
    acps:       D.acps.reduce((a,x)=> (a[x.id]=x.found, a), {}), // register checkbox
    folders:    [...D.folders],                   // array of {id, path, note}
    newFolder:  "",
    scan:       { started: false, done: false, tick: 0, eventsShown: 0 },
    solutions:  D.discoveredSolutions.map(s => ({ ...s, confirmed: true, renamed: null })),
    roles:      D.discoveredSolutions.flatMap(s => s.projects).reduce((a,p)=> (a[p.id] = p.suggestedRole, a), {}),
    links:      D.externalLinks.autoDiscovered.reduce((a,l)=> (a[l.id] = true, a), {}),
    metadata:   D.discoveredSolutions.reduce((a,s)=> (a[s.id] = { status: "active", client: "", goal: "" }, a), {}),
    libraries:  (D.discoveredLibraries ? D.discoveredLibraries.detected : []).reduce((a,l)=> (a[l.id] = true, a), {}),
    libExtras:  [],   // user-added: {id, name, url, lang}
    mcps:       (D.mcpRegistry ? D.mcpRegistry.available : [])
                  .reduce((a,m)=> (a[m.id] = !!(m.installed || m.recommended), a), {}),
    models:     (D.inference ? D.inference.localModels : [])
                  .reduce((a,m)=> (a[m.id] = !!m.recommended || !!m.pulled, a), {}),
    apiKeys:    (D.inference ? D.inference.providers : [])
                  .reduce((a,p)=> (a[p.id] = "", a), {}),

    // Preferences — derived from $HOME on first read, editable in the
    // Preferences stage and persisted via writeSenseiSettings(). The defaults
    // here mirror SETTINGS_DEFAULTS but live independently so the wizard can
    // be re-entered to tweak them without touching anything else.
    prefs: (() => {
      const seeded = (typeof readSenseiSettings === "function")
        ? readSenseiSettings()
        : {};
      // Pull what looks like a username out of $HOME ("/Users/keiko" → "keiko").
      const homeUser = (D.system && D.system.homeDir
                          ? D.system.homeDir.split("/").filter(Boolean).pop()
                          : "you");
      const niceName = homeUser
        ? homeUser.charAt(0).toUpperCase() + homeUser.slice(1)
        : "you";
      return {
        displayName:             seeded.displayName             ?? niceName,
        homeDir:                 D.system?.homeDir              ?? "~",
        contributeLearnings:     seeded.contributeLearnings     ?? true,
        reviewBeforeShare:       seeded.reviewBeforeShare       ?? true,
        shareSchedule:           seeded.shareSchedule           ?? "weekly-saturday",
        downloadCollective:      seeded.downloadCollective      ?? "weekly",
        correctionAggressiveness:seeded.correctionAggressiveness?? "balanced",
        digestCadence:           seeded.digestCadence           ?? "daily",
        nudgeOnRegression:       seeded.nudgeOnRegression       ?? true,
        anonymizedTelemetry:     seeded.anonymizedTelemetry     ?? false,
        showWelcome:             seeded.showWelcome             ?? true,
      };
    })(),
  });
  const upd = (patch) => setState(prev => ({ ...prev, ...patch }));

  const next = () => setStageIdx(i => Math.min(i + 1, stages.length - 1));
  const back = () => setStageIdx(i => Math.max(i - 1, 0));
  const saveAndClose = () => { writeSenseiSettings(state.prefs || {}); onDone && onDone(); };

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label={isPrefs ? "Preferences" : "Setup Wizard"}
 >
      <TauriChrome title={isPrefs ? "Sensei  先生  ·  preferences" : "Sensei  先生  ·  setup"}/>
      <div className="flex-1 grid min-h-0" style={{ gridTemplateColumns: '260px 1fr' }}>
        <WizRail stages={stages} stageIdx={stageIdx} setStageIdx={setStageIdx} onExit={onExit}
                 freeNav={isPrefs} railTitle={isPrefs ? "Preferences" : "Setup"}/>
        <div className="flex flex-col min-h-0" >
          <div className="pt-12 pb-8 px-16 flex-1 overflow-auto" >
            {stage.id === "welcome"    && <WizWelcome/>}
            {stage.id === "components" && <WizComponents state={state} upd={upd}/>}
            {stage.id === "acps"       && <WizAcps state={state} upd={upd}/>}
            {stage.id === "folders"    && <WizFolders state={state} upd={upd}/>}
            {stage.id === "scan"       && <WizScan state={state} upd={upd} context={isPrefs ? "preferences" : "setup"}/>}
            {stage.id === "projects"   && <WizProjects state={state} upd={upd}/>}
            {stage.id === "libraries"  && <WizLibraries state={state} upd={upd}/>}
            {stage.id === "registry"   && <WizRegistry state={state} upd={upd}/>}
            {stage.id === "inference"   && <WizInference state={state} upd={upd}/>}
            {stage.id === "assignments" && <WizAssignments state={state} upd={upd}/>}
            {stage.id === "preferences" && <WizPreferences state={state} upd={upd}/>}
            {stage.id === "done"        && <WizDone state={state}/>}
          </div>
          <WizBottom stage={stage} stageIdx={stageIdx} total={stages.length}
                     back={back} next={next} mode={mode}
                     onSaveClose={saveAndClose}
                     onDone={saveAndClose}
                     state={state}/>
        </div>
      </div>
    </div>
  );
}

// ─── Services status (all green / error) ────────────────────
function ServicesStatus() {
  // Pull live-ish values from the bootstrap model if present; otherwise all green.
  const statuses = (window.BOOT_PRESETS && window.BOOT_PRESETS["all-green"].statuses) || {};
  const services = [
    { id: "postgres", label: "postgres" },
    { id: "ollama",   label: "ollama"   },
    { id: "daemon",   label: "daemon"   },
  ];
  const anyBad = services.some(s => {
    const v = statuses[s.id];
    return v && v !== "ready";
  });

  return (
    <div className="gap-2 flex items-center" >
      <span style={{ width: 7, height: 7, borderRadius: 4,
                      background: anyBad ? 'var(--accent)' : 'var(--success)' }}/>
      <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.4 }}>
        <div className="uppercase text-ink-3" style={{ letterSpacing: '0.1em',
 fontSize: 11 }}>
          Services
        </div>
        <div className="mt-1" >
          {anyBad ? "one or more down" : "all green"}
        </div>
      </div>
    </div>
  );
}

// ─── Left rail ───────────────────────────────────────────────
function WizRail({ stages, stageIdx, setStageIdx, onExit, freeNav = false, railTitle = "Setup" }) {
  const [q, setQ] = useS("");
  const ql = q.trim().toLowerCase();
  // Keyword index so a search hits the setting inside a stage, not just its title.
  const KW = {
    scan: "folders repos rescan index roots",
    folders: "folders paths exclude roots watch",
    projects: "projects repos merge split roles",
    acps: "assistants claude cursor copilot mcp connect",
    libraries: "libraries wrap docs dependencies",
    registry: "instruments mcp tools playground",
    inference: "routers inference models ollama embedded cloud routing moe deliberation",
    assignments: "assignments roles model which",
    preferences: "profile name tone digest telemetry privacy regression",
  };
  const REVIEW = new Set(["preferences", "inference"]);  // what most users change first
  const matches = (s) => !ql || s.title.toLowerCase().includes(ql)
    || (s.sub || "").toLowerCase().includes(ql) || (KW[s.id] || "").includes(ql);
  const anyVisible = stages.some(s => matches(s));
  return (
    <aside className="py-6 px-6 border-r flex flex-col bg-paper-2 overflow-hidden" >
      <div className="gap-2 mb-6 flex items-center" >
        <Wordmark size={24}/>
        <span className="flex-1" />
        <button className="text-ink-3" onClick={onExit} title="Exit setup"
 style={{ fontSize: 11, letterSpacing: '0.1em' }}>
          ESC
        </button>
      </div>

      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                    textTransform: 'uppercase'
}} className={freeNav ? "mb-1" : "mb-3"} >{railTitle}</div>

      {freeNav && (
        <>
          <div style={{ fontSize: 11, lineHeight: 1.45 }} className="mb-3 text-ink-4" >
            Everything Sensei can be told — search or browse.
          </div>
          <div style={{ gap: 6, borderRadius: 6, padding: '8px 8px' }} className="mb-3 flex items-center bg-paper border border-paper-edge" >
            <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
            <input className="border-0 bg-transparent flex-1 text-ink" value={q} onChange={e => setQ(e.target.value)}
 placeholder="search settings…"
 style={{ outline: 'none',
 fontSize: 12 }}/>
            {q && <button className="text-ink-4" onClick={() => setQ("")} style={{ fontSize: 12 }}>×</button>}
          </div>
        </>
      )}

      <div className="gap-1 flex flex-col" >
        {stages.map((s, i) => {
          if (freeNav && !matches(s)) return null;
          const isCur = i === stageIdx;
          const isDone = i < stageIdx;
          const locked = !freeNav && i > stageIdx;
          return (
            <button key={s.id}
 onClick={() => setStageIdx(i)}
 disabled={locked}
 style={{
 gridTemplateColumns: '24px 1fr 14px',
 padding: isCur ? '10px 10px' : '7px 10px',
 borderRadius: 6,
 background: isCur ? 'var(--paper)' : 'transparent',
 border: isCur ? 'var(--hairline)' : '1px solid transparent',
 color: isCur ? 'var(--ink)' : isDone ? 'var(--ink-2)' : 'var(--ink-4)',
 cursor: locked ? 'default' : 'pointer',
 transition: 'all .14s'
 }} className="gap-2 grid items-center text-left" >
              {/* Always show the stage's kanji label so re-entering the
                  wizard reads as the same stepper, not a column of ✓s. */}
              <span className="kanji text-center" style={{
 fontSize: 13,
 color: isCur ? 'var(--accent)'
 : isDone ? 'var(--ink-2)'
 : 'var(--ink-4)'
 }}>{s.n}</span>
              <div className="overflow-hidden" >
                <div className="flex items-center" style={{ gap: 6 }}>
                  <span style={{ fontSize: 13 }}>{s.title}</span>
                  {freeNav && !ql && REVIEW.has(s.id) && (
                    <span className="mono uppercase text-accent bg-accent-soft" style={{ fontSize: 9, letterSpacing: '0.06em', borderRadius: 10, padding: '4px 8px' }}>review</span>
                  )}
                </div>
                {isCur && (
                  <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                    {s.sub}
                  </div>
                )}
              </div>
              {/* Completion tick lives on the right rail — kanji stays the
                  identity; tick is the status. */}
              <span className="text-center text-success" style={{
 fontSize: 11, lineHeight: 1,
 opacity: isDone ? 1 : 0,
 transition: 'opacity .14s'
 }}>✓</span>
            </button>
          );
        })}
      </div>

      {freeNav && ql && !anyVisible && (
        <div style={{ fontSize: 12 }} className="py-3 px-2 text-ink-4 italic" >
          No settings match “{q}”.
        </div>
      )}

      <div className="flex-1" />

      <div className="pt-3 border-t" >
        <ServicesStatus/>
      </div>
    </aside>
  );
}

// ─── Bottom bar ──────────────────────────────────────────────
function WizBottom({ stage, stageIdx, total, back, next, onDone, onSaveClose, mode = "setup", state }) {
  const isLast = stageIdx === total - 1;
  const isFirst = stageIdx === 0;
  const isPrefs = mode === "preferences";
  const canAdvance = (() => {
    if (isPrefs) return true;   // settings: jump freely, no gates
    if (stage.id === "folders") return state.folders.length > 0;
    if (stage.id === "scan") return state.scan.done;
    if (stage.id === "preferences") {
      // Don't let the user step in nameless — sensei has to call you something.
      return !!(state.prefs && state.prefs.displayName && state.prefs.displayName.trim());
    }
    return true;
  })();

  return (
    <div className="gap-4 py-3 px-16 border-t flex items-center bg-paper" >
      <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.12em' }}>
        {String(stageIdx + 1).padStart(2, "0")} <span className="text-ink-4" >/ {total}</span>
        <span style={{
 letterSpacing: 0, fontSize: 13
 }} className="ml-3 text-ink-2 normal-case" >{stage.title}</span>
      </div>

      {/* progress ticks */}
      <div className="gap-1 flex-1 flex items-center" >
        {Array.from({ length: total }).map((_, i) => (
          <span className="flex-1" key={i} style={{ height: 2, borderRadius: 1,
 background: i <= stageIdx ? 'var(--ink)' : 'var(--edge)',
 transition: 'background .2s'
 }}/>
        ))}
      </div>

      <button onClick={back} disabled={isFirst}
              style={{
 fontSize: 13, color: isFirst ? 'var(--ink-4)' : 'var(--ink-2)'
}} className="py-2 px-3" >
        ← Back
      </button>

      {isPrefs ? (
        <>
          {!isLast && (
            <button onClick={next}
 style={{ fontSize: 13,
 border: 'var(--ink-line)', borderRadius: 6 }}
 className="py-2 px-4 text-ink-2" >
              Next →
            </button>
          )}
          <button onClick={onSaveClose}
 style={{
 fontSize: 13, borderRadius: 6, letterSpacing: 0.2
 }} className="py-2 px-6 bg-ink text-paper" >
            Save &amp; close
          </button>
        </>
      ) : isLast ? (
        <button onClick={onDone}
 style={{
 fontSize: 13, borderRadius: 6, letterSpacing: 0.2
 }} className="py-2 px-6 bg-ink text-paper" >
          Enter observatory →
        </button>
      ) : (
        <button onClick={next} disabled={!canAdvance}
                style={{
 fontSize: 13,
                         background: canAdvance ? 'var(--ink)' : 'var(--edge)',
                         color: canAdvance ? 'var(--paper)' : 'var(--ink-3)', borderRadius: 6, letterSpacing: 0.2
}} className="py-2 px-6" >
          Continue →
        </button>
      )}
    </div>
  );
}

// ─── 1 Welcome ───────────────────────────────────────────────
function WizWelcome() {
  return (
    <div style={{ maxWidth: 680 }} className="mt-2 mb-0 mx-auto" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-2 text-ink-3 uppercase" >礼 · Welcome</div>
      <h1 className="display mt-0 mb-8 font-light" style={{
 fontSize: 56, lineHeight: 1.08, letterSpacing: '-0.02em'
 }}>
        A teacher does not<br/>
        <span className="text-accent" >write the code.</span>
      </h1>

      <p style={{
 fontSize: 15, lineHeight: 1.7, maxWidth: 560
 }} className="mt-0 mb-6 text-ink-2" >
        Sensei watches how you and your AI assistants work together — the sessions that
        completed cleanly, the ones that didn't, and the patterns underneath both.
      </p>

      <p style={{
 fontSize: 13, lineHeight: 1.7, maxWidth: 560
 }} className="mt-0 mb-12 text-ink-3" >
        The next few minutes: install the local components, point to your folders, confirm
        what was found. Nothing leaves your machine.
      </p>

      <div style={{ gridTemplateColumns: '1fr 1fr 1fr' }} className="gap-4 py-6 px-0 grid border-t border-b" >
        {[
          { k: "観", t: "Observe", s: "FTR · turns · corrections" },
          { k: "師", t: "Teach",   s: "patterns · rules · skills" },
          { k: "静", t: "Local",   s: "on your machine" }
        ].map(item => (
          <div key={item.k}>
            <div className="kanji mb-2 text-accent" style={{ fontSize: 28 }}>{item.k}</div>
            <div className="display" style={{ fontSize: 22 }}>{item.t}</div>
            <div style={{ fontSize: 13 }} className="mt-1 text-ink-3" >{item.s}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ─── 2 Components ────────────────────────────────────────────
// Auto-resolves: detects state, transitions all components to installed/ready.
function WizComponents({ state, upd }) {
  const D = window.SENSEI_SETUP;
  const variant = D.componentsVariants.find(v => v.id === state.components.variant);
  // phases[id] = "detecting" | "installing" | "starting" | "ready"
  const [phases, setPhases] = useS(() =>
    variant.components.reduce((a, c) => (a[c.id] = c.status === "installed" ? "ready" : "detecting", a), {})
  );

  // Animate each component through its phase
  useE(() => {
    variant.components.forEach((c, i) => {
      const current = phases[c.id];
      if (current === "ready") return;
      const targetPhase =
        c.status === "installed" ? "ready" :
        c.status === "missing"   ? "installing" :
        c.status === "stopped"   ? "starting" : "ready";
      const dur = c.status === "missing" ? 1400 : 800;
      const kickoff = 400 + i * 350;
      const t1 = setTimeout(() => setPhases(p => ({ ...p, [c.id]: targetPhase })), kickoff);
      const t2 = setTimeout(() => setPhases(p => ({ ...p, [c.id]: "ready" })), kickoff + dur);
      return () => { clearTimeout(t1); clearTimeout(t2); };
    });
  }, [state.components.variant]);

  const overallReady = Object.values(phases).every(p => p === "ready");

  return (
    <div style={{ maxWidth: 820 }} className="mx-auto" >
      <WizHeader n="二" title="Components"
                 tagline={overallReady ? "Everything is in place." : "Detecting, installing, starting. No input needed."}/>

      {/* Variant toggle kept only as a subtle demo aid */}
      <div className="gap-2 mb-6 flex items-center" >
        <span className="uppercase text-ink-4" style={{ fontSize: 11, letterSpacing: '0.14em' }}>demo · starting state</span>
        <div style={{
 borderRadius: 5 }} className="p-1 gap-0 flex bg-paper-2 border border-paper-edge" >
          {D.componentsVariants.map(v => (
            <button key={v.id}
 onClick={() => {
 upd({ components: { variant: v.id, acting: {} }});
 const nv = D.componentsVariants.find(x => x.id === v.id);
 setPhases(nv.components.reduce((a, c) =>
 (a[c.id] = c.status === "installed" ? "ready" : "detecting", a), {}));
 }}
 style={{
 fontSize: 11, borderRadius: 3,
 background: state.components.variant === v.id ? 'var(--paper)' : 'transparent',
 color: state.components.variant === v.id ? 'var(--ink-2)' : 'var(--ink-3)' }} className="py-1 px-2 border-0" >
              {v.label}
            </button>
          ))}
        </div>
      </div>

      <div className="gap-2 flex flex-col" >
        {variant.components.map(c => {
          const phase = phases[c.id] || "detecting";
          const isBusy = phase === "installing" || phase === "starting";
          const statusLabel =
            phase === "detecting"  ? "checking…" :
            phase === "installing" ? "installing · 12.4 MB" :
            phase === "starting"   ? "starting…" : `${c.version || "0.9.3"} · ready`;
          const dotColor =
            phase === "ready" ? 'var(--success)' : 'var(--accent)';

          return (
            <div key={c.id} style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 8,
 transition: 'all .3s'
 }} className="gap-4 py-4 px-4 grid border border-paper-edge bg-paper-2 items-center" >
              <div className="bg-paper border border-paper-edge flex items-center justify-center" style={{ width: 36, height: 36, borderRadius: 6 }}>
                <span className="mono text-ink-2" style={{ fontSize: 13 }}>
                  {c.id === "cli" ? "$" : c.id === "mcp" ? "⟷" : "◇"}
                </span>
              </div>
              <div>
                <div style={{ fontSize: 13 }}>{c.name}</div>
                <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                  {statusLabel}
                </div>
                {isBusy && (
                  <div style={{
 height: 2, background: 'var(--edge)', borderRadius: 1 }} className="mt-2 overflow-hidden" >
                    <div className="h-full bg-accent" style={{ width: '40%',
 animation: 'cSlide 1.2s ease-in-out infinite'
 }}/>
                  </div>
                )}
              </div>
              <div className="gap-2 flex items-center" >
                <span className="rounded-full" style={{
 width: 8, height: 8, background: dotColor,
 boxShadow: phase !== "ready" ? `0 0 0 4px ${dotColor}22` : 'none',
 animation: phase !== "ready" ? "cPulse 1.2s ease-in-out infinite" : 'none'
 }}/>
                <span className="mono uppercase" style={{ fontSize: 11,
 color: phase === "ready" ? 'var(--success)' : 'var(--ink-2)',
 letterSpacing: '0.06em' }}>
                  {phase === "ready" ? "ready" : phase}
                </span>
              </div>
            </div>
          );
        })}
        <style>{`
          @keyframes cPulse { 0%,100% { opacity: 1 } 50% { opacity: 0.4 } }
          @keyframes cSlide { 0% { transform: translateX(-50%) } 100% { transform: translateX(250%) } }
        `}</style>
      </div>

      <p style={{ fontSize: 13, lineHeight: 1.7 }} className="mt-4 text-ink-3" >
        Nothing leaves <span className="mono">localhost:9823</span>.
      </p>
    </div>
  );
}

function StatusDot({ status }) {
  const map = {
    installed: { c: 'var(--success)',  l: "Ready" },
    missing:   { c: 'var(--ink-4)', l: "Missing" },
    stopped:   { c: 'var(--warning)', l: "Stopped" },
    working:   { c: 'var(--accent)',   l: "Working" }
  };
  const m = map[status] || map.missing;
  return (
    <div className="gap-2 flex items-center" >
      <span className="rounded-full" style={{
 width: 8, height: 8, background: m.c,
 boxShadow: status === "working" ? `0 0 0 4px ${m.c}22` : 'none',
 animation: status === "working" ? "pulse 1.2s ease-in-out infinite" : 'none'
 }}/>
      <span className="mono text-ink-2" style={{ fontSize: 11 }}>{m.l}</span>
      <style>{`@keyframes pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.45 } }`}</style>
    </div>
  );
}

// ─── 3 ACPs ──────────────────────────────────────────────────
// Grouped by vendor family. A family card shows once even if it has
// multiple products (e.g. Claude Code + Claude Desktop). Toggling the
// card connects every found product in that family at once. Each
// product surfaces as a small chip with its own path on hover.
function WizAcps({ state, upd }) {
  const D = window.SENSEI_SETUP;

  // Per-part configure lifecycle (local to this stage). The wizard's
  // state.acps[id] stays the all-or-nothing enabled flag (the Done step
  // counts it); the spinner→✓/✗ progress lives here.
  //   life[familyId][partId] = "idle" | "configuring" | "done" | "error"
  const FLAKY = { "claude/agents": "~/.claude/agents — permission denied" };
  const partsOf = (f) => f.parts || [{ id: "mcp", label: "mcp server" }];

  const [life, setLife] = useS(() => {
    const o = {};
    for (const f of D.acps) {
      o[f.id] = {};
      const on = !!state.acps[f.id];
      for (const c of partsOf(f)) o[f.id][c.id] = (f.found && on) ? "done" : "idle";
    }
    return o;
  });
  const [errs, setErrs] = useS({});
  const timers = useR({});
  const attempts = useR({});

  const resolvePart = (fid, cid, delay) => {
    const key = fid + "/" + cid;
    clearTimeout(timers.current[key]);
    timers.current[key] = setTimeout(() => {
      const n = attempts.current[key] || 0;
      const willFail = !!FLAKY[key] && n === 0;
      attempts.current[key] = n + 1;
      setLife(p => ({ ...p, [fid]: { ...p[fid], [cid]: willFail ? "error" : "done" } }));
      setErrs(p => ({ ...p, [fid]: { ...(p[fid] || {}), [cid]: willFail ? FLAKY[key] : null } }));
    }, delay);
  };

  const runFamily = (fid) => {
    const f = D.acps.find(x => x.id === fid);
    setLife(p => {
      const caps = {};
      for (const c of partsOf(f)) caps[c.id] = "configuring";
      return { ...p, [fid]: caps };
    });
    setErrs(p => ({ ...p, [fid]: {} }));
    partsOf(f).forEach((c, i) => resolvePart(fid, c.id, 600 + i * 280 + Math.random() * 250));
  };

  const toggle = (fid) => {
    const f = D.acps.find(x => x.id === fid);
    if (!f.found) return;
    if (!state.acps[fid]) { upd({ acps: { ...state.acps, [fid]: true } }); runFamily(fid); }
    else {
      upd({ acps: { ...state.acps, [fid]: false } });
      setLife(p => {
        const caps = {};
        for (const c of partsOf(f)) caps[c.id] = "idle";
        return { ...p, [fid]: caps };
      });
      setErrs(p => ({ ...p, [fid]: {} }));
    }
  };

  const retry = (fid) => {
    const f = D.acps.find(x => x.id === fid);
    const failed = partsOf(f).filter(c => life[fid] && life[fid][c.id] === "error");
    setLife(p => {
      const caps = { ...p[fid] };
      failed.forEach(c => { caps[c.id] = "configuring"; });
      return { ...p, [fid]: caps };
    });
    failed.forEach((c, i) => resolvePart(fid, c.id, 500 + i * 250));
  };

  return (
    <div style={{ maxWidth: 760 }} className="mx-auto" >
      <WizHeader n="連" title="Assistants" tagline="Connect the AI tools you already use. Each registers every part it supports — plugins, skills, commands, agents, or an MCP server."/>

      <div className="grid grid-cols-2 gap-3">
        {D.acps.map(family => {
          const enabled = !!state.acps[family.id];
          const fl = life[family.id] || {};
          const parts = partsOf(family).map(c => ({
            id: c.id, label: c.label, status: enabled ? (fl[c.id] || "idle") : "idle",
          }));
          const firstErr = partsOf(family).find(c => fl[c.id] === "error");
          return (
            <AssistantCard key={family.id}
              name={family.name} logoId={family.id}
              found={family.found} enabled={enabled} parts={parts}
              error={enabled && firstErr ? (errs[family.id] || {})[firstErr.id] : null}
              onToggle={() => toggle(family.id)}
              onRetry={() => retry(family.id)}/>
          );
        })}
      </div>

      <div className="zs-body-sm text-ink-mute mt-4">
        Each switch is all-or-nothing — it registers every part the assistant supports, or none. You can fine-tune individual parts later from <span className="mono">Settings → Assistants</span>.
      </div>
    </div>
  );
}

// ─── 4 Folders ───────────────────────────────────────────────
function WizFolders({ state, upd }) {
  const add = () => {
    if (!state.newFolder.trim()) return;
    upd({ folders: [...state.folders, { id: "n" + Date.now(), path: state.newFolder.trim(), note: "added" }], newFolder: "" });
  };
  const remove = (id) => upd({ folders: state.folders.filter(f => f.id !== id) });

  return (
    <div style={{ maxWidth: 820 }} className="mx-auto" >
      <WizHeader n="庵" title="Folders" tagline="Where your work lives. Sensei recurses and finds repos."/>

      <div className="gap-2 mb-4 flex" >
        <input value={state.newFolder}
 onChange={e => upd({ newFolder: e.target.value })}
 onKeyDown={e => e.key === "Enter" && add()}
 placeholder="~/code/my-project"
 className="mono py-2 px-3 flex-1 bg-paper-2 border border-paper-edge"
 style={{ fontSize: 13, borderRadius: 6,
 outline: 'none'
 }}/>
        <button onClick={add}
 style={{
 fontSize: 13, borderRadius: 6 }} className="py-2 px-4 bg-ink text-paper" >
          Add
        </button>
        <button style={{
 fontSize: 13, borderRadius: 6,
                          border: 'var(--ink-line)'
}} className="py-2 px-4" >
          Browse…
        </button>
      </div>

      <div className="gap-1 flex flex-col" >
        {state.folders.map(f => (
          <div key={f.id} style={{ gridTemplateColumns: 'auto 1fr auto auto', borderRadius: 6 }} className="gap-3 py-3 px-4 grid border border-paper-edge bg-paper-2 items-center" >
            <span className="text-ink-3" style={{ fontSize: 13 }}>▸</span>
            <div>
              <div className="mono" style={{ fontSize: 13 }}>{f.path}</div>
              <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{f.note}</div>
            </div>
            <span className="mono py-1 px-2 text-ink-3 bg-paper border border-paper-edge" style={{
 fontSize: 11,
 borderRadius: 3 }}>recursive</span>
            <button onClick={() => remove(f.id)}
 style={{ fontSize: 11 }} className="py-1 px-2 text-ink-3" >
              remove
            </button>
          </div>
        ))}
      </div>

      <div style={{ fontSize: 13, lineHeight: 1.7 }} className="mt-4 text-ink-3" >
        You can manage folders and exclusions later from <span className="mono">Settings</span>.
      </div>
    </div>
  );
}

// ─── 5 Scan (SSE-style live view) ────────────────────────────
// context: "first-run" (empty start → must add a root) · "preferences"
// (pre-seeded roots → "Re-scan") · "setup" (legacy linear). The not-started
// state is a folder-input surface; the running/done views are identical
// across contexts.
function WizScan({ state, upd, context = "setup", onComplete }) {
  const D = window.SENSEI_SETUP;
  const [tick, setTick] = useS(0);
  const [dragActive, setDragActive] = useS(false);
  const fileInputRef = useR(null);
  const { started, done } = state.scan;

  // Add a root folder (deduped). Demo-friendly: accepts a path string.
  const addRoot = (raw) => {
    const path = (raw || "").trim();
    if (!path || state.folders.some(f => f.path === path)) { upd({ newFolder: "" }); return; }
    upd({ folders: [...state.folders, { id: "n" + Date.now(), path, note: "added" }], newFolder: "" });
  };
  const removeRoot = (id) => upd({ folders: state.folders.filter(f => f.id !== id) });

  // Drag/drop — pull a folder name from the drop if the browser exposes one,
  // otherwise fall back to a representative root so the gesture still lands.
  const DEMO_ROOTS = ["~/code", "~/work", "~/projects", "~/src"];
  const onDrop = (e) => {
    e.preventDefault(); setDragActive(false);
    let name = "";
    try {
      const it = e.dataTransfer && e.dataTransfer.items && e.dataTransfer.items[0];
      const entry = it && it.webkitGetAsEntry && it.webkitGetAsEntry();
      if (entry && entry.name) name = entry.name;
      else if (e.dataTransfer.files && e.dataTransfer.files[0]) name = e.dataTransfer.files[0].name;
    } catch {}
    addRoot(name ? `~/${name}`
                 : (DEMO_ROOTS.find(r => !state.folders.some(f => f.path === r)) || `~/code-${state.folders.length + 1}`));
  };
  const onBrowse = (e) => {
    const files = e.target.files;
    let name = "";
    if (files && files.length) {
      const rel = files[0].webkitRelativePath || files[0].name;
      name = rel.split("/")[0];
    }
    if (name) addRoot(`~/${name}`);
    e.target.value = "";
  };

  useE(() => {
    if (!started || done) return;
    const iv = setInterval(() => setTick(t => t + 40), 40);
    return () => clearInterval(iv);
  }, [started, done]);

  useE(() => {
    if (!started) return;
    const lastT = D.scanEvents[D.scanEvents.length - 1].t;
    if (tick >= lastT + 200 && !done) {
      upd({ scan: { ...state.scan, done: true } });
    }
  }, [tick, started]);

  const events = D.scanEvents.filter(e => e.t <= tick);
  const lastEvent = events[events.length - 1];
  const progress = Math.min(1, tick / 1260);
  const stats = {
    discovered: events.filter(e => e.level === "discover").length,
    queued:     events.filter(e => e.level === "queue").length,
    processed:  events.filter(e => e.level === "process").length
  };

  const start = () => {
    upd({ scan: { started: true, done: false, tick: 0 } });
    setTick(0);
  };

  if (!started) {
    const hasRoots = state.folders.length > 0;
    const isPrefs = context === "preferences";
    const isFirstRun = context === "first-run";
    // First-run and Preferences carry distinct header copy — the old separate
    // welcome strip / footer message now live here in the step header itself.
    const hdr = isFirstRun
      ? { eyebrow: "Welcome · one step",
          title: "Welcome to Sensei",
          tagline: "Point sensei at the folders your code lives in — the one step to begin. Everything else (libraries, instruments, models, your profile) starts on sensible defaults you can change anytime in Preferences." }
      : isPrefs
      ? { eyebrow: "Preferences",
          title: "Scan",
          tagline: hasRoots
            ? "Re-scan to pick up new, moved, or renamed repositories."
            : "Point sensei at where your code lives." }
      : { eyebrow: "Step",
          title: "Scan",
          tagline: hasRoots
            ? `Ready to scan ${state.folders.length} ${state.folders.length === 1 ? "root" : "roots"}.`
            : "Point sensei at where your code lives." };
    return (
      <div style={{ maxWidth: 720 }} className="mx-auto" >
        <WizHeader n="観" eyebrow={hdr.eyebrow} title={hdr.title} tagline={hdr.tagline}/>

        {/* What sensei does with these folders — so the ask is clear up front */}
        <div style={{ gridTemplateColumns: '1fr 1fr 1fr', borderRadius: 8 }} className="mb-4 grid border border-paper-edge bg-paper-2 overflow-hidden" >
          {[
            { k: "探", t: "Recurses", s: "walks every subfolder to find each repo" },
            { k: "図", t: "Maps",     s: "extracts the code graph — files, symbols, docs" },
            { k: "観", t: "Watches",  s: "observes future sessions in these folders" },
          ].map((x, i) => (
            <div key={x.k} style={{ borderRight: i < 2 ? 'var(--hairline)' : 'none' }} className="py-3 px-4" >
              <div className="gap-2 flex items-baseline" >
                <span className="kanji text-accent" style={{ fontSize: 15 }}>{x.k}</span>
                <span className="text-ink font-medium" style={{ fontSize: 13 }}>{x.t}</span>
              </div>
              <div style={{ fontSize: 11, lineHeight: 1.45 }} className="mt-1 text-ink-3" >{x.s}</div>
            </div>
          ))}
        </div>

        {/* Dropzone — doubles as the quiet empty room until a root is added */}
        <div
          onDragOver={e => { e.preventDefault(); setDragActive(true); }}
          onDragLeave={e => { e.preventDefault(); setDragActive(false); }}
          onDrop={onDrop}
          style={{
            border: `1.5px dashed ${dragActive ? 'var(--accent)' : 'var(--edge)'}`,
            background: dragActive ? 'var(--accent-soft)' : 'var(--paper-2)',
            borderRadius: 10, textAlign: 'center', transition: 'all .15s'
          }} className={hasRoots ? "py-6 px-8" : "py-16 px-8"} >
          <div className="kanji text-accent" style={{ fontSize: hasRoots ? 30 : 52,
 opacity: dragActive ? 0.75 : 0.4, lineHeight: 1 }} >庵</div>
          <div style={{ fontSize: 15 }} className="mt-3 text-ink font-medium" >
            {dragActive ? "Drop to add this folder" : "Drag a code folder here"}
          </div>
          <div style={{ fontSize: 13, lineHeight: 1.6, maxWidth: 440 }}
 className="mt-1 mx-auto text-ink-3" >
            Add the top-level folders your projects live in — like <span className="mono">~/code</span> or{" "}
            <span className="mono">~/work</span>. Not individual repos; sensei recurses to find them.
          </div>
          <div 
 className="gap-2 mt-4 flex items-center justify-center" >
            <button onClick={() => fileInputRef.current && fileInputRef.current.click()}
 style={{ fontSize: 13,
 borderRadius: 6 }} className="py-2 px-4 bg-ink text-paper" >
              Browse…
            </button>
            <span className="text-ink-4" style={{ fontSize: 11 }}>or</span>
            <div style={{ borderRadius: 6 }} className="gap-1 pl-2 pr-1 py-1 flex items-center bg-paper border border-paper-edge" >
              <input value={state.newFolder}
 onChange={e => upd({ newFolder: e.target.value })}
 onKeyDown={e => e.key === "Enter" && addRoot(state.newFolder)}
 placeholder="paste a path…"
 className="mono border-0 bg-transparent text-ink"
 style={{ outline: 'none',
 fontSize: 12, width: 156 }}/>
              <button onClick={() => addRoot(state.newFolder)}
 style={{ fontSize: 11,
 borderRadius: 4 }} className="py-1 px-2 text-ink-2 border border-paper-edge" >add</button>
            </div>
          </div>
          <input className="hidden" ref={fileInputRef} type="file" webkitdirectory="" directory=""
 onChange={onBrowse} />
        </div>

        {/* Added roots */}
        {hasRoots && (
          <div className="gap-1 mt-4 flex flex-col" >
            {state.folders.map(f => (
              <div key={f.id} style={{ gridTemplateColumns: 'auto 1fr auto auto', borderRadius: 6 }} className="gap-3 py-2 px-3 grid items-center border border-paper-edge bg-paper-2" >
                <span className="kanji text-accent" style={{ fontSize: 13 }}>庵</span>
                <span className="mono text-ink" style={{ fontSize: 13 }}>{f.path}</span>
                <span className="mono py-1 px-2 text-ink-3 bg-paper border border-paper-edge" style={{ fontSize: 11, borderRadius: 3 }}>recursive</span>
                <button onClick={() => removeRoot(f.id)}
 style={{ fontSize: 11 }} className="py-1 px-2 text-ink-4" >remove</button>
              </div>
            ))}
          </div>
        )}

        {/* Begin — gated on at least one root */}
        <div className="gap-3 mt-6 flex items-center" >
          <button onClick={start} disabled={!hasRoots}
                  style={{ fontSize: 13,
                           background: hasRoots ? 'var(--ink)' : 'var(--edge)',
                           color: hasRoots ? 'var(--paper)' : 'var(--ink-4)',
                           borderRadius: 6, cursor: hasRoots ? 'pointer' : 'default' }}
                  className="py-3 px-6" >
            {isPrefs ? "Re-scan" : "Begin scan"} →
          </button>
          <span className="text-ink-3" style={{ fontSize: 11, lineHeight: 1.5 }}>
            {hasRoots
              ? `Two workers · ~2M files / minute on this machine.${isPrefs ? " Re-scanning picks up new repos." : ""}`
              : "Add at least one folder to begin."}
          </span>
        </div>

        {isFirstRun && (
          <div style={{ fontSize: 11, lineHeight: 1.6 }} className="mt-4 text-ink-3" >
            Scanning is entirely local — nothing leaves your machine.
          </div>
        )}
      </div>
    );
  }

  // Derive per-solution/project live state from the stream
  // Each discovered repo slug maps to the solution/project in discoveredSolutions.
  const repoState = {}; // repoId -> { state: "discovered"|"queued"|"processing"|"done", queued, processed, totalFiles }
  const solutionState = {}; // solutionId -> { state, discoveredAt }

  // Build lookup: phrase in event msg -> repo id
  const D2 = D.discoveredSolutions;
  const allRepos = D2.flatMap(s => s.projects.map(p => ({ ...p, solution: s.id })));

  events.forEach(e => {
    if (e.level === "discover") {
      // Check if a solution root was discovered (e.g. "~/code/lumen · found")
      D2.forEach(s => {
        if (e.msg.startsWith(s.path + " ·") || e.msg.startsWith(s.path + "/")) {
          solutionState[s.id] ??= { state: "discovered", discoveredAt: e.t };
        }
      });
      // Check if a specific repo was discovered
      allRepos.forEach(r => {
        if (e.msg.startsWith(r.path + " ·")) {
          repoState[r.id] = {
            state: "discovered", queued: 0, processed: 0,
            totalFiles: r.files, discoveredAt: e.t
          };
        }
      });
    }
    if (e.level === "queue") {
      allRepos.forEach(r => {
        if (e.msg.startsWith(r.name + " ·")) {
          repoState[r.id] = { ...(repoState[r.id] || {}), state: "queued",
                              queued: r.files, processed: 0, totalFiles: r.files };
        }
      });
    }
    if (e.level === "process") {
      allRepos.forEach(r => {
        if (e.msg.startsWith(r.name + " ·")) {
          const m = e.msg.match(/(\d+)\s*\/\s*(\d+)/);
          if (m) {
            const processed = parseInt(m[1]);
            const total = parseInt(m[2]);
            repoState[r.id] = { ...(repoState[r.id] || {}),
                                state: processed >= total ? "done" : "processing",
                                queued: total, processed, totalFiles: total };
          }
        }
      });
    }
  });

  // Mark solutions that have at least one repo as discovered; done if all repos done
  D2.forEach(s => {
    const rs = s.projects.map(p => repoState[p.id]).filter(Boolean);
    if (rs.length > 0) {
      solutionState[s.id] ??= { state: "discovered", discoveredAt: rs[0].discoveredAt };
      if (rs.every(r => r.state === "done") && rs.length === s.projects.length) {
        solutionState[s.id] = { ...solutionState[s.id], state: "done" };
      } else if (rs.some(r => r.state === "processing" || r.state === "queued")) {
        solutionState[s.id] = { ...solutionState[s.id], state: "active" };
      }
    }
  });

  const discoveredSolutions = D2.filter(s => solutionState[s.id]);

  return (
    <div style={{ maxWidth: 1000 }} className="mx-auto" >
      <WizHeader n="観" title={done ? "Scan complete" : "Scanning"}
                 tagline={done ? "The map is drawn." : "Workers recurse. Repos surface."}/>

      {/* Stats strip */}
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)', borderRadius: 8 }} className="mb-4 gap-0 grid border border-paper-edge bg-paper-2 overflow-hidden" >
        <ScanStat label="Roots"      value={state.folders.length}/>
        <ScanStat label="Discovered" value={stats.discovered}/>
        <ScanStat label="Queued"     value={stats.queued}/>
        <ScanStat label="Processed"  value={stats.processed} accent/>
      </div>

      {/* Progress line */}
      <div style={{
 height: 2, background: 'var(--edge)', borderRadius: 1 }} className="mb-6 overflow-hidden" >
        <div className="h-full" style={{ width: `${progress * 100}%`,
 background: done ? 'var(--success)' : 'var(--ink)',
 transition: 'width 80ms linear' }}/>
      </div>

      <div style={{ gridTemplateColumns: '1fr 320px' }} className="gap-4 grid items-start" >
        {/* ─── Left: solutions + repos materializing ─── */}
        <div style={{ minHeight: 360 }} className="gap-3 flex flex-col" >
          {discoveredSolutions.length === 0 && (
            <div style={{
 border: '1px dashed var(--edge)',
 borderRadius: 10,
 fontSize: 13 }} className="p-8 text-center text-ink-4 italic" >
              <div className="kanji mb-2 text-accent" style={{
 fontSize: 40,
 opacity: 0.3
 }}>待</div>
              listening…
            </div>
          )}
          {discoveredSolutions.map(s => (
            <ScanSolutionCard key={s.id} sol={s}
                              solState={solutionState[s.id]}
                              repoStates={s.projects.map(p => [p, repoState[p.id]])}/>
          ))}
        </div>

        {/* ─── Right: SSE event card ─── */}
        <div className="border border-paper-edge bg-paper-2 overflow-hidden sticky" style={{ borderRadius: 8, top: 0
 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="py-2 px-3 gap-2 border-b text-ink-3 uppercase flex items-center" >
            <span className="rounded-full" style={{
 width: 6, height: 6,
 background: done ? 'var(--success)' : 'var(--accent)',
 animation: done ? 'none' : 'pulseSm 1.2s ease-in-out infinite'
 }}/>
            <span>SSE · /events</span>
            <span className="flex-1" />
            <span className="mono normal-case" style={{ letterSpacing: 0 }}>
              {(tick/1000).toFixed(1)}s
            </span>
          </div>
          <div style={{ height: 360 }} className="py-2 px-3 overflow-auto" >
            {events.slice().reverse().map((e, i) => (
              <div key={e.t} style={{ gridTemplateColumns: '42px 60px 1fr', fontSize: 11,
 color: i === 0 ? 'var(--ink)' : 'var(--ink-2)',
 opacity: i === 0 ? 1 : Math.max(0.28, 1 - i * 0.07),
 animation: i === 0 ? 'eventIn .26s ease-out' : 'none'
 }} className="gap-2 py-1 px-0 grid" >
                <span className="mono text-ink-3" >+{(e.t/1000).toFixed(2)}s</span>
                <span className="mono" style={{
                  color: e.level === "success"  ? 'var(--success)' :
                         e.level === "discover" ? 'var(--accent)'  :
                         e.level === "process"  ? 'var(--ink)' :
                         e.level === "queue"    ? 'var(--warning)' : 'var(--ink-3)'
                }}>
                  {e.level}
                </span>
                <span className="mono overflow-hidden text-ellipsis whitespace-nowrap" >{e.msg}</span>
              </div>
            ))}
          </div>
          <style>{`
            @keyframes pulseSm { 0%,100% { opacity: 1 } 50% { opacity: 0.35 } }
            @keyframes eventIn { from { opacity: 0; transform: translateY(-4px) } to { opacity: 1 } }
            @keyframes cardIn  { from { opacity: 0; transform: translateY(6px) } to { opacity: 1 } }
            @keyframes repoIn  { from { opacity: 0; transform: translateX(-6px) } to { opacity: 1 } }
            @keyframes shimmer { 0% { background-position: -200px 0 } 100% { background-position: 200px 0 } }
          `}</style>
        </div>
      </div>

      {done && (
        <div style={{
 borderRadius: 8, fontSize: 13 }} className="mt-4 py-3 px-4 gap-3 bg-success-soft text-ink flex items-center" >
          <span className="kanji text-success" style={{ fontSize: 17 }}>✓</span>
          <span className="flex-1" style={{ lineHeight: 1.5 }}>
            8 repos indexed across {state.folders.length} {state.folders.length === 1 ? "root" : "roots"} · graph extracted
            {context === "first-run" ? " · your projects are ready." : " · you may continue."}
          </span>
          {context === "first-run" && onComplete && (
            <button onClick={onComplete}
 style={{ fontSize: 13,
 borderRadius: 6, letterSpacing: 0.2 }}
 className="py-2 px-4 bg-ink text-paper whitespace-nowrap" >
              Open {discoveredSolutions.length} projects →
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// A solution card that grows as its repos are discovered and processed
function ScanSolutionCard({ sol, solState, repoStates }) {
  const discoveredRepos = repoStates.filter(([p, rs]) => rs);
  const doneCount = discoveredRepos.filter(([p, rs]) => rs.state === "done").length;
  const allDone = solState.state === "done";
  const totalFiles = discoveredRepos.reduce((a, [,rs]) => a + (rs.totalFiles || 0), 0);
  const processedFiles = discoveredRepos.reduce((a, [,rs]) => a + (rs.processed || 0), 0);
  const overallPct = totalFiles > 0 ? processedFiles / totalFiles : 0;

  return (
    <div style={{
 border: allDone ? '1.5px solid var(--ink-2)' : 'var(--hairline)',
 borderRadius: 10,
 animation: 'cardIn .34s ease-out',
 transition: 'border .3s'
 }} className="py-4 px-4 bg-paper-2" >
      <div style={{ gridTemplateColumns: 'auto 1fr auto' }} className="gap-3 grid items-center" >
        <div className="kanji rounded-full bg-paper border border-paper-edge flex items-center justify-center" style={{
 fontSize: 22, width: 38, height: 38,
 color: allDone ? 'var(--accent)' : 'var(--ink-3)',
 transition: 'color .3s'
 }}>{sol.kanji}</div>
        <div className="min-w-0" >
          <div className="display" style={{ fontSize: 17 }}>{sol.name}</div>
          <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
            {sol.path} · {discoveredRepos.length} {discoveredRepos.length === 1 ? "repo" : "repos"}
            {!allDone && discoveredRepos.length > 0 && ` · ${doneCount} ready`}
          </div>
        </div>
        <div className="text-right" >
          <div className="mono uppercase" style={{ fontSize: 11,
 color: allDone ? 'var(--success)' : 'var(--ink-3)',
 letterSpacing: '0.1em' }}>
            {allDone ? "ready" : solState.state}
          </div>
          {totalFiles > 0 && !allDone && (
            <div className="mono mt-1 text-ink-2" style={{ fontSize: 11 }}>
              {processedFiles.toLocaleString()} / {totalFiles.toLocaleString()}
            </div>
          )}
        </div>
      </div>

      {/* Repos list */}
      {discoveredRepos.length > 0 && (
        <div className="mt-3 gap-1 pl-12 flex flex-col" >
          {discoveredRepos.map(([p, rs]) => {
            const pct = rs.totalFiles > 0 ? rs.processed / rs.totalFiles : 0;
            const isDone = rs.state === "done";
            const isProcessing = rs.state === "processing";
            return (
              <div key={p.id} style={{ gridTemplateColumns: '140px 1fr 70px',
 animation: 'repoIn .26s ease-out'
 }} className="gap-3 grid items-center" >
                <div className="gap-1 flex items-center" >
                  <span className="rounded-full" style={{
 width: 5, height: 5,
 background: isDone ? 'var(--success)' :
 isProcessing ? 'var(--accent)' :
 rs.state === "queued" ? 'var(--warning)' : 'var(--ink-4)',
 animation: isProcessing ? 'pulseSm 1.2s ease-in-out infinite' : 'none'
 }}/>
                  <span className="mono text-ink-2 overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11 }}>
                    {p.name}
                  </span>
                </div>
                {/* progress track */}
                <div className="overflow-hidden relative" style={{ height: 2, background: 'var(--edge)', borderRadius: 1 }}>
                  <div className="absolute" style={{ inset: 0, width: `${pct * 100}%`,
 background: isDone ? 'var(--success)' : 'var(--ink)',
 transition: 'width .3s ease-out'
 }}/>
                  {isProcessing && (
                    <div className="absolute" style={{ inset: 0, width: `${pct * 100}%`,
 background: 'linear-gradient(90deg, transparent, var(--paper) 50%, transparent)',
 backgroundSize: '80px 100%',
 animation: 'shimmer 1.4s linear infinite',
 mixBlendMode: 'overlay'
 }}/>
                  )}
                </div>
                <span className="mono text-ink-3 text-right" style={{ fontSize: 11 }}>
                  {isDone ? p.lang.split(' ')[0] :
                   rs.state === "queued" ? `${rs.queued}f` :
                   `${rs.processed}/${rs.totalFiles}`}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ScanStat({ label, value, accent }) {
  return (
    <div className="py-4 px-4 border-r" >
      <div className="display font-normal" style={{ fontSize: 28,
 color: accent ? 'var(--accent)' : 'var(--ink)' }}>{value}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 uppercase text-ink-3" >{label}</div>
    </div>
  );
}

// ─── 6 Projects ──────────────────────────────────────────────
// Every project has one or more repos. Default: 1 repo = 1 project.
// Multi-repo projects (grouped products) are auto-detected, user can split.
function WizProjects({ state, upd }) {
  const D = window.SENSEI_SETUP;
  const sols = state.solutions;
  const [selected, setSelected] = useS(null); // id of selected for role editing
  const [repoMenu, setRepoMenu] = useS(null); // { sid, pid } for repo action menu
  const [mergeMenu, setMergeMenu] = useS(null); // sid whose merge-target picker is open

  // Exclude an accidentally-detected project. It stays inside its watch folder,
  // but sensei stops treating it as a project — recorded as a folder exclusion.
  const excludedIds = state.excludedProjects || [];
  const setExcludedIds = (ids) => upd({ excludedProjects: ids });
  const exclude = (id) => { setExcludedIds([...excludedIds, id]); setSelected(null); setMergeMenu(null); };
  const restore = (id) => setExcludedIds(excludedIds.filter(i => i !== id));
  const activeSols   = sols.filter(s => !excludedIds.includes(s.id));
  const excludedSols = sols.filter(s =>  excludedIds.includes(s.id));

  const toggle = (id) => upd({
    solutions: sols.map(s => s.id === id ? { ...s, confirmed: !s.confirmed } : s)
  });
  const rename = (id, name) => upd({
    solutions: sols.map(s => s.id === id ? { ...s, renamed: name } : s)
  });
  const setRole = (pid, role) => upd({ roles: { ...state.roles, [pid]: role } });

  // Split a multi-repo project into individual single-repo projects
  const split = (sid) => {
    const sol = sols.find(s => s.id === sid);
    if (!sol || sol.projects.length < 2) return;
    const individuals = sol.projects.map(p => ({
      id: `proj-${p.id}`,
      name: p.name,
      kanji: sol.kanji,
      path: p.path,
      autoDetected: false,
      confidence: "split",
      confirmed: true,
      renamed: null,
      projects: [p]
    }));
    const idx = sols.findIndex(s => s.id === sid);
    upd({ solutions: [...sols.slice(0, idx), ...individuals, ...sols.slice(idx + 1)] });
  };

  // Move a single repo into another project (or to a brand-new project)
  const moveRepo = (fromSid, pid, toSid) => {
    const from = sols.find(s => s.id === fromSid);
    if (!from) return;
    const repo = from.projects.find(p => p.id === pid);
    if (!repo) return;

    // Remove repo from source; drop source project if now empty
    let nextSols = sols.map(s => s.id === fromSid
      ? { ...s, projects: s.projects.filter(p => p.id !== pid) }
      : s
    ).filter(s => s.projects.length > 0);

    if (toSid === "__new__") {
      const newProj = {
        id: `proj-${repo.id}-${Date.now()}`,
        name: repo.name,
        kanji: from.kanji,
        path: repo.path,
        autoDetected: false,
        confidence: "manual",
        confirmed: true,
        renamed: null,
        projects: [repo]
      };
      nextSols = [...nextSols, newProj];
    } else {
      nextSols = nextSols.map(s => s.id === toSid
        ? { ...s, projects: [...s.projects, repo] }
        : s
      );
    }
    upd({ solutions: nextSols });
    setRepoMenu(null);
  };

  // Merge entire project into another
  const mergeInto = (fromSid, toSid) => {
    const from = sols.find(s => s.id === fromSid);
    const to = sols.find(s => s.id === toSid);
    if (!from || !to) return;
    const nextSols = sols
      .map(s => s.id === toSid
        ? { ...s, projects: [...s.projects, ...from.projects] }
        : s)
      .filter(s => s.id !== fromSid);
    upd({ solutions: nextSols });
    setMergeMenu(null);
  };

  return (
    <div style={{ maxWidth: 940 }} className="mx-auto" >
      <WizHeader n="組" title="Projects"
                 tagline="A project has one or more repos. Edit, split, or confirm."/>

      <div style={{ fontSize: 13 }} className="mb-4 text-ink-3" >
        A single-repo project is the default. Multi-repo projects are auto-grouped from sibling folders and name patterns. Split when they shouldn't be together.
      </div>

      <div className="gap-3 flex flex-col" >
        {activeSols.map(s => {
          const isMulti = s.projects.length > 1;
          const isExpanded = selected === s.id;
          const isMergeOpen = mergeMenu === s.id;
          const mergeTargets = sols.filter(x => x.id !== s.id);
          return (
          <div key={s.id} style={{
 borderRadius: 10, background: s.confirmed ? 'var(--paper-2)' : 'var(--paper)', opacity: s.confirmed ? 1 : 0.55, transition: 'all .2s' }} className="p-4 border border-paper-edge relative" >
            <div style={{
 gridTemplateColumns: 'auto 1fr auto auto auto auto auto' }} className="gap-3 grid items-center" >
              <div className="kanji text-accent rounded-full bg-paper border border-paper-edge flex items-center justify-center" style={{
 fontSize: 28,
 width: 42, height: 42 }}>{s.kanji}</div>
              <div>
                <input value={s.renamed ?? s.name}
 onChange={e => rename(s.id, e.target.value)}
 className="display p-0 font-normal bg-transparent border-0 w-full"
 style={{
 fontSize: 22, outline: 'none',
 borderBottom: '1px dashed transparent' }}
 onFocus={e => e.target.style.borderBottom = '1px dashed var(--ink-3)'}
 onBlur={e => e.target.style.borderBottom = '1px dashed transparent'}/>
                <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                  {s.path} · {s.projects.length} {s.projects.length === 1 ? "repo" : "repos"}
                </div>
              </div>
              {isMulti ? (
                <span className="mono py-1 px-2 text-accent uppercase bg-accent-soft" style={{
 fontSize: 11,
 letterSpacing: '0.1em', border: '1px solid var(--accent)',
 borderRadius: 3 }}>
                  multi-repo
                </span>
              ) : <span/>}

              {/* merge button (only shown when there's another project to merge with) */}
              {mergeTargets.length > 0 ? (
                <button onClick={() => { setMergeMenu(isMergeOpen ? null : s.id); setRepoMenu(null); }}
 style={{
 fontSize: 11,
 borderRadius: 4
 }} className="py-1 px-2 text-ink-3" >
                  merge…
                </button>
              ) : <span/>}

              <button onClick={() => setSelected(isExpanded ? null : s.id)}
 style={{
 fontSize: 11,
 borderRadius: 4 }} className="py-1 px-2 gap-1 text-ink-3 flex items-center" >
                {isExpanded ? "hide" : "edit"}
                <span style={{ fontSize: 11, transform: isExpanded ? 'rotate(180deg)' : 'none',
                                transition: 'transform .2s' }}>▾</span>
              </button>
              <button onClick={() => exclude(s.id)}
 title="Not a project — exclude from scan"
 style={{ fontSize: 11, borderRadius: 4 }}
 className="py-1 px-2 text-ink-3" >
                exclude
              </button>
              <button className="text-paper flex items-center justify-center" onClick={() => toggle(s.id)}
 style={{
 width: 22, height: 22, borderRadius: 4,
 background: s.confirmed ? 'var(--ink)' : 'transparent',
 border: s.confirmed ? 'none' : 'var(--ink-line)' }}>
                {s.confirmed && <svg width="12" height="12" viewBox="0 0 16 16"><path d="M3 8 L7 12 L13 4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>}
              </button>
            </div>

            {/* Merge target picker */}
            {isMergeOpen && (
              <div style={{ borderRadius: 6,
 animation: 'expandIn .2s ease-out'
 }} className="mt-3 py-3 px-3 pl-12 bg-paper border border-paper-edge" >
                <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
                  Merge {s.renamed ?? s.name} into…
                </div>
                <div className="gap-1 flex flex-wrap" >
                  {mergeTargets.map(t => (
                    <button key={t.id} onClick={() => mergeInto(s.id, t.id)}
 className="mono py-1 px-2 gap-1 border border-paper-edge bg-paper-2 text-ink-2 inline-flex items-center" style={{
 fontSize: 11, borderRadius: 4 }}>
                      <span className="kanji text-accent" style={{ fontSize: 13 }}>{t.kanji}</span>
                      {t.renamed ?? t.name}
                      <span className="text-ink-4" style={{ fontSize: 11 }}>({t.projects.length})</span>
                    </button>
                  ))}
                  <button onClick={() => setMergeMenu(null)} className="mono py-1 px-2 text-ink-3" style={{
 fontSize: 11 }}>cancel</button>
                </div>
              </div>
            )}

            {/* Repo chips — compact, always visible */}
            <div className="mt-3 gap-1 pl-12 flex flex-wrap" >
              {s.projects.map(p => {
                const role = D.roles.find(r => r.id === state.roles[p.id]);
                const isOpen = repoMenu && repoMenu.sid === s.id && repoMenu.pid === p.id;
                const moveTargets = sols.filter(x => x.id !== s.id);
                return (
                  <span className="relative inline-block" key={p.id} >
                    <span className="mono gap-2 py-1 pl-2 pr-1 bg-paper border border-paper-edge text-ink-2 inline-flex items-center" style={{
 fontSize: 11,
 borderRadius: 3 }}>
                      {p.name}
                      <span className="text-ink-4" >{p.files}f</span>
                      {role && (
                        <span style={{ fontSize: 11,
 borderLeft: '1px solid var(--edge)'
 }} className="pl-2 text-accent" >
                          {role.label.toLowerCase()}
                        </span>
                      )}
                      <button onClick={(e) => {
 e.stopPropagation();
 setRepoMenu(isOpen ? null : { sid: s.id, pid: p.id });
 setMergeMenu(null);
 }}
 title="Move this repo"
 style={{
 fontSize: 13,
 borderLeft: '1px solid var(--edge)',
 lineHeight: 1
 }} className="px-1 ml-1 text-ink-3" >
                        ⋯
                      </button>
                    </span>
                    {isOpen && (
                      <div style={{ top: 'calc(100% + 4px)', left: 0, zIndex: 10,
 minWidth: 220, borderRadius: 6,
 animation: 'expandIn .15s ease-out'
 }} className="p-1 absolute bg-paper border border-paper-edge shadow" >
                        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="py-1 px-2 text-ink-3 uppercase" >
                          Move {p.name} to…
                        </div>
                        {moveTargets.map(t => (
                          <button key={t.id} onClick={() => moveRepo(s.id, p.id, t.id)}
 className="mono py-2 px-2 gap-2 flex w-full text-ink-2 items-center text-left" style={{
 fontSize: 11, borderRadius: 3 }}>
                            <span className="kanji text-accent" >{t.kanji}</span>
                            {t.renamed ?? t.name}
                          </button>
                        ))}
                        {isMulti && (
                          <button onClick={() => moveRepo(s.id, p.id, "__new__")}
 style={{
 fontSize: 11, borderRadius: 3 }} className="py-2 px-2 mt-1 pt-2 flex w-full text-ink-3 border-t text-left" >
                            + split out as new project
                          </button>
                        )}
                      </div>
                    )}
                  </span>
                );
              })}
              {isMulti && (
                <button onClick={() => split(s.id)} className="mono py-1 px-2 text-ink-3" style={{
 fontSize: 11,
 border: '1px dashed var(--edge)', borderRadius: 3
 }}>
                  split all into {s.projects.length} projects
                </button>
              )}
            </div>

            {/* Expanded: rename + role picker per repo */}
            {isExpanded && (
              <div style={{
 animation: 'expandIn .22s ease-out'
 }} className="mt-4 gap-1 pt-3 pl-12 border-t flex flex-col" >
                <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
                  Repo roles
                </div>
                {s.projects.map(p => (
                  <div key={p.id} style={{ gridTemplateColumns: '1fr auto' }} className="gap-3 py-2 px-0 grid items-center" >
                    <div>
                      <div className="mono text-ink-2" style={{ fontSize: 13 }}>{p.name}</div>
                      <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
                        {p.lang} · {p.files} files
                      </div>
                    </div>
                    <div style={{ borderRadius: 4
 }} className="gap-1 p-1 flex bg-paper" >
                      {D.roles.map(r => (
                        <button key={r.id} onClick={() => setRole(p.id, r.id)}
                                title={r.label}
                                style={{
 fontSize: 11, borderRadius: 3,
                                  background: state.roles[p.id] === r.id ? 'var(--ink)' : 'transparent',
                                  color: state.roles[p.id] === r.id ? 'var(--paper)' : 'var(--ink-3)'
}} className="py-1 px-2" >
                          {r.label}
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
          );
        })}
        <style>{`
          @keyframes expandIn { from { opacity: 0; max-height: 0 } to { opacity: 1; max-height: 400px } }
        `}</style>
      </div>

      {excludedSols.length > 0 && (
        <div className="mt-6" >
          <div style={{ fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
            Excluded from scan · {excludedSols.length}
          </div>
          <div style={{ fontSize: 12, lineHeight: 1.5, maxWidth: 620 }} className="mb-3 text-ink-3" >
            These folders sit inside a watched folder but aren't projects. Sensei keeps watching the
            folder and skips them — recorded as exclusions on the folder.
          </div>
          <div className="gap-1 flex flex-col" >
            {excludedSols.map(s => (
              <div key={s.id} style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 6, opacity: 0.85
 }} className="gap-3 py-2 px-3 grid items-center bg-paper-2 border border-paper-edge" >
                <span className="kanji text-ink-4" style={{ fontSize: 15 }}>{s.kanji}</span>
                <div>
                  <div className="text-ink-2" style={{ fontSize: 13 }}>{s.renamed ?? s.name}</div>
                  <div className="mono mt-1 text-ink-4" style={{ fontSize: 11 }}>{s.path}</div>
                </div>
                <button onClick={() => restore(s.id)}
 style={{ fontSize: 11, borderRadius: 4 }}
 className="py-1 px-2 text-accent" >restore</button>
              </div>
            ))}
          </div>
        </div>
      )}

      <div style={{ fontSize: 13 }} className="mt-4 text-ink-3" >
        More options — external integrations, clients, custom rules — per project later from its Settings.
      </div>
    </div>
  );
}

// (WizRoles + WizLinks removed — roles inline inside WizProjects;
//  external links moved to per-project Settings.)

// ─── 7 Metadata ──────────────────────────────────────────────
function WizMetadata({ state, upd }) {
  const D = window.SENSEI_SETUP;
  const setMeta = (sid, k, v) => upd({
    metadata: { ...state.metadata, [sid]: { ...state.metadata[sid], [k]: v } }
  });

  return (
    <div style={{ maxWidth: 820 }} className="mx-auto" >
      <WizHeader n="七" title="Context" tagline="Optional. Helps sensei tailor its coaching."/>

      <div className="gap-4 flex flex-col" >
        {state.solutions.filter(s => s.confirmed).map(s => (
          <div key={s.id} style={{ borderRadius: 10 }} className="p-6 border border-paper-edge bg-paper-2" >
            <div className="gap-2 mb-4 flex items-baseline" >
              <span className="kanji text-accent" style={{ fontSize: 22 }}>{s.kanji}</span>
              <span className="display" style={{ fontSize: 22 }}>{s.renamed ?? s.name}</span>
            </div>

            <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-4 grid" >
              <MetaField label="Stage">
                <div style={{ borderRadius: 5
 }} className="gap-1 p-1 flex bg-paper" >
                  {D.metadata.statuses.map(st => (
                    <button key={st.id}
 onClick={() => setMeta(s.id, "status", st.id)}
 style={{ fontSize: 11, borderRadius: 3,
 background: state.metadata[s.id].status === st.id ? 'var(--ink)' : 'transparent',
 color: state.metadata[s.id].status === st.id ? 'var(--paper)' : 'var(--ink-3)'
 }} className="py-1 px-2 flex-1" >
                      {st.label}
                    </button>
                  ))}
                </div>
              </MetaField>

              <MetaField label="Client (optional)">
                <input value={state.metadata[s.id].client}
 onChange={e => setMeta(s.id, "client", e.target.value)}
 placeholder="e.g. Internal"
 style={{ fontSize: 13,
 borderRadius: 5, outline: 'none'
 }} className="py-2 px-3 w-full bg-paper border border-paper-edge" />
              </MetaField>

              <div style={{ gridColumn: 'span 2' }}>
                <MetaField label="Goal">
                  <input value={state.metadata[s.id].goal}
 onChange={e => setMeta(s.id, "goal", e.target.value)}
 placeholder="One sentence. Why this exists."
 style={{ fontSize: 13,
 borderRadius: 5, outline: 'none'
 }} className="py-2 px-3 w-full bg-paper border border-paper-edge" />
                </MetaField>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div style={{ fontSize: 13 }} className="mt-4 text-ink-3" >
        Skip if you like. These can be edited per-solution from the Coaching page.
      </div>
    </div>
  );
}

function MetaField({ label, children }) {
  return (
    <div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 uppercase text-ink-3" >{label}</div>
      {children}
    </div>
  );
}

// ─── Libraries — things sensei should WRAP ──────────────────
// These are libs WITHOUT their own MCP. Sensei indexes code + docs
// and exposes its own tools over them. Libraries with a proper MCP
// (Postgres, Stripe, etc.) belong in the Instruments step instead.
function WizLibraries({ state, upd }) {
  const D = window.SENSEI_SETUP.discoveredLibraries || { detected: [] };
  const [form, setForm] = useS({ name: "", url: "", lang: "Rust" });
  const [showAdd, setShowAdd] = useS(false);

  const toggle = (id) => upd({ libraries: { ...state.libraries, [id]: !state.libraries[id] } });
  const toggleExtra = (id) => upd({
    libExtras: state.libExtras.map(x => x.id === id ? { ...x, on: !x.on } : x)
  });
  const addExtra = () => {
    if (!form.name.trim()) return;
    const id = "usr-" + form.name.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-");
    upd({ libExtras: [...state.libExtras, { id, ...form, on: true, custom: true }] });
    setForm({ name: "", url: "", lang: form.lang });
    setShowAdd(false);
  };
  const removeExtra = (id) =>
    upd({ libExtras: state.libExtras.filter(x => x.id !== id) });

  const activeCount = D.detected.filter(l => state.libraries[l.id]).length
                     + state.libExtras.filter(x => x.on).length;

  return (
    <div>
      <WizHeader n="書" title="Libraries"
                 tagline="Libraries without their own MCP — sensei indexes docs & code and wraps them with its own tools. Anything with a proper MCP (like Postgres or Stripe) comes in the next step."/>

      {/* Summary bar */}
      <div className="gap-3 mb-4 flex items-center" >
        <span className="mono py-1 px-2 text-ink-2 bg-paper-2 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 4
 }}>
          {D.detected.length} detected
        </span>
        <span className="mono py-1 px-2 text-success bg-success-soft" style={{
 fontSize: 11,
 borderRadius: 4
 }}>
          {activeCount} will be wrapped
        </span>
        {state.libExtras.length > 0 && (
          <span className="mono py-1 px-2 text-accent bg-accent-soft" style={{
 fontSize: 11,
 borderRadius: 4
 }}>
            {state.libExtras.length} added by you
          </span>
        )}
      </div>

      {/* Detected libraries */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
        Detected · sensei will wrap
      </div>
      <div style={{ borderRadius: 6 }} className="mb-6 flex flex-col border border-paper-edge bg-paper-2" >
        {D.detected.map((lib, i) => {
          const on = !!state.libraries[lib.id];
          return (
            <div key={lib.id}
 style={{
 gridTemplateColumns: 'auto 1fr auto auto auto',
 borderBottom: i < D.detected.length - 1 ? 'var(--hairline)' : 'none',
 opacity: on ? 1 : 0.45
 }} className="gap-3 py-3 px-3 grid items-center" >
              <button className="text-paper" onClick={() => toggle(lib.id)}
 style={{ width: 18, height: 18, borderRadius: 3,
 border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
 background: on ? 'var(--accent)' : 'transparent', fontSize: 11, lineHeight: 1 }}>
                {on ? "✓" : ""}
              </button>
              <div>
                <div className="text-ink" style={{ fontSize: 13 }}>
                  {lib.name}
                  <span className="mono ml-2 text-ink-4" style={{
 fontSize: 11 }}>{lib.version}</span>
                </div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
                  {lib.why}
                </div>
              </div>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {lib.lang}
              </span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {lib.usage}× uses
              </span>
              <LibDocChip status={lib.docs}/>
            </div>
          );
        })}
      </div>

      {/* User-added */}
      {state.libExtras.length > 0 && (
        <>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
            Added by you
          </div>
          <div style={{ borderRadius: 6 }} className="mb-6 flex flex-col border border-paper-edge bg-paper-2" >
            {state.libExtras.map((lib, i) => {
              const on = lib.on;
              return (
                <div key={lib.id}
 style={{
 gridTemplateColumns: 'auto 1fr auto auto',
 borderBottom: i < state.libExtras.length - 1 ? 'var(--hairline)' : 'none',
 opacity: on ? 1 : 0.45
 }} className="gap-3 py-3 px-3 grid items-center" >
                  <button className="text-paper" onClick={() => toggleExtra(lib.id)}
 style={{ width: 18, height: 18, borderRadius: 3,
 border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
 background: on ? 'var(--accent)' : 'transparent', fontSize: 11, lineHeight: 1 }}>
                    {on ? "✓" : ""}
                  </button>
                  <div>
                    <div className="text-ink" style={{ fontSize: 13 }}>{lib.name}</div>
                    <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                      {lib.url || "no URL"}
                    </div>
                  </div>
                  <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                    {lib.lang}
                  </span>
                  <button className="text-ink-4" onClick={() => removeExtra(lib.id)}
 style={{ fontSize: 11 }}>remove</button>
                </div>
              );
            })}
          </div>
        </>
      )}

      {/* Add custom library */}
      {!showAdd ? (
        <button onClick={() => setShowAdd(true)}
 style={{
 fontSize: 13,
 border: '1px dashed var(--ink-4)', borderRadius: 6 }} className="py-2 px-4 bg-paper-2 text-ink-2" >
          + Add a library
        </button>
      ) : (
        <div style={{ borderRadius: 8,
 maxWidth: 640
 }} className="py-4 px-4 bg-paper-2 border border-paper-edge" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-3 text-ink-3 uppercase" >
            Add a library sensei should wrap
          </div>
          <div style={{ gridTemplateColumns: '1.2fr 1fr 0.5fr'
 }} className="gap-2 mb-3 grid" >
            <div>
              <div style={{ fontSize: 11 }} className="mb-1 text-ink-2" >Name</div>
              <input value={form.name}
                     onChange={e => setForm({ ...form, name: e.target.value })}
                     placeholder="e.g. @internal/fx"
                     style={wizInputStyle} className="mb-1" />
            </div>
            <div>
              <div className="text-ink-2" style={{ fontSize: 11 }}>
                Docs URL <span className="text-ink-4" >· optional</span>
              </div>
              <input value={form.url}
                     onChange={e => setForm({ ...form, url: e.target.value })}
                     placeholder="https://docs.rs/… or internal wiki"
                     style={wizInputStyle} className="mb-1" />
            </div>
            <div>
              <div className="text-ink-2" style={{ fontSize: 11 }}>Lang</div>
              <select value={form.lang}
                      onChange={e => setForm({ ...form, lang: e.target.value })}
                      style={wizInputStyle} className="gap-2" >
                <option>Rust</option>
                <option>TypeScript</option>
                <option>Python</option>
                <option>Go</option>
                <option>Other</option>
              </select>
            </div>
          </div>
          <div className="flex" >
            <button onClick={addExtra} disabled={!form.name.trim()}
                    style={{
 fontSize: 13,
                              background: form.name.trim() ? 'var(--ink)' : 'var(--paper-3)',
                              color: form.name.trim() ? 'var(--paper)' : 'var(--ink-3)',
                              borderRadius: 4
}} className="py-2 px-3" >
              Add
            </button>
            <button onClick={() => setShowAdd(false)}
 style={{
 fontSize: 13 }} className="py-2 px-3 text-ink-3" >
              Cancel
            </button>
            <span className="flex-1" />
            <span className="text-ink-3 self-center" style={{ fontSize: 11 }}>
              Sensei will index docs and expose tools that answer questions about this library.
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Instruments — recommended MCPs based on detected stack ─
// These are libraries / services that bring their OWN MCP.
// Sensei doesn't need to index them; it just installs the MCP
// so other tools (including sensei) can call it.
function WizRegistry({ state, upd }) {
  const R = window.SENSEI_SETUP.mcpRegistry || { available: [] };
  const stack = window.SENSEI_SETUP.detectedStack || { services: [], frameworks: [], languages: [], runtimes: [] };

  const toggle = (id) => upd({ mcps: { ...state.mcps, [id]: !state.mcps[id] } });

  const recommended = R.available.filter(m => m.recommended);
  const others      = R.available.filter(m => !m.recommended);
  const installCount = R.available.filter(m => state.mcps[m.id]).length;

  return (
    <div>
      <WizHeader n="器" title="Instruments"
                 tagline="Tools sensei can reach for — recommended based on what's in your stack. Each MCP brings its own capabilities, no wrapping needed."/>

      {/* Detected stack summary */}
      <div style={{ borderRadius: 8
 }} className="py-3 px-4 mb-6 bg-paper-2 border border-paper-edge" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
          Detected in your stack
        </div>
        <div className="gap-1 flex flex-wrap" >
          {[...stack.languages, ...stack.frameworks, ...stack.services].map(s => (
            <span key={s} className="mono py-1 px-2 bg-paper border border-paper-edge text-ink-2" style={{
 fontSize: 11, borderRadius: 3 }}>
              {s}
            </span>
          ))}
        </div>
      </div>

      {/* Summary row */}
      <div className="gap-3 mb-4 flex items-center" >
        <span className="mono py-1 px-2 text-success bg-success-soft" style={{
 fontSize: 11,
 borderRadius: 4
 }}>
          {recommended.length} recommended
        </span>
        <span className="mono py-1 px-2 text-ink-2 bg-paper-2 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 4
 }}>
          {installCount} will be installed
        </span>
      </div>

      {/* Recommended */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
        Recommended for your stack
      </div>
      <div style={{ borderRadius: 6 }} className="mb-6 flex flex-col border border-paper-edge bg-paper-2" >
        {recommended.map((mcp, i) => (
          <McpRow key={mcp.id} mcp={mcp} on={!!state.mcps[mcp.id]}
                  onToggle={() => toggle(mcp.id)}
                  last={i === recommended.length - 1}/>
        ))}
      </div>

      {/* Available */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
        Also available
      </div>
      <div className="flex flex-col border border-paper-edge bg-paper-2" style={{ borderRadius: 6 }}>
        {others.map((mcp, i) => (
          <McpRow key={mcp.id} mcp={mcp} on={!!state.mcps[mcp.id]}
                  onToggle={() => toggle(mcp.id)}
                  last={i === others.length - 1}/>
        ))}
      </div>
    </div>
  );
}

function McpRow({ mcp, on, onToggle, last }) {
  return (
    <div style={{
 gridTemplateColumns: 'auto auto 1fr auto auto',
 borderBottom: last ? 'none' : 'var(--hairline)',
 opacity: on ? 1 : 0.55
 }} className="gap-3 py-3 px-3 grid items-center" >
      <button className="text-paper" onClick={onToggle}
 style={{ width: 18, height: 18, borderRadius: 3,
 border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
 background: on ? 'var(--accent)' : 'transparent', fontSize: 11, lineHeight: 1 }}>
        {on ? "✓" : ""}
      </button>
      <div className="bg-paper-3 flex items-center justify-center" style={{ width: 32, height: 32, borderRadius: 6 }}>
        <span className="kanji text-accent" style={{ fontSize: 15 }}>
          {mcp.kanji}
        </span>
      </div>
      <div>
        <div className="text-ink" style={{ fontSize: 13 }}>
          {mcp.name}
          <span className="mono ml-2 text-ink-4" style={{ fontSize: 11 }}>
            by {mcp.publisher}
          </span>
          {mcp.verified && (
            <span className="mono ml-2 py-1 px-1 text-success bg-success-soft" style={{
 fontSize: 11, borderRadius: 3
 }}>
              verified
            </span>
          )}
        </div>
        <div style={{
 fontSize: 11,
 lineHeight: 1.45
 }} className="mt-1 text-ink-3" >
          {mcp.summary}
        </div>
      </div>
      <span className="mono text-ink-3" style={{ fontSize: 11 }}>
        {mcp.tools} tools
      </span>
      {mcp.trigger && mcp.trigger.length > 0 ? (
        <span className="mono py-1 px-2 text-success bg-success-soft whitespace-nowrap" style={{
 fontSize: 11,
 borderRadius: 3 }}>
          matches {mcp.trigger[0]}
        </span>
      ) : (
        <span className="mono text-ink-4" style={{ fontSize: 11 }}>
          {mcp.kind}
        </span>
      )}
    </div>
  );
}

function LibDocChip({ status }) {
  const map = {
    indexed: { label: "docs indexed", tone: 'var(--success)',  bg: 'var(--success-soft)' },
    partial: { label: "partial",      tone: 'var(--warning)', bg: 'var(--warning-soft)' },
    schema:  { label: "schema only",  tone: 'var(--ink-2)', bg: 'var(--paper-3)'   },
    none:    { label: "no docs",      tone: 'var(--ink-3)', bg: 'var(--paper-3)'   }
  };
  const m = map[status] || map.none;
  return (
    <span className="mono py-1 px-2 whitespace-nowrap" style={{
 fontSize: 11, borderRadius: 3,
 background: m.bg, color: m.tone }}>
      {m.label}
    </span>
  );
}

const wizInputStyle = {
  width: '100%', padding: '8px 8px', fontSize: 13,
  border: 'var(--hairline)', borderRadius: 5,
  background: 'var(--paper)', color: 'var(--ink)',
  fontFamily: 'var(--font-mono)', outline: 'none'
};

// ─── 8 Inference ─────────────────────────────────────────────
// WizInference lives in lib/wiz-inference.jsx — loaded via <script> tag after this file.
// Expects window.SENSEI_SETUP.inference (system, providers, rolePriority, addable).

// ─── 9 Preferences ──────────────────────────────────────────
// Pre-flight tweaks before stepping into the observatory. The display name
// is seeded from $HOME (system.homeDir → username), but the user can override
// it. Telemetry, sharing cadence, and sensei's tone all live here so they
// can be revisited any time by re-opening the wizard.
function WizPreferences({ state, upd }) {
  const D = window.SENSEI_SETUP;
  const p = state.prefs || {};
  const setP = (patch) => upd({ prefs: { ...p, ...patch } });

  // Display-name basename pulled from $HOME — used as the input's placeholder
  // so the user sees a sensible default that they can accept by leaving the
  // field as-is or overwrite to whatever they prefer.
  const homeBase = (D.system?.homeDir || "").split("/").filter(Boolean).pop() || "";

  // Reusable Section / Row primitives kept local to this stage so the
  // wizard file stays self-contained.
  // `right` is an optional slot rendered on the same row as the kanji + title
  // so a section with a single control (e.g. a name input) doesn't waste a
  // whole vertical block. When `right` is provided, the section renders as
  // a single row; `children` is omitted.
  const Section = ({ kanji, title, sub, children, right }) => (
    <section className="pt-6 pb-1" >
      <header style={{
 marginBottom: right ? 0 : 14
 }} className="gap-3 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 22,
 lineHeight: 1, width: 30 }}>{kanji}</span>
        <div className="flex-1 min-w-0" >
          <h3 className="display m-0 font-normal text-ink" style={{
 fontSize: 17 }}>{title}</h3>
          {sub && (
            <p style={{
 fontSize: 13,
 maxWidth: 540, lineHeight: 1.5
 }} className="mt-1 mb-0 text-ink-3" >{sub}</p>
          )}
        </div>
        {right && (
          <div className="shrink-0 self-center" style={{ minWidth: 220 }}>
            {right}
          </div>
        )}
      </header>
      {!right && <div className="divide-y pl-12">{children}</div>}
    </section>
  );
  const Row = ({ label, hint, children }) => (
    <div style={{ gridTemplateColumns: '1fr auto' }} className="gap-8 py-3 px-0 grid items-center" >
      <div className="min-w-0" >
        <div className="text-ink" style={{ fontSize: 13 }}>{label}</div>
        {hint && <div style={{
 fontSize: 11,
 lineHeight: 1.45, maxWidth: 460
 }} className="mt-1 text-ink-3" >{hint}</div>}
      </div>
      <div className="shrink-0" >{children}</div>
    </div>
  );
  const Toggle = ({ value, onChange }) => (
    <button onClick={() => onChange(!value)}
 style={{
 width: 36, height: 20, borderRadius: 999,
 background: value ? 'var(--ink)' : 'var(--paper-3)',
 transition: 'background 0.15s' }} className="p-0 relative cursor-pointer border-0" >
      <span className="absolute rounded-full bg-paper shadow-sm" style={{ top: 2, left: value ? 18 : 2,
 width: 16, height: 16,
 transition: 'left 0.18s ease' }}/>
    </button>
  );
  const Segment = ({ value, onChange, options }) => (
    <div className="inline-flex border border-paper-edge overflow-hidden" style={{
 borderRadius: 5 }}>
      {options.map((opt, i) => (
        <button key={opt.value} onClick={() => onChange(opt.value)}
 style={{
 fontSize: 11,
 borderLeft: i === 0 ? 'none' : 'var(--hairline)',
 background: value === opt.value ? 'var(--paper-3)' : 'var(--paper)',
 color: value === opt.value ? 'var(--ink)' : 'var(--ink-3)' }} className="py-1 px-3 cursor-pointer" >
          {opt.label}
        </button>
      ))}
    </div>
  );
  const Sel = ({ value, onChange, options }) => (
    <select value={value} onChange={e => onChange(e.target.value)}
 style={{
 fontSize: 13,
 borderRadius: 5,
 fontFamily: 'inherit'
 }} className="py-1 px-2 border border-paper-edge bg-paper text-ink cursor-pointer" >
      {options.map(o =>
        <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );

  return (
    <div style={{ maxWidth: 760 }} className="mx-auto" >
      <WizHeader n="名" title="Profile"
                 tagline="A few small choices. Anything here can be changed later from Preferences."/>

      <div className="divide-y">
      {/* ── What should sensei call you ──────────────────────────────
          Inline single-row layout: kanji + title + description on the
          left, prefilled input on the right. Prefilled with the user's
          home folder name; no extra hint UI per design spec. */}
      <Section kanji="名" title="What should sensei call you?"
               sub="Pulled from your home folder. Change it to whatever feels right."
               right={
                 <input
 value={p.displayName || ""}
 onChange={e => setP({ displayName: e.target.value })}
 placeholder={homeBase || "your name"}
 style={{
 width: 240, fontSize: 13, borderRadius: 5,
 fontFamily: 'inherit', outline: 'none' }}
 onFocus={e => e.target.style.borderColor = 'var(--accent)'}
 onBlur={e => e.target.style.borderColor = ''}
 className="py-2 px-3 border border-paper-edge bg-paper text-ink text-right" />
               }/>

      {/* ── Shared learnings ─────────────────────────────────────── */}
      {/* Sharing + collective controls now live in the Observatory's Dōjō section. */}

      {/* ── Sensei behavior ──────────────────────────────────────── */}
      <Section kanji="師" title="Sensei behavior"
               sub="How forward sensei is — when it nudges, how it phrases corrections.">
        <Row label="Correction tone"
             hint="How direct sensei is when something repeats.">
          <Segment value={p.correctionAggressiveness}
                    onChange={v => setP({ correctionAggressiveness: v })}
                    options={[
                      { value: "gentle",   label: "Gentle" },
                      { value: "balanced", label: "Balanced" },
                      { value: "direct",   label: "Direct" }
                    ]}/>
        </Row>
        <Row label="Morning digest"
             hint="The Today view. Off keeps the dashboard quiet.">
          <Segment value={p.digestCadence}
                    onChange={v => setP({ digestCadence: v })}
                    options={[
                      { value: "off",    label: "Off" },
                      { value: "daily",  label: "Daily" },
                      { value: "weekly", label: "Weekly" }
                    ]}/>
        </Row>
        <Row label="Nudge on regression"
             hint="If FTR drops sharply on a project, sensei surfaces it on Today.">
          <Toggle value={p.nudgeOnRegression}
                   onChange={v => setP({ nudgeOnRegression: v })}/>
        </Row>
      </Section>

      {/* ── Telemetry ────────────────────────────────────────────── */}
      <Section kanji="守" title="Telemetry"
               sub="Help us improve sensei itself — separate from shared learnings, this is about the app, not your work.">
        <Row label="Anonymized usage telemetry"
             hint="Crashes, performance, which views you visit. Never code, prompts, or session content. Off by default.">
          <Toggle value={p.anonymizedTelemetry}
                   onChange={v => setP({ anonymizedTelemetry: v })}/>
        </Row>
        <Row label="Show welcome message on first entry"
             hint="The greeting toast that appears when you first open the observatory each day.">
          <Toggle value={p.showWelcome}
                   onChange={v => setP({ showWelcome: v })}/>
        </Row>
      </Section>

      </div>

      <p style={{
 fontSize: 13, lineHeight: 1.6 }} className="mt-6 mb-0 text-ink-3 italic text-center" >
        Saved when you press <span className="text-ink-2" >Save &amp; close</span>.
        Re-open from the sidebar's <span className="text-ink-2" >調 Preferences</span> link anytime.
      </p>
    </div>
  );
}


function WizDone({ state }) {
  const confirmedSols = state.solutions.filter(s => s.confirmed);
  const repoCount = confirmedSols.reduce((a, s) => a + s.projects.length, 0);
  const activeAcps = Object.values(state.acps).filter(Boolean).length;
  const libCount = Object.values(state.libraries || {}).filter(Boolean).length
                 + (state.libExtras || []).filter(x => x.on).length;
  const mcpCount = Object.values(state.mcps || {}).filter(Boolean).length;

  return (
    <div style={{ maxWidth: 680 }} className="mt-4 mb-0 mx-auto text-center" >
      <div className="kanji mb-2 text-accent" style={{ fontSize: 56 }}>観</div>
      <h1 className="display mt-0 mb-4 font-light" style={{
 fontSize: 40, letterSpacing: '-0.02em'
 }}>
        The observatory is ready.
      </h1>
      <p style={{
 fontSize: 15, lineHeight: 1.6, maxWidth: 480
 }} className="mt-0 mb-8 mx-auto text-ink-2" >
        Start a session with your assistant. Sensei will watch in silence for a few days,
        then begin to teach.
      </p>

      <div style={{ gridTemplateColumns: 'repeat(5, 1fr)', borderRadius: 10 }} className="gap-0 grid border border-paper-edge bg-paper-2 overflow-hidden text-left" >
        <DoneStat label="Projects"   value={confirmedSols.length}/>
        <DoneStat label="Repos"      value={repoCount}/>
        <DoneStat label="Libraries"  value={libCount}/>
        <DoneStat label="MCPs"       value={mcpCount}/>
        <DoneStat label="Assistants" value={activeAcps} last/>
      </div>

      <p className="mono mt-8 text-ink-3 italic" style={{
 fontSize: 11 }}>
        師 · the first session is always the teacher
      </p>
    </div>
  );
}

function DoneStat({ label, value, last }) {
  return (
    <div style={{ borderRight: last ? 'none' : 'var(--hairline)' }} className="py-4 px-4" >
      <div className="display font-normal" style={{ fontSize: 28 }}>{value}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 uppercase text-ink-3" >{label}</div>
    </div>
  );
}

// ─── Shared: step header ─────────────────────────────────────
function WizHeader({ n, title, tagline, eyebrow = "Step" }) {
  // Sticky to the top of the stage's scroll container so the step title +
  // tagline stay anchored as the user scrolls long stages.
  return (
    <div className="mb-6 pt-1 pb-4 sticky bg-paper border-b"
 style={{ top: -44, zIndex: 5 }}>
      <KanjiHeader variant="h1" kanji={n} eyebrow={eyebrow} title={title} description={tagline}/>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// Empty Observatory — what the user sees before any sessions.
// No sidebar (nothing to navigate to yet). Center the invitation.
function EmptyObservatoryApp({ onBeginSetup }) {
  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Empty Observatory"
 >
      <TauriChrome title="Sensei  先生"/>
      <main className="py-8 px-16 flex-1 overflow-auto relative flex items-center justify-center" >
        {/* faint watermark — 空 = emptiness */}
        <div className="kanji absolute text-accent" style={{ top: '50%', left: '50%',
 transform: 'translate(-50%, -50%)',
 fontSize: 56, opacity: 0.035,
 lineHeight: 1, userSelect: 'none', pointerEvents: 'none'
 }}>空</div>

        <div style={{
 maxWidth: 680, zIndex: 1, gridTemplateColumns: '1fr 1fr' }} className="gap-12 w-full relative grid items-center" >
          {/* Left: the invitation */}
          <div>
            <div className="gap-2 mb-6 flex items-baseline" >
              <span className="kanji text-accent" style={{ fontSize: 28 }}>先生</span>
              <span className="display font-normal" style={{ fontSize: 22 }}>Sensei</span>
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-3 text-ink-3 uppercase" >
              Welcome
            </div>
            <h1 className="display mt-0 mb-4 font-light" style={{
 fontSize: 56,
 letterSpacing: '-0.02em', lineHeight: 1.08
 }}>
              A quiet<br/>
              <span className="text-accent" >empty room.</span>
            </h1>
            <p style={{
 fontSize: 15, lineHeight: 1.7
 }} className="mt-0 mb-8 text-ink-2" >
              Point sensei at your folders and keep working. It watches in silence, learns
              the shape of each project, and later begins to teach.
            </p>

            <button onClick={onBeginSetup}
 style={{
 fontSize: 13, borderRadius: 6, letterSpacing: 0.2
 }} className="py-3 px-6 bg-ink text-paper" >
              Begin setup →
            </button>

            <div style={{ fontSize: 11 }} className="mt-4 text-ink-3" >
              <span className="mono">~4 minutes</span>
              <span className="mx-2 text-ink-4" >·</span>
              nothing leaves your machine
            </div>
          </div>

          {/* Right: what sensei will do — a real preview, not placeholder stats */}
          <div style={{ borderRadius: 10 }} className="py-6 px-6 border border-paper-edge bg-paper-2" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-4 text-ink-3 uppercase" >
              What sensei does
            </div>
            <div className="gap-4 flex flex-col" >
              {[
                { k: "観", label: "Watches",
                  note: "Every assistant session — prompts, tool calls, diffs." },
                { k: "察", label: "Notices",
                  note: "Which prompts work, which patterns repeat, where you rework." },
                { k: "教", label: "Teaches",
                  note: "After ~3 sessions per project, offers concrete suggestions." }
              ].map((x, i) => (
                <div key={i} style={{ gridTemplateColumns: 'auto 1fr' }} className="gap-3 grid items-start" >
                  <div className="kanji text-accent rounded-full bg-paper border border-paper-edge flex items-center justify-center" style={{
 fontSize: 17,
 width: 32, height: 32 }}>{x.k}</div>
                  <div>
                    <div className="display mb-1 font-normal" style={{ fontSize: 13 }}>
                      {x.label}
                    </div>
                    <div className="text-ink-3" style={{ fontSize: 13, lineHeight: 1.5 }}>
                      {x.note}
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <div style={{
 fontSize: 11, lineHeight: 1.6
 }} className="mt-4 pt-4 border-t text-ink-3" >
              Works with{' '}
              <span className="mono text-ink-2" >claude-code</span>,{' '}
              <span className="mono text-ink-2" >cursor</span>,{' '}
              <span className="mono text-ink-2" >codex</span>,{' '}
              <span className="mono text-ink-2" >aider</span>.
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

// Combined artboard: empty observatory → click "begin setup" → walk the
// wizard → "Enter Observatory" lands in the early · still-listening state
// (welcome toast included), mirroring the live Configure flow inside the
// observatory shell.
function EmptyToWizardApp() {
  const [mode, setMode] = useS("empty"); // "empty" | "wizard" | "entered"
  if (mode === "wizard") {
    return <SetupWizard onExit={() => setMode("empty")}
                         onDone={() => setMode("entered")}/>;
  }
  if (mode === "entered" && window.ObservatoryDaily) {
    // stateMode="early" + firstEntry={true} → freshly-configured observatory
    // with the welcome toast surfaced.
    return <window.ObservatoryDaily stateMode="early" firstEntry={true}
                                     onBack={() => setMode("empty")}/>;
  }
  return <EmptyObservatoryApp onBeginSetup={() => setMode("wizard")}/>;
}

// ─────────────────────────────────────────────────────────────
// First run — the ENTIRE first-time gate is now just the scan.
// Bootstrap (splash) has already confirmed the foundation; here we point
// sensei at the folders and watch projects materialize. Everything else
// (libraries, instruments, inference, assignments, profile) starts on
// sensible defaults the user can revisit in Preferences. On completion we
// hand off to the observatory, which lands on Projects (not an empty Today)
// until the first insights form.
function FirstRunScan({ onDone }) {
  const [state, setState] = useS({
    folders: [],   // first run starts empty — you add a root before scanning
    newFolder: "",
    scan: { started: false, done: false, tick: 0 }
  });
  const upd = (patch) => setState(prev => ({ ...prev, ...patch }));

  // No separate welcome strip or footer — the scan's own header carries the
  // welcome + "defaults live in Preferences" copy, and the only forward action
  // (Open projects) lives in the scan-results banner once the scan completes.
  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="First run · Scan"
 >
      <TauriChrome title="Sensei  先生  ·  welcome"/>
      <div className="pt-12 pb-12 px-16 flex-1 overflow-auto" >
        <WizScan state={state} upd={upd} context="first-run" onComplete={() => onDone && onDone()}/>
      </div>
    </div>
  );
}

// Harness — first-run scan, then the observatory landing on Projects (early).
// No join gate: sign-in → scan your machine → land straight on your running
// work. Joining a Dōjō is never a step here; it's an optional offer that lives
// on the workspace itself (and in Today / Preferences), taken when you want to
// share. Nothing blocks you from your own projects and running tasks.
function FirstRunApp() {
  // scan your machine → enter the observatory (Projects, early)
  const [phase, setPhase] = useS("scan");
  if (phase === "entered" && window.ObservatoryDaily) {
    return <window.ObservatoryDaily stateMode="early" firstEntry={true}/>;
  }
  return <FirstRunScan onDone={() => setPhase("entered")}/>;
}

Object.assign(window, {
  SetupWizard, EmptyObservatoryApp, EmptyToWizardApp, WIZ_STAGES,
  FirstRunScan, FirstRunApp
});
