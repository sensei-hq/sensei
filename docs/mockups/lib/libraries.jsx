// Libraries area — three variations.
// Sensei treats libraries as first-class: detected, imported, external services.
// Each lib can be explained / searched / queried via the sensei MCP.

const { useState: lS } = React;

// Shared building blocks ──────────────────────────────────────
function LibIcon({ letter, tone = 'var(--accent)', size = 28 }) {
  return (
    <div style={{
      width: size, height: size, borderRadius: 6,
      background: 'var(--paper-3)', border: 'var(--hairline)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontSize: size * 0.45, fontWeight: 600, color: tone,
      fontFamily: 'var(--font-display)', flexShrink: 0
    }}>{letter}</div>
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
    <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-4" >
      <div style={{ display: 'flex', alignItems: 'flex-start' }} className="gap-3" >
        <LibIcon letter={d.name.charAt(0)} size={40}/>
        <div style={{ flex: 1 }}>
          <div className="display" style={{ fontSize: 22, fontWeight: 400, letterSpacing: '-0.01em' }}>
            {d.name}
          </div>
          <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55 }} className="mt-1" >
            {d.tagline}
          </div>
          <div style={{
 display: 'flex', alignItems: 'center',
                         fontSize: 11, color: 'var(--ink-3)'
}} className="mono gap-2 mt-2">
            <span>v{d.version}</span><span>·</span>
            <span>{d.lang}</span><span>·</span>
            <DocChip status={d.docs}/><span>·</span>
            <span>{d.source}</span>
          </div>
        </div>
      </div>

      <div style={{
 background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 6,
                     fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55
}} className="py-3 px-3" >
        {d.summary}
      </div>

      {/* Usage grid */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.2fr' }} className="gap-4" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-2" >Top symbols</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {d.usage.topSymbols.map(s => (
              <div key={s.symbol} style={{
 display: 'grid',
                            gridTemplateColumns: '1fr auto', alignItems: 'baseline', borderBottom: 'var(--hairline)'
}} className="gap-2 py-1 px-1" >
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink)' }}>{s.symbol}</span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{s.n}×</span>
              </div>
            ))}
          </div>
        </div>
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-2" >Used at</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
            {d.usage.places.map((p, i) => (
              <div key={i} style={{ borderBottom: 'var(--hairline)' }} className="py-2 px-1" >
                <div className="mono" style={{ fontSize: 11, color: 'var(--ink)' }}>
                  {p.file}<span style={{ color: 'var(--ink-4)' }}>:{p.line}</span>
                </div>
                <div className="mono mt-1" style={{
 fontSize: 11, color: 'var(--ink-3)', whiteSpace: 'nowrap', overflow: 'hidden',
                              textOverflow: 'ellipsis'
}}>{p.snippet}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Rules attached */}
      {d.rules && d.rules.length > 0 && (
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-2" >Rules attached</div>
          <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
            {d.rules.map((r, i) => (
              <div key={i} style={{
 borderRadius: 5,
                            background: 'var(--paper-2)',
                            borderLeft: '2px solid var(--accent)', border: 'var(--hairline)'
}} className="py-2 px-3" >
                <div style={{ fontSize: 13, color: 'var(--ink)' }}>"{r.rule}"</div>
                <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                  {r.source}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* MCP example interactions — the key insight */}
      <div>
        <div style={{
 display: 'flex', alignItems: 'baseline', justifyContent: 'space-between'
}} className="mb-2" >
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
              <span className="kanji" style={{ fontSize: 15, color: 'var(--accent)' }}>具</span>
              <div className="display" style={{ fontSize: 15, fontWeight: 400 }}>
                What sensei can do with this library
              </div>
            </div>
            <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1 ml-5" >
              Example MCP interactions · each tool callable by an assistant with sensei attached.
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1 mb-3" >
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

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr' }} className="gap-3" >
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-1" >
              Intent
            </div>
            <div style={{
 fontSize: 13, color: 'var(--ink)',
                           fontStyle: 'italic', lineHeight: 1.5
}} className="mb-3" >
              "{ex.intent}"
            </div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-1" >
              Request
            </div>
            <pre style={{
                           fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.5,
                           background: 'var(--paper-2)', border: 'var(--hairline)',
                           borderRadius: 5, color: 'var(--ink-2)',
                           whiteSpace: 'pre-wrap', overflow: 'auto'
}} className="py-2 px-3 m-0" >
              {ex.request}
            </pre>
          </div>
          <div>
            <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                           textTransform: 'uppercase'
}} className="mb-1" >
              Response
            </div>
            <pre style={{
                           fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
                           background: 'var(--paper-2)',
                           borderLeft: '2px solid var(--accent)',
                           border: 'var(--hairline)',
                           borderRadius: 5, color: 'var(--ink)',
                           whiteSpace: 'pre-wrap', overflow: 'auto',
                           minHeight: 180
}} className="py-3 px-3 m-0" >
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
      display: 'grid',
      gridTemplateColumns: 'auto 1fr auto auto auto', alignItems: 'center', borderRadius: 6,
      textAlign: 'left',
      background: active ? 'var(--paper-2)' : 'transparent',
      borderBottom: 'var(--hairline)'
}} className="gap-3 py-3 px-3" >
      <LibIcon letter={item.icon}
               tone={item.service ? 'var(--success)' : item.internal ? 'var(--warning)' : 'var(--accent)'}
               size={32}/>
      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
          <span style={{ fontSize: 13, color: 'var(--ink)' }}>{item.name}</span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>v{item.version}</span>
        </div>
        <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
          {item.source}
        </div>
      </div>
      <DocChip status={item.docs}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                    minWidth: 60, textAlign: 'right' }}>
        {item.usage}× calls
      </span>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
                    minWidth: 70, textAlign: 'right' }}>
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
function LibrariesVariantA() {
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
    <div className="sensei" data-screen-label="Libraries · Unified"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生  ·  libraries"/>

      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'flex-end'
}} className="gap-4 pt-5 pb-4 px-7" >
        <div className="kanji" style={{ fontSize: 56, color: 'var(--accent)', lineHeight: 1 }}>庫</div>
        <div style={{ flex: 1 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >Libraries</div>
          <h1 className="display m-0" style={{ fontSize: 28, fontWeight: 400 }}>
            Tools the student uses. Kept close.
          </h1>
          <p style={{ fontSize: 13, color: 'var(--ink-2)', maxWidth: 620 }} className="mt-1 mb-0" >
            Sensei watches imports and flags docs that drift. Ask it anything about how
            you actually use each library — through any assistant that speaks MCP.
          </p>
        </div>
        <button style={{
 fontSize: 13,
                          background: 'var(--ink)', color: 'var(--paper)',
                          borderRadius: 5
}} className="py-2 px-3" >+ add library</button>
      </div>

      {/* Filter row */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center', flexWrap: 'wrap'
}} className="py-3 px-7 gap-4" >
        <div style={{ display: 'flex' }} className="gap-1" >
          {kinds.map(k => {
            const on = kind === k.id;
            return (
              <button key={k.id} onClick={() => setKind(k.id)}
                      style={{
 fontSize: 11,
                                borderRadius: 4, display: 'inline-flex', alignItems: 'center',
                                background: on ? 'var(--ink)' : 'transparent',
                                color: on ? 'var(--paper)' : 'var(--ink-2)'
}} className="py-1 px-3 gap-2" >
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
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
          <span style={{ fontSize: 11, color: 'var(--ink-3)', letterSpacing: '0.12em',
                         textTransform: 'uppercase' }}>Lang</span>
          <div style={{ display: 'flex' }} className="gap-1" >
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
        <span style={{ flex: 1 }}/>
        <div style={{
 display: 'flex', alignItems: 'center',
                       background: 'var(--paper-2)', borderRadius: 5, border: 'var(--hairline)', minWidth: 220
}} className="gap-2 py-1 px-2" >
          <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search libraries…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 13, flex: 1, color: 'var(--ink)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--ink-4)' }}>×</button>
          )}
        </div>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {filtered.length} of {all.length}
        </span>
      </div>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <div style={{
 overflow: 'auto',
                       borderRight: 'var(--hairline)'
}} className="pt-4 pb-6 px-7" >
          {filtered.length === 0 && (
            <div style={{
 textAlign: 'center',
                           fontSize: 13, color: 'var(--ink-3)'
}} className="py-6 px-0" >
              No libraries match.
            </div>
          )}
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {filtered.map(x => (
              <LibRow key={x.id} item={x}
                      active={focus === x.id}
                      onClick={() => setFocus(x.id)}/>
            ))}
          </div>
        </div>
        <div style={{ overflow: 'auto', background: 'var(--paper-2)' }} className="py-5 px-6" >
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
    <div className="sensei" data-screen-label="Libraries · Workspace"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title="Sensei  先生  ·  libraries · workspace"/>

      <div style={{ borderBottom: 'var(--hairline)' }} className="pt-4 pb-0 px-7" >
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-3 mb-3" >
          <span className="kanji" style={{ fontSize: 22, color: 'var(--accent)' }}>庫</span>
          <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>
            Libraries
          </h1>
          <span style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            detected + imported + connected
          </span>
          <span style={{ flex: 1 }}/>
          <button style={{
 fontSize: 11,
                            color: 'var(--ink-2)', border: 'var(--ink-line)',
                            borderRadius: 5
}} className="py-2 px-3" >+ import URL</button>
          <button style={{
 fontSize: 11,
                            background: 'var(--ink)', color: 'var(--paper)',
                            borderRadius: 5
}} className="py-2 px-3" >+ register library</button>
        </div>
        <div style={{ display: 'flex' }} className="gap-1" >
          {D.groups.map(g => {
            const on = tab === g.id;
            return (
              <button key={g.id} onClick={() => setTab(g.id)}
                      style={{
 fontSize: 13,
                                borderBottom: on ? '2px solid var(--accent)' : '2px solid transparent',
                                color: on ? 'var(--ink)' : 'var(--ink-3)', marginBottom: -1,
                                display: 'inline-flex', alignItems: 'center'
}} className="py-2 px-3 gap-2" >
                <span className="kanji" style={{ fontSize: 13 }}>{g.kanji}</span>
                {g.label}
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                  {g.items.length}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '320px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <div style={{
 overflow: 'auto',
                       borderRight: 'var(--hairline)', background: 'var(--paper-2)'
}} className="py-3 px-3" >
          <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="pt-1 pb-2 px-2" >
            {group.sub}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {group.items.map(x => (
              <button key={x.id} onClick={() => setFocus(x.id)}
                      style={{
                        display: 'grid', gridTemplateColumns: 'auto 1fr auto',
                        alignItems: 'center', borderRadius: 5,
                        textAlign: 'left',
                        background: focus === x.id ? 'var(--paper)' : 'transparent',
                        borderBottom: 'var(--hairline)'
}} className="gap-2 py-2 px-3" >
                <LibIcon letter={x.icon}
                         tone={x.service ? 'var(--success)' : x.internal ? 'var(--warning)' : 'var(--accent)'}
                         size={26}/>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 13, color: 'var(--ink)' }}>{x.name}</div>
                  <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
                    v{x.version} · {x.usage}× calls
                  </div>
                </div>
                <DocChip status={x.docs}/>
              </button>
            ))}
          </div>
        </div>

        <div style={{ overflow: 'auto' }} className="py-5 px-7" >
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
                <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-3)', border: 'var(--hairline)', borderRadius: 3
}}>
                  {scopeMcp.tools} tools
                </span>
              }>

      {/* MCP scope selector — horizontal pill row */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center',
                     background: 'var(--paper-2)'
}} className="py-2 px-7 gap-3" >
        <span style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>
          MCP
        </span>
        <div style={{ display: 'flex', flexWrap: 'wrap' }} className="gap-1" >
          {mcpSources.map(m => {
            const on = scope === m.id;
            return (
              <button key={m.id} onClick={() => setScope(m.id)}
                      style={{
 display: 'inline-flex', alignItems: 'center', fontSize: 11, borderRadius: 4,
                                background: on ? 'var(--paper)' : 'transparent',
                                border: on ? '1px solid var(--ink-4)' : '1px solid transparent',
                                color: on ? 'var(--ink)' : 'var(--ink-2)'
}} className="gap-1 py-1 px-2" >
                <span className="kanji" style={{ fontSize: 11,
                              color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{m.kanji}</span>
                <span>{m.name}</span>
                <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                  {m.tools}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {mcpSources.length} installed
        </span>
      </div>

      {!isSensei && (
        <div style={{
                       background: 'var(--paper-2)',
                       borderBottom: 'var(--hairline)',
                       fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55
}} className="py-3 px-7" >
          <span className="kanji mr-2" style={{ color: 'var(--warning)', fontSize: 13 }}>告</span>
          Third-party MCP. Sensei lists these tools from the server's manifest — you can inspect each,
          but sensei doesn't wrap or index them.
        </div>
      )}

      {/* Filter row */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'center'
}} className="py-3 px-7 gap-3" >
        <div style={{ display: 'flex' }} className="gap-1" >
          <CatChip on={category === "all"} onClick={() => setCategory("all")}
                   kanji="全" label="All" count={T.tools.length}/>
          {T.categories.map(c => (
            <CatChip key={c.id} on={category === c.id}
                     onClick={() => setCategory(c.id)}
                     kanji={c.kanji} label={c.label}
                     count={T.tools.filter(t => t.category === c.id).length}/>
          ))}
        </div>
        <span style={{ flex: 1 }}/>
        <div style={{
 display: 'flex', alignItems: 'center',
                       background: 'var(--paper-2)', borderRadius: 5, border: 'var(--hairline)', minWidth: 240
}} className="gap-2 py-1 px-2" >
          <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search tools…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 13, flex: 1, color: 'var(--ink)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--ink-4)' }}>×</button>
          )}
        </div>
      </div>

      {/* Two-pane */}
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '340px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <aside style={{ overflow: 'auto', borderRight: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
          {filtered.length === 0 && (
            <div style={{
 fontSize: 13, color: 'var(--ink-3)',
                           textAlign: 'center'
}} className="py-5 px-4" >
              No tools match.
            </div>
          )}
          {T.categories.map(c => {
            const items = filtered.filter(t => t.category === c.id);
            if (items.length === 0) return null;
            return (
              <div key={c.id} className="pt-4 pb-1 px-3" >
                <div style={{
                               display: 'flex', alignItems: 'baseline'
}} className="gap-2 pt-0 pb-2 px-2" >
                  <span className="kanji" style={{ fontSize: 11, color: 'var(--accent)' }}>
                    {c.kanji}
                  </span>
                  <span style={{ fontSize: 11, letterSpacing: '0.16em',
                                  color: 'var(--ink-3)', textTransform: 'uppercase' }}>
                    {c.label}
                  </span>
                  <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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

        <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-7" >
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
                      borderRadius: 4, display: 'inline-flex', alignItems: 'center',
                      background: on ? 'var(--ink)' : 'transparent',
                      color: on ? 'var(--paper)' : 'var(--ink-2)'
}} className="py-1 px-3 gap-2" >
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
            style={{
              display: 'block', width: '100%', textAlign: 'left', borderRadius: 5,
              background: active ? 'var(--paper)' : 'transparent',
              border: active ? '1px solid var(--edge)' : '1px solid transparent'
}} className="py-2 px-3 mb-1" >
      <div className="mono" style={{ fontSize: 11,
                    color: active ? 'var(--ink)' : 'var(--ink-2)' }}>
        {tool.name}
      </div>
      <div style={{
 fontSize: 11, color: 'var(--ink-3)',
                     lineHeight: 1.45,
                     overflow: 'hidden', display: '-webkit-box',
                     WebkitLineClamp: 2, WebkitBoxOrient: 'vertical'
}} className="mt-1" >
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
    <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-5" >
      {/* Tool header */}
      <div>
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
          <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>
            {cat.kanji}
          </span>
          <span style={{ fontSize: 11, letterSpacing: '0.16em', color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>
            {cat.label}
          </span>
        </div>
        <div className="mono mb-2" style={{ fontSize: 17, color: 'var(--ink)' }}>
          {tool.name}
        </div>
        <div style={{ fontSize: 13, color: 'var(--ink-2)', lineHeight: 1.55,
                       maxWidth: 680 }}>
          {tool.summary}
        </div>
      </div>

      {/* Form */}
      <div style={{
 background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 8
}} className="py-4 px-4" >
        <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-3" >
          <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase' }}>Inputs</span>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            · {tool.inputs.length}
          </span>
        </div>
        <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))'
}} className="gap-3 mb-4" >
          {tool.inputs.map(input => (
            <InputField key={input.key} input={input}
                        value={values[input.key]}
                        onChange={v => setVal(input.key, v)}/>
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center' }} className="gap-2" >
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
            <span style={{ fontSize: 11, color: 'var(--warning)' }}>
              required: {missing.map(m => m.label || m.key).join(", ")}
            </span>
          )}
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            callable by any MCP-attached assistant
          </span>
        </div>
      </div>

      {/* Request + Response */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.3fr' }} className="gap-4" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-2" >
            Request
          </div>
          <pre style={{
                         fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
                         background: 'var(--paper-2)', border: 'var(--hairline)',
                         borderRadius: 6, color: 'var(--ink-2)',
                         whiteSpace: 'pre-wrap'
}} className="py-3 px-3 m-0" >
{JSON.stringify({ tool: tool.name, args: request }, null, 2)}
          </pre>
        </div>
        <div>
          <div style={{
 display: 'flex', alignItems: 'baseline', justifyContent: 'space-between'
}} className="mb-2" >
            <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                           textTransform: 'uppercase' }}>
              {hasRun ? "Response" : "Example response"}
            </span>
            {!hasRun && (
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
                with example inputs · click Run for live
              </span>
            )}
          </div>
          <pre style={{
                         fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.6,
                         background: 'var(--paper)',
                         borderLeft: '2px solid var(--accent)',
                         border: 'var(--hairline)',
                         borderRadius: 6, color: 'var(--ink)',
                         whiteSpace: 'pre-wrap', minHeight: 200
}} className="py-3 px-4 m-0" >
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
    <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-1 mb-1" >
      <span style={{ fontSize: 11, letterSpacing: '0.1em', color: 'var(--ink-2)',
                     textTransform: 'uppercase' }}>
        {input.label || input.key}
      </span>
      {input.required
        ? <span className="mono" style={{ fontSize: 11, color: 'var(--accent)' }}>required</span>
        : <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>optional</span>}
      <span style={{ flex: 1 }}/>
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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
        <div style={{ display: 'flex' }}>
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
