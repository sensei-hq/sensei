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
    <div className="sensei" data-screen-label="Observatory · Learnings"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      {/* ─── Hero ─── */}
      <LearnHero counts={L.counts} tab={tab}/>

      <div style={{ flex: 1, overflow: 'auto', minHeight: 0, padding: '12px 32px 32px',
                     display: 'flex', flexDirection: 'column', gap: 16, position: 'relative' }}>
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
    <div style={{ padding: '24px 32px 16px', borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 24, background: 'var(--paper)' }}>
      <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>学</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase', marginBottom: 4 }}>
          Observatory · Learnings
        </div>
        <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                          color: 'var(--ink)' }}>
          What sensei knows — and what to do about it.
        </h1>
        <p style={{ fontSize: 13, color: 'var(--ink-2)', margin: '4px 0 0',
                     maxWidth: 720, lineHeight: 1.55 }}>
          Patterns become memory. Memory shapes how assistants think.
          Every entry below can be promoted, enriched, or retired.
        </p>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, auto)', gap: 24,
                     paddingLeft: 24, borderLeft: 'var(--hairline)' }}>
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
    <div style={{ textAlign: 'center' }}>
      <div className={mono ? "mono" : ""}
           style={{ fontSize: 22, fontWeight: 300, lineHeight: 1,
                     color: accent ? 'var(--accent)' : 'var(--ink)',
                     fontFeatureSettings: '"tnum"' }}>
        {n}
      </div>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                     color: 'var(--ink-4)', marginTop: 4 }}>{label}</div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// Recommendations inbox
