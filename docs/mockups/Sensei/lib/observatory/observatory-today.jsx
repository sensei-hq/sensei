// Sensei · Today (Observatory home) — SHARED MODULE
// Extracted from observatory.jsx so the marketing site and the Observatory
// canvas render the SAME Today screen from one source — no mock divergence.
// Exports: OBS_DATA (window) + ObsSparkline · ObsFtrStrip · ObsHome · ObsHero
// · ObsInsights · ObsAdopted · ObsRecentSessions · ObsToday (standalone wrapper).
// Must load BEFORE observatory.jsx (which consumes these globals) and before
// any site variant that renders <ObsToday/>.

// ─── Fake daily data ─────────────────────────────────────────
window.OBS_DATA = {
  today: "Wed · 22 Apr",
  greetings: {
    morning: ["Good morning", "おはよう", "Settled in?"],
    afternoon: ["Good afternoon", "こんにちは"],
    evening: ["Evening", "こんばんは"]
  },

  // Signature metric — "First-Try-Right" rate, last 14 days
  ftr: {
    value: 0.78,
    delta: +0.06,           // vs prior 14d
    prev: 0.72,
    trend14: [0.71, 0.69, 0.74, 0.72, 0.68, 0.70, 0.73,
              0.75, 0.72, 0.78, 0.74, 0.79, 0.76, 0.78]
  },

  // The hero teaching — always exactly one, the most important thing right now.
  hero: {
    mature: {
      kanji: "聴",
      koan: "The AI does not know your auth.",
      body: "Three sessions corrected this week in lumen-auth — all touched refresh or device flow. There is no integration-test persona for this module yet.",
      impact: "Projected FTR + 14% in Lumen Cloud",
      action: "Draft a persona",
      source: "from s-2891 · s-2889 · s-2886",
      noticed: "noticed 2 days ago"
    },
    early: {
      kanji: "観",
      koan: "Still listening.",
      body: "Sensei has watched 4 sessions so far. A few early signals are forming in lumen-auth, but nothing confident enough to teach yet.",
      impact: "~2–3 more sessions until first lesson",
      action: null,
      source: "s-2891 · s-2890 · s-2889 · s-2888",
      noticed: "since setup · 2d ago"
    }
  },

  // Insights behind the hero — things worth seeing, never more than 3
  insights: {
    mature: [
      { kanji: "繰", label: "Pattern recurring",
        text: "Cache invalidation missed again in session s-2891.",
        tag: "3rd time", tone: "warn" },
      { kanji: "昇", label: "Teaching adopted",
        text: "Canvas smoothing pattern promoted to rule — applied in 4 subsequent sessions.",
        tag: "+7% FTR", tone: "good" },
      { kanji: "探", label: "Drift detected",
        text: "brand-tokens README is 47 days old. 3 APIs have drifted.",
        tag: "low urgency", tone: "mute" }
    ],
    early: [
      { kanji: "耳", label: "Listening",
        text: "Watching prompt style in lumen-canvas. Early signal: you prefer terse instructions.",
        tag: "forming", tone: "mute" },
      { kanji: "試", label: "Calibrating",
        text: "Sensei is still learning your correction cadence. Too early to suggest rules.",
        tag: "—", tone: "mute" }
    ]
  },

  // Rules/skills sensei has actually adopted (learning → system behavior)
  adopted: {
    mature: [
      { when: "2d ago", what: "Canvas smoothing pattern → rule",
        scope: "lumen-studio", source: "from session s-2890" },
      { when: "5d ago", what: "Auth refresh · clock-skew tolerance",
        scope: "lumen-cloud", source: "from session s-2891" },
      { when: "1w ago", what: "Token drift watchdog enabled",
        scope: "brand-kit", source: "manual + sensei recommend" }
    ],
    early: []
  },

  // Recent sessions — tight list
  recentSessions: [
    { id: "s-2891", project: "lumen-auth",   title: "Fix refresh token rotation",
      time: "10:42", duration: "38m", ftr: false, corrections: 3 },
    { id: "s-2890", project: "lumen-canvas", title: "Bezier smoothing tool",
      time: "09:15", duration: "22m", ftr: true,  corrections: 0 },
    { id: "s-2889", project: "lumen-auth",   title: "OAuth device flow",
      time: "Yesterday", duration: "1h 12m", ftr: false, corrections: 4 },
    { id: "s-2888", project: "brand-tokens", title: "Dark-mode color ramps",
      time: "Yesterday", duration: "18m", ftr: true,  corrections: 0 }
  ],

  projects: {
    active: [
      { id: "lumen-studio", kanji: "工", name: "Lumen Studio", ftr: 0.82, sessions7d: 41, warn: false },
      { id: "lumen-cloud",  kanji: "雲", name: "Lumen Cloud",  ftr: 0.64, sessions7d: 28, warn: true  },
      { id: "brand-kit",    kanji: "紋", name: "Brand Kit",    ftr: 0.91, sessions7d: 12, warn: false }
    ],
    recent: [
      { id: "sketch-tool",  kanji: "筆", name: "Sketch tool",   lastSeen: "3w ago",  sessions: 6 },
      { id: "old-docs",     kanji: "巻", name: "Docs site",     lastSeen: "2mo ago", sessions: 14 }
    ],
    archived: [
      { id: "prototype-x",  kanji: "試", name: "Prototype X",   archived: "6mo ago" }
    ]
  },

  // First-session guidance — fills the "quiet first days" after the scan,
  // before any sessions have been watched. Turns the wait into one concrete
  // task: go run a real session so sensei has something to learn from. The
  // first lesson forms around the third session.
  firstSession: {
    watched: 0,
    target: 3,
    assistant: "Claude Code",     // the assistant connected during setup
    projectsFound: 6,
    suggested: { id: "lumen-studio", kanji: "工", name: "Lumen Studio", path: "~/code/lumen" },
    command: "cd ~/code/lumen && claude"
  }
};

