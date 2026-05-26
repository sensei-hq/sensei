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
//   想 inference   · "thought / reasoning" — what models do
//   任 assignments · "entrust / assign" roles to models
//   入 done        · "enter" — the door at the end
// Rail subs are written in the same voice as the page taglines —
// short complete sentences, lowercase sensei, second-person, periods.
const WIZ_STAGES = [
  { id: "welcome",     n: "礼",  title: "Welcome",         sub: "A quiet observer. Nothing more." },
  { id: "preferences", n: "名",  title: "Preferences",     sub: "A few small choices before you step in." },
  { id: "acps",        n: "連",  title: "Assistants",      sub: "Connect the AI tools you already use." },
  { id: "folders",     n: "庵",  title: "Folders",         sub: "Where your work lives." },
  { id: "scan",        n: "観",  title: "Scan",            sub: "Workers recurse. Repos surface." },
  { id: "projects",    n: "組",  title: "Projects",        sub: "Each project, one or more repos." },
  { id: "libraries",   n: "書",  title: "Libraries",       sub: "What sensei should wrap." },
  { id: "registry",    n: "器",  title: "Instruments",     sub: "Tools sensei can reach for." },
  { id: "inference",   n: "想",  title: "Inference",       sub: "Local models, and a few clouds." },
  { id: "assignments", n: "任",  title: "Assignments",     sub: "Which model handles which role." },
  { id: "done",        n: "入",  title: "Enter",           sub: "The observatory is ready." }
];

// ─────────────────────────────────────────────────────────────
// Root wizard
function SetupWizard({ onDone, onExit }) {
  const D = window.SENSEI_SETUP;
  const [stageIdx, setStageIdx] = useS(0);
  const stage = WIZ_STAGES[stageIdx];

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

  const next = () => setStageIdx(i => Math.min(i + 1, WIZ_STAGES.length - 1));
  const back = () => setStageIdx(i => Math.max(i - 1, 0));

  return (
    <div className="sensei" data-screen-label="Setup Wizard"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生  ·  setup"/>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '260px 1fr', minHeight: 0 }}>
        <WizRail stages={WIZ_STAGES} stageIdx={stageIdx} setStageIdx={setStageIdx} onExit={onExit}/>
        <div style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <div style={{ flex: 1, overflow: 'auto' }} className="pt-7 pb-6 px-8" >
            {stage.id === "welcome"    && <WizWelcome/>}
            {stage.id === "components" && <WizComponents state={state} upd={upd}/>}
            {stage.id === "acps"       && <WizAcps state={state} upd={upd}/>}
            {stage.id === "folders"    && <WizFolders state={state} upd={upd}/>}
            {stage.id === "scan"       && <WizScan state={state} upd={upd}/>}
            {stage.id === "projects"   && <WizProjects state={state} upd={upd}/>}
            {stage.id === "libraries"  && <WizLibraries state={state} upd={upd}/>}
            {stage.id === "registry"   && <WizRegistry state={state} upd={upd}/>}
            {stage.id === "inference"   && <WizInference state={state} upd={upd}/>}
            {stage.id === "assignments" && <WizAssignments state={state} upd={upd}/>}
            {stage.id === "preferences" && <WizPreferences state={state} upd={upd}/>}
            {stage.id === "done"        && <WizDone state={state}/>}
          </div>
          <WizBottom stage={stage} stageIdx={stageIdx} total={WIZ_STAGES.length}
                     back={back} next={next}
                     onDone={() => {
                       // Persist whatever the user landed on in Preferences
                       // before handing control back to the host.
                       writeSenseiSettings(state.prefs || {});
                       onDone && onDone();
                     }}
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
    <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
      <span style={{ width: 7, height: 7, borderRadius: 4,
                      background: anyBad ? 'var(--accent)' : 'var(--success)' }}/>
      <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.4 }}>
        <div style={{ letterSpacing: '0.1em', textTransform: 'uppercase',
                       fontSize: 11, color: 'var(--ink-3)' }}>
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
function WizRail({ stages, stageIdx, setStageIdx, onExit }) {
  return (
    <aside style={{
 borderRight: 'var(--hairline)',
                    display: 'flex', flexDirection: 'column',
                    background: 'var(--paper-2)', overflow: 'hidden'
}} className="py-5 px-5" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-5" >
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>先生</span>
        <span className="display" style={{ fontSize: 17 }}>Sensei</span>
        <span style={{ flex: 1 }}/>
        <button onClick={onExit} title="Exit setup"
                style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.1em' }}>
          ESC
        </button>
      </div>

      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                    textTransform: 'uppercase'
}} className="mb-3" >Setup</div>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
        {stages.map((s, i) => {
          const isCur = i === stageIdx;
          const isDone = i < stageIdx;
          return (
            <button key={s.id}
                    onClick={() => setStageIdx(i)}
                    disabled={i > stageIdx}
                    style={{
                      display: 'grid',
                      gridTemplateColumns: '24px 1fr 14px', alignItems: 'center',
                      padding: isCur ? '10px 10px' : '7px 10px',
                      borderRadius: 6, textAlign: 'left',
                      background: isCur ? 'var(--paper)' : 'transparent',
                      border: isCur ? 'var(--hairline)' : '1px solid transparent',
                      color: isCur ? 'var(--ink)' : isDone ? 'var(--ink-2)' : 'var(--ink-4)',
                      cursor: i > stageIdx ? 'default' : 'pointer',
                      transition: 'all .14s'
}} className="gap-2" >
              {/* Always show the stage's kanji label so re-entering the
                  wizard reads as the same stepper, not a column of ✓s. */}
              <span className="kanji" style={{
                fontSize: 13, textAlign: 'center',
                color: isCur  ? 'var(--accent)'
                     : isDone ? 'var(--ink-2)'
                              : 'var(--ink-4)'
              }}>{s.n}</span>
              <div style={{ overflow: 'hidden' }}>
                <div style={{ fontSize: 13 }}>{s.title}</div>
                {isCur && (
                  <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    {s.sub}
                  </div>
                )}
              </div>
              {/* Completion tick lives on the right rail — kanji stays the
                  identity; tick is the status. */}
              <span style={{
                fontSize: 11, textAlign: 'center', lineHeight: 1,
                color: 'var(--success)',
                opacity: isDone ? 1 : 0,
                transition: 'opacity .14s'
              }}>✓</span>
            </button>
          );
        })}
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ borderTop: 'var(--hairline)' }} className="pt-3" >
        <ServicesStatus/>
      </div>
    </aside>
  );
}

