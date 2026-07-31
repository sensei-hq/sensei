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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Merged · Refined"
 >
      <TauriChrome title="Sensei  先生"/>
      <div className="flex-1 flex min-h-0" >
        <MergedSidebar page={page} setPage={setPage}
                       solutions={data.solutions}
                       activeSolution={activeSolution} setActiveSolution={setActiveSolution}
                       collapsed={sidebarCollapsed}
                       setCollapsed={setSidebarCollapsed}/>
        <main className="flex-1 overflow-auto" >
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
    <aside className="border-r flex flex-col shrink-0 overflow-hidden" style={{
 width: w,
 padding: collapsed ? '18px 0' : '22px 18px',
 alignItems: collapsed ? 'center' : 'stretch',
 gap: collapsed ? 18 : 22,
 background: collapsed ? 'var(--paper-2)' : 'var(--paper)', transition: 'width .22s, padding .22s, background .22s' }}>
      {/* Logo row + toggle */}
      {collapsed ? (
        <div className="kanji text-accent" style={{ fontSize: 22 }}>先</div>
      ) : (
        <div className="flex items-center justify-between" >
          <div className="gap-2 flex items-baseline" >
            <span className="kanji text-accent" style={{ fontSize: 22 }}>先</span>
            <span className="display font-normal" style={{ fontSize: 17 }}>Sensei</span>
          </div>
          <button className="text-ink-3 flex items-center justify-center" onClick={() => setCollapsed(true)} title="Collapse sidebar"
 style={{ width: 24, height: 24, borderRadius: 4 }}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4">
              <path d="M10 3 L5 8 L10 13"/>
            </svg>
          </button>
        </div>
      )}

      {/* Solutions */}
      {collapsed ? (
        <div className="gap-2 flex flex-col items-center" >
          {solutions.map(s => {
            const isActive = activeSolution === s.id;
            return (
              <button className="rounded-full flex items-center justify-center" key={s.id} title={s.name}
 onClick={() => setActiveSolution(s.id)}
 style={{
 width: 40, height: 40,
 background: isActive ? 'var(--ink)' : 'var(--paper)',
 color: isActive ? 'var(--paper)' : 'var(--ink)',
 border: isActive ? 'none' : 'var(--hairline)',
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
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Solutions</div>
          <div className="gap-1 flex flex-col" >
            {solutions.map(s => (
              <button key={s.id}
 onClick={() => setActiveSolution(s.id)}
 style={{ borderRadius: 6,
 background: activeSolution === s.id ? 'var(--paper-3)' : 'transparent',
 color: activeSolution === s.id ? 'var(--ink)' : 'var(--ink-2)',
 fontSize: 13, transition: 'background .12s'
 }} className="gap-2 py-2 px-2 flex items-center text-left" >
                <span className="kanji" style={{ fontSize: 13, width: 16,
                              color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)' }}>{s.kanji}</span>
                <span className="flex-1" >{s.name}</span>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>{pct(s.ftr)}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {collapsed && <hr className="hairline my-1 mx-0 border-0" style={{ width: 28, background: 'var(--edge)', height: 1 }}/>}

      {/* Views */}
      {collapsed ? (
        <div className="gap-1 flex flex-col items-center" >
          {PAGES.map(p => (
            <button className="flex items-center justify-center" key={p.id} title={p.label} onClick={() => setPage(p.id)}
 style={{
 width: 36, height: 36, borderRadius: 6,
 background: page === p.id ? 'var(--accent-soft)' : 'transparent',
 color: page === p.id ? 'var(--accent)' : 'var(--ink-3)',
 transition: 'all .12s'
 }}>
              <span className="kanji" style={{ fontSize: 13 }}>{p.kanji}</span>
            </button>
          ))}
        </div>
      ) : (
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >View</div>
          <div className="gap-1 flex flex-col" >
            {PAGES.map(p => (
              <button key={p.id} onClick={() => setPage(p.id)}
 style={{ borderRadius: 6,
 background: page === p.id ? 'var(--ink)' : 'transparent',
 color: page === p.id ? 'var(--paper)' : 'var(--ink-2)',
 fontSize: 13, transition: 'background .12s'
 }} className="gap-2 py-2 px-2 flex items-center text-left" >
                <span className="kanji" style={{ fontSize: 13, width: 14, opacity: 0.7 }}>{p.kanji}</span>
                <span>{p.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex-1" />

      {collapsed ? (
        <>
          <button className="text-ink-3 flex items-center justify-center" onClick={() => setCollapsed(false)} title="Expand sidebar"
 style={{ width: 28, height: 28, borderRadius: 6 }}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4">
              <path d="M6 3 L11 8 L6 13"/>
            </svg>
          </button>
          <Avatar name="Aiko" size={28}/>
        </>
      ) : (
        <div style={{ fontSize: 13 }} className="gap-2 flex items-center text-ink-3" >
          <Avatar name="Aiko" size={22}/>
          <div className="flex-1 overflow-hidden" >
            <div className="text-ink-2" style={{ fontSize: 13 }}>Aiko</div>
            <div style={{ fontSize: 11 }}>daemon · 9823</div>
          </div>
          <span className="ink-dot bg-success" />
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
    <div style={{ gridTemplateColumns: '1.2fr 1fr' }} className="py-8 px-12 gap-12 grid relative" >
      {/* Kanji watermark */}
      <div className="kanji absolute text-accent" style={{ top: 20, right: 40, fontSize: 56, opacity: 0.05, lineHeight: 1, userSelect: 'none',
 pointerEvents: 'none', zIndex: 0
 }}>{sol.kanji}</div>

      {/* LEFT */}
      <div className="relative" style={{ zIndex: 1 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >
          Observatory · {sol.name}
        </div>
        <h1 className="display mt-0 mb-8 font-normal" style={{ fontSize: 28 }}>
          {sol.description}
        </h1>

        {/* Hero FTR — Ma style */}
        <div className="gap-6 mb-2 flex items-baseline" >
          <div className="display font-light" style={{ fontSize: 56, lineHeight: 0.9,
 letterSpacing: '-0.04em', fontFeatureSettings: '"ss01"' }}>
            {Math.round(sol.ftr * 100)}
            <span style={{ fontSize: 56 }} className="ml-1 text-ink-3 font-light" >%</span>
          </div>
          <div className="pb-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 uppercase text-ink-3" >一 First try right</div>
            <div className="gap-2 flex items-center" >
              <span className="mono" style={{ fontSize: 13,
                        color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
                {trendUp ? "↗" : "↘"} {delta >= 0 ? "+" : ""}{delta}%
              </span>
              <span className="text-ink-3" style={{ fontSize: 13 }}>vs. last week</span>
            </div>
          </div>
        </div>

        {/* Trendline — Ma's sparkline, wide */}
        <div style={{ color: trendUp ? 'var(--success)' : 'var(--accent)' }} className="mt-4" >
          <Sparkline data={history} width={560} height={72} fill={trendUp ? 'var(--success-soft)' : 'var(--accent-soft)'} showDots/>
          <div className="mono mt-1 flex justify-between text-ink-3" style={{
 fontSize: 11 }}>
            <span>14d ago</span><span>7d ago</span><span>today</span>
          </div>
        </div>

        {/* Stats row */}
        <div style={{ borderRadius: 10 }} className="mt-8 py-4 px-4 flex justify-between border border-paper-edge bg-paper-2" >
          <MStat label="Sessions"  value={sol.sessions7d}     suffix="· 7d"/>
          <MDivider/>
          <MStat label="Tokens"    value={sol.tokens7d + "M"} suffix="· 7d"/>
          <MDivider/>
          <MStat label="Skills"    value={sol.activeSkills}   suffix="active"/>
          <MDivider/>
          <MStat label="Repos"     value={sol.repos.length}   suffix={sol.repos.join(' · ')}/>
        </div>

        {/* Recent sessions */}
        <div className="mt-8" >
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 text-ink-3 uppercase" >Recent sessions</div>
          <div>
            {solSessions.slice(0, 4).map((s, i) => (
              <button key={s.id}
 onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
 style={{ gridTemplateColumns: '10px 1fr auto auto',
 borderBottom: i < 3 ? 'var(--hairline)' : 'none' }} className="gap-3 py-3 px-0 grid items-center text-left w-full bg-transparent" >
                <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
                <div>
                  <div className="text-ink" style={{ fontSize: 13 }}>{s.title}</div>
                  <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                    {s.project} · {s.module}
                  </div>
                </div>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                  {s.turns}t · {s.duration}
                </span>
                <span className="text-ink-3" style={{ fontSize: 13 }}>→</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* RIGHT — coaching + signals */}
      <div style={{ zIndex: 1 }} className="gap-6 flex flex-col relative" >
        <div style={{ borderRadius: 14 }} className="py-6 px-6 bg-paper-2 border border-paper-edge" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-4 uppercase text-accent" >師 · sensei says</div>
          <p className="display m-0 font-light" style={{
 fontSize: 28, lineHeight: 1.25, textWrap: 'balance'
 }}>
            {topCoach.koan}
          </p>
          <p style={{ fontSize: 13, lineHeight: 1.6 }} className="mt-4 mb-6 text-ink-2" >
            {topCoach.body}
          </p>
          <div className="gap-3 flex items-center" >
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
 style={{
 background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--accent)',
 color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
 borderRadius: 8, fontSize: 13 }} className="py-2 px-4 font-medium" >
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action + " →"}
            </button>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {topCoach.impact}
            </span>
          </div>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-3 uppercase text-ink-3" >Quality signals</div>
          <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-2 grid" >
            {[
              { k: "Pattern compliance", v: "94%",     d: "+3",         good: true },
              { k: "Test coverage Δ",    v: "+2.1%",   d: "this week",  good: true },
              { k: "Doc drift",          v: "3 files", d: "brand-kit",  good: false },
              { k: "Tokens / session",   v: "14.2k",   d: "−1.8k",      good: true }
            ].map(s => (
              <div key={s.k} style={{ borderRadius: 8 }} className="p-3 border border-paper-edge bg-paper" >
                <div className="text-ink-3" style={{ fontSize: 11 }}>{s.k}</div>
                <div className="display mt-1 font-normal" style={{ fontSize: 22 }}>{s.v}</div>
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
          <div style={{ borderRadius: 10 }} className="p-4 border border-paper-edge bg-paper" >
            <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Also observed</div>
            <div className="text-ink-2 italic" style={{ fontSize: 13, lineHeight: 1.45 }}>
              "{data.coaching[1].koan}"
            </div>
            <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
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
      <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.1em' }}>{label}</div>
      <div className="display mt-1 font-normal" style={{ fontSize: 22 }}>{value}</div>
      <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>{suffix}</div>
    </div>
  );
}
function MDivider() { return <div style={{ width: 1, background: 'var(--edge)' }}/>; }

// Overview — reuse the solution cards from Enso but with trendlines instead of rings
function MergedOverview({ data, setPage, setActiveSolution }) {
  return (
    <div style={{ maxWidth: 1120 }} className="py-8 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >全 · Overview</div>
      <h1 className="display mt-0 mb-2 font-light" style={{ fontSize: 28 }}>
        All solutions
      </h1>
      <div style={{ fontSize: 13 }} className="mb-8 text-ink-3" >
        Global FTR <span className="mono text-ink" >78%</span> · week to date.
      </div>

      <div className="mb-8 text-accent" >
        <Sparkline data={data.ftrHistory} width={900} height={70} fill="var(--accent-soft)" showDots/>
        <div className="mono mt-1 flex justify-between text-ink-3" style={{
 fontSize: 11 }}>
          <span>Apr 9</span><span>Apr 16</span><span>Apr 22</span>
        </div>
      </div>

      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4 grid" >
        {data.solutions.map(s => {
          const up = s.ftr >= s.ftrPrev;
          return (
            <button key={s.id} onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
 style={{
 borderRadius: 12,
 transition: 'all .15s' }}
 onMouseEnter={e => { e.currentTarget.style.borderColor = 'var(--ink-3)'; e.currentTarget.style.transform = 'translateY(-2px)'; }}
 onMouseLeave={e => { e.currentTarget.style.borderColor = ''; e.currentTarget.style.transform = ''; }} className="p-6 border border-paper-edge bg-paper text-left relative overflow-hidden" >
              <span className="kanji absolute text-accent" style={{ top: -24, right: -18, fontSize: 56, opacity: 0.06, lineHeight: 1
 }}>{s.kanji}</span>
              <div className="display font-medium" style={{ fontSize: 17 }}>{s.name}</div>
              <div style={{ fontSize: 11 }} className="mb-4 text-ink-3" >{s.description}</div>
              <div className="display font-light" style={{ fontSize: 56, lineHeight: 1, letterSpacing: '-0.03em' }}>
                {Math.round(s.ftr*100)}<span className="text-ink-3" style={{ fontSize: 17 }}>%</span>
              </div>
              <div className="mono mt-1 text-ink-3 uppercase" style={{
 fontSize: 11,
 letterSpacing: '0.08em' }}>First try right</div>
              <div style={{ color: up ? 'var(--success)' : 'var(--accent)' }} className="mt-3" >
                <Sparkline data={data.ftrBySolution[s.id]} width={220} height={32} fill={up ? 'var(--success-soft)' : 'var(--accent-soft)'}/>
              </div>
              <hr className="hairline mt-4 mb-3 border-0" style={{ background: 'var(--edge)', height: 1 }}/>
              <div className="mono text-ink-3 flex justify-between" style={{ fontSize: 11 }}>
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
