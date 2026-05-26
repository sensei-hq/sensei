// Direction 4 — MERGED (Ma + Enso) · Enso layout with Ma's trendline + collapsible sidebar.
// Sidebar collapses from wide (Ma) to icon-only (Enso) via a toggle.

const MergedApp = () => {
  const data = window.SENSEI_DATA;
  const [page, setPage] = React.useState("observatory");
  const [activeSolution, setActiveSolution] = React.useState("lumen-cloud");
  const [focusedSession, setFocusedSession] = React.useState(null);
  const [appliedCoaching, setAppliedCoaching] = React.useState({});
  const [sidebarCollapsed, setSidebarCollapsed] = React.useState(false);

  const sol = data.solutions.find(s => s.id === activeSolution);

  return (
    <div className="sensei" data-screen-label="Merged · Refined"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生"/>
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <MergedSidebar page={page} setPage={setPage}
                       solutions={data.solutions}
                       activeSolution={activeSolution} setActiveSolution={setActiveSolution}
                       collapsed={sidebarCollapsed}
                       setCollapsed={setSidebarCollapsed}/>
        <main style={{ flex: 1, overflow: 'auto' }}>
          {page === "overview"    && <MergedOverview data={data} setPage={setPage} setActiveSolution={setActiveSolution}/>}
          {page === "observatory" && <MergedObservatory data={data} sol={sol} setPage={setPage}
                                                        setFocusedSession={setFocusedSession}
                                                        applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
          {page === "sessions"    && <EnsoSessions data={data} sol={sol}
                                                   focused={focusedSession} setFocused={setFocusedSession}/>}
          {page === "codebase"    && <EnsoCodebase data={data} sol={sol}/>}
          {page === "coaching"    && <EnsoCoaching data={data} applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
          {page === "config"      && <EnsoConfig data={data}/>}
          {page === "onboarding"  && <EnsoOnboarding/>}
        </main>
      </div>
    </div>
  );
};

// Collapsible sidebar: wide (220) ↔ icon-only (64)
function MergedSidebar({ page, setPage, solutions, activeSolution, setActiveSolution, collapsed, setCollapsed }) {
  const w = collapsed ? 64 : 220;
  return (
    <aside style={{
      width: w, borderRight: 'var(--hairline)',
      padding: collapsed ? '18px 0' : '22px 18px',
      display: 'flex', flexDirection: 'column',
      alignItems: collapsed ? 'center' : 'stretch',
      gap: collapsed ? 18 : 22,
      background: collapsed ? 'var(--paper-2)' : 'var(--paper)',
      flexShrink: 0, transition: 'width .22s, padding .22s, background .22s',
      overflow: 'hidden'
    }}>
      {/* Logo row + toggle */}
      {collapsed ? (
        <div className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>先</div>
      ) : (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
            <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>先</span>
            <span className="display" style={{ fontSize: 17, fontWeight: 400 }}>Sensei</span>
          </div>
          <button onClick={() => setCollapsed(true)} title="Collapse sidebar"
                  style={{ width: 24, height: 24, borderRadius: 4, color: 'var(--ink-3)',
                            display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4">
              <path d="M10 3 L5 8 L10 13"/>
            </svg>
          </button>
        </div>
      )}

      {/* Solutions */}
      {collapsed ? (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }} className="gap-2" >
          {solutions.map(s => {
            const isActive = activeSolution === s.id;
            return (
              <button key={s.id} title={s.name}
                      onClick={() => setActiveSolution(s.id)}
                      style={{
                        width: 40, height: 40, borderRadius: '50%',
                        background: isActive ? 'var(--ink)' : 'var(--paper)',
                        color: isActive ? 'var(--paper)' : 'var(--ink)',
                        border: isActive ? 'none' : 'var(--hairline)',
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        transition: 'all .14s'
                      }}>
                <span className="kanji" style={{ fontSize: 17 }}>{s.kanji}</span>
              </button>
            );
          })}
        </div>
      ) : (
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase'
}} className="mb-2" >Solutions</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {solutions.map(s => (
              <button key={s.id}
                      onClick={() => setActiveSolution(s.id)}
                      style={{
                        display: 'flex', alignItems: 'center', borderRadius: 6, textAlign: 'left',
                        background: activeSolution === s.id ? 'var(--paper-3)' : 'transparent',
                        color: activeSolution === s.id ? 'var(--ink)' : 'var(--ink-2)',
                        fontSize: 13, transition: 'background .12s'
}} className="gap-2 py-2 px-2" >
                <span className="kanji" style={{ fontSize: 13, width: 16,
                              color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
                <span style={{ flex: 1 }}>{s.name}</span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{pct(s.ftr)}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {collapsed && <hr className="hairline my-1 mx-0" style={{ width: 28, border: 'none', background: 'var(--edge)', height: 1 }}/>}

      {/* Views */}
      {collapsed ? (
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }} className="gap-1" >
          {PAGES.map(p => (
            <button key={p.id} title={p.label} onClick={() => setPage(p.id)}
                    style={{
                      width: 36, height: 36, borderRadius: 6,
                      background: page === p.id ? 'var(--accent-soft)' : 'transparent',
                      color: page === p.id ? 'var(--accent)' : 'var(--ink-3)',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      transition: 'all .12s'
                    }}>
              <span className="kanji" style={{ fontSize: 13 }}>{p.kanji}</span>
            </button>
          ))}
        </div>
      ) : (
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase'
}} className="mb-2" >View</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {PAGES.map(p => (
              <button key={p.id} onClick={() => setPage(p.id)}
                      style={{
                        display: 'flex', alignItems: 'center', borderRadius: 6, textAlign: 'left',
                        background: page === p.id ? 'var(--ink)' : 'transparent',
                        color: page === p.id ? 'var(--paper)' : 'var(--ink-2)',
                        fontSize: 13, transition: 'background .12s'
}} className="gap-2 py-2 px-2" >
                <span className="kanji" style={{ fontSize: 13, width: 14, opacity: 0.7 }}>{p.kanji}</span>
                <span>{p.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      <div style={{ flex: 1 }}/>

      {collapsed ? (
        <>
          <button onClick={() => setCollapsed(false)} title="Expand sidebar"
                  style={{ width: 28, height: 28, borderRadius: 6, color: 'var(--ink-3)',
                            display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4">
              <path d="M6 3 L11 8 L6 13"/>
            </svg>
          </button>
          <Avatar name="Aiko" size={28}/>
        </>
      ) : (
        <div style={{ display: 'flex', alignItems: 'center', fontSize: 13, color: 'var(--ink-3)' }} className="gap-2" >
          <Avatar name="Aiko" size={22}/>
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <div style={{ color: 'var(--ink-2)', fontSize: 13 }}>Aiko</div>
            <div style={{ fontSize: 11 }}>daemon · 9823</div>
          </div>
          <span className="ink-dot" style={{ background: 'var(--success)' }}/>
        </div>
      )}
    </aside>
  );
}

// Observatory — Enso's two-column layout, but Ma's big FTR number + trendline
function MergedObservatory({ data, sol, setPage, setFocusedSession, applied, setApplied }) {
  const topCoach = data.coaching[0];
  const history = data.ftrBySolution[sol.id];
  const solSessions = data.sessions.filter(s => s.solution === sol.id).slice(0, 6);
  const delta = Math.round((sol.ftr - sol.ftrPrev) * 100);
  const trendUp = delta >= 0;

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1.2fr 1fr', position: 'relative' }} className="py-6 px-7 gap-7" >
      {/* Kanji watermark */}
      <div className="kanji" style={{
        position: 'absolute', top: 20, right: 40, fontSize: 56,
        color: 'var(--accent)', opacity: 0.05, lineHeight: 1, userSelect: 'none',
        pointerEvents: 'none', zIndex: 0
      }}>{sol.kanji}</div>

      {/* LEFT */}
      <div style={{ position: 'relative', zIndex: 1 }}>
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase'
}} className="mb-1" >
          Observatory · {sol.name}
        </div>
        <h1 className="display mt-0 mb-6" style={{ fontSize: 28, fontWeight: 400 }}>
          {sol.description}
        </h1>

        {/* Hero FTR — Ma style */}
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-5 mb-2" >
          <div className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 0.9,
                        letterSpacing: '-0.04em', fontFeatureSettings: '"ss01"' }}>
            {Math.round(sol.ftr * 100)}
            <span style={{ fontSize: 56, color: 'var(--ink-3)', fontWeight: 300 }} className="ml-1" >%</span>
          </div>
          <div className="pb-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                          color: 'var(--ink-3)'
}} className="mb-1" >一 First try right</div>
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
              <span className="mono" style={{ fontSize: 13,
                        color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
                {trendUp ? "↗" : "↘"} {delta >= 0 ? "+" : ""}{delta}%
              </span>
              <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>vs. last week</span>
            </div>
          </div>
        </div>

        {/* Trendline — Ma's sparkline, wide */}
        <div style={{ color: trendUp ? 'var(--success)' : 'var(--accent)' }} className="mt-4" >
          <Sparkline data={history} width={560} height={72} fill={trendUp ? 'var(--success-soft)' : 'var(--accent-soft)'} showDots/>
          <div className="mono mt-1" style={{
 display: 'flex', justifyContent: 'space-between',
                        fontSize: 11, color: 'var(--ink-3)'
}}>
            <span>14d ago</span><span>7d ago</span><span>today</span>
          </div>
        </div>

        {/* Stats row */}
        <div style={{
 display: 'flex', justifyContent: 'space-between', border: 'var(--hairline)', borderRadius: 10,
                      background: 'var(--paper-2)'
}} className="mt-6 py-4 px-4" >
          <MStat label="Sessions"  value={sol.sessions7d}     suffix="· 7d"/>
          <MDivider/>
          <MStat label="Tokens"    value={sol.tokens7d + "M"} suffix="· 7d"/>
          <MDivider/>
          <MStat label="Skills"    value={sol.activeSkills}   suffix="active"/>
          <MDivider/>
          <MStat label="Repos"     value={sol.repos.length}   suffix={sol.repos.join(' · ')}/>
        </div>

        {/* Recent sessions */}
        <div className="mt-6" >
          <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase'
}} className="mb-3" >Recent sessions</div>
          <div>
            {solSessions.slice(0, 4).map((s, i) => (
              <button key={s.id}
                      onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
                      style={{
                        display: 'grid', gridTemplateColumns: '10px 1fr auto auto', alignItems: 'center', textAlign: 'left',
                        borderBottom: i < 3 ? 'var(--hairline)' : 'none', width: '100%', background: 'transparent'
}} className="gap-3 py-3 px-0" >
                <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
                <div>
                  <div style={{ fontSize: 13, color: 'var(--ink)' }}>{s.title}</div>
                  <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    {s.project} · {s.module}
                  </div>
                </div>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {s.turns}t · {s.duration}
                </span>
                <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>→</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* RIGHT — coaching + signals */}
      <div style={{ display: 'flex', flexDirection: 'column', position: 'relative', zIndex: 1 }} className="gap-5" >
        <div style={{
 background: 'var(--paper-2)', borderRadius: 14,
                      border: 'var(--hairline)'
}} className="py-5 px-5" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                        color: 'var(--accent)'
}} className="mb-4" >師 · sensei says</div>
          <p className="display m-0" style={{
 fontSize: 28, fontWeight: 300, lineHeight: 1.25, textWrap: 'balance'
}}>
            {topCoach.koan}
          </p>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6 }} className="mt-4 mb-5" >
            {topCoach.body}
          </p>
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-3" >
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
                    style={{
                      background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--accent)',
                      color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
                      borderRadius: 8, fontSize: 13, fontWeight: 500
}} className="py-2 px-4" >
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action + " →"}
            </button>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {topCoach.impact}
            </span>
          </div>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                        color: 'var(--ink-3)'
}} className="mb-3" >Quality signals</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-2" >
            {[
              { k: "Pattern compliance", v: "94%",     d: "+3",         good: true },
              { k: "Test coverage Δ",    v: "+2.1%",   d: "this week",  good: true },
              { k: "Doc drift",          v: "3 files", d: "brand-kit",  good: false },
              { k: "Tokens / session",   v: "14.2k",   d: "−1.8k",      good: true }
            ].map(s => (
              <div key={s.k} style={{
 border: 'var(--hairline)', borderRadius: 8,
                            background: 'var(--paper)'
}} className="p-3" >
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.k}</div>
                <div className="display mt-1" style={{ fontSize: 22, fontWeight: 400 }}>{s.v}</div>
                <div className="mono mt-1" style={{
 fontSize: 11,
                              color: s.good ? 'var(--success)' : 'var(--accent)'
}}>{s.d}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Second coaching — below */}
        {data.coaching[1] && (
          <div style={{ border: 'var(--hairline)', borderRadius: 10, background: 'var(--paper)' }} className="p-4" >
            <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                          textTransform: 'uppercase'
}} className="mb-1" >Also observed</div>
            <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.45, fontStyle: 'italic' }}>
              "{data.coaching[1].koan}"
            </div>
            <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {data.coaching[1].actionDetail}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function MStat({ label, value, suffix }) {
  return (
    <div>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.1em', textTransform: 'uppercase' }}>{label}</div>
      <div className="display mt-1" style={{ fontSize: 22, fontWeight: 400 }}>{value}</div>
      <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{suffix}</div>
    </div>
  );
}
function MDivider() { return <div style={{ width: 1, background: 'var(--edge)' }}/>; }

