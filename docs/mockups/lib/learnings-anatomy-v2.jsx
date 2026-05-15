// Learnings · Anatomy v2 — quieter
//
// Same idea as v1 (one memory at a time, what/why/how/where), but the
// page is calmer:
//
//   ▸ Toolbar — sits above everything: shared ProjectFilter (pills + search
//                 input on the right) and a memory-search input. This is the
//                 single place to scope/filter; the sidebar stays quiet.
//   ▸ Sidebar  — flat list sorted by strength. A small surface glyph (則 ·
//                 技 · 作 · 令 · 禁) to the left of each item so it's
//                 scannable without reading. No search, no chips, no
//                 strength bar in the rail.
//   ▸ Stage    — H2 title (the "what"), one slim meta strip with strength +
//                 lifecycle counts, then THREE blocks (Why / How / Where) in
//                 a wide row. Each block trimmed.
//   ▸ Actions — three primary; rest behind a "···" menu.
//
// Reuses HealthChart, StrengthBar, inferHow, scopeChips from learnings-v2.

const { useState: laS, useMemo: laM } = React;

function LearningsAnatomyV2() {
  const L = window.LEARNINGS;
  const all = L.memories.filter(m => m.state !== "archived");
  const [query, setQuery]   = laS("");
  const [project, setProj]  = laS("all");

  const filtered = laM(() => {
    let xs = all;
    if (project !== "all") xs = xs.filter(m => m.scope.project === project || !m.scope.project);
    if (query.trim()) {
      const q = query.toLowerCase();
      xs = xs.filter(m => m.what.toLowerCase().includes(q) ||
                          m.because.toLowerCase().includes(q));
    }
    return [...xs].sort((a, b) => b.strength - a.strength);
  }, [query, project]);

  const [openId, setOpen] = laS(filtered[0]?.id || all[0].id);
  const memory = all.find(m => m.id === openId) || filtered[0] || all[0];

  return (
    <div className="sensei" data-screen-label="Observatory · Learnings · Anatomy v2"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <L2Hero kanji="覚" title="Every memory has the same anatomy."
              sub="What it is · why it matters · how it's surfaced · where it applies."
              right={<HealthChart memories={all}/>}/>

      {/* ── Toolbar · the single place to scope + search ───────────── */}
      <div style={{
                     borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center',
                     flexWrap: 'wrap'
}} className="py-3 px-6 gap-4" >
        <ProjectFilter value={project} onChange={setProj}
                        projects={L.projects}/>
        <span style={{ width: 1, height: 18, background: 'var(--edge)' }}/>
        <div style={{
 display: 'flex', alignItems: 'center',
                       background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 16,
                       minWidth: 200
}} className="gap-1 py-1 px-2" >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="none"
               style={{ flexShrink: 0, opacity: 0.55 }}>
            <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.3"/>
            <line x1="11" y1="11" x2="14" y2="14"
                  stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search memories…"
                 style={{
 flex: 1, fontSize: 11,
                           background: 'transparent', border: 'none',
                           color: 'var(--ink)', fontFamily: 'inherit',
                           outline: 'none', minWidth: 0
}} className="p-0" />
          {query && (
            <button onClick={() => setQuery("")}
                    style={{
 background: 'transparent', border: 'none',
                              color: 'var(--ink-4)', cursor: 'pointer', fontSize: 13, lineHeight: 1,
                              fontFamily: 'inherit'
}} className="p-0" >×</button>
          )}
        </div>
        <span style={{ flex: 1 }}/>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>
          {filtered.length} of {all.length} memories
        </span>
      </div>

      <div style={{ flex: 1, display: 'grid',
                     gridTemplateColumns: '244px 1fr',
                     minHeight: 0 }}>
        {/* ── Calm rail · just the list ────────────────────────── */}
        <aside style={{
 borderRight: 'var(--hairline)',
                         overflow: 'auto'
}} className="py-1 px-0" >
          {filtered.length === 0 && (
            <div style={{
 fontSize: 11,
                           color: 'var(--ink-4)', textAlign: 'center'
}} className="py-5 px-4" >
              no matches
            </div>
          )}
          {filtered.map(m => {
            const open = openId === m.id;
            const how = inferHow(m);
            return (
              <button key={m.id} onClick={() => setOpen(m.id)}
                      title={how.label}
                      style={{
 width: '100%', textAlign: 'left',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                 : '2px solid transparent',
                                cursor: 'pointer',
                                display: 'flex', alignItems: 'flex-start'
}} className="gap-2 py-2 px-3" >
                <span className="kanji mt-1"
                      style={{
 fontSize: 15, lineHeight: 1.3,
                                color: open ? 'var(--accent)' : 'var(--ink-3)',
                                flexShrink: 0
}}>
                  {how.glyph}
                </span>
                <span style={{ fontSize: 13, flex: 1, minWidth: 0,
                               color: open ? 'var(--ink)' : 'var(--ink-2)',
                               lineHeight: 1.4,
                               display: '-webkit-box', WebkitLineClamp: 2,
                               WebkitBoxOrient: 'vertical', overflow: 'hidden' }}>
                  {m.what}
                </span>
              </button>
            );
          })}
        </aside>

        {/* ── Stage ────────────────────────────────────── */}
        <main style={{ overflow: 'auto' }} className="py-6 px-7" >
          <AnatomyStageV2 memory={memory}/>
        </main>
      </div>
    </div>
  );
}