// ─── Small helpers ───────────────────────────────────────────
function ObsSparkline({ data, width = 120, height = 30, color = 'var(--accent)' }) {
  const min = Math.min(...data), max = Math.max(...data);
  const range = max - min || 1;
  const pad = 2;
  const step = (width - pad*2) / (data.length - 1);
  const pts = data.map((v,i) => [
    pad + i*step,
    pad + (height - pad*2) * (1 - (v - min)/range)
  ]);
  const d = pts.map((p,i) => (i ? "L" : "M") + p[0].toFixed(1) + " " + p[1].toFixed(1)).join(" ");
  return (
    <svg className="block overflow-visible" width={width} height={height} style={{ color }}>
      <path d={d} className="sparkline-path"/>
      <circle cx={pts[pts.length-1][0]} cy={pts[pts.length-1][1]} r={2.5} fill="currentColor"/>
    </svg>
  );
}

// 14-day FTR strip — one tall bar per day, height = FTR rate.
// Shows the actual shape of the last two weeks, not just a single average.
function ObsFtrStrip({ data, value, delta, dim = false }) {
  const w = 168, h = 56, n = data.length;
  const gap = 2, barW = (w - gap * (n - 1)) / n;
  const baseColor = dim ? 'var(--ink-3)' : 'var(--accent)';
  const bgColor   = dim ? 'var(--ink-4)' : 'var(--edge)';
  return (
    <svg className="block overflow-visible" width={w} height={h + 14} >
      {/* faint baseline rule at 50% to anchor reading */}
      <line x1="0" x2={w} y1={h - h*0.5} y2={h - h*0.5}
            stroke="var(--edge)" strokeDasharray="2 3"/>
      {data.map((v, i) => {
        const bh = Math.max(3, v * h);
        const isLast = i === n - 1;
        return (
          <g key={i}>
            <rect x={i * (barW + gap)} y={0} width={barW} height={h}
                  fill={bgColor} opacity="0.35"/>
            <rect x={i * (barW + gap)} y={h - bh} width={barW} height={bh}
                  fill={isLast ? baseColor : bgColor}
                  opacity={isLast ? 1 : 0.85}/>
          </g>
        );
      })}
      {/* day-of-week tick markers — only first, mid, last */}
      <text x={0} y={h + 11} fontSize="9" fill="var(--ink-3)"
             fontFamily="var(--font-ui)" letterSpacing="0.08em">14d ago</text>
      <text x={w} y={h + 11} fontSize="9" fill="var(--ink-3)" textAnchor="end"
             fontFamily="var(--font-ui)" letterSpacing="0.08em">today</text>
    </svg>
  );
}

