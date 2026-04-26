// Sensei — Setup Wizard (10 stages) + Empty Observatory shell.
// Full-bleed flow; reuses primitives. Hybrid layout:
//   - left rail: stepper (completed steps show as collapsed "chips", current expanded)
//   - main area: current stage content
//   - bottom bar: primary/secondary actions + terse progress

const { useState: useS, useEffect: useE, useRef: useR, useMemo: useM } = React;

// ─────────────────────────────────────────────────────────────
// Stage list (order matters)
const WIZ_STAGES = [
  { id: "welcome",     n: "一",  title: "Welcome",         sub: "先生 · a quiet observer of your work" },
  { id: "acps",        n: "二",  title: "Assistants",      sub: "plugins · skills · commands · logging" },
  { id: "folders",     n: "三",  title: "Folders",         sub: "where does your work live" },
  { id: "scan",        n: "四",  title: "Scan",            sub: "watching the worker" },
  { id: "projects",    n: "五",  title: "Projects",        sub: "one or more repos each" },
  { id: "libraries",   n: "六",  title: "Libraries",       sub: "what sensei should wrap" },
  { id: "registry",    n: "七",  title: "Instruments",     sub: "recommended MCPs for your stack" },
  { id: "inference",   n: "八",  title: "Inference",       sub: "providers · models" },
  { id: "assignments", n: "九",  title: "Assignments",     sub: "which models handle which roles" },
  { id: "done",        n: "十",  title: "Enter",           sub: "the observatory is ready" }
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
                  .reduce((a,p)=> (a[p.id] = "", a), {})
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
          <div style={{ flex: 1, overflow: 'auto', padding: '44px 64px 32px' }}>
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
            {stage.id === "done"        && <WizDone state={state}/>}
          </div>
          <WizBottom stage={stage} stageIdx={stageIdx} total={WIZ_STAGES.length}
                     back={back} next={next} onDone={onDone} state={state}/>
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
    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
      <span style={{ width: 7, height: 7, borderRadius: 4,
                      background: anyBad ? 'var(--shu)' : 'var(--matcha)' }}/>
      <div style={{ fontSize: 11, color: 'var(--sumi-2)', lineHeight: 1.4 }}>
        <div style={{ letterSpacing: '0.1em', textTransform: 'uppercase',
                       fontSize: 10, color: 'var(--sumi-3)' }}>
          Services
        </div>
        <div style={{ marginTop: 2 }}>
          {anyBad ? "one or more down" : "all green"}
        </div>
      </div>
    </div>
  );
}

// ─── Left rail ───────────────────────────────────────────────
function WizRail({ stages, stageIdx, setStageIdx, onExit }) {
  return (
    <aside style={{ borderRight: 'var(--hairline)', padding: '26px 22px',
                    display: 'flex', flexDirection: 'column',
                    background: 'var(--paper-2)', overflow: 'hidden' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 28 }}>
        <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>先生</span>
        <span className="display" style={{ fontSize: 17 }}>Sensei</span>
        <span style={{ flex: 1 }}/>
        <button onClick={onExit} title="Exit setup"
                style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.1em' }}>
          ESC
        </button>
      </div>

      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                    textTransform: 'uppercase', marginBottom: 14 }}>Setup</div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
        {stages.map((s, i) => {
          const isCur = i === stageIdx;
          const isDone = i < stageIdx;
          return (
            <button key={s.id}
                    onClick={() => setStageIdx(i)}
                    disabled={i > stageIdx}
                    style={{
                      display: 'grid', gridTemplateColumns: '24px 1fr', gap: 10,
                      padding: isCur ? '10px 10px' : '7px 10px',
                      borderRadius: 6, textAlign: 'left',
                      background: isCur ? 'var(--paper)' : 'transparent',
                      border: isCur ? 'var(--hairline)' : '1px solid transparent',
                      color: isCur ? 'var(--sumi)' : isDone ? 'var(--sumi-2)' : 'var(--sumi-4)',
                      cursor: i > stageIdx ? 'default' : 'pointer',
                      transition: 'all .14s'
                    }}>
              <span className="kanji" style={{
                fontSize: 14, textAlign: 'center',
                color: isCur ? 'var(--shu)' : isDone ? 'var(--jade)' : 'var(--sumi-4)'
              }}>{isDone ? "✓" : s.n}</span>
              <div style={{ overflow: 'hidden' }}>
                <div style={{ fontSize: 13 }}>{s.title}</div>
                {isCur && (
                  <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 2 }}>
                    {s.sub}
                  </div>
                )}
              </div>
            </button>
          );
        })}
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ borderTop: 'var(--hairline)', paddingTop: 12 }}>
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
    return true;
  })();

  return (
    <div style={{ borderTop: 'var(--hairline)', padding: '14px 64px',
                  display: 'flex', alignItems: 'center', gap: 20,
                  background: 'var(--paper)' }}>
      <div style={{ fontSize: 11, letterSpacing: '0.12em', color: 'var(--sumi-3)',
                    textTransform: 'uppercase' }}>
        {String(stageIdx + 1).padStart(2, "0")} <span style={{ color: 'var(--sumi-4)' }}>/ {total}</span>
        <span style={{ marginLeft: 12, color: 'var(--sumi-2)', textTransform: 'none',
                       letterSpacing: 0, fontSize: 13 }}>{stage.title}</span>
      </div>

      {/* progress ticks */}
      <div style={{ flex: 1, display: 'flex', gap: 4, alignItems: 'center' }}>
        {Array.from({ length: total }).map((_, i) => (
          <span key={i} style={{
            flex: 1, height: 2, borderRadius: 1,
            background: i <= stageIdx ? 'var(--sumi)' : 'var(--paper-edge)',
            transition: 'background .2s'
          }}/>
        ))}
      </div>

      <button onClick={back} disabled={isFirst}
              style={{ fontSize: 12, color: isFirst ? 'var(--sumi-4)' : 'var(--sumi-2)',
                       padding: '8px 14px' }}>
        ← Back
      </button>

      {isLast ? (
        <button onClick={onDone}
                style={{ fontSize: 13, background: 'var(--sumi)', color: 'var(--paper)',
                         padding: '10px 22px', borderRadius: 6, letterSpacing: 0.2 }}>
          Enter observatory →
        </button>
      ) : (
        <button onClick={next} disabled={!canAdvance}
                style={{ fontSize: 13,
                         background: canAdvance ? 'var(--sumi)' : 'var(--paper-edge)',
                         color: canAdvance ? 'var(--paper)' : 'var(--sumi-3)',
                         padding: '10px 22px', borderRadius: 6, letterSpacing: 0.2 }}>
          Continue →
        </button>
      )}
    </div>
  );
}

