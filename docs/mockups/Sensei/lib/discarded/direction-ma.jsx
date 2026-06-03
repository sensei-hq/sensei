// Direction 1 — MA (間) · negative space, one insight at a time.
// Big serif numerals, vast white space, a single shu accent, kanji watermarks.

const MaApp = () => {
  const data = window.SENSEI_DATA;
  const [page, setPage] = React.useState("observatory");
  const [activeSolution, setActiveSolution] = React.useState("lumen-cloud");
  const [focusedSession, setFocusedSession] = React.useState(null);
  const [appliedCoaching, setAppliedCoaching] = React.useState({});
  const [sessionFilter, setSessionFilter] = React.useState("all");
  const [hoverSpark, setHoverSpark] = React.useState(null);

  const sol = data.solutions.find(s => s.id === activeSolution);

  return (
    <div className="sensei" data-screen-label="Ma · Direction 1"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden',
                  fontFamily: 'var(--font-ui)' }}>
      <TauriChrome title="Sensei  先生" />
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <MaSidebar page={page} setPage={setPage}
                   solutions={data.solutions}
                   activeSolution={activeSolution} setActiveSolution={setActiveSolution} />
        <main style={{ flex: 1, overflow: 'auto', position: 'relative' }}>
          {page === "overview"    && <MaOverview data={data} setPage={setPage} setActiveSolution={setActiveSolution}/>}
          {page === "observatory" && <MaObservatory data={data} sol={sol} setPage={setPage}
                                                   setFocusedSession={setFocusedSession}
                                                   applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
          {page === "sessions"    && <MaSessions data={data} sol={sol}
                                                 filter={sessionFilter} setFilter={setSessionFilter}
                                                 focused={focusedSession} setFocused={setFocusedSession}/>}
          {page === "codebase"    && <MaCodebase data={data} sol={sol}/>}
          {page === "coaching"    && <MaCoaching data={data} sol={sol}
                                                 applied={appliedCoaching} setApplied={setAppliedCoaching}/>}
          {page === "config"      && <MaConfig data={data}/>}
          {page === "onboarding"  && <MaOnboarding/>}
        </main>
      </div>
    </div>
  );
};

