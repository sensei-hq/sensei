// Direction 2 — ENSO (円相) · circular brushstroke data viz.
// FTR is a brush-ring. Sessions orbit. Coaching speaks in koans.
// Warmer paper, slightly more ink.

const EnsoApp = () => {
  const data = window.SENSEI_DATA;
  const [page, setPage] = React.useState("observatory");
  const [activeSolution, setActiveSolution] = React.useState("lumen-cloud");
  const [focusedSession, setFocusedSession] = React.useState(null);
  const [appliedCoaching, setAppliedCoaching] = React.useState({});

  const sol = data.solutions.find(s => s.id === activeSolution);

  return (
    <div className="sensei" data-screen-label="Enso · Direction 2"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生"/>
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <EnsoSidebar page={page} setPage={setPage}
                     solutions={data.solutions}
                     activeSolution={activeSolution} setActiveSolution={setActiveSolution}
                     data={data}/>
        <main style={{ flex: 1, overflow: 'auto' }}>
          {page === "overview"    && <EnsoOverview data={data} setPage={setPage} setActiveSolution={setActiveSolution}/>}
          {page === "observatory" && <EnsoObservatory data={data} sol={sol} setPage={setPage}
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

// ────────────────────────────────────────────────────────────
// SIDEBAR — compact, icons-first
// ────────────────────────────────────────────────────────────
function EnsoSidebar({ page, setPage, solutions, activeSolution, setActiveSolution, data }) {
  return (
    <aside style={{
      width: 64, borderRight: 'var(--hairline)', padding: '18px 0',
      display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18,
      flexShrink: 0, background: 'var(--paper-2)'
    }}>
      <div className="kanji" style={{ fontSize: 22, color: 'var(--shu)' }}>先</div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'center' }}>
        {solutions.map(s => {
          const isActive = activeSolution === s.id;
          return (
            <button key={s.id}
                    title={s.name}
                    onClick={() => setActiveSolution(s.id)}
                    style={{
                      width: 40, height: 40, borderRadius: '50%',
                      background: isActive ? 'var(--sumi)' : 'var(--paper)',
                      color: isActive ? 'var(--paper)' : 'var(--sumi)',
                      border: isActive ? 'none' : 'var(--hairline)',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      transition: 'all .14s'
                    }}>
              <span className="kanji" style={{ fontSize: 18 }}>{s.kanji}</span>
            </button>
          );
        })}
      </div>

      <hr className="hairline" style={{ width: 28, margin: '4px 0' }}/>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, alignItems: 'center' }}>
        {PAGES.map(p => (
          <button key={p.id}
                  title={p.label}
                  onClick={() => setPage(p.id)}
                  style={{
                    width: 36, height: 36, borderRadius: 6,
                    background: page === p.id ? 'var(--shu-soft)' : 'transparent',
                    color: page === p.id ? 'var(--shu)' : 'var(--sumi-3)',
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    transition: 'all .12s'
                  }}>
            <span className="kanji" style={{ fontSize: 14 }}>{p.kanji}</span>
          </button>
        ))}
      </div>

      <div style={{ flex: 1 }}/>
      <Avatar name="Aiko" size={28}/>
    </aside>
  );
}