// ─── 1 Welcome ───────────────────────────────────────────────
function WizWelcome() {
  return (
    <div style={{ maxWidth: 680, margin: '10px auto 0' }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 8 }}>一 · Welcome</div>
      <h1 className="display" style={{ fontSize: 54, fontWeight: 300, lineHeight: 1.08,
                    margin: '0 0 32px', letterSpacing: '-0.02em' }}>
        A teacher does not<br/>
        <span style={{ color: 'var(--shu)' }}>write the code.</span>
      </h1>

      <p style={{ fontSize: 16, color: 'var(--sumi-2)', lineHeight: 1.7, maxWidth: 560,
                   margin: '0 0 28px' }}>
        Sensei watches how you and your AI assistants work together — the sessions that
        completed cleanly, the ones that didn't, and the patterns underneath both.
      </p>

      <p style={{ fontSize: 14, color: 'var(--sumi-3)', lineHeight: 1.7, maxWidth: 560,
                   margin: '0 0 44px' }}>
        The next few minutes: install the local components, point to your folders, confirm
        what was found. Nothing leaves your machine.
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 18,
                     padding: '22px 0', borderTop: 'var(--hairline)', borderBottom: 'var(--hairline)' }}>
        {[
          { k: "観", t: "Observe", s: "FTR · turns · corrections" },
          { k: "師", t: "Teach",   s: "patterns · rules · skills" },
          { k: "静", t: "Local",   s: "on your machine" }
        ].map(item => (
          <div key={item.k}>
            <div className="kanji" style={{ fontSize: 26, color: 'var(--shu)', marginBottom: 8 }}>{item.k}</div>
            <div className="display" style={{ fontSize: 20 }}>{item.t}</div>
            <div style={{ fontSize: 12, color: 'var(--sumi-3)', marginTop: 3 }}>{item.s}</div>
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
    <div style={{ maxWidth: 820, margin: '0 auto' }}>
      <WizHeader n="二" title="Components"
                 tagline={overallReady ? "Everything is in place." : "Detecting, installing, starting. No input needed."}/>

      {/* Variant toggle kept only as a subtle demo aid */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 22 }}>
        <span style={{ fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase',
                        color: 'var(--sumi-4)' }}>demo · starting state</span>
        <div style={{ display: 'flex', gap: 0, background: 'var(--paper-2)', padding: 2,
                       borderRadius: 5, border: 'var(--hairline)' }}>
          {D.componentsVariants.map(v => (
            <button key={v.id}
                    onClick={() => {
                      upd({ components: { variant: v.id, acting: {} }});
                      const nv = D.componentsVariants.find(x => x.id === v.id);
                      setPhases(nv.components.reduce((a, c) =>
                        (a[c.id] = c.status === "installed" ? "ready" : "detecting", a), {}));
                    }}
                    style={{ padding: '5px 10px', fontSize: 10.5, borderRadius: 3,
                             background: state.components.variant === v.id ? 'var(--paper)' : 'transparent',
                             color: state.components.variant === v.id ? 'var(--sumi-2)' : 'var(--sumi-3)',
                             border: 'none' }}>
              {v.label}
            </button>
          ))}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {variant.components.map(c => {
          const phase = phases[c.id] || "detecting";
          const isBusy = phase === "installing" || phase === "starting";
          const statusLabel =
            phase === "detecting"  ? "checking…" :
            phase === "installing" ? "installing · 12.4 MB" :
            phase === "starting"   ? "starting…" : `${c.version || "0.9.3"} · ready`;
          const dotColor =
            phase === "ready" ? 'var(--jade)' : 'var(--shu)';

          return (
            <div key={c.id} style={{
              display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 18,
              padding: '16px 20px', border: 'var(--hairline)', borderRadius: 8,
              background: 'var(--paper-2)', alignItems: 'center',
              transition: 'all .3s'
            }}>
              <div style={{ width: 36, height: 36, borderRadius: 6,
                             background: 'var(--paper)', border: 'var(--hairline)',
                             display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                <span className="mono" style={{ fontSize: 13, color: 'var(--sumi-2)' }}>
                  {c.id === "cli" ? "$" : c.id === "mcp" ? "⟷" : "◇"}
                </span>
              </div>
              <div>
                <div style={{ fontSize: 14 }}>{c.name}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 2 }}>
                  {statusLabel}
                </div>
                {isBusy && (
                  <div style={{ height: 2, background: 'var(--paper-edge)', borderRadius: 1,
                                 marginTop: 8, overflow: 'hidden' }}>
                    <div style={{
                      height: '100%', width: '40%', background: 'var(--shu)',
                      animation: 'cSlide 1.2s ease-in-out infinite'
                    }}/>
                  </div>
                )}
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{
                  width: 8, height: 8, borderRadius: '50%', background: dotColor,
                  boxShadow: phase !== "ready" ? `0 0 0 4px ${dotColor}22` : 'none',
                  animation: phase !== "ready" ? "cPulse 1.2s ease-in-out infinite" : 'none'
                }}/>
                <span className="mono" style={{ fontSize: 11,
                               color: phase === "ready" ? 'var(--jade)' : 'var(--sumi-2)',
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

      <p style={{ fontSize: 12, color: 'var(--sumi-3)', marginTop: 20, lineHeight: 1.7 }}>
        Nothing leaves <span className="mono">localhost:9823</span>.
      </p>
    </div>
  );
}

function StatusDot({ status }) {
  const map = {
    installed: { c: 'var(--jade)',  l: "Ready" },
    missing:   { c: 'var(--sumi-4)', l: "Missing" },
    stopped:   { c: 'var(--amber)', l: "Stopped" },
    working:   { c: 'var(--shu)',   l: "Working" }
  };
  const m = map[status] || map.missing;
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <span style={{
        width: 8, height: 8, borderRadius: '50%', background: m.c,
        boxShadow: status === "working" ? `0 0 0 4px ${m.c}22` : 'none',
        animation: status === "working" ? "pulse 1.2s ease-in-out infinite" : 'none'
      }}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)' }}>{m.l}</span>
      <style>{`@keyframes pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.45 } }`}</style>
    </div>
  );
}

// ─── 3 ACPs ──────────────────────────────────────────────────
function WizAcps({ state, upd }) {
  const D = window.SENSEI_SETUP;
  return (
    <div style={{ maxWidth: 820, margin: '0 auto' }}>
      <WizHeader n="三" title="Assistants" tagline="Registers plugins, skills, commands, agents, logging and metrics."/>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        {D.acps.map(a => {
          const on = state.acps[a.id];
          return (
            <button key={a.id}
                    disabled={!a.found}
                    onClick={() => upd({ acps: { ...state.acps, [a.id]: !on } })}
                    style={{
                      textAlign: 'left', padding: '18px 20px', borderRadius: 8,
                      border: on ? '1.5px solid var(--sumi)' : 'var(--hairline)',
                      background: on ? 'var(--paper-2)' : 'var(--paper)',
                      opacity: a.found ? 1 : 0.55,
                      cursor: a.found ? 'pointer' : 'default',
                      display: 'grid', gridTemplateColumns: '1fr auto', gap: 12, alignItems: 'center'
                    }}>
              <div>
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
                  <span style={{ fontSize: 15 }}>{a.name}</span>
                  {a.version && <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>v{a.version}</span>}
                </div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 4 }}>
                  {a.found ? a.path : "not found"}
                </div>
              </div>
              <div style={{
                width: 22, height: 22, borderRadius: 4, flexShrink: 0,
                background: on ? 'var(--sumi)' : 'transparent',
                border: on ? 'none' : 'var(--ink-line)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--paper)'
              }}>
                {on && <svg width="12" height="12" viewBox="0 0 16 16"><path d="M3 8 L7 12 L13 4" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/></svg>}
              </div>
            </button>
          );
        })}
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
    <div style={{ maxWidth: 820, margin: '0 auto' }}>
      <WizHeader n="四" title="Folders" tagline="Where your work lives. Sensei recurses and finds repos."/>

      <div style={{ display: 'flex', gap: 8, marginBottom: 20 }}>
        <input value={state.newFolder}
               onChange={e => upd({ newFolder: e.target.value })}
               onKeyDown={e => e.key === "Enter" && add()}
               placeholder="~/code/my-project"
               className="mono"
               style={{
                 flex: 1, padding: '10px 14px', fontSize: 13,
                 background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6,
                 outline: 'none'
               }}/>
        <button onClick={add}
                style={{ padding: '10px 18px', fontSize: 13, borderRadius: 6,
                         background: 'var(--sumi)', color: 'var(--paper)' }}>
          Add
        </button>
        <button style={{ padding: '10px 18px', fontSize: 13, borderRadius: 6,
                          border: 'var(--ink-line)' }}>
          Browse…
        </button>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {state.folders.map(f => (
          <div key={f.id} style={{
            display: 'grid', gridTemplateColumns: 'auto 1fr auto auto', gap: 14,
            padding: '14px 16px', border: 'var(--hairline)', borderRadius: 6,
            background: 'var(--paper-2)', alignItems: 'center'
          }}>
            <span style={{ color: 'var(--sumi-3)', fontSize: 14 }}>▸</span>
            <div>
              <div className="mono" style={{ fontSize: 13 }}>{f.path}</div>
              <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 2 }}>{f.note}</div>
            </div>
            <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)',
                          padding: '3px 8px', background: 'var(--paper)',
                          borderRadius: 3, border: 'var(--hairline)' }}>recursive</span>
            <button onClick={() => remove(f.id)}
                    style={{ fontSize: 11, color: 'var(--sumi-3)', padding: '4px 8px' }}>
              remove
            </button>
          </div>
        ))}
      </div>

      <div style={{ marginTop: 20, fontSize: 12, color: 'var(--sumi-3)', lineHeight: 1.7 }}>
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
      <div style={{ maxWidth: 820, margin: '0 auto' }}>
        <WizHeader n="五" title="Scan" tagline={`Ready to scan ${state.folders.length} ${state.folders.length === 1 ? "root" : "roots"}.`}/>
        <div style={{ padding: '50px 40px', background: 'var(--paper-2)',
                       borderRadius: 10, border: 'var(--hairline)', textAlign: 'center' }}>
          <div className="kanji" style={{ fontSize: 60, color: 'var(--shu)', opacity: 0.4,
                                           marginBottom: 20 }}>探</div>
          <p style={{ fontSize: 15, color: 'var(--sumi-2)', margin: '0 0 20px',
                       lineHeight: 1.6, maxWidth: 440, marginInline: 'auto' }}>
            The daemon will recurse your folders, identify repositories, and extract the
            code graph. Two workers, ~2M files / minute on this machine.
          </p>
          <button onClick={start}
                  style={{ padding: '12px 28px', fontSize: 14, background: 'var(--sumi)',
                           color: 'var(--paper)', borderRadius: 6 }}>
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
    <div style={{ maxWidth: 1000, margin: '0 auto' }}>
      <WizHeader n="五" title={done ? "Scan complete" : "Scanning"}
                 tagline={done ? "The map is drawn." : "Workers recurse. Repos surface."}/>

      {/* Stats strip */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 0,
                     border: 'var(--hairline)', borderRadius: 8, background: 'var(--paper-2)',
                     marginBottom: 18, overflow: 'hidden' }}>
        <ScanStat label="Roots"      value={state.folders.length}/>
        <ScanStat label="Discovered" value={stats.discovered}/>
        <ScanStat label="Queued"     value={stats.queued}/>
        <ScanStat label="Processed"  value={stats.processed} accent/>
      </div>

      {/* Progress line */}
      <div style={{ height: 2, background: 'var(--paper-edge)', borderRadius: 1,
                     marginBottom: 22, overflow: 'hidden' }}>
        <div style={{ height: '100%', width: `${progress * 100}%`,
                       background: done ? 'var(--jade)' : 'var(--sumi)',
                       transition: 'width 80ms linear' }}/>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 20, alignItems: 'start' }}>
        {/* ─── Left: solutions + repos materializing ─── */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12, minHeight: 360 }}>
          {discoveredSolutions.length === 0 && (
            <div style={{ padding: 40, border: '1px dashed var(--paper-edge)',
                           borderRadius: 10, textAlign: 'center', color: 'var(--sumi-4)',
                           fontSize: 13, fontStyle: 'italic' }}>
              <div className="kanji" style={{ fontSize: 38, color: 'var(--shu)',
                             opacity: 0.3, marginBottom: 10 }}>待</div>
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
          <div style={{ padding: '10px 14px', borderBottom: 'var(--hairline)',
                         fontSize: 10, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                         textTransform: 'uppercase', display: 'flex',
                         alignItems: 'center', gap: 8 }}>
            <span style={{
              width: 6, height: 6, borderRadius: '50%',
              background: done ? 'var(--jade)' : 'var(--shu)',
              animation: done ? 'none' : 'pulseSm 1.2s ease-in-out infinite'
            }}/>
            <span>SSE · /events</span>
            <span style={{ flex: 1 }}/>
            <span className="mono" style={{ textTransform: 'none', letterSpacing: 0 }}>
              {(tick/1000).toFixed(1)}s
            </span>
          </div>
          <div style={{ height: 360, overflow: 'auto', padding: '8px 14px' }}>
            {events.slice().reverse().map((e, i) => (
              <div key={e.t} style={{
                display: 'grid', gridTemplateColumns: '42px 60px 1fr',
                gap: 8, fontSize: 10.5, padding: '3px 0',
                color: i === 0 ? 'var(--sumi)' : 'var(--sumi-2)',
                opacity: i === 0 ? 1 : Math.max(0.28, 1 - i * 0.07),
                animation: i === 0 ? 'eventIn .26s ease-out' : 'none'
              }}>
                <span className="mono" style={{ color: 'var(--sumi-3)' }}>+{(e.t/1000).toFixed(2)}s</span>
                <span className="mono" style={{
                  color: e.level === "success"  ? 'var(--jade)' :
                         e.level === "discover" ? 'var(--shu)'  :
                         e.level === "process"  ? 'var(--sumi)' :
                         e.level === "queue"    ? 'var(--amber)' : 'var(--sumi-3)'
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
        <div style={{ marginTop: 20, padding: '14px 18px', borderRadius: 8,
                       background: 'var(--jade-soft)', fontSize: 13, color: 'var(--sumi)',
                       display: 'flex', alignItems: 'center', gap: 10 }}>
          <span className="kanji" style={{ fontSize: 18, color: 'var(--jade)' }}>✓</span>
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
      border: allDone ? '1.5px solid var(--sumi-2)' : 'var(--hairline)',
      borderRadius: 10, background: 'var(--paper-2)',
      padding: '16px 18px',
      animation: 'cardIn .34s ease-out',
      transition: 'border .3s'
    }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 14, alignItems: 'center' }}>
        <div className="kanji" style={{
          fontSize: 22, width: 38, height: 38, borderRadius: '50%',
          background: 'var(--paper)', border: 'var(--hairline)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          color: allDone ? 'var(--shu)' : 'var(--sumi-3)',
          transition: 'color .3s'
        }}>{sol.kanji}</div>
        <div style={{ minWidth: 0 }}>
          <div className="display" style={{ fontSize: 17 }}>{sol.name}</div>
          <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 1 }}>
            {sol.path} · {discoveredRepos.length} {discoveredRepos.length === 1 ? "repo" : "repos"}
            {!allDone && discoveredRepos.length > 0 && ` · ${doneCount} ready`}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div className="mono" style={{ fontSize: 10,
                         color: allDone ? 'var(--jade)' : 'var(--sumi-3)',
                         letterSpacing: '0.1em', textTransform: 'uppercase' }}>
            {allDone ? "ready" : solState.state}
          </div>
          {totalFiles > 0 && !allDone && (
            <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-2)', marginTop: 2 }}>
              {processedFiles.toLocaleString()} / {totalFiles.toLocaleString()}
            </div>
          )}
        </div>
      </div>

      {/* Repos list */}
      {discoveredRepos.length > 0 && (
        <div style={{ marginTop: 14, paddingLeft: 52,
                       display: 'flex', flexDirection: 'column', gap: 5 }}>
          {discoveredRepos.map(([p, rs]) => {
            const pct = rs.totalFiles > 0 ? rs.processed / rs.totalFiles : 0;
            const isDone = rs.state === "done";
            const isProcessing = rs.state === "processing";
            return (
              <div key={p.id} style={{
                display: 'grid', gridTemplateColumns: '140px 1fr 70px',
                gap: 12, alignItems: 'center',
                animation: 'repoIn .26s ease-out'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span style={{
                    width: 5, height: 5, borderRadius: '50%',
                    background: isDone ? 'var(--jade)' :
                                isProcessing ? 'var(--shu)' :
                                rs.state === "queued" ? 'var(--amber)' : 'var(--sumi-4)',
                    animation: isProcessing ? 'pulseSm 1.2s ease-in-out infinite' : 'none'
                  }}/>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)',
                                overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {p.name}
                  </span>
                </div>
                {/* progress track */}
                <div style={{ height: 2, background: 'var(--paper-edge)', borderRadius: 1,
                               overflow: 'hidden', position: 'relative' }}>
                  <div style={{
                    position: 'absolute', inset: 0, width: `${pct * 100}%`,
                    background: isDone ? 'var(--jade)' : 'var(--sumi)',
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
                <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)',
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
    <div style={{ padding: '16px 18px', borderRight: 'var(--hairline)' }}>
      <div className="display" style={{ fontSize: 30, fontWeight: 400,
                     color: accent ? 'var(--shu)' : 'var(--sumi)' }}>{value}</div>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--sumi-3)', marginTop: 2 }}>{label}</div>
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
    <div style={{ maxWidth: 940, margin: '0 auto' }}>
      <WizHeader n="六" title="Projects"
                 tagline="A project has one or more repos. Edit, split, or confirm."/>

      <div style={{ fontSize: 12, color: 'var(--sumi-3)', marginBottom: 18 }}>
        A single-repo project is the default. Multi-repo projects are auto-grouped from sibling folders and name patterns. Split when they shouldn't be together.
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
        {sols.map(s => {
          const isMulti = s.projects.length > 1;
          const isExpanded = selected === s.id;
          const isMergeOpen = mergeMenu === s.id;
          const mergeTargets = sols.filter(x => x.id !== s.id);
          return (
          <div key={s.id} style={{
            border: 'var(--hairline)',
            borderRadius: 10, background: s.confirmed ? 'var(--paper-2)' : 'var(--paper)',
            padding: 18, opacity: s.confirmed ? 1 : 0.55, transition: 'all .2s',
            position: 'relative'
          }}>
            <div style={{ display: 'grid',
                           gridTemplateColumns: 'auto 1fr auto auto auto auto',
                           gap: 12, alignItems: 'center' }}>
              <div className="kanji" style={{
                fontSize: 26, color: 'var(--shu)',
                width: 42, height: 42, borderRadius: '50%',
                background: 'var(--paper)', border: 'var(--hairline)',
                display: 'flex', alignItems: 'center', justifyContent: 'center'
              }}>{s.kanji}</div>
              <div>
                <input value={s.renamed ?? s.name}
                       onChange={e => rename(s.id, e.target.value)}
                       className="display"
                       style={{
                         fontSize: 19, fontWeight: 400, background: 'transparent',
                         border: 'none', outline: 'none', padding: 0,
                         borderBottom: '1px dashed transparent', width: '100%'
                       }}
                       onFocus={e => e.target.style.borderBottom = '1px dashed var(--sumi-3)'}
                       onBlur={e => e.target.style.borderBottom = '1px dashed transparent'}/>
                <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 2 }}>
                  {s.path} · {s.projects.length} {s.projects.length === 1 ? "repo" : "repos"}
                </div>
              </div>
              {isMulti ? (
                <span className="mono" style={{ fontSize: 9.5, color: 'var(--shu)',
                              letterSpacing: '0.1em', textTransform: 'uppercase',
                              padding: '3px 8px', border: '1px solid var(--shu)',
                              borderRadius: 3, background: 'var(--shu-soft)' }}>
                  multi-repo
                </span>
              ) : <span/>}

              {/* merge button (only shown when there's another project to merge with) */}
              {mergeTargets.length > 0 ? (
                <button onClick={() => { setMergeMenu(isMergeOpen ? null : s.id); setRepoMenu(null); }}
                        style={{ fontSize: 11, color: 'var(--sumi-3)', padding: '6px 10px',
                                  borderRadius: 4 }}>
                  merge…
                </button>
              ) : <span/>}

              <button onClick={() => setSelected(isExpanded ? null : s.id)}
                      style={{ fontSize: 11, color: 'var(--sumi-3)', padding: '6px 10px',
                                borderRadius: 4, display: 'flex', alignItems: 'center', gap: 4 }}>
                {isExpanded ? "hide" : "edit"}
                <span style={{ fontSize: 9, transform: isExpanded ? 'rotate(180deg)' : 'none',
                                transition: 'transform .2s' }}>▾</span>
              </button>
              <button onClick={() => toggle(s.id)}
                      style={{
                        width: 22, height: 22, borderRadius: 4,
                        background: s.confirmed ? 'var(--sumi)' : 'transparent',
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
                marginTop: 14, padding: '12px 14px', paddingLeft: 56,
                background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 6,
                animation: 'expandIn .2s ease-out'
              }}>
                <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                               textTransform: 'uppercase', marginBottom: 8 }}>
                  Merge {s.renamed ?? s.name} into…
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                  {mergeTargets.map(t => (
                    <button key={t.id} onClick={() => mergeInto(s.id, t.id)}
                            className="mono" style={{
                              fontSize: 11, padding: '6px 10px', borderRadius: 4,
                              border: 'var(--hairline)', background: 'var(--paper-2)',
                              color: 'var(--sumi-2)', display: 'inline-flex',
                              alignItems: 'center', gap: 6
                            }}>
                      <span className="kanji" style={{ color: 'var(--shu)', fontSize: 12 }}>{t.kanji}</span>
                      {t.renamed ?? t.name}
                      <span style={{ color: 'var(--sumi-4)', fontSize: 10 }}>({t.projects.length})</span>
                    </button>
                  ))}
                  <button onClick={() => setMergeMenu(null)} className="mono" style={{
                    fontSize: 10.5, padding: '6px 10px', color: 'var(--sumi-3)'
                  }}>cancel</button>
                </div>
              </div>
            )}

            {/* Repo chips — compact, always visible */}
            <div style={{ marginTop: 14, display: 'flex', flexWrap: 'wrap', gap: 6,
                           paddingLeft: 56 }}>
              {s.projects.map(p => {
                const role = D.roles.find(r => r.id === state.roles[p.id]);
                const isOpen = repoMenu && repoMenu.sid === s.id && repoMenu.pid === p.id;
                const moveTargets = sols.filter(x => x.id !== s.id);
                return (
                  <span key={p.id} style={{ position: 'relative', display: 'inline-block' }}>
                    <span className="mono" style={{
                      fontSize: 11, padding: '4px 4px 4px 10px',
                      background: 'var(--paper)', border: 'var(--hairline)',
                      borderRadius: 3, color: 'var(--sumi-2)',
                      display: 'inline-flex', alignItems: 'center', gap: 8
                    }}>
                      {p.name}
                      <span style={{ color: 'var(--sumi-4)' }}>{p.files}f</span>
                      {role && (
                        <span style={{ color: 'var(--shu)', fontSize: 10,
                                        borderLeft: '1px solid var(--paper-edge)',
                                        paddingLeft: 8 }}>
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
                                padding: '0 6px', fontSize: 13, color: 'var(--sumi-3)',
                                borderLeft: '1px solid var(--paper-edge)',
                                lineHeight: 1, marginLeft: 2
                              }}>
                        ⋯
                      </button>
                    </span>
                    {isOpen && (
                      <div style={{
                        position: 'absolute', top: 'calc(100% + 4px)', left: 0, zIndex: 10,
                        minWidth: 220, padding: 6,
                        background: 'var(--paper)', border: 'var(--hairline)', borderRadius: 6,
                        boxShadow: '0 4px 16px rgba(0,0,0,0.08)',
                        animation: 'expandIn .15s ease-out'
                      }}>
                        <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                                       textTransform: 'uppercase', padding: '4px 8px 6px' }}>
                          Move {p.name} to…
                        </div>
                        {moveTargets.map(t => (
                          <button key={t.id} onClick={() => moveRepo(s.id, p.id, t.id)}
                                  className="mono" style={{
                                    display: 'flex', width: '100%', padding: '7px 8px',
                                    fontSize: 11, color: 'var(--sumi-2)', borderRadius: 3,
                                    alignItems: 'center', gap: 8, textAlign: 'left'
                                  }}>
                            <span className="kanji" style={{ color: 'var(--shu)' }}>{t.kanji}</span>
                            {t.renamed ?? t.name}
                          </button>
                        ))}
                        {isMulti && (
                          <button onClick={() => moveRepo(s.id, p.id, "__new__")}
                                  style={{
                                    display: 'flex', width: '100%', padding: '7px 8px',
                                    fontSize: 11, color: 'var(--sumi-3)', borderRadius: 3,
                                    borderTop: 'var(--hairline)', marginTop: 4, paddingTop: 9,
                                    textAlign: 'left'
                                  }}>
                            + split out as new project
                          </button>
                        )}
                      </div>
                    )}
                  </span>
                );
              })}
              {isMulti && (
                <button onClick={() => split(s.id)} className="mono" style={{
                  fontSize: 10.5, padding: '4px 10px', color: 'var(--sumi-3)',
                  border: '1px dashed var(--paper-edge)', borderRadius: 3
                }}>
                  split all into {s.projects.length} projects
                </button>
              )}
            </div>

            {/* Expanded: rename + role picker per repo */}
            {isExpanded && (
              <div style={{
                marginTop: 16, paddingTop: 14, paddingLeft: 56,
                borderTop: 'var(--hairline)',
                display: 'flex', flexDirection: 'column', gap: 6,
                animation: 'expandIn .22s ease-out'
              }}>
                <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                               textTransform: 'uppercase', marginBottom: 6 }}>
                  Repo roles
                </div>
                {s.projects.map(p => (
                  <div key={p.id} style={{
                    display: 'grid', gridTemplateColumns: '1fr auto', gap: 12,
                    padding: '8px 0', alignItems: 'center'
                  }}>
                    <div>
                      <div className="mono" style={{ fontSize: 12, color: 'var(--sumi-2)' }}>{p.name}</div>
                      <div style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 1 }}>
                        {p.lang} · {p.files} files
                      </div>
                    </div>
                    <div style={{ display: 'flex', gap: 2, padding: 2,
                                   background: 'var(--paper)', borderRadius: 4 }}>
                      {D.roles.map(r => (
                        <button key={r.id} onClick={() => setRole(p.id, r.id)}
                                title={r.label}
                                style={{
                                  padding: '4px 8px', fontSize: 10.5, borderRadius: 3,
                                  background: state.roles[p.id] === r.id ? 'var(--sumi)' : 'transparent',
                                  color: state.roles[p.id] === r.id ? 'var(--paper)' : 'var(--sumi-3)'
                                }}>
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

      <div style={{ marginTop: 20, fontSize: 12, color: 'var(--sumi-3)' }}>
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
    <div style={{ maxWidth: 820, margin: '0 auto' }}>
      <WizHeader n="七" title="Context" tagline="Optional. Helps sensei tailor its coaching."/>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
        {state.solutions.filter(s => s.confirmed).map(s => (
          <div key={s.id} style={{
            padding: 22, border: 'var(--hairline)', borderRadius: 10,
            background: 'var(--paper-2)'
          }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 18 }}>
              <span className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>{s.kanji}</span>
              <span className="display" style={{ fontSize: 20 }}>{s.renamed ?? s.name}</span>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
              <MetaField label="Stage">
                <div style={{ display: 'flex', gap: 3, padding: 3,
                               background: 'var(--paper)', borderRadius: 5 }}>
                  {D.metadata.statuses.map(st => (
                    <button key={st.id}
                            onClick={() => setMeta(s.id, "status", st.id)}
                            style={{
                              flex: 1, padding: '6px 8px', fontSize: 11, borderRadius: 3,
                              background: state.metadata[s.id].status === st.id ? 'var(--sumi)' : 'transparent',
                              color: state.metadata[s.id].status === st.id ? 'var(--paper)' : 'var(--sumi-3)'
                            }}>
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
                         width: '100%', padding: '8px 12px', fontSize: 13,
                         background: 'var(--paper)', border: 'var(--hairline)',
                         borderRadius: 5, outline: 'none'
                       }}/>
              </MetaField>

              <div style={{ gridColumn: 'span 2' }}>
                <MetaField label="Goal">
                  <input value={state.metadata[s.id].goal}
                         onChange={e => setMeta(s.id, "goal", e.target.value)}
                         placeholder="One sentence. Why this exists."
                         style={{
                           width: '100%', padding: '8px 12px', fontSize: 13,
                           background: 'var(--paper)', border: 'var(--hairline)',
                           borderRadius: 5, outline: 'none'
                         }}/>
                </MetaField>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div style={{ marginTop: 16, fontSize: 12, color: 'var(--sumi-3)' }}>
        Skip if you like. These can be edited per-solution from the Coaching page.
      </div>
    </div>
  );
}

function MetaField({ label, children }) {
  return (
    <div>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--sumi-3)', marginBottom: 6 }}>{label}</div>
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
      <WizHeader n="七" title="Libraries"
                 tagline="Libraries without their own MCP — sensei indexes docs & code and wraps them with its own tools. Anything with a proper MCP (like Postgres or Stripe) comes in the next step."/>

      {/* Summary bar */}
      <div style={{ display: 'flex', gap: 14, marginBottom: 20, alignItems: 'center' }}>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)',
                      padding: '5px 10px', background: 'var(--paper-2)',
                      border: 'var(--hairline)', borderRadius: 4 }}>
          {D.detected.length} detected
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--jade)',
                      padding: '5px 10px', background: 'var(--jade-soft)',
                      borderRadius: 4 }}>
          {activeCount} will be wrapped
        </span>
        {state.libExtras.length > 0 && (
          <span className="mono" style={{ fontSize: 11, color: 'var(--shu)',
                        padding: '5px 10px', background: 'var(--shu-soft)',
                        borderRadius: 4 }}>
            {state.libExtras.length} added by you
          </span>
        )}
      </div>

      {/* Detected libraries */}
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 10 }}>
        Detected · sensei will wrap
      </div>
      <div style={{ display: 'flex', flexDirection: 'column',
                     border: 'var(--hairline)', borderRadius: 6,
                     background: 'var(--paper-2)', marginBottom: 24 }}>
        {D.detected.map((lib, i) => {
          const on = !!state.libraries[lib.id];
          return (
            <div key={lib.id}
                 style={{ display: 'grid',
                           gridTemplateColumns: 'auto 1fr auto auto auto',
                           gap: 14, alignItems: 'center',
                           padding: '11px 14px',
                           borderBottom: i < D.detected.length - 1 ? 'var(--hairline)' : 'none',
                           opacity: on ? 1 : 0.45 }}>
              <button onClick={() => toggle(lib.id)}
                      style={{ width: 18, height: 18, borderRadius: 3,
                                border: '1.5px solid ' + (on ? 'var(--shu)' : 'var(--sumi-4)'),
                                background: on ? 'var(--shu)' : 'transparent',
                                color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
                {on ? "✓" : ""}
              </button>
              <div>
                <div style={{ fontSize: 13, color: 'var(--sumi)' }}>
                  {lib.name}
                  <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                                marginLeft: 8 }}>{lib.version}</span>
                </div>
                <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 2 }}>
                  {lib.why}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
                {lib.lang}
              </span>
              <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
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
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 10 }}>
            Added by you
          </div>
          <div style={{ display: 'flex', flexDirection: 'column',
                         border: 'var(--hairline)', borderRadius: 6,
                         background: 'var(--paper-2)', marginBottom: 24 }}>
            {state.libExtras.map((lib, i) => {
              const on = lib.on;
              return (
                <div key={lib.id}
                     style={{ display: 'grid',
                               gridTemplateColumns: 'auto 1fr auto auto',
                               gap: 14, alignItems: 'center',
                               padding: '11px 14px',
                               borderBottom: i < state.libExtras.length - 1 ? 'var(--hairline)' : 'none',
                               opacity: on ? 1 : 0.45 }}>
                  <button onClick={() => toggleExtra(lib.id)}
                          style={{ width: 18, height: 18, borderRadius: 3,
                                    border: '1.5px solid ' + (on ? 'var(--shu)' : 'var(--sumi-4)'),
                                    background: on ? 'var(--shu)' : 'transparent',
                                    color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
                    {on ? "✓" : ""}
                  </button>
                  <div>
                    <div style={{ fontSize: 13, color: 'var(--sumi)' }}>{lib.name}</div>
                    <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 2 }}>
                      {lib.url || "no URL"}
                    </div>
                  </div>
                  <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
                    {lib.lang}
                  </span>
                  <button onClick={() => removeExtra(lib.id)}
                          style={{ fontSize: 11, color: 'var(--sumi-4)' }}>remove</button>
                </div>
              );
            })}
          </div>
        </>
      )}

      {/* Add custom library */}
      {!showAdd ? (
        <button onClick={() => setShowAdd(true)}
                style={{ padding: '10px 16px', fontSize: 12.5,
                          background: 'var(--paper-2)',
                          border: '1px dashed var(--sumi-4)', borderRadius: 6,
                          color: 'var(--sumi-2)' }}>
          + Add a library
        </button>
      ) : (
        <div style={{ padding: '18px 20px', background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 8,
                       maxWidth: 640 }}>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 12 }}>
            Add a library sensei should wrap
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr 0.5fr',
                         gap: 10, marginBottom: 12 }}>
            <div>
              <div style={{ fontSize: 10.5, color: 'var(--sumi-2)', marginBottom: 4 }}>Name</div>
              <input value={form.name}
                     onChange={e => setForm({ ...form, name: e.target.value })}
                     placeholder="e.g. @internal/fx"
                     style={wizInputStyle}/>
            </div>
            <div>
              <div style={{ fontSize: 10.5, color: 'var(--sumi-2)', marginBottom: 4 }}>
                Docs URL <span style={{ color: 'var(--sumi-4)' }}>· optional</span>
              </div>
              <input value={form.url}
                     onChange={e => setForm({ ...form, url: e.target.value })}
                     placeholder="https://docs.rs/… or internal wiki"
                     style={wizInputStyle}/>
            </div>
            <div>
              <div style={{ fontSize: 10.5, color: 'var(--sumi-2)', marginBottom: 4 }}>Lang</div>
              <select value={form.lang}
                      onChange={e => setForm({ ...form, lang: e.target.value })}
                      style={wizInputStyle}>
                <option>Rust</option>
                <option>TypeScript</option>
                <option>Python</option>
                <option>Go</option>
                <option>Other</option>
              </select>
            </div>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={addExtra} disabled={!form.name.trim()}
                    style={{ padding: '7px 14px', fontSize: 12,
                              background: form.name.trim() ? 'var(--sumi)' : 'var(--paper-3)',
                              color: form.name.trim() ? 'var(--paper)' : 'var(--sumi-3)',
                              borderRadius: 4 }}>
              Add
            </button>
            <button onClick={() => setShowAdd(false)}
                    style={{ padding: '7px 14px', fontSize: 12,
                              color: 'var(--sumi-3)' }}>
              Cancel
            </button>
            <span style={{ flex: 1 }}/>
            <span style={{ fontSize: 11, color: 'var(--sumi-3)', alignSelf: 'center' }}>
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
      <WizHeader n="七" title="Instruments"
                 tagline="Tools sensei can reach for — recommended based on what's in your stack. Each MCP brings its own capabilities, no wrapping needed."/>

      {/* Detected stack summary */}
      <div style={{ padding: '14px 18px', background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 8,
                     marginBottom: 24 }}>
        <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 10 }}>
          Detected in your stack
        </div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {[...stack.languages, ...stack.frameworks, ...stack.services].map(s => (
            <span key={s} className="mono" style={{ fontSize: 11,
                          padding: '3px 9px', background: 'var(--paper)',
                          border: 'var(--hairline)', borderRadius: 3,
                          color: 'var(--sumi-2)' }}>
              {s}
            </span>
          ))}
        </div>
      </div>

      {/* Summary row */}
      <div style={{ display: 'flex', gap: 14, marginBottom: 16, alignItems: 'center' }}>
        <span className="mono" style={{ fontSize: 11, color: 'var(--jade)',
                      padding: '5px 10px', background: 'var(--jade-soft)',
                      borderRadius: 4 }}>
          {recommended.length} recommended
        </span>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-2)',
                      padding: '5px 10px', background: 'var(--paper-2)',
                      border: 'var(--hairline)', borderRadius: 4 }}>
          {installCount} will be installed
        </span>
      </div>

      {/* Recommended */}
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 10 }}>
        Recommended for your stack
      </div>
      <div style={{ display: 'flex', flexDirection: 'column',
                     border: 'var(--hairline)', borderRadius: 6,
                     background: 'var(--paper-2)', marginBottom: 24 }}>
        {recommended.map((mcp, i) => (
          <McpRow key={mcp.id} mcp={mcp} on={!!state.mcps[mcp.id]}
                  onToggle={() => toggle(mcp.id)}
                  last={i === recommended.length - 1}/>
        ))}
      </div>

      {/* Available */}
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 10 }}>
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
    <div style={{ display: 'grid',
                   gridTemplateColumns: 'auto auto 1fr auto auto',
                   gap: 14, alignItems: 'center',
                   padding: '12px 14px',
                   borderBottom: last ? 'none' : 'var(--hairline)',
                   opacity: on ? 1 : 0.55 }}>
      <button onClick={onToggle}
              style={{ width: 18, height: 18, borderRadius: 3,
                        border: '1.5px solid ' + (on ? 'var(--shu)' : 'var(--sumi-4)'),
                        background: on ? 'var(--shu)' : 'transparent',
                        color: 'var(--paper)', fontSize: 11, lineHeight: 1 }}>
        {on ? "✓" : ""}
      </button>
      <div style={{ width: 32, height: 32, borderRadius: 6,
                     background: 'var(--paper-3)', display: 'flex',
                     alignItems: 'center', justifyContent: 'center' }}>
        <span className="kanji" style={{ fontSize: 15, color: 'var(--shu)' }}>
          {mcp.kanji}
        </span>
      </div>
      <div>
        <div style={{ fontSize: 13, color: 'var(--sumi)' }}>
          {mcp.name}
          <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)', marginLeft: 8 }}>
            by {mcp.publisher}
          </span>
          {mcp.verified && (
            <span className="mono" style={{ fontSize: 9.5, color: 'var(--jade)',
                          marginLeft: 8, padding: '1px 6px',
                          background: 'var(--jade-soft)', borderRadius: 3 }}>
              verified
            </span>
          )}
        </div>
        <div style={{ fontSize: 11.5, color: 'var(--sumi-3)', marginTop: 2,
                       lineHeight: 1.45 }}>
          {mcp.summary}
        </div>
      </div>
      <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
        {mcp.tools} tools
      </span>
      {mcp.trigger && mcp.trigger.length > 0 ? (
        <span className="mono" style={{ fontSize: 10, color: 'var(--jade)',
                      padding: '2px 7px', background: 'var(--jade-soft)',
                      borderRadius: 3, whiteSpace: 'nowrap' }}>
          matches {mcp.trigger[0]}
        </span>
      ) : (
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
          {mcp.kind}
        </span>
      )}
    </div>
  );
}

