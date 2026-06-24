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
        height: 32,
        display: 'flex', alignItems: 'center',
        borderBottom: 'var(--hairline)',
        background: 'var(--paper)',
        flexShrink: 0
}} className="gap-2 px-3" >
        <span style={{ display: 'flex' }} className="gap-1" >
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.72 0.14 28)' }}/>
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.82 0.13 85)' }}/>
          <span style={{ width: 10, height: 10, borderRadius: '50%',
                          background: 'oklch(0.72 0.11 145)' }}/>
        </span>
        <div style={{ flex: 1, textAlign: 'center', fontSize: 11,
                       color: 'var(--ink-3)', letterSpacing: '0.02em' }}>
          {title}
        </div>
        <span style={{ width: 30 }}/>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>{children}</div>
    </div>
  );
}

// Sidebar shell that the per-screen mocks reuse — mirrors the current app:
// Anchors · Needs you · Review · Settings clusters, with the All|Focus toggle.
function MockSidebar({ active = "home" }) {
  const clusters = [
    { label: null, items: [["home","家","Today"],["projects","場","Projects",4]] },
    { label: "Needs you", items: [["insights","今","Insights",6],["memories","覚","Memories",7],["impact","果","Impact",3],["traceability","巻","Traceability",4],["upgrades","贈","Upgrades",5]] },
    { label: "Review", items: [["sessions","録","Sessions",41],["libraries","庫","Libraries",14],["instruments","具","Instruments"],["logs","診","Logs"]] },
    { label: "Settings", items: [["connection","鍵","Connection"],["collective","群","Collective"],["configure","調","Preferences"]] },
  ];
  const Row = (it) => {
    const [id, kanji, label, badge] = it;
    return (
      <div key={id} style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto', alignItems: 'center',
            borderRadius: 5, background: active === id ? 'var(--paper-3)' : 'transparent', fontSize: 11 }} className="gap-2 py-1 px-2" >
        <span className="kanji" style={{ fontSize: 11, color: active === id ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
        <span style={{ color: active === id ? 'var(--ink)' : 'var(--ink-2)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
        {badge != null && <span className="mono" style={{ fontSize: 10, color: 'var(--ink-4)' }}>{badge}</span>}
      </div>
    );
  };
  return (
    <aside style={{ width: 178, background: 'var(--paper-2)', borderRight: 'var(--hairline)',
                    display: 'flex', flexDirection: 'column', overflow: 'hidden' }} className="py-4 px-2 gap-2" >
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 px-1 mb-1" >
        <span style={{ display: 'inline-block', width: 18, height: 18, background: 'var(--accent)',
                       WebkitMaskImage: 'url(uploads/sensei.svg?v=2)', maskImage: 'url(uploads/sensei.svg?v=2)',
                       WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
                       WebkitMaskPosition: 'center', maskPosition: 'center', flexShrink: 0 }} />
        <span className="display" style={{ fontSize: 14, lineHeight: 1 }}>Sensei</span>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }} className="px-2" >
        <span style={{ fontSize: 9, letterSpacing: '0.16em', color: 'var(--ink-3)', textTransform: 'uppercase' }}>Observatory</span>
        <span style={{ display: 'flex', background: 'var(--paper-3)', borderRadius: 4, padding: 2, fontSize: 8.5 }}>
          <span style={{ padding: '1px 6px', borderRadius: 3, background: 'var(--paper)', color: 'var(--ink)' }}>All</span>
          <span style={{ padding: '1px 6px', color: 'var(--ink-3)' }}>Focus</span>
        </span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
        {clusters.map((c, ci) => (
          <div key={ci} style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {c.label && <div style={{ fontSize: 8.5, letterSpacing: '0.12em', color: 'var(--ink-4)',
                          textTransform: 'uppercase', fontWeight: 600 }} className="px-2" >{c.label}</div>}
            {c.items.map(Row)}
          </div>
        ))}
      </div>
    </aside>
  );
}

