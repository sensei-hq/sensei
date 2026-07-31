// Libraries area — three variations.
// Sensei treats libraries as first-class: detected, imported, external services.
// Each lib can be explained / searched / queried via the sensei MCP.

const { useState: lS } = React;

// Shared building blocks ──────────────────────────────────────
function LibIcon({ letter, tone = 'var(--accent)', size = 28 }) {
  return (
    <div className="bg-paper-3 border border-paper-edge flex items-center justify-center font-semibold shrink-0" style={{
 width: size, height: size, borderRadius: 6,
 fontSize: size * 0.45, color: tone,
 fontFamily: 'var(--font-display)' }}>{letter}</div>
  );
}

function DocChip({ status }) {
  const map = {
    indexed: { label: "docs indexed", tone: 'var(--success)',  bg: 'var(--success-soft)' },
    partial: { label: "partial",      tone: 'var(--warning)', bg: 'var(--warning-soft)' },
    schema:  { label: "schema only",  tone: 'var(--ink-2)', bg: 'var(--paper-3)'   },
    none:    { label: "no docs",      tone: 'var(--ink-3)', bg: 'var(--paper-3)'   }
  };
  const m = map[status] || map.none;
  return (
    <span className="mono py-1 px-2" style={{
 fontSize: 11, borderRadius: 3,
                background: m.bg, color: m.tone
}}>{m.label}</span>
  );
}

