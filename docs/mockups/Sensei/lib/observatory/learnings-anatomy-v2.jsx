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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Learnings · Anatomy v2"
 >
      <L2Hero kanji="覚" title="Every memory has the same anatomy."
              sub="What it is · why it matters · how it's surfaced · where it applies."
              right={<HealthChart memories={all}/>}/>

      {/* ── Toolbar · the single place to scope + search ───────────── */}
      <div className="py-3 px-8 gap-4 border-b flex items-center flex-wrap" >
        <ProjectFilter value={project} onChange={setProj}
                        projects={L.projects}/>
        <span style={{ width: 1, height: 18, background: 'var(--edge)' }}/>
        <div style={{ borderRadius: 16,
 minWidth: 200
 }} className="gap-1 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <svg className="shrink-0" width="11" height="11" viewBox="0 0 16 16" fill="none"
 style={{ opacity: 0.55 }}>
            <circle cx="7" cy="7" r="5" stroke="currentColor" strokeWidth="1.3"/>
            <line x1="11" y1="11" x2="14" y2="14"
                  stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
          </svg>
          <input value={query} onChange={e => setQuery(e.target.value)}
 placeholder="search memories…"
 style={{ fontSize: 11, fontFamily: 'inherit',
 outline: 'none' }} className="p-0 flex-1 bg-transparent border-0 text-ink min-w-0" />
          {query && (
            <button onClick={() => setQuery("")}
 style={{ fontSize: 13, lineHeight: 1,
 fontFamily: 'inherit'
 }} className="p-0 bg-transparent border-0 text-ink-4 cursor-pointer" >×</button>
          )}
        </div>
        <span className="flex-1" />
        <span className="text-ink-4" style={{ fontSize: 11 }}>
          {filtered.length} of {all.length} memories
        </span>
      </div>

      <div className="flex-1 grid min-h-0" style={{
 gridTemplateColumns: '244px 1fr' }}>
        {/* ── Calm rail · just the list ────────────────────────── */}
        <aside className="py-1 px-0 border-r overflow-auto" >
          {filtered.length === 0 && (
            <div style={{
 fontSize: 11 }} className="py-6 px-4 text-ink-4 text-center" >
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
 background: open ? 'var(--paper-2)' : 'transparent',
 borderLeft: open ? '2px solid var(--accent)'
 : '2px solid transparent' }} className="gap-2 py-2 px-3 w-full text-left cursor-pointer flex items-start" >
                <span className="kanji mt-1 shrink-0"
 style={{
 fontSize: 15, lineHeight: 1.3,
 color: open ? 'var(--accent)' : 'var(--ink-3)' }}>
                  {how.glyph}
                </span>
                <span className="flex-1 min-w-0 overflow-hidden" style={{ fontSize: 13,
 color: open ? 'var(--ink)' : 'var(--ink-2)',
 lineHeight: 1.4,
 display: '-webkit-box', WebkitLineClamp: 2,
 WebkitBoxOrient: 'vertical' }}>
                  {m.what}
                </span>
              </button>
            );
          })}
        </aside>

        {/* ── Stage ────────────────────────────────────── */}
        <main className="py-8 px-12 overflow-auto" >
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
 maxWidth: 720 }} className="mt-1 mb-0 mx-auto flex flex-col" >
      {/* Eyebrow + surface tag, on one row */}
      <div style={{
 fontSize: 11, letterSpacing: '0.12em' }} className="gap-3 mb-3 flex items-center text-ink-3 uppercase" >
        <span>{memory.category.replace("_", "-")}</span>
        <span className="rounded-full bg-ink-4" style={{ width: 3, height: 3 }}/>
        <span>{memory.state}</span>
        <span className="flex-1" />
        <span style={{ letterSpacing: '0.12em'
 }} className="gap-2 inline-flex items-center text-accent" >
          <span className="kanji" style={{ fontSize: 13, lineHeight: 1 }}>{how.glyph}</span>
          {SURFACE_LABEL[how.kind] || how.label}
        </span>
      </div>

      {/* The memory message — display-scale, like the welcome page */}
      <h2 className="display mt-0 mb-6 font-light text-ink" style={{
 fontSize: 40, lineHeight: 1.15,
 letterSpacing: '-0.015em' }}>
        {memory.what}
      </h2>

      {/* Why — a quiet paragraph */}
      <p style={{
 fontSize: 15, lineHeight: 1.7
 }} className="mt-0 mb-4 text-ink-2" >
        {memory.because}
      </p>

      {/* Consequence of NOT following it — only when we have evidence */}
      {memory.violated > 0 && (
        <p style={{
 fontSize: 13, lineHeight: 1.7
 }} className="mt-0 mb-8 text-ink-3" >
          When this slipped, sensei saw{" "}
          <span className="text-accent" >
            {memory.violated} correction{memory.violated === 1 ? "" : "s"}
          </span>{" "}
          across recent sessions
          {memory.references.bad_example && (
            <> — most often in <span className="mono text-ink-2" >
              {memory.references.bad_example}
            </span></>
          )}.
        </p>
      )}
      {memory.violated === 0 && (
        <p style={{
 fontSize: 13, lineHeight: 1.7
 }} className="mt-0 mb-8 text-ink-3" >
          Reinforced{" "}
          <span className="text-ink-2" >{memory.reinforced} times</span>
          {" "}without a violation. Last seen {memory.lastRelevant}.
        </p>
      )}

      {/* Hairline · two stacked observation rows in the teacher style */}
      <div style={{ gridTemplateColumns: '1fr 1fr'
 }} className="gap-0 border-t grid" >

        {/* HOW · surfaced as */}
        <ObservationRow kanji={how.glyph} title="Surfaced as"
                        value={SURFACE_LABEL[how.kind] || how.label}
                        sub={<span className="mono text-ink-2" style={{ fontSize: 11 }}>{how.target}</span>}/>

        {/* WHERE · scope */}
        <ObservationRow kanji="域" title="Applies in"
                        value={scope.find(c => c.label === "project")?.value || "global"}
                        sub={
                          <div className="gap-1 flex flex-wrap" >
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
        <div className="gap-1 py-4 px-0 border-t flex flex-col" >
          <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-4 uppercase" >
            In the codebase
          </div>
          {memory.references.good_example && (
            <div className="mono text-success" style={{ fontSize: 13 }}>
              ✓ {memory.references.good_example}
            </div>
          )}
          {memory.references.bad_example && (
            <div className="mono text-accent" style={{ fontSize: 13 }}>
              ✗ {memory.references.bad_example}
            </div>
          )}
        </div>
      )}

      {/* Strength · meta */}
      <div style={{
 fontSize: 11 }} className="gap-3 pt-4 border-t flex items-center text-ink-3" >
        <span className="gap-2 inline-flex items-center" >
          <StrengthBar value={memory.strength}/>
          <span className="mono text-ink-2" >
            strength {memory.strength}/5
          </span>
        </span>
        <Sep/>
        <span>learned {memory.learned}</span>
        <Sep/>
        <span>last seen {memory.lastRelevant}</span>
      </div>

      {/* Actions */}
      <div className="gap-2 mt-6 flex items-center" >
        <FlatBtn glyph="昇" label="Promote to rule"/>
        <FlatBtn glyph="育" label="Enrich"/>
        <FlatBtn glyph="渡" label="Apply elsewhere"/>
        <span className="flex-1" />
        <button className="bg-transparent border border-paper-edge text-ink-3 cursor-pointer" title="More"
 style={{ width: 30, height: 28, fontSize: 13, borderRadius: 5,
 letterSpacing: 1 }}>···</button>
      </div>
    </div>
  );
}

// One observation row in the "teacher" voice — kanji, eyebrow, value, sub.
function ObservationRow({ kanji, title, value, sub }) {
  return (
    <div className="gap-3 py-4 pl-0 pr-6 flex items-start" >
      <span className="kanji mt-1 text-accent" style={{
 fontSize: 22,
 lineHeight: 1
 }}>{kanji}</span>
      <div className="flex-1 min-w-0" >
        <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-4 uppercase" >{title}</div>
        <div className="display mb-1 text-ink" style={{
 fontSize: 17, lineHeight: 1.3
 }}>
          {value}
        </div>
        {sub && <div className="text-ink-3" style={{ fontSize: 11,
 lineHeight: 1.5 }}>{sub}</div>}
      </div>
    </div>
  );
}

function Sep() {
  return <span className="rounded-full bg-ink-4 inline-block" style={{ width: 3, height: 3 }}/>;
}

window.LearningsAnatomyV2 = LearningsAnatomyV2;
