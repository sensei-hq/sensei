// Instruments — simpler layout.
//
// Replaces the three-tab InstrumentsApp with three independent destinations
// that each live as a sibling under "Instruments" in the observatory sidebar.
//
// Design goals (vs the old shell):
//   · no internal tabs                      → nav moves to the host sidebar
//   · no top-level MCP pill row             → MCP becomes grouped sections in the left rail
//   · no separate kind filter               → kind shows inline on each tool row
//   · one slim hero strip                   → reclaim vertical space
//
// Exports:
//   · InstrumentsPlaygroundSimple
//   · InstrumentsReplaySimple    (thin wrapper over the old Replay body)
//   · InstrumentsHealthSimple   (thin wrapper over the old Insights body)

const { useState: isS, useEffect: isE } = React;

// ═══════════════════════════════════════════════════════════════════════
// Shared slim hero
// ═══════════════════════════════════════════════════════════════════════
// Delegates to the shared ScreenHeader for a band identical to every other screen.
function InstrHero({ kanji, eyebrow, title, sub, right }) {
  return (
    <ScreenHeader kanji={kanji} eyebrow={eyebrow} title={title} sub={sub} right={right}/>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// PLAYGROUND — simplified
// Left rail: search + collapsible MCP groups → tools
// Right: detail
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsPlaygroundSimple({ subNav } = {}) {
  const I = window.INSTRUMENTS;
  const [q, setQ] = isS("");
  const [focusId, setFocusId] = isS(null);
  // All MCPs start expanded (tree view, collapsed at MCP level means clickable).
  const [collapsed, setCollapsed] = isS(() => {
    // Only sensei starts open. Third-parties collapsed.
    const s = {};
    I.mcps.forEach(m => { s[m.id] = m.id !== "sensei"; });
    return s;
  });

  const ql = q.toLowerCase().trim();
  const groups = I.mcps.map(m => {
    const tools = I.tools.filter(t =>
      t.mcp === m.id &&
      (!ql || t.name.toLowerCase().includes(ql) || t.summary.toLowerCase().includes(ql))
    );
    return { mcp: m, tools };
  }).filter(g => !ql || g.tools.length > 0);

  // If we have a query, auto-expand any group with hits.
  const effectiveCollapsed = ql
    ? Object.fromEntries(groups.map(g => [g.mcp.id, false]))
    : collapsed;

  // Pick a focus tool — prefer the currently-focused one if still visible,
  // otherwise the first tool in the first non-empty group.
  const flat = groups.flatMap(g => g.tools);
  const focus = flat.find(t => t.id === focusId) || flat[0] || null;

  const toggle = (mid) =>
    setCollapsed(s => ({ ...s, [mid]: !s[mid] }));

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label="Instruments · Playground"
 >
      <InstrHero
        kanji="具"
        eyebrow="Instruments · Playground"
        title="Try any tool before you trust it."
        sub="A room of tools. See what each one does, what it returns. Try one."/>

      {subNav}

      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '300px 1fr' }}>
        {/* ─── Left rail — search + MCP tree ─── */}
        <aside className="border-r bg-paper-2 flex flex-col overflow-hidden" >
          <div className="py-3 px-3 gap-2 border-b flex items-center" >
            <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
            <input className="border-0 bg-transparent flex-1 text-ink" value={q} onChange={e => setQ(e.target.value)}
 placeholder="search tools…"
 style={{ outline: 'none',
 fontSize: 13 }}/>
            {q && (
              <button className="text-ink-4" onClick={() => setQ("")}
 style={{ fontSize: 11 }}>×</button>
            )}
          </div>

          <div className="pt-1 pb-4 overflow-auto flex-1" >
            {groups.length === 0 && (
              <div style={{
 fontSize: 13 }} className="py-4 px-3 text-center text-ink-4" >
                no tools match.
              </div>
            )}
            {groups.map(g => (
              <MCPGroup key={g.mcp.id}
                        mcp={g.mcp}
                        tools={g.tools}
                        collapsed={effectiveCollapsed[g.mcp.id]}
                        onToggle={() => toggle(g.mcp.id)}
                        focusId={focus && focus.id}
                        onPick={setFocusId}/>
            ))}
          </div>

          <div style={{
 fontSize: 11 }} className="py-2 px-3 gap-2 border-t text-ink-4 flex justify-between" >
            <span>{I.mcps.length} MCPs · {I.tools.length} tools</span>
            <button className="text-ink-3" style={{ fontSize: 11 }}>+ add MCP</button>
          </div>
        </aside>

        {/* ─── Detail ─── */}
        <main className="pt-6 pb-8 px-12 overflow-auto" >
          {focus ? <ToolDetailCompact tool={focus} mcp={I.mcps.find(m => m.id === focus.mcp)}/>
                 : <EmptyDetail/>}
        </main>
      </div>
    </div>
  );
}

