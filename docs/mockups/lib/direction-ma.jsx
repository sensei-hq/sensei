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
      width: 220, borderRight: 'var(--hairline)', padding: '24px 16px',
      display: 'flex', flexDirection: 'column', gap: 24,
      background: 'var(--paper)', flexShrink: 0
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
        <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>先</span>
        <span className="display" style={{ fontSize: 17, fontWeight: 400 }}>Sensei</span>
      </div>

      <div>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                      textTransform: 'uppercase', marginBottom: 8 }}>Solutions</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {solutions.map(s => (
            <button key={s.id}
                    onClick={() => setActiveSolution(s.id)}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 8,
                      padding: '8px 8px', borderRadius: 6, textAlign: 'left',
                      background: activeSolution === s.id ? 'var(--paper-3)' : 'transparent',
                      color: activeSolution === s.id ? 'var(--ink)' : 'var(--ink-2)',
                      fontSize: 13, transition: 'background .12s'
                    }}>
              <span className="kanji" style={{ fontSize: 13, color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)', width: 16 }}>{s.kanji}</span>
              <span style={{ flex: 1 }}>{s.name}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{pct(s.ftr)}</span>
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                      textTransform: 'uppercase', marginBottom: 8 }}>View</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {PAGES.map(p => (
            <button key={p.id}
                    onClick={() => setPage(p.id)}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 8,
                      padding: '8px 8px', borderRadius: 6, textAlign: 'left',
                      background: page === p.id ? 'var(--ink)' : 'transparent',
                      color: page === p.id ? 'var(--paper)' : 'var(--ink-2)',
                      fontSize: 13, transition: 'background .12s'
                    }}>
              <span className="kanji" style={{ fontSize: 13, width: 14, opacity: 0.7 }}>{p.kanji}</span>
              <span>{p.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div style={{ flex: 1 }}/>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: 'var(--ink-3)' }}>
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
    <div style={{ padding: '48px 64px 64px', position: 'relative', maxWidth: 1100 }}>
      {/* Huge kanji watermark */}
      <div className="kanji" style={{
        position: 'absolute', top: 40, right: 40, fontSize: 56,
        color: 'var(--accent)', opacity: 0.05, lineHeight: 1, userSelect: 'none',
        pointerEvents: 'none'
      }}>{sol.kanji}</div>

      {/* Breadcrumb */}
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>
        Observatory · {sol.name}
      </div>
      <h1 className="display" style={{ fontSize: 40, fontWeight: 300, margin: '4px 0 4px', letterSpacing: '-0.02em' }}>
        How am I doing?
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13, marginBottom: 48 }}>
        The week of April 16 — 22. Three repos, {sol.sessions7d} sessions, {sol.tokens7d}M tokens.
      </div>

      {/* Hero FTR number */}
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 24, marginBottom: 12, position: 'relative' }}>
        <div className="display" style={{ fontSize: 56, fontWeight: 300, lineHeight: 0.9,
                                          letterSpacing: '-0.04em', fontFeatureSettings: '"ss01"' }}>
          {Math.round(sol.ftr * 100)}
          <span style={{ fontSize: 56, color: 'var(--ink-3)', fontWeight: 300, marginLeft: 4 }}>%</span>
        </div>
        <div style={{ paddingBottom: 16 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)', marginBottom: 4 }}>First try right</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span className="mono" style={{ fontSize: 13,
                      color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
              {trendUp ? "↗" : "↘"} {delta >= 0 ? "+" : ""}{delta}%
            </span>
            <span style={{ fontSize: 13, color: 'var(--ink-3)' }}>vs. last week</span>
          </div>
          <div style={{ marginTop: 12, color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
            <Sparkline data={history} width={180} height={38} />
          </div>
        </div>
      </div>

      <hr className="hairline" style={{ margin: '48px 0 32px' }}/>

      {/* The koan — coaching pulled front & center */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 48 }}>
        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)', marginBottom: 12 }}>Sensei says</div>
          <blockquote style={{ margin: 0 }}>
            <p className="display" style={{ fontSize: 28, fontWeight: 300, lineHeight: 1.25,
                                            margin: 0, color: 'var(--ink)', textWrap: 'balance' }}>
              {topCoach.koan}
            </p>
            <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55, margin: '24px 0 0',
                        maxWidth: 420 }}>{topCoach.body}</p>
          </blockquote>

          <div style={{ marginTop: 24, display: 'flex', gap: 12, alignItems: 'center' }}>
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
                    style={{
                      padding: '8px 16px', background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--ink)',
                      color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
                      borderRadius: 999, fontSize: 13, fontWeight: 500,
                      letterSpacing: '0.01em', transition: 'all .18s'
                    }}>
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action}
            </button>
            <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>{topCoach.impact}</span>
          </div>
        </div>

        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.12em', textTransform: 'uppercase',
                        color: 'var(--ink-3)', marginBottom: 12 }}>Recent sessions</div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {solSessions.map((s, i) => (
              <button key={s.id}
                      onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
                      style={{
                        display: 'grid', gridTemplateColumns: '10px 1fr auto auto',
                        gap: 12, padding: '12px 0', alignItems: 'center', textAlign: 'left',
                        borderBottom: i < solSessions.length - 1 ? 'var(--hairline)' : 'none'
                      }}>
                <span className="ink-dot"
                      style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)', width: 6, height: 6 }}/>
                <div>
                  <div style={{ fontSize: 13, color: 'var(--ink)', fontWeight: 400 }}>{s.title}</div>
                  <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
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

      <hr className="hairline" style={{ margin: '48px 0 32px' }}/>

      {/* Quality signals — understated grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 32, marginBottom: 8 }}>
        {[
          { label: "Pattern compliance", v: "94%", delta: "+3", good: true },
          { label: "Test coverage Δ",    v: "+2.1%", delta: "", good: true },
          { label: "Doc drift",          v: "3 files", delta: "brand-kit", good: false },
          { label: "Tokens / session",   v: "14.2k", delta: "−1.8k", good: true },
        ].map(s => (
          <div key={s.label}>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                          textTransform: 'uppercase', marginBottom: 8 }}>{s.label}</div>
            <div className="display" style={{ fontSize: 28, fontWeight: 300, letterSpacing: '-0.02em' }}>{s.v}</div>
            <div className="mono" style={{ fontSize: 11, color: s.good ? 'var(--success)' : 'var(--accent)',
                          marginTop: 4 }}>{s.delta}</div>
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
    <div style={{ padding: '48px 64px 64px', maxWidth: 1100 }}>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>Overview</div>
      <h1 className="display" style={{ fontSize: 40, fontWeight: 300, margin: '0 0 4px', letterSpacing: '-0.02em' }}>
        Three solutions. Eight repos.
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13, marginBottom: 48 }}>
        Global FTR <span className="mono" style={{ color: 'var(--ink)' }}>78%</span>, week to date.
      </div>

      {/* Global sparkline */}
      <div style={{ marginBottom: 48 }}>
        <div style={{ color: 'var(--accent)' }}>
          <Sparkline data={data.ftrHistory} width={800} height={60} />
        </div>
        <div className="mono" style={{ display: 'flex', justifyContent: 'space-between',
                          fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
          <span>Apr 9</span><span>Apr 16</span><span>Apr 22</span>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 24 }}>
        {data.solutions.map(s => (
          <button key={s.id}
                  onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
                  style={{
                    background: 'var(--paper)', border: 'var(--hairline)',
                    borderRadius: 10, padding: '24px 24px', textAlign: 'left',
                    position: 'relative', overflow: 'hidden',
                    transition: 'border-color .15s, transform .15s'
                  }}
                  onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--ink-3)'}
                  onMouseLeave={e => e.currentTarget.style.borderColor = ''}>
            <span className="kanji" style={{
              position: 'absolute', top: -30, right: -20, fontSize: 56,
              color: 'var(--accent)', opacity: 0.06, lineHeight: 1
            }}>{s.kanji}</span>
            <div className="display" style={{ fontSize: 17, fontWeight: 400, marginBottom: 4 }}>{s.name}</div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', marginBottom: 24 }}>{s.description}</div>
            <div className="display" style={{ fontSize: 56, fontWeight: 300, letterSpacing: '-0.03em', lineHeight: 1 }}>
              {Math.round(s.ftr*100)}<span style={{ fontSize: 17, color: 'var(--ink-3)' }}>%</span>
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4, letterSpacing: '0.08em',
                          textTransform: 'uppercase' }}>First try right</div>
            <div style={{ marginTop: 16, color: s.ftr >= s.ftrPrev ? 'var(--success)' : 'var(--accent)' }}>
              <Sparkline data={data.ftrBySolution[s.id]} width={200} height={28}/>
            </div>
            <hr className="hairline" style={{ margin: '16px 0 12px' }}/>
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
      <div style={{ flex: selectedSession ? '0 0 380px' : '1', padding: '48px 32px 32px',
                    borderRight: selectedSession ? 'var(--hairline)' : 'none', overflow: 'auto' }}>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase', marginBottom: 4 }}>Sessions · 刻</div>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 24px' }}>
          Every session is a lesson.
        </h1>

        <div style={{ display: 'flex', gap: 4, marginBottom: 24 }}>
          {filters.map(f => (
            <button key={f.id}
                    onClick={() => setFilter(f.id)}
                    style={{
                      padding: '4px 8px', borderRadius: 999, fontSize: 11,
                      border: 'var(--hairline)',
                      background: filter === f.id ? 'var(--ink)' : 'transparent',
                      color: filter === f.id ? 'var(--paper)' : 'var(--ink-2)',
                      transition: 'all .12s'
                    }}>{f.label}</button>
          ))}
        </div>

        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {sessions.map((s, i) => (
            <button key={s.id}
                    onClick={() => setFocused(s.id)}
                    style={{
                      textAlign: 'left', padding: '12px 0',
                      borderBottom: i < sessions.length-1 ? 'var(--hairline)' : 'none',
                      background: focused === s.id ? 'var(--paper-3)' : 'transparent',
                      paddingLeft: focused === s.id ? 12 : 0,
                      margin: focused === s.id ? '0 -12px' : 0,
                      paddingRight: focused === s.id ? 12 : 0,
                      borderRadius: focused === s.id ? 6 : 0,
                      transition: 'all .12s'
                    }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {s.date} · {s.started}
                </span>
                {!s.ftr && <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                  {s.corrections} corrections
                </span>}
              </div>
              <div style={{ fontSize: 13, color: 'var(--ink)', marginBottom: 4 }}>{s.title}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                {s.project} · {s.turns}t · {s.duration}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Right detail */}
      {selectedSession && (
        <div style={{ flex: 1, padding: '48px 48px', overflow: 'auto', position: 'relative' }}>
          <button onClick={() => setFocused(null)}
                  style={{ position: 'absolute', top: 22, right: 22, fontSize: 17,
                           color: 'var(--ink-3)', width: 30, height: 30 }}>×</button>
          <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase', marginBottom: 4 }}>
            Session {selectedSession.id}
          </div>
          <h2 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 8px', letterSpacing: '-0.01em' }}>
            {selectedSession.title}
          </h2>
          <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginBottom: 24 }}>
            {selectedSession.project} · {selectedSession.date} {selectedSession.started} · {selectedSession.duration} · {selectedSession.turns} turns · {(selectedSession.tokens/1000).toFixed(1)}k tokens
          </div>

          <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 8, padding: '16px 16px', marginBottom: 32,
                        display: 'flex', alignItems: 'flex-start', gap: 12 }}>
            <span className="kanji" style={{ fontSize: 22, color: selectedSession.ftr ? 'var(--success)' : 'var(--accent)', lineHeight: 1 }}>
              {selectedSession.ftr ? '一' : '修'}
            </span>
            <div>
              <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                            textTransform: 'uppercase', marginBottom: 4 }}>
                {selectedSession.ftr ? 'First try right' : `Corrected · ${selectedSession.corrections}`}
              </div>
              <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }}>
                {selectedSession.summary}
              </div>
            </div>
          </div>

          {selectedSession.events && (
            <>
              <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                            textTransform: 'uppercase', marginBottom: 16 }}>Event timeline</div>
              <div style={{ position: 'relative', paddingLeft: 24 }}>
                <div style={{ position: 'absolute', left: 7, top: 4, bottom: 4, width: 1,
                              background: 'var(--edge)' }}/>
                {selectedSession.events.map((e, i) => (
                  <div key={i} style={{ display: 'flex', gap: 12, marginBottom: 12,
                                        position: 'relative' }}>
                    <div style={{ position: 'absolute', left: -22, top: 2,
                                  color: e.kind === 'correction' ? 'var(--accent)'
                                       : e.kind === 'test' ? 'var(--success)'
                                       : 'var(--ink-3)' }}>
                      <div style={{ background: 'var(--paper)', padding: 4 }}>
                        <EventGlyph kind={e.kind}/>
                      </div>
                    </div>
                    <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', width: 42, paddingTop: 4 }}>
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
    <div style={{ padding: '48px 48px', maxWidth: 1100 }}>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>Codebase · {sol.name}</div>
      <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 32px' }}>
        Where the weight gathers.
      </h1>

      <div style={{ display: 'flex', gap: 4, marginBottom: 32, flexWrap: 'wrap' }}>
        {sol.repos.map(r => (
          <button key={r}
                  onClick={() => setSelectedRepo(r)}
                  className="mono"
                  style={{
                    padding: '4px 12px', borderRadius: 999, fontSize: 11,
                    border: 'var(--hairline)',
                    background: selectedRepo === r ? 'var(--ink)' : 'transparent',
                    color: selectedRepo === r ? 'var(--paper)' : 'var(--ink-2)'
                  }}>{r}</button>
        ))}
      </div>

      {/* Graph placeholder — abstract constellation */}
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                    borderRadius: 10, padding: 24, marginBottom: 32, minHeight: 280,
                    position: 'relative', overflow: 'hidden' }}>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.08em',
                      textTransform: 'uppercase', marginBottom: 4 }}>Code graph</div>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {selectedRepo} · 247 nodes · 4 communities
        </div>
        <svg viewBox="0 0 600 240" width="100%" height="240" style={{ marginTop: 8 }}>
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

      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 48 }}>
        <div>
          <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase', marginBottom: 12 }}>Hotspots</div>
          {data.hotspots.map((h, i) => (
            <div key={i} style={{ padding: '12px 0', borderBottom: i < data.hotspots.length - 1 ? 'var(--hairline)' : 'none',
                                 display: 'grid', gridTemplateColumns: '14px 1fr auto auto auto', gap: 12, alignItems: 'center' }}>
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
          <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase', marginBottom: 12 }}>Health</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
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
    <div style={{ padding: '48px 64px', maxWidth: 880 }}>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>Coaching · 師</div>
      <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 4px' }}>
        What the sessions are teaching.
      </h1>
      <div style={{ color: 'var(--ink-3)', fontSize: 13, marginBottom: 48 }}>
        Three observations, in descending urgency.
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
        {data.coaching.map((c, i) => {
          const isApplied = applied[c.id];
          return (
            <div key={c.id}
                 style={{ padding: '32px 0', borderTop: 'var(--hairline)',
                          borderBottom: i === data.coaching.length - 1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '72px 1fr 180px', gap: 24 }}>
              <div>
                <div className="display" style={{ fontSize: 56, fontWeight: 300, color: 'var(--accent)',
                              opacity: c.urgency === 'high' ? 1 : c.urgency === 'medium' ? 0.5 : 0.25,
                              lineHeight: 1 }}>
                  0{i+1}
                </div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4,
                              letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                  {c.urgency}
                </div>
              </div>
              <div>
                <p className="display" style={{ fontSize: 22, fontWeight: 300, margin: '0 0 12px', lineHeight: 1.3 }}>
                  {c.koan}
                </p>
                <p style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.6, margin: 0 }}>
                  {c.body}
                </p>
                <div className="mono" style={{ marginTop: 12, fontSize: 11, color: 'var(--ink-3)' }}>
                  module: {c.module}
                </div>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8, alignItems: 'flex-start' }}>
                <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
                        style={{
                          padding: '8px 16px', borderRadius: 999, fontSize: 13,
                          background: isApplied ? 'var(--success-soft)' : 'var(--ink)',
                          color: isApplied ? 'var(--success)' : 'var(--paper)',
                          fontWeight: 500, width: '100%', textAlign: 'center',
                          transition: 'all .18s'
                        }}>
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
      <div style={{ marginTop: 48 }}>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase', marginBottom: 12 }}>Active personas</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2,1fr)', gap: 8 }}>
          {data.personas.map(p => (
            <div key={p.id} style={{ padding: 12, border: 'var(--hairline)',
                          borderRadius: 6, background: 'var(--paper-2)' }}>
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{p.name}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
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
    <div style={{ padding: '48px 48px', maxWidth: 960 }}>
      <div style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>Configuration · 設</div>
      <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 24px' }}>
        What sensei is allowed to do.
      </h1>

      <div style={{ display: 'flex', gap: 16, borderBottom: 'var(--hairline)', marginBottom: 24 }}>
        {[
          { id: "skills",    label: "Skills" },
          { id: "libraries", label: "Libraries" },
          { id: "acps",      label: "ACPs" },
          { id: "daemon",    label: "Daemon" }
        ].map(t => (
          <button key={t.id}
                  onClick={() => setTab(t.id)}
                  style={{
                    padding: '8px 0', fontSize: 13,
                    color: tab === t.id ? 'var(--ink)' : 'var(--ink-3)',
                    borderBottom: tab === t.id ? '1.5px solid var(--accent)' : '1.5px solid transparent',
                    marginBottom: -1
                  }}>{t.label}</button>
        ))}
      </div>

      {tab === "skills" && (
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {data.skills.map((s, i) => (
            <div key={s.id} style={{ padding: '12px 0', borderBottom: i < data.skills.length-1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '1fr auto auto', gap: 16, alignItems: 'center' }}>
              <div>
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>{s.name}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
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
            <div key={l.name} style={{ padding: '12px 0', borderBottom: i < data.libraries.length-1 ? 'var(--hairline)' : 'none',
                          display: 'grid', gridTemplateColumns: '1fr auto auto auto', gap: 16, alignItems: 'center' }}>
              <div style={{ fontSize: 13, color: 'var(--ink)' }}>{l.name}</div>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>v{l.version}</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.pages} pages</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{l.lastIndexed}</span>
            </div>
          ))}
          <button style={{ marginTop: 16, alignSelf: 'flex-start',
                          padding: '8px 16px', borderRadius: 999, fontSize: 13,
                          border: '1px dashed var(--ink-3)', color: 'var(--ink-2)' }}>
            + Index a library
          </button>
        </div>
      )}

      {tab === "acps" && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {[
            { name: "Claude Code",  version: "1.8.2", status: "connected" },
            { name: "Cursor",       version: "0.42",  status: "connected" },
            { name: "Zed",          version: "0.148", status: "available" }
          ].map(a => (
            <div key={a.name} style={{ padding: 16, border: 'var(--hairline)', borderRadius: 8,
                        display: 'flex', alignItems: 'center', gap: 16 }}>
              <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--success)' : 'var(--ink-4)' }}/>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 13 }}>{a.name}</div>
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  MCP · v{a.version} · {a.status}
                </div>
              </div>
              <button style={{ fontSize: 11, padding: '4px 12px', borderRadius: 999,
                              border: 'var(--hairline)', color: 'var(--ink-2)' }}>
                {a.status === 'connected' ? 'Configure' : 'Connect'}
              </button>
            </div>
          ))}
        </div>
      )}

      {tab === "daemon" && (
        <div>
          <div style={{ padding: 16, background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 8, marginBottom: 16 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
              <span className="ink-dot" style={{ background: 'var(--success)' }}/>
              <span style={{ fontSize: 13 }}>Daemon running</span>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)', marginLeft: 'auto' }}>
                pid 12492 · uptime 4d 2h
              </span>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 12, marginTop: 16 }}>
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
    <div style={{ padding: '64px 64px', maxWidth: 820 }}>
      <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', opacity: 0.85,
                          marginBottom: 24, lineHeight: 1 }}>始</div>
      <h1 className="display" style={{ fontSize: 40, fontWeight: 300, margin: '0 0 12px', letterSpacing: '-0.02em' }}>
        Begin.
      </h1>
      <p style={{ color: 'var(--ink-2)', fontSize: 15, lineHeight: 1.6, marginBottom: 48, maxWidth: 520 }}>
        Sensei will watch how you work and, in time, help you work better.
        Four steps, each takes a minute.
      </p>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {steps.map(s => (
          <button key={s.n}
                  onClick={() => setStep(s.n)}
                  style={{
                    display: 'grid', gridTemplateColumns: '48px 1fr auto', gap: 16,
                    padding: '16px 4px', textAlign: 'left', alignItems: 'center',
                    borderTop: 'var(--hairline)',
                    borderBottom: s.n === 4 ? 'var(--hairline)' : 'none',
                    opacity: s.n > step ? 0.4 : 1, transition: 'opacity .2s'
                  }}>
            <div className="display" style={{ fontSize: 28, fontWeight: 300,
                        color: s.n < step ? 'var(--success)' : s.n === step ? 'var(--accent)' : 'var(--ink-3)',
                        lineHeight: 1 }}>
              {s.n < step ? '✓' : '0' + s.n}
            </div>
            <div>
              <div className="display" style={{ fontSize: 22, fontWeight: 300 }}>{s.title}</div>
              <div style={{ fontSize: 13, color: 'var(--ink-3)', marginTop: 4 }}>{s.detail}</div>
            </div>
            {s.n === step && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>in progress</span>
            )}
            {s.n < step && <span className="mono" style={{ fontSize: 11, color: 'var(--success)' }}>done</span>}
          </button>
        ))}
      </div>

      <div style={{ display: 'flex', gap: 8, marginTop: 32 }}>
        <button onClick={() => setStep(Math.min(4, step+1))}
                style={{ padding: '12px 24px', background: 'var(--ink)', color: 'var(--paper)',
                          borderRadius: 999, fontSize: 13, fontWeight: 500 }}>
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
        <button style={{ padding: '12px 16px', color: 'var(--ink-3)', fontSize: 13 }}>
          Skip for now
        </button>
      </div>
    </div>
  );
}

window.MaApp = MaApp;