// Overview — reuse the solution cards from Enso but with trendlines instead of rings
function MergedOverview({ data, setPage, setActiveSolution }) {
  return (
    <div style={{ maxWidth: 1120 }} className="py-6 px-7" >
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >全 · Overview</div>
      <h1 className="display mt-0 mb-2" style={{ fontSize: 28, fontWeight: 300 }}>
        All solutions
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13 }} className="mb-6" >
        Global FTR <span className="mono" style={{ color: 'var(--ink)' }}>78%</span> · week to date.
      </div>

      <div style={{ color: 'var(--accent)' }} className="mb-6" >
        <Sparkline data={data.ftrHistory} width={900} height={70} fill="var(--accent-soft)" showDots/>
        <div className="mono mt-1" style={{
 display: 'flex', justifyContent: 'space-between',
                      fontSize: 11, color: 'var(--ink-3)'
}}>
          <span>Apr 9</span><span>Apr 16</span><span>Apr 22</span>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4" >
        {data.solutions.map(s => {
          const up = s.ftr >= s.ftrPrev;
          return (
            <button key={s.id} onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
                    style={{
 borderRadius: 12, border: 'var(--hairline)',
                              background: 'var(--paper)', textAlign: 'left',
                              transition: 'all .15s', position: 'relative', overflow: 'hidden'
}}
                    onMouseEnter={e => { e.currentTarget.style.borderColor = 'var(--ink-3)'; e.currentTarget.style.transform = 'translateY(-2px)'; }}
                    onMouseLeave={e => { e.currentTarget.style.borderColor = ''; e.currentTarget.style.transform = ''; }} className="p-5" >
              <span className="kanji" style={{
                position: 'absolute', top: -24, right: -18, fontSize: 56,
                color: 'var(--accent)', opacity: 0.06, lineHeight: 1
              }}>{s.kanji}</span>
              <div className="display" style={{ fontSize: 17, fontWeight: 500 }}>{s.name}</div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mb-4" >{s.description}</div>
              <div className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 1, letterSpacing: '-0.03em' }}>
                {Math.round(s.ftr*100)}<span style={{ fontSize: 17, color: 'var(--ink-3)' }}>%</span>
              </div>
              <div className="mono mt-1" style={{
 fontSize: 11, color: 'var(--ink-3)',
                            letterSpacing: '0.08em', textTransform: 'uppercase'
}}>First try right</div>
              <div style={{ color: up ? 'var(--success)' : 'var(--accent)' }} className="mt-3" >
                <Sparkline data={data.ftrBySolution[s.id]} width={220} height={32} fill={up ? 'var(--success-soft)' : 'var(--accent-soft)'}/>
              </div>
              <hr className="hairline mt-4 mb-3" style={{ border: 'none', background: 'var(--edge)', height: 1 }}/>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                            display: 'flex', justifyContent: 'space-between' }}>
                <span>{s.repos.length} repos</span>
                <span>{s.sessions7d} sessions</span>
                <span>{s.tokens7d}M</span>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

window.MergedApp = MergedApp;