function LibDocChip({ status }) {
  const map = {
    indexed: { label: "docs indexed", tone: 'var(--jade)',  bg: 'var(--jade-soft)' },
    partial: { label: "partial",      tone: 'var(--amber)', bg: 'var(--amber-soft)' },
    schema:  { label: "schema only",  tone: 'var(--sumi-2)', bg: 'var(--paper-3)'   },
    none:    { label: "no docs",      tone: 'var(--sumi-3)', bg: 'var(--paper-3)'   }
  };
  const m = map[status] || map.none;
  return (
    <span className="mono" style={{ fontSize: 10, padding: '2px 7px', borderRadius: 3,
                background: m.bg, color: m.tone, whiteSpace: 'nowrap' }}>
      {m.label}
    </span>
  );
}

const wizInputStyle = {
  width: '100%', padding: '7px 10px', fontSize: 12,
  border: 'var(--hairline)', borderRadius: 5,
  background: 'var(--paper)', color: 'var(--sumi)',
  fontFamily: 'var(--font-mono)', outline: 'none'
};

// ─── 8 Inference ─────────────────────────────────────────────
// WizInference lives in lib/wiz-inference.jsx — loaded via <script> tag after this file.
// Expects window.SENSEI_SETUP.inference (system, providers, rolePriority, addable).

