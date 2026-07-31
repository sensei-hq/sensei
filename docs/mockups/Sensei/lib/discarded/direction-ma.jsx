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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Ma · Direction 1"
 style={{
 fontFamily: 'var(--font-ui)' }}>
      <TauriChrome title="Sensei  先生" />
      <div className="flex-1 flex min-h-0" >
        <MaSidebar page={page} setPage={setPage}
                   solutions={data.solutions}
                   activeSolution={activeSolution} setActiveSolution={setActiveSolution} />
        <main className="flex-1 overflow-auto relative" >
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
 width: 220 }} className="py-6 px-4 gap-6 border-r flex flex-col bg-paper shrink-0" >
      <div className="gap-2 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 22 }}>先</span>
        <span className="display font-normal" style={{ fontSize: 17 }}>Sensei</span>
      </div>

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
              <span className="kanji" style={{ fontSize: 13, color: activeSolution === s.id ? 'var(--accent)' : 'var(--ink-3)', width: 16 }}>{s.kanji}</span>
              <span className="flex-1" >{s.name}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{pct(s.ftr)}</span>
            </button>
          ))}
        </div>
      </div>

      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >View</div>
        <div className="gap-1 flex flex-col" >
          {PAGES.map(p => (
            <button key={p.id}
 onClick={() => setPage(p.id)}
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

      <div className="flex-1" />

      <div style={{ fontSize: 13 }} className="gap-2 flex items-center text-ink-3" >
        <Avatar name="Aiko" size={22}/>
        <div className="flex-1 overflow-hidden" >
          <div className="text-ink-2" style={{ fontSize: 13 }}>Aiko</div>
          <div style={{ fontSize: 11 }}>daemon · 9823</div>
        </div>
        <span className="ink-dot bg-success" />
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
    <div style={{ maxWidth: 1100 }} className="pt-12 pb-16 px-16 relative" >
      {/* Huge kanji watermark */}
      <div className="kanji absolute text-accent" style={{ top: 40, right: 40, fontSize: 56, opacity: 0.05, lineHeight: 1, userSelect: 'none',
 pointerEvents: 'none'
 }}>{sol.kanji}</div>

      {/* Breadcrumb */}
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >
        Observatory · {sol.name}
      </div>
      <h1 className="display my-1 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
        How am I doing?
      </h1>
      <div style={{ fontSize: 13 }} className="mb-12 text-ink-3" >
        The week of April 16 — 22. Three repos, {sol.sessions7d} sessions, {sol.tokens7d}M tokens.
      </div>

      {/* Hero FTR number */}
      <div className="gap-6 mb-3 flex items-baseline relative" >
        <div className="display font-light" style={{ fontSize: 56, lineHeight: 0.9,
 letterSpacing: '-0.04em', fontFeatureSettings: '"ss01"' }}>
          {Math.round(sol.ftr * 100)}
          <span style={{ fontSize: 56 }} className="ml-1 text-ink-3 font-light" >%</span>
        </div>
        <div className="pb-4" >
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 uppercase text-ink-3" >First try right</div>
          <div className="gap-2 flex items-center" >
            <span className="mono" style={{ fontSize: 13,
                      color: trendUp ? 'var(--success)' : 'var(--accent)' }}>
              {trendUp ? "↗" : "↘"} {delta >= 0 ? "+" : ""}{delta}%
            </span>
            <span className="text-ink-3" style={{ fontSize: 13 }}>vs. last week</span>
          </div>
          <div style={{ color: trendUp ? 'var(--success)' : 'var(--accent)' }} className="mt-3" >
            <Sparkline data={history} width={180} height={38} />
          </div>
        </div>
      </div>

      <hr className="hairline mt-12 mb-8"/>

      {/* The koan — coaching pulled front & center */}
      <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-12 grid" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 uppercase text-ink-3" >Sensei says</div>
          <blockquote className="m-0" >
            <p className="display m-0 font-light text-ink" style={{
 fontSize: 28, lineHeight: 1.25, textWrap: 'balance'
 }}>
              {topCoach.koan}
            </p>
            <p style={{
 fontSize: 13, lineHeight: 1.55,
 maxWidth: 420
 }} className="mt-6 mb-0 text-ink-2" >{topCoach.body}</p>
          </blockquote>

          <div className="mt-6 gap-3 flex items-center" >
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
 style={{
 background: applied[topCoach.id] ? 'var(--success-soft)' : 'var(--ink)',
 color: applied[topCoach.id] ? 'var(--success)' : 'var(--paper)',
 borderRadius: 999, fontSize: 13,
 letterSpacing: '0.01em', transition: 'all .18s'
 }} className="py-2 px-4 font-medium" >
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action}
            </button>
            <span className="text-ink-3" style={{ fontSize: 11 }}>{topCoach.impact}</span>
          </div>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 uppercase text-ink-3" >Recent sessions</div>
          <div className="flex flex-col" >
            {solSessions.map((s, i) => (
              <button key={s.id}
 onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
 style={{ gridTemplateColumns: '10px 1fr auto auto',
 borderBottom: i < solSessions.length - 1 ? 'var(--hairline)' : 'none'
 }} className="gap-3 py-3 px-0 grid items-center text-left" >
                <span className="ink-dot"
                      style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)', width: 6, height: 6 }}/>
                <div>
                  <div className="text-ink font-normal" style={{ fontSize: 13 }}>{s.title}</div>
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

      <hr className="hairline mt-12 mb-8"/>

      {/* Quality signals — understated grid */}
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)' }} className="gap-8 mb-2 grid" >
        {[
          { label: "Pattern compliance", v: "94%", delta: "+3", good: true },
          { label: "Test coverage Δ",    v: "+2.1%", delta: "", good: true },
          { label: "Doc drift",          v: "3 files", delta: "brand-kit", good: false },
          { label: "Tokens / session",   v: "14.2k", delta: "−1.8k", good: true },
        ].map(s => (
          <div key={s.label}>
            <div style={{
 fontSize: 11, letterSpacing: '0.08em' }} className="mb-2 text-ink-3 uppercase" >{s.label}</div>
            <div className="display font-light" style={{ fontSize: 28, letterSpacing: '-0.02em' }}>{s.v}</div>
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
    <div style={{ maxWidth: 1100 }} className="pt-12 pb-16 px-16" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Overview</div>
      <h1 className="display mt-0 mb-1 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
        Three solutions. Eight repos.
      </h1>
      <div style={{ fontSize: 13 }} className="mb-12 text-ink-3" >
        Global FTR <span className="mono text-ink" >78%</span>, week to date.
      </div>

      {/* Global sparkline */}
      <div className="mb-12" >
        <div className="text-accent" >
          <Sparkline data={data.ftrHistory} width={800} height={60} />
        </div>
        <div className="mono mt-1 flex justify-between text-ink-3" style={{
 fontSize: 11 }}>
          <span>Apr 9</span><span>Apr 16</span><span>Apr 22</span>
        </div>
      </div>

      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-6 grid" >
        {data.solutions.map(s => (
          <button key={s.id}
 onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
 style={{
 borderRadius: 10,
 transition: 'border-color .15s, transform .15s'
 }}
 onMouseEnter={e => e.currentTarget.style.borderColor = 'var(--ink-3)'}
 onMouseLeave={e => e.currentTarget.style.borderColor = ''} className="py-6 px-6 bg-paper border border-paper-edge text-left relative overflow-hidden" >
            <span className="kanji absolute text-accent" style={{ top: -30, right: -20, fontSize: 56, opacity: 0.06, lineHeight: 1
 }}>{s.kanji}</span>
            <div className="display mb-1 font-normal" style={{ fontSize: 17 }}>{s.name}</div>
            <div style={{ fontSize: 11 }} className="mb-6 text-ink-3" >{s.description}</div>
            <div className="display font-light" style={{ fontSize: 56, letterSpacing: '-0.03em', lineHeight: 1 }}>
              {Math.round(s.ftr*100)}<span className="text-ink-3" style={{ fontSize: 17 }}>%</span>
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.08em' }} className="mt-1 text-ink-3 uppercase" >First try right</div>
            <div style={{ color: s.ftr >= s.ftrPrev ? 'var(--success)' : 'var(--accent)' }} className="mt-4" >
              <Sparkline data={data.ftrBySolution[s.id]} width={200} height={28}/>
            </div>
            <hr className="hairline mt-4 mb-3"/>
            <div className="mono flex justify-between text-ink-3" style={{
 fontSize: 11 }}>
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
    <div className="flex h-full" >
      {/* Left list */}
      <div style={{
 flex: selectedSession ? '0 0 380px' : '1',
 borderRight: selectedSession ? 'var(--hairline)' : 'none' }} className="pt-12 pb-8 px-8 overflow-auto" >
        <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Sessions · 刻</div>
        <h1 className="display mt-0 mb-6 font-light" style={{ fontSize: 28 }}>
          Every session is a lesson.
        </h1>

        <div className="gap-1 mb-6 flex" >
          {filters.map(f => (
            <button key={f.id}
 onClick={() => setFilter(f.id)}
 style={{
 borderRadius: 999, fontSize: 11,
 background: filter === f.id ? 'var(--ink)' : 'transparent',
 color: filter === f.id ? 'var(--paper)' : 'var(--ink-2)',
 transition: 'all .12s'
 }} className="py-1 px-2 border border-paper-edge" >{f.label}</button>
          ))}
        </div>

        <div className="flex flex-col" >
          {sessions.map((s, i) => (
            <button key={s.id}
 onClick={() => setFocused(s.id)}
 style={{
 borderBottom: i < sessions.length-1 ? 'var(--hairline)' : 'none',
 background: focused === s.id ? 'var(--paper-3)' : 'transparent',
 paddingLeft: focused === s.id ? 12 : 0,
 margin: focused === s.id ? '0 -12px' : 0,
 paddingRight: focused === s.id ? 12 : 0,
 borderRadius: focused === s.id ? 6 : 0,
 transition: 'all .12s'
 }} className="py-3 px-0 text-left" >
              <div className="gap-2 mb-1 flex items-center" >
                <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                  {s.date} · {s.started}
                </span>
                {!s.ftr && <span className="mono text-accent" style={{ fontSize: 11 }}>
                  {s.corrections} corrections
                </span>}
              </div>
              <div style={{ fontSize: 13 }} className="mb-1 text-ink" >{s.title}</div>
              <div className="mono text-ink-3" style={{ fontSize: 11 }}>
                {s.project} · {s.turns}t · {s.duration}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Right detail */}
      {selectedSession && (
        <div className="py-12 px-12 flex-1 overflow-auto relative" >
          <button className="absolute text-ink-3" onClick={() => setFocused(null)}
 style={{ top: 22, right: 22, fontSize: 17, width: 30, height: 30 }}>×</button>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >
            Session {selectedSession.id}
          </div>
          <h2 className="display mt-0 mb-2 font-light" style={{ fontSize: 28, letterSpacing: '-0.01em' }}>
            {selectedSession.title}
          </h2>
          <div className="mono mb-6 text-ink-3" style={{ fontSize: 11 }}>
            {selectedSession.project} · {selectedSession.date} {selectedSession.started} · {selectedSession.duration} · {selectedSession.turns} turns · {(selectedSession.tokens/1000).toFixed(1)}k tokens
          </div>

          <div style={{
 borderRadius: 8 }} className="py-4 px-4 mb-8 gap-3 bg-paper-2 border border-paper-edge flex items-start" >
            <span className="kanji" style={{ fontSize: 22, color: selectedSession.ftr ? 'var(--success)' : 'var(--accent)', lineHeight: 1 }}>
              {selectedSession.ftr ? '一' : '修'}
            </span>
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.08em' }} className="mb-1 text-ink-3 uppercase" >
                {selectedSession.ftr ? 'First try right' : `Corrected · ${selectedSession.corrections}`}
              </div>
              <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55 }}>
                {selectedSession.summary}
              </div>
            </div>
          </div>

          {selectedSession.events && (
            <>
              <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-4 text-ink-3 uppercase" >Event timeline</div>
              <div className="pl-6 relative" >
                <div className="absolute" style={{ left: 7, top: 4, bottom: 4, width: 1,
 background: 'var(--edge)' }}/>
                {selectedSession.events.map((e, i) => (
                  <div key={i} className="gap-3 mb-3 flex relative" >
                    <div className="absolute" style={{ left: -22, top: 2,
 color: e.kind === 'correction' ? 'var(--accent)'
 : e.kind === 'test' ? 'var(--success)'
 : 'var(--ink-3)' }}>
                      <div className="p-1 bg-paper" >
                        <EventGlyph kind={e.kind}/>
                      </div>
                    </div>
                    <div className="mono pt-1 text-ink-3" style={{ fontSize: 11, width: 42 }}>
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
    <div style={{ maxWidth: 1100 }} className="py-12 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Codebase · {sol.name}</div>
      <h1 className="display mt-0 mb-8 font-light" style={{ fontSize: 28 }}>
        Where the weight gathers.
      </h1>

      <div className="gap-1 mb-8 flex flex-wrap" >
        {sol.repos.map(r => (
          <button key={r}
 onClick={() => setSelectedRepo(r)}
 className="mono py-1 px-3 border border-paper-edge"
 style={{
 borderRadius: 999, fontSize: 11,
 background: selectedRepo === r ? 'var(--ink)' : 'transparent',
 color: selectedRepo === r ? 'var(--paper)' : 'var(--ink-2)'
 }}>{r}</button>
        ))}
      </div>

      {/* Graph placeholder — abstract constellation */}
      <div style={{
 borderRadius: 10, minHeight: 280 }} className="p-6 mb-8 bg-paper-2 border border-paper-edge relative overflow-hidden" >
        <div style={{
 fontSize: 11, letterSpacing: '0.08em' }} className="mb-1 text-ink-3 uppercase" >Code graph</div>
        <div className="mono text-ink-3" style={{ fontSize: 11 }}>
          {selectedRepo} · 247 nodes · 4 communities
        </div>
        <svg viewBox="0 0 600 240" width="100%" height="240" className="mt-2" >
          {/* static constellation */}
          {Array.from({length: 60}).map((_, i) => {
            const x = 40 + (i * 37) % 540 + Math.sin(i) * 20;
            const y = 30 + ((i * 53) % 180) + Math.cos(i*0.7) * 10;
            const big = i % 11 === 0;
            return <circle className="text-ink-2" key={i} cx={x} cy={y} r={big ? 5 : 2} fill="currentColor"
 opacity={big ? 0.8 : 0.25} />;
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

      <div style={{ gridTemplateColumns: '2fr 1fr' }} className="gap-12 grid" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 text-ink-3 uppercase" >Hotspots</div>
          {data.hotspots.map((h, i) => (
            <div key={i} style={{
 borderBottom: i < data.hotspots.length - 1 ? 'var(--hairline)' : 'none', gridTemplateColumns: '14px 1fr auto auto auto' }} className="gap-3 py-3 px-0 grid items-center" >
              <span className="ink-dot" style={{
                background: h.severity === 'god' ? 'var(--accent)' :
                            h.severity === 'cluster' ? 'var(--warning)' : 'var(--success)'
              }}/>
              <span className="mono text-ink" style={{ fontSize: 13 }}>{h.name}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>in {h.fanIn}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>out {h.fanOut}</span>
              <span className="mono" style={{ fontSize: 11, color: h.rework > 3 ? 'var(--accent)' : 'var(--ink-3)' }}>
                ↻{h.rework}
              </span>
            </div>
          ))}
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 text-ink-3 uppercase" >Health</div>
          <div className="gap-3 flex flex-col" >
            {[
              { l: "Dead code",      v: "14 exports" },
              { l: "Test ratio",     v: "0.72 : 1" },
              { l: "Largest file",   v: "router.ts · 812 ln" },
              { l: "Last indexed",   v: "2m ago" }
            ].map(r => (
              <div className="flex justify-between" key={r.l} >
                <span className="text-ink-3" style={{ fontSize: 13 }}>{r.l}</span>
                <span className="mono text-ink" style={{ fontSize: 13 }}>{r.v}</span>
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
    <div style={{ maxWidth: 880 }} className="py-12 px-16" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Coaching · 師</div>
      <h1 className="display mt-0 mb-1 font-light" style={{ fontSize: 28 }}>
        What the sessions are teaching.
      </h1>
      <div style={{ fontSize: 13 }} className="mb-12 text-ink-3" >
        Three observations, in descending urgency.
      </div>

      <div className="gap-0 flex flex-col" >
        {data.coaching.map((c, i) => {
          const isApplied = applied[c.id];
          return (
            <div key={c.id}
 style={{
 borderBottom: i === data.coaching.length - 1 ? 'var(--hairline)' : 'none', gridTemplateColumns: '72px 1fr 180px'
 }} className="gap-6 py-8 px-0 border-t grid" >
              <div>
                <div className="display font-light text-accent" style={{ fontSize: 56,
 opacity: c.urgency === 'high' ? 1 : c.urgency === 'medium' ? 0.5 : 0.25,
 lineHeight: 1 }}>
                  0{i+1}
                </div>
                <div className="mono mt-1 text-ink-3 uppercase" style={{
 fontSize: 11,
 letterSpacing: '0.12em' }}>
                  {c.urgency}
                </div>
              </div>
              <div>
                <p className="display mt-0 mb-3 font-light" style={{ fontSize: 22, lineHeight: 1.3 }}>
                  {c.koan}
                </p>
                <p style={{ fontSize: 13, lineHeight: 1.6 }} className="m-0 text-ink-2" >
                  {c.body}
                </p>
                <div className="mono mt-3 text-ink-3" style={{ fontSize: 11 }}>
                  module: {c.module}
                </div>
              </div>
              <div className="gap-2 flex flex-col items-start" >
                <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
 style={{
 borderRadius: 999, fontSize: 13,
 background: isApplied ? 'var(--success-soft)' : 'var(--ink)',
 color: isApplied ? 'var(--success)' : 'var(--paper)',
 transition: 'all .18s'
 }} className="py-2 px-4 font-medium w-full text-center" >
                  {isApplied ? "✓  Applied" : c.action}
                </button>
                <div className="text-ink-3" style={{ fontSize: 11 }}>{c.actionDetail}</div>
                <div className="text-ink-2 italic" style={{ fontSize: 11 }}>{c.impact}</div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Personas */}
      <div className="mt-12" >
        <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 text-ink-3 uppercase" >Active personas</div>
        <div style={{ gridTemplateColumns: 'repeat(2,1fr)' }} className="gap-2 grid" >
          {data.personas.map(p => (
            <div key={p.id} style={{
 borderRadius: 6 }} className="p-3 border border-paper-edge bg-paper-2" >
              <div className="text-ink" style={{ fontSize: 13 }}>{p.name}</div>
              <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
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
    <div style={{ maxWidth: 960 }} className="py-12 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >Configuration · 設</div>
      <h1 className="display mt-0 mb-6 font-light" style={{ fontSize: 28 }}>
        What sensei is allowed to do.
      </h1>

      <div className="gap-4 mb-6 flex border-b" >
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
        <div className="flex flex-col" >
          {data.skills.map((s, i) => (
            <div key={s.id} style={{
 borderBottom: i < data.skills.length-1 ? 'var(--hairline)' : 'none', gridTemplateColumns: '1fr auto auto' }} className="gap-4 py-3 px-0 grid items-center" >
              <div>
                <div className="text-ink" style={{ fontSize: 13 }}>{s.name}</div>
                <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                  {s.solutions.length ? `active in ${s.solutions.join(', ')}` : 'not installed'}
                </div>
              </div>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{s.id}</span>
              <div className="relative" style={{ width: 32, height: 18, borderRadius: 999,
 background: s.active ? 'var(--accent)' : 'var(--paper-3)', transition: 'background .15s' }}>
                <div className="absolute bg-paper rounded-full" style={{ top: 2, left: s.active ? 16 : 2,
 width: 14, height: 14, transition: 'left .15s' }}/>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "libraries" && (
        <div className="flex flex-col" >
          {data.libraries.map((l, i) => (
            <div key={l.name} style={{
 borderBottom: i < data.libraries.length-1 ? 'var(--hairline)' : 'none', gridTemplateColumns: '1fr auto auto auto' }} className="gap-4 py-3 px-0 grid items-center" >
              <div className="text-ink" style={{ fontSize: 13 }}>{l.name}</div>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>v{l.version}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{l.pages} pages</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{l.lastIndexed}</span>
            </div>
          ))}
          <button style={{ borderRadius: 999, fontSize: 13,
 border: '1px dashed var(--ink-3)' }} className="mt-4 py-2 px-4 self-start text-ink-2" >
            + Index a library
          </button>
        </div>
      )}

      {tab === "acps" && (
        <div className="gap-3 flex flex-col" >
          {[
            { name: "Claude Code",  version: "1.8.2", status: "connected" },
            { name: "Cursor",       version: "0.42",  status: "connected" },
            { name: "Zed",          version: "0.148", status: "available" }
          ].map(a => (
            <div key={a.name} style={{ borderRadius: 8 }} className="p-4 gap-4 border border-paper-edge flex items-center" >
              <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--success)' : 'var(--ink-4)' }}/>
              <div className="flex-1" >
                <div style={{ fontSize: 13 }}>{a.name}</div>
                <div className="mono text-ink-3" style={{ fontSize: 11 }}>
                  MCP · v{a.version} · {a.status}
                </div>
              </div>
              <button style={{
 fontSize: 11, borderRadius: 999 }} className="py-1 px-3 border border-paper-edge text-ink-2" >
                {a.status === 'connected' ? 'Configure' : 'Connect'}
              </button>
            </div>
          ))}
        </div>
      )}

      {tab === "daemon" && (
        <div>
          <div style={{
 borderRadius: 8
 }} className="p-4 mb-4 bg-paper-2 border border-paper-edge" >
            <div className="gap-2 mb-2 flex items-center" >
              <span className="ink-dot bg-success" />
              <span style={{ fontSize: 13 }}>Daemon running</span>
              <span className="mono ml-auto text-ink-3" style={{ fontSize: 11 }}>
                pid 12492 · uptime 4d 2h
              </span>
            </div>
            <div style={{ gridTemplateColumns: 'repeat(3,1fr)' }} className="gap-3 mt-4 grid" >
              <div>
                <div className="text-ink-3" style={{ fontSize: 11 }}>Port</div>
                <div className="mono" style={{ fontSize: 13 }}>9823</div>
              </div>
              <div>
                <div className="text-ink-3" style={{ fontSize: 11 }}>Events today</div>
                <div className="mono" style={{ fontSize: 13 }}>1,842</div>
              </div>
              <div>
                <div className="text-ink-3" style={{ fontSize: 11 }}>Memory</div>
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
    <div style={{ maxWidth: 820 }} className="py-16 px-16" >
      <div className="kanji mb-6 text-accent" style={{
 fontSize: 56, opacity: 0.85, lineHeight: 1
 }}>始</div>
      <h1 className="display mt-0 mb-3 font-light" style={{ fontSize: 40, letterSpacing: '-0.02em' }}>
        Begin.
      </h1>
      <p style={{ fontSize: 15, lineHeight: 1.6, maxWidth: 520 }} className="mb-12 text-ink-2" >
        Sensei will watch how you work and, in time, help you work better.
        Four steps, each takes a minute.
      </p>

      <div className="gap-1 flex flex-col" >
        {steps.map(s => (
          <button key={s.n}
 onClick={() => setStep(s.n)}
 style={{ gridTemplateColumns: '48px 1fr auto',
 borderBottom: s.n === 4 ? 'var(--hairline)' : 'none',
 opacity: s.n > step ? 0.4 : 1, transition: 'opacity .2s'
 }} className="gap-4 py-4 px-1 grid text-left items-center border-t" >
            <div className="display font-light" style={{ fontSize: 28,
 color: s.n < step ? 'var(--success)' : s.n === step ? 'var(--accent)' : 'var(--ink-3)',
 lineHeight: 1 }}>
              {s.n < step ? '✓' : '0' + s.n}
            </div>
            <div>
              <div className="display font-light" style={{ fontSize: 22 }}>{s.title}</div>
              <div style={{ fontSize: 13 }} className="mt-1 text-ink-3" >{s.detail}</div>
            </div>
            {s.n === step && (
              <span className="mono text-accent" style={{ fontSize: 11 }}>in progress</span>
            )}
            {s.n < step && <span className="mono text-success" style={{ fontSize: 11 }}>done</span>}
          </button>
        ))}
      </div>

      <div className="gap-2 mt-8 flex" >
        <button onClick={() => setStep(Math.min(4, step+1))}
 style={{
 borderRadius: 999, fontSize: 13 }} className="py-3 px-6 bg-ink text-paper font-medium" >
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
        <button style={{ fontSize: 13 }} className="py-3 px-4 text-ink-3" >
          Skip for now
        </button>
      </div>
    </div>
  );
}

window.MaApp = MaApp;