// ── Surface classification: agent · command · skill · rule · lint ────
const SURFACE_LABEL = {
  rule:    "Inline rule",
  skill:   "Skill",
  agent:   "Agent",
  command: "Command",
  lint:    "Lint check"
};

function AnatomyStageV2({ memory }) {
  const L = window.LEARNINGS;
  const how = inferHow(memory);
  const scope = scopeChips(memory.scope, L);

  return (
    <div style={{
 maxWidth: 720,
                   display: 'flex', flexDirection: 'column'
}} className="mt-1 mb-0 mx-auto" >
      {/* Eyebrow + surface tag, on one row */}
      <div style={{
 display: 'flex', alignItems: 'center',
                     fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase'
}} className="gap-3 mb-3" >
        <span>{memory.category.replace("_", "-")}</span>
        <span style={{ width: 3, height: 3, borderRadius: '50%',
                        background: 'var(--ink-4)' }}/>
        <span>{memory.state}</span>
        <span style={{ flex: 1 }}/>
        <span style={{
 display: 'inline-flex', alignItems: 'center',
                        color: 'var(--accent)', letterSpacing: '0.12em'
}} className="gap-2" >
          <span className="kanji" style={{ fontSize: 13, lineHeight: 1 }}>{how.glyph}</span>
          {SURFACE_LABEL[how.kind] || how.label}
        </span>
      </div>

      {/* The memory message — display-scale, like the welcome page */}
      <h2 className="display mt-0 mb-5" style={{
 fontSize: 40, fontWeight: 300, lineHeight: 1.15,
                                        letterSpacing: '-0.015em', color: 'var(--ink)'
}}>
        {memory.what}
      </h2>

      {/* Why — a quiet paragraph */}
      <p style={{
 fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.7
}} className="mt-0 mb-4" >
        {memory.because}
      </p>

      {/* Consequence of NOT following it — only when we have evidence */}
      {memory.violated > 0 && (
        <p style={{
 fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7
}} className="mt-0 mb-6" >
          When this slipped, sensei saw{" "}
          <span style={{ color: 'var(--accent)' }}>
            {memory.violated} correction{memory.violated === 1 ? "" : "s"}
          </span>{" "}
          across recent sessions
          {memory.references.bad_example && (
            <> — most often in <span className="mono" style={{ color: 'var(--ink-2)' }}>
              {memory.references.bad_example}
            </span></>
          )}.
        </p>
      )}
      {memory.violated === 0 && (
        <p style={{
 fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7
}} className="mt-0 mb-6" >
          Reinforced{" "}
          <span style={{ color: 'var(--ink-2)' }}>{memory.reinforced} times</span>
          {" "}without a violation. Last seen {memory.lastRelevant}.
        </p>
      )}

      {/* Hairline · two stacked observation rows in the teacher style */}
      <div style={{
 borderTop: 'var(--hairline)',
                     display: 'grid', gridTemplateColumns: '1fr 1fr'
}} className="gap-0" >

        {/* HOW · surfaced as */}
        <ObservationRow kanji={how.glyph} title="Surfaced as"
                        value={SURFACE_LABEL[how.kind] || how.label}
                        sub={<span className="mono" style={{ fontSize: 11,
                                       color: 'var(--ink-2)' }}>{how.target}</span>}/>

        {/* WHERE · scope */}
        <ObservationRow kanji="域" title="Applies in"
                        value={scope.find(c => c.label === "project")?.value || "global"}
                        sub={
                          <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
                            {scope.filter(c => c.label !== "project").slice(0, 4).map((c, i) => (
                              <span key={i} className={c.mono ? "mono" : ""}
                                    style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                                {c.value}{i < 3 ? " ·" : ""}
                              </span>
                            ))}
                          </div>
                        }/>
      </div>

      {/* Examples — when present, as a quiet note row */}
      {(memory.references.good_example || memory.references.bad_example) && (
        <div style={{
 borderTop: 'var(--hairline)',
                       display: 'flex', flexDirection: 'column'
}} className="gap-1 py-4 px-0" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            In the codebase
          </div>
          {memory.references.good_example && (
            <div className="mono" style={{ fontSize: 13, color: 'var(--success)' }}>
              ✓ {memory.references.good_example}
            </div>
          )}
          {memory.references.bad_example && (
            <div className="mono" style={{ fontSize: 13, color: 'var(--accent)' }}>
              ✗ {memory.references.bad_example}
            </div>
          )}
        </div>
      )}

      {/* Strength · meta */}
      <div style={{
 borderTop: 'var(--hairline)',
                     display: 'flex', alignItems: 'center',
                     fontSize: 11, color: 'var(--ink-3)'
}} className="gap-3 pt-4" >
        <span style={{ display: 'inline-flex', alignItems: 'center' }} className="gap-2" >
          <StrengthBar value={memory.strength}/>
          <span className="mono" style={{ color: 'var(--ink-2)' }}>
            strength {memory.strength}/5
          </span>
        </span>
        <Sep/>
        <span>learned {memory.learned}</span>
        <Sep/>
        <span>last seen {memory.lastRelevant}</span>
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2 mt-5" >
        <FlatBtn glyph="昇" label="Promote to rule"/>
        <FlatBtn glyph="育" label="Enrich"/>
        <FlatBtn glyph="渡" label="Apply elsewhere"/>
        <span style={{ flex: 1 }}/>
        <button title="More"
                style={{ width: 30, height: 28, fontSize: 13,
                          background: 'transparent',
                          border: 'var(--hairline)', borderRadius: 5,
                          color: 'var(--ink-3)', cursor: 'pointer',
                          letterSpacing: 1 }}>···</button>
      </div>
    </div>
  );
}

// One observation row in the "teacher" voice — kanji, eyebrow, value, sub.
function ObservationRow({ kanji, title, value, sub }) {
  return (
    <div style={{
 display: 'flex',
                   alignItems: 'flex-start'
}} className="gap-3 py-4 pl-0 pr-5" >
      <span className="kanji mt-1" style={{
 fontSize: 22, color: 'var(--accent)',
                                         lineHeight: 1
}}>{kanji}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                       textTransform: 'uppercase'
}} className="mb-1" >{title}</div>
        <div className="display mb-1" style={{
 fontSize: 17, color: 'var(--ink)', lineHeight: 1.3
}}>
          {value}
        </div>
        {sub && <div style={{ fontSize: 11, color: 'var(--ink-3)',
                                lineHeight: 1.5 }}>{sub}</div>}
      </div>
    </div>
  );
}

function Sep() {
  return <span style={{ width: 3, height: 3, borderRadius: '50%',
                         background: 'var(--ink-4)', display: 'inline-block' }}/>;
}

window.LearningsAnatomyV2 = LearningsAnatomyV2;