function MCPGroup({ mcp, tools, collapsed, onToggle, focusId, onPick }) {
  return (
    <div className="mb-1" >
      <button onClick={onToggle}
 style={{
 gridTemplateColumns: '14px 18px 1fr auto auto' }} className="gap-2 py-2 px-3 w-full grid items-center text-left bg-transparent border-0 cursor-pointer text-ink-2" >
        <span className="mono text-ink-3" style={{ fontSize: 11,
 transform: collapsed ? 'none' : 'rotate(90deg)',
 transition: 'transform 0.15s' }}>▶</span>
        <span className="kanji text-accent" style={{ fontSize: 13 }}>
          {mcp.kanji}
        </span>
        <span style={{ fontSize: 13 }}>{mcp.name}</span>
        {!mcp.installed && (
          <span className="text-warning uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
            off
          </span>
        )}
        <span className="mono text-ink-4" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>
          {tools.length}
        </span>
      </button>
      {!collapsed && (
        <div>
          {tools.map(t => (
            <ToolLine key={t.id} tool={t}
                      active={focusId === t.id}
                      onClick={() => onPick(t.id)}/>
          ))}
        </div>
      )}
    </div>
  );
}

function ToolLine({ tool, active, onClick }) {
  const isAction = tool.kind === "action";
  const kindColor = isAction ? "var(--accent)" : "var(--success)";
  const kindGlyph = isAction ? "作" : "問";
  return (
    <button onClick={onClick}
 style={{
 gridTemplateColumns: '32px 14px 1fr',
 borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent' }} className="gap-1 py-1 pl-1 pr-3 w-full grid text-left bg-transparent border-0 cursor-pointer" >
      <span/>
      <span className="kanji" style={{ fontSize: 11, color: kindColor }}>{kindGlyph}</span>
      <span className="mono overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11,
 color: active ? 'var(--ink)' : 'var(--ink-2)' }}>
        {tool.name}
      </span>
    </button>
  );
}