// ─── Bottom bar ──────────────────────────────────────────────
function WizBottom({ stage, stageIdx, total, back, next, onDone, state }) {
  const isLast = stageIdx === total - 1;
  const isFirst = stageIdx === 0;
  const canAdvance = (() => {
    if (stage.id === "folders") return state.folders.length > 0;
    if (stage.id === "scan") return state.scan.done;
    if (stage.id === "preferences") {
      // Don't let the user step in nameless — sensei has to call you something.
      return !!(state.prefs && state.prefs.displayName && state.prefs.displayName.trim());
    }
    return true;
  })();

  return (
    <div style={{
 borderTop: 'var(--hairline)',
                  display: 'flex', alignItems: 'center',
                  background: 'var(--paper)'
}} className="gap-4 py-3 px-8" >
      <div style={{ fontSize: 11, letterSpacing: '0.12em', color: 'var(--ink-3)',
                    textTransform: 'uppercase' }}>
        {String(stageIdx + 1).padStart(2, "0")} <span style={{ color: 'var(--ink-4)' }}>/ {total}</span>
        <span style={{
 color: 'var(--ink-2)', textTransform: 'none',
                       letterSpacing: 0, fontSize: 13
}} className="ml-3" >{stage.title}</span>
      </div>

      {/* progress ticks */}
      <div style={{ flex: 1, display: 'flex', alignItems: 'center' }} className="gap-1" >
        {Array.from({ length: total }).map((_, i) => (
          <span key={i} style={{
            flex: 1, height: 2, borderRadius: 1,
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

      {isLast ? (
        <button onClick={onDone}
                style={{
 fontSize: 13, background: 'var(--ink)', color: 'var(--paper)', borderRadius: 6, letterSpacing: 0.2
}} className="py-2 px-5" >
          Enter observatory →
        </button>
      ) : (
        <button onClick={next} disabled={!canAdvance}
                style={{
 fontSize: 13,
                         background: canAdvance ? 'var(--ink)' : 'var(--edge)',
                         color: canAdvance ? 'var(--paper)' : 'var(--ink-3)', borderRadius: 6, letterSpacing: 0.2
}} className="py-2 px-5" >
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
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-2" >礼 · Welcome</div>
      <h1 className="display mt-0 mb-6" style={{
 fontSize: 56, fontWeight: 300, lineHeight: 1.08, letterSpacing: '-0.02em'
}}>
        A teacher does not<br/>
        <span style={{ color: 'var(--accent)' }}>write the code.</span>
      </h1>

      <p style={{
 fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.7, maxWidth: 560
}} className="mt-0 mb-5" >
        Sensei watches how you and your AI assistants work together — the sessions that
        completed cleanly, the ones that didn't, and the patterns underneath both.
      </p>

      <p style={{
 fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7, maxWidth: 560
}} className="mt-0 mb-7" >
        The next few minutes: install the local components, point to your folders, confirm
        what was found. Nothing leaves your machine.
      </p>

      <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', borderTop: 'var(--hairline)', borderBottom: 'var(--hairline)'
}} className="gap-4 py-5 px-0" >
        {[
          { k: "観", t: "Observe", s: "FTR · turns · corrections" },
          { k: "師", t: "Teach",   s: "patterns · rules · skills" },
          { k: "静", t: "Local",   s: "on your machine" }
        ].map(item => (
          <div key={item.k}>
            <div className="kanji mb-2" style={{ fontSize: 28, color: 'var(--accent)' }}>{item.k}</div>
            <div className="display" style={{ fontSize: 22 }}>{item.t}</div>
            <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-1" >{item.s}</div>
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
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-5" >
        <span style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                        color: 'var(--ink-4)' }}>demo · starting state</span>
        <div style={{
 display: 'flex', background: 'var(--paper-2)',
                       borderRadius: 5, border: 'var(--hairline)'
}} className="p-1 gap-0" >
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
                             color: state.components.variant === v.id ? 'var(--ink-2)' : 'var(--ink-3)',
                             border: 'none'
}} className="py-1 px-2" >
              {v.label}
            </button>
          ))}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
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
            <div key={c.id} style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', border: 'var(--hairline)', borderRadius: 8,
              background: 'var(--paper-2)', alignItems: 'center',
              transition: 'all .3s'
}} className="gap-4 py-4 px-4" >
              <div style={{ width: 36, height: 36, borderRadius: 6,
                             background: 'var(--paper)', border: 'var(--hairline)',
                             display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <span className="mono" style={{ fontSize: 13, color: 'var(--ink-2)' }}>
                  {c.id === "cli" ? "$" : c.id === "mcp" ? "⟷" : "◇"}
                </span>
              </div>
              <div>
                <div style={{ fontSize: 13 }}>{c.name}</div>
                <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {statusLabel}
                </div>
                {isBusy && (
                  <div style={{
 height: 2, background: 'var(--edge)', borderRadius: 1, overflow: 'hidden'
}} className="mt-2" >
                    <div style={{
                      height: '100%', width: '40%', background: 'var(--accent)',
                      animation: 'cSlide 1.2s ease-in-out infinite'
                    }}/>
                  </div>
                )}
              </div>
              <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
                <span style={{
                  width: 8, height: 8, borderRadius: '50%', background: dotColor,
                  boxShadow: phase !== "ready" ? `0 0 0 4px ${dotColor}22` : 'none',
                  animation: phase !== "ready" ? "cPulse 1.2s ease-in-out infinite" : 'none'
                }}/>
                <span className="mono" style={{ fontSize: 11,
                               color: phase === "ready" ? 'var(--success)' : 'var(--ink-2)',
                               letterSpacing: '0.06em', textTransform: 'uppercase' }}>
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

      <p style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7 }} className="mt-4" >
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
    <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
      <span style={{
        width: 8, height: 8, borderRadius: '50%', background: m.c,
        boxShadow: status === "working" ? `0 0 0 4px ${m.c}22` : 'none',
        animation: status === "working" ? "pulse 1.2s ease-in-out infinite" : 'none'
      }}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>{m.l}</span>
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
  return (
    <div style={{ maxWidth: 820 }} className="mx-auto" >
      <WizHeader n="連" title="Assistants" tagline="Registers plugins, skills, commands, agents, logging and metrics."/>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-3" >
        {D.acps.map(family => {
          const on = state.acps[family.id];
          const foundProducts = family.products.filter(p => p.found);
          const hasMultiple = family.products.length > 1;
          return (
            <button key={family.id}
                    disabled={!family.found}
                    onClick={() => upd({ acps: { ...state.acps, [family.id]: !on } })}
                    style={{
                      textAlign: 'left', borderRadius: 8,
                      border: on ? '1.5px solid var(--ink)' : 'var(--hairline)',
                      background: on ? 'var(--paper-2)' : 'var(--paper)',
                      opacity: family.found ? 1 : 0.55,
                      cursor: family.found ? 'pointer' : 'default',
                      display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                      alignItems: 'start'
}} className="py-4 px-4 gap-3" >
              {/* Family glyph */}
              <div style={{
                width: 36, height: 36, borderRadius: 6,
                background: 'var(--paper-2)', border: 'var(--hairline)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontSize: 15, fontWeight: 600, color: 'var(--ink)'
}} className="mt-1" >
                {family.kanji}
              </div>

              {/* Title + product chips */}
              <div style={{ minWidth: 0 }}>
                <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
                  <span style={{ fontSize: 15 }}>{family.name}</span>
                  {family.found && hasMultiple && (
                    <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                      {foundProducts.length} found
                    </span>
                  )}
                </div>

                {family.found ? (
                  <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1 mt-2" >
                    {family.products.map(p => (
                      <span key={p.id}
                            title={p.path || "not detected"}
                            style={{
                              display: 'inline-flex', alignItems: 'center', borderRadius: 3,
                              fontSize: 11,
                              background: p.found ? 'var(--paper)' : 'transparent',
                              border: p.found ? 'var(--hairline)' : '1px dashed var(--sumi-4, #d8d4cc)',
                              color: p.found ? 'var(--ink)' : 'var(--ink-3)'
}} className="gap-1 py-1 px-2" >
                        <span style={{
                          width: 5, height: 5, borderRadius: '50%',
                          background: p.found ? 'var(--jade, #6b8e7f)' : 'var(--sumi-4, #c8c4bc)'
                        }}/>
                        {p.label}
                        {p.version && <span className="mono" style={{ color: 'var(--ink-3)' }}>v{p.version}</span>}
                      </span>
                    ))}
                  </div>
                ) : (
                  <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    not installed
                  </div>
                )}
              </div>

              {/* Toggle */}
              <div style={{
                width: 22, height: 22, borderRadius: 4, flexShrink: 0,
                background: on ? 'var(--ink)' : 'transparent',
                border: on ? 'none' : 'var(--ink-line)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                color: 'var(--paper)'
}} className="mt-2" >
                {on && <svg width="12" height="12" viewBox="0 0 16 16"><path d="M3 8 L7 12 L13 4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>}
              </div>
            </button>
          );
        })}
      </div>

      <div style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7 }} className="mt-4" >
        Connecting a family registers sensei across every product it finds — Code &amp; Desktop, CLI &amp; app. You can disable individual products from <span className="mono">Settings → Assistants</span>.
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

      <div style={{ display: 'flex' }} className="gap-2 mb-4" >
        <input value={state.newFolder}
               onChange={e => upd({ newFolder: e.target.value })}
               onKeyDown={e => e.key === "Enter" && add()}
               placeholder="~/code/my-project"
               className="mono py-2 px-3"
               style={{
                 flex: 1, fontSize: 13,
                 background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6,
                 outline: 'none'
}}/>
        <button onClick={add}
                style={{
 fontSize: 13, borderRadius: 6,
                         background: 'var(--ink)', color: 'var(--paper)'
}} className="py-2 px-4" >
          Add
        </button>
        <button style={{
 fontSize: 13, borderRadius: 6,
                          border: 'var(--ink-line)'
}} className="py-2 px-4" >
          Browse…
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
        {state.folders.map(f => (
          <div key={f.id} style={{
            display: 'grid', gridTemplateColumns: 'auto 1fr auto auto', border: 'var(--hairline)', borderRadius: 6,
            background: 'var(--paper-2)', alignItems: 'center'
}} className="gap-3 py-3 px-4" >
            <span style={{ color: 'var(--ink-3)', fontSize: 13 }}>▸</span>
            <div>
              <div className="mono" style={{ fontSize: 13 }}>{f.path}</div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{f.note}</div>
            </div>
            <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-3)', background: 'var(--paper)',
                          borderRadius: 3, border: 'var(--hairline)'
}}>recursive</span>
            <button onClick={() => remove(f.id)}
                    style={{ fontSize: 11, color: 'var(--ink-3)' }} className="py-1 px-2" >
              remove
            </button>
          </div>
        ))}
      </div>

      <div style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7 }} className="mt-4" >
        You can manage folders and exclusions later from <span className="mono">Settings</span>.
      </div>
    </div>
  );
}