// ═══════════════════════════════════════════════════════════════════════
function RecsInbox({ recs, onDismiss }) {
  return (
    <section>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     marginBottom: 8 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>薦</span>
          <span style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                          textTransform: 'uppercase' }}>
            Recommended actions
          </span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            {recs.length}
          </span>
        </div>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>
          inferred from patterns · violations · correction history
        </span>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))',
                     gap: 8 }}>
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
    <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                   borderRadius: 7, borderLeft: `2px solid ${k.color}`,
                   padding: '12px 12px', display: 'flex', flexDirection: 'column',
                   gap: 8, minHeight: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span className="kanji" style={{ fontSize: 13, color: k.color }}>{k.glyph}</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', textTransform: 'uppercase',
                        color: k.color }}>{k.label}</span>
        <span style={{ width: 5, height: 5, borderRadius: '50%',
                        background: impactDot, marginLeft: 4 }}/>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>{rec.impact}</span>
        <span style={{ flex: 1 }}/>
        <button onClick={onDismiss}
                style={{ fontSize: 13, color: 'var(--ink-4)', lineHeight: 1,
                          padding: 0, background: 'transparent', border: 'none',
                          cursor: 'pointer' }} title="dismiss">×</button>
      </div>

      <div style={{ fontSize: 13, lineHeight: 1.45, color: 'var(--ink)',
                     fontWeight: 500 }}>
        {rec.title}
      </div>
      <div style={{ fontSize: 11, lineHeight: 1.55, color: 'var(--ink-2)' }}>
        {rec.reasoning}
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 'auto',
                     paddingTop: 8, borderTop: '1px dashed var(--edge)' }}>
        <button style={{ padding: '4px 12px', fontSize: 11,
                          background: 'var(--ink)', color: 'var(--paper)',
                          border: 'none', borderRadius: 4, cursor: 'pointer' }}>
          {rec.action} →
        </button>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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
    <div style={{ display: 'flex', gap: 0, borderBottom: 'var(--hairline)',
                   margin: '0 0 -4px' }}>
      {items.map(it => {
        const active = tab === it.id;
        return (
          <button key={it.id} onClick={() => setTab(it.id)}
                  style={{ padding: '8px 16px 8px', fontSize: 13,
                            background: 'transparent', border: 'none', cursor: 'pointer',
                            color: active ? 'var(--ink)' : 'var(--ink-3)',
                            borderBottom: active ? '2px solid var(--accent)' : '2px solid transparent',
                            marginBottom: -1,
                            display: 'inline-flex', alignItems: 'center', gap: 8,
                            letterSpacing: '0.02em' }}>
            {it.label}
            {it.count != null && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
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
    <div style={{ display: 'flex', gap: 12, alignItems: 'center',
                   padding: '4px 0 4px', flexWrap: 'wrap' }}>
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
          <span style={{ flex: 1 }}/>
          <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                          textTransform: 'uppercase' }}>sort</span>
          <select value={sort} onChange={e => setSort(e.target.value)}
                  style={{ fontSize: 11, border: 'var(--hairline)', background: 'var(--paper)',
                            padding: '4px 8px', borderRadius: 4, color: 'var(--ink-2)' }}>
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
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <span style={{ fontSize: 11, color: 'var(--ink-4)', letterSpacing: '0.14em',
                      textTransform: 'uppercase', marginRight: 4 }}>{label}</span>
      {children}
    </div>
  );
}
function Chip({ active, onClick, children }) {
  return (
    <button onClick={onClick}
            style={{ padding: '4px 8px', fontSize: 11,
                      background: active ? 'var(--ink)' : 'transparent',
                      color: active ? 'var(--paper)' : 'var(--ink-2)',
                      border: active ? '1px solid var(--ink)' : '1px solid var(--edge)',
                      borderRadius: 20, cursor: 'pointer',
                      fontFamily: 'inherit', textTransform: 'lowercase' }}>
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
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
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
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
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
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
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
      <div style={{ position: 'relative' }}>
        <div style={{ position: 'absolute', left: 92, top: 4, bottom: 4,
                       width: 1, background: 'var(--edge)' }}/>
        {events.map(ev => <LifecycleRow key={ev.id} ev={ev} onOpen={onOpen}/>)}
      </div>
    </section>
  );
}

function SectionHeader({ kanji, title, sub }) {
  return (
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 8 }}>
      <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>{kanji}</span>
      <h3 style={{ fontSize: 13, fontWeight: 500, margin: 0, color: 'var(--ink)',
                    letterSpacing: '0.02em' }}>{title}</h3>
      {sub && <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>· {sub}</span>}
    </div>
  );
}
function EmptyState({ text }) {
  return (
    <div style={{ padding: 32, textAlign: 'center', fontSize: 13, color: 'var(--ink-4)' }}>
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
             style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${stateColor}`,
                       borderRadius: 6, padding: '12px 16px',
                       cursor: 'pointer', transition: 'background 0.12s',
                       display: 'grid',
                       gridTemplateColumns: '26px 1fr auto',
                       gap: '0 14px', alignItems: 'start' }}
             onMouseEnter={e => e.currentTarget.style.background = 'var(--paper-3)'}
             onMouseLeave={e => e.currentTarget.style.background = 'var(--paper-2)'}>
      <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)',
                    marginTop: 4 }}>{categoryGlyph}</span>

      <div style={{ minWidth: 0 }}>
        {/* What */}
        <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.4,
                       fontWeight: 500 }}>
          {memory.what}
        </div>

        {/* Because */}
        <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55,
                       marginTop: 4, fontStyle: 'italic' }}>
          because <span style={{ fontStyle: 'normal' }}>{memory.because}</span>
        </div>

        {/* Scope + references row */}
        <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap',
                       gap: '6px 14px', marginTop: 8 }}>
          <ScopeBadges scope={memory.scope}/>
          {memory.references.good_example && (
            <RefLink kind="good" path={memory.references.good_example}/>
          )}
          {memory.references.bad_example && (
            <RefLink kind="bad" path={memory.references.bad_example}/>
          )}
          {memory.references.pattern && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
              紋 {memory.references.pattern}
            </span>
          )}
          {memory.references.evidence && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
              {memory.references.evidence.length} session{memory.references.evidence.length === 1 ? "" : "s"}
            </span>
          )}
        </div>
      </div>

      {/* Right rail: strength + state */}
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end',
                     gap: 4, minWidth: 120 }}>
        <StrengthMeter value={memory.strength} violations={memory.violated}/>
        <span style={{ fontSize: 11, color: stateColor, letterSpacing: '0.12em',
                        textTransform: 'uppercase' }}>
          {memory.state}
        </span>
        <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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
    <div style={{ display: 'inline-flex', gap: 4, flexWrap: 'wrap' }}>
      {chips.map((c, i) => (
        <span key={i} className={c.k === "module" ? "mono" : ""}
              style={{ fontSize: 11, padding: '4px 8px',
                        background: 'var(--paper)', border: 'var(--hairline)',
                        borderRadius: 10, color: 'var(--ink-3)',
                        textTransform: c.k === "level" ? 'uppercase' : 'none',
                        letterSpacing: c.k === "level" ? '0.12em' : 0 }}>
          {c.text}
        </span>
      ))}
    </div>
  );
}

function RefLink({ kind, path }) {
  const isGood = kind === "good";
  return (
    <span className="mono" style={{ display: 'inline-flex', alignItems: 'center', gap: 4,
                   fontSize: 11, color: isGood ? 'var(--success)' : 'var(--accent)' }}>
      <span>{isGood ? "✓" : "✗"}</span>
      <span style={{ color: 'var(--ink-3)' }}>{path}</span>
    </span>
  );
}

function StrengthMeter({ value, violations }) {
  // 5 dots, filled to `value`
  const dots = [0, 1, 2, 3, 4].map(i => i < value);
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      {violations > 0 && (
        <span className="mono" style={{ fontSize: 11, color: 'var(--warning)' }}>
          {violations}×broken
        </span>
      )}
      <div style={{ display: 'flex', gap: 4 }}>
        {dots.map((on, i) => (
          <span key={i} style={{ width: 6, height: 6, borderRadius: '50%',
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
    <article style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${k.color}`,
                       borderRadius: 6, padding: '12px 16px',
                       display: 'grid',
                       gridTemplateColumns: '26px 1fr auto',
                       gap: '0 14px', alignItems: 'start' }}>
      <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)',
                    marginTop: 4 }}>紋</span>
      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, flexWrap: 'wrap' }}>
          <span className="mono" style={{ fontSize: 13, color: 'var(--ink)', fontWeight: 500 }}>
            {pattern.name}
          </span>
          <span style={{ fontSize: 11, color: k.color, letterSpacing: '0.12em',
                          textTransform: 'uppercase' }}>
            {k.glyph} {k.label}
          </span>
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55,
                       marginTop: 4 }}>
          {pattern.desc}
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: '4px 14px', marginTop: 8,
                       fontSize: 11 }}>
          <span className="mono" style={{ color: 'var(--ink-3)' }}>
            {pattern.sample}
          </span>
          <span style={{ color: 'var(--ink-4)' }}>
            {pattern.projects.map(p => L.projects[p]?.name || p).join(" · ")}
          </span>
          {pattern.memoryId && (
            <button onClick={() => onOpen(pattern.memoryId)}
                    style={{ fontSize: 11, color: 'var(--ink-2)', background: 'transparent',
                              border: 'none', cursor: 'pointer', padding: 0 }}>
              → linked memory
            </button>
          )}
        </div>
      </div>
      <div style={{ textAlign: 'right', minWidth: 110 }}>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink)',
                      fontFeatureSettings: '"tnum"' }}>
          {pattern.occurrences} places
        </div>
        <div className="mono" style={{ fontSize: 11, marginTop: 4,
                      color: pattern.ftrDelta > 0 ? 'var(--success)' : 'var(--accent)',
                      fontFeatureSettings: '"tnum"' }}>
          FTR {pattern.ftrDelta > 0 ? "+" : ""}{Math.round(pattern.ftrDelta*100)}%
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4,
                       letterSpacing: '0.12em', textTransform: 'uppercase' }}>
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
    <article style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderRadius: 6, padding: '8px 16px',
                       display: 'grid',
                       gridTemplateColumns: '26px 1fr auto auto',
                       gap: '0 14px', alignItems: 'center' }}>
      <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>直</span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.4 }}>
          {correction.text}
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4,
                       lineHeight: 1.5 }}>
          {correction.suggestion}
        </div>
      </div>
      <div style={{ textAlign: 'right' }}>
        <div className="mono" style={{ fontSize: 13, color: 'var(--ink)',
                      fontFeatureSettings: '"tnum"' }}>
          {correction.count}×
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-4)', marginTop: 4 }}>
          last {correction.lastSeen}
        </div>
      </div>
      {correction.memoryId && (
        <button onClick={() => onOpen(correction.memoryId)}
                style={{ padding: '4px 8px', fontSize: 11, background: 'transparent',
                          border: '1px solid var(--edge)', borderRadius: 4,
                          color: 'var(--ink-2)', cursor: 'pointer' }}>
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
    <div style={{ display: 'grid', gridTemplateColumns: '80px 24px 1fr',
                   gap: 8, alignItems: 'center', padding: '8px 0' }}>
      <div style={{ fontSize: 11, color: 'var(--ink-4)', textAlign: 'right' }}>
        {ev.when}
      </div>
      <span className="kanji" style={{ fontSize: 13, color: k.color,
                    textAlign: 'center', background: 'var(--paper)',
                    width: 22, height: 22, lineHeight: '22px',
                    borderRadius: '50%', border: 'var(--hairline)',
                    position: 'relative', zIndex: 1 }}>{k.glyph}</span>
      <div>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, flexWrap: 'wrap' }}>
          <span style={{ fontSize: 11, color: k.color, letterSpacing: '0.14em',
                          textTransform: 'uppercase' }}>{k.label}</span>
          <button onClick={() => onOpen(ev.memoryId)}
                  style={{ fontSize: 13, color: 'var(--ink)', background: 'transparent',
                            border: 'none', cursor: 'pointer', padding: 0, textAlign: 'left' }}>
            {mem?.what || ev.memoryId}
          </button>
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
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
      <div onClick={onClose}
           style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0.3)',
                     zIndex: 10 }}/>
      <aside style={{ position: 'absolute', top: 0, right: 0, bottom: 0, width: 520,
                       background: 'var(--paper)', borderLeft: 'var(--hairline)',
                       boxShadow: '-8px 0 24px rgba(0,0,0,0.12)',
                       zIndex: 11, display: 'flex', flexDirection: 'column',
                       overflow: 'hidden' }}>
        {/* Header */}
        <div style={{ padding: '16px 24px 12px', borderBottom: 'var(--hairline)',
                       display: 'flex', alignItems: 'flex-start', gap: 12 }}>
          <div style={{ flex: 1 }}>
            <div style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                           textTransform: 'uppercase', marginBottom: 4 }}>
              Memory · {memory.category.replace("_", "-")}
            </div>
            <h2 style={{ fontSize: 15, fontWeight: 500, margin: 0, color: 'var(--ink)',
                          lineHeight: 1.4 }}>
              {memory.what}
            </h2>
          </div>
          <button onClick={onClose}
                  style={{ fontSize: 17, color: 'var(--ink-3)', background: 'transparent',
                            border: 'none', cursor: 'pointer', padding: 0,
                            lineHeight: 1 }}>×</button>
        </div>

        {/* Body */}
        <div style={{ flex: 1, overflow: 'auto', padding: '16px 24px 24px',
                       display: 'flex', flexDirection: 'column', gap: 16 }}>
          {/* Because */}
          <DrawerBlock title="Because">
            <div style={{ fontSize: 13, color: 'var(--ink)', lineHeight: 1.6 }}>
              {memory.because}
            </div>
          </DrawerBlock>

          {/* Scope */}
          <DrawerBlock title="Scope">
            <ScopeBadges scope={memory.scope}/>
          </DrawerBlock>

          {/* Strength */}
          <DrawerBlock title="Strength">
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <StrengthMeter value={memory.strength} violations={0}/>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-2)' }}>
                {memory.strength} / 5
              </span>
              <span style={{ flex: 1 }}/>
              <span className="mono" style={{ fontSize: 11, color: 'var(--success)' }}>
                +{memory.reinforced} reinforced
              </span>
              {memory.violated > 0 && (
                <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>
                  −{memory.violated} violated
                </span>
              )}
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>
              Learned {memory.learned} · last relevant {memory.lastRelevant} · source: {memory.source}.
            </div>
          </DrawerBlock>

          {/* References */}
          <DrawerBlock title="References">
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
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
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 4 }}>
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
      <div style={{ fontSize: 11, letterSpacing: '0.16em', textTransform: 'uppercase',
                     color: 'var(--ink-3)', marginBottom: 8 }}>
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
    <div style={{ display: 'grid', gridTemplateColumns: '22px 1fr', gap: 8,
                   alignItems: 'baseline', padding: '4px 8px',
                   background: 'var(--paper-2)', borderRadius: 5 }}>
      <span className={kind === "good" || kind === "bad" ? "mono" : "kanji"}
            style={{ fontSize: 13, color: k.color, textAlign: 'center' }}>
        {k.glyph}
      </span>
      <div>
        <div className="mono" style={{ fontSize: 11, color: 'var(--ink)',
                      lineHeight: 1.5 }}>{text}</div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)', marginTop: 4 }}>{label}</div>
      </div>
    </div>
  );
}

function ActionBtn({ glyph, label, subtle }) {
  return (
    <button style={{ padding: '8px 12px', fontSize: 11,
                      background: 'var(--paper-2)', border: 'var(--hairline)',
                      borderRadius: 5, color: subtle ? 'var(--ink-3)' : 'var(--ink)',
                      cursor: 'pointer', textAlign: 'left',
                      display: 'flex', alignItems: 'center', gap: 8 }}>
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