function ToolDetailCompact({ tool, mcp }) {
  const [values, setValues] = isS(() => Object.fromEntries(
    (tool.inputs || []).map(i => [i.key, i.default ?? ""])
  ));
  const [status, setStatus] = isS("idle");
  const [response, setResponse] = isS("");

  isE(() => {
    setValues(Object.fromEntries((tool.inputs || []).map(i => [i.key, i.default ?? ""])));
    setStatus("idle");
    setResponse("");
  }, [tool.id]);

  const isAction = tool.kind === "action";
  const kind = isAction
    ? { label: "action", color: "var(--accent)",    glyph: "作", hint: "performs an operation" }
    : { label: "query",  color: "var(--success)", glyph: "問", hint: "returns information" };

  const runExample = () => {
    setStatus("running");
    setTimeout(() => {
      setResponse(tool.example?.response || "(no example response)");
      setStatus("done");
    }, 360);
  };

  return (
    <div>
      {/* Heading */}
      <div className="mb-4" >
        <div className="gap-2 mb-2 flex items-center flex-wrap" >
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            {mcp.kanji} {mcp.name.toLowerCase()}
          </span>
          <span className="text-ink-4" style={{ fontSize: 11 }}>·</span>
          <span style={{ fontSize: 11,
 borderRadius: 3, color: kind.color,
 letterSpacing: '0.14em' }} className="gap-1 py-1 px-2 inline-flex items-center bg-paper-2 border border-paper-edge uppercase" >
            <span className="kanji" style={{ fontSize: 11 }}>{kind.glyph}</span>
            {kind.label}
          </span>
        </div>
        <h2 className="mono m-0 text-ink font-normal" style={{
 fontSize: 17 }}>
          {tool.name}
        </h2>
        <p style={{
 fontSize: 13,
 lineHeight: 1.55, maxWidth: 700
 }} className="mt-1 mb-0 text-ink-2" >
          {tool.summary}
        </p>
      </div>

      {/* Inputs */}
      <div style={{
 borderRadius: 7
 }} className="py-3 px-4 mb-3 bg-paper-2 border border-paper-edge" >
        <div className="mb-2 flex items-baseline justify-between" >
          <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>
            Inputs
          </span>
          <span className="text-ink-4" style={{ fontSize: 11 }}>
            {kind.hint}
          </span>
        </div>

        {(tool.inputs || []).length === 0 ? (
          <div style={{ fontSize: 13 }} className="mb-2 text-ink-3" >
            No inputs — just call it.
          </div>
        ) : (
          <div className="grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)',
 gap: '8px 16px' }}>
            {tool.inputs.map(i => (
              <InputRowS key={i.key} input={i}
                         value={values[i.key]}
                         onChange={v => setValues(s => ({ ...s, [i.key]: v }))}/>
            ))}
          </div>
        )}

        <div className="gap-2 mt-3 pt-2 flex items-center border-t" >
          <button onClick={runExample}
 style={{
 fontSize: 13, borderRadius: 5, letterSpacing: '0.04em'
 }} className="py-1 px-3 bg-ink text-paper border-0 cursor-pointer" >
            {isAction ? "Run →" : "Query →"}
          </button>
          <span className="flex-1" />
          {status === "running" && (
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>calling …</span>
          )}
          {status === "done" && (
            <span className="mono text-success" style={{ fontSize: 11 }}>200 ok</span>
          )}
        </div>
      </div>

      {/* Response */}
      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
          Response {status === "idle" && "· preview"}
        </div>
        <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
 borderLeft: `2px solid ${kind.color}`,
 borderRadius: 5,
 whiteSpace: 'pre-wrap',
 maxHeight: 360, opacity: status === "idle" ? 0.68 : 1
 }} className="py-3 px-3 m-0 bg-paper-2 border border-paper-edge text-ink-2 overflow-auto" >
{status === "idle" ? tool.example?.response || "—" : response}
        </pre>
      </div>
    </div>
  );
}

function InputRowS({ input, value, onChange }) {
  const label = (
    <label style={{
 fontSize: 11 }} className="gap-1 text-ink-2 flex items-baseline" >
      <span>{input.label}</span>
      {input.required && <span className="text-accent" >*</span>}
      <span className="mono text-ink-4" style={{ fontSize: 11 }}>
        {input.kind}
      </span>
    </label>
  );
  const fieldStyle = {
    padding: '4px 8px', fontSize: 13,
    border: '1px solid var(--edge)', borderRadius: 4,
    background: 'var(--paper)', color: 'var(--ink)',
    fontFamily: 'var(--font-mono)', outline: 'none'
  };
  let control;
  if (input.kind === "enum" || input.kind === "since") {
    control = (
      <select value={value ?? ""} onChange={e => onChange(e.target.value)} style={fieldStyle} className="gap-1" >
        {(input.options || []).map(o => <option key={o} value={o}>{o}</option>)}
      </select>
    );
  } else if (input.kind === "number") {
    control = (
      <input type="number" value={value ?? ""}
             onChange={e => onChange(e.target.value)} style={fieldStyle}/>
    );
  } else {
    control = (
      <input type="text" value={value ?? ""}
             onChange={e => onChange(e.target.value)}
             placeholder={input.placeholder || ""} style={fieldStyle}/>
    );
  }
  return (
    <div className="flex flex-col" >
      {label}
      {control}
    </div>
  );
}

function EmptyDetail() {
  return (
    <div style={{ fontSize: 13 }} className="p-8 text-ink-4 text-center" >
      Pick a tool to inspect it.
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// REPLAY + INSIGHTS — wrappers around the existing components
// They already have their own internal layouts. We just swap the chrome
// so the slim hero is consistent and the old tab strip is gone.
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsReplaySimple({ subNav } = {}) {
  return <InstrumentsReplay simple={true} embedded={true} subNav={subNav}/>;
}

function InstrumentsHealthSimple({ subNav } = {}) {
  return <InstrumentsHealth simple={true} embedded={true} subNav={subNav}/>;
}

Object.assign(window, {
  InstrumentsPlaygroundSimple,
  InstrumentsReplaySimple,
  InstrumentsHealthSimple,
  // back-compat alias
  InstrumentsInsightsSimple: InstrumentsHealthSimple
});