// ─── 9 Done ─────────────────────────────────────────────────
function WizDone({ state }) {
  const confirmedSols = state.solutions.filter(s => s.confirmed);
  const repoCount = confirmedSols.reduce((a, s) => a + s.projects.length, 0);
  const activeAcps = Object.values(state.acps).filter(Boolean).length;
  const libCount = Object.values(state.libraries || {}).filter(Boolean).length
                 + (state.libExtras || []).filter(x => x.on).length;
  const mcpCount = Object.values(state.mcps || {}).filter(Boolean).length;

  return (
    <div style={{ maxWidth: 680, margin: '20px auto 0', textAlign: 'center' }}>
      <div className="kanji" style={{ fontSize: 92, color: 'var(--shu)', marginBottom: 10 }}>観</div>
      <h1 className="display" style={{ fontSize: 44, fontWeight: 300, letterSpacing: '-0.02em',
                    margin: '0 0 16px' }}>
        The observatory is ready.
      </h1>
      <p style={{ fontSize: 15, color: 'var(--sumi-2)', lineHeight: 1.6, maxWidth: 480,
                   margin: '0 auto 36px' }}>
        Start a session with your assistant. Sensei will watch in silence for a few days,
        then begin to teach.
      </p>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 0,
                     border: 'var(--hairline)', borderRadius: 10,
                     background: 'var(--paper-2)', overflow: 'hidden',
                     textAlign: 'left' }}>
        <DoneStat label="Projects"   value={confirmedSols.length}/>
        <DoneStat label="Repos"      value={repoCount}/>
        <DoneStat label="Libraries"  value={libCount}/>
        <DoneStat label="MCPs"       value={mcpCount}/>
        <DoneStat label="Assistants" value={activeAcps} last/>
      </div>

      <p className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)',
                     marginTop: 36, fontStyle: 'italic' }}>
        一 · the first session is always the teacher
      </p>
    </div>
  );
}