// ─── Home (today) ───────────────────────────────────────────
function ObsHome({ mode, hero, insights, adopted, D }) {
  return (
    <ScreenShell label="Observatory · Today" header={
      <div 
 className="pt-6 pb-4 px-12 border-b bg-paper shrink-0" >
      {/* Greeting strip — pinned */}
      <div style={{ maxWidth: 1060
 }} className="mx-auto flex items-baseline justify-between" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            {D.today}
          </div>
          <h1 className="display m-0 font-normal" style={{
 fontSize: 28,
 letterSpacing: '-0.01em'
 }}>
            Good morning, {(() => {
              // Honor whatever name the user committed in the wizard's
              // Preferences stage. Falls back to the prototype's default
              // persona "Aiko" when no override is stored.
              try {
                const raw = localStorage.getItem("sensei-settings");
                const n = raw && JSON.parse(raw).displayName;
                if (n && n.trim()) return n.trim();
              } catch {}
              return "Aiko";
            })()}.
          </h1>
        </div>
        <div style={{
 color: mode === "early" ? 'var(--ink-3)' : 'var(--ink-2)'
 }} className="gap-6 flex items-end" >
          <div className="text-right" >
            <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>
              First-Try-Right · 14d
            </div>
            <div className="gap-2 mt-1 flex items-baseline justify-end" >
              <span className="display font-normal"
 style={{ fontSize: 40, lineHeight: 1,
 color: mode === "early" ? 'var(--ink-3)' : 'var(--ink)' }}>
                {Math.round(D.ftr.value * 100)}
              </span>
              <span className="text-ink-3" style={{ fontSize: 13 }}>%</span>
              <span className="mono ml-1"
                     style={{
 fontSize: 11,
                               color: D.ftr.delta >= 0 ? 'var(--success)' : 'var(--warning)'
}}>
                {D.ftr.delta >= 0 ? "↑" : "↓"} {Math.abs(Math.round(D.ftr.delta * 100))}%
              </span>
            </div>
          </div>
          <ObsFtrStrip data={D.ftr.trend14} value={D.ftr.value}
                        delta={D.ftr.delta} dim={mode === "early"}/>
        </div>
      </div>
      </div>
    }>
      <div style={{ maxWidth: 1060 }} className="pt-8 pb-12 px-12 mx-auto" >
      {/* Hero koan — the one focal thing */}
      <ObsHero hero={hero} mode={mode}/>

      {/* Two columns: Insights + Adopted teachings */}
      <div style={{ gridTemplateColumns: '1.4fr 1fr'
 }} className="mt-8 gap-8 grid" >
        <ObsInsights items={insights} mode={mode}/>
        <ObsAdopted items={adopted} mode={mode}/>
      </div>

      {/* Recent sessions — compact */}
      <ObsRecentSessions sessions={D.recentSessions}/>
      </div>
    </ScreenShell>
  );
}

function ObsHero({ hero, mode }) {
  return (
    <div style={{ borderRadius: 12, gridTemplateColumns: 'auto 1fr'
 }} className="gap-6 py-8 px-8 relative bg-paper-2 border border-paper-edge grid" >
      {/* Giant kanji — still the focal anchor */}
      <div className="relative" >
        <div className="kanji text-accent" style={{
 fontSize: 56, lineHeight: 1,
 opacity: mode === "early" ? 0.55 : 1
 }}>{hero.kanji}</div>
        <div className="absolute text-ink-3 uppercase" style={{ left: -6, top: -4,
 fontSize: 11, letterSpacing: '0.18em', writingMode: 'vertical-rl',
 transform: 'rotate(180deg)', height: 96
 }}>
          {mode === "early" ? "sensei is listening" : "sensei speaks"}
        </div>
      </div>

      <div className="flex flex-col" >
        <div className="display mb-3 font-normal text-ink" style={{
 fontSize: 28, letterSpacing: '-0.01em',
 lineHeight: 1.2 }}>
          {hero.koan}
        </div>
        <p style={{
 fontSize: 15, lineHeight: 1.65, maxWidth: 620
 }} className="mt-0 mb-4 text-ink-2" >
          {hero.body}
        </p>

        <div style={{
 marginTop: 'auto'
 }} className="gap-4 flex items-center flex-wrap" >
          {hero.action && (
            <button style={{
 fontSize: 13, borderRadius: 6, letterSpacing: 0.2 }} className="py-2 px-4 gap-2 bg-ink text-paper inline-flex items-center" >
              {hero.action} →
            </button>
          )}
          <div style={{
 fontSize: 13 }} className="gap-1 text-accent flex items-center" >
            <span className="rounded-full bg-accent" style={{ width: 5, height: 5 }}/>
            {hero.impact}
          </div>
          <span className="flex-1" />
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            {hero.source} · {hero.noticed}
          </span>
        </div>
      </div>
    </div>
  );
}

