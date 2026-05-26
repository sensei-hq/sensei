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
function InstrHero({ kanji, eyebrow, title, sub, right }) {
  return (
    <div style={{
 borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', background: 'var(--paper)'
}} className="gap-4 pt-5 pb-4 px-7" >
      <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>
        {kanji}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-1" >
          {eyebrow}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-1" >
          <h1 className="display m-0" style={{
 fontSize: 22, fontWeight: 400,
                                            color: 'var(--ink)'
}}>
            {title}
          </h1>
          {sub && (
            <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                         maxWidth: 680, lineHeight: 1.55
}} className="m-0" >
              {sub}
            </p>
          )}
        </div>
      </div>
      {right}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// PLAYGROUND — simplified
// Left rail: search + collapsible MCP groups → tools
// Right: detail
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsPlaygroundSimple() {
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
    <div className="sensei" data-screen-label="Instruments · Playground"
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <InstrHero
        kanji="具"
        eyebrow="Instruments · Playground"
        title="Try any tool before you trust it."
        sub="A room of tools. See what each one does, what it returns. Try one."/>

      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '300px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        {/* ─── Left rail — search + MCP tree ─── */}
        <aside style={{ borderRight: 'var(--hairline)', background: 'var(--paper-2)',
                         display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div style={{
 borderBottom: 'var(--hairline)',
                         display: 'flex', alignItems: 'center'
}} className="py-3 px-3 gap-2" >
            <span className="kanji" style={{ fontSize: 11, color: 'var(--ink-3)' }}>探</span>
            <input value={q} onChange={e => setQ(e.target.value)}
                   placeholder="search tools…"
                   style={{ border: 'none', outline: 'none', background: 'transparent',
                            fontSize: 13, flex: 1, color: 'var(--ink)' }}/>
            {q && (
              <button onClick={() => setQ("")}
                      style={{ fontSize: 11, color: 'var(--ink-4)' }}>×</button>
            )}
          </div>

          <div style={{ overflow: 'auto', flex: 1 }} className="pt-1 pb-4" >
            {groups.length === 0 && (
              <div style={{
 textAlign: 'center',
                             fontSize: 13, color: 'var(--ink-4)'
}} className="py-4 px-3" >
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
 borderTop: 'var(--hairline)',
                         fontSize: 11, color: 'var(--ink-4)',
                         display: 'flex', justifyContent: 'space-between'
}} className="py-2 px-3 gap-2" >
            <span>{I.mcps.length} MCPs · {I.tools.length} tools</span>
            <button style={{ fontSize: 11, color: 'var(--ink-3)' }}>+ add MCP</button>
          </div>
        </aside>

        {/* ─── Detail ─── */}
        <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-7" >
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
 width: '100%', display: 'grid',
                        gridTemplateColumns: '14px 18px 1fr auto auto', alignItems: 'center', textAlign: 'left',
                        background: 'transparent', border: 'none', cursor: 'pointer',
                        color: 'var(--ink-2)'
}} className="gap-2 py-2 px-3" >
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                      transform: collapsed ? 'none' : 'rotate(90deg)',
                      transition: 'transform 0.15s' }}>▶</span>
        <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>
          {mcp.kanji}
        </span>
        <span style={{ fontSize: 13 }}>{mcp.name}</span>
        {!mcp.installed && (
          <span style={{ fontSize: 11, color: 'var(--warning)',
                          letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            off
          </span>
        )}
        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
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
 width: '100%', display: 'grid',
                      gridTemplateColumns: '32px 14px 1fr',
                      textAlign: 'left', background: 'transparent', border: 'none',
                      borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent',
                      cursor: 'pointer'
}} className="gap-1 py-1 pl-1 pr-3" >
      <span/>
      <span className="kanji" style={{ fontSize: 11, color: kindColor }}>{kindGlyph}</span>
      <span className="mono" style={{ fontSize: 11,
                    color: active ? 'var(--ink)' : 'var(--ink-2)',
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
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
        <div style={{
 display: 'flex', alignItems: 'center', flexWrap: 'wrap'
}} className="gap-2 mb-2" >
          <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
            {mcp.kanji} {mcp.name.toLowerCase()}
          </span>
          <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>·</span>
          <span style={{
 display: 'inline-flex', alignItems: 'center', fontSize: 11,
                          background: 'var(--paper-2)', border: 'var(--hairline)',
                          borderRadius: 3, color: kind.color,
                          letterSpacing: '0.14em', textTransform: 'uppercase'
}} className="gap-1 py-1 px-2" >
            <span className="kanji" style={{ fontSize: 11 }}>{kind.glyph}</span>
            {kind.label}
          </span>
        </div>
        <h2 className="mono m-0" style={{
 fontSize: 17, color: 'var(--ink)',
                      fontWeight: 400
}}>
          {tool.name}
        </h2>
        <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                     lineHeight: 1.55, maxWidth: 700
}} className="mt-1 mb-0" >
          {tool.summary}
        </p>
      </div>

      {/* Inputs */}
      <div style={{
 background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderRadius: 7
}} className="py-3 px-4 mb-3" >
        <div style={{
 display: 'flex', alignItems: 'baseline',
                       justifyContent: 'space-between'
}} className="mb-2" >
          <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                          textTransform: 'uppercase' }}>
            Inputs
          </span>
          <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>
            {kind.hint}
          </span>
        </div>

        {(tool.inputs || []).length === 0 ? (
          <div style={{ fontSize: 13, color: 'var(--ink-3)' }} className="mb-2" >
            No inputs — just call it.
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)',
                         gap: '8px 16px' }}>
            {tool.inputs.map(i => (
              <InputRowS key={i.key} input={i}
                         value={values[i.key]}
                         onChange={v => setValues(s => ({ ...s, [i.key]: v }))}/>
            ))}
          </div>
        )}

        <div style={{
 display: 'flex', alignItems: 'center', borderTop: 'var(--hairline)'
}} className="gap-2 mt-3 pt-2" >
          <button onClick={runExample}
                  style={{
 fontSize: 13,
                            background: 'var(--ink)', color: 'var(--paper)',
                            border: 'none', borderRadius: 5,
                            cursor: 'pointer', letterSpacing: '0.04em'
}} className="py-1 px-3" >
            {isAction ? "Run →" : "Query →"}
          </button>
          <span style={{ flex: 1 }}/>
          {status === "running" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>calling …</span>
          )}
          {status === "done" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--success)' }}>200 ok</span>
          )}
        </div>
      </div>

      {/* Response */}
      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                       textTransform: 'uppercase'
}} className="mb-1" >
          Response {status === "idle" && "· preview"}
        </div>
        <pre style={{
                       fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
                       background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${kind.color}`,
                       borderRadius: 5, color: 'var(--ink-2)',
                       whiteSpace: 'pre-wrap', overflow: 'auto',
                       maxHeight: 360, opacity: status === "idle" ? 0.68 : 1
}} className="py-3 px-3 m-0" >
{status === "idle" ? tool.example?.response || "—" : response}
        </pre>
      </div>
    </div>
  );
}

function InputRowS({ input, value, onChange }) {
  const label = (
    <label style={{
 fontSize: 11, color: 'var(--ink-2)',
                     display: 'flex', alignItems: 'baseline'
}} className="gap-1" >
      <span>{input.label}</span>
      {input.required && <span style={{ color: 'var(--accent)' }}>*</span>}
      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)' }}>
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
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      {label}
      {control}
    </div>
  );
}

function EmptyDetail() {
  return (
    <div style={{
 color: 'var(--ink-4)', fontSize: 13,
                   textAlign: 'center'
}} className="p-6" >
      Pick a tool to inspect it.
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// REPLAY + INSIGHTS — wrappers around the existing components
// They already have their own internal layouts. We just swap the chrome
// so the slim hero is consistent and the old tab strip is gone.
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsReplaySimple() {
  return <InstrumentsReplay simple={true} embedded={true}/>;
}

function InstrumentsHealthSimple() {
  return <InstrumentsHealth simple={true} embedded={true}/>;
}

Object.assign(window, {
  InstrumentsPlaygroundSimple,
  InstrumentsReplaySimple,
  InstrumentsHealthSimple,
  // back-compat alias
  InstrumentsInsightsSimple: InstrumentsHealthSimple
});
