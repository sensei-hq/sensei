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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Enso · Direction 2"
 >
      <TauriChrome title="Sensei  先生"/>
      <div className="flex-1 flex min-h-0" >
        <EnsoSidebar page={page} setPage={setPage}
                     solutions={data.solutions}
                     activeSolution={activeSolution} setActiveSolution={setActiveSolution}
                     data={data}/>
        <main className="flex-1 overflow-auto" >
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
 width: 64 }} className="gap-4 py-4 px-0 border-r flex flex-col items-center shrink-0 bg-paper-2" >
      <div className="kanji text-accent" style={{ fontSize: 22 }}>先</div>

      <div className="gap-2 flex flex-col items-center" >
        {solutions.map(s => {
          const isActive = activeSolution === s.id;
          return (
            <button className="rounded-full flex items-center justify-center" key={s.id}
 title={s.name}
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

      <hr className="hairline my-1 mx-0" style={{ width: 28 }}/>

      <div className="gap-1 flex flex-col items-center" >
        {PAGES.map(p => (
          <button className="flex items-center justify-center" key={p.id}
 title={p.label}
 onClick={() => setPage(p.id)}
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

      <div className="flex-1" />
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
    <div style={{ gridTemplateColumns: '1.2fr 1fr' }} className="py-8 px-12 gap-12 grid" >
      {/* LEFT: the ring + orbit of sessions */}
      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >
          Observatory · {sol.name}
        </div>
        <h1 className="display mt-0 mb-8 font-normal" style={{ fontSize: 28 }}>
          {sol.description}
        </h1>

        <div style={{ width: 440, height: 440 }} className="mx-auto relative" >
          {/* The ENSO ring itself */}
          <div className="absolute" style={{ inset: 0, color: delta >= 0 ? 'var(--success)' : 'var(--accent)' }}>
            <EnsoRing progress={sol.ftr} size={440} stroke={18}
                      color={delta >= 0 ? 'oklch(0.58 0.15 35)' : 'oklch(0.58 0.15 35)'}
                      trackColor="var(--ink)"/>
          </div>
          {/* Center readout */}
          <div className="absolute flex flex-col items-center justify-center" style={{ inset: 0,
 pointerEvents: 'none' }}>
            <div className="display font-light" style={{ fontSize: 56, lineHeight: 1,
 letterSpacing: '-0.03em' }}>
              {Math.round(sol.ftr * 100)}
              <span className="text-ink-3" style={{ fontSize: 28 }}>%</span>
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mt-1 uppercase text-ink-3" >一  First try right</div>
            <div className="mono mt-3" style={{
 fontSize: 13,
                          color: delta >= 0 ? 'var(--success)' : 'var(--accent)'
}}>
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
              <button className="absolute rounded-full" key={s.id}
 title={s.title}
 onClick={() => { setFocusedSession(s.id); setPage("sessions"); }}
 style={{ left: x - 7, top: y - 7,
 width: 14, height: 14,
 background: s.ftr ? 'var(--success)' : 'var(--accent)',
 border: '2px solid var(--paper)',
 boxShadow: '0 0 0 1px ' + (s.ftr ? 'var(--success)' : 'var(--accent)'),
 transition: 'transform .15s'
 }}
 onMouseEnter={e => e.currentTarget.style.transform = 'scale(1.6)'}
 onMouseLeave={e => e.currentTarget.style.transform = ''}/>
            );
          })}
        </div>

        <div style={{ borderRadius: 10 }} className="mt-6 py-4 px-4 flex justify-around border border-paper-edge bg-paper-2" >
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
      <div className="gap-8 flex flex-col" >
        <div style={{ borderRadius: 14 }} className="py-8 px-6 bg-paper-2 border border-paper-edge relative overflow-hidden" >
          {/* small enso watermark */}
          <svg className="absolute" width="120" height="120" style={{ top: -20, right: -20, opacity: 0.12 }}>
            <EnsoRingInline size={120} stroke={4} color="var(--accent)"/>
          </svg>

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
 borderRadius: 8, fontSize: 13, letterSpacing: '0.01em'
 }} className="py-2 px-4 font-medium" >
              {applied[topCoach.id] ? "✓  Applied" : topCoach.action + " →"}
            </button>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {topCoach.impact}
            </span>
          </div>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-4 uppercase text-ink-3" >Quality signals</div>
          <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-2 grid" >
            {[
              { k: "Pattern compliance", v: "94%",      d: "+3", good: true },
              { k: "Test coverage Δ",    v: "+2.1%",    d: "this week", good: true },
              { k: "Doc drift",          v: "3 files",  d: "brand-kit", good: false },
              { k: "Tokens / session",   v: "14.2k",    d: "−1.8k", good: true },
            ].map(s => (
              <div key={s.k} style={{ borderRadius: 8 }} className="p-3 border border-paper-edge bg-paper" >
                <div className="text-ink-3" style={{ fontSize: 11, letterSpacing: '0.05em' }}>{s.k}</div>
                <div className="display mt-1 font-normal" style={{ fontSize: 22 }}>{s.v}</div>
                <div className="mono mt-1" style={{ fontSize: 11, color: s.good ? 'var(--success)' : 'var(--accent)' }}>
                  {s.d}
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="mt-2 text-accent" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 uppercase text-ink-3" >FTR · 14 days</div>
          <Sparkline data={history} width={440} height={46} fill="var(--accent-soft)" showDots/>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value, suffix }) {
  return (
    <div>
      <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.1em' }}>{label}</div>
      <div className="display mt-1 font-normal" style={{ fontSize: 22 }}>{value}</div>
      <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>{suffix}</div>
    </div>
  );
}
function Divider() {
  return <div style={{ width: 1, background: 'var(--edge)' }}/>;
}

// A small variant of the ring for inline use in overview cards
function EnsoRingInline({ progress = 1, size = 120, stroke = 4, color = 'var(--accent)', startAngle = -140, sweep = 300 }) {
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
    <div style={{ maxWidth: 1120 }} className="py-8 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >全 · Overview</div>
      <h1 className="display mt-0 mb-8 font-light" style={{ fontSize: 28 }}>
        All solutions · <span className="mono text-accent" style={{ fontSize: 22 }}>78%</span> global
      </h1>

      <div style={{ gridTemplateColumns: 'repeat(3, 1fr)' }} className="gap-4 grid" >
        {data.solutions.map(s => {
          const up = s.ftr >= s.ftrPrev;
          return (
            <button key={s.id} onClick={() => { setActiveSolution(s.id); setPage("observatory"); }}
 style={{
 borderRadius: 16,
 transition: 'transform .15s, border-color .15s'
 }}
 onMouseEnter={e => { e.currentTarget.style.borderColor = 'var(--ink-3)'; e.currentTarget.style.transform = 'translateY(-2px)'; }}
 onMouseLeave={e => { e.currentTarget.style.borderColor = ''; e.currentTarget.style.transform = ''; }} className="py-6 px-6 border border-paper-edge bg-paper-2 text-left relative flex flex-col items-center" >
              <div className="kanji text-accent self-start" style={{ fontSize: 13 }}>{s.kanji}</div>
              <div className="display mt-1 font-normal self-start" style={{ fontSize: 17 }}>{s.name}</div>
              <div style={{ fontSize: 11 }} className="mb-4 text-ink-3 self-start" >{s.description}</div>
              <div className="relative" style={{ width: 180, height: 180 }}>
                <EnsoRing progress={s.ftr} size={180} stroke={10}
                          color="oklch(0.58 0.15 35)"/>
                <div className="absolute flex items-center justify-center flex-col" style={{ inset: 0 }}>
                  <span className="display font-light" style={{ fontSize: 40, lineHeight: 1 }}>
                    {Math.round(s.ftr*100)}
                  </span>
                  <span style={{ fontSize: 11, letterSpacing: '0.12em' }} className="mt-1 text-ink-3" >FTR</span>
                </div>
              </div>
              <div className="mono mt-4" style={{
 fontSize: 11,
                            color: up ? 'var(--success)' : 'var(--accent)'
}}>
                {up ? '↗' : '↘'} {((s.ftr - s.ftrPrev)*100 >= 0 ? '+' : '')}{Math.round((s.ftr - s.ftrPrev)*100)}% week
              </div>
              <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
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
    <div className="py-8 px-12" >
      <div className="gap-4 mb-6 flex items-baseline" >
        <h1 className="display m-0 font-light" style={{ fontSize: 28 }}>
          刻 · Sessions
        </h1>
        <div className="mono text-ink-3" style={{ fontSize: 11 }}>
          {sessions.length} of {data.sessions.length}
        </div>
        <div className="flex-1" />
        <div className="gap-1 flex" >
          {["all", "first-try", "corrected"].map(f => (
            <button key={f} onClick={() => setFilter(f)}
 style={{
 fontSize: 11, borderRadius: 999,
 background: filter === f ? 'var(--ink)' : 'transparent',
 color: filter === f ? 'var(--paper)' : 'var(--ink-2)' }} className="py-1 px-3 border border-paper-edge" >
              {f}
            </button>
          ))}
        </div>
      </div>

      <div className="gap-2 flex flex-col" >
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
    <div className="border border-paper-edge overflow-hidden" style={{ borderRadius: 10,
 background: expanded ? 'var(--paper-2)' : 'var(--paper)', transition: 'background .15s' }}>
      <button onClick={onToggle}
 style={{ gridTemplateColumns: '12px 80px 1fr 220px 80px 20px' }} className="gap-4 py-3 px-4 grid items-center w-full text-left" >
        <span className="ink-dot" style={{ background: s.ftr ? 'var(--success)' : 'var(--accent)' }}/>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>{s.id}</span>
        <div>
          <div className="text-ink" style={{ fontSize: 13 }}>{s.title}</div>
          <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
            {s.project} · {s.module}
          </div>
        </div>
        {/* Event ribbon */}
        <div className="gap-1 flex items-center" >
          {(s.events || [{kind:'start'},{kind:'edit'},{kind:'test'},{kind:'end'}]).map((e, i) => (
            <div key={i} style={{
              width: 12, height: 16, borderRadius: 2,
              background: e.kind === 'correction' ? 'var(--accent)' :
                          e.kind === 'test' ? 'var(--success-soft)' :
                          e.kind === 'edit' ? 'var(--ink-4)' : 'var(--edge)'
            }} title={e.kind}/>
          ))}
        </div>
        <span className="mono text-ink-3 text-right" style={{ fontSize: 11 }}>
          {s.duration}
        </span>
        <span className="text-ink-3" style={{ transform: expanded ? 'rotate(90deg)' : '',
 transition: 'transform .15s' }}>›</span>
      </button>

      {expanded && s.events && (
        <div className="pt-0 pb-4 pl-24 pr-4 border-t" >
          <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="pt-4 pb-3 text-ink-2 italic" >
            {s.summary}
          </div>
          <div className="gap-1 flex flex-col" >
            {s.events.map((e, i) => (
              <div key={i} style={{ gridTemplateColumns: '16px 46px 1fr' }} className="gap-3 grid items-center" >
                <span style={{ color: e.kind === 'correction' ? 'var(--accent)' :
                                      e.kind === 'test' ? 'var(--success)' : 'var(--ink-3)' }}>
                  <EventGlyph kind={e.kind} size={12}/>
                </span>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>{e.t}</span>
                <span style={{ fontSize: 13, color: e.kind === 'correction' ? 'var(--accent)' : 'var(--ink-2)' }}>
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
    <div className="py-8 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >構 · Codebase</div>
      <h1 className="display mt-0 mb-6 font-light" style={{ fontSize: 28 }}>{sol.name}</h1>

      <div style={{ gridTemplateColumns: '1.3fr 1fr' }} className="gap-8 grid" >
        {/* Orbital graph */}
        <div style={{ borderRadius: 12, minHeight: 420
 }} className="p-4 border border-paper-edge bg-paper-2 relative" >
          <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.12em' }}>Graph · communities</div>
          <svg viewBox="0 0 500 400" width="100%" height="400" className="mt-2" >
            {/* community rings */}
            {[90, 160, 220].map((r, i) => (
              <circle key={i} cx="250" cy="200" r={r} fill="none"
                      stroke="var(--edge)" strokeWidth="0.8" strokeDasharray="2 3"/>
            ))}
            {/* nodes per community */}
            {Array.from({length: 8}).map((_, i) => {
              const a = (i/8) * Math.PI * 2;
              return <circle key={'c1'+i} cx={250 + 90*Math.cos(a)} cy={200 + 90*Math.sin(a)}
                             r="5" fill="var(--ink-2)"/>;
            })}
            {Array.from({length: 14}).map((_, i) => {
              const a = (i/14) * Math.PI * 2 + 0.2;
              return <circle key={'c2'+i} cx={250 + 160*Math.cos(a)} cy={200 + 160*Math.sin(a)}
                             r="3.5" fill="var(--ink-3)"/>;
            })}
            {Array.from({length: 22}).map((_, i) => {
              const a = (i/22) * Math.PI * 2;
              return <circle key={'c3'+i} cx={250 + 220*Math.cos(a)} cy={200 + 220*Math.sin(a)}
                             r="2.5" fill="var(--ink-4)"/>;
            })}
            {/* god node */}
            <circle cx="250" cy="200" r="14" fill="var(--accent-soft)" stroke="var(--accent)" strokeWidth="2"/>
            <text x="250" y="204" textAnchor="middle" fontSize="10" fill="var(--accent)"
                  fontFamily="var(--font-mono)">router</text>
            {/* spokes */}
            <g stroke="var(--accent)" strokeWidth="0.5" opacity="0.35">
              {Array.from({length: 8}).map((_, i) => {
                const a = (i/8) * Math.PI * 2;
                return <line key={i} x1="250" y1="200"
                             x2={250 + 85*Math.cos(a)} y2={200 + 85*Math.sin(a)}/>;
              })}
            </g>
            <text x="340" y="92"  fontSize="9.5" fill="var(--ink-3)" fontFamily="var(--font-mono)">core · 8</text>
            <text x="100" y="120" fontSize="9.5" fill="var(--ink-3)" fontFamily="var(--font-mono)">api · 14</text>
            <text x="370" y="330" fontSize="9.5" fill="var(--ink-3)" fontFamily="var(--font-mono)">ui · 22</text>
          </svg>
        </div>

        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-3 text-ink-3 uppercase" >Hotspots</div>
          {data.hotspots.map((h, i) => (
            <div key={i} style={{ borderRadius: 8 }} className="p-3 mb-2 border border-paper-edge bg-paper" >
              <div className="gap-2 mb-2 flex items-center" >
                <span className="ink-dot" style={{
                  background: h.severity === 'god' ? 'var(--accent)' :
                              h.severity === 'cluster' ? 'var(--warning)' : 'var(--success)'
                }}/>
                <span className="mono text-ink" style={{ fontSize: 11 }}>{h.name}</span>
              </div>
              <div className="gap-4 flex" >
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
      <div className="text-ink-3" style={{ fontSize: 11, letterSpacing: '0.08em' }}>{label}</div>
      <div className="mono" style={{ fontSize: 13, color: warn ? 'var(--accent)' : 'var(--ink)' }}>{value}</div>
    </div>
  );
}

// ────────────────────────────────────────────────────────────
// COACHING
// ────────────────────────────────────────────────────────────
function EnsoCoaching({ data, applied, setApplied }) {
  return (
    <div style={{ maxWidth: 900 }} className="py-8 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >師 · Coaching</div>
      <h1 className="display mt-0 mb-8 font-light" style={{ fontSize: 28 }}>
        Recommendations from the week.
      </h1>

      {data.coaching.map((c, i) => {
        const isApplied = applied[c.id];
        return (
          <div key={c.id} style={{ borderRadius: 12 }} className="mb-3 border border-paper-edge bg-paper-2 overflow-hidden" >
            <div style={{ gridTemplateColumns: '120px 1fr' }} className="gap-6 p-6 grid" >
              <div className="text-center" >
                <EnsoRing progress={c.urgency === 'high' ? 0.9 : c.urgency === 'medium' ? 0.55 : 0.25}
                          size={96} stroke={6}
                          color="oklch(0.58 0.15 35)"/>
                <div className="mono mt-2 text-ink-3 uppercase" style={{
 fontSize: 11,
 letterSpacing: '0.12em' }}>
                  {c.urgency}
                </div>
              </div>
              <div>
                <p className="display m-0 font-normal" style={{ fontSize: 22, lineHeight: 1.3 }}>
                  {c.koan}
                </p>
                <p style={{ fontSize: 13, lineHeight: 1.6 }} className="mt-2 text-ink-2" >
                  {c.body}
                </p>
                <div className="gap-3 mt-4 flex items-center" >
                  <button onClick={() => setApplied({...applied, [c.id]: !isApplied})}
 style={{
 borderRadius: 8, fontSize: 13,
 background: isApplied ? 'var(--success-soft)' : 'var(--accent)',
 color: isApplied ? 'var(--success)' : 'var(--paper)' }} className="py-2 px-4 font-medium" >
                    {isApplied ? "✓ Applied" : c.action}
                  </button>
                  <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                    {c.actionDetail}
                  </span>
                  <span className="flex-1" />
                  <span className="text-ink-2 italic" style={{ fontSize: 11 }}>
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
    <div style={{ maxWidth: 960 }} className="py-8 px-12" >
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="mb-1 text-ink-3 uppercase" >設 · Configuration</div>
      <h1 className="display mt-0 mb-8 font-light" style={{ fontSize: 28 }}>
        Skills · Libraries · Assistants
      </h1>

      <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-4 mb-6 grid" >
        <Card title="Skills" kanji="技">
          {data.skills.slice(0, 5).map(s => (
            <div key={s.id} className="gap-2 py-2 px-0 flex items-center border-b" >
              <span className="ink-dot" style={{ background: s.active ? 'var(--success)' : 'var(--ink-4)' }}/>
              <span className="flex-1" style={{ fontSize: 13 }}>{s.name}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {s.solutions.length}
              </span>
            </div>
          ))}
        </Card>
        <Card title="Libraries" kanji="書">
          {data.libraries.map(l => (
            <div key={l.name} className="gap-2 py-2 px-0 flex items-center border-b" >
              <span className="flex-1" style={{ fontSize: 13 }}>{l.name}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>v{l.version}</span>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>{l.pages}p</span>
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
          <div key={a.name} className="gap-3 py-2 px-0 flex items-center border-b" >
            <span className="ink-dot" style={{ background: a.status === 'connected' ? 'var(--success)' : 'var(--ink-4)' }}/>
            <span className="flex-1" style={{ fontSize: 13 }}>{a.name}</span>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>v{a.version}</span>
            <button style={{
 fontSize: 11, borderRadius: 6 }} className="py-1 px-2 border border-paper-edge text-ink-2" >
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
    <div style={{ borderRadius: 12 }} className="p-6 border border-paper-edge bg-paper-2" >
      <div className="gap-2 mb-3 flex items-baseline" >
        <span className="kanji text-accent" style={{ fontSize: 13 }}>{kanji}</span>
        <span className="text-ink" style={{ fontSize: 13 }}>{title}</span>
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
    <div className="p-16 gap-16 flex items-center min-h-full" >
      <div className="relative" style={{ width: 320, height: 320 }}>
        <EnsoRing progress={progress} size={320} stroke={14} color="oklch(0.58 0.15 35)"/>
        <div className="absolute flex flex-col items-center justify-center" style={{ inset: 0 }}>
          <div className="display font-light" style={{ fontSize: 56, lineHeight: 1 }}>{step}</div>
          <div style={{
 fontSize: 11, letterSpacing: '0.2em' }} className="mt-1 text-ink-3 uppercase" >of four</div>
        </div>
      </div>

      <div className="flex-1" style={{ maxWidth: 460 }}>
        <div className="kanji text-accent" style={{ fontSize: 56 }}>始</div>
        <h1 className="display my-3 font-light" style={{ fontSize: 40 }}>Begin.</h1>
        <p style={{ fontSize: 13, lineHeight: 1.6 }} className="mb-6 text-ink-2" >
          Sensei watches how you work and, in time, helps you work better.
          One circle, four strokes.
        </p>

        {steps.map((t, i) => (
          <div key={i} style={{
 borderTop: i === 0 ? 'var(--hairline)' : 'none',
 opacity: i + 1 > step ? 0.4 : 1
 }} className="gap-3 py-2 px-0 flex border-b items-center" >
            <span className="mono text-ink-3" style={{ fontSize: 11, width: 24 }}>
              {i+1 < step ? '✓' : '0' + (i+1)}
            </span>
            <span className="flex-1" style={{ fontSize: 13 }}>{t}</span>
            {i+1 === step && <span className="mono text-accent" style={{ fontSize: 11 }}>active</span>}
          </div>
        ))}

        <button onClick={() => setStep(Math.min(4, step+1))}
 style={{ borderRadius: 8, fontSize: 13
 }} className="mt-6 py-3 px-6 bg-accent text-paper" >
          {step === 4 ? "Enter observatory →" : "Continue"}
        </button>
      </div>
    </div>
  );
}

window.EnsoApp = EnsoApp;