// ── Detail panel (shared by all variations) ──────────────────
function LibraryDetail({ libId, compact = false }) {
  const d = window.LIBRARIES_DATA.details[libId] || window.LIBRARIES_DATA.details.axum;
  const [example, setExample] = lS(0);
  const ex = d.mcpExamples[example];
  return (
    <div className="gap-4 flex flex-col" >
      <div className="gap-3 flex items-start" >
        <LibIcon letter={d.name.charAt(0)} size={40}/>
        <div className="flex-1" >
          <div className="display font-normal" style={{ fontSize: 22, letterSpacing: '-0.01em' }}>
            {d.name}
          </div>
          <div style={{ fontSize: 13, lineHeight: 1.55 }} className="mt-1 text-ink-2" >
            {d.tagline}
          </div>
          <div style={{
 fontSize: 11 }} className="mono gap-2 mt-2 flex items-center text-ink-3">
            <span>v{d.version}</span><span>·</span>
            <span>{d.lang}</span><span>·</span>
            <DocChip status={d.docs}/><span>·</span>
            <span>{d.source}</span>
          </div>
        </div>
      </div>

      <div style={{ borderRadius: 6,
 fontSize: 13, lineHeight: 1.55
 }} className="py-3 px-3 bg-paper-2 border border-paper-edge text-ink-2" >
        {d.summary}
      </div>

      {/* Usage grid */}
      <div style={{ gridTemplateColumns: '1fr 1.2fr' }} className="gap-4 grid" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Top symbols</div>
          <div className="gap-1 flex flex-col" >
            {d.usage.topSymbols.map(s => (
              <div key={s.symbol} style={{
 gridTemplateColumns: '1fr auto' }} className="gap-2 py-1 px-1 grid items-baseline border-b" >
                <span className="mono text-ink" style={{ fontSize: 11 }}>{s.symbol}</span>
                <span className="mono text-ink-3" style={{ fontSize: 11 }}>{s.n}×</span>
              </div>
            ))}
          </div>
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Used at</div>
          <div className="gap-1 flex flex-col" >
            {d.usage.places.map((p, i) => (
              <div key={i} className="py-2 px-1 border-b" >
                <div className="mono text-ink" style={{ fontSize: 11 }}>
                  {p.file}<span className="text-ink-4" >:{p.line}</span>
                </div>
                <div className="mono mt-1 text-ink-3 whitespace-nowrap overflow-hidden text-ellipsis" style={{
 fontSize: 11 }}>{p.snippet}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Rules attached */}
      {d.rules && d.rules.length > 0 && (
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >Rules attached</div>
          <div className="gap-2 flex flex-col" >
            {d.rules.map((r, i) => (
              <div key={i} style={{
 borderRadius: 5,
 borderLeft: '2px solid var(--accent)' }} className="py-2 px-3 bg-paper-2 border border-paper-edge" >
                <div className="text-ink" style={{ fontSize: 13 }}>"{r.rule}"</div>
                <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
                  {r.source}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* MCP example interactions — the key insight */}
      <div>
        <div className="mb-2 flex items-baseline justify-between" >
          <div>
            <div className="gap-2 flex items-baseline" >
              <span className="kanji text-accent" style={{ fontSize: 15 }}>具</span>
              <div className="display font-normal" style={{ fontSize: 15 }}>
                What sensei can do with this library
              </div>
            </div>
            <div style={{ fontSize: 11 }} className="mt-1 ml-6 text-ink-3" >
              Example MCP interactions · each tool callable by an assistant with sensei attached.
            </div>
          </div>
        </div>

        <div className="gap-1 mb-3 flex flex-wrap" >
          {d.mcpExamples.map((e, i) => {
            const on = example === i;
            return (
              <button key={i} onClick={() => setExample(i)}
                      style={{
 fontSize: 11,
                                borderRadius: 4,
                                background: on ? 'var(--ink)' : 'var(--paper-2)',
                                color: on ? 'var(--paper)' : 'var(--ink-2)',
                                border: on ? 'none' : 'var(--hairline)',
                                fontFamily: 'var(--font-mono)'
}} className="py-1 px-2" >
                {e.tool}
              </button>
            );
          })}
        </div>

        <div style={{ gridTemplateColumns: '1fr 1fr' }} className="gap-3 grid" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
              Intent
            </div>
            <div style={{
 fontSize: 13, lineHeight: 1.5
 }} className="mb-3 text-ink italic" >
              "{ex.intent}"
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
              Request
            </div>
            <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.5,
 borderRadius: 5,
 whiteSpace: 'pre-wrap' }} className="py-2 px-3 m-0 bg-paper-2 border border-paper-edge text-ink-2 overflow-auto" >
              {ex.request}
            </pre>
          </div>
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
              Response
            </div>
            <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
 borderLeft: '2px solid var(--accent)',
 borderRadius: 5,
 whiteSpace: 'pre-wrap',
 minHeight: 180
 }} className="py-3 px-3 m-0 bg-paper-2 border border-paper-edge text-ink overflow-auto" >
              {ex.response}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}

// Library list item ─────────────────────────────────────────
function LibRow({ item, onClick, active }) {
  return (
    <button onClick={onClick} style={{
 gridTemplateColumns: 'auto 1fr auto auto auto', borderRadius: 6,
 background: active ? 'var(--paper-2)' : 'transparent' }} className="gap-3 py-3 px-3 grid items-center text-left border-b" >
      <LibIcon letter={item.icon}
               tone={item.service ? 'var(--success)' : item.internal ? 'var(--warning)' : 'var(--accent)'}
               size={32}/>
      <div className="min-w-0" >
        <div className="gap-2 flex items-baseline" >
          <span className="text-ink" style={{ fontSize: 13 }}>{item.name}</span>
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>v{item.version}</span>
        </div>
        <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
          {item.source}
        </div>
      </div>
      <DocChip status={item.docs}/>
      <span className="mono text-ink-3 text-right" style={{ fontSize: 11,
 minWidth: 60 }}>
        {item.usage}× calls
      </span>
      <span className="mono text-ink-4 text-right" style={{ fontSize: 11,
 minWidth: 70 }}>
        {item.lastIndexed || "—"}
      </span>
    </button>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation A — Unified list + side detail panel
// A library is a library. No split between detected and imported.
// Tiny chip on the row hints at origin, but doesn't segment.
// ═════════════════════════════════════════════════════════════
function LibrariesVariantA({ embedded = false, state = "ready" } = {}) {
  if (state !== "ready") return <window.ScreenState state={state} kanji="庫"
    emptyTitle="No libraries watched yet"
    emptyHint="Sensei wraps each dependency your projects import as its own instrument. Run a session that uses one and it'll show up here with its docs-health."
    errorHint="Couldn't load your libraries. Try again." onRetry={() => {}} />;
  const D = window.LIBRARIES_DATA;
  const all = D.groups.flatMap(g => g.items.map(i => ({ ...i, kind: g.id })));
  const [kind, setKind] = lS("all");      // all | code | service
  const [lang, setLang] = lS("all");      // all | rust | ts | docs | mcp
  const [query, setQuery] = lS("");
  const [focus, setFocus] = lS(D.focus);

  // Group items as code vs service, since that's a meaningful distinction
  // (code libraries have types/symbols; services have schemas/APIs).
  const kinds = [
    { id: "all",     label: "All",      kanji: "全", count: all.length },
    { id: "code",    label: "Code",     kanji: "書",
      count: all.filter(x => !x.service).length },
    { id: "service", label: "Services", kanji: "繋",
      count: all.filter(x => x.service).length }
  ];
  const langs = ["all", ...Array.from(new Set(all.map(x => x.lang)))];

  const ql = query.toLowerCase().trim();
  const filtered = all.filter(x => {
    if (kind === "code"    && x.service) return false;
    if (kind === "service" && !x.service) return false;
    if (lang !== "all" && x.lang !== lang) return false;
    if (ql && !(x.name.toLowerCase().includes(ql) ||
                x.source.toLowerCase().includes(ql))) return false;
    return true;
  }).sort((a, b) => (b.usage || 0) - (a.usage || 0));

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Libraries · Unified"
 >
      {!embedded && <TauriChrome title="Sensei  先生  ·  libraries"/>}

      <ScreenHeader
        kanji="庫"
        eyebrow="Libraries"
        title="Tools the student uses. Kept close."
        sub="Sensei watches imports and flags docs that drift. Ask it anything about how you actually use each library — through any assistant that speaks MCP."
        right={
          <button style={{ fontSize: 13,
 borderRadius: 5 }} className="py-2 px-3 bg-ink text-paper" >+ add library</button>
        }/>

      {/* Filter row */}
      <div className="py-3 px-12 gap-4 border-b flex items-center flex-wrap" >
        <div className="gap-1 flex" >
          {kinds.map(k => {
            const on = kind === k.id;
            return (
              <button key={k.id} onClick={() => setKind(k.id)}
 style={{
 fontSize: 11,
 borderRadius: 4,
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : 'var(--ink-2)'
 }} className="py-1 px-3 gap-2 inline-flex items-center" >
                <span className="kanji" style={{ fontSize: 11 }}>{k.kanji}</span>
                {k.label}
                <span className="mono" style={{ fontSize: 11,
                              color: on ? 'var(--paper)' : 'var(--ink-4)', opacity: 0.85 }}>
                  {k.count}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ width: 1, height: 18, background: 'var(--edge)' }}/>
        <div className="gap-2 flex items-center" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.12em' }}>Lang</span>
          <div className="gap-1 flex" >
            {langs.map(l => {
              const on = lang === l;
              return (
                <button key={l} onClick={() => setLang(l)}
                        style={{
 fontSize: 11,
                                  borderRadius: 4,
                                  background: on ? 'var(--paper-3)' : 'transparent',
                                  color: on ? 'var(--ink)' : 'var(--ink-3)'
}} className="py-1 px-2" >
                  {l}
                </button>
              );
            })}
          </div>
        </div>
        <span className="flex-1" />
        <div style={{ borderRadius: 5, minWidth: 220
 }} className="gap-2 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
          <input className="border-0 bg-transparent flex-1 text-ink" value={query} onChange={e => setQuery(e.target.value)}
 placeholder="search libraries…"
 style={{ outline: 'none',
 fontSize: 13 }}/>
          {query && (
            <button className="text-ink-4" onClick={() => setQuery("")}
 style={{ fontSize: 11 }}>×</button>
          )}
        </div>
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {filtered.length} of {all.length}
        </span>
      </div>

      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '1fr 1fr' }}>
        <div className="pt-4 pb-8 px-12 overflow-auto border-r" >
          {filtered.length === 0 && (
            <div style={{
 fontSize: 13 }} className="py-8 px-0 text-center text-ink-3" >
              No libraries match.
            </div>
          )}
          <div className="flex flex-col" >
            {filtered.map(x => (
              <LibRow key={x.id} item={x}
                      active={focus === x.id}
                      onClick={() => setFocus(x.id)}/>
            ))}
          </div>
        </div>
        <div className="py-6 px-8 overflow-auto bg-paper-2" >
          <LibraryDetail libId={focus}/>
        </div>
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// Variation B — Full workspace · tabs + detailed single-pane
// ═════════════════════════════════════════════════════════════
function LibrariesVariantB() {
  const D = window.LIBRARIES_DATA;
  const [tab, setTab] = lS("detected");
  const [focus, setFocus] = lS(D.focus);
  const group = D.groups.find(g => g.id === tab);

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Libraries · Workspace"
 >
      <TauriChrome title="Sensei  先生  ·  libraries · workspace"/>

      <div className="pt-4 pb-0 px-12 border-b" >
        <div className="gap-3 mb-3 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 22 }}>庫</span>
          <h1 className="display m-0 font-normal" style={{ fontSize: 22 }}>
            Libraries
          </h1>
          <span className="text-ink-3" style={{ fontSize: 11 }}>
            detected + imported + connected
          </span>
          <span className="flex-1" />
          <button style={{
 fontSize: 11, border: 'var(--ink-line)',
 borderRadius: 5
 }} className="py-2 px-3 text-ink-2" >+ import URL</button>
          <button style={{
 fontSize: 11,
 borderRadius: 5
 }} className="py-2 px-3 bg-ink text-paper" >+ register library</button>
        </div>
        <div className="gap-1 flex" >
          {D.groups.map(g => {
            const on = tab === g.id;
            return (
              <button key={g.id} onClick={() => setTab(g.id)}
 style={{
 fontSize: 13,
 borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-3)', marginBottom: -1 }} className="py-2 px-3 gap-2 inline-flex items-center" >
                <span className="kanji" style={{ fontSize: 13 }}>{g.kanji}</span>
                {g.label}
                <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                  {g.items.length}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '320px 1fr' }}>
        <div className="py-3 px-3 overflow-auto border-r bg-paper-2" >
          <div style={{ fontSize: 11 }} className="pt-1 pb-2 px-2 text-ink-3" >
            {group.sub}
          </div>
          <div className="flex flex-col" >
            {group.items.map(x => (
              <button key={x.id} onClick={() => setFocus(x.id)}
 style={{ gridTemplateColumns: 'auto 1fr auto', borderRadius: 5,
 background: focus === x.id ? 'var(--paper)' : 'transparent' }} className="gap-2 py-2 px-3 grid items-center text-left border-b" >
                <LibIcon letter={x.icon}
                         tone={x.service ? 'var(--success)' : x.internal ? 'var(--warning)' : 'var(--accent)'}
                         size={26}/>
                <div className="min-w-0" >
                  <div className="text-ink" style={{ fontSize: 13 }}>{x.name}</div>
                  <div className="mono text-ink-3" style={{ fontSize: 11 }}>
                    v{x.version} · {x.usage}× calls
                  </div>
                </div>
                <DocChip status={x.docs}/>
              </button>
            ))}
          </div>
        </div>

        <div className="py-6 px-12 overflow-auto" >
          <LibraryDetail libId={focus}/>
        </div>
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// MCP Playground — its own top-level screen.
// Sensei exposes tools over MCP. The playground lists tools
// and lets users try each one. Tools declare their inputs
// (project, library, session, text, enums) — the form adapts.
// ═════════════════════════════════════════════════════════════
function MCPPlayground({ activeTab = "playground", onTab = () => {} } = {}) {
  const T = window.MCP_TOOLS;
  // Scope switcher — default is sensei, can switch to any installed MCP
  // sourced from the global registry (if present) or fall back to sensei only.
  const registry = (window.SENSEI_SETUP && window.SENSEI_SETUP.mcpRegistry &&
                    window.SENSEI_SETUP.mcpRegistry.available) || [];
  const mcpSources = [
    { id: "sensei", name: "Sensei MCP", publisher: "先生", tools: T.tools.length, kanji: "具", verified: true, active: true },
    ...registry.filter(m => m.installed || m.recommended).map(m => ({
      id: m.id, name: m.name, publisher: m.publisher, tools: m.tools, kanji: m.kanji,
      verified: m.verified, active: false
    }))
  ];
  const [scope, setScope] = lS("sensei");
  const scopeMcp = mcpSources.find(m => m.id === scope) || mcpSources[0];

  const [category, setCategory] = lS("all");
  const [focusId, setFocusId] = lS(T.tools[0].id);
  const [query, setQuery] = lS("");

  // Other-MCP scopes render a stub state (sensei can inspect tools but
  // the detail panel shows a notice that it's a third-party MCP).
  const isSensei = scope === "sensei";

  const ql = query.toLowerCase().trim();
  const filtered = T.tools.filter(t =>
    (category === "all" || t.category === category) &&
    (!ql || t.name.toLowerCase().includes(ql) || t.summary.toLowerCase().includes(ql))
  );
  const focus = T.tools.find(t => t.id === focusId) || T.tools[0];
  const cat = T.categories.find(c => c.id === focus.category);

  return (
    <MCPShell activeTab={activeTab} onTab={onTab}
              kanji={scopeMcp.kanji}
              title="Playground"
              tagline={isSensei
                ? "Sensei's tools, in your hands."
                : scopeMcp.name + " · in your hands."}
              sub={isSensei
                ? "Sensei exposes these tools over MCP — any assistant with sensei attached can call them. Try any tool here; some take a project, some take a library."
                : "Installed for this project. Inspect tools and try them the same way you'd try sensei's own."}
              chip={
                <span className="mono py-1 px-2 text-ink-3 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3
 }}>
                  {scopeMcp.tools} tools
                </span>
              }>

      {/* MCP scope selector — horizontal pill row */}
      <div className="py-2 px-12 gap-3 border-b flex items-center bg-paper-2" >
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>
          MCP
        </span>
        <div className="gap-1 flex flex-wrap" >
          {mcpSources.map(m => {
            const on = scope === m.id;
            return (
              <button key={m.id} onClick={() => setScope(m.id)}
 style={{ fontSize: 11, borderRadius: 4,
 background: on ? 'var(--paper)' : 'transparent',
 border: on ? '1px solid var(--ink-4)' : '1px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-2)'
 }} className="gap-1 py-1 px-2 inline-flex items-center" >
                <span className="kanji" style={{ fontSize: 11,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{m.kanji}</span>
                <span>{m.name}</span>
                <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                  {m.tools}
                </span>
              </button>
            );
          })}
        </div>
        <span className="flex-1" />
        <span className="mono text-ink-3" style={{ fontSize: 11 }}>
          {mcpSources.length} installed
        </span>
      </div>

      {!isSensei && (
        <div style={{
 fontSize: 13, lineHeight: 1.55
 }} className="py-3 px-12 bg-paper-2 border-b text-ink-2" >
          <span className="kanji mr-2 text-warning" style={{ fontSize: 13 }}>告</span>
          Third-party MCP. Sensei lists these tools from the server's manifest — you can inspect each,
          but sensei doesn't wrap or index them.
        </div>
      )}

      {/* Filter row */}
      <div className="py-3 px-12 gap-3 border-b flex items-center" >
        <div className="gap-1 flex" >
          <CatChip on={category === "all"} onClick={() => setCategory("all")}
                   kanji="全" label="All" count={T.tools.length}/>
          {T.categories.map(c => (
            <CatChip key={c.id} on={category === c.id}
                     onClick={() => setCategory(c.id)}
                     kanji={c.kanji} label={c.label}
                     count={T.tools.filter(t => t.category === c.id).length}/>
          ))}
        </div>
        <span className="flex-1" />
        <div style={{ borderRadius: 5, minWidth: 240
 }} className="gap-2 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
          <input className="border-0 bg-transparent flex-1 text-ink" value={query} onChange={e => setQuery(e.target.value)}
 placeholder="search tools…"
 style={{ outline: 'none',
 fontSize: 13 }}/>
          {query && (
            <button className="text-ink-4" onClick={() => setQuery("")}
 style={{ fontSize: 11 }}>×</button>
          )}
        </div>
      </div>

      {/* Two-pane */}
      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '340px 1fr' }}>
        <aside className="overflow-auto border-r bg-paper-2" >
          {filtered.length === 0 && (
            <div style={{
 fontSize: 13 }} className="py-6 px-4 text-ink-3 text-center" >
              No tools match.
            </div>
          )}
          {T.categories.map(c => {
            const items = filtered.filter(t => t.category === c.id);
            if (items.length === 0) return null;
            return (
              <div key={c.id} className="pt-4 pb-1 px-3" >
                <div className="gap-2 pt-0 pb-2 px-2 flex items-baseline" >
                  <span className="kanji text-accent" style={{ fontSize: 11 }}>
                    {c.kanji}
                  </span>
                  <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>
                    {c.label}
                  </span>
                  <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                    {items.length}
                  </span>
                </div>
                {items.map(t => (
                  <ToolRow key={t.id} tool={t}
                           active={focusId === t.id}
                           onClick={() => setFocusId(t.id)}/>
                ))}
              </div>
            );
          })}
        </aside>

        <main className="pt-6 pb-8 px-12 overflow-auto" >
          <ToolDetail tool={focus} cat={cat}/>
        </main>
      </div>
    </MCPShell>
  );
}

function CatChip({ on, onClick, kanji, label, count }) {
  return (
    <button onClick={onClick}
 style={{
 fontSize: 11,
 borderRadius: 4,
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : 'var(--ink-2)'
 }} className="py-1 px-3 gap-2 inline-flex items-center" >
      <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
      {label}
      <span className="mono" style={{ fontSize: 11,
                    color: on ? 'var(--paper)' : 'var(--ink-4)', opacity: 0.85 }}>
        {count}
      </span>
    </button>
  );
}

function ToolRow({ tool, active, onClick }) {
  // Show the short method name (last segment) as the primary label
  const short = tool.name.split('.').slice(-1)[0];
  return (
    <button onClick={onClick}
 style={{ borderRadius: 5,
 background: active ? 'var(--paper)' : 'transparent',
 border: active ? '1px solid var(--edge)' : '1px solid transparent'
 }} className="py-2 px-3 mb-1 block w-full text-left" >
      <div className="mono" style={{ fontSize: 11,
                    color: active ? 'var(--ink)' : 'var(--ink-2)' }}>
        {tool.name}
      </div>
      <div style={{
 fontSize: 11,
 lineHeight: 1.45, display: '-webkit-box',
 WebkitLineClamp: 2, WebkitBoxOrient: 'vertical'
 }} className="mt-1 text-ink-3 overflow-hidden" >
        {tool.summary}
      </div>
    </button>
  );
}

// ── Tool detail + run-able form ──────────────────────────────
function ToolDetail({ tool, cat }) {
  // seed form with example inputs so "Run" shows a real response immediately
  const seed = React.useMemo(() => {
    const s = {};
    tool.inputs.forEach(i => {
      s[i.key] = (tool.example && tool.example[i.key] != null)
                 ? tool.example[i.key]
                 : (i.default != null ? i.default : "");
    });
    return s;
  }, [tool.id]);
  const [values, setValues] = lS(seed);
  const [hasRun, setHasRun] = lS(false);
  React.useEffect(() => { setValues(seed); setHasRun(false); }, [tool.id]);

  const setVal = (k, v) => setValues(prev => ({ ...prev, [k]: v }));

  // Build the request JSON from the form
  const request = React.useMemo(() => {
    const obj = {};
    tool.inputs.forEach(i => {
      if (values[i.key] !== "" && values[i.key] != null) obj[i.key] = values[i.key];
    });
    return obj;
  }, [values, tool.id]);

  const missing = tool.inputs.filter(i => i.required &&
                    (values[i.key] === "" || values[i.key] == null));

  return (
    <div className="gap-6 flex flex-col" >
      {/* Tool header */}
      <div>
        <div className="gap-2 mb-1 flex items-baseline" >
          <span className="kanji text-accent" style={{ fontSize: 13 }}>
            {cat.kanji}
          </span>
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>
            {cat.label}
          </span>
        </div>
        <div className="mono mb-2 text-ink" style={{ fontSize: 17 }}>
          {tool.name}
        </div>
        <div className="text-ink-2" style={{ fontSize: 13, lineHeight: 1.55,
 maxWidth: 680 }}>
          {tool.summary}
        </div>
      </div>

      {/* Form */}
      <div style={{ borderRadius: 8
 }} className="py-4 px-4 bg-paper-2 border border-paper-edge" >
        <div className="gap-2 mb-3 flex items-baseline" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>Inputs</span>
          <span className="mono text-ink-4" style={{ fontSize: 11 }}>
            · {tool.inputs.length}
          </span>
        </div>
        <div style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))'
 }} className="gap-3 mb-4 grid" >
          {tool.inputs.map(input => (
            <InputField key={input.key} input={input}
                        value={values[input.key]}
                        onChange={v => setVal(input.key, v)}/>
          ))}
        </div>
        <div className="gap-2 flex items-center" >
          <button onClick={() => setHasRun(true)}
                  disabled={missing.length > 0}
                  style={{
 fontSize: 13,
                            background: missing.length > 0 ? 'var(--paper-3)' : 'var(--ink)',
                            color: missing.length > 0 ? 'var(--ink-3)' : 'var(--paper)',
                            borderRadius: 5,
                            cursor: missing.length > 0 ? 'not-allowed' : 'pointer'
}} className="py-2 px-4" >
            Run tool →
          </button>
          {missing.length > 0 && (
            <span className="text-warning" style={{ fontSize: 11 }}>
              required: {missing.map(m => m.label || m.key).join(", ")}
            </span>
          )}
          <span className="flex-1" />
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            callable by any MCP-attached assistant
          </span>
        </div>
      </div>

      {/* Request + Response */}
      <div style={{ gridTemplateColumns: '1fr 1.3fr' }} className="gap-4 grid" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
            Request
          </div>
          <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
 borderRadius: 6,
 whiteSpace: 'pre-wrap'
 }} className="py-3 px-3 m-0 bg-paper-2 border border-paper-edge text-ink-2" >
{JSON.stringify({ tool: tool.name, args: request }, null, 2)}
          </pre>
        </div>
        <div>
          <div className="mb-2 flex items-baseline justify-between" >
            <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
              {hasRun ? "Response" : "Example response"}
            </span>
            {!hasRun && (
              <span className="mono text-ink-4" style={{ fontSize: 11 }}>
                with example inputs · click Run for live
              </span>
            )}
          </div>
          <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.6,
 borderLeft: '2px solid var(--accent)',
 borderRadius: 6,
 whiteSpace: 'pre-wrap', minHeight: 200
 }} className="py-3 px-4 m-0 bg-paper border border-paper-edge text-ink" >
            {tool.example.response}
          </pre>
        </div>
      </div>
    </div>
  );
}

