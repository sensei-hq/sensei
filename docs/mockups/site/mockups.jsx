// Stylized "marketing screenshot" mockups of the key Sensei screens.
// These are not the real components — they're simplified, slightly idealized
// renderings authored specifically for the website. Three reasons for that:
//
//   1. The real components carry a lot of incidental detail (badges, hover
//      states, scroll affordances) that distract in a marketing context.
//   2. Marketing imagery is read in 2 seconds, not 20 — every line must
//      pull weight.
//   3. The real components are responsive; for a website hero we want fixed
//      compositions that look composed at any viewport.
//
// We export one component per screen plus a <BrowserChrome>/<AppChrome>
// wrapper. All three website variants consume them; they decide framing,
// scale, and shadowing.

const { useState: mUseState } = React;

// ─── Frame chrome ──────────────────────────────────────────────────
// The mockups are full Tauri-style app windows (not browser tabs) — the
// product is a desktop app and pretending otherwise would mislead.
function AppFrame({ children, title = "Sensei", width, height,
                    radius = 12, shadow = true, style = {} }) {
  return (
    <div style={{
      width, height,
      borderRadius: radius,
      background: 'var(--paper)',
      border: 'var(--hairline)',
      overflow: 'hidden',
      display: 'flex', flexDirection: 'column',
      boxShadow: shadow
        ? '0 30px 60px -20px rgba(20,18,14,0.18), 0 12px 24px -12px rgba(20,18,14,0.10)'
        : 'none',
      ...style
    }}>
      <div style={{
        height: 32, padding: '0 12px',
        display: 'flex', alignItems: 'center', gap: 8,
        borderBottom: 'var(--hairline)',
        background: 'var(--paper)',
        flexShrink: 0
      }}>
        <span style={{ display: 'flex', gap: 6 }}>
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.72 0.14 28)' }}/>
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.82 0.13 85)' }}/>
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.72 0.11 145)' }}/>
        </span>
        <div style={{ flex: 1, textAlign: 'center', fontSize: 11,
                       color: 'var(--sumi-3)', letterSpacing: '0.02em' }}>
          {title}
        </div>
        <span style={{ width: 30 }}/>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>{children}</div>
    </div>
  );
}

// Sidebar shell that the per-screen mocks reuse. Highlight active row.
function MockSidebar({ active = "home", showInstruments = false }) {
  const items = [
    { id: "home",      kanji: "今", label: "Today" },
    { id: "projects",  kanji: "場", label: "Projects",  badge: 4 },
    { id: "sessions",  kanji: "録", label: "Sessions",  badge: 41 },
    { id: "insights",  kanji: "察", label: "Insights",  badge: 6 },
    { id: "memories",  kanji: "覚", label: "Memories",  badge: 24 },
    { id: "libraries", kanji: "庫", label: "Libraries", badge: 14 }
  ];
  return (
    <aside style={{
      width: 168, padding: '16px 10px',
      background: 'var(--paper-2)',
      borderRight: 'var(--hairline)',
      display: 'flex', flexDirection: 'column', gap: 14
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 6,
                     padding: '0 6px' }}>
        <span className="kanji" style={{ fontSize: 16,
                       color: 'var(--shu)' }}>先</span>
        <span className="display" style={{ fontSize: 13 }}>Sensei</span>
      </div>
      <div>
        <div style={{ fontSize: 8.5, letterSpacing: '0.18em',
                       color: 'var(--sumi-3)', textTransform: 'uppercase',
                       padding: '0 8px 6px' }}>
          Observatory
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          {items.map(it => (
            <div key={it.id} style={{
              display: 'grid',
              gridTemplateColumns: 'auto 1fr auto',
              alignItems: 'center', gap: 8,
              padding: '5px 8px', borderRadius: 5,
              background: active === it.id ? 'var(--paper-3)' : 'transparent',
              fontSize: 11
            }}>
              <span className="kanji" style={{ fontSize: 11,
                       color: active === it.id ? 'var(--shu)' : 'var(--sumi-3)' }}>
                {it.kanji}
              </span>
              <span style={{ color: active === it.id
                                ? 'var(--sumi)' : 'var(--sumi-2)' }}>
                {it.label}
              </span>
              {it.badge != null && (
                <span className="mono" style={{ fontSize: 8.5,
                              color: 'var(--sumi-3)' }}>{it.badge}</span>
              )}
            </div>
          ))}
        </div>
      </div>
      <div>
        <div style={{ fontSize: 8.5, letterSpacing: '0.18em',
                       color: 'var(--sumi-3)', textTransform: 'uppercase',
                       padding: '0 8px 6px' }}>
          Active
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4,
                       padding: '0 8px', fontSize: 10.5,
                       color: 'var(--sumi-2)' }}>
          <div>kazoku-app</div>
          <div>tea-ceremony</div>
          <div>shoji-ui</div>
        </div>
      </div>
    </aside>
  );
}