function ObsInsights({ items, mode }) {
  return (
    <div>
      <div className="mb-4 flex items-baseline justify-between" >
        <h2 className="display m-0 font-normal" style={{
 fontSize: 17,
 letterSpacing: '-0.005em'
 }}>
          {mode === "early" ? "Early signals" : "Also worth noticing"}
        </h2>
        <button className="text-ink-3" style={{ fontSize: 11 }}>all insights →</button>
      </div>
      <div className="gap-1 flex flex-col" >
        {items.map((x, i) => {
          const toneColor =
            x.tone === "warn" ? 'var(--warning)' :
            x.tone === "good" ? 'var(--success)'  : 'var(--ink-3)';
          return (
            <button key={i} style={{ gridTemplateColumns: 'auto 1fr auto',
 borderRadius: 6 }} className="gap-3 py-3 px-4 grid items-start text-left border-b" >
              <span className="kanji" style={{ fontSize: 22, color: toneColor,
                            width: 26 }}>{x.kanji}</span>
              <div>
                <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 uppercase text-ink-3" >
                  {x.label}
                </div>
                <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55 }}>
                  {x.text}
                </div>
              </div>
              <span className="mono py-1 px-2 whitespace-nowrap" style={{
 fontSize: 11, color: toneColor, borderRadius: 3,
 background: x.tone === "warn" ? 'var(--warning-soft)' :
 x.tone === "good" ? 'var(--success-soft)' :
 'var(--paper-3)' }}>
                {x.tag}
              </span>
            </button>
          );
        })}
        {items.length === 0 && (
          <div style={{
 fontSize: 13,
 border: '1px dashed var(--edge)', borderRadius: 8
 }} className="p-4 text-ink-4 italic text-center" >
            Nothing yet. Keep working — sensei watches.
          </div>
        )}
      </div>
    </div>
  );
}

function ObsAdopted({ items, mode }) {
  return (
    <div>
      <div className="mb-4 flex items-baseline justify-between" >
        <h2 className="display m-0 font-normal" style={{
 fontSize: 17,
 letterSpacing: '-0.005em'
 }}>
          System has learned
        </h2>
        <button className="text-ink-3" style={{ fontSize: 11 }}>all teachings →</button>
      </div>
      <div className="gap-2 flex flex-col" >
        {items.map((x, i) => (
          <div key={i} style={{
 borderRadius: 6,
 borderLeft: '2px solid var(--accent)'
 }} className="py-3 px-3 bg-paper-2 border border-paper-edge" >
            <div className="gap-2 mb-1 flex items-baseline" >
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {x.when}
              </span>
              <span className="text-ink-4" style={{ fontSize: 11 }}>·</span>
              <span className="mono text-accent" style={{ fontSize: 11 }}>
                {x.scope}
              </span>
            </div>
            <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.45 }}>
              {x.what}
            </div>
            <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
              {x.source}
            </div>
          </div>
        ))}
        {items.length === 0 && (
          <div style={{
 border: '1px dashed var(--edge)', borderRadius: 8 }} className="py-6 px-4 text-center" >
            <div className="kanji mb-2 text-ink-4" style={{
 fontSize: 28 }}>空</div>
            <div className="text-ink-3" style={{ fontSize: 13, lineHeight: 1.5 }}>
              No teachings adopted yet.<br/>
              Sensei needs a few more sessions to be confident.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function ObsRecentSessions({ sessions }) {
  return (
    <div className="mt-8" >
      <div className="mb-3 flex items-baseline justify-between" >
        <h2 className="display m-0 font-normal" style={{ fontSize: 17 }}>
          Recent sessions
        </h2>
        <button className="text-ink-3" style={{ fontSize: 11 }}>all sessions →</button>
      </div>
      <div className="flex flex-col" >
        {sessions.map(s => (
          <button key={s.id} style={{
 gridTemplateColumns: 'auto 120px 1fr auto auto auto' }} className="gap-4 py-3 px-1 grid items-center text-left border-b" >
            <span className="rounded-full" style={{ width: 8, height: 8,
 background: s.ftr ? 'var(--success)' : 'var(--warning)' }}/>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {s.project}
            </span>
            <span className="text-ink-2 overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 13 }}>
              {s.title}
            </span>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {s.corrections === 0 ? "first-try" : `${s.corrections}×`}
            </span>
            <span className="mono text-ink-3 text-right" style={{ fontSize: 11,
 minWidth: 50 }}>
              {s.duration}
            </span>
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>
              {s.time}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

// ─── ObsToday — standalone Today for embedding (site gallery / hero) ──
// The Observatory shell passes its own props; the site has no shell, so this
// wrapper supplies mature-state data straight from OBS_DATA. Renders just the
// main panel (ScreenShell) — pair it with <AppFrame>/<MockSidebar> for chrome.
function ObsToday({ mode = "mature" }) {
  const D = window.OBS_DATA;
  return (
    <ObsHome mode={mode}
             hero={D.hero[mode]}
             insights={D.insights[mode]}
             adopted={D.adopted[mode]}
             D={D}/>
  );
}

Object.assign(window, {
  ObsSparkline, ObsFtrStrip,
  ObsHome, ObsHero, ObsInsights, ObsAdopted, ObsRecentSessions,
  ObsToday
});