function DoneStat({ label, value, last }) {
  return (
    <div style={{ padding: '18px 20px', borderRight: last ? 'none' : 'var(--hairline)' }}>
      <div className="display" style={{ fontSize: 30, fontWeight: 400 }}>{value}</div>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--sumi-3)', marginTop: 2 }}>{label}</div>
    </div>
  );
}

// ─── Shared: step header ─────────────────────────────────────
function WizHeader({ n, title, tagline }) {
  return (
    <div style={{ marginBottom: 32 }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase', marginBottom: 8 }}>
        <span className="kanji" style={{ color: 'var(--shu)', marginRight: 8 }}>{n}</span>
        Step
      </div>
      <h1 className="display" style={{ fontSize: 36, fontWeight: 300,
                     letterSpacing: '-0.02em', margin: '0 0 6px' }}>
        {title}
      </h1>
      <p style={{ fontSize: 14, color: 'var(--sumi-3)', margin: 0 }}>{tagline}</p>
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
      <main style={{ flex: 1, overflow: 'auto', position: 'relative',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      padding: '40px 60px' }}>
        {/* faint watermark — 空 = emptiness */}
        <div className="kanji" style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          fontSize: 640, color: 'var(--shu)', opacity: 0.035,
          lineHeight: 1, userSelect: 'none', pointerEvents: 'none'
        }}>空</div>

        <div style={{ maxWidth: 680, width: '100%', position: 'relative', zIndex: 1,
                       display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 48,
                       alignItems: 'center' }}>
          {/* Left: the invitation */}
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 28 }}>
              <span className="kanji" style={{ fontSize: 28, color: 'var(--shu)' }}>先生</span>
              <span className="display" style={{ fontSize: 22, fontWeight: 400 }}>Sensei</span>
            </div>
            <div style={{ fontSize: 10, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 14 }}>
              Welcome
            </div>
            <h1 className="display" style={{ fontSize: 48, fontWeight: 300,
                          letterSpacing: '-0.02em', lineHeight: 1.08, margin: '0 0 20px' }}>
              A quiet<br/>
              <span style={{ color: 'var(--shu)' }}>empty room.</span>
            </h1>
            <p style={{ fontSize: 14.5, color: 'var(--sumi-2)', lineHeight: 1.7,
                         margin: '0 0 36px' }}>
              Point sensei at your folders and keep working. It watches in silence, learns
              the shape of each project, and later begins to teach.
            </p>

            <button onClick={onBeginSetup}
                    style={{ padding: '13px 28px', fontSize: 14, background: 'var(--sumi)',
                              color: 'var(--paper)', borderRadius: 6, letterSpacing: 0.2 }}>
              Begin setup →
            </button>

            <div style={{ marginTop: 18, fontSize: 11, color: 'var(--sumi-3)' }}>
              <span className="mono">~4 minutes</span>
              <span style={{ margin: '0 8px', color: 'var(--sumi-4)' }}>·</span>
              nothing leaves your machine
            </div>
          </div>

          {/* Right: what sensei will do — a real preview, not placeholder stats */}
          <div style={{
            padding: '24px 22px',
            border: 'var(--hairline)', borderRadius: 10,
            background: 'var(--paper-2)'
          }}>
            <div style={{ fontSize: 10, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 16 }}>
              What sensei does
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
              {[
                { k: "観", label: "Watches",
                  note: "Every assistant session — prompts, tool calls, diffs." },
                { k: "察", label: "Notices",
                  note: "Which prompts work, which patterns repeat, where you rework." },
                { k: "教", label: "Teaches",
                  note: "After ~3 sessions per project, offers concrete suggestions." }
              ].map((x, i) => (
                <div key={i} style={{ display: 'grid', gridTemplateColumns: 'auto 1fr',
                                        gap: 14, alignItems: 'start' }}>
                  <div className="kanji" style={{
                    fontSize: 18, color: 'var(--shu)',
                    width: 32, height: 32, borderRadius: '50%',
                    background: 'var(--paper)', border: 'var(--hairline)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center'
                  }}>{x.k}</div>
                  <div>
                    <div className="display" style={{ fontSize: 14, fontWeight: 400, marginBottom: 2 }}>
                      {x.label}
                    </div>
                    <div style={{ fontSize: 12, color: 'var(--sumi-3)', lineHeight: 1.5 }}>
                      {x.note}
                    </div>
                  </div>
                </div>
              ))}
            </div>

            <div style={{ marginTop: 20, paddingTop: 16, borderTop: 'var(--hairline)',
                           fontSize: 11, color: 'var(--sumi-3)', lineHeight: 1.6 }}>
              Works with{' '}
              <span className="mono" style={{ color: 'var(--sumi-2)' }}>claude-code</span>,{' '}
              <span className="mono" style={{ color: 'var(--sumi-2)' }}>cursor</span>,{' '}
              <span className="mono" style={{ color: 'var(--sumi-2)' }}>codex</span>,{' '}
              <span className="mono" style={{ color: 'var(--sumi-2)' }}>aider</span>.
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

// Combined artboard: empty observatory → clicking "begin setup" enters wizard
function EmptyToWizardApp() {
  const [mode, setMode] = useS("empty"); // "empty" | "wizard" | "entered"
  if (mode === "wizard") {
    return <SetupWizard onExit={() => setMode("empty")} onDone={() => setMode("empty")}/>;
  }
  return <EmptyObservatoryApp onBeginSetup={() => setMode("wizard")}/>;
}

Object.assign(window, {
  SetupWizard, EmptyObservatoryApp, EmptyToWizardApp, WIZ_STAGES
});
