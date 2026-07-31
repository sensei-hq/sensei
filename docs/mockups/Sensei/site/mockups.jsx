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
    <div className="bg-paper border border-paper-edge overflow-hidden flex flex-col" style={{
 width, height,
 borderRadius: radius,
 boxShadow: shadow
 ? '0 30px 60px -20px rgba(20,18,14,0.18), 0 12px 24px -12px rgba(20,18,14,0.10)'
 : 'none',
 ...style
 }}>
      <div style={{
 height: 32 }} className="gap-2 px-3 flex items-center border-b bg-paper shrink-0" >
        <span className="gap-1 flex" >
          <span className="rounded-full" style={{ width: 10, height: 10,
 background: 'oklch(0.72 0.14 28)' }}/>
          <span className="rounded-full" style={{ width: 10, height: 10,
 background: 'oklch(0.82 0.13 85)' }}/>
          <span className="rounded-full" style={{ width: 10, height: 10,
 background: 'oklch(0.72 0.11 145)' }}/>
        </span>
        <div className="flex-1 text-center text-ink-3" style={{ fontSize: 11, letterSpacing: '0.02em' }}>
          {title}
        </div>
        <span style={{ width: 30 }}/>
      </div>
      <div className="flex-1 min-h-0 flex" >{children}</div>
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
      <div key={id} style={{ gridTemplateColumns: 'auto 1fr auto',
 borderRadius: 5, background: active === id ? 'var(--paper-3)' : 'transparent', fontSize: 11 }} className="gap-2 py-1 px-2 grid items-center" >
        <span className="kanji" style={{ fontSize: 11, color: active === id ? 'var(--accent)' : 'var(--ink-3)' }}>{kanji}</span>
        <span className="whitespace-nowrap overflow-hidden text-ellipsis" style={{ color: active === id ? 'var(--ink)' : 'var(--ink-2)' }}>{label}</span>
        {badge != null && <span className="mono text-ink-4" style={{ fontSize: 10 }}>{badge}</span>}
      </div>
    );
  };
  return (
    <aside style={{ width: 178 }} className="py-4 px-2 gap-2 bg-paper-2 border-r flex flex-col overflow-hidden" >
      <div className="gap-2 px-1 mb-1 flex items-center" >
        <span className="inline-block bg-accent shrink-0" style={{ width: 18, height: 18,
 WebkitMaskImage: 'url(uploads/sensei.svg?v=3)', maskImage: 'url(uploads/sensei.svg?v=3)',
 WebkitMaskSize: 'contain', maskSize: 'contain', WebkitMaskRepeat: 'no-repeat', maskRepeat: 'no-repeat',
 WebkitMaskPosition: 'center', maskPosition: 'center' }} />
        <span className="display" style={{ fontSize: 14, lineHeight: 1 }}>Sensei</span>
      </div>
      <div className="px-2 flex items-center justify-between" >
        <span className="text-ink-3 uppercase" style={{ fontSize: 9, letterSpacing: '0.16em' }}>Observatory</span>
        <span className="flex bg-paper-3" style={{ borderRadius: 4, padding: 2, fontSize: 8.5 }}>
          <span className="bg-paper text-ink" style={{ padding: '1px 6px', borderRadius: 3 }}>All</span>
          <span className="text-ink-3" style={{ padding: '1px 6px' }}>Focus</span>
        </span>
      </div>
      <div className="gap-2 flex flex-col" >
        {clusters.map((c, ci) => (
          <div key={ci} className="gap-1 flex flex-col" >
            {c.label && <div style={{ fontSize: 8.5, letterSpacing: '0.12em' }} className="px-2 text-ink-4 uppercase font-semibold" >{c.label}</div>}
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
      <main className="py-6 px-8 flex-1 overflow-hidden bg-paper" >
        <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>
          Tuesday, March 12
        </div>
        <h1 className="display mt-1 mb-4 font-normal" style={{
 fontSize: 22, letterSpacing: '-0.01em'
 }}>
          Good morning, {name}.
        </h1>

        {/* Hero observation */}
        <div style={{ gridTemplateColumns: '1fr auto' }} className="gap-6 pb-4 grid items-start border-b" >
          <div>
            <div className="gap-2 mb-1 flex items-baseline" >
              <span className="kanji text-accent" style={{ fontSize: 22 }}>繰</span>
              <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>
                Pattern recurring
              </span>
            </div>
            <div className="display text-ink" style={{ fontSize: 17, lineHeight: 1.4, maxWidth: 380 }}>
              You've reached for <em>useEffect</em> three times this week
              when state could've stayed local. Worth a closer look?
            </div>
          </div>
          <div style={{ fontSize: 11
 }} className="gap-1 flex flex-col items-end text-ink-3" >
            <div className="mono text-ink" style={{ fontSize: 17 }}>3×</div>
            <div>this week</div>
          </div>
        </div>

        {/* Two columns: insights + adopted */}
        <div style={{ gridTemplateColumns: '1.4fr 1fr'
 }} className="gap-6 mt-6 grid" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
              Also worth noticing
            </div>
            <div className="gap-2 flex flex-col" >
              {[
                { k: "結", l: "Refactor compounding well",     d: "kazoku-app" },
                { k: "問", l: "Tests skipped 4 sessions",       d: "shoji-ui" },
                { k: "灯", l: "New idiom emerging in shoji-ui", d: "shoji-ui" }
              ].map((it, i) => (
                <div key={i} style={{
 gridTemplateColumns: 'auto 1fr auto', borderBottom:
 i < 2 ? 'var(--ink-line)' : 'none'
 }} className="gap-2 py-2 px-0 grid items-baseline" >
                  <span className="kanji text-ink-2" style={{ fontSize: 13 }}>{it.k}</span>
                  <span className="text-ink" style={{ fontSize: 11 }}>{it.l}</span>
                  <span className="mono text-ink-3" style={{ fontSize: 11 }}>{it.d}</span>
                </div>
              ))}
            </div>
          </div>
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
              Adopted teachings
            </div>
            <div style={{ fontSize: 11 }} className="gap-2 flex flex-col text-ink-2" >
              <div>· Prefer local state</div>
              <div>· Co-locate tests</div>
              <div>· Keep async at edges</div>
              <div className="text-ink-4" >+ 21 more</div>
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
      <main className="py-6 px-8 flex-1 overflow-hidden bg-paper" >
        <div className="mb-4 flex items-baseline justify-between" >
          <div>
            <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>Sessions · 録</div>
            <h1 className="display mt-1 mb-0 font-normal" style={{
 fontSize: 22 }}>The week in review</h1>
          </div>
          <div className="mono text-ink-3" style={{ fontSize: 11 }}>8 · 5 · 2 · 1h 4m</div>
        </div>

        {/* Sparkline trend */}
        <div style={{ height: 60 }} className="mb-6 relative" >
          <svg className="w-full h-full" viewBox="0 0 600 60" preserveAspectRatio="none"
 >
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10"
                  fill="none" stroke="var(--success)" strokeWidth="1.5"
                  strokeLinecap="round"/>
            <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10 L 600 60 L 0 60 Z"
                  fill="var(--success-soft)" stroke="none"/>
          </svg>
        </div>

        {/* Retro lanes */}
        <div style={{ gridTemplateColumns: '1fr 1fr 1fr'
 }} className="gap-4 grid" >
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
              <div className="gap-2 mb-2 flex items-baseline" >
                <span className="kanji" style={{ fontSize: 13,
                               color: lane.accent }}>{lane.kanji}</span>
                <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>
                  {lane.title}
                </span>
              </div>
              <div style={{ fontSize: 11 }} className="gap-1 flex flex-col text-ink-2" >
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
      <main className="py-6 px-8 flex-1 overflow-hidden bg-paper" >
        <div className="mb-4" >
          <div className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>Insights · 今</div>
          <h1 className="display mt-1 mb-0 font-normal" style={{
 fontSize: 22 }}>
            What sensei has noticed
          </h1>
        </div>
        <div style={{ gridTemplateColumns: '1fr 1fr'
 }} className="gap-3 grid" >
          {cards.map((c, i) => (
            <div key={i} style={{
 borderRadius: 8 }} className="py-3 px-4 gap-2 bg-paper-2 border border-paper-edge flex flex-col" >
              <div className="gap-2 flex items-baseline" >
                <span className="kanji text-accent" style={{ fontSize: 17 }}>{c.kanji}</span>
                <span className="display text-ink flex-1" style={{ fontSize: 13 }}>
                  {c.title}
                </span>
              </div>
              <div className="flex justify-between text-ink-3" style={{
 fontSize: 11 }}>
                <span className="mono">{Math.round(c.conf * 100)}% confident</span>
                <span>{c.projects} projects · {c.ages}</span>
              </div>
              {/* tiny confidence bar */}
              <div className="bg-paper-3 overflow-hidden" style={{ height: 2,
 borderRadius: 1 }}>
                <div className="h-full bg-accent" style={{ width: `${c.conf * 100}%` }}/>
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
      <main className="py-6 px-8 flex-1 overflow-hidden bg-paper" >
        <div className="gap-2 mb-1 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 22 }}>覚</span>
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.18em' }}>
            Memory · adopted
          </span>
        </div>
        <h1 className="display mt-0 mb-4 font-normal" style={{
 fontSize: 22, letterSpacing: '-0.01em'
 }}>
          Prefer local component state to lifted state
        </h1>

        <div style={{ gridTemplateColumns: '1.5fr 1fr'
 }} className="gap-6 grid" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
              When to apply
            </div>
            <div style={{
 fontSize: 13,
 lineHeight: 1.65
 }} className="mb-3 text-ink" >
              When state is read by a single component and its direct children,
              keep it local. Lift only when a sibling needs it. Premature
              lifting creates effect chains that are hard to reason about.
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
              Examples sensei watched
            </div>
            <div style={{ fontSize: 11 }} className="gap-1 flex flex-col text-ink-2" >
              <div>· kazoku-app · Mar 8 · accordion state</div>
              <div>· shoji-ui · Mar 5 · panel collapse</div>
              <div>· tea-ceremony · Feb 28 · form draft</div>
            </div>
          </div>
          <aside className="pl-4 border-l" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-2 text-ink-3 uppercase" >
              Provenance
            </div>
            <div style={{ fontSize: 11 }} className="gap-2 flex flex-col text-ink-2" >
              <div><span className="mono text-ink-3" >seen</span> 17 sessions</div>
              <div><span className="mono text-ink-3" >first</span> Feb 14</div>
              <div><span className="mono text-ink-3" >conf</span> 91%</div>
              <div><span className="mono text-ink-3" >by</span> you</div>
            </div>
            <div style={{
 height: 1, background: 'var(--edge)'
}} className="my-3 mx-0" />
            <div className="text-ink-2" style={{ fontSize: 11,
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
      <main className="flex-1 flex flex-col overflow-hidden bg-paper" >
        {/* Tabs */}
        <div className="gap-0 pt-3 pb-0 px-6 flex border-b" >
          {[
            { k: "具", l: "Playground", on: true  },
            { k: "録", l: "Replay",     on: false },
            { k: "健", l: "Health",     on: false }
          ].map((t, i) => (
            <div key={i} style={{ borderBottom: t.on
 ? '1.5px solid var(--accent)' : '1.5px solid transparent',
 marginBottom: -1
 }} className="py-2 px-4 gap-1 flex items-baseline" >
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

        <div className="flex-1 min-h-0 grid" style={{ gridTemplateColumns: '208px 1fr' }}>
          {/* tree of MCP groups → tools */}
          <div className="py-3 px-3 border-r overflow-hidden" >
            {[
              ["filesystem", [["fs.read","query"],["fs.write","action"]]],
              ["git",        [["git.log","query"],["git.diff","query"]]],
              ["shell",      [["shell.run","action"]]],
              ["search",     [["search.code","query"]]],
              ["sensei",     [["pattern.promote","action"]]],
            ].map(([grp, gtools]) => (
              <div key={grp} className="mb-2" >
                <div className="gap-2 mb-1 flex items-center" >
                  <span className="text-ink-4" style={{ fontSize: 8 }}>▾</span>
                  <span className="mono text-ink-2" style={{ fontSize: 11 }}>{grp}</span>
                  <span className="mono text-ink-4" style={{ fontSize: 9, marginLeft: 'auto' }}>{gtools.length}</span>
                </div>
                {gtools.map(([name, kind]) => {
                  const sel = name === "git.diff";
                  return (
                    <div key={name} style={{ borderRadius: 4,
 background: sel ? 'var(--paper-3)' : 'transparent' }} className="gap-2 py-1 px-2 ml-3 flex items-center" >
                      <span className="shrink-0" style={{ width: 5, height: 5, borderRadius: 1, background: kind === 'action' ? 'var(--accent)' : 'var(--ink-4)' }}/>
                      <span className="mono" style={{ fontSize: 11, color: sel ? 'var(--ink)' : 'var(--ink-2)' }}>{name}</span>
                    </div>
                  );
                })}
              </div>
            ))}
          </div>
          {/* playground for the selected tool */}
          <div className="py-3 px-4 gap-3 overflow-hidden flex flex-col" >
            <div className="gap-2 flex items-center" >
              <span className="mono text-ink" style={{ fontSize: 12 }}>git.diff</span>
              <span className="mono bg-paper-3 text-ink-3 uppercase" style={{ fontSize: 9, padding: '1px 6px', borderRadius: 3, letterSpacing: '0.08em' }}>query</span>
              <span className="flex-1" />
              <span className="bg-ink text-paper" style={{ fontSize: 11, padding: '4px 13px', borderRadius: 5 }}>Run</span>
            </div>
            <div className="text-ink-3" style={{ fontSize: 10.5, lineHeight: 1.5 }}>
              Diff a file against HEAD. Run a tool in isolation; sensei watches every call to learn which ones work.
            </div>
            <div>
              <div style={{ fontSize: 8.5, letterSpacing: '0.12em' }} className="mb-1 uppercase text-ink-4 font-semibold" >Arguments</div>
              <div style={{ borderRadius: 6, fontSize: 10.5 }} className="mono py-2 px-3 bg-paper-2 border border-paper-edge text-ink-2" >{`{ "path": "src/auth/refresh.ts" }`}</div>
            </div>
            <div className="flex-1 min-h-0 flex flex-col" >
              <div style={{ fontSize: 8.5, letterSpacing: '0.12em' }} className="mb-1 uppercase text-ink-4 font-semibold" >Result · 18ms</div>
              <div style={{ borderRadius: 6, fontSize: 10, lineHeight: 1.6 }} className="mono py-2 px-3 bg-paper-2 border border-paper-edge flex-1 overflow-hidden" >
                <div className="text-ink-3" >@@ -12,7 +12,9 @@ refreshToken()</div>
                <div className="text-accent" >- if (token.expired)</div>
                <div className="text-success" >+ if (token.expired &amp;&amp; !token.revoked)</div>
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
