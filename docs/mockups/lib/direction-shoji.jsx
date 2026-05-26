// Direction 3 — SHOJI (障子) · rice-paper grid, dense but calm.
// Modular panes like a shoji screen. Dark ink lines, monospace-forward data.

const ShojiApp = () => {
  const data = window.SENSEI_DATA;
  const [page, setPage] = React.useState("observatory");
  const [activeSolution, setActiveSolution] = React.useState("lumen-cloud");
  const [focusedSession, setFocusedSession] = React.useState(null);
  const [appliedCoaching, setAppliedCoaching] = React.useState({});

  const sol = data.solutions.find(s => s.id === activeSolution);

  return (
    <div className="sensei" data-screen-label="Shoji · Direction 3"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei · 障子"/>
      <ShojiTopbar page={page} setPage={setPage} solutions={data.solutions}
                   activeSolution={activeSolution} setActiveSolution={setActiveSolution}/>
      <main style={{ flex: 1, overflow: 'auto', minHeight: 0 }}>
        {page === "overview"    && <ShojiOverview data={data} setPage={setPage} setActiveSolution={setActiveSolution}/>}
        {page === "observatory" && <ShojiObservatory data={data} sol={sol} setPage={setPage}
                                                     setFocusedSession={setFocusedSession}
                                                     applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
        {page === "sessions"    && <ShojiSessions data={data} sol={sol}
                                                  focused={focusedSession} setFocused={setFocusedSession}/>}
        {page === "codebase"    && <ShojiCodebase data={data} sol={sol}/>}
        {page === "coaching"    && <ShojiCoaching data={data} applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
        {page === "config"      && <ShojiConfig data={data}/>}
        {page === "onboarding"  && <ShojiOnboarding/>}
      </main>
    </div>
  );
};

// ────────────────────────────────────────────────────────────
// TOPBAR — horizontal tabs, solution pill
// ────────────────────────────────────────────────────────────
function ShojiTopbar({ page, setPage, solutions, activeSolution, setActiveSolution }) {
  const sol = solutions.find(s => s.id === activeSolution);
  return (
    <div style={{
 display: 'flex', alignItems: 'center',
                  borderBottom: 'var(--hairline)', flexShrink: 0, background: 'var(--paper)'
}} className="gap-4 py-2 px-4" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
        <span className="kanji" style={{ fontSize: 17, color: 'var(--accent)' }}>先</span>
        <span className="display" style={{ fontSize: 13, fontWeight: 500 }}>Sensei</span>
      </div>

      <div style={{ width: 1, height: 20, background: 'var(--edge)' }}/>

      {/* Solution switcher pill */}
      <div style={{
 display: 'flex', background: 'var(--paper-3)',
                    borderRadius: 8
}} className="gap-1 p-1" >
        {solutions.map(s => (
          <button key={s.id} onClick={() => setActiveSolution(s.id)}
                  style={{
 borderRadius: 6, fontSize: 13,
                            display: 'flex', alignItems: 'center',
                            background: activeSolution === s.id ? 'var(--paper)' : 'transparent',
                            color: activeSolution === s.id ? 'var(--ink)' : 'var(--ink-2)',
                            boxShadow: activeSolution === s.id ? '0 1px 2px rgba(0,0,0,0.04)' : '',
                            transition: 'all .14s'
}} className="py-1 px-3 gap-1" >
            <span className="kanji" style={{ fontSize: 11,
                          color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
            {s.name}
          </button>
        ))}
      </div>

      {/* Primary nav */}
      <nav style={{ display: 'flex' }} className="gap-0 ml-2" >
        {PAGES.map(p => (
          <button key={p.id} onClick={() => setPage(p.id)}
                  style={{
 fontSize: 13,
                            color: page === p.id ? 'var(--ink)' : 'var(--ink-3)',
                            borderBottom: page === p.id ? '2px solid var(--accent)' : '2px solid transparent',
                            marginBottom: -11, fontWeight: page === p.id ? 500 : 400
}} className="py-2 px-3" >
            {p.label}
          </button>
        ))}
      </nav>

      <div style={{ flex: 1 }}/>

      <div style={{
 display: 'flex', alignItems: 'center', fontSize: 11,
                    color: 'var(--ink-3)'
}} className="gap-3" >
        <span className="mono">daemon · 9823</span>
        <span className="ink-dot" style={{ background: 'var(--success)' }}/>
        <Avatar name="Aiko" size={22}/>
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// OBSERVATORY — shoji grid of panes
// ────────────────────────────────────────────────────────────
function ShojiObservatory({ data, sol, setPage, setFocusedSession, applied, setApplied }) {
  const history = data.ftrBySolution[sol.id];
  const topCoach = data.coaching[0];
  const solSessions = data.sessions.filter(s => s.solution === sol.id).slice(0, 6);
  const delta = Math.round((sol.ftr - sol.ftrPrev) * 100);

  return (
    <div style={{
 display: 'grid', height: '100%',
                  gridTemplateColumns: 'repeat(12, 1fr)',
                  gridTemplateRows: 'auto auto auto auto',
                  gridAutoFlow: 'dense'
}} className="p-4 gap-3" >
      {/* FTR — hero pane */}
      <Pane title="First Try Right" kanji="一" span={{ col: 'span 5', row: 'span 2' }}>
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3" >
          <span className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 0.9, letterSpacing: '-0.03em' }}>
            {Math.round(sol.ftr * 100)}
          </span>
          <span style={{ fontSize: 22, color: 'var(--ink-3)' }}>%</span>
          <span className="mono ml-auto" style={{
 fontSize: 13,
                        color: delta >= 0 ? 'var(--success)' : 'var(--accent)'
}}>
            {delta >= 0 ? '↗ +' : '↘ '}{delta}% / w
          </span>
        </div>
        <div style={{ color: delta >= 0 ? 'var(--success)' : 'var(--accent)' }} className="mt-3" >
          <Sparkline data={history} width={380} height={50} fill="currentColor" />
        </div>
        <div className="mono mt-1" style={{
 display: 'flex', justifyContent: 'space-between',
                      fontSize: 11, color: 'var(--ink-3)'
}}>
          <span>14d ago</span>
          <span>7d ago</span>
          <span>today</span>
        </div>
      </Pane>

      {/* Sensei says — coaching pane */}
      <Pane title="Sensei says" kanji="師" span={{ col: 'span 4', row: 'span 2' }}
            accent>
        <p className="display mt-1 mb-2" style={{
 fontSize: 22, fontWeight: 300, lineHeight: 1.3, textWrap: 'balance'
}}>
          {topCoach.koan}
        </p>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }} className="mb-4" >
          {topCoach.body}
        </p>
        <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
                style={{
 borderRadius: 6, fontSize: 13,
                          background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--accent)',
                          color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
                          fontWeight: 500
}} className="py-2 px-3" >
          {applied[topCoach.id] ? '✓ Applied' : topCoach.action}
        </button>
        <div className="mono mt-2" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {topCoach.impact}
        </div>
      </Pane>

      {/* Counter stats */}
      <Pane title="Sessions" kanji="刻" span={{ col: 'span 3', row: 'span 1' }}>
        <div className="display" style={{ fontSize: 40, fontWeight: 300, lineHeight: 1 }}>
          {sol.sessions7d}
        </div>
        <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          7d · {sol.tokens7d}M tokens
        </div>
      </Pane>

      <Pane title="Skills active" kanji="技" span={{ col: 'span 3', row: 'span 1' }}>
        <div className="display" style={{ fontSize: 40, fontWeight: 300, lineHeight: 1 }}>
          {sol.activeSkills}
        </div>
        <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          of {data.skills.length} installed
        </div>
      </Pane>

      {/* Quality signals */}
      <Pane title="Quality signals" kanji="質" span={{ col: 'span 6', row: 'span 1' }}>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-3" >
          {[
            { k: "Pattern compliance", v: "94%",     d: "+3",      good: true },
            { k: "Test coverage Δ",    v: "+2.1%",   d: "this wk", good: true },
            { k: "Doc drift",          v: "3 files", d: "brand-kit", good: false },
            { k: "Tokens / session",   v: "14.2k",   d: "−1.8k",   good: true },
          ].map(s => (
            <div key={s.k}>
              <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.05em' }}>{s.k}</div>
              <div className="display mt-1" style={{ fontSize: 22, fontWeight: 400 }}>{s.v}</div>
              <div className="mono" style={{ fontSize: 11,
                            color: s.good ? 'var(--success)' : 'var(--accent)' }}>{s.d}</div>
            </div>
          ))}
        </div>
      </Pane>

      {/* Recent sessions table */}
      <Pane title="Recent sessions" kanji="刻" span={{ col: 'span 8', row: 'span 2' }}
            action={<button onClick={() => setPage("sessions")}
                            style={{ fontSize: 11, color: 'var(--ink-3)' }}>all →</button>}>
        <div>
          <div className="mono gap-2 py-1 px-0" style={{
 display: 'grid',
                        gridTemplateColumns: '12px 60px 1fr 120px 60px 60px', fontSize: 11,
                        color: 'var(--ink-3)', letterSpacing: '0.06em',
                        borderBottom: 'var(--hairline)', textTransform: 'uppercase'
}}>
            <span/><span>id</span><span>title</span><span>module</span><span>turns</span><span>dur</span>
          </div>
          {solSessions.map(s => (
            <button key={s.id} onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
                    style={{
 display: 'grid',
                              gridTemplateColumns: '12px 60px 1fr 120px 60px 60px', alignItems: 'center',
                              borderBottom: 'var(--hairline)', width: '100%', textAlign: 'left',
                              background: 'transparent'
}} className="gap-2 py-2 px-0" >
              <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.id}</span>
              <span style={{ fontSize: 13, color: 'var(--ink)' }}>{s.title}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.module}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.turns}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.duration}</span>
            </button>
          ))}
        </div>
      </Pane>

      {/* Hotspots */}
      <Pane title="Hotspots" kanji="熱" span={{ col: 'span 4', row: 'span 2' }}>
        {data.hotspots.map((h, i) => (
          <div key={i} style={{ borderBottom: i < data.hotspots.length-1 ? 'var(--hairline)' : 'none' }} className="py-2 px-0" >
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
              <span className="ink-dot" style={{
                background: h.severity === 'god' ? 'var(--accent)' :
                            h.severity === 'cluster' ? 'var(--warning)' : 'var(--success)'
              }}/>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink)' }}>{h.name.split('/').pop()}</span>
            </div>
            <div className="mono pl-3" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              in {h.fanIn} · out {h.fanOut} · <span style={{ color: h.rework > 3 ? 'var(--accent)' : 'var(--ink-3)' }}>↻ {h.rework}</span>
            </div>
          </div>
        ))}
      </Pane>
    </div>
  );
}

