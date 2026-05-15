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
      <div style={{ padding: '12px 32px',
                     borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', gap: 16,
                     flexWrap: 'wrap' }}>
        <ProjectFilter value={project} onChange={setProj}
                        projects={L.projects}/>
        <span style={{ width: 1, height: 18, background: 'var(--edge)' }}/>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4,
                       padding: '4px 8px',
                       background: 'var(--paper-2)',
                       border: 'var(--hairline)', borderRadius: 16,
                       minWidth: 200 }}>
          <svg width="11" height="11" viewBox="0 0 16 16" fill="none"
               style={{ flexShrink: 0, opacity: 0.55 }}>
            <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.3"/>
            <line x1="11" y1="11" x2="14" y2="14"
                  stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search memories…"
                 style={{ flex: 1, padding: 0, fontSize: 11,
                           background: 'transparent', border: 'none',
                           color: 'var(--ink)', fontFamily: 'inherit',
                           outline: 'none', minWidth: 0 }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ background: 'transparent', border: 'none',
                              color: 'var(--ink-4)', cursor: 'pointer',
                              padding: 0, fontSize: 13, lineHeight: 1,
                              fontFamily: 'inherit' }}>×</button>
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
        <aside style={{ borderRight: 'var(--hairline)',
                         overflow: 'auto', padding: '4px 0' }}>
          {filtered.length === 0 && (
            <div style={{ padding: '24px 16px', fontSize: 11,
                           color: 'var(--ink-4)', textAlign: 'center' }}>
              no matches
            </div>
          )}
          {filtered.map(m => {
            const open = openId === m.id;
            const how = inferHow(m);
            return (
              <button key={m.id} onClick={() => setOpen(m.id)}
                      title={how.label}
                      style={{ width: '100%', textAlign: 'left',
                                padding: '8px 12px 8px 12px',
                                background: open ? 'var(--paper-2)' : 'transparent',
                                borderLeft: open ? '2px solid var(--accent)'
                                                 : '2px solid transparent',
                                cursor: 'pointer',
                                display: 'flex', alignItems: 'flex-start', gap: 8 }}>
                <span className="kanji"
                      style={{ fontSize: 15, lineHeight: 1.3,
                                color: open ? 'var(--accent)' : 'var(--ink-3)',
                                flexShrink: 0, marginTop: 4 }}>
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
        <main style={{ overflow: 'auto', padding: '32px 48px 32px' }}>
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
    <div style={{ maxWidth: 720, margin: '4px auto 0',
                   display: 'flex', flexDirection: 'column' }}>
      {/* Eyebrow + surface tag, on one row */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12,
                     fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                     textTransform: 'uppercase', marginBottom: 12 }}>
        <span>{memory.category.replace("_", "-")}</span>
        <span style={{ width: 3, height: 3, borderRadius: '50%',
                        background: 'var(--ink-4)' }}/>
        <span>{memory.state}</span>
        <span style={{ flex: 1 }}/>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8,
                        color: 'var(--accent)', letterSpacing: '0.12em' }}>
          <span className="kanji" style={{ fontSize: 13, lineHeight: 1 }}>{how.glyph}</span>
          {SURFACE_LABEL[how.kind] || how.label}
        </span>
      </div>

      {/* The memory message — display-scale, like the welcome page */}
      <h2 className="display" style={{ fontSize: 40, fontWeight: 300, lineHeight: 1.15,
                                        letterSpacing: '-0.015em',
                                        margin: '0 0 24px', color: 'var(--ink)' }}>
        {memory.what}
      </h2>

      {/* Why — a quiet paragraph */}
      <p style={{ fontSize: 15, color: 'var(--ink-2)', lineHeight: 1.7,
                   margin: '0 0 16px' }}>
        {memory.because}
      </p>

      {/* Consequence of NOT following it — only when we have evidence */}
      {memory.violated > 0 && (
        <p style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7,
                     margin: '0 0 32px' }}>
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
        <p style={{ fontSize: 13, color: 'var(--ink-3)', lineHeight: 1.7,
                     margin: '0 0 32px' }}>
          Reinforced{" "}
          <span style={{ color: 'var(--ink-2)' }}>{memory.reinforced} times</span>
          {" "}without a violation. Last seen {memory.lastRelevant}.
        </p>
      )}

      {/* Hairline · two stacked observation rows in the teacher style */}
      <div style={{ borderTop: 'var(--hairline)',
                     display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 0 }}>

        {/* HOW · surfaced as */}
        <ObservationRow kanji={how.glyph} title="Surfaced as"
                        value={SURFACE_LABEL[how.kind] || how.label}
                        sub={<span className="mono" style={{ fontSize: 11,
                                       color: 'var(--ink-2)' }}>{how.target}</span>}/>

        {/* WHERE · scope */}
        <ObservationRow kanji="域" title="Applies in"
                        value={scope.find(c => c.label === "project")?.value || "global"}
                        sub={
                          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
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
        <div style={{ borderTop: 'var(--hairline)', padding: '16px 0',
                       display: 'flex', flexDirection: 'column', gap: 4 }}>
          <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
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
      <div style={{ borderTop: 'var(--hairline)', paddingTop: 16,
                     display: 'flex', alignItems: 'center', gap: 12,
                     fontSize: 11, color: 'var(--ink-3)' }}>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
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
      <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 24 }}>
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
    <div style={{ padding: '16px 24px 16px 0', display: 'flex', gap: 12,
                   alignItems: 'flex-start' }}>
      <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)',
                                         lineHeight: 1, marginTop: 4 }}>{kanji}</span>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-4)',
                       textTransform: 'uppercase', marginBottom: 4 }}>{title}</div>
        <div className="display" style={{ fontSize: 17, color: 'var(--ink)',
                                            marginBottom: 4, lineHeight: 1.3 }}>
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