// Input renderers — kind drives the widget.
function InputField({ input, value, onChange }) {
  const label = (
    <div className="gap-1 mb-1 flex items-baseline" >
      <span className="text-ink-2 uppercase" style={{ fontSize: 11, letterSpacing: '0.1em' }}>
        {input.label || input.key}
      </span>
      {input.required
        ? <span className="mono text-accent" style={{ fontSize: 11 }}>required</span>
        : <span className="mono text-ink-4" style={{ fontSize: 11 }}>optional</span>}
      <span className="flex-1" />
      <span className="mono text-ink-4" style={{ fontSize: 11 }}>
        {input.kind}
      </span>
    </div>
  );

  if (input.kind === "project") {
    const projects = window.PROJECTS_INDEX.projects;
    return (
      <div>
        {label}
        <select value={value || ""} onChange={e => onChange(e.target.value)}
                style={selectStyle} className="gap-1" >
          {!input.required && <option value="">— none —</option>}
          {projects.map(p => (
            <option key={p.id} value={p.id}>{p.name}</option>
          ))}
        </select>
      </div>
    );
  }

  if (input.kind === "library") {
    const libs = window.LIBRARIES_DATA.groups.flatMap(g => g.items);
    return (
      <div>
        {label}
        <select value={value || ""} onChange={e => onChange(e.target.value)}
                style={selectStyle}>
          {!input.required && <option value="">— none —</option>}
          {libs.map(l => (
            <option key={l.id} value={l.id}>{l.name}</option>
          ))}
        </select>
      </div>
    );
  }

  if (input.kind === "session") {
    const sessions = window.SENSEI_DATA.sessions;
    return (
      <div>
        {label}
        <select value={value || ""} onChange={e => onChange(e.target.value)}
                style={selectStyle}>
          {!input.required && <option value="">— none —</option>}
          {sessions.map(s => (
            <option key={s.id} value={s.id}>{s.id} · {s.title.slice(0,40)}</option>
          ))}
        </select>
      </div>
    );
  }

  if (input.kind === "since" || input.kind === "enum") {
    const opts = input.options || [];
    return (
      <div>
        {label}
        <div className="flex" >
          {opts.map(o => {
            const on = value === o;
            return (
              <button key={o} onClick={() => onChange(o)}
                      style={{
 fontSize: 11,
                                borderRadius: 4,
                                background: on ? 'var(--ink)' : 'var(--paper-3)',
                                color: on ? 'var(--paper)' : 'var(--ink-2)'
}} className="py-1 px-2" >
                {o}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  if (input.kind === "number") {
    return (
      <div>
        {label}
        <input type="number" value={value || ""}
               onChange={e => onChange(Number(e.target.value) || 0)}
               style={inputStyle}/>
      </div>
    );
  }

  // default: text
  return (
    <div>
      {label}
      <input value={value || ""}
             placeholder={input.placeholder || ""}
             onChange={e => onChange(e.target.value)}
             style={inputStyle}/>
    </div>
  );
}

const inputStyle = {
  width: '100%', padding: '8px 8px', fontSize: 13,
  border: 'var(--hairline)', borderRadius: 5,
  background: 'var(--paper)', color: 'var(--ink)',
  fontFamily: 'var(--font-mono)', outline: 'none'
};
const selectStyle = {
  ...inputStyle, fontFamily: 'Inter, sans-serif', fontSize: 13
};


Object.assign(window, { LibrariesVariantA, LibrariesVariantB, MCPPlayground });