// Pane helper — a shoji rice-paper pane
function Pane({ title, kanji, span, children, action, accent }) {
  return (
    <section style={{
      gridColumn: span.col, gridRow: span.row,
      background: accent ? 'var(--paper-2)' : 'var(--paper)',
      border: 'var(--hairline)', borderRadius: 10, display: 'flex', flexDirection: 'column', minWidth: 0,
      position: 'relative', overflow: 'hidden'
}} className="p-4" >
      <header style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-3" >
        <span className="kanji" style={{ fontSize: 11, color: accent ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
        <span style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.1em',
                        textTransform: 'uppercase', fontWeight: 500 }}>{title}</span>
        <div style={{ flex: 1 }}/>
        {action}
      </header>
      <div style={{ flex: 1, minHeight: 0 }}>{children}</div>
    </section>
  );
}

// ────────────────────────────────────────────────────────────
// OVERVIEW
// ────────────────────────────────────────────────────────────
function ShojiOverview({ data, setPage, setActiveSolution }) {
  return (
    <div style={{
 display: 'grid',
                  gridTemplateColumns: 'repeat(3, 1fr)'
}} className="p-5 gap-3" >
      <section style={{
 gridColumn: 'span 3', border: 'var(--hairline)', borderRadius: 10, background: 'var(--paper-2)'
}} className="p-5" >
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.1em',
                      textTransform: 'uppercase'
}} className="mb-1" >全 · Global</div>
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-6" >
          <div>
            <div className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 1 }}>
              78<span style={{ fontSize: 22, color: 'var(--ink-3)' }}>%</span>
            </div>
            <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              first try right · 14d
            </div>
          </div>
          <div style={{ flex: 1, color: 'var(--accent)' }}>
            <Sparkline data={data.ftrHistory} width={720} height={54} fill="var(--accent-soft)" showDots/>
          </div>
        </div>
      </section>

      {data.solutions.map(s => (
        <button key={s.id}
                onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
                style={{
 border: 'var(--hairline)', borderRadius: 10,
                          background: 'var(--paper)', textAlign: 'left', cursor: 'pointer',
                          display: 'flex', flexDirection: 'column',
                          transition: 'border-color .12s'
}}
                onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--ink-3)'}
                onMouseLeave={e => e.currentTarget.style.borderColor = ''} className="p-5 gap-2" >
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
            <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>{s.kanji}</span>
            <span className="display" style={{ fontSize: 17, fontWeight: 500 }}>{s.name}</span>
            {s.warning && <span className="mono py-1 px-2 ml-auto" style={{
 fontSize: 11, borderRadius: 4, background: 'var(--accent-soft)',
                          color: 'var(--accent)'
}}>attention</span>}
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.description}</div>
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3 mt-1" >
            <span className="display" style={{ fontSize: 28, fontWeight: 300 }}>{Math.round(s.ftr*100)}%</span>
            <span className="mono" style={{ fontSize: 11,
                          color: s.ftr >= s.ftrPrev ? 'var(--success)' : 'var(--accent)' }}>
              {s.ftr >= s.ftrPrev ? '+' : ''}{Math.round((s.ftr - s.ftrPrev)*100)}%
            </span>
            <div style={{ flex: 1, color: s.ftr >= s.ftrPrev ? 'var(--success)' : 'var(--accent)' }}>
              <Sparkline data={data.ftrBySolution[s.id]} width={140} height={24}/>
            </div>
          </div>
          <hr className="hairline"/>
          <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                        display: 'flex', justifyContent: 'space-between' }}>
            <span>{s.repos.length} repos</span>
            <span>{s.sessions7d} sessions</span>
            <span>{s.tokens7d}M</span>
          </div>
        </button>
      ))}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// SESSIONS — wide table + inline drawer