// ────────────────────────────────────────────────────────────
// OBSERVATORY — ring is the hero, everything orbits
// ────────────────────────────────────────────────────────────
function EnsoObservatory({ data, sol, setPage, setFocusedSession, applied, setApplied }) {
  const topCoach = data.coaching[0];
  const history = data.ftrBySolution[sol.id];
  const solSessions = data.sessions.filter(s => s.solution === sol.id).slice(0, 8);
  const delta = Math.round((sol.ftr - sol.ftrPrev) * 100);

  return (
    <div style={{ padding: '40px 56px', display: 'grid', gridTemplateColumns: '1.2fr 1fr', gap: 56 }}>
      {/* LEFT: the ring + orbit of sessions */}
      <div>
        <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                      textTransform: 'uppercase', marginBottom: 4 }}>
          Observatory · {sol.name}
        </div>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 400, margin: '0 0 36px' }}>
          {sol.description}
        </h1>

        <div style={{ position: 'relative', width: 440, height: 440, margin: '0 auto' }}>
          {/* The ENSO ring itself */}
          <div style={{ position: 'absolute', inset: 0, color: delta >= 0 ? 'var(--jade)' : 'var(--shu)' }}>
            <EnsoRing progress={sol.ftr} size={440} stroke={18}
                      color={delta >= 0 ? 'oklch(0.58 0.15 35)' : 'oklch(0.58 0.15 35)'}
                      trackColor="var(--sumi)"/>
          </div>
          {/* Center readout */}
          <div style={{ position: 'absolute', inset: 0, display: 'flex',
                        flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
                        pointerEvents: 'none' }}>
            <div className="display" style={{ fontSize: 98, fontWeight: 300, lineHeight: 1,
                          letterSpacing: '-0.03em' }}>
              {Math.round(sol.ftr * 100)}
              <span style={{ fontSize: 30, color: 'var(--sumi-3)' }}>%</span>
            </div>
            <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                          color: 'var(--sumi-3)', marginTop: 6 }}>一  First try right</div>
            <div className="mono" style={{ fontSize: 12, marginTop: 14,
                          color: delta >= 0 ? 'var(--jade)' : 'var(--shu)' }}>
              {delta >= 0 ? '↗' : '↘'} {delta >= 0 ? '+' : ''}{delta}% week
            </div>
          </div>

          {/* Orbiting session dots */}
          {solSessions.map((s, i) => {
            const angle = -140 + (i / solSessions.length) * 300;
            const rad = angle * Math.PI / 180;
            const r = 230;
            const x = 220 + r * Math.cos(rad);
            const y = 220 + r * Math.sin(rad);
            return (
              <button key={s.id}
                      title={s.title}
                      onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
                      style={{
                        position: 'absolute', left: x - 7, top: y - 7,
                        width: 14, height: 14, borderRadius: '50%',
                        background: s.ftr ? 'var(--jade)' : 'var(--shu)',
                        border: '2px solid var(--paper)',
                        boxShadow: '0 0 0 1px ' + (s.ftr ? 'var(--jade)' : 'var(--shu)'),
                        transition: 'transform .15s'
                      }}
                      onMouseEnter={e => e.currentTarget.style.transform = 'scale(1.6)'}
                      onMouseLeave={e => e.currentTarget.style.transform = ''}/>
            );
          })}
        </div>

        <div style={{ display: 'flex', justifyContent: 'space-around', marginTop: 28,
                      padding: '16px 20px', border: 'var(--hairline)', borderRadius: 10,
                      background: 'var(--paper-2)' }}>
          <Stat label="Sessions"  value={sol.sessions7d}     suffix="· 7d"/>
          <Divider/>
          <Stat label="Tokens"    value={sol.tokens7d + "M"} suffix="· 7d"/>
          <Divider/>
          <Stat label="Skills"    value={sol.activeSkills}   suffix="active"/>
          <Divider/>
          <Stat label="Repos"     value={sol.repos.length}   suffix={sol.repos.join(' · ')}/>
        </div>
      </div>

      {/* RIGHT: the koan + signals */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 32 }}>
        <div style={{ background: 'var(--paper-2)', borderRadius: 14,
                      padding: '32px 28px', border: 'var(--hairline)', position: 'relative',
                      overflow: 'hidden' }}>
          {/* small enso watermark */}
          <svg width="120" height="120" style={{ position: 'absolute', top: -20, right: -20, opacity: 0.12 }}>
            <EnsoRingInline size={120} stroke={4} color="var(--shu)"/>
          </svg>

          <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                        color: 'var(--shu)', marginBottom: 18 }}>師 · sensei says</div>
          <p className="display" style={{ fontSize: 28, fontWeight: 300, lineHeight: 1.25,
                        margin: 0, textWrap: 'balance' }}>
            {topCoach.koan}
          </p>
          <p style={{ fontSize: 13, color: 'var(--sumi-2)', lineHeight: 1.6, marginTop: 18, marginBottom: 24 }}>
            {topCoach.body}
          </p>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <button onClick={() => setApplied({...applied, [topCoach.id]: true})}
                    style={{
                      padding: '10px 18px',
                      background: applied[topCoach.id] ? 'var(--jade-soft)' : 'var(--shu)',
                      color: applied[topCoach.id] ? 'var(--jade)' : 'var(--paper)',
                      borderRadius: 8, fontSize: 12.5, fontWeight: 500, letterSpacing: '0.01em'
                    }}>
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action + " →"}
            </button>
            <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
              {topCoach.impact}
            </span>
          </div>
        </div>

        <div>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                        color: 'var(--sumi-3)', marginBottom: 16 }}>Quality signals</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
            {[
              { k: "Pattern compliance", v: "94%",      d: "+3", good: true },
              { k: "Test coverage Δ",    v: "+2.1%",    d: "this week", good: true },
              { k: "Doc drift",          v: "3 files",  d: "brand-kit", good: false },
              { k: "Tokens / session",   v: "14.2k",    d: "−1.8k", good: true },
            ].map(s => (
              <div key={s.k} style={{ padding: 14, border: 'var(--hairline)', borderRadius: 8,
                            background: 'var(--paper)' }}>
                <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', letterSpacing: '0.05em' }}>{s.k}</div>
                <div className="display" style={{ fontSize: 22, fontWeight: 400, marginTop: 4 }}>{s.v}</div>
                <div className="mono" style={{ fontSize: 10, color: s.good ? 'var(--jade)' : 'var(--shu)', marginTop: 2 }}>
                  {s.d}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: 8, color: 'var(--shu)' }}>
          <div style={{ fontSize: 11, letterSpacing: '0.18em', textTransform: 'uppercase',
                        color: 'var(--sumi-3)', marginBottom: 8 }}>FTR · 14 days</div>
          <Sparkline data={history} width={440} height={46} fill="var(--shu-soft)" showDots/>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value, suffix }) {
  return (
    <div>
      <div style={{ fontSize: 10, color: 'var(--sumi-3)', letterSpacing: '0.1em', textTransform: 'uppercase' }}>{label}</div>
      <div className="display" style={{ fontSize: 20, fontWeight: 400, marginTop: 2 }}>{value}</div>
      <div className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-3)', marginTop: 1 }}>{suffix}</div>
    </div>
  );
}
function Divider() {
  return <div style={{ width: 1, background: 'var(--paper-edge)' }}/>;
}