// ─── 5 Scan (SSE-style live view) ────────────────────────────
function WizScan({ state, upd }) {
  const D = window.SENSEI_SETUP;
  const [tick, setTick] = useS(0);
  const { started, done } = state.scan;

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
    return (
      <div style={{ maxWidth: 820 }} className="mx-auto" >
        <WizHeader n="観" title="Scan" tagline={`Ready to scan ${state.folders.length} ${state.folders.length === 1 ? "root" : "roots"}.`}/>
        <div style={{
 background: 'var(--paper-2)',
                       borderRadius: 10, border: 'var(--hairline)', textAlign: 'center'
}} className="py-7 px-6" >
          <div className="kanji mb-4" style={{
 fontSize: 56, color: 'var(--accent)', opacity: 0.4
}}>探</div>
          <p style={{
 fontSize: 15, color: 'var(--ink-2)',
                       lineHeight: 1.6, maxWidth: 440
}} className="mt-0 mb-4 mx-auto" >
            The daemon will recurse your folders, identify repositories, and extract the
            code graph. Two workers, ~2M files / minute on this machine.
          </p>
          <button onClick={start}
                  style={{
 fontSize: 13, background: 'var(--ink)',
                           color: 'var(--paper)', borderRadius: 6
}} className="py-3 px-5" >
            Begin scan →
          </button>
        </div>
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
      <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)',
                     border: 'var(--hairline)', borderRadius: 8, background: 'var(--paper-2)', overflow: 'hidden'
}} className="mb-4 gap-0" >
        <ScanStat label="Roots"      value={state.folders.length}/>
        <ScanStat label="Discovered" value={stats.discovered}/>
        <ScanStat label="Queued"     value={stats.queued}/>
        <ScanStat label="Processed"  value={stats.processed} accent/>
      </div>

      {/* Progress line */}
      <div style={{
 height: 2, background: 'var(--edge)', borderRadius: 1, overflow: 'hidden'
}} className="mb-5" >
        <div style={{ height: '100%', width: `${progress * 100}%`,
                       background: done ? 'var(--success)' : 'var(--ink)',
                       transition: 'width 80ms linear' }}/>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', alignItems: 'start' }} className="gap-4" >
        {/* ─── Left: solutions + repos materializing ─── */}
        <div style={{ display: 'flex', flexDirection: 'column', minHeight: 360 }} className="gap-3" >
          {discoveredSolutions.length === 0 && (
            <div style={{
 border: '1px dashed var(--edge)',
                           borderRadius: 10, textAlign: 'center', color: 'var(--ink-4)',
                           fontSize: 13, fontStyle: 'italic'
}} className="p-6" >
              <div className="kanji mb-2" style={{
 fontSize: 40, color: 'var(--accent)',
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
        <div style={{
          border: 'var(--hairline)', borderRadius: 8, background: 'var(--paper-2)',
          overflow: 'hidden', position: 'sticky', top: 0
        }}>
          <div style={{
 borderBottom: 'var(--hairline)',
                         fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                         textTransform: 'uppercase', display: 'flex',
                         alignItems: 'center'
}} className="py-2 px-3 gap-2" >
            <span style={{
              width: 6, height: 6, borderRadius: '50%',
              background: done ? 'var(--success)' : 'var(--accent)',
              animation: done ? 'none' : 'pulseSm 1.2s ease-in-out infinite'
            }}/>
            <span>SSE · /events</span>
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ textTransform: 'none', letterSpacing: 0 }}>
              {(tick/1000).toFixed(1)}s
            </span>
          </div>
          <div style={{ height: 360, overflow: 'auto' }} className="py-2 px-3" >
            {events.slice().reverse().map((e, i) => (
              <div key={e.t} style={{
                display: 'grid', gridTemplateColumns: '42px 60px 1fr', fontSize: 11,
                color: i === 0 ? 'var(--ink)' : 'var(--ink-2)',
                opacity: i === 0 ? 1 : Math.max(0.28, 1 - i * 0.07),
                animation: i === 0 ? 'eventIn .26s ease-out' : 'none'
}} className="gap-2 py-1 px-0" >
                <span className="mono" style={{ color: 'var(--ink-3)' }}>+{(e.t/1000).toFixed(2)}s</span>
                <span className="mono" style={{
                  color: e.level === "success"  ? 'var(--success)' :
                         e.level === "discover" ? 'var(--accent)'  :
                         e.level === "process"  ? 'var(--ink)' :
                         e.level === "queue"    ? 'var(--warning)' : 'var(--ink-3)'
                }}>
                  {e.level}
                </span>
                <span className="mono" style={{ overflow: 'hidden',
                               textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.msg}</span>
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
 borderRadius: 8,
                       background: 'var(--success-soft)', fontSize: 13, color: 'var(--ink)',
                       display: 'flex', alignItems: 'center'
}} className="mt-4 py-3 px-4 gap-2" >
          <span className="kanji" style={{ fontSize: 17, color: 'var(--success)' }}>✓</span>
          8 repos indexed across 3 roots · graph extracted · you may continue.
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
      borderRadius: 10, background: 'var(--paper-2)',
      animation: 'cardIn .34s ease-out',
      transition: 'border .3s'
}} className="py-4 px-4" >
      <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', alignItems: 'center' }} className="gap-3" >
        <div className="kanji" style={{
          fontSize: 22, width: 38, height: 38, borderRadius: '50%',
          background: 'var(--paper)', border: 'var(--hairline)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          color: allDone ? 'var(--accent)' : 'var(--ink-3)',
          transition: 'color .3s'
        }}>{sol.kanji}</div>
        <div style={{ minWidth: 0 }}>
          <div className="display" style={{ fontSize: 17 }}>{sol.name}</div>
          <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {sol.path} · {discoveredRepos.length} {discoveredRepos.length === 1 ? "repo" : "repos"}
            {!allDone && discoveredRepos.length > 0 && ` · ${doneCount} ready`}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div className="mono" style={{ fontSize: 11,
                         color: allDone ? 'var(--success)' : 'var(--ink-3)',
                         letterSpacing: '0.1em', textTransform: 'uppercase' }}>
            {allDone ? "ready" : solState.state}
          </div>
          {totalFiles > 0 && !allDone && (
            <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
              {processedFiles.toLocaleString()} / {totalFiles.toLocaleString()}
            </div>
          )}
        </div>
      </div>

      {/* Repos list */}
      {discoveredRepos.length > 0 && (
        <div style={{
                       display: 'flex', flexDirection: 'column'
}} className="mt-3 gap-1 pl-7" >
          {discoveredRepos.map(([p, rs]) => {
            const pct = rs.totalFiles > 0 ? rs.processed / rs.totalFiles : 0;
            const isDone = rs.state === "done";
            const isProcessing = rs.state === "processing";
            return (
              <div key={p.id} style={{
                display: 'grid', gridTemplateColumns: '140px 1fr 70px', alignItems: 'center',
                animation: 'repoIn .26s ease-out'
}} className="gap-3" >
                <div style={{ display: 'flex', alignItems: 'center' }} className="gap-1" >
                  <span style={{
                    width: 5, height: 5, borderRadius: '50%',
                    background: isDone ? 'var(--success)' :
                                isProcessing ? 'var(--accent)' :
                                rs.state === "queued" ? 'var(--warning)' : 'var(--ink-4)',
                    animation: isProcessing ? 'pulseSm 1.2s ease-in-out infinite' : 'none'
                  }}/>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)',
                                overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {p.name}
                  </span>
                </div>
                {/* progress track */}
                <div style={{ height: 2, background: 'var(--edge)', borderRadius: 1,
                               overflow: 'hidden', position: 'relative' }}>
                  <div style={{
                    position: 'absolute', inset: 0, width: `${pct * 100}%`,
                    background: isDone ? 'var(--success)' : 'var(--ink)',
                    transition: 'width .3s ease-out'
                  }}/>
                  {isProcessing && (
                    <div style={{
                      position: 'absolute', inset: 0, width: `${pct * 100}%`,
                      background: 'linear-gradient(90deg, transparent, var(--paper) 50%, transparent)',
                      backgroundSize: '80px 100%',
                      animation: 'shimmer 1.4s linear infinite',
                      mixBlendMode: 'overlay'
                    }}/>
                  )}
                </div>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                                textAlign: 'right' }}>
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
    <div style={{ borderRight: 'var(--hairline)' }} className="py-4 px-4" >
      <div className="display" style={{ fontSize: 28, fontWeight: 400,
                     color: accent ? 'var(--accent)' : 'var(--ink)' }}>{value}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-3)'
}} className="mt-1" >{label}</div>
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

      <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mb-4" >
        A single-repo project is the default. Multi-repo projects are auto-grouped from sibling folders and name patterns. Split when they shouldn't be together.
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
        {sols.map(s => {
          const isMulti = s.projects.length > 1;
          const isExpanded = selected === s.id;
          const isMergeOpen = mergeMenu === s.id;
          const mergeTargets = sols.filter(x => x.id !== s.id);
          return (
          <div key={s.id} style={{
            border: 'var(--hairline)',
            borderRadius: 10, background: s.confirmed ? 'var(--paper-2)' : 'var(--paper)', opacity: s.confirmed ? 1 : 0.55, transition: 'all .2s',
            position: 'relative'
}} className="p-4" >
            <div style={{
 display: 'grid',
                           gridTemplateColumns: 'auto 1fr auto auto auto auto', alignItems: 'center'
}} className="gap-3" >
              <div className="kanji" style={{
                fontSize: 28, color: 'var(--accent)',
                width: 42, height: 42, borderRadius: '50%',
                background: 'var(--paper)', border: 'var(--hairline)',
                display: 'flex', alignItems: 'center', justifyContent: 'center'
              }}>{s.kanji}</div>
              <div>
                <input value={s.renamed ?? s.name}
                       onChange={e => rename(s.id, e.target.value)}
                       className="display p-0"
                       style={{
                         fontSize: 22, fontWeight: 400, background: 'transparent',
                         border: 'none', outline: 'none',
                         borderBottom: '1px dashed transparent', width: '100%'
}}
                       onFocus={e => e.target.style.borderBottom = '1px dashed var(--ink-3)'}
                       onBlur={e => e.target.style.borderBottom = '1px dashed transparent'}/>
                <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {s.path} · {s.projects.length} {s.projects.length === 1 ? "repo" : "repos"}
                </div>
              </div>
              {isMulti ? (
                <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--accent)',
                              letterSpacing: '0.1em', textTransform: 'uppercase', border: '1px solid var(--accent)',
                              borderRadius: 3, background: 'var(--accent-soft)'
}}>
                  multi-repo
                </span>
              ) : <span/>}

              {/* merge button (only shown when there's another project to merge with) */}
              {mergeTargets.length > 0 ? (
                <button onClick={() => { setMergeMenu(isMergeOpen ? null : s.id); setRepoMenu(null); }}
                        style={{
 fontSize: 11, color: 'var(--ink-3)',
                                  borderRadius: 4
}} className="py-1 px-2" >
                  merge…
                </button>
              ) : <span/>}

              <button onClick={() => setSelected(isExpanded ? null : s.id)}
                      style={{
 fontSize: 11, color: 'var(--ink-3)',
                                borderRadius: 4, display: 'flex', alignItems: 'center'
}} className="py-1 px-2 gap-1" >
                {isExpanded ? "hide" : "edit"}
                <span style={{ fontSize: 11, transform: isExpanded ? 'rotate(180deg)' : 'none',
                                transition: 'transform .2s' }}>▾</span>
              </button>
              <button onClick={() => toggle(s.id)}
                      style={{
                        width: 22, height: 22, borderRadius: 4,
                        background: s.confirmed ? 'var(--ink)' : 'transparent',
                        border: s.confirmed ? 'none' : 'var(--ink-line)',
                        color: 'var(--paper)',
                        display: 'flex', alignItems: 'center', justifyContent: 'center'
                      }}>
                {s.confirmed && <svg width="12" height="12" viewBox="0 0 16 16"><path d="M3 8 L7 12 L13 4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>}
              </button>
            </div>

            {/* Merge target picker */}
            {isMergeOpen && (
              <div style={{
                background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 6,
                animation: 'expandIn .2s ease-out'
}} className="mt-3 py-3 px-3 pl-7" >
                <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                               textTransform: 'uppercase'
}} className="mb-2" >
                  Merge {s.renamed ?? s.name} into…
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
                  {mergeTargets.map(t => (
                    <button key={t.id} onClick={() => mergeInto(s.id, t.id)}
                            className="mono py-1 px-2 gap-1" style={{
                              fontSize: 11, borderRadius: 4,
                              border: 'var(--hairline)', background: 'var(--paper-2)',
                              color: 'var(--ink-2)', display: 'inline-flex',
                              alignItems: 'center'
}}>
                      <span className="kanji" style={{ color: 'var(--accent)', fontSize: 13 }}>{t.kanji}</span>
                      {t.renamed ?? t.name}
                      <span style={{ color: 'var(--ink-4)', fontSize: 11 }}>({t.projects.length})</span>
                    </button>
                  ))}
                  <button onClick={() => setMergeMenu(null)} className="mono py-1 px-2" style={{
                    fontSize: 11, color: 'var(--ink-3)'
}}>cancel</button>
                </div>
              </div>
            )}

            {/* Repo chips — compact, always visible */}
            <div style={{
 display: 'flex', flexWrap: 'wrap'
}} className="mt-3 gap-1 pl-7" >
              {s.projects.map(p => {
                const role = D.roles.find(r => r.id === state.roles[p.id]);
                const isOpen = repoMenu && repoMenu.sid === s.id && repoMenu.pid === p.id;
                const moveTargets = sols.filter(x => x.id !== s.id);
                return (
                  <span key={p.id} style={{ position: 'relative', display: 'inline-block' }}>
                    <span className="mono gap-2 py-1 pl-2 pr-1" style={{
                      fontSize: 11,
                      background: 'var(--paper)', border: 'var(--hairline)',
                      borderRadius: 3, color: 'var(--ink-2)',
                      display: 'inline-flex', alignItems: 'center'
}}>
                      {p.name}
                      <span style={{ color: 'var(--ink-4)' }}>{p.files}f</span>
                      {role && (
                        <span style={{
 color: 'var(--accent)', fontSize: 11,
                                        borderLeft: '1px solid var(--edge)'
}} className="pl-2" >
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
 fontSize: 13, color: 'var(--ink-3)',
                                borderLeft: '1px solid var(--edge)',
                                lineHeight: 1
}} className="px-1 ml-1" >
                        ⋯
                      </button>
                    </span>
                    {isOpen && (
                      <div style={{
                        position: 'absolute', top: 'calc(100% + 4px)', left: 0, zIndex: 10,
                        minWidth: 220,
                        background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 6,
                        boxShadow: '0 4px 16px rgba(0,0,0,0.08)',
                        animation: 'expandIn .15s ease-out'
}} className="p-1" >
                        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                                       textTransform: 'uppercase'
}} className="py-1 px-2" >
                          Move {p.name} to…
                        </div>
                        {moveTargets.map(t => (
                          <button key={t.id} onClick={() => moveRepo(s.id, p.id, t.id)}
                                  className="mono py-2 px-2 gap-2" style={{
                                    display: 'flex', width: '100%',
                                    fontSize: 11, color: 'var(--ink-2)', borderRadius: 3,
                                    alignItems: 'center', textAlign: 'left'
}}>
                            <span className="kanji" style={{ color: 'var(--accent)' }}>{t.kanji}</span>
                            {t.renamed ?? t.name}
                          </button>
                        ))}
                        {isMulti && (
                          <button onClick={() => moveRepo(s.id, p.id, "__new__")}
                                  style={{
                                    display: 'flex', width: '100%',
                                    fontSize: 11, color: 'var(--ink-3)', borderRadius: 3,
                                    borderTop: 'var(--hairline)',
                                    textAlign: 'left'
}} className="py-2 px-2 mt-1 pt-2" >
                            + split out as new project
                          </button>
                        )}
                      </div>
                    )}
                  </span>
                );
              })}
              {isMulti && (
                <button onClick={() => split(s.id)} className="mono py-1 px-2" style={{
                  fontSize: 11, color: 'var(--ink-3)',
                  border: '1px dashed var(--edge)', borderRadius: 3
}}>
                  split all into {s.projects.length} projects
                </button>
              )}
            </div>

            {/* Expanded: rename + role picker per repo */}
            {isExpanded && (
              <div style={{
                borderTop: 'var(--hairline)',
                display: 'flex', flexDirection: 'column',
                animation: 'expandIn .22s ease-out'
}} className="mt-4 gap-1 pt-3 pl-7" >
                <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                               textTransform: 'uppercase'
}} className="mb-1" >
                  Repo roles
                </div>
                {s.projects.map(p => (
                  <div key={p.id} style={{
                    display: 'grid', gridTemplateColumns: '1fr auto', alignItems: 'center'
}} className="gap-3 py-2 px-0" >
                    <div>
                      <div className="mono" style={{ fontSize: 13, color: 'var(--ink-2)' }}>{p.name}</div>
                      <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
                        {p.lang} · {p.files} files
                      </div>
                    </div>
                    <div style={{
 display: 'flex',
                                   background: 'var(--paper)', borderRadius: 4
}} className="gap-1 p-1" >
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

      <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-4" >
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

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-4" >
        {state.solutions.filter(s => s.confirmed).map(s => (
          <div key={s.id} style={{
 border: 'var(--hairline)', borderRadius: 10,
            background: 'var(--paper-2)'
}} className="p-5" >
            <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-4" >
              <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>{s.kanji}</span>
              <span className="display" style={{ fontSize: 22 }}>{s.renamed ?? s.name}</span>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-4" >
              <MetaField label="Stage">
                <div style={{
 display: 'flex',
                               background: 'var(--paper)', borderRadius: 5
}} className="gap-1 p-1" >
                  {D.metadata.statuses.map(st => (
                    <button key={st.id}
                            onClick={() => setMeta(s.id, "status", st.id)}
                            style={{
                              flex: 1, fontSize: 11, borderRadius: 3,
                              background: state.metadata[s.id].status === st.id ? 'var(--ink)' : 'transparent',
                              color: state.metadata[s.id].status === st.id ? 'var(--paper)' : 'var(--ink-3)'
}} className="py-1 px-2" >
                      {st.label}
                    </button>
                  ))}
                </div>
              </MetaField>

              <MetaField label="Client (optional)">
                <input value={state.metadata[s.id].client}
                       onChange={e => setMeta(s.id, "client", e.target.value)}
                       placeholder="e.g. Internal"
                       style={{
                         width: '100%', fontSize: 13,
                         background: 'var(--paper)', border: 'var(--hairline)',
                         borderRadius: 5, outline: 'none'
}} className="py-2 px-3" />
              </MetaField>

              <div style={{ gridColumn: 'span 2' }}>
                <MetaField label="Goal">
                  <input value={state.metadata[s.id].goal}
                         onChange={e => setMeta(s.id, "goal", e.target.value)}
                         placeholder="One sentence. Why this exists."
                         style={{
                           width: '100%', fontSize: 13,
                           background: 'var(--paper)', border: 'var(--hairline)',
                           borderRadius: 5, outline: 'none'
}} className="py-2 px-3" />
                </MetaField>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-4" >
        Skip if you like. These can be edited per-solution from the Coaching page.
      </div>
    </div>
  );
}

function MetaField({ label, children }) {
  return (
    <div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-3)'
}} className="mb-1" >{label}</div>
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
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-3 mb-4" >
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-2)', background: 'var(--paper-2)',
                      border: 'var(--hairline)', borderRadius: 4
}}>
          {D.detected.length} detected
        </span>
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--success)', background: 'var(--success-soft)',
                      borderRadius: 4
}}>
          {activeCount} will be wrapped
        </span>
        {state.libExtras.length > 0 && (
          <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--accent)', background: 'var(--accent-soft)',
                        borderRadius: 4
}}>
            {state.libExtras.length} added by you
          </span>
        )}
      </div>

      {/* Detected libraries */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase'
}} className="mb-2" >
        Detected · sensei will wrap
      </div>
      <div style={{
 display: 'flex', flexDirection: 'column',
                     border: 'var(--hairline)', borderRadius: 6,
                     background: 'var(--paper-2)'
}} className="mb-5" >
        {D.detected.map((lib, i) => {
          const on = !!state.libraries[lib.id];
          return (
            <div key={lib.id}
                 style={{
 display: 'grid',
                           gridTemplateColumns: 'auto 1fr auto auto auto', alignItems: 'center',
                           borderBottom: i < D.detected.length - 1 ? 'var(--hairline)' : 'none',
                           opacity: on ? 1 : 0.45
}} className="gap-3 py-3 px-3" >
              <button onClick={() => toggle(lib.id)}
                      style={{ width: 18, height: 18, borderRadius: 3,
                                border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
                                background: on ? 'var(--accent)' : 'transparent',
                                color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
                {on ? "✓" : ""}
              </button>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>
                  {lib.name}
                  <span className="mono ml-2" style={{
 fontSize: 11, color: 'var(--ink-4)'
}}>{lib.version}</span>
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
                  {lib.why}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {lib.lang}
              </span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
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
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-2" >
            Added by you
          </div>
          <div style={{
 display: 'flex', flexDirection: 'column',
                         border: 'var(--hairline)', borderRadius: 6,
                         background: 'var(--paper-2)'
}} className="mb-5" >
            {state.libExtras.map((lib, i) => {
              const on = lib.on;
              return (
                <div key={lib.id}
                     style={{
 display: 'grid',
                               gridTemplateColumns: 'auto 1fr auto auto', alignItems: 'center',
                               borderBottom: i < state.libExtras.length - 1 ? 'var(--hairline)' : 'none',
                               opacity: on ? 1 : 0.45
}} className="gap-3 py-3 px-3" >
                  <button onClick={() => toggleExtra(lib.id)}
                          style={{ width: 18, height: 18, borderRadius: 3,
                                    border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
                                    background: on ? 'var(--accent)' : 'transparent',
                                    color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
                    {on ? "✓" : ""}
                  </button>
                  <div>
                    <div style={{ fontSize: 13, color: 'var(--ink)' }}>{lib.name}</div>
                    <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                      {lib.url || "no URL"}
                    </div>
                  </div>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    {lib.lang}
                  </span>
                  <button onClick={() => removeExtra(lib.id)}
                          style={{ fontSize: 11, color: 'var(--ink-4)' }}>remove</button>
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
                          background: 'var(--paper-2)',
                          border: '1px dashed var(--ink-4)', borderRadius: 6,
                          color: 'var(--ink-2)'
}} className="py-2 px-4" >
          + Add a library
        </button>
      ) : (
        <div style={{
 background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 8,
                       maxWidth: 640
}} className="py-4 px-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-3" >
            Add a library sensei should wrap
          </div>
          <div style={{
 display: 'grid', gridTemplateColumns: '1.2fr 1fr 0.5fr'
}} className="gap-2 mb-3" >
            <div>
              <div style={{ fontSize: 11, color: 'var(--ink-2)' }} className="mb-1" >Name</div>
              <input value={form.name}
                     onChange={e => setForm({ ...form, name: e.target.value })}
                     placeholder="e.g. @internal/fx"
                     style={wizInputStyle} className="mb-1" />
            </div>
            <div>
              <div style={{ fontSize: 11, color: 'var(--ink-2)' }}>
                Docs URL <span style={{ color: 'var(--ink-4)' }}>· optional</span>
              </div>
              <input value={form.url}
                     onChange={e => setForm({ ...form, url: e.target.value })}
                     placeholder="https://docs.rs/… or internal wiki"
                     style={wizInputStyle} className="mb-1" />
            </div>
            <div>
              <div style={{ fontSize: 11, color: 'var(--ink-2)' }}>Lang</div>
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
          <div style={{ display: 'flex' }}>
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
 fontSize: 13,
                              color: 'var(--ink-3)'
}} className="py-2 px-3" >
              Cancel
            </button>
            <span style={{ flex: 1 }}/>
            <span style={{ fontSize: 11, color: 'var(--ink-3)', alignSelf: 'center' }}>
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
      <div style={{
 background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 8
}} className="py-3 px-4 mb-5" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-2" >
          Detected in your stack
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
          {[...stack.languages, ...stack.frameworks, ...stack.services].map(s => (
            <span key={s} className="mono py-1 px-2" style={{
 fontSize: 11, background: 'var(--paper)',
                          border: 'var(--hairline)', borderRadius: 3,
                          color: 'var(--ink-2)'
}}>
              {s}
            </span>
          ))}
        </div>
      </div>

      {/* Summary row */}
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-3 mb-4" >
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--success)', background: 'var(--success-soft)',
                      borderRadius: 4
}}>
          {recommended.length} recommended
        </span>
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-2)', background: 'var(--paper-2)',
                      border: 'var(--hairline)', borderRadius: 4
}}>
          {installCount} will be installed
        </span>
      </div>

      {/* Recommended */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase'
}} className="mb-2" >
        Recommended for your stack
      </div>
      <div style={{
 display: 'flex', flexDirection: 'column',
                     border: 'var(--hairline)', borderRadius: 6,
                     background: 'var(--paper-2)'
}} className="mb-5" >
        {recommended.map((mcp, i) => (
          <McpRow key={mcp.id} mcp={mcp} on={!!state.mcps[mcp.id]}
                  onToggle={() => toggle(mcp.id)}
                  last={i === recommended.length - 1}/>
        ))}
      </div>

      {/* Available */}
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase'
}} className="mb-2" >
        Also available
      </div>
      <div style={{ display: 'flex', flexDirection: 'column',
                     border: 'var(--hairline)', borderRadius: 6,
                     background: 'var(--paper-2)' }}>
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
 display: 'grid',
                   gridTemplateColumns: 'auto auto 1fr auto auto', alignItems: 'center',
                   borderBottom: last ? 'none' : 'var(--hairline)',
                   opacity: on ? 1 : 0.55
}} className="gap-3 py-3 px-3" >
      <button onClick={onToggle}
              style={{ width: 18, height: 18, borderRadius: 3,
                        border: '1.5px solid ' + (on ? 'var(--accent)' : 'var(--ink-4)'),
                        background: on ? 'var(--accent)' : 'transparent',
                        color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
        {on ? "✓" : ""}
      </button>
      <div style={{ width: 32, height: 32, borderRadius: 6,
                     background: 'var(--paper-3)', display: 'flex',
                     alignItems: 'center', justifyContent: 'center' }}>
        <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>
          {mcp.kanji}
        </span>
      </div>
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>
          {mcp.name}
          <span className="mono ml-2" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            by {mcp.publisher}
          </span>
          {mcp.verified && (
            <span className="mono ml-2 py-1 px-1" style={{
 fontSize: 11, color: 'var(--success)',
                          background: 'var(--success-soft)', borderRadius: 3
}}>
              verified
            </span>
          )}
        </div>
        <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                       lineHeight: 1.45
}} className="mt-1" >
          {mcp.summary}
        </div>
      </div>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
        {mcp.tools} tools
      </span>
      {mcp.trigger && mcp.trigger.length > 0 ? (
        <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--success)', background: 'var(--success-soft)',
                      borderRadius: 3, whiteSpace: 'nowrap'
}}>
          matches {mcp.trigger[0]}
        </span>
      ) : (
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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
    <span className="mono py-1 px-2" style={{
 fontSize: 11, borderRadius: 3,
                background: m.bg, color: m.tone, whiteSpace: 'nowrap'
}}>
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
    <section className="pt-5 pb-1" >
      <header style={{
 display: 'flex', alignItems: 'baseline',
                        marginBottom: right ? 0 : 14
}} className="gap-3" >
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)',
                                           lineHeight: 1, width: 30 }}>{kanji}</span>
        <div style={{ flex: 1, minWidth: 0 }}>
          <h3 className="display m-0" style={{
 fontSize: 17, fontWeight: 400,
                          color: 'var(--ink)'
}}>{title}</h3>
          {sub && (
            <p style={{
 fontSize: 13, color: 'var(--ink-3)',
                          maxWidth: 540, lineHeight: 1.5
}} className="mt-1 mb-0" >{sub}</p>
          )}
        </div>
        {right && (
          <div style={{ flexShrink: 0, alignSelf: 'center', minWidth: 220 }}>
            {right}
          </div>
        )}
      </header>
      {!right && <div className="divide-y pl-7">{children}</div>}
    </section>
  );
  const Row = ({ label, hint, children }) => (
    <div style={{
 display: 'grid', gridTemplateColumns: '1fr auto',
                   alignItems: 'center'
}} className="gap-6 py-3 px-0" >
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{label}</div>
        {hint && <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                                 lineHeight: 1.45, maxWidth: 460
}} className="mt-1" >{hint}</div>}
      </div>
      <div style={{ flexShrink: 0 }}>{children}</div>
    </div>
  );
  const Toggle = ({ value, onChange }) => (
    <button onClick={() => onChange(!value)}
            style={{
 width: 36, height: 20, borderRadius: 999,
                      background: value ? 'var(--accent)' : 'var(--edge)',
                      position: 'relative', cursor: 'pointer',
                      transition: 'background 0.15s',
                      border: 'none'
}} className="p-0" >
      <span style={{ position: 'absolute', top: 2, left: value ? 18 : 2,
                       width: 16, height: 16, borderRadius: '50%',
                       background: 'var(--paper)',
                       transition: 'left 0.18s ease',
                       boxShadow: '0 1px 3px rgba(0,0,0,0.18)' }}/>
    </button>
  );
  const Segment = ({ value, onChange, options }) => (
    <div style={{ display: 'inline-flex', border: 'var(--hairline)',
                   borderRadius: 5, overflow: 'hidden' }}>
      {options.map((opt, i) => (
        <button key={opt.value} onClick={() => onChange(opt.value)}
                style={{
 fontSize: 11,
                          borderLeft: i === 0 ? 'none' : 'var(--hairline)',
                          background: value === opt.value ? 'var(--paper-3)' : 'var(--paper)',
                          color: value === opt.value ? 'var(--ink)' : 'var(--ink-3)',
                          cursor: 'pointer'
}} className="py-1 px-3" >
          {opt.label}
        </button>
      ))}
    </div>
  );
  const Sel = ({ value, onChange, options }) => (
    <select value={value} onChange={e => onChange(e.target.value)}
            style={{
 fontSize: 13, border: 'var(--hairline)',
                      borderRadius: 5, background: 'var(--paper)',
                      color: 'var(--ink)', cursor: 'pointer',
                      fontFamily: 'inherit'
}} className="py-1 px-2" >
      {options.map(o =>
        <option key={o.value} value={o.value}>{o.label}</option>)}
    </select>
  );

  return (
    <div style={{ maxWidth: 760 }} className="mx-auto" >
      <WizHeader n="名" title="Preferences"
                 tagline="A few small choices before you step in. Anything here can be changed later by re-opening this wizard."/>

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
 width: 240, fontSize: 13,
                             border: 'var(--hairline)', borderRadius: 5,
                             background: 'var(--paper)', color: 'var(--ink)',
                             fontFamily: 'inherit', outline: 'none',
                             textAlign: 'right'
}}
                   onFocus={e => e.target.style.borderColor = 'var(--accent)'}
                   onBlur={e => e.target.style.borderColor = ''}
                 className="py-2 px-3" />
               }/>

      {/* ── Shared learnings ─────────────────────────────────────── */}
      <Section kanji="共" title="Shared learnings"
               sub="Sensei can contribute the patterns it finds to a collective pool — and pull what others have learned back into your library.">
        <Row label="Contribute my learnings"
             hint="Anonymized patterns only. No code, no commit messages, no project names.">
          <Toggle value={p.contributeLearnings}
                   onChange={v => setP({ contributeLearnings: v })}/>
        </Row>
        <Row label="Review before sharing"
             hint="Each learning shows up in a queue for your approval before it leaves your machine.">
          <Toggle value={p.reviewBeforeShare}
                   onChange={v => setP({ reviewBeforeShare: v })}/>
        </Row>
        <Row label="Sharing schedule">
          <Sel value={p.shareSchedule}
                onChange={v => setP({ shareSchedule: v })}
                options={[
                  { value: "off",             label: "Off · manual only" },
                  { value: "daily",           label: "Daily" },
                  { value: "weekly-saturday", label: "Every Saturday" },
                  { value: "monthly",         label: "Monthly" }
                ]}/>
        </Row>
        <Row label="Download collective learnings"
             hint="Reviewed before they enter your library.">
          <Sel value={p.downloadCollective}
                onChange={v => setP({ downloadCollective: v })}
                options={[
                  { value: "never",     label: "Never" },
                  { value: "weekly",    label: "Weekly" },
                  { value: "daily",     label: "Daily" },
                  { value: "on-demand", label: "On demand" }
                ]}/>
        </Row>
      </Section>

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
 fontSize: 13, color: 'var(--ink-3)',
                   fontStyle: 'italic', lineHeight: 1.6, textAlign: 'center'
}} className="mt-5 mb-0" >
        These save when you press <span style={{ color: 'var(--ink-2)' }}>Enter observatory</span>.
        Re-open this wizard from the sidebar's <span style={{ color: 'var(--ink-2)' }}>調 Configure</span> link to change them later.
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
    <div style={{ maxWidth: 680, textAlign: 'center' }} className="mt-4 mb-0 mx-auto" >
      <div className="kanji mb-2" style={{ fontSize: 56, color: 'var(--accent)' }}>観</div>
      <h1 className="display mt-0 mb-4" style={{
 fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em'
}}>
        The observatory is ready.
      </h1>
      <p style={{
 fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.6, maxWidth: 480
}} className="mt-0 mb-6 mx-auto" >
        Start a session with your assistant. Sensei will watch in silence for a few days,
        then begin to teach.
      </p>

      <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)',
                     border: 'var(--hairline)', borderRadius: 10,
                     background: 'var(--paper-2)', overflow: 'hidden',
                     textAlign: 'left'
}} className="gap-0" >
        <DoneStat label="Projects"   value={confirmedSols.length}/>
        <DoneStat label="Repos"      value={repoCount}/>
        <DoneStat label="Libraries"  value={libCount}/>
        <DoneStat label="MCPs"       value={mcpCount}/>
        <DoneStat label="Assistants" value={activeAcps} last/>
      </div>

      <p className="mono mt-6" style={{
 fontSize: 11, color: 'var(--ink-3)', fontStyle: 'italic'
}}>
        師 · the first session is always the teacher
      </p>
    </div>
  );
}

