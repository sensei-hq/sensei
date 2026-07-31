// Learnings — consolidated patterns + memories + recommendations across projects.
//
// Page anatomy, reading top to bottom:
//   · Slim hero (kanji 学)
//   · Counters strip — memories · patterns · corrections · pending recs · FTR lift
//   · Recommendations inbox — horizontal scroll of 3-5 actionable cards
//   · Tab bar — All / Memories / Patterns / Corrections / Lifecycle / Archive
//   · Filter sub-row — scope chips · project chips · sort
//   · Feed — cards
//   · Drawer (click a memory) — full memory anatomy
//
// Voice: the zen shared with the rest of the observatory.
//        Memory is the *why* behind rules. Patterns are signals that become memory.
//        Recommendations ask the user to act: promote · write · enrich · archive · transfer.

const { useState: lnS, useMemo: lnM } = React;

// ═══════════════════════════════════════════════════════════════════════
// Top-level page
// ═══════════════════════════════════════════════════════════════════════
function LearningsPage({ initialTab = "all" }) {
  const L = window.LEARNINGS;
  const [tab, setTab]           = lnS(initialTab);  // all | memories | patterns | corrections | lifecycle | archive
  // keep in sync if the host route changes
  React.useEffect(() => { setTab(initialTab); }, [initialTab]);
  const [scopeFilter, setScope] = lnS("all");      // all | global | project | task | module | stack
  const [projectFilter, setPrj] = lnS("all");      // all | lumen | koto | ...
  const [sort, setSort]         = lnS("priority"); // priority | strength | recency
  const [openMemory, setOpenMem]= lnS(null);       // memory id for drawer
  const [dismissed, setDismissed] = lnS(new Set()); // recommendation ids

  // ─── filter / sort the feed ──────────────────────────────
  const memories = L.memories
    .filter(m => tab === "archive" ? m.state === "archived" : m.state !== "archived")
    .filter(m => scopeFilter === "all" || m.scope.level === scopeFilter)
    .filter(m => projectFilter === "all" || m.scope.project === projectFilter)
    .sort((a, b) => {
      if (sort === "strength") return b.strength - a.strength;
      if (sort === "recency")  return a.lastRelevant.localeCompare(b.lastRelevant);
      // priority — violations first, then strength
      return (b.violated - a.violated) || (b.strength - a.strength);
    });

  const patterns = L.patterns.filter(p =>
    projectFilter === "all" || p.projects.includes(projectFilter)
  );

  const corrections = L.corrections.filter(c =>
    projectFilter === "all" || c.projects.includes(projectFilter) || c.projects.includes("all")
  );

  const lifecycle   = L.lifecycle;
  const recs        = L.recommendations.filter(r => !dismissed.has(r.id));

  const focus = L.memories.find(m => m.id === openMemory);

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Observatory · Learnings"
 >
      {/* ─── Hero ─── */}
      <LearnHero counts={L.counts} tab={tab}/>

      <div className="gap-4 pt-3 pb-8 px-8 flex-1 overflow-auto min-h-0 flex flex-col relative" >
        {/* Recommendations inbox */}
        {recs.length > 0 && tab !== "archive" && (
          <RecsInbox recs={recs} onDismiss={(id) =>
            setDismissed(s => { const n = new Set(s); n.add(id); return n; })}/>
        )}

        {/* Tabs */}
        <LearnTabs tab={tab} setTab={setTab} counts={{
          memories: L.memories.filter(m => m.state !== "archived").length,
          patterns: L.patterns.length,
          corrections: L.corrections.length,
          lifecycle: L.lifecycle.length,
          archive: L.memories.filter(m => m.state === "archived").length
        }}/>

        {/* Filters */}
        {(tab === "all" || tab === "memories") && (
          <FilterRow scope={scopeFilter} setScope={setScope}
                     projectFilter={projectFilter} setPrj={setPrj}
                     sort={sort} setSort={setSort}/>
        )}
        {(tab === "patterns" || tab === "corrections") && (
          <FilterRow scope={null} projectFilter={projectFilter} setPrj={setPrj}/>
        )}

        {/* Feed */}
        {(tab === "all" || tab === "memories") && (
          <FeedMemories memories={memories} onOpen={setOpenMem}/>
        )}
        {(tab === "all" || tab === "patterns") && (
          <FeedPatterns patterns={patterns} onOpen={setOpenMem}/>
        )}
        {(tab === "all" || tab === "corrections") && (
          <FeedCorrections corrections={corrections} onOpen={setOpenMem}/>
        )}
        {tab === "lifecycle" && (
          <FeedLifecycle events={lifecycle} onOpen={setOpenMem}/>
        )}
        {tab === "archive" && (
          <FeedMemories memories={memories} onOpen={setOpenMem} archive={true}/>
        )}
      </div>

      {/* Memory drawer */}
      {focus && <MemoryDrawer memory={focus} onClose={() => setOpenMem(null)}/>}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Hero
// ═══════════════════════════════════════════════════════════════════════
function LearnHero({ counts }) {
  return (
    <div className="gap-6 pt-6 pb-4 px-8 border-b flex items-center bg-paper" >
      <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>学</div>
      <div className="flex-1 min-w-0" >
        <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
          Observatory · Learnings
        </div>
        <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
          What sensei knows — and what to do about it.
        </h1>
        <p style={{
 fontSize: 13,
 maxWidth: 720, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
          Patterns become memory. Memory shapes how assistants think.
          Every entry below can be promoted, enriched, or retired.
        </p>
      </div>
      <div style={{ gridTemplateColumns: 'repeat(4, auto)' }} className="gap-6 pl-6 grid border-l" >
        <Stat n={counts.memories} label="memories"/>
        <Stat n={counts.patterns} label="patterns"/>
        <Stat n={counts.recs}     label="to act on" accent={true}/>
        <Stat n={`+${Math.round(counts.ftrFromMemory*100)}%`} label="FTR from memory" mono={true}/>
      </div>
    </div>
  );
}
function Stat({ n, label, accent, mono }) {
  return (
    <div className="text-center" >
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 22, fontWeight: 300, lineHeight: 1,
                     color: accent ? 'var(--accent)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>
        {n}
      </div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mt-1 uppercase text-ink-4" >{label}</div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Recommendations inbox
// ═══════════════════════════════════════════════════════════════════════
function RecsInbox({ recs, onDismiss }) {
  return (
    <section>
      <div className="mb-2 flex items-baseline justify-between" >
        <div className="gap-2 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 13 }}>薦</span>
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>
            Recommended actions
          </span>
          <span className="mono text-ink-4" style={{ fontSize: 11 }}>
            {recs.length}
          </span>
        </div>
        <span className="text-ink-4" style={{ fontSize: 11 }}>
          inferred from patterns · violations · correction history
        </span>
      </div>
      <div style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))'
 }} className="gap-2 grid" >
        {recs.slice(0, 6).map(r => (
          <RecCard key={r.id} rec={r} onDismiss={() => onDismiss(r.id)}/>
        ))}
      </div>
    </section>
  );
}

function RecCard({ rec, onDismiss }) {
  const kindMap = {
    "promote-pattern": { glyph: "昇", label: "promote", color: "var(--accent)"    },
    "create-agent":    { glyph: "作", label: "agent",   color: "var(--success)" },
    "write-skill":     { glyph: "技", label: "skill",   color: "var(--success)" },
    "archive-memory":  { glyph: "納", label: "archive", color: "var(--ink-3)" },
    "enrich-memory":   { glyph: "育", label: "enrich",  color: "var(--ink-2)"     },
    "cross-project":   { glyph: "渡", label: "transfer",color: "var(--ink-2)"     }
  };
  const k = kindMap[rec.kind] || { glyph: "?", label: "action", color: "var(--ink)" };
  const impactDot = rec.impact === "high" ? "var(--accent)" : rec.impact === "medium" ? "var(--warning)" : "var(--ink-4)";
  return (
    <div style={{
 borderRadius: 7, borderLeft: `2px solid ${k.color}` }} className="py-3 px-3 gap-2 bg-paper-2 border border-paper-edge flex flex-col min-h-0" >
      <div className="gap-2 flex items-center" >
        <span className="kanji" style={{ fontSize: 13, color: k.color }}>{k.glyph}</span>
        <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.14em',
 color: k.color }}>{k.label}</span>
        <span style={{
 width: 5, height: 5,
 background: impactDot
 }} className="ml-1 rounded-full" />
        <span className="text-ink-4" style={{ fontSize: 11 }}>{rec.impact}</span>
        <span className="flex-1" />
        <button onClick={onDismiss}
 style={{
 fontSize: 13, lineHeight: 1 }} title="dismiss" className="p-0 text-ink-4 bg-transparent border-0 cursor-pointer" >×</button>
      </div>

      <div className="text-ink font-medium" style={{ fontSize: 13, lineHeight: 1.45 }}>
        {rec.title}
      </div>
      <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
        {rec.reasoning}
      </div>

      <div style={{ marginTop: 'auto', borderTop: '1px dashed var(--edge)'
 }} className="gap-2 pt-2 flex items-center" >
        <button style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-3 bg-ink text-paper border-0 cursor-pointer" >
          {rec.action} →
        </button>
        <span className="flex-1" />
        <span className="mono text-ink-4" style={{ fontSize: 11 }}>
          {rec.targetKind} · {rec.targetName}
        </span>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Tabs
// ═══════════════════════════════════════════════════════════════════════
function LearnTabs({ tab, setTab, counts }) {
  const items = [
    { id: "all",         label: "Everything",  count: null },
    { id: "memories",    label: "Memories",    count: counts.memories },
    { id: "patterns",    label: "Patterns",    count: counts.patterns },
    { id: "corrections", label: "Corrections", count: counts.corrections },
    { id: "lifecycle",   label: "Lifecycle",   count: counts.lifecycle },
    { id: "archive",     label: "Archive",     count: counts.archive }
  ];
  return (
    <div style={{
 margin: '0 0 -4px'
 }} className="gap-0 flex border-b" >
      {items.map(it => {
        const active = tab === it.id;
        return (
          <button key={it.id} onClick={() => setTab(it.id)}
 style={{
 fontSize: 13,
 color: active ? 'var(--ink)' : 'var(--ink-3)',
 borderBottom: active ? '2px solid var(--accent)' : '2px solid transparent',
 marginBottom: -1,
 letterSpacing: '0.02em'
 }} className="gap-2 py-2 px-4 bg-transparent border-0 cursor-pointer inline-flex items-center" >
            {it.label}
            {it.count != null && (
              <span className="mono text-ink-4" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>
                {it.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Filters
// ═══════════════════════════════════════════════════════════════════════
function FilterRow({ scope, setScope, projectFilter, setPrj, sort, setSort }) {
  const scopes = ["all", "global", "project", "task", "module", "stack"];
  const projs  = ["all", ...Object.keys(window.LEARNINGS.projects)];
  return (
    <div className="gap-3 py-1 flex items-center flex-wrap" >
      {scope != null && (
        <ChipRow label="scope">
          {scopes.map(s => (
            <Chip key={s} active={scope === s} onClick={() => setScope(s)}>{s}</Chip>
          ))}
        </ChipRow>
      )}
      <ChipRow label="project">
        <ProjectFilter value={projectFilter} onChange={setPrj}
                       projects={window.LEARNINGS.projects} label={null}/>
      </ChipRow>
      {sort != null && (
        <>
          <span className="flex-1" />
          <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>sort</span>
          <select value={sort} onChange={e => setSort(e.target.value)}
 style={{
 fontSize: 11, borderRadius: 4 }} className="py-1 px-2 border border-paper-edge bg-paper text-ink-2" >
            <option value="priority">priority</option>
            <option value="strength">strength</option>
            <option value="recency">recency</option>
          </select>
        </>
      )}
    </div>
  );
}
function ChipRow({ label, children }) {
  return (
    <div className="gap-1 flex items-center" >
      <span style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mr-1 text-ink-4 uppercase" >{label}</span>
      {children}
    </div>
  );
}
function Chip({ active, onClick, children }) {
  return (
    <button onClick={onClick}
 style={{
 fontSize: 11,
 background: active ? 'var(--ink)' : 'transparent',
 color: active ? 'var(--paper)' : 'var(--ink-2)',
 border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
 borderRadius: 20,
 fontFamily: 'inherit' }} className="py-1 px-2 cursor-pointer lowercase" >
      {children}
    </button>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Feeds
// ═══════════════════════════════════════════════════════════════════════
function FeedMemories({ memories, onOpen, archive }) {
  if (memories.length === 0) {
    return <EmptyState text={archive ? "no archived memories." : "no memories match."}/>;
  }
  return (
    <section>
      {!archive && (
        <SectionHeader kanji="覚" title="Memories"
                       sub="knowledge with a reason. the why behind every rule."/>
      )}
      <div className="gap-2 flex flex-col" >
        {memories.map(m => <MemoryCard key={m.id} memory={m} onClick={() => onOpen(m.id)}/>)}
      </div>
    </section>
  );
}

function FeedPatterns({ patterns, onOpen }) {
  if (patterns.length === 0) return null;
  return (
    <section>
      <SectionHeader kanji="紋" title="Patterns"
                     sub="code signals sensei has detected. some are adopted · some are candidates · some are anti."/>
      <div className="gap-2 flex flex-col" >
        {patterns.map(p => <PatternCard key={p.id} pattern={p} onOpen={onOpen}/>)}
      </div>
    </section>
  );
}

function FeedCorrections({ corrections, onOpen }) {
  if (corrections.length === 0) return null;
  return (
    <section>
      <SectionHeader kanji="直" title="Recurring corrections"
                     sub="things you keep fixing. each one either reinforces a memory or asks for a new one."/>
      <div className="gap-2 flex flex-col" >
        {corrections.map(c => <CorrectionRow key={c.id} correction={c} onOpen={onOpen}/>)}
      </div>
    </section>
  );
}

function FeedLifecycle({ events, onOpen }) {
  return (
    <section>
      <SectionHeader kanji="巡" title="Lifecycle"
                     sub="memories learned · reinforced · challenged · superseded · archived."/>
      <div className="relative" >
        <div className="absolute" style={{ left: 92, top: 4, bottom: 4,
 width: 1, background: 'var(--edge)' }}/>
        {events.map(ev => <LifecycleRow key={ev.id} ev={ev} onOpen={onOpen}/>)}
      </div>
    </section>
  );
}

function SectionHeader({ kanji, title, sub }) {
  return (
    <div className="gap-2 mb-2 flex items-baseline" >
      <span className="kanji text-accent" style={{ fontSize: 15 }}>{kanji}</span>
      <h3 style={{
 fontSize: 13,
 letterSpacing: '0.02em'
 }} className="m-0 font-medium text-ink" >{title}</h3>
      {sub && <span className="text-ink-3" style={{ fontSize: 11 }}>· {sub}</span>}
    </div>
  );
}
function EmptyState({ text }) {
  return (
    <div style={{ fontSize: 13 }} className="p-8 text-center text-ink-4" >
      {text}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Memory card
// ═══════════════════════════════════════════════════════════════════════
function MemoryCard({ memory, onClick }) {
  const L = window.LEARNINGS;
  const stateColor = {
    "battle-tested": "var(--success)",
    "reinforced":    "var(--success)",
    "active":        "var(--ink-2)",
    "challenged":    "var(--warning)",
    "archived":      "var(--ink-4)"
  }[memory.state] || 'var(--ink-3)';

  const categoryGlyph = {
    "correctness": "正",
    "convention":  "流",
    "preference":  "好",
    "pattern":     "紋",
    "anti_pattern":"禁"
  }[memory.category] || "覚";

  return (
    <article onClick={onClick}
 style={{
 borderLeft: `2px solid ${stateColor}`,
 borderRadius: 6, transition: 'background 0.12s',
 gridTemplateColumns: '26px 1fr auto',
 gap: '0 12px' }}
 onMouseEnter={e => e.currentTarget.style.background = 'var(--paper-3)'}
 onMouseLeave={e => e.currentTarget.style.background = 'var(--paper-2)'} className="py-3 px-4 bg-paper-2 border border-paper-edge cursor-pointer grid items-start" >
      <span className="kanji mt-1 text-accent" style={{
 fontSize: 15 }}>{categoryGlyph}</span>

      <div className="min-w-0" >
        {/* What */}
        <div className="text-ink font-medium" style={{ fontSize: 13, lineHeight: 1.4 }}>
          {memory.what}
        </div>

        {/* Because */}
        <div style={{
 fontSize: 11, lineHeight: 1.55 }} className="mt-1 text-ink-2 italic" >
          because <span className="not-italic" >{memory.because}</span>
        </div>

        {/* Scope + references row */}
        <div style={{
 gap: '4px 12px'
 }} className="mt-2 flex items-center flex-wrap" >
          <ScopeBadges scope={memory.scope}/>
          {memory.references.good_example && (
            <RefLink kind="good" path={memory.references.good_example}/>
          )}
          {memory.references.bad_example && (
            <RefLink kind="bad" path={memory.references.bad_example}/>
          )}
          {memory.references.pattern && (
            <span className="mono text-ink-2" style={{ fontSize: 11 }}>
              紋 {memory.references.pattern}
            </span>
          )}
          {memory.references.evidence && (
            <span className="mono text-ink-4" style={{ fontSize: 11 }}>
              {memory.references.evidence.length} session{memory.references.evidence.length === 1 ? "" : "s"}
            </span>
          )}
        </div>
      </div>

      {/* Right rail: strength + state */}
      <div style={{ minWidth: 120
 }} className="gap-1 flex flex-col items-end" >
        <StrengthMeter value={memory.strength} violations={memory.violated}/>
        <span className="uppercase" style={{ fontSize: 11, color: stateColor, letterSpacing: '0.12em' }}>
          {memory.state}
        </span>
        <span className="text-ink-4" style={{ fontSize: 11 }}>
          seen {memory.lastRelevant}
        </span>
      </div>
    </article>
  );
}

function ScopeBadges({ scope }) {
  const L = window.LEARNINGS;
  const chips = [];
  chips.push({ k: "level", text: scope.level });
  if (scope.project) {
    const p = L.projects[scope.project];
    chips.push({ k: "project", text: p ? `${p.kanji} ${p.name}` : scope.project });
  }
  if (scope.modules) scope.modules.forEach(m => chips.push({ k: "module", text: m }));
  if (scope.taskTypes) scope.taskTypes.forEach(t => chips.push({ k: "task", text: t }));
  if (scope.stack) scope.stack.forEach(s => chips.push({ k: "stack", text: s }));
  return (
    <div className="gap-1 inline-flex flex-wrap" >
      {chips.map((c, i) => (
        <span key={i} className={(c.k === "module" ? "mono" : "") + ' py-1 px-2'}
              style={{
 fontSize: 11,
                        background: 'var(--paper)', border: 'var(--hairline)',
                        borderRadius: 10, color: 'var(--ink-3)',
                        textTransform: c.k === "level" ? 'uppercase' : 'none',
                        letterSpacing: c.k === "level" ? '0.12em' : 0
}}>
          {c.text}
        </span>
      ))}
    </div>
  );
}

function RefLink({ kind, path }) {
  const isGood = kind === "good";
  return (
    <span className="mono gap-1 inline-flex items-center" style={{
 fontSize: 11, color: isGood ? 'var(--success)' : 'var(--accent)'
 }}>
      <span>{isGood ? "✓" : "✗"}</span>
      <span className="text-ink-3" >{path}</span>
    </span>
  );
}

function StrengthMeter({ value, violations }) {
  // 5 dots, filled to `value`
  const dots = [0, 1, 2, 3, 4].map(i => i < value);
  return (
    <div className="gap-1 flex items-center" >
      {violations > 0 && (
        <span className="mono text-warning" style={{ fontSize: 11 }}>
          {violations}×broken
        </span>
      )}
      <div className="gap-1 flex" >
        {dots.map((on, i) => (
          <span className="rounded-full" key={i} style={{ width: 6, height: 6,
 background: on ? 'var(--ink)' : 'var(--edge)' }}/>
        ))}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Pattern card
// ═══════════════════════════════════════════════════════════════════════
function PatternCard({ pattern, onOpen }) {
  const kindMap = {
    "adopted":  { glyph: "✓", label: "adopted",    color: "var(--success)" },
    "emerging": { glyph: "⟡", label: "emerging",   color: "var(--ink-2)"     },
    "anti":     { glyph: "✗", label: "anti-pattern",color: "var(--accent)"   }
  };
  const k = kindMap[pattern.kind];
  const L = window.LEARNINGS;
  return (
    <article style={{
 borderLeft: `2px solid ${k.color}`,
 borderRadius: 6,
 gridTemplateColumns: '26px 1fr auto',
 gap: '0 12px' }} className="py-3 px-4 bg-paper-2 border border-paper-edge grid items-start" >
      <span className="kanji mt-1 text-accent" style={{
 fontSize: 15 }}>紋</span>
      <div className="min-w-0" >
        <div className="gap-2 flex items-baseline flex-wrap" >
          <span className="mono text-ink font-medium" style={{ fontSize: 13 }}>
            {pattern.name}
          </span>
          <span className="uppercase" style={{ fontSize: 11, color: k.color, letterSpacing: '0.12em' }}>
            {k.glyph} {k.label}
          </span>
        </div>
        <div style={{
 fontSize: 11, lineHeight: 1.55
 }} className="mt-1 text-ink-2" >
          {pattern.desc}
        </div>
        <div style={{ gap: '4px 12px',
 fontSize: 11
 }} className="mt-2 flex flex-wrap" >
          <span className="mono text-ink-3" >
            {pattern.sample}
          </span>
          <span className="text-ink-4" >
            {pattern.projects.map(p => L.projects[p]?.name || p).join(" · ")}
          </span>
          {pattern.memoryId && (
            <button onClick={() => onOpen(pattern.memoryId)}
 style={{
 fontSize: 11 }} className="p-0 text-ink-2 bg-transparent border-0 cursor-pointer" >
              → linked memory
            </button>
          )}
        </div>
      </div>
      <div className="text-right" style={{ minWidth: 110 }}>
        <div className="mono text-ink" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>
          {pattern.occurrences} places
        </div>
        <div className="mono mt-1" style={{
 fontSize: 11,
                      color: pattern.ftrDelta > 0 ? 'var(--success)' : 'var(--accent)',
                      fontFeatureSettings: '"tnum"'
}}>
          FTR {pattern.ftrDelta > 0 ? "+" : ""}{Math.round(pattern.ftrDelta*100)}%
        </div>
        <div style={{
 fontSize: 11,
 letterSpacing: '0.12em' }} className="mt-1 text-ink-4 uppercase" >
          confidence {Math.round(pattern.confidence*100)}
        </div>
      </div>
    </article>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Correction row
// ═══════════════════════════════════════════════════════════════════════
function CorrectionRow({ correction, onOpen }) {
  return (
    <article style={{
 borderRadius: 6,
 gridTemplateColumns: '26px 1fr auto auto',
 gap: '0 12px' }} className="py-2 px-4 bg-paper-2 border border-paper-edge grid items-center" >
      <span className="kanji text-accent" style={{ fontSize: 15 }}>直</span>
      <div className="min-w-0" >
        <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.4 }}>
          {correction.text}
        </div>
        <div style={{
 fontSize: 11,
 lineHeight: 1.5
 }} className="mt-1 text-ink-3" >
          {correction.suggestion}
        </div>
      </div>
      <div className="text-right" >
        <div className="mono text-ink" style={{ fontSize: 13,
 fontFeatureSettings: '"tnum"' }}>
          {correction.count}×
        </div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
          last {correction.lastSeen}
        </div>
      </div>
      {correction.memoryId && (
        <button onClick={() => onOpen(correction.memoryId)}
 style={{
 fontSize: 11,
 border: '1px solid var(--edge)', borderRadius: 4 }} className="py-1 px-2 bg-transparent text-ink-2 cursor-pointer" >
          open memory →
        </button>
      )}
    </article>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Lifecycle row
// ═══════════════════════════════════════════════════════════════════════
function LifecycleRow({ ev, onOpen }) {
  const L = window.LEARNINGS;
  const kindMap = {
    learned:    { glyph: "生", label: "learned",    color: "var(--success)" },
    reinforced: { glyph: "重", label: "reinforced", color: "var(--success)" },
    violated:   { glyph: "破", label: "violated",   color: "var(--accent)"    },
    challenged: { glyph: "疑", label: "challenged", color: "var(--warning)"  },
    superseded: { glyph: "替", label: "superseded", color: "var(--ink-2)"     },
    archived:   { glyph: "納", label: "archived",   color: "var(--ink-3)" }
  };
  const k = kindMap[ev.kind];
  const mem = L.memories.find(m => m.id === ev.memoryId);
  return (
    <div style={{ gridTemplateColumns: '80px 24px 1fr' }} className="gap-2 py-2 px-0 grid items-center" >
      <div className="text-ink-4 text-right" style={{ fontSize: 11 }}>
        {ev.when}
      </div>
      <span className="kanji text-center bg-paper rounded-full border border-paper-edge relative" style={{ fontSize: 13, color: k.color,
 width: 22, height: 22, lineHeight: '22px', zIndex: 1 }}>{k.glyph}</span>
      <div>
        <div className="gap-2 flex items-baseline flex-wrap" >
          <span className="uppercase" style={{ fontSize: 11, color: k.color, letterSpacing: '0.14em' }}>{k.label}</span>
          <button onClick={() => onOpen(ev.memoryId)}
 style={{
 fontSize: 13 }} className="p-0 text-ink bg-transparent border-0 cursor-pointer text-left" >
            {mem?.what || ev.memoryId}
          </button>
        </div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
          {ev.note}
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Memory drawer (right-side slide-in with full anatomy)
// ═══════════════════════════════════════════════════════════════════════
function MemoryDrawer({ memory, onClose }) {
  const L = window.LEARNINGS;
  const refs = memory.references || {};
  return (
    <>
      <div className="absolute" onClick={onClose}
 style={{ inset: 0, background: 'rgba(0,0,0,0.3)',
 zIndex: 10 }}/>
      <aside className="absolute bg-paper border-l flex flex-col overflow-hidden" style={{ top: 0, right: 0, bottom: 0, width: 520,
 boxShadow: '-8px 0 24px rgba(0,0,0,0.12)',
 zIndex: 11 }}>
        {/* Header */}
        <div className="gap-3 pt-4 pb-3 px-6 border-b flex items-start" >
          <div className="flex-1" >
            <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-1 text-ink-3 uppercase" >
              Memory · {memory.category.replace("_", "-")}
            </div>
            <h2 style={{
 fontSize: 15,
 lineHeight: 1.4
 }} className="m-0 font-medium text-ink" >
              {memory.what}
            </h2>
          </div>
          <button onClick={onClose}
 style={{
 fontSize: 17,
 lineHeight: 1
 }} className="p-0 text-ink-3 bg-transparent border-0 cursor-pointer" >×</button>
        </div>

        {/* Body */}
        <div className="gap-4 pt-4 pb-6 px-6 flex-1 overflow-auto flex flex-col" >
          {/* Because */}
          <DrawerBlock title="Because">
            <div className="text-ink" style={{ fontSize: 13, lineHeight: 1.6 }}>
              {memory.because}
            </div>
          </DrawerBlock>

          {/* Scope */}
          <DrawerBlock title="Scope">
            <ScopeBadges scope={memory.scope}/>
          </DrawerBlock>

          {/* Strength */}
          <DrawerBlock title="Strength">
            <div className="gap-3 flex items-center" >
              <StrengthMeter value={memory.strength} violations={0}/>
              <span className="mono text-ink-2" style={{ fontSize: 11 }}>
                {memory.strength} / 5
              </span>
              <span className="flex-1" />
              <span className="mono text-success" style={{ fontSize: 11 }}>
                +{memory.reinforced} reinforced
              </span>
              {memory.violated > 0 && (
                <span className="mono text-accent" style={{ fontSize: 11 }}>
                  −{memory.violated} violated
                </span>
              )}
            </div>
            <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
              Learned {memory.learned} · last relevant {memory.lastRelevant} · source: {memory.source}.
            </div>
          </DrawerBlock>

          {/* References */}
          <DrawerBlock title="References">
            <div className="gap-1 flex flex-col" >
              {refs.good_example && (
                <DrawerRef kind="good" text={refs.good_example}
                           label="canonical implementation — follow this"/>
              )}
              {refs.bad_example && (
                <DrawerRef kind="bad" text={refs.bad_example}
                           label="don't do it like this"/>
              )}
              {refs.pattern && (
                <DrawerRef kind="pattern" text={refs.pattern}
                           label="detected pattern"/>
              )}
              {refs.doc && (
                <DrawerRef kind="doc" text={refs.doc}
                           label="documentation"/>
              )}
              {refs.evidence && refs.evidence.length > 0 && (
                <DrawerRef kind="evidence"
                           text={refs.evidence.join(" · ")}
                           label={`${refs.evidence.length} session${refs.evidence.length === 1 ? "" : "s"} of evidence`}/>
              )}
              {refs.related && refs.related.map(rid => {
                const rm = L.memories.find(m => m.id === rid);
                return rm && (
                  <DrawerRef key={rid} kind="related" text={rm.what}
                             label="related memory"/>
                );
              })}
            </div>
          </DrawerBlock>

          {/* Actions */}
          <DrawerBlock title="Actions">
            <div style={{ gridTemplateColumns: 'repeat(2, 1fr)' }} className="gap-1 grid" >
              <ActionBtn glyph="昇" label="Promote to rule"/>
              <ActionBtn glyph="育" label="Enrich scope"/>
              <ActionBtn glyph="渡" label="Cross-project"/>
              <ActionBtn glyph="替" label="Supersede"/>
              <ActionBtn glyph="疑" label="Challenge"/>
              <ActionBtn glyph="納" label="Archive" subtle={true}/>
            </div>
          </DrawerBlock>
        </div>
      </aside>
    </>
  );
}

function DrawerBlock({ title, children }) {
  return (
    <section>
      <div style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mb-2 uppercase text-ink-3" >
        {title}
      </div>
      {children}
    </section>
  );
}

function DrawerRef({ kind, text, label }) {
  const map = {
    good:     { glyph: "✓",  color: "var(--success)" },
    bad:      { glyph: "✗",  color: "var(--accent)"    },
    pattern:  { glyph: "紋", color: "var(--ink-2)"     },
    doc:      { glyph: "文", color: "var(--ink-2)" },
    evidence: { glyph: "証", color: "var(--ink-3)" },
    related:  { glyph: "縁", color: "var(--ink-2)"     }
  };
  const k = map[kind];
  return (
    <div style={{ gridTemplateColumns: '22px 1fr', borderRadius: 5
 }} className="gap-2 py-1 px-2 grid items-baseline bg-paper-2" >
      <span className={kind === "good" || kind === "bad" ? "mono" : "kanji"}
            style={{ fontSize: 13, color: k.color, textAlign: 'center' }}>
        {k.glyph}
      </span>
      <div>
        <div className="mono text-ink" style={{ fontSize: 11,
 lineHeight: 1.5 }}>{text}</div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >{label}</div>
      </div>
    </div>
  );
}

function ActionBtn({ glyph, label, subtle }) {
  return (
    <button style={{
 fontSize: 11,
 borderRadius: 5, color: subtle ? 'var(--ink-3)' : 'var(--ink)' }} className="py-2 px-3 gap-2 bg-paper-2 border border-paper-edge cursor-pointer text-left flex items-center" >
      <span className="kanji" style={{ fontSize: 13,
                    color: subtle ? 'var(--ink-3)' : 'var(--accent)' }}>{glyph}</span>
      {label}
    </button>
  );
}

Object.assign(window, { LearningsPage });

// Harness: open with the adapter-pattern memory already in the drawer
function LearningsPageWithDrawer() {
  React.useEffect(() => {
    // Click the first memory card after mount to open the drawer.
    const t = setTimeout(() => {
      const first = document.querySelector('[data-screen-label="Observatory · Learnings"] article');
      if (first) first.click();
    }, 80);
    return () => clearTimeout(t);
  }, []);
  return <LearningsPage/>;
}
Object.assign(window, { LearningsPageWithDrawer });