// ─── Today (the hero shot — what most people will see first) ──────
function MockToday({ width = 720, height = 460, name = "Aiko" }) {
  return (
    <AppFrame title="Sensei · Today" width={width} height={height}>
      <MockSidebar active="home"/>
      <main style={{ flex: 1, padding: '28px 36px', overflow: 'hidden',
                       background: 'var(--paper)' }}>
        <div style={{ fontSize: 9.5, letterSpacing: '0.18em',
                       color: 'var(--sumi-3)', textTransform: 'uppercase' }}>
          Tuesday, March 12
        </div>
        <h1 className="display" style={{ fontSize: 24, fontWeight: 400,
                       margin: '6px 0 18px', letterSpacing: '-0.01em' }}>
          Good morning, {name}.
        </h1>

        {/* Hero observation */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr auto',
                       gap: 24, alignItems: 'start',
                       paddingBottom: 18, borderBottom: 'var(--hairline)' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 10,
                           marginBottom: 6 }}>
              <span className="kanji" style={{ fontSize: 22,
                             color: 'var(--shu)' }}>繰</span>
              <span style={{ fontSize: 9.5, letterSpacing: '0.18em',
                             color: 'var(--sumi-3)',
                             textTransform: 'uppercase' }}>
                Pattern recurring
              </span>
            </div>
            <div className="display" style={{ fontSize: 17, lineHeight: 1.4,
                           color: 'var(--sumi)', maxWidth: 380 }}>
              You've reached for <em>useEffect</em> three times this week
              when state could've stayed local. Worth a closer look?
            </div>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4,
                         alignItems: 'flex-end',
                         color: 'var(--sumi-3)', fontSize: 10 }}>
            <div className="mono" style={{ fontSize: 18,
                           color: 'var(--sumi)' }}>3×</div>
            <div>this week</div>
          </div>
        </div>

        {/* Two columns: insights + adopted */}
        <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 1fr',
                       gap: 24, marginTop: 22 }}>
          <div>
            <div style={{ fontSize: 9, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 10 }}>
              Also worth noticing
            </div>
            <div style={{ display: 'flex', flexDirection: 'column',
                           gap: 10 }}>
              {[
                { k: "結", l: "Refactor compounding well",     d: "kazoku-app" },
                { k: "問", l: "Tests skipped 4 sessions",       d: "shoji-ui" },
                { k: "灯", l: "New idiom emerging in shoji-ui", d: "shoji-ui" }
              ].map((it, i) => (
                <div key={i} style={{ display: 'grid',
                       gridTemplateColumns: 'auto 1fr auto',
                       gap: 10, alignItems: 'baseline',
                       padding: '8px 0', borderBottom:
                         i < 2 ? 'var(--ink-line)' : 'none' }}>
                  <span className="kanji" style={{ fontSize: 13,
                                 color: 'var(--sumi-2)' }}>{it.k}</span>
                  <span style={{ fontSize: 11.5,
                                 color: 'var(--sumi)' }}>{it.l}</span>
                  <span className="mono" style={{ fontSize: 9,
                                 color: 'var(--sumi-3)' }}>{it.d}</span>
                </div>
              ))}
            </div>
          </div>
          <div>
            <div style={{ fontSize: 9, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 10 }}>
              Adopted teachings
            </div>
            <div style={{ display: 'flex', flexDirection: 'column',
                           gap: 8, fontSize: 11,
                           color: 'var(--sumi-2)' }}>
              <div>· Prefer local state</div>
              <div>· Co-locate tests</div>
              <div>· Keep async at edges</div>
              <div style={{ color: 'var(--sumi-4)' }}>+ 21 more</div>
            </div>
          </div>
        </div>
      </main>
    </AppFrame>
  );
}