function DoneStat({ label, value, last }) {
  return (
    <div style={{ borderRight: last ? 'none' : 'var(--hairline)' }} className="py-4 px-4" >
      <div className="display" style={{ fontSize: 28, fontWeight: 400 }}>{value}</div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-3)'
}} className="mt-1" >{label}</div>
    </div>
  );
}

// ─── Shared: step header ─────────────────────────────────────
function WizHeader({ n, title, tagline }) {
  // Sticky to the top of the stage's scroll container so the step title +
  // tagline stay anchored as the user scrolls long stages.
  return (
    <div className="mb-5 pt-1 pb-4"
         style={{
           position: 'sticky', top: -44, zIndex: 5,
           background: 'var(--paper)',
           borderBottom: 'var(--hairline)',
}}>
      <KanjiHeader variant="h1" kanji={n} eyebrow="Step" title={title} description={tagline}/>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────
// Empty Observatory — what the user sees before any sessions.
// No sidebar (nothing to navigate to yet). Center the invitation.
function EmptyObservatoryApp({ onBeginSetup }) {
  return (
    <div className="sensei" data-screen-label="Empty Observatory"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生"/>
      <main style={{
 flex: 1, overflow: 'auto', position: 'relative',
                      display: 'flex', alignItems: 'center', justifyContent: 'center'
}} className="py-6 px-8" >
        {/* faint watermark — 空 = emptiness */}
        <div className="kanji" style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          fontSize: 56, color: 'var(--accent)', opacity: 0.035,
          lineHeight: 1, userSelect: 'none', pointerEvents: 'none'
        }}>空</div>

        <div style={{
 maxWidth: 680, width: '100%', position: 'relative', zIndex: 1,
                       display: 'grid', gridTemplateColumns: '1fr 1fr',
                       alignItems: 'center'
}} className="gap-7" >
          {/* Left: the invitation */}
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-5" >
              <span className="kanji" style={{ fontSize: 28, color: 'var(--accent)' }}>先生</span>
              <span className="display" style={{ fontSize: 22, fontWeight: 400 }}>Sensei</span>
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-3" >
              Welcome
            </div>
            <h1 className="display mt-0 mb-4" style={{
 fontSize: 56, fontWeight: 300,
                          letterSpacing: '-0.02em', lineHeight: 1.08
}}>
              A quiet<br/>
              <span style={{ color: 'var(--accent)' }}>empty room.</span>
            </h1>
            <p style={{
 fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.7
}} className="mt-0 mb-6" >
              Point sensei at your folders and keep working. It watches in silence, learns
              the shape of each project, and later begins to teach.
            </p>

            <button onClick={onBeginSetup}
                    style={{
 fontSize: 13, background: 'var(--ink)',
                              color: 'var(--paper)', borderRadius: 6, letterSpacing: 0.2
}} className="py-3 px-5" >
              Begin setup →
            </button>

            <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-4" >
              <span className="mono">~4 minutes</span>
              <span style={{ color: 'var(--ink-4)' }} className="mx-2" >·</span>
              nothing leaves your machine
            </div>
          </div>

          {/* Right: what sensei will do — a real preview, not placeholder stats */}
          <div style={{
            border: 'var(--hairline)', borderRadius: 10,
            background: 'var(--paper-2)'
}} className="py-5 px-5" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-4" >
              What sensei does
            </div>
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-4" >
              {[
                { k: "観", label: "Watches",
                  note: "Every assistant session — prompts, tool calls, diffs." },
                { k: "察", label: "Notices",
                  note: "Which prompts work, which patterns repeat, where you rework." },
                { k: "教", label: "Teaches",
                  note: "After ~3 sessions per project, offers concrete suggestions." }
              ].map((x, i) => (
                <div key={i} style={{
 display: 'grid', gridTemplateColumns: 'auto 1fr', alignItems: 'start'
}} className="gap-3" >
                  <div className="kanji" style={{
                    fontSize: 17, color: 'var(--accent)',
                    width: 32, height: 32, borderRadius: '50%',
                    background: 'var(--paper)', border: 'var(--hairline)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center'
                  }}>{x.k}</div>
                  <div>
                    <div className="display mb-1" style={{ fontSize: 13, fontWeight: 400 }}>
                      {x.label}
                    </div>
                    <div style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.5 }}>
                      {x.note}
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <div style={{
 borderTop: 'var(--hairline)',
                           fontSize: 11, color: 'var(--ink-3)', lineHeight: 1.6
}} className="mt-4 pt-4" >
              Works with{' '}
              <span className="mono" style={{ color: 'var(--ink-2)' }}>claude-code</span>,{' '}
              <span className="mono" style={{ color: 'var(--ink-2)' }}>cursor</span>,{' '}
              <span className="mono" style={{ color: 'var(--ink-2)' }}>codex</span>,{' '}
              <span className="mono" style={{ color: 'var(--ink-2)' }}>aider</span>.
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

Object.assign(window, {
  SetupWizard, EmptyObservatoryApp, EmptyToWizardApp, WIZ_STAGES
});