// ────────────────────────────────────────────────────────────
function ShojiSessions({ data, sol, focused, setFocused }) {
  const [filter, setFilter] = React.useState("all");
  const [moduleFilter, setModuleFilter] = React.useState("all");
  const modules = ["all", ...new Set(data.sessions.map(s => s.module))];
  const sessions = data.sessions.filter(s => {
    if (filter === "corrected" && s.outcome !== "corrected") return false;
    if (filter === "first-try" && s.outcome !== "first-try") return false;
    if (moduleFilter !== "all" && s.module !== moduleFilter) return false;
    return true;
  });

  return (
    <div className="p-5" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-4 mb-4" >
        <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>
          刻 · Sessions
        </h1>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {sessions.length} records
        </div>
        <div style={{ flex: 1 }}/>
        <div style={{ display: 'flex', background: 'var(--paper-3)', borderRadius: 6 }} className="gap-1 p-1" >
          {["all","first-try","corrected"].map(f => (
            <button key={f} onClick={() => setFilter(f)}
                    style={{
 fontSize: 11, borderRadius: 4,
                              background: filter === f ? 'var(--paper)' : 'transparent',
                              color: filter === f ? 'var(--ink)' : 'var(--ink-2)'
}} className="py-1 px-2" >
              {f}
            </button>
          ))}
        </div>
        <select value={moduleFilter} onChange={e => setModuleFilter(e.target.value)}
                className="mono py-1 px-2"
                style={{
 fontSize: 11, border: 'var(--hairline)',
                          borderRadius: 6, background: 'var(--paper)', color: 'var(--ink)'
}}>
          {modules.map(m => <option key={m}>{m}</option>)}
        </select>
      </div>

      <div style={{ border: 'var(--hairline)', borderRadius: 10, overflow: 'hidden' }}>
        <div className="mono gap-3 py-2 px-4" style={{
 display: 'grid',
                      gridTemplateColumns: '14px 70px 2fr 1fr 100px 70px 70px 70px', fontSize: 11, letterSpacing: '0.06em',
                      color: 'var(--ink-3)', textTransform: 'uppercase',
                      borderBottom: 'var(--hairline)', background: 'var(--paper-3)'
}}>
          <span/><span>id</span><span>title</span><span>module</span>
          <span>date</span><span style={{ textAlign: 'right' }}>turns</span>
          <span style={{ textAlign: 'right' }}>tokens</span><span style={{ textAlign: 'right' }}>dur</span>
        </div>
        {sessions.map(s => (
          <React.Fragment key={s.id}>
            <button onClick={() => setFocused(focused === s.id ? null : s.id)}
                    style={{
 display: 'grid',
                              gridTemplateColumns: '14px 70px 2fr 1fr 100px 70px 70px 70px', alignItems: 'center',
                              borderBottom: 'var(--hairline)', width: '100%', textAlign: 'left',
                              background: focused === s.id ? 'var(--paper-3)' : 'transparent',
                              transition: 'background .12s'
}} className="gap-3 py-3 px-4" >
              <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.id}</span>
              <span style={{ fontSize: 13, color: 'var(--ink)' }}>{s.title}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>{s.module}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.date} {s.started.split(' ').pop()}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', textAlign: 'right' }}>{s.turns}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', textAlign: 'right' }}>{(s.tokens/1000).toFixed(1)}k</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', textAlign: 'right' }}>{s.duration}</span>
            </button>
            {focused === s.id && (
              <div style={{
 background: 'var(--paper-2)',
                            borderBottom: 'var(--hairline)'
}} className="pt-4 pb-5 pl-9 pr-4" >
                <div style={{
 fontSize: 13, color: 'var(--ink-2)', fontStyle: 'italic', maxWidth: 680, lineHeight: 1.55
}} className="mb-3" >
                  {s.summary}
                </div>
                {s.events && (
                  <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
                    {s.events.map((e, i) => (
                      <div key={i} style={{
 display: 'grid', gridTemplateColumns: '14px 50px 1fr', alignItems: 'center'
}} className="gap-3" >
                        <span style={{ color: e.kind === 'correction' ? 'var(--accent)' :
                                              e.kind === 'test' ? 'var(--success)' : 'var(--ink-3)' }}>
                          <EventGlyph kind={e.kind} size={12}/>
                        </span>
                        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{e.t}</span>
                        <span style={{ fontSize: 13, color: e.kind === 'correction' ? 'var(--accent)' : 'var(--ink-2)' }}>
                          {e.text}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </React.Fragment>
        ))}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CODEBASE
// ────────────────────────────────────────────────────────────
function ShojiCodebase({ data, sol }) {
  const [repo, setRepo] = React.useState(sol.repos[0]);
  return (
    <div style={{
 display: 'grid',
                  gridTemplateColumns: 'repeat(12, 1fr)',
                  gridAutoRows: 'min-content'
}} className="p-5 gap-3" >
      <section style={{ gridColumn: 'span 12', display: 'flex', alignItems: 'center' }} className="gap-3" >
        <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>構 · {sol.name}</h1>
        <div style={{ display: 'flex', background: 'var(--paper-3)', borderRadius: 6 }} className="gap-1 p-1" >
          {sol.repos.map(r => (
            <button key={r} onClick={() => setRepo(r)}
                    className="mono py-1 px-2"
                    style={{
 fontSize: 11, borderRadius: 4,
                              background: repo === r ? 'var(--paper)' : 'transparent',
                              color: repo === r ? 'var(--ink)' : 'var(--ink-2)'
}}>
              {r}
            </button>
          ))}
        </div>
        <div style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>247 nodes · 4 communities · indexed 2m ago</span>
      </section>

      <Pane title="Graph" kanji="網" span={{ col: 'span 8', row: 'span 1' }}>
        <svg viewBox="0 0 600 280" width="100%" height="280">
          {/* grid */}
          <g stroke="var(--edge)" strokeWidth="0.5">
            {Array.from({length: 12}).map((_,i) => (
              <line key={'v'+i} x1={50 * i} y1="0" x2={50 * i} y2="280"/>
            ))}
            {Array.from({length: 6}).map((_,i) => (
              <line key={'h'+i} x1="0" y1={50 * i} x2="600" y2={50 * i}/>
            ))}
          </g>
          {/* communities (clusters) */}
          <g>
            <ellipse cx="160" cy="90"  rx="90" ry="55" fill="var(--accent-soft)" opacity="0.4"/>
            <ellipse cx="440" cy="100" rx="80" ry="50" fill="var(--success-soft)" opacity="0.3"/>
            <ellipse cx="260" cy="210" rx="120" ry="50" fill="var(--warning-soft)" opacity="0.3"/>
          </g>
          {/* edges */}
          <g stroke="var(--ink-4)" strokeWidth="0.6" opacity="0.6">
            <path d="M160 90 Q 300 30 440 100" fill="none"/>
            <path d="M160 90 Q 180 140 260 210" fill="none"/>
            <path d="M440 100 Q 380 160 260 210" fill="none"/>
          </g>
          {/* nodes */}
          {Array.from({length: 70}).map((_, i) => {
            const cluster = i % 3;
            const cx = cluster === 0 ? 160 : cluster === 1 ? 440 : 260;
            const cy = cluster === 0 ? 90  : cluster === 1 ? 100 : 210;
            const r = Math.random();
            const a = (i / 70) * Math.PI * 14;
            const d = 40 + r * 60;
            const x = cx + Math.cos(a) * d;
            const y = cy + Math.sin(a) * d * 0.6;
            return <circle key={i} cx={x} cy={y} r={1.5 + r * 1.5} fill="var(--ink-2)" opacity="0.6"/>;
          })}
          {/* god nodes */}
          <circle cx="160" cy="90" r="10" fill="var(--accent-soft)" stroke="var(--accent)" strokeWidth="2"/>
          <text x="175" y="94" fontSize="10" fill="var(--accent)" fontFamily="var(--font-mono)">router.ts</text>
          <text x="120" y="60" fontSize="10" fill="var(--ink-3)" fontFamily="var(--font-mono)">core</text>
          <text x="400" y="65" fontSize="10" fill="var(--ink-3)" fontFamily="var(--font-mono)">api</text>
          <text x="220" y="255" fontSize="10" fill="var(--ink-3)" fontFamily="var(--font-mono)">ui</text>
        </svg>
      </Pane>

      <Pane title="Communities" kanji="群" span={{ col: 'span 4', row: 'span 1' }}>
        {[
          { name: "core",  nodes: 32, color: 'var(--accent)' },
          { name: "api",   nodes: 14, color: 'var(--success)' },
          { name: "ui",    nodes: 22, color: 'var(--warning)' },
          { name: "utils", nodes: 8,  color: 'var(--ink-3)' },
        ].map((c, i) => (
          <div key={c.name} style={{
 display: 'flex', alignItems: 'center',
                        borderBottom: i < 3 ? 'var(--hairline)' : 'none'
}} className="gap-2 py-1 px-0" >
            <span className="ink-dot" style={{ background: c.color }}/>
            <span style={{ flex: 1, fontSize: 13 }}>{c.name}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{c.nodes} nodes</span>
          </div>
        ))}
      </Pane>

      <Pane title="Hotspots" kanji="熱" span={{ col: 'span 8', row: 'span 1' }}>
        <div className="mono gap-3 py-1 px-0" style={{
 display: 'grid',
                      gridTemplateColumns: '14px 2fr 70px 70px 70px', fontSize: 11,
                      color: 'var(--ink-3)', letterSpacing: '0.06em',
                      borderBottom: 'var(--hairline)', textTransform: 'uppercase'
}}>
          <span/><span>path</span>
          <span style={{ textAlign: 'right' }}>fan in</span>
          <span style={{ textAlign: 'right' }}>fan out</span>
          <span style={{ textAlign: 'right' }}>rework</span>
        </div>
        {data.hotspots.map((h, i) => (
          <div key={i} style={{
 display: 'grid',
                        gridTemplateColumns: '14px 2fr 70px 70px 70px', alignItems: 'center',
                        borderBottom: i < data.hotspots.length - 1 ? 'var(--hairline)' : 'none'
}} className="gap-3 py-2 px-0" >
            <span className="ink-dot" style={{
              background: h.severity === 'god' ? 'var(--accent)' :
                          h.severity === 'cluster' ? 'var(--warning)' : 'var(--success)'
            }}/>
            <span className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{h.name}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', textAlign: 'right' }}>{h.fanIn}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', textAlign: 'right' }}>{h.fanOut}</span>
            <span className="mono" style={{ fontSize: 11, color: h.rework > 3 ? 'var(--accent)' : 'var(--ink-3)', textAlign: 'right' }}>
              ↻ {h.rework}
            </span>
          </div>
        ))}
      </Pane>

      <Pane title="Health" kanji="健" span={{ col: 'span 4', row: 'span 1' }}>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
          {[
            { l: "Dead code",     v: "14 exports" },
            { l: "Test ratio",    v: "0.72 : 1" },
            { l: "Largest file",  v: "router.ts · 812 ln" },
            { l: "Median LOC",    v: "68 lines" },
            { l: "Last indexed",  v: "2m ago" }
          ].map(r => (
            <div key={r.l} style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>{r.l}</span>
              <span className="mono" style={{ fontSize: 11 }}>{r.v}</span>
            </div>
          ))}
        </div>
      </Pane>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// COACHING
// ────────────────────────────────────────────────────────────
function ShojiCoaching({ data, applied, setApplied }) {
  return (
    <div style={{
 display: 'grid',
                  gridTemplateColumns: 'repeat(12, 1fr)'
}} className="p-5 gap-3" >
      <section style={{ gridColumn: 'span 12' }}>
        <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>
          師 · Coaching
        </h1>
        <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-1" >
          Three observations. Apply to improve the week.
        </div>
      </section>

      {data.coaching.map((c) => {
        const isApplied = applied[c.id];
        const urgencyColor = c.urgency === 'high' ? 'var(--accent)' :
                             c.urgency === 'medium' ? 'var(--warning)' : 'var(--ink-3)';
        return (
          <section key={c.id} style={{
 gridColumn: 'span 4', border: 'var(--hairline)',
                        borderRadius: 10, background: 'var(--paper)',
                        display: 'flex', flexDirection: 'column'
}} className="p-4 gap-3" >
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
              <span style={{ width: 6, height: 22, borderRadius: 2, background: urgencyColor }}/>
              <span className="mono" style={{ fontSize: 11, color: urgencyColor,
                            letterSpacing: '0.12em', textTransform: 'uppercase' }}>{c.urgency}</span>
              <span className="mono ml-auto" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {c.module}
              </span>
            </div>
            <p className="display m-0" style={{ fontSize: 17, fontWeight: 400, lineHeight: 1.3 }}>
              {c.koan}
            </p>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55, flex: 1 }} className="m-0" >
              {c.body}
            </p>
            <hr className="hairline"/>
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
              <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
                      style={{
 borderRadius: 6, fontSize: 11,
                                background: isApplied ? 'var(--success-soft)' : 'var(--ink)',
                                color: isApplied ? 'var(--success)' : 'var(--paper)',
                                fontWeight: 500
}} className="py-2 px-3" >
                {isApplied ? "✓ Applied" : c.action}
              </button>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {c.actionDetail}
              </span>
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-2)', fontStyle: 'italic' }}>
              → {c.impact}
            </div>
          </section>
        );
      })}

      <section style={{ gridColumn: 'span 12' }} className="mt-2" >
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase'
}} className="mb-2" >Active personas</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-2" >
          {data.personas.map(p => (
            <div key={p.id} style={{
 border: 'var(--hairline)', borderRadius: 8,
                          background: 'var(--paper)'
}} className="p-3" >
              <div style={{ fontSize: 13 }}>{p.name}</div>
              <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {p.triggers}
              </div>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CONFIG
// ────────────────────────────────────────────────────────────
function ShojiConfig({ data }) {
  return (
    <div style={{
 display: 'grid',
                  gridTemplateColumns: 'repeat(12, 1fr)'
}} className="p-5 gap-3" >
      <Pane title="Skills" kanji="技" span={{ col: 'span 7', row: 'span 1' }}>
        {data.skills.map((s, i) => (
          <div key={s.id} style={{
 display: 'grid', gridTemplateColumns: '14px 1fr auto auto', alignItems: 'center',
                        borderBottom: i < data.skills.length-1 ? 'var(--hairline)' : 'none'
}} className="gap-3 py-2 px-0" >
            <span className="ink-dot" style={{ background: s.active ? 'var(--success)' : 'var(--ink-4)' }}/>
            <div>
              <div style={{ fontSize: 13 }}>{s.name}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {s.solutions.length ? s.solutions.join(' · ') : 'not installed'}
              </div>
            </div>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.id}</span>
            <div style={{ width: 28, height: 16, borderRadius: 999,
                          background: s.active ? 'var(--accent)' : 'var(--paper-3)',
                          position: 'relative' }}>
              <div style={{ position: 'absolute', top: 2, left: s.active ? 14 : 2,
                            width: 12, height: 12, background: 'var(--paper)',
                            borderRadius: '50%', transition: 'left .15s' }}/>
            </div>
          </div>
        ))}
      </Pane>

      <Pane title="Libraries" kanji="書" span={{ col: 'span 5', row: 'span 1' }}>
        {data.libraries.map((l, i) => (
          <div key={l.name} style={{
 display: 'grid', gridTemplateColumns: '1fr auto auto', alignItems: 'center',
                        borderBottom: 'var(--hairline)'
}} className="gap-3 py-2 px-0" >
            <div style={{ fontSize: 13 }}>{l.name} <span className="mono" style={{ color: 'var(--ink-3)', fontSize: 11 }}>v{l.version}</span></div>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.pages}p</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.lastIndexed}</span>
          </div>
        ))}
        <button style={{
 borderRadius: 6, fontSize: 11,
                      border: '1px dashed var(--ink-3)', color: 'var(--ink-2)'
}} className="mt-3 py-2 px-3" >
          + Index a library
        </button>
      </Pane>

      <Pane title="Assistants (ACPs)" kanji="助" span={{ col: 'span 7', row: 'span 1' }}>
        {[
          { name: "Claude Code", version: "1.8.2", status: "connected" },
          { name: "Cursor",      version: "0.42",  status: "connected" },
          { name: "Zed",         version: "0.148", status: "available" }
        ].map((a, i, arr) => (
          <div key={a.name} style={{
 display: 'grid', gridTemplateColumns: '14px 1fr auto auto', alignItems: 'center',
                        borderBottom: i < arr.length-1 ? 'var(--hairline)' : 'none'
}} className="gap-3 py-2 px-0" >
            <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--success)' : 'var(--ink-4)' }}/>
            <div>
              <div style={{ fontSize: 13 }}>{a.name}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>MCP · v{a.version} · {a.status}</div>
            </div>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{a.status}</span>
            <button style={{
 fontSize: 11, borderRadius: 5,
                          border: 'var(--hairline)', color: 'var(--ink-2)'
}} className="py-1 px-2" >
              {a.status === 'connected' ? 'Configure' : 'Connect'}
            </button>
          </div>
        ))}
      </Pane>

      <Pane title="Daemon" kanji="守" span={{ col: 'span 5', row: 'span 1' }}>
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-3" >
          <span className="ink-dot" style={{ background: 'var(--success)' }}/>
          <span style={{ fontSize: 13 }}>running</span>
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>pid 12492</span>
        </div>
        {[
          ["Port",           "9823"],
          ["Uptime",         "4d 2h"],
          ["Events / day",   "1,842"],
          ["Memory",         "42 MB"],
        ].map(([l,v]) => (
          <div key={l} style={{ display: 'flex', justifyContent: 'space-between' }} className="py-1 px-0" >
            <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>{l}</span>
            <span className="mono" style={{ fontSize: 11 }}>{v}</span>
          </div>
        ))}
      </Pane>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// ONBOARDING
// ────────────────────────────────────────────────────────────
function ShojiOnboarding() {
  const [step, setStep] = React.useState(1);
  const steps = [
    { n: 1, t: "Find assistants",    d: "detect Claude Code · Cursor · Zed" },
    { n: 2, t: "Scan folders",       d: "discover git repositories" },
    { n: 3, t: "Group into solutions",d: "confirm, assign roles" },
    { n: 4, t: "First index",        d: "extract graphs, compute baseline" }
  ];
  return (
    <div style={{
 display: 'grid',
                  gridTemplateColumns: 'repeat(4, 1fr)', maxWidth: 1100
}} className="p-6 gap-4" >
      <section style={{ gridColumn: 'span 4' }}>
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)' }}>始</div>
        <h1 className="display mt-2 mb-1" style={{ fontSize: 28, fontWeight: 300 }}>Begin.</h1>
        <div style={{ fontSize: 13, color: 'var(--ink-3)' }}>Four panes, four strokes.</div>
      </section>

      {steps.map(s => (
        <section key={s.n}
                 onClick={() => setStep(s.n)}
                 style={{
 border: 'var(--hairline)', borderRadius: 10,
                          background: s.n === step ? 'var(--paper-2)' : 'var(--paper)',
                          opacity: s.n > step ? 0.5 : 1,
                          borderColor: s.n === step ? 'var(--accent)' : '',
                          cursor: 'pointer', minHeight: 160
}} className="p-5" >
          <div className="display" style={{ fontSize: 56, fontWeight: 300,
                        color: s.n < step ? 'var(--success)' : s.n === step ? 'var(--accent)' : 'var(--ink-3)',
                        lineHeight: 1 }}>
            {s.n < step ? '✓' : '0' + s.n}
          </div>
          <div className="display mt-3" style={{ fontSize: 17, fontWeight: 500 }}>{s.t}</div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >{s.d}</div>
        </section>
      ))}

      <section style={{ gridColumn: 'span 4', display: 'flex' }} className="gap-2 mt-1" >
        <button onClick={() => setStep(Math.min(4, step+1))}
                style={{
 background: 'var(--ink)', color: 'var(--paper)',
                          borderRadius: 6, fontSize: 13
}} className="py-2 px-4" >
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
        <button style={{ color: 'var(--ink-3)', fontSize: 13 }} className="py-2 px-4" >Skip</button>
      </section>
    </div>
  );
}

window.ShojiApp = ShojiApp;