// A small variant of the ring for inline use in overview cards
function EnsoRingInline({ progress = 1, size = 120, stroke = 4, color = 'var(--shu)', startAngle = -140, sweep = 300 }) {
  const r = (size - stroke) / 2;
  const cx = size / 2, cy = size / 2;
  const toXY = d => { const rd = d*Math.PI/180; return [cx + r*Math.cos(rd), cy + r*Math.sin(rd)] };
  const fullEnd = startAngle + sweep;
  const [x0,y0] = toXY(startAngle), [x1,y1] = toXY(fullEnd);
  return (
    <>
      <path d={`M ${x0.toFixed(2)} ${y0.toFixed(2)} A ${r} ${r} 0 1 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`}
            fill="none" stroke={color} strokeWidth={stroke} strokeLinecap="round"/>
      <circle cx={x0} cy={y0} r={stroke*0.9} fill={color}/>
    </>
  );
}

// ────────────────────────────────────────────────────────────
// OVERVIEW — rings for all solutions
// ────────────────────────────────────────────────────────────
function EnsoOverview({ data, setPage, setActiveSolution }) {
  return (
    <div style={{ padding: '40px 56px', maxWidth: 1120 }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>全 · Overview</div>
      <h1 className="display" style={{ fontSize: 30, fontWeight: 300, margin: '0 0 32px' }}>
        All solutions · <span className="mono" style={{ fontSize: 22, color: 'var(--shu)' }}>78%</span> global
      </h1>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 18 }}>
        {data.solutions.map(s => {
          const up = s.ftr >= s.ftrPrev;
          return (
            <button key={s.id} onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
                    style={{ padding: '28px 24px', borderRadius: 16, border: 'var(--hairline)',
                            background: 'var(--paper-2)', textAlign: 'left', position: 'relative',
                            display: 'flex', flexDirection: 'column', alignItems: 'center',
                            transition: 'transform .15s, border-color .15s' }}
                    onMouseEnter={e => { e.currentTarget.style.borderColor = 'var(--sumi-3)'; e.currentTarget.style.transform = 'translateY(-2px)'; }}
                    onMouseLeave={e => { e.currentTarget.style.borderColor = ''; e.currentTarget.style.transform = ''; }}>
              <div className="kanji" style={{ fontSize: 14, color: 'var(--shu)', alignSelf: 'flex-start' }}>{s.kanji}</div>
              <div className="display" style={{ fontSize: 18, fontWeight: 400, alignSelf: 'flex-start', marginTop: 4 }}>{s.name}</div>
              <div style={{ fontSize: 11, color: 'var(--sumi-3)', alignSelf: 'flex-start', marginBottom: 18 }}>{s.description}</div>
              <div style={{ position: 'relative', width: 180, height: 180 }}>
                <EnsoRing progress={s.ftr} size={180} stroke={10}
                          color="oklch(0.58 0.15 35)"/>
                <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center',
                              justifyContent: 'center', flexDirection: 'column' }}>
                  <span className="display" style={{ fontSize: 38, fontWeight: 300, lineHeight: 1 }}>
                    {Math.round(s.ftr*100)}
                  </span>
                  <span style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 4, letterSpacing: '0.12em' }}>FTR</span>
                </div>
              </div>
              <div className="mono" style={{ marginTop: 20, fontSize: 11,
                            color: up ? 'var(--jade)' : 'var(--shu)' }}>
                {up ? '↗' : '↘'} {((s.ftr - s.ftrPrev)*100 >= 0 ? '+' : '')}{Math.round((s.ftr - s.ftrPrev)*100)}% week
              </div>
              <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 4 }}>
                {s.sessions7d} sessions · {s.tokens7d}M
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// SESSIONS — horizontal event ribbon per session
// ────────────────────────────────────────────────────────────
function EnsoSessions({ data, sol, focused, setFocused }) {
  const [filter, setFilter] = React.useState("all");
  const sessions = data.sessions.filter(s => {
    if (filter === "all") return true;
    if (filter === "corrected") return s.outcome === "corrected";
    if (filter === "first-try") return s.outcome === "first-try";
    return true;
  });

  return (
    <div style={{ padding: '36px 48px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 20, marginBottom: 28 }}>
        <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: 0 }}>
          刻 · Sessions
        </h1>
        <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
          {sessions.length} of {data.sessions.length}
        </div>
        <div style={{ flex: 1 }}/>
        <div style={{ display: 'flex', gap: 4 }}>
          {["all", "first-try", "corrected"].map(f => (
            <button key={f} onClick={() => setFilter(f)}
                    style={{ padding: '5px 11px', fontSize: 11, borderRadius: 999,
                              background: filter === f ? 'var(--sumi)' : 'transparent',
                              color: filter === f ? 'var(--paper)' : 'var(--sumi-2)',
                              border: 'var(--hairline)' }}>
              {f}
            </button>
          ))}
        </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {sessions.map(s => (
          <EnsoSessionCard key={s.id} s={s} expanded={focused === s.id}
                           onToggle={() => setFocused(focused === s.id ? null : s.id)}/>
        ))}
      </div>
    </div>
  );
}