// ────────────────────────────────────────────────────────────
// SIDEBAR
// ────────────────────────────────────────────────────────────
function MaSidebar({ page, setPage, solutions, activeSolution, setActiveSolution }) {
  return (
    <aside style={{
      width: 220, borderRight: 'var(--hairline)',
      display: 'flex', flexDirection: 'column',
      background: 'var(--paper)', flexShrink: 0
}} className="py-5 px-4 gap-5" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>先</span>
        <span className="display" style={{ fontSize: 17, fontWeight: 400 }}>Sensei</span>
      </div>

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
              <span className="kanji" style={{ fontSize: 13, color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)', width: 16 }}>{s.kanji}</span>
              <span style={{ flex: 1 }}>{s.name}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{pct(s.ftr)}</span>
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                      textTransform: 'uppercase'
}} className="mb-2" >View</div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          {PAGES.map(p => (
            <button key={p.id}
                    onClick={() => setPage(p.id)}
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

      <div style={{ flex: 1 }}/>

      <div style={{ display: 'flex', alignItems: 'center', fontSize: 13, color: 'var(--ink-3)' }} className="gap-2" >
        <Avatar name="Aiko" size={22}/>
        <div style={{ flex: 1, overflow: 'hidden' }}>
          <div style={{ color: 'var(--ink-2)', fontSize: 13 }}>Aiko</div>
          <div style={{ fontSize: 11 }}>daemon · 9823</div>
        </div>
        <span className="ink-dot" style={{ background: 'var(--success)' }}/>
      </div>
    </aside>
  );
}

// ────────────────────────────────────────────────────────────
// OBSERVATORY (solution dashboard) — the hero
// ────────────────────────────────────────────────────────────
function MaObservatory({ data, sol, setPage, setFocusedSession, applied, setApplied }) {
  const history = data.ftrBySolution[sol.id];
  const topCoach = data.coaching[0];
  const solSessions = data.sessions.filter(s => s.solution === sol.id).slice(0, 4);
  const delta = Math.round((sol.ftr - sol.ftrPrev) * 100);
  const trendUp = delta >= 0;

  return (
    <div style={{ position: 'relative', maxWidth: 1100 }} className="pt-7 pb-8 px-8" >
      {/* Huge kanji watermark */}
      <div className="kanji" style={{
        position: 'absolute', top: 40, right: 40, fontSize: 56,
        color: 'var(--accent)', opacity: 0.05, lineHeight: 1, userSelect: 'none',
        pointerEvents: 'none'
      }}>{sol.kanji}</div>

      {/* Breadcrumb */}
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >
        Observatory · {sol.name}
      </div>
      <h1 className="display my-1" style={{ fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em' }}>
        How am I doing?
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13 }} className="mb-7" >
        The week of April 16 — 22. Three repos, {sol.sessions7d} sessions, {sol.tokens7d}M tokens.
      </div>

      {/* Hero FTR number */}
      <div style={{ display: 'flex', alignItems: 'baseline', position: 'relative' }} className="gap-5 mb-3" >
        <div className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 0.9,
                                          letterSpacing: '-0.04em', fontFeatureSettings: '"ss01"' }}>
          {Math.round(sol.ftr * 100)}
          <span style={{ fontSize: 56, color: 'var(--ink-3)', fontWeight: 300 }} className="ml-1" >%</span>
        </div>
        <div className="pb-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)'
}} className="mb-1" >First try right</div>
          <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
            <span className="mono" style={{ fontSize: 13,
                      color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
              {trendUp ? "↗" : "↘"} {delta >= 0 ? "+" : ""}{delta}%
            </span>
            <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>vs. last week</span>
          </div>
          <div style={{ color: trendUp ? 'var(--success)' : 'var(--accent)' }} className="mt-3" >
            <Sparkline data={history} width={180} height={38} />
          </div>
        </div>
      </div>

      <hr className="hairline mt-7 mb-6"/>

      {/* The koan — coaching pulled front & center */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-7" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)'
}} className="mb-3" >Sensei says</div>
          <blockquote className="m-0" >
            <p className="display m-0" style={{
 fontSize: 28, fontWeight: 300, lineHeight: 1.25, color: 'var(--ink)', textWrap: 'balance'
}}>
              {topCoach.koan}
            </p>
            <p style={{
 fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55,
                        maxWidth: 420
}} className="mt-5 mb-0" >{topCoach.body}</p>
          </blockquote>

          <div style={{ display: 'flex', alignItems: 'center' }} className="mt-5 gap-3" >
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
                    style={{
 background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--ink)',
                      color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
                      borderRadius: 999, fontSize: 13, fontWeight: 500,
                      letterSpacing: '0.01em', transition: 'all .18s'
}} className="py-2 px-4" >
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action}
            </button>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>{topCoach.impact}</span>
          </div>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)'
}} className="mb-3" >Recent sessions</div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {solSessions.map((s, i) => (
              <button key={s.id}
                      onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
                      style={{
                        display: 'grid', gridTemplateColumns: '10px 1fr auto auto', alignItems: 'center', textAlign: 'left',
                        borderBottom: i < solSessions.length - 1 ? 'var(--hairline)' : 'none'
}} className="gap-3 py-3 px-0" >
                <span className="ink-dot"
                      style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)', width: 6, height: 6 }}/>
                <div>
                  <div style={{ fontSize: 13, color: 'var(--ink)', fontWeight: 400 }}>{s.title}</div>
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

      <hr className="hairline mt-7 mb-6"/>

      {/* Quality signals — understated grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-6 mb-2" >
        {[
          { label: "Pattern compliance", v: "94%", delta: "+3", good: true },
          { label: "Test coverage Δ",    v: "+2.1%", delta: "", good: true },
          { label: "Doc drift",          v: "3 files", delta: "brand-kit", good: false },
          { label: "Tokens / session",   v: "14.2k", delta: "−1.8k", good: true },
        ].map(s => (
          <div key={s.label}>
            <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                          textTransform: 'uppercase'
}} className="mb-2" >{s.label}</div>
            <div className="display" style={{ fontSize: 28, fontWeight: 300, letterSpacing: '-0.02em' }}>{s.v}</div>
            <div className="mono mt-1" style={{
 fontSize: 11, color: s.good ? 'var(--success)' : 'var(--accent)'
}}>{s.delta}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// OVERVIEW — all solutions at once
// ────────────────────────────────────────────────────────────
function MaOverview({ data, setPage, setActiveSolution }) {
  return (
    <div style={{ maxWidth: 1100 }} className="pt-7 pb-8 px-8" >
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >Overview</div>
      <h1 className="display mt-0 mb-1" style={{ fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em' }}>
        Three solutions. Eight repos.
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13 }} className="mb-7" >
        Global FTR <span className="mono" style={{ color: 'var(--ink)' }}>78%</span>, week to date.
      </div>

      {/* Global sparkline */}
      <div className="mb-7" >
        <div style={{ color: 'var(--accent)' }}>
          <Sparkline data={data.ftrHistory} width={800} height={60} />
        </div>
        <div className="mono mt-1" style={{
 display: 'flex', justifyContent: 'space-between',
                          fontSize: 11, color: 'var(--ink-3)'
}}>
          <span>Apr 9</span><span>Apr 16</span><span>Apr 22</span>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-5" >
        {data.solutions.map(s => (
          <button key={s.id}
                  onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
                  style={{
                    background: 'var(--paper)', border: 'var(--hairline)',
                    borderRadius: 10, textAlign: 'left',
                    position: 'relative', overflow: 'hidden',
                    transition: 'border-color .15s, transform .15s'
}}
                  onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--ink-3)'}
                  onMouseLeave={e => e.currentTarget.style.borderColor = ''} className="py-5 px-5" >
            <span className="kanji" style={{
              position: 'absolute', top: -30, right: -20, fontSize: 56,
              color: 'var(--accent)', opacity: 0.06, lineHeight: 1
            }}>{s.kanji}</span>
            <div className="display mb-1" style={{ fontSize: 17, fontWeight: 400 }}>{s.name}</div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mb-5" >{s.description}</div>
            <div className="display" style={{ fontSize: 56, fontWeight: 300, letterSpacing: '-0.03em', lineHeight: 1 }}>
              {Math.round(s.ftr*100)}<span style={{ fontSize: 17, color: 'var(--ink-3)' }}>%</span>
            </div>
            <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                          textTransform: 'uppercase'
}} className="mt-1" >First try right</div>
            <div style={{ color: s.ftr >= s.ftrPrev ? 'var(--success)' : 'var(--accent)' }} className="mt-4" >
              <Sparkline data={data.ftrBySolution[s.id]} width={200} height={28}/>
            </div>
            <hr className="hairline mt-4 mb-3"/>
            <div className="mono" style={{ display: 'flex', justifyContent: 'space-between',
                          fontSize: 11, color: 'var(--ink-3)' }}>
              <span>{s.repos.length} repos</span>
              <span>{s.sessions7d} sessions</span>
              <span>{s.tokens7d}M tok</span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// SESSIONS — list + drill-in
// ────────────────────────────────────────────────────────────
function MaSessions({ data, sol, filter, setFilter, focused, setFocused }) {
  const filters = [
    { id: "all", label: "All" },
    { id: "corrected", label: "Corrected" },
    { id: "first-try", label: "First try" },
    { id: "auth", label: "auth module" }
  ];
  const sessions = data.sessions.filter(s => {
    if (filter === "all") return true;
    if (filter === "corrected") return s.outcome === "corrected";
    if (filter === "first-try") return s.outcome === "first-try";
    if (filter === "auth") return s.module === "auth";
    return true;
  });

  const selectedSession = focused ? data.sessions.find(s => s.id === focused) : null;

  return (
    <div style={{ display: 'flex', height: '100%' }}>
      {/* Left list */}
      <div style={{
 flex: selectedSession ? '0 0 380px' : '1',
                    borderRight: selectedSession ? 'var(--hairline)' : 'none', overflow: 'auto'
}} className="pt-7 pb-6 px-6" >
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase'
}} className="mb-1" >Sessions · 刻</div>
        <h1 className="display mt-0 mb-5" style={{ fontSize: 28, fontWeight: 300 }}>
          Every session is a lesson.
        </h1>

        <div style={{ display: 'flex' }} className="gap-1 mb-5" >
          {filters.map(f => (
            <button key={f.id}
                    onClick={() => setFilter(f.id)}
                    style={{
 borderRadius: 999, fontSize: 11,
                      border: 'var(--hairline)',
                      background: filter === f.id ? 'var(--ink)' : 'transparent',
                      color: filter === f.id ? 'var(--paper)' : 'var(--ink-2)',
                      transition: 'all .12s'
}} className="py-1 px-2" >{f.label}</button>
          ))}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {sessions.map((s, i) => (
            <button key={s.id}
                    onClick={() => setFocused(s.id)}
                    style={{
                      textAlign: 'left',
                      borderBottom: i < sessions.length-1 ? 'var(--hairline)' : 'none',
                      background: focused === s.id ? 'var(--paper-3)' : 'transparent',
                      paddingLeft: focused === s.id ? 12 : 0,
                      margin: focused === s.id ? '0 -12px' : 0,
                      paddingRight: focused === s.id ? 12 : 0,
                      borderRadius: focused === s.id ? 6 : 0,
                      transition: 'all .12s'
}} className="py-3 px-0" >
              <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
                <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {s.date} · {s.started}
                </span>
                {!s.ftr && <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                  {s.corrections} corrections
                </span>}
              </div>
              <div style={{ fontSize: 13, color: 'var(--ink)' }} className="mb-1" >{s.title}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {s.project} · {s.turns}t · {s.duration}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Right detail */}
      {selectedSession && (
        <div style={{ flex: 1, overflow: 'auto', position: 'relative' }} className="py-7 px-7" >
          <button onClick={() => setFocused(null)}
                  style={{ position: 'absolute', top: 22, right: 22, fontSize: 17,
                           color: 'var(--ink-3)', width: 30, height: 30 }}>×</button>
          <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase'
}} className="mb-1" >
            Session {selectedSession.id}
          </div>
          <h2 className="display mt-0 mb-2" style={{ fontSize: 28, fontWeight: 300, letterSpacing: '-0.01em' }}>
            {selectedSession.title}
          </h2>
          <div className="mono mb-5" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {selectedSession.project} · {selectedSession.date} {selectedSession.started} · {selectedSession.duration} · {selectedSession.turns} turns · {(selectedSession.tokens/1000).toFixed(1)}k tokens
          </div>

          <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 8,
                        display: 'flex', alignItems: 'flex-start'
}} className="py-4 px-4 mb-6 gap-3" >
            <span className="kanji" style={{ fontSize: 22, color: selectedSession.ftr ? 'var(--success)' : 'var(--accent)', lineHeight: 1 }}>
              {selectedSession.ftr ? '一' : '修'}
            </span>
            <div>
              <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                            textTransform: 'uppercase'
}} className="mb-1" >
                {selectedSession.ftr ? 'First try right' : `Corrected · ${selectedSession.corrections}`}
              </div>
              <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }}>
                {selectedSession.summary}
              </div>
            </div>
          </div>

          {selectedSession.events && (
            <>
              <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                            textTransform: 'uppercase'
}} className="mb-4" >Event timeline</div>
              <div style={{ position: 'relative' }} className="pl-5" >
                <div style={{ position: 'absolute', left: 7, top: 4, bottom: 4, width: 1,
                              background: 'var(--edge)' }}/>
                {selectedSession.events.map((e, i) => (
                  <div key={i} style={{
 display: 'flex',
                                        position: 'relative'
}} className="gap-3 mb-3" >
                    <div style={{ position: 'absolute', left: -22, top: 2,
                                  color: e.kind === 'correction' ? 'var(--accent)'
                                       : e.kind === 'test' ? 'var(--success)'
                                       : 'var(--ink-3)' }}>
                      <div style={{ background: 'var(--paper)' }} className="p-1" >
                        <EventGlyph kind={e.kind}/>
                      </div>
                    </div>
                    <div className="mono pt-1" style={{ fontSize: 11, color: 'var(--ink-3)', width: 42 }}>
                      {e.t}
                    </div>
                    <div style={{ fontSize: 13, color: e.kind === 'correction' ? 'var(--accent)' : 'var(--ink-2)',
                                  lineHeight: 1.5 }}>
                      {e.text}
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CODEBASE
// ────────────────────────────────────────────────────────────
function MaCodebase({ data, sol }) {
  const [selectedRepo, setSelectedRepo] = React.useState(sol.repos[0]);
  return (
    <div style={{ maxWidth: 1100 }} className="py-7 px-7" >
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >Codebase · {sol.name}</div>
      <h1 className="display mt-0 mb-6" style={{ fontSize: 28, fontWeight: 300 }}>
        Where the weight gathers.
      </h1>

      <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1 mb-6" >
        {sol.repos.map(r => (
          <button key={r}
                  onClick={() => setSelectedRepo(r)}
                  className="mono py-1 px-3"
                  style={{
 borderRadius: 999, fontSize: 11,
                    border: 'var(--hairline)',
                    background: selectedRepo === r ? 'var(--ink)' : 'transparent',
                    color: selectedRepo === r ? 'var(--paper)' : 'var(--ink-2)'
}}>{r}</button>
        ))}
      </div>

      {/* Graph placeholder — abstract constellation */}
      <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                    borderRadius: 10, minHeight: 280,
                    position: 'relative', overflow: 'hidden'
}} className="p-5 mb-6" >
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                      textTransform: 'uppercase'
}} className="mb-1" >Code graph</div>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {selectedRepo} · 247 nodes · 4 communities
        </div>
        <svg viewBox="0 0 600 240" width="100%" height="240" className="mt-2" >
          {/* static constellation */}
          {Array.from({length: 60}).map((_, i) => {
            const x = 40 + (i * 37) % 540 + Math.sin(i) * 20;
            const y = 30 + ((i * 53) % 180) + Math.cos(i*0.7) * 10;
            const big = i % 11 === 0;
            return <circle key={i} cx={x} cy={y} r={big ? 5 : 2} fill="currentColor"
                           opacity={big ? 0.8 : 0.25} style={{ color: 'var(--ink-2)' }}/>;
          })}
          {/* a few highlighted god nodes */}
          <circle cx="180" cy="90"  r="12" fill="oklch(0.58 0.15 35 / 0.15)" stroke="var(--accent)" strokeWidth="1.5"/>
          <circle cx="380" cy="150" r="10" fill="oklch(0.58 0.15 35 / 0.15)" stroke="var(--accent)" strokeWidth="1.5"/>
          <text x="196" y="94" fontSize="10" fill="var(--accent)" fontFamily="var(--font-mono)">router.ts</text>
          <text x="396" y="154" fontSize="10" fill="var(--accent)" fontFamily="var(--font-mono)">session.ts</text>
          {/* connection lines — subtle */}
          <g stroke="var(--ink-4)" strokeWidth="0.5" opacity="0.4" fill="none">
            <path d="M180 90 Q 260 120 380 150"/>
            <path d="M180 90 Q 140 140 100 200"/>
            <path d="M380 150 Q 460 180 520 130"/>
          </g>
        </svg>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr' }} className="gap-7" >
        <div>
          <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase'
}} className="mb-3" >Hotspots</div>
          {data.hotspots.map((h, i) => (
            <div key={i} style={{
 borderBottom: i < data.hotspots.length - 1 ? 'var(--hairline)' : 'none',
                                 display: 'grid', gridTemplateColumns: '14px 1fr auto auto auto', alignItems: 'center'
}} className="gap-3 py-3 px-0" >
              <span className="ink-dot" style={{
                background: h.severity === 'god' ? 'var(--accent)' :
                            h.severity === 'cluster' ? 'var(--warning)' : 'var(--success)'
              }}/>
              <span className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{h.name}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>in {h.fanIn}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>out {h.fanOut}</span>
              <span className="mono" style={{ fontSize: 11, color: h.rework > 3 ? 'var(--accent)' : 'var(--ink-3)' }}>
                ↻{h.rework}
              </span>
            </div>
          ))}
        </div>
        <div>
          <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase'
}} className="mb-3" >Health</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
            {[
              { l: "Dead code",      v: "14 exports" },
              { l: "Test ratio",     v: "0.72 : 1" },
              { l: "Largest file",   v: "router.ts · 812 ln" },
              { l: "Last indexed",   v: "2m ago" }
            ].map(r => (
              <div key={r.l} style={{ display: 'flex', justifyContent: 'space-between' }}>
                <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>{r.l}</span>
                <span className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>{r.v}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// COACHING
// ────────────────────────────────────────────────────────────
function MaCoaching({ data, sol, applied, setApplied }) {
  return (
    <div style={{ maxWidth: 880 }} className="py-7 px-8" >
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >Coaching · 師</div>
      <h1 className="display mt-0 mb-1" style={{ fontSize: 28, fontWeight: 300 }}>
        What the sessions are teaching.
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13 }} className="mb-7" >
        Three observations, in descending urgency.
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-0" >
        {data.coaching.map((c, i) => {
          const isApplied = applied[c.id];
          return (
            <div key={c.id}
                 style={{
 borderTop: 'var(--hairline)',
                          borderBottom: i === data.coaching.length - 1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '72px 1fr 180px'
}} className="gap-5 py-6 px-0" >
              <div>
                <div className="display" style={{ fontSize: 56, fontWeight: 300, color: 'var(--accent)',
                              opacity: c.urgency === 'high' ? 1 : c.urgency === 'medium' ? 0.5 : 0.25,
                              lineHeight: 1 }}>
                  0{i+1}
                </div>
                <div className="mono mt-1" style={{
 fontSize: 11, color: 'var(--ink-3)',
                              letterSpacing: '0.12em', textTransform: 'uppercase'
}}>
                  {c.urgency}
                </div>
              </div>
              <div>
                <p className="display mt-0 mb-3" style={{ fontSize: 22, fontWeight: 300, lineHeight: 1.3 }}>
                  {c.koan}
                </p>
                <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6 }} className="m-0" >
                  {c.body}
                </p>
                <div className="mono mt-3" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  module: {c.module}
                </div>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start' }} className="gap-2" >
                <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
                        style={{
 borderRadius: 999, fontSize: 13,
                          background: isApplied ? 'var(--success-soft)' : 'var(--ink)',
                          color: isApplied ? 'var(--success)' : 'var(--paper)',
                          fontWeight: 500, width: '100%', textAlign: 'center',
                          transition: 'all .18s'
}} className="py-2 px-4" >
                  {isApplied ? "✓  Applied" : c.action}
                </button>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>{c.actionDetail}</div>
                <div style={{ fontSize: 11, color: 'var(--ink-2)', fontStyle: 'italic' }}>{c.impact}</div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Personas */}
      <div className="mt-7" >
        <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase'
}} className="mb-3" >Active personas</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2,1fr)' }} className="gap-2" >
          {data.personas.map(p => (
            <div key={p.id} style={{
 border: 'var(--hairline)',
                          borderRadius: 6, background: 'var(--paper-2)'
}} className="p-3" >
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{p.name}</div>
              <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {p.triggers}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CONFIG
// ────────────────────────────────────────────────────────────
function MaConfig({ data }) {
  const [tab, setTab] = React.useState("skills");
  return (
    <div style={{ maxWidth: 960 }} className="py-7 px-7" >
      <div style={{
 fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase'
}} className="mb-1" >Configuration · 設</div>
      <h1 className="display mt-0 mb-5" style={{ fontSize: 28, fontWeight: 300 }}>
        What sensei is allowed to do.
      </h1>

      <div style={{ display: 'flex', borderBottom: 'var(--hairline)' }} className="gap-4 mb-5" >
        {[
          { id: "skills",    label: "Skills" },
          { id: "libraries", label: "Libraries" },
          { id: "acps",      label: "ACPs" },
          { id: "daemon",    label: "Daemon" }
        ].map(t => (
          <button key={t.id}
                  onClick={() => setTab(t.id)}
                  style={{
 fontSize: 13,
                    color: tab === t.id ? 'var(--ink)' : 'var(--ink-3)',
                    borderBottom: tab === t.id ? '1.5px solid var(--accent)' : '1.5px solid transparent',
                    marginBottom: -1
}} className="py-2 px-0" >{t.label}</button>
        ))}
      </div>

      {tab === "skills" && (
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {data.skills.map((s, i) => (
            <div key={s.id} style={{
 borderBottom: i < data.skills.length-1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '1fr auto auto', alignItems: 'center'
}} className="gap-4 py-3 px-0" >
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{s.name}</div>
                <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {s.solutions.length ? `active in ${s.solutions.join(', ')}` : 'not installed'}
                </div>
              </div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.id}</span>
              <div style={{ width: 32, height: 18, borderRadius: 999,
                            background: s.active ? 'var(--accent)' : 'var(--paper-3)',
                            position: 'relative', transition: 'background .15s' }}>
                <div style={{ position: 'absolute', top: 2, left: s.active ? 16 : 2,
                            width: 14, height: 14, background: 'var(--paper)',
                            borderRadius: '50%', transition: 'left .15s' }}/>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "libraries" && (
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {data.libraries.map((l, i) => (
            <div key={l.name} style={{
 borderBottom: i < data.libraries.length-1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '1fr auto auto auto', alignItems: 'center'
}} className="gap-4 py-3 px-0" >
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{l.name}</div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>v{l.version}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.pages} pages</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.lastIndexed}</span>
            </div>
          ))}
          <button style={{
 alignSelf: 'flex-start', borderRadius: 999, fontSize: 13,
                          border: '1px dashed var(--ink-3)', color: 'var(--ink-2)'
}} className="mt-4 py-2 px-4" >
            + Index a library
          </button>
        </div>
      )}

      {tab === "acps" && (
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
          {[
            { name: "Claude Code",  version: "1.8.2", status: "connected" },
            { name: "Cursor",       version: "0.42",  status: "connected" },
            { name: "Zed",          version: "0.148", status: "available" }
          ].map(a => (
            <div key={a.name} style={{
 border: 'var(--hairline)', borderRadius: 8,
                        display: 'flex', alignItems: 'center'
}} className="p-4 gap-4" >
              <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--success)' : 'var(--ink-4)' }}/>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 13 }}>{a.name}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  MCP · v{a.version} · {a.status}
                </div>
              </div>
              <button style={{
 fontSize: 11, borderRadius: 999,
                              border: 'var(--hairline)', color: 'var(--ink-2)'
}} className="py-1 px-3" >
                {a.status === 'connected' ? 'Configure' : 'Connect'}
              </button>
            </div>
          ))}
        </div>
      )}

      {tab === "daemon" && (
        <div>
          <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 8
}} className="p-4 mb-4" >
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-2" >
              <span className="ink-dot" style={{ background: 'var(--success)' }}/>
              <span style={{ fontSize: 13 }}>Daemon running</span>
              <span className="mono ml-auto" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                pid 12492 · uptime 4d 2h
              </span>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)' }} className="gap-3 mt-4" >
              <div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>Port</div>
                <div className="mono" style={{ fontSize: 13 }}>9823</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>Events today</div>
                <div className="mono" style={{ fontSize: 13 }}>1,842</div>
              </div>
              <div>
                <div style={{ fontSize: 11, color: 'var(--ink-3)' }}>Memory</div>
                <div className="mono" style={{ fontSize: 13 }}>42 MB</div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// ONBOARDING
// ────────────────────────────────────────────────────────────
function MaOnboarding() {
  const [step, setStep] = React.useState(1);
  const steps = [
    { n: 1, title: "Find assistants",  detail: "detect Claude Code, Cursor, Zed · register MCP" },
    { n: 2, title: "Scan folders",     detail: "discover git repos" },
    { n: 3, title: "Group into solutions", detail: "auto-match, confirm, assign roles" },
    { n: 4, title: "First index",      detail: "extract graph, compute baseline" }
  ];

  return (
    <div style={{ maxWidth: 820 }} className="py-8 px-8" >
      <div className="kanji mb-5" style={{
 fontSize: 56, color: 'var(--accent)', opacity: 0.85, lineHeight: 1
}}>始</div>
      <h1 className="display mt-0 mb-3" style={{ fontSize: 40, fontWeight: 300, letterSpacing: '-0.02em' }}>
        Begin.
      </h1>
      <p style={{ color: 'var(--ink-2)', fontSize: 15, lineHeight: 1.6, maxWidth: 520 }} className="mb-7" >
        Sensei will watch how you work and, in time, help you work better.
        Four steps, each takes a minute.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
        {steps.map(s => (
          <button key={s.n}
                  onClick={() => setStep(s.n)}
                  style={{
                    display: 'grid', gridTemplateColumns: '48px 1fr auto', textAlign: 'left', alignItems: 'center',
                    borderTop: 'var(--hairline)',
                    borderBottom: s.n === 4 ? 'var(--hairline)' : 'none',
                    opacity: s.n > step ? 0.4 : 1, transition: 'opacity .2s'
}} className="gap-4 py-4 px-1" >
            <div className="display" style={{ fontSize: 28, fontWeight: 300,
                        color: s.n < step ? 'var(--success)' : s.n === step ? 'var(--accent)' : 'var(--ink-3)',
                        lineHeight: 1 }}>
              {s.n < step ? '✓' : '0' + s.n}
            </div>
            <div>
              <div className="display" style={{ fontSize: 22, fontWeight: 300 }}>{s.title}</div>
              <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mt-1" >{s.detail}</div>
            </div>
            {s.n === step && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>in progress</span>
            )}
            {s.n < step && <span className="mono" style={{ fontSize: 11, color: 'var(--success)' }}>done</span>}
          </button>
        ))}
      </div>

      <div style={{ display: 'flex' }} className="gap-2 mt-6" >
        <button onClick={() => setStep(Math.min(4, step+1))}
                style={{
 background: 'var(--ink)', color: 'var(--paper)',
                          borderRadius: 999, fontSize: 13, fontWeight: 500
}} className="py-3 px-5" >
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
        <button style={{ color: 'var(--ink-3)', fontSize: 13 }} className="py-3 px-4" >
          Skip for now
        </button>
      </div>
    </div>
  );
}

window.MaApp = MaApp;
