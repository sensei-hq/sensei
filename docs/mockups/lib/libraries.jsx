// Libraries area — three variations.
// Sensei treats libraries as first-class: detected, imported, external services.
// Each lib can be explained / searched / queried via the sensei MCP.

const { useState: lS } = React;

// Shared building blocks ──────────────────────────────────────
function LibIcon({ letter, tone = 'var(--shu)', size = 28 }) {
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
    indexed: { label: "docs indexed", tone: 'var(--jade)',  bg: 'var(--jade-soft)' },
    partial: { label: "partial",      tone: 'var(--amber)', bg: 'var(--amber-soft)' },
    schema:  { label: "schema only",  tone: 'var(--sumi-2)', bg: 'var(--paper-3)'   },
    none:    { label: "no docs",      tone: 'var(--sumi-3)', bg: 'var(--paper-3)'   }
  };
  const m = map[status] || map.none;
  return (
    <span className="mono" style={{ fontSize: 10, padding: '2px 7px', borderRadius: 3,
                background: m.bg, color: m.tone }}>{m.label}</span>
  );
}

// ── Detail panel (shared by all variations) ──────────────────
function LibraryDetail({ libId, compact = false }) {
  const d = window.LIBRARIES_DATA.details[libId] || window.LIBRARIES_DATA.details.axum;
  const [example, setExample] = lS(0);
  const ex = d.mcpExamples[example];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', gap: 14, alignItems: 'flex-start' }}>
        <LibIcon letter={d.name.charAt(0)} size={40}/>
        <div style={{ flex: 1 }}>
          <div className="display" style={{ fontSize: 20, fontWeight: 400, letterSpacing: '-0.01em' }}>
            {d.name}
          </div>
          <div style={{ fontSize: 12, color: 'var(--sumi-2)', marginTop: 3, lineHeight: 1.55 }}>
            {d.tagline}
          </div>
          <div style={{ display: 'flex', gap: 10, marginTop: 8, alignItems: 'center',
                         fontSize: 10.5, color: 'var(--sumi-3)' }} className="mono">
            <span>v{d.version}</span><span>·</span>
            <span>{d.lang}</span><span>·</span>
            <DocChip status={d.docs}/><span>·</span>
            <span>{d.source}</span>
          </div>
        </div>
      </div>

      <div style={{ padding: '12px 14px', background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 6,
                     fontSize: 12.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
        {d.summary}
      </div>

      {/* Usage grid */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 18 }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>Top symbols</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
            {d.usage.topSymbols.map(s => (
              <div key={s.symbol} style={{ display: 'grid',
                            gridTemplateColumns: '1fr auto', gap: 10, alignItems: 'baseline',
                            padding: '6px 2px', borderBottom: 'var(--hairline)' }}>
                <span className="mono" style={{ fontSize: 11, color: 'var(--sumi)' }}>{s.symbol}</span>
                <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>{s.n}×</span>
              </div>
            ))}
          </div>
        </div>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>Used at</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
            {d.usage.places.map((p, i) => (
              <div key={i} style={{ padding: '7px 2px', borderBottom: 'var(--hairline)' }}>
                <div className="mono" style={{ fontSize: 11, color: 'var(--sumi)' }}>
                  {p.file}<span style={{ color: 'var(--sumi-4)' }}>:{p.line}</span>
                </div>
                <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                              marginTop: 2, whiteSpace: 'nowrap', overflow: 'hidden',
                              textOverflow: 'ellipsis' }}>{p.snippet}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Rules attached */}
      {d.rules && d.rules.length > 0 && (
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>Rules attached</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {d.rules.map((r, i) => (
              <div key={i} style={{ padding: '10px 12px', borderRadius: 5,
                            background: 'var(--paper-2)',
                            borderLeft: '2px solid var(--shu)', border: 'var(--hairline)' }}>
                <div style={{ fontSize: 12.5, color: 'var(--sumi)' }}>"{r.rule}"</div>
                <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)', marginTop: 3 }}>
                  {r.source}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* MCP example interactions — the key insight */}
      <div>
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                       marginBottom: 10 }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
              <span className="kanji" style={{ fontSize: 15, color: 'var(--shu)' }}>具</span>
              <div className="display" style={{ fontSize: 15, fontWeight: 400 }}>
                What sensei can do with this library
              </div>
            </div>
            <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 4, marginLeft: 25 }}>
              Example MCP interactions · each tool callable by an assistant with sensei attached.
            </div>
          </div>
        </div>

        <div style={{ display: 'flex', gap: 6, marginBottom: 12, flexWrap: 'wrap' }}>
          {d.mcpExamples.map((e, i) => {
            const on = example === i;
            return (
              <button key={i} onClick={() => setExample(i)}
                      style={{ padding: '6px 10px', fontSize: 10.5,
                                borderRadius: 4,
                                background: on ? 'var(--sumi)' : 'var(--paper-2)',
                                color: on ? 'var(--paper)' : 'var(--sumi-2)',
                                border: on ? 'none' : 'var(--hairline)',
                                fontFamily: 'var(--font-mono)' }}>
                {e.tool}
              </button>
            );
          })}
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
          <div>
            <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 6 }}>
              Intent
            </div>
            <div style={{ fontSize: 13, color: 'var(--sumi)', marginBottom: 14,
                           fontStyle: 'italic', lineHeight: 1.5 }}>
              "{ex.intent}"
            </div>
            <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 6 }}>
              Request
            </div>
            <pre style={{ margin: 0, padding: '10px 12px',
                           fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.5,
                           background: 'var(--paper-2)', border: 'var(--hairline)',
                           borderRadius: 5, color: 'var(--sumi-2)',
                           whiteSpace: 'pre-wrap', overflow: 'auto' }}>
              {ex.request}
            </pre>
          </div>
          <div>
            <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 6 }}>
              Response
            </div>
            <pre style={{ margin: 0, padding: '12px 14px',
                           fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
                           background: 'var(--paper-2)',
                           borderLeft: '2px solid var(--shu)',
                           border: 'var(--hairline)',
                           borderRadius: 5, color: 'var(--sumi)',
                           whiteSpace: 'pre-wrap', overflow: 'auto',
                           minHeight: 180 }}>
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
      gridTemplateColumns: 'auto 1fr auto auto auto',
      gap: 14, alignItems: 'center',
      padding: '12px 14px', borderRadius: 6,
      textAlign: 'left',
      background: active ? 'var(--paper-2)' : 'transparent',
      borderBottom: 'var(--hairline)'
    }}>
      <LibIcon letter={item.icon}
               tone={item.service ? 'var(--jade)' : item.internal ? 'var(--amber)' : 'var(--shu)'}
               size={32}/>
      <div style={{ minWidth: 0 }}>
        <div style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}>
          <span style={{ fontSize: 13, color: 'var(--sumi)' }}>{item.name}</span>
          <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>v{item.version}</span>
        </div>
        <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 2 }}>
          {item.source}
        </div>
      </div>
      <DocChip status={item.docs}/>
      <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                    minWidth: 60, textAlign: 'right' }}>
        {item.usage}× calls
      </span>
      <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
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

      <div style={{ padding: '28px 44px 18px', borderBottom: 'var(--hairline)',
                     display: 'flex', alignItems: 'flex-end', gap: 20 }}>
        <div className="kanji" style={{ fontSize: 48, color: 'var(--shu)', lineHeight: 1 }}>庫</div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 6 }}>Libraries</div>
          <h1 className="display" style={{ fontSize: 26, fontWeight: 400, margin: 0 }}>
            Tools the student uses. Kept close.
          </h1>
          <p style={{ fontSize: 12.5, color: 'var(--sumi-2)', margin: '6px 0 0', maxWidth: 620 }}>
            Sensei watches imports and flags docs that drift. Ask it anything about how
            you actually use each library — through any assistant that speaks MCP.
          </p>
        </div>
        <button style={{ padding: '9px 14px', fontSize: 12,
                          background: 'var(--sumi)', color: 'var(--paper)',
                          borderRadius: 5 }}>+ add library</button>
      </div>

      {/* Filter row */}
      <div style={{ padding: '14px 44px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 16, alignItems: 'center', flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', gap: 2 }}>
          {kinds.map(k => {
            const on = kind === k.id;
            return (
              <button key={k.id} onClick={() => setKind(k.id)}
                      style={{ padding: '6px 12px', fontSize: 11.5,
                                borderRadius: 4, display: 'inline-flex', gap: 7, alignItems: 'center',
                                background: on ? 'var(--sumi)' : 'transparent',
                                color: on ? 'var(--paper)' : 'var(--sumi-2)' }}>
                <span className="kanji" style={{ fontSize: 11 }}>{k.kanji}</span>
                {k.label}
                <span className="mono" style={{ fontSize: 10,
                              color: on ? 'var(--paper)' : 'var(--sumi-4)', opacity: 0.85 }}>
                  {k.count}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ width: 1, height: 18, background: 'var(--sumi-5, #00000018)' }}/>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 10.5, color: 'var(--sumi-3)', letterSpacing: '0.12em',
                         textTransform: 'uppercase' }}>Lang</span>
          <div style={{ display: 'flex', gap: 2 }}>
            {langs.map(l => {
              const on = lang === l;
              return (
                <button key={l} onClick={() => setLang(l)}
                        style={{ padding: '5px 9px', fontSize: 11,
                                  borderRadius: 4,
                                  background: on ? 'var(--paper-3)' : 'transparent',
                                  color: on ? 'var(--sumi)' : 'var(--sumi-3)' }}>
                  {l}
                </button>
              );
            })}
          </div>
        </div>
        <span style={{ flex: 1 }}/>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                       background: 'var(--paper-2)', borderRadius: 5,
                       padding: '6px 10px', border: 'var(--hairline)', minWidth: 220 }}>
          <span className="kanji" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search libraries…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 12, flex: 1, color: 'var(--sumi)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--sumi-4)' }}>×</button>
          )}
        </div>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
          {filtered.length} of {all.length}
        </span>
      </div>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '1fr 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <div style={{ overflow: 'auto', padding: '20px 44px 32px',
                       borderRight: 'var(--hairline)' }}>
          {filtered.length === 0 && (
            <div style={{ padding: '40px 0', textAlign: 'center',
                           fontSize: 12, color: 'var(--sumi-3)' }}>
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
        <div style={{ overflow: 'auto', padding: '26px 32px', background: 'var(--paper-2)' }}>
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

      <div style={{ padding: '20px 44px 0', borderBottom: 'var(--hairline)' }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 14, marginBottom: 14 }}>
          <span className="kanji" style={{ fontSize: 24, color: 'var(--shu)' }}>庫</span>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0 }}>
            Libraries
          </h1>
          <span style={{ fontSize: 11.5, color: 'var(--sumi-3)' }}>
            detected + imported + connected
          </span>
          <span style={{ flex: 1 }}/>
          <button style={{ fontSize: 11.5, padding: '7px 11px',
                            color: 'var(--sumi-2)', border: 'var(--ink-line)',
                            borderRadius: 5 }}>+ import URL</button>
          <button style={{ fontSize: 11.5, padding: '7px 11px',
                            background: 'var(--sumi)', color: 'var(--paper)',
                            borderRadius: 5 }}>+ register library</button>
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          {D.groups.map(g => {
            const on = tab === g.id;
            return (
              <button key={g.id} onClick={() => setTab(g.id)}
                      style={{ padding: '10px 14px', fontSize: 12.5,
                                borderBottom: on ? '2px solid var(--shu)' : '2px solid transparent',
                                color: on ? 'var(--sumi)' : 'var(--sumi-3)', marginBottom: -1,
                                display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                <span className="kanji" style={{ fontSize: 12 }}>{g.kanji}</span>
                {g.label}
                <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
                  {g.items.length}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '320px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <div style={{ overflow: 'auto', padding: '14px 12px',
                       borderRight: 'var(--hairline)', background: 'var(--paper-2)' }}>
          <div style={{ fontSize: 11, color: 'var(--sumi-3)', padding: '4px 10px 8px' }}>
            {group.sub}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            {group.items.map(x => (
              <button key={x.id} onClick={() => setFocus(x.id)}
                      style={{
                        display: 'grid', gridTemplateColumns: 'auto 1fr auto', gap: 10,
                        alignItems: 'center', padding: '10px 12px', borderRadius: 5,
                        textAlign: 'left',
                        background: focus === x.id ? 'var(--paper)' : 'transparent',
                        borderBottom: 'var(--hairline)'
                      }}>
                <LibIcon letter={x.icon}
                         tone={x.service ? 'var(--jade)' : x.internal ? 'var(--amber)' : 'var(--shu)'}
                         size={26}/>
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontSize: 12.5, color: 'var(--sumi)' }}>{x.name}</div>
                  <div className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
                    v{x.version} · {x.usage}× calls
                  </div>
                </div>
                <DocChip status={x.docs}/>
              </button>
            ))}
          </div>
        </div>

        <div style={{ overflow: 'auto', padding: '28px 44px' }}>
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
                <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                              padding: '5px 9px', border: 'var(--hairline)', borderRadius: 3 }}>
                  {scopeMcp.tools} tools
                </span>
              }>

      {/* MCP scope selector — horizontal pill row */}
      <div style={{ padding: '10px 56px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 14, alignItems: 'center',
                     background: 'var(--paper-2)' }}>
        <span style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase' }}>
          MCP
        </span>
        <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap' }}>
          {mcpSources.map(m => {
            const on = scope === m.id;
            return (
              <button key={m.id} onClick={() => setScope(m.id)}
                      style={{ display: 'inline-flex', gap: 6, alignItems: 'center',
                                padding: '5px 10px', fontSize: 11.5, borderRadius: 4,
                                background: on ? 'var(--paper)' : 'transparent',
                                border: on ? '1px solid var(--sumi-4)' : '1px solid transparent',
                                color: on ? 'var(--sumi)' : 'var(--sumi-2)' }}>
                <span className="kanji" style={{ fontSize: 11,
                              color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{m.kanji}</span>
                <span>{m.name}</span>
                <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
                  {m.tools}
                </span>
              </button>
            );
          })}
        </div>
        <span style={{ flex: 1 }}/>
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
          {mcpSources.length} installed
        </span>
      </div>

      {!isSensei && (
        <div style={{ padding: '14px 56px',
                       background: 'var(--paper-2)',
                       borderBottom: 'var(--hairline)',
                       fontSize: 12, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
          <span className="kanji" style={{ color: 'var(--amber)', fontSize: 13, marginRight: 8 }}>告</span>
          Third-party MCP. Sensei lists these tools from the server's manifest — you can inspect each,
          but sensei doesn't wrap or index them.
        </div>
      )}

      {/* Filter row */}
      <div style={{ padding: '12px 56px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 14, alignItems: 'center' }}>
        <div style={{ display: 'flex', gap: 2 }}>
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
        <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                       background: 'var(--paper-2)', borderRadius: 5,
                       padding: '6px 10px', border: 'var(--hairline)', minWidth: 240 }}>
          <span className="kanji" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>探</span>
          <input value={query} onChange={e => setQuery(e.target.value)}
                 placeholder="search tools…"
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 12, flex: 1, color: 'var(--sumi)' }}/>
          {query && (
            <button onClick={() => setQuery("")}
                    style={{ fontSize: 11, color: 'var(--sumi-4)' }}>×</button>
          )}
        </div>
      </div>

      {/* Two-pane */}
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '340px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <aside style={{ overflow: 'auto', borderRight: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
          {filtered.length === 0 && (
            <div style={{ padding: '24px 18px', fontSize: 12, color: 'var(--sumi-3)',
                           textAlign: 'center' }}>
              No tools match.
            </div>
          )}
          {T.categories.map(c => {
            const items = filtered.filter(t => t.category === c.id);
            if (items.length === 0) return null;
            return (
              <div key={c.id} style={{ padding: '16px 12px 6px' }}>
                <div style={{ padding: '0 10px 8px',
                               display: 'flex', gap: 8, alignItems: 'baseline' }}>
                  <span className="kanji" style={{ fontSize: 11, color: 'var(--shu)' }}>
                    {c.kanji}
                  </span>
                  <span style={{ fontSize: 9.5, letterSpacing: '0.16em',
                                  color: 'var(--sumi-3)', textTransform: 'uppercase' }}>
                    {c.label}
                  </span>
                  <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>
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

        <main style={{ overflow: 'auto', padding: '26px 44px 40px' }}>
          <ToolDetail tool={focus} cat={cat}/>
        </main>
      </div>
    </MCPShell>
  );
}

function CatChip({ on, onClick, kanji, label, count }) {
  return (
    <button onClick={onClick}
            style={{ padding: '6px 11px', fontSize: 11.5,
                      borderRadius: 4, display: 'inline-flex', gap: 7, alignItems: 'center',
                      background: on ? 'var(--sumi)' : 'transparent',
                      color: on ? 'var(--paper)' : 'var(--sumi-2)' }}>
      <span className="kanji" style={{ fontSize: 11 }}>{kanji}</span>
      {label}
      <span className="mono" style={{ fontSize: 10,
                    color: on ? 'var(--paper)' : 'var(--sumi-4)', opacity: 0.85 }}>
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
              display: 'block', width: '100%', textAlign: 'left',
              padding: '9px 12px', borderRadius: 5,
              background: active ? 'var(--paper)' : 'transparent',
              border: active ? '1px solid var(--paper-edge)' : '1px solid transparent',
              marginBottom: 2
            }}>
      <div className="mono" style={{ fontSize: 11.5,
                    color: active ? 'var(--sumi)' : 'var(--sumi-2)' }}>
        {tool.name}
      </div>
      <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 3,
                     lineHeight: 1.45,
                     overflow: 'hidden', display: '-webkit-box',
                     WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
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
    <div style={{ display: 'flex', flexDirection: 'column', gap: 22 }}>
      {/* Tool header */}
      <div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'baseline', marginBottom: 6 }}>
          <span className="kanji" style={{ fontSize: 13, color: 'var(--shu)' }}>
            {cat.kanji}
          </span>
          <span style={{ fontSize: 10, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase' }}>
            {cat.label}
          </span>
        </div>
        <div className="mono" style={{ fontSize: 17, color: 'var(--sumi)', marginBottom: 8 }}>
          {tool.name}
        </div>
        <div style={{ fontSize: 13, color: 'var(--sumi-2)', lineHeight: 1.55,
                       maxWidth: 680 }}>
          {tool.summary}
        </div>
      </div>

      {/* Form */}
      <div style={{ padding: '18px 20px', background: 'var(--paper-2)',
                     border: 'var(--hairline)', borderRadius: 8 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 14 }}>
          <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase' }}>Inputs</span>
          <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
            · {tool.inputs.length}
          </span>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))',
                       gap: 14, marginBottom: 16 }}>
          {tool.inputs.map(input => (
            <InputField key={input.key} input={input}
                        value={values[input.key]}
                        onChange={v => setVal(input.key, v)}/>
          ))}
        </div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <button onClick={() => setHasRun(true)}
                  disabled={missing.length > 0}
                  style={{ padding: '8px 16px', fontSize: 12,
                            background: missing.length > 0 ? 'var(--paper-3)' : 'var(--sumi)',
                            color: missing.length > 0 ? 'var(--sumi-3)' : 'var(--paper)',
                            borderRadius: 5,
                            cursor: missing.length > 0 ? 'not-allowed' : 'pointer' }}>
            Run tool →
          </button>
          {missing.length > 0 && (
            <span style={{ fontSize: 11, color: 'var(--amber)' }}>
              required: {missing.map(m => m.label || m.key).join(", ")}
            </span>
          )}
          <span style={{ flex: 1 }}/>
          <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-3)' }}>
            callable by any MCP-attached assistant
          </span>
        </div>
      </div>

      {/* Request + Response */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.3fr', gap: 20 }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 8 }}>
            Request
          </div>
          <pre style={{ margin: 0, padding: '12px 14px',
                         fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
                         background: 'var(--paper-2)', border: 'var(--hairline)',
                         borderRadius: 6, color: 'var(--sumi-2)',
                         whiteSpace: 'pre-wrap' }}>
{JSON.stringify({ tool: tool.name, args: request }, null, 2)}
          </pre>
        </div>
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                         marginBottom: 8 }}>
            <span style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase' }}>
              {hasRun ? "Response" : "Example response"}
            </span>
            {!hasRun && (
              <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)' }}>
                with example inputs · click Run for live
              </span>
            )}
          </div>
          <pre style={{ margin: 0, padding: '14px 16px',
                         fontFamily: 'var(--font-mono)', fontSize: 11.5, lineHeight: 1.6,
                         background: 'var(--paper)',
                         borderLeft: '2px solid var(--shu)',
                         border: 'var(--hairline)',
                         borderRadius: 6, color: 'var(--sumi)',
                         whiteSpace: 'pre-wrap', minHeight: 200 }}>
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
    <div style={{ display: 'flex', alignItems: 'baseline', gap: 6, marginBottom: 6 }}>
      <span style={{ fontSize: 10.5, letterSpacing: '0.1em', color: 'var(--sumi-2)',
                     textTransform: 'uppercase' }}>
        {input.label || input.key}
      </span>
      {input.required
        ? <span className="mono" style={{ fontSize: 9.5, color: 'var(--shu)' }}>required</span>
        : <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>optional</span>}
      <span style={{ flex: 1 }}/>
      <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>
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
                style={selectStyle}>
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
        <div style={{ display: 'flex', gap: 2 }}>
          {opts.map(o => {
            const on = value === o;
            return (
              <button key={o} onClick={() => onChange(o)}
                      style={{ padding: '6px 10px', fontSize: 11,
                                borderRadius: 4,
                                background: on ? 'var(--sumi)' : 'var(--paper-3)',
                                color: on ? 'var(--paper)' : 'var(--sumi-2)' }}>
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
  width: '100%', padding: '7px 10px', fontSize: 12,
  border: 'var(--hairline)', borderRadius: 5,
  background: 'var(--paper)', color: 'var(--sumi)',
  fontFamily: 'var(--font-mono)', outline: 'none'
};
const selectStyle = {
  ...inputStyle, fontFamily: 'Inter, sans-serif', fontSize: 12
};


Object.assign(window, { LibrariesVariantA, LibrariesVariantB, MCPPlayground });