// ─── Sessions (digest with retro lanes) ───────────────────────────
function MockSessions({ width = 720, height = 460 }) {
  return (
    <AppFrame title="Sensei · Sessions" width={width} height={height}>
      <MockSidebar active="sessions"/>
      <main style={{ flex: 1, padding: '24px 32px', overflow: 'hidden',
                       background: 'var(--paper)' }}>
        <div style={{ display: 'flex', alignItems: 'baseline',
                       justifyContent: 'space-between', marginBottom: 18 }}>
          <div>
            <div style={{ fontSize: 9.5, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase' }}>Sessions · 録</div>
            <h1 className="display" style={{ fontSize: 22, fontWeight: 400,
                           margin: '4px 0 0' }}>The week in review</h1>
          </div>
          <div className="mono" style={{ fontSize: 11,
                           color: 'var(--sumi-3)' }}>8 · 5 · 2 · 1h 4m</div>
        </div>

        {/* Sparkline trend */}
        <div style={{ height: 60, marginBottom: 22, position: 'relative' }}>
          <svg viewBox="0 0 600 60" preserveAspectRatio="none"
                style={{ width: '100%', height: '100%' }}>
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10"
                  fill="none" stroke="var(--jade)" strokeWidth="1.5"
                  strokeLinecap="round"/>
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10 L 600 60 L 0 60 Z"
                  fill="var(--jade-soft)" stroke="none"/>
          </svg>
        </div>

        {/* Retro lanes */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr',
                       gap: 20 }}>
          {[
            { kanji: "良", title: "Going well",    accent: 'var(--jade)',
              items: ["Compound refactors", "Naming consistent",
                      "Clear test boundaries"] },
            { kanji: "破", title: "Not going well", accent: 'var(--shu)',
              items: ["Tests skipped 4×", "useEffect overreach",
                      "PRs sit > 24h"] },
            { kanji: "観", title: "Insights",      accent: 'var(--sumi-2)',
              items: ["New shoji idiom forming",
                      "Pattern in error handling",
                      "Tea-ceremony slower start"] }
          ].map((lane, i) => (
            <div key={i}>
              <div style={{ display: 'flex', alignItems: 'baseline',
                             gap: 8, marginBottom: 10 }}>
                <span className="kanji" style={{ fontSize: 13,
                               color: lane.accent }}>{lane.kanji}</span>
                <span style={{ fontSize: 10, letterSpacing: '0.16em',
                               color: 'var(--sumi-3)',
                               textTransform: 'uppercase' }}>
                  {lane.title}
                </span>
              </div>
              <div style={{ display: 'flex', flexDirection: 'column',
                             gap: 6, fontSize: 11,
                             color: 'var(--sumi-2)' }}>
                {lane.items.map((t, j) => (
                  <div key={j} style={{ paddingLeft: 8,
                                 borderLeft: `2px solid ${lane.accent}33` }}>
                    {t}
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </main>
    </AppFrame>
  );
}

// ─── Insights (memories + patterns triage view) ───────────────────
function MockInsights({ width = 720, height = 460 }) {
  const cards = [
    { kanji: "繰", title: "useEffect when state stays local",
      conf: 0.86, projects: 3, ages: "3 weeks" },
    { kanji: "問", title: "Tests skipped on hot paths",
      conf: 0.72, projects: 1, ages: "this week" },
    { kanji: "灯", title: "Shoji-style composable panels",
      conf: 0.64, projects: 1, ages: "5 days" },
    { kanji: "結", title: "Edge-only async, pure cores",
      conf: 0.91, projects: 4, ages: "2 months" }
  ];
  return (
    <AppFrame title="Sensei · Insights" width={width} height={height}>
      <MockSidebar active="insights"/>
      <main style={{ flex: 1, padding: '24px 32px', overflow: 'hidden',
                       background: 'var(--paper)' }}>
        <div style={{ marginBottom: 20 }}>
          <div style={{ fontSize: 9.5, letterSpacing: '0.18em',
                         color: 'var(--sumi-3)',
                         textTransform: 'uppercase' }}>Insights · 今</div>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400,
                         margin: '4px 0 0' }}>
            What sensei has noticed
          </h1>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr',
                       gap: 14 }}>
          {cards.map((c, i) => (
            <div key={i} style={{
              padding: '14px 16px',
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 8,
              display: 'flex', flexDirection: 'column', gap: 10
            }}>
              <div style={{ display: 'flex', alignItems: 'baseline',
                             gap: 10 }}>
                <span className="kanji" style={{ fontSize: 18,
                               color: 'var(--shu)' }}>{c.kanji}</span>
                <span className="display" style={{ fontSize: 13,
                               color: 'var(--sumi)', flex: 1 }}>
                  {c.title}
                </span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between',
                             fontSize: 9.5, color: 'var(--sumi-3)' }}>
                <span className="mono">{Math.round(c.conf * 100)}% confident</span>
                <span>{c.projects} projects · {c.ages}</span>
              </div>
              {/* tiny confidence bar */}
              <div style={{ height: 2, background: 'var(--paper-3)',
                             borderRadius: 1, overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${c.conf * 100}%`,
                               background: 'var(--shu)' }}/>
              </div>
            </div>
          ))}
        </div>
      </main>
    </AppFrame>
  );
}

// ─── Memories (anatomy of a single memory) ────────────────────────
function MockMemory({ width = 720, height = 460 }) {
  return (
    <AppFrame title="Sensei · Memory" width={width} height={height}>
      <MockSidebar active="memories"/>
      <main style={{ flex: 1, padding: '24px 32px', overflow: 'hidden',
                       background: 'var(--paper)' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 10,
                       marginBottom: 4 }}>
          <span className="kanji" style={{ fontSize: 24,
                         color: 'var(--shu)' }}>覚</span>
          <span style={{ fontSize: 9.5, letterSpacing: '0.18em',
                         color: 'var(--sumi-3)',
                         textTransform: 'uppercase' }}>
            Memory · adopted
          </span>
        </div>
        <h1 className="display" style={{ fontSize: 20, fontWeight: 400,
                         margin: '0 0 16px', letterSpacing: '-0.01em' }}>
          Prefer local component state to lifted state
        </h1>

        <div style={{ display: 'grid', gridTemplateColumns: '1.5fr 1fr',
                       gap: 24 }}>
          <div>
            <div style={{ fontSize: 9, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 10 }}>
              When to apply
            </div>
            <div style={{ fontSize: 12, color: 'var(--sumi)',
                           lineHeight: 1.65, marginBottom: 14 }}>
              When state is read by a single component and its direct children,
              keep it local. Lift only when a sibling needs it. Premature
              lifting creates effect chains that are hard to reason about.
            </div>
            <div style={{ fontSize: 9, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 8 }}>
              Examples sensei watched
            </div>
            <div style={{ display: 'flex', flexDirection: 'column',
                           gap: 6, fontSize: 11, color: 'var(--sumi-2)' }}>
              <div>· kazoku-app · Mar 8 · accordion state</div>
              <div>· shoji-ui · Mar 5 · panel collapse</div>
              <div>· tea-ceremony · Feb 28 · form draft</div>
            </div>
          </div>
          <aside style={{ paddingLeft: 18,
                           borderLeft: 'var(--hairline)' }}>
            <div style={{ fontSize: 9, letterSpacing: '0.18em',
                           color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 10 }}>
              Provenance
            </div>
            <div style={{ display: 'flex', flexDirection: 'column',
                           gap: 8, fontSize: 10.5,
                           color: 'var(--sumi-2)' }}>
              <div><span className="mono" style={{ color: 'var(--sumi-3)' }}>seen</span> 17 sessions</div>
              <div><span className="mono" style={{ color: 'var(--sumi-3)' }}>first</span> Feb 14</div>
              <div><span className="mono" style={{ color: 'var(--sumi-3)' }}>conf</span> 91%</div>
              <div><span className="mono" style={{ color: 'var(--sumi-3)' }}>by</span> you</div>
            </div>
            <div style={{ height: 1, background: 'var(--paper-edge)',
                           margin: '14px 0' }}/>
            <div style={{ fontSize: 11, color: 'var(--sumi-2)',
                           lineHeight: 1.5 }}>
              Adopted into 4 projects. Sensei surfaces it when local
              state could replace a useEffect chain.
            </div>
          </aside>
        </div>
      </main>
    </AppFrame>
  );
}

// ─── Instruments (playground) ─────────────────────────────────────
function MockInstruments({ width = 720, height = 460 }) {
  const tools = [
    { name: "fs.read",       kind: "query"  },
    { name: "fs.write",      kind: "action" },
    { name: "git.log",       kind: "query"  },
    { name: "git.diff",      kind: "query"  },
    { name: "shell.run",     kind: "action" },
    { name: "search.code",   kind: "query"  },
    { name: "test.run",      kind: "action" },
    { name: "pattern.promote", kind: "action" }
  ];
  return (
    <AppFrame title="Sensei · Instruments" width={width} height={height}>
      <MockSidebar active="instruments"/>
      <main style={{ flex: 1, display: 'flex', flexDirection: 'column',
                      overflow: 'hidden', background: 'var(--paper)' }}>
        {/* Tabs */}
        <div style={{ display: 'flex', gap: 0, padding: '14px 28px 0',
                       borderBottom: 'var(--hairline)' }}>
          {[
            { k: "具", l: "Playground", on: true  },
            { k: "録", l: "Replay",     on: false },
            { k: "健", l: "Health",     on: false }
          ].map((t, i) => (
            <div key={i} style={{
              padding: '8px 16px', display: 'flex', alignItems: 'baseline',
              gap: 6, borderBottom: t.on
                ? '1.5px solid var(--shu)' : '1.5px solid transparent',
              marginBottom: -1
            }}>
              <span className="kanji" style={{ fontSize: 12,
                             color: t.on ? 'var(--shu)' : 'var(--sumi-3)' }}>
                {t.k}
              </span>
              <span style={{ fontSize: 11,
                             color: t.on ? 'var(--sumi)' : 'var(--sumi-3)' }}>
                {t.l}
              </span>
            </div>
          ))}
        </div>

        <div style={{ flex: 1, padding: '20px 28px', overflow: 'hidden' }}>
          <div style={{ fontSize: 10.5, color: 'var(--sumi-2)',
                         marginBottom: 14, maxWidth: 480 }}>
            What can these tools do? Try them in isolation; sensei watches
            usage in the background.
          </div>

          {/* MCP chooser */}
          <div style={{ display: 'flex', gap: 6, marginBottom: 16 }}>
            {["filesystem", "git", "shell", "search", "sensei"].map((m, i) => (
              <div key={i} style={{
                padding: '5px 10px', fontSize: 10.5,
                borderRadius: 4, border: 'var(--hairline)',
                background: i === 0 ? 'var(--paper-3)' : 'transparent',
                color: i === 0 ? 'var(--sumi)' : 'var(--sumi-2)'
              }}>{m}</div>
            ))}
          </div>

          {/* Tool list */}
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr',
                         gap: 6 }}>
            {tools.map((t, i) => (
              <div key={i} style={{
                padding: '8px 10px',
                display: 'grid',
                gridTemplateColumns: 'auto 1fr auto',
                gap: 8, alignItems: 'center',
                fontSize: 10.5,
                borderBottom: 'var(--ink-line)'
              }}>
                <span className="mono" style={{ fontSize: 9,
                              padding: '2px 5px', borderRadius: 2,
                              background: t.kind === "action"
                                ? 'var(--shu-soft)' : 'var(--paper-3)',
                              color: t.kind === "action"
                                ? 'var(--shu)' : 'var(--sumi-3)',
                              textTransform: 'uppercase',
                              letterSpacing: '0.08em' }}>
                  {t.kind}
                </span>
                <span className="mono" style={{ color: 'var(--sumi)' }}>
                  {t.name}
                </span>
                <span style={{ fontSize: 9,
                              color: 'var(--sumi-3)' }}>try →</span>
              </div>
            ))}
          </div>
        </div>
      </main>
    </AppFrame>
  );
}

Object.assign(window, {
  AppFrame, MockSidebar,
  MockToday, MockSessions, MockInsights, MockMemory, MockInstruments
});