function EnsoSessionCard({ s, expanded, onToggle }) {
  return (
    <div style={{ border: 'var(--hairline)', borderRadius: 10,
                  background: expanded ? 'var(--paper-2)' : 'var(--paper)',
                  overflow: 'hidden', transition: 'background .15s' }}>
      <button onClick={onToggle}
              style={{ display: 'grid', gridTemplateColumns: '12px 80px 1fr 220px 80px 20px',
                        gap: 16, padding: '14px 18px', alignItems: 'center', width: '100%',
                        textAlign: 'left' }}>
        <span className="ink-dot" style={{ background: s.ftr ? 'var(--jade)' : 'var(--shu)' }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{s.id}</span>
        <div>
          <div style={{ fontSize: 13, color: 'var(--sumi)' }}>{s.title}</div>
          <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 2 }}>
            {s.project} · {s.module}
          </div>
        </div>
        {/* Event ribbon */}
        <div style={{ display: 'flex', gap: 2, alignItems: 'center' }}>
          {(s.events || [{kind:'start'},{kind:'edit'},{kind:'test'},{kind:'end'}]).map((e, i) => (
            <div key={i} style={{
              width: 12, height: 16, borderRadius: 2,
              background: e.kind === 'correction' ? 'var(--shu)' :
                          e.kind === 'test' ? 'var(--jade-soft)' :
                          e.kind === 'edit' ? 'var(--sumi-4)' : 'var(--paper-edge)'
            }} title={e.kind}/>
          ))}
        </div>
        <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)', textAlign: 'right' }}>
          {s.duration}
        </span>
        <span style={{ color: 'var(--sumi-3)', transform: expanded ? 'rotate(90deg)' : '',
                        transition: 'transform .15s' }}>›</span>
      </button>

      {expanded && s.events && (
        <div style={{ padding: '0 18px 20px 122px', borderTop: 'var(--hairline)' }}>
          <div style={{ padding: '16px 0 12px', fontSize: 12, color: 'var(--sumi-2)',
                        fontStyle: 'italic', lineHeight: 1.55 }}>
            {s.summary}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {s.events.map((e, i) => (
              <div key={i} style={{ display: 'grid', gridTemplateColumns: '16px 46px 1fr',
                            gap: 12, alignItems: 'center' }}>
                <span style={{ color: e.kind === 'correction' ? 'var(--shu)' :
                                      e.kind === 'test' ? 'var(--jade)' : 'var(--sumi-3)' }}>
                  <EventGlyph kind={e.kind} size={12}/>
                </span>
                <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{e.t}</span>
                <span style={{ fontSize: 12, color: e.kind === 'correction' ? 'var(--shu)' : 'var(--sumi-2)' }}>
                  {e.text}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CODEBASE — orbital layout
// ────────────────────────────────────────────────────────────
function EnsoCodebase({ data, sol }) {
  return (
    <div style={{ padding: '36px 48px' }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>構 · Codebase</div>
      <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 28px' }}>{sol.name}</h1>

      <div style={{ display: 'grid', gridTemplateColumns: '1.3fr 1fr', gap: 36 }}>
        {/* Orbital graph */}
        <div style={{ border: 'var(--hairline)', borderRadius: 12, padding: 20,
                      background: 'var(--paper-2)', position: 'relative', minHeight: 420 }}>
          <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase' }}>Graph · communities</div>
          <svg viewBox="0 0 500 400" width="100%" height="400" style={{ marginTop: 10 }}>
            {/* community rings */}
            {[90, 160, 220].map((r, i) => (
              <circle key={i} cx="250" cy="200" r={r} fill="none"
                      stroke="var(--paper-edge)" strokeWidth="0.8" strokeDasharray="2 3"/>
            ))}
            {/* nodes per community */}
            {Array.from({length: 8}).map((_, i) => {
              const a = (i/8) * Math.PI * 2;
              return <circle key={'c1'+i} cx={250 + 90*Math.cos(a)} cy={200 + 90*Math.sin(a)}
                             r="5" fill="var(--sumi-2)"/>;
            })}
            {Array.from({length: 14}).map((_, i) => {
              const a = (i/14) * Math.PI * 2 + 0.2;
              return <circle key={'c2'+i} cx={250 + 160*Math.cos(a)} cy={200 + 160*Math.sin(a)}
                             r="3.5" fill="var(--sumi-3)"/>;
            })}
            {Array.from({length: 22}).map((_, i) => {
              const a = (i/22) * Math.PI * 2;
              return <circle key={'c3'+i} cx={250 + 220*Math.cos(a)} cy={200 + 220*Math.sin(a)}
                             r="2.5" fill="var(--sumi-4)"/>;
            })}
            {/* god node */}
            <circle cx="250" cy="200" r="14" fill="var(--shu-soft)" stroke="var(--shu)" strokeWidth="2"/>
            <text x="250" y="204" textAnchor="middle" fontSize="10" fill="var(--shu)"
                  fontFamily="var(--font-mono)">router</text>
            {/* spokes */}
            <g stroke="var(--shu)" strokeWidth="0.5" opacity="0.35">
              {Array.from({length: 8}).map((_, i) => {
                const a = (i/8) * Math.PI * 2;
                return <line key={i} x1="250" y1="200"
                             x2={250 + 85*Math.cos(a)} y2={200 + 85*Math.sin(a)}/>;
              })}
            </g>
            <text x="340" y="92"  fontSize="9.5" fill="var(--sumi-3)" fontFamily="var(--font-mono)">core · 8</text>
            <text x="100" y="120" fontSize="9.5" fill="var(--sumi-3)" fontFamily="var(--font-mono)">api · 14</text>
            <text x="370" y="330" fontSize="9.5" fill="var(--sumi-3)" fontFamily="var(--font-mono)">ui · 22</text>
          </svg>
        </div>

        <div>
          <div style={{ fontSize: 10.5, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                        textTransform: 'uppercase', marginBottom: 14 }}>Hotspots</div>
          {data.hotspots.map((h, i) => (
            <div key={i} style={{ padding: 14, border: 'var(--hairline)', borderRadius: 8,
                          marginBottom: 8, background: 'var(--paper)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <span className="ink-dot" style={{
                  background: h.severity === 'god' ? 'var(--shu)' :
                              h.severity === 'cluster' ? 'var(--amber)' : 'var(--jade)'
                }}/>
                <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi)' }}>{h.name}</span>
              </div>
              <div style={{ display: 'flex', gap: 16 }}>
                <Mini label="in"     value={h.fanIn}/>
                <Mini label="out"    value={h.fanOut}/>
                <Mini label="rework" value={h.rework} warn={h.rework > 3}/>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function Mini({ label, value, warn }) {
  return (
    <div>
      <div style={{ fontSize: 9.5, color: 'var(--sumi-3)', letterSpacing: '0.08em' }}>{label}</div>
      <div className="mono" style={{ fontSize: 14, color: warn ? 'var(--shu)' : 'var(--sumi)' }}>{value}</div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// COACHING
// ────────────────────────────────────────────────────────────
function EnsoCoaching({ data, applied, setApplied }) {
  return (
    <div style={{ padding: '40px 56px', maxWidth: 900 }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>師 · Coaching</div>
      <h1 className="display" style={{ fontSize: 30, fontWeight: 300, margin: '0 0 40px' }}>
        Recommendations from the week.
      </h1>

      {data.coaching.map((c, i) => {
        const isApplied = applied[c.id];
        return (
          <div key={c.id} style={{ marginBottom: 14, border: 'var(--hairline)', borderRadius: 12,
                          background: 'var(--paper-2)', overflow: 'hidden' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '120px 1fr', gap: 28, padding: 28 }}>
              <div style={{ textAlign: 'center' }}>
                <EnsoRing progress={c.urgency === 'high' ? 0.9 : c.urgency === 'medium' ? 0.55 : 0.25}
                          size={96} stroke={6}
                          color="oklch(0.58 0.15 35)"/>
                <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 8,
                              letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                  {c.urgency}
                </div>
              </div>
              <div>
                <p className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0, lineHeight: 1.3 }}>
                  {c.koan}
                </p>
                <p style={{ fontSize: 13, color: 'var(--sumi-2)', lineHeight: 1.6, marginTop: 10 }}>
                  {c.body}
                </p>
                <div style={{ display: 'flex', alignItems: 'center', gap: 14, marginTop: 16 }}>
                  <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
                          style={{ padding: '8px 16px', borderRadius: 8, fontSize: 12,
                                    background: isApplied ? 'var(--jade-soft)' : 'var(--shu)',
                                    color: isApplied ? 'var(--jade)' : 'var(--paper)',
                                    fontWeight: 500 }}>
                    {isApplied ? "✓ Applied" : c.action}
                  </button>
                  <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
                    {c.actionDetail}
                  </span>
                  <span style={{ flex: 1 }}/>
                  <span style={{ fontSize: 11, color: 'var(--sumi-2)', fontStyle: 'italic' }}>
                    {c.impact}
                  </span>
                </div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// CONFIG
// ────────────────────────────────────────────────────────────
function EnsoConfig({ data }) {
  return (
    <div style={{ padding: '40px 56px', maxWidth: 960 }}>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                    textTransform: 'uppercase', marginBottom: 4 }}>設 · Configuration</div>
      <h1 className="display" style={{ fontSize: 28, fontWeight: 300, margin: '0 0 32px' }}>
        Skills · Libraries · Assistants
      </h1>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 18, marginBottom: 24 }}>
        <Card title="Skills" kanji="技">
          {data.skills.slice(0, 5).map(s => (
            <div key={s.id} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 0',
                          borderBottom: 'var(--hairline)' }}>
              <span className="ink-dot" style={{ background: s.active ? 'var(--jade)' : 'var(--sumi-4)' }}/>
              <span style={{ flex: 1, fontSize: 12.5 }}>{s.name}</span>
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
                {s.solutions.length}
              </span>
            </div>
          ))}
        </Card>
        <Card title="Libraries" kanji="書">
          {data.libraries.map(l => (
            <div key={l.name} style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '8px 0',
                          borderBottom: 'var(--hairline)' }}>
              <span style={{ flex: 1, fontSize: 12.5 }}>{l.name}</span>
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>v{l.version}</span>
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>{l.pages}p</span>
            </div>
          ))}
        </Card>
      </div>
      <Card title="Assistants (ACPs)" kanji="助">
        {[
          { name: "Claude Code", version: "1.8.2", status: "connected" },
          { name: "Cursor",      version: "0.42",  status: "connected" },
          { name: "Zed",         version: "0.148", status: "available" }
        ].map(a => (
          <div key={a.name} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '10px 0',
                        borderBottom: 'var(--hairline)' }}>
            <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--jade)' : 'var(--sumi-4)' }}/>
            <span style={{ flex: 1, fontSize: 13 }}>{a.name}</span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>v{a.version}</span>
            <button style={{ fontSize: 11, padding: '4px 10px', borderRadius: 6,
                          border: 'var(--hairline)', color: 'var(--sumi-2)' }}>
              {a.status === 'connected' ? 'Configure' : 'Connect'}
            </button>
          </div>
        ))}
      </Card>
    </div>
  );
}

function Card({ title, kanji, children }) {
  return (
    <div style={{ padding: 22, border: 'var(--hairline)', borderRadius: 12, background: 'var(--paper-2)' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 12 }}>
        <span className="kanji" style={{ fontSize: 14, color: 'var(--shu)' }}>{kanji}</span>
        <span style={{ fontSize: 13, color: 'var(--sumi)' }}>{title}</span>
      </div>
      {children}
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// ONBOARDING — enso grows as steps complete
// ────────────────────────────────────────────────────────────
function EnsoOnboarding() {
  const [step, setStep] = React.useState(1);
  const progress = step / 4;
  const steps = [
    "Find assistants",
    "Scan folders",
    "Group solutions",
    "First index"
  ];
  return (
    <div style={{ padding: 60, display: 'flex', alignItems: 'center', gap: 72, minHeight: '100%' }}>
      <div style={{ position: 'relative', width: 320, height: 320 }}>
        <EnsoRing progress={progress} size={320} stroke={14} color="oklch(0.58 0.15 35)"/>
        <div style={{ position: 'absolute', inset: 0, display: 'flex', flexDirection: 'column',
                      alignItems: 'center', justifyContent: 'center' }}>
          <div className="display" style={{ fontSize: 72, fontWeight: 300, lineHeight: 1 }}>{step}</div>
          <div style={{ fontSize: 11, color: 'var(--sumi-3)', letterSpacing: '0.2em',
                        textTransform: 'uppercase', marginTop: 4 }}>of four</div>
        </div>
      </div>

      <div style={{ flex: 1, maxWidth: 460 }}>
        <div className="kanji" style={{ fontSize: 48, color: 'var(--shu)' }}>始</div>
        <h1 className="display" style={{ fontSize: 34, fontWeight: 300, margin: '14px 0 14px' }}>Begin.</h1>
        <p style={{ fontSize: 14, color: 'var(--sumi-2)', lineHeight: 1.6, marginBottom: 28 }}>
          Sensei watches how you work and, in time, helps you work better.
          One circle, four strokes.
        </p>

        {steps.map((t, i) => (
          <div key={i} style={{ display: 'flex', gap: 14, padding: '10px 0',
                        borderTop: i === 0 ? 'var(--hairline)' : 'none',
                        borderBottom: 'var(--hairline)', alignItems: 'center',
                        opacity: i + 1 > step ? 0.4 : 1 }}>
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)', width: 24 }}>
              {i+1 < step ? '✓' : '0' + (i+1)}
            </span>
            <span style={{ flex: 1, fontSize: 14 }}>{t}</span>
            {i+1 === step && <span className="mono" style={{ fontSize: 10, color: 'var(--shu)' }}>active</span>}
          </div>
        ))}

        <button onClick={() => setStep(Math.min(4, step+1))}
                style={{ marginTop: 28, padding: '11px 24px', background: 'var(--shu)',
                          color: 'var(--paper)', borderRadius: 8, fontSize: 13 }}>
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
      </div>
    </div>
  );
}

window.EnsoApp = EnsoApp;