// ─── Today (the hero shot — what most people will see first) ──────
function MockToday({ width = 720, height = 460, name = "Aiko" }) {
  return (
    <AppFrame title="Sensei · Today" width={width} height={height}>
      <MockSidebar active="home"/>
      <main style={{
 flex: 1, overflow: 'hidden',
                       background: 'var(--paper)'
}} className="py-5 px-6" >
        <div style={{ fontSize: 11, letterSpacing: '0.18em',
                       color: 'var(--ink-3)', textTransform: 'uppercase' }}>
          Tuesday, March 12
        </div>
        <h1 className="display mt-1 mb-4" style={{
 fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em'
}}>
          Good morning, {name}.
        </h1>

        {/* Hero observation */}
        <div style={{
 display: 'grid', gridTemplateColumns: '1fr auto', alignItems: 'start', borderBottom: 'var(--hairline)'
}} className="gap-5 pb-4" >
          <div>
            <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-1" >
              <span className="kanji" style={{ fontSize: 22,
                             color: 'var(--accent)' }}>繰</span>
              <span style={{ fontSize: 11, letterSpacing: '0.18em',
                             color: 'var(--ink-3)',
                             textTransform: 'uppercase' }}>
                Pattern recurring
              </span>
            </div>
            <div className="display" style={{ fontSize: 17, lineHeight: 1.4,
                           color: 'var(--ink)', maxWidth: 380 }}>
              You've reached for <em>useEffect</em> three times this week
              when state could've stayed local. Worth a closer look?
            </div>
          </div>
          <div style={{
 display: 'flex', flexDirection: 'column',
                         alignItems: 'flex-end',
                         color: 'var(--ink-3)', fontSize: 11
}} className="gap-1" >
            <div className="mono" style={{ fontSize: 17,
                           color: 'var(--ink)' }}>3×</div>
            <div>this week</div>
          </div>
        </div>

        {/* Two columns: insights + adopted */}
        <div style={{
 display: 'grid', gridTemplateColumns: '1.4fr 1fr'
}} className="gap-5 mt-5" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              Also worth noticing
            </div>
            <div style={{
 display: 'flex', flexDirection: 'column'
}} className="gap-2" >
              {[
                { k: "結", l: "Refactor compounding well",     d: "kazoku-app" },
                { k: "問", l: "Tests skipped 4 sessions",       d: "shoji-ui" },
                { k: "灯", l: "New idiom emerging in shoji-ui", d: "shoji-ui" }
              ].map((it, i) => (
                <div key={i} style={{
 display: 'grid',
                       gridTemplateColumns: 'auto 1fr auto', alignItems: 'baseline', borderBottom:
                         i < 2 ? 'var(--ink-line)' : 'none'
}} className="gap-2 py-2 px-0" >
                  <span className="kanji" style={{ fontSize: 13,
                                 color: 'var(--ink-2)' }}>{it.k}</span>
                  <span style={{ fontSize: 11,
                                 color: 'var(--ink)' }}>{it.l}</span>
                  <span className="mono" style={{ fontSize: 11,
                                 color: 'var(--ink-3)' }}>{it.d}</span>
                </div>
              ))}
            </div>
          </div>
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              Adopted teachings
            </div>
            <div style={{
 display: 'flex', flexDirection: 'column', fontSize: 11,
                           color: 'var(--ink-2)'
}} className="gap-2" >
              <div>· Prefer local state</div>
              <div>· Co-locate tests</div>
              <div>· Keep async at edges</div>
              <div style={{ color: 'var(--ink-4)' }}>+ 21 more</div>
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
      <main style={{
 flex: 1, overflow: 'hidden',
                       background: 'var(--paper)'
}} className="py-5 px-6" >
        <div style={{
 display: 'flex', alignItems: 'baseline',
                       justifyContent: 'space-between'
}} className="mb-4" >
          <div>
            <div style={{ fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase' }}>Sessions · 録</div>
            <h1 className="display mt-1 mb-0" style={{
 fontSize: 22, fontWeight: 400
}}>The week in review</h1>
          </div>
          <div className="mono" style={{ fontSize: 11,
                           color: 'var(--ink-3)' }}>8 · 5 · 2 · 1h 4m</div>
        </div>

        {/* Sparkline trend */}
        <div style={{ height: 60, position: 'relative' }} className="mb-5" >
          <svg viewBox="0 0 600 60" preserveAspectRatio="none"
                style={{ width: '100%', height: '100%' }}>
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10"
                  fill="none" stroke="var(--success)" strokeWidth="1.5"
                  strokeLinecap="round"/>
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10 L 600 60 L 0 60 Z"
                  fill="var(--success-soft)" stroke="none"/>
          </svg>
        </div>

        {/* Retro lanes */}
        <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr 1fr'
}} className="gap-4" >
          {[
            { kanji: "良", title: "Going well",    accent: 'var(--success)',
              items: ["Compound refactors", "Naming consistent",
                      "Clear test boundaries"] },
            { kanji: "破", title: "Not going well", accent: 'var(--accent)',
              items: ["Tests skipped 4×", "useEffect overreach",
                      "PRs sit > 24h"] },
            { kanji: "観", title: "Insights",      accent: 'var(--ink-2)',
              items: ["New shoji idiom forming",
                      "Pattern in error handling",
                      "Tea-ceremony slower start"] }
          ].map((lane, i) => (
            <div key={i}>
              <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-2" >
                <span className="kanji" style={{ fontSize: 13,
                               color: lane.accent }}>{lane.kanji}</span>
                <span style={{ fontSize: 11, letterSpacing: '0.16em',
                               color: 'var(--ink-3)',
                               textTransform: 'uppercase' }}>
                  {lane.title}
                </span>
              </div>
              <div style={{
 display: 'flex', flexDirection: 'column', fontSize: 11,
                             color: 'var(--ink-2)'
}} className="gap-1" >
                {lane.items.map((t, j) => (
                  <div key={j} style={{
                                 borderLeft: `2px solid ${lane.accent}33`
}} className="pl-2" >
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
      <main style={{
 flex: 1, overflow: 'hidden',
                       background: 'var(--paper)'
}} className="py-5 px-6" >
        <div className="mb-4" >
          <div style={{ fontSize: 11, letterSpacing: '0.18em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>Insights · 今</div>
          <h1 className="display mt-1 mb-0" style={{
 fontSize: 22, fontWeight: 400
}}>
            What sensei has noticed
          </h1>
        </div>
        <div style={{
 display: 'grid', gridTemplateColumns: '1fr 1fr'
}} className="gap-3" >
          {cards.map((c, i) => (
            <div key={i} style={{
              background: 'var(--paper-2)',
              border: 'var(--hairline)',
              borderRadius: 8,
              display: 'flex', flexDirection: 'column'
}} className="py-3 px-4 gap-2" >
              <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2" >
                <span className="kanji" style={{ fontSize: 17,
                               color: 'var(--accent)' }}>{c.kanji}</span>
                <span className="display" style={{ fontSize: 13,
                               color: 'var(--ink)', flex: 1 }}>
                  {c.title}
                </span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between',
                             fontSize: 11, color: 'var(--ink-3)' }}>
                <span className="mono">{Math.round(c.conf * 100)}% confident</span>
                <span>{c.projects} projects · {c.ages}</span>
              </div>
              {/* tiny confidence bar */}
              <div style={{ height: 2, background: 'var(--paper-3)',
                             borderRadius: 1, overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${c.conf * 100}%`,
                               background: 'var(--accent)' }}/>
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
      <main style={{
 flex: 1, overflow: 'hidden',
                       background: 'var(--paper)'
}} className="py-5 px-6" >
        <div style={{
 display: 'flex', alignItems: 'baseline'
}} className="gap-2 mb-1" >
          <span className="kanji" style={{ fontSize: 22,
                         color: 'var(--accent)' }}>覚</span>
          <span style={{ fontSize: 11, letterSpacing: '0.18em',
                         color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>
            Memory · adopted
          </span>
        </div>
        <h1 className="display mt-0 mb-4" style={{
 fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em'
}}>
          Prefer local component state to lifted state
        </h1>

        <div style={{
 display: 'grid', gridTemplateColumns: '1.5fr 1fr'
}} className="gap-5" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              When to apply
            </div>
            <div style={{
 fontSize: 13, color: 'var(--ink)',
                           lineHeight: 1.65
}} className="mb-3" >
              When state is read by a single component and its direct children,
              keep it local. Lift only when a sibling needs it. Premature
              lifting creates effect chains that are hard to reason about.
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              Examples sensei watched
            </div>
            <div style={{
 display: 'flex', flexDirection: 'column', fontSize: 11, color: 'var(--ink-2)'
}} className="gap-1" >
              <div>· kazoku-app · Mar 8 · accordion state</div>
              <div>· shoji-ui · Mar 5 · panel collapse</div>
              <div>· tea-ceremony · Feb 28 · form draft</div>
            </div>
          </div>
          <aside style={{
                           borderLeft: 'var(--hairline)'
}} className="pl-4" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em',
                           color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-2" >
              Provenance
            </div>
            <div style={{
 display: 'flex', flexDirection: 'column', fontSize: 11,
                           color: 'var(--ink-2)'
}} className="gap-2" >
              <div><span className="mono" style={{ color: 'var(--ink-3)' }}>seen</span> 17 sessions</div>
              <div><span className="mono" style={{ color: 'var(--ink-3)' }}>first</span> Feb 14</div>
              <div><span className="mono" style={{ color: 'var(--ink-3)' }}>conf</span> 91%</div>
              <div><span className="mono" style={{ color: 'var(--ink-3)' }}>by</span> you</div>
            </div>
            <div style={{
 height: 1, background: 'var(--edge)'
}} className="my-3 mx-0" />
            <div style={{ fontSize: 11, color: 'var(--ink-2)',
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
        <div style={{
 display: 'flex',
                       borderBottom: 'var(--hairline)'
}} className="gap-0 pt-3 pb-0 px-5" >
          {[
            { k: "具", l: "Playground", on: true  },
            { k: "録", l: "Replay",     on: false },
            { k: "健", l: "Health",     on: false }
          ].map((t, i) => (
            <div key={i} style={{
 display: 'flex', alignItems: 'baseline', borderBottom: t.on
                ? '1.5px solid var(--accent)' : '1.5px solid transparent',
              marginBottom: -1
}} className="py-2 px-4 gap-1" >
              <span className="kanji" style={{ fontSize: 13,
                             color: t.on ? 'var(--accent)' : 'var(--ink-3)' }}>
                {t.k}
              </span>
              <span style={{ fontSize: 11,
                             color: t.on ? 'var(--ink)' : 'var(--ink-3)' }}>
                {t.l}
              </span>
            </div>
          ))}
        </div>

        <div style={{ flex: 1, minHeight: 0, display: 'grid', gridTemplateColumns: '208px 1fr' }}>
          {/* tree of MCP groups → tools */}
          <div style={{ borderRight: 'var(--hairline)', overflow: 'hidden' }} className="py-3 px-3" >
            {[
              ["filesystem", [["fs.read","query"],["fs.write","action"]]],
              ["git",        [["git.log","query"],["git.diff","query"]]],
              ["shell",      [["shell.run","action"]]],
              ["search",     [["search.code","query"]]],
              ["sensei",     [["pattern.promote","action"]]],
            ].map(([grp, gtools]) => (
              <div key={grp} className="mb-2" >
                <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mb-1" >
                  <span style={{ fontSize: 8, color: 'var(--ink-4)' }}>▾</span>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>{grp}</span>
                  <span className="mono" style={{ fontSize: 9, color: 'var(--ink-4)', marginLeft: 'auto' }}>{gtools.length}</span>
                </div>
                {gtools.map(([name, kind]) => {
                  const sel = name === "git.diff";
                  return (
                    <div key={name} style={{ display: 'flex', alignItems: 'center', borderRadius: 4,
                          background: sel ? 'var(--paper-3)' : 'transparent' }} className="gap-2 py-1 px-2 ml-3" >
                      <span style={{ width: 5, height: 5, borderRadius: 1, background: kind === 'action' ? 'var(--accent)' : 'var(--ink-4)', flexShrink: 0 }}/>
                      <span className="mono" style={{ fontSize: 11, color: sel ? 'var(--ink)' : 'var(--ink-2)' }}>{name}</span>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
          {/* playground for the selected tool */}
          <div style={{ overflow: 'hidden', display: 'flex', flexDirection: 'column' }} className="py-3 px-4 gap-3" >
            <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
              <span className="mono" style={{ fontSize: 12, color: 'var(--ink)' }}>git.diff</span>
              <span className="mono" style={{ fontSize: 9, padding: '1px 6px', borderRadius: 3, background: 'var(--paper-3)',
                            color: 'var(--ink-3)', textTransform: 'uppercase', letterSpacing: '0.08em' }}>query</span>
              <span style={{ flex: 1 }}/>
              <span style={{ fontSize: 11, padding: '4px 13px', borderRadius: 5, background: 'var(--ink)', color: 'var(--paper)' }}>Run</span>
            </div>
            <div style={{ fontSize: 10.5, color: 'var(--ink-3)', lineHeight: 1.5 }}>
              Diff a file against HEAD. Run a tool in isolation; sensei watches every call to learn which ones work.
            </div>
            <div>
              <div style={{ fontSize: 8.5, letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--ink-4)', fontWeight: 600 }} className="mb-1" >Arguments</div>
              <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6, fontSize: 10.5, color: 'var(--ink-2)' }} className="mono py-2 px-3" >{`{ "path": "src/auth/refresh.ts" }`}</div>
            </div>
            <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
              <div style={{ fontSize: 8.5, letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--ink-4)', fontWeight: 600 }} className="mb-1" >Result · 18ms</div>
              <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 6, fontSize: 10, lineHeight: 1.6, flex: 1, overflow: 'hidden' }} className="mono py-2 px-3" >
                <div style={{ color: 'var(--ink-3)' }}>@@ -12,7 +12,9 @@ refreshToken()</div>
                <div style={{ color: 'var(--accent)' }}>- if (token.expired)</div>
                <div style={{ color: 'var(--success)' }}>+ if (token.expired &amp;&amp; !token.revoked)</div>
              </div>
            </div>
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
