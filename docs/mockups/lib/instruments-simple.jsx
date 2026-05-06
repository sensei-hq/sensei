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
    <div style={{ padding: '22px 44px 18px', borderBottom: 'var(--hairline)',
                   display: 'flex', alignItems: 'center', gap: 18, background: 'var(--paper)' }}>
      <div className="kanji" style={{ fontSize: 40, color: 'var(--shu)', lineHeight: 1 }}>
        {kanji}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 5 }}>
          {eyebrow}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
          <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                            color: 'var(--sumi)' }}>
            {title}
          </h1>
          {sub && (
            <p style={{ fontSize: 12.5, color: 'var(--sumi-2)', margin: 0,
                         maxWidth: 680, lineHeight: 1.55 }}>
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
          <div style={{ padding: '12px 14px', borderBottom: 'var(--hairline)',
                         display: 'flex', gap: 8, alignItems: 'center' }}>
            <span className="kanji" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>探</span>
            <input value={q} onChange={e => setQ(e.target.value)}
                   placeholder="search tools…"
                   style={{ border: 'none', outline: 'none', background: 'transparent',
                            fontSize: 12, flex: 1, color: 'var(--sumi)' }}/>
            {q && (
              <button onClick={() => setQ("")}
                      style={{ fontSize: 11, color: 'var(--sumi-4)' }}>×</button>
            )}
          </div>

          <div style={{ overflow: 'auto', flex: 1, padding: '6px 0 16px' }}>
            {groups.length === 0 && (
              <div style={{ padding: '20px 14px', textAlign: 'center',
                             fontSize: 12, color: 'var(--sumi-4)' }}>
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

          <div style={{ padding: '10px 14px', borderTop: 'var(--hairline)',
                         fontSize: 10.5, color: 'var(--sumi-4)',
                         display: 'flex', gap: 10, justifyContent: 'space-between' }}>
            <span>{I.mcps.length} MCPs · {I.tools.length} tools</span>
            <button style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>+ add MCP</button>
          </div>
        </aside>

        {/* ─── Detail ─── */}
        <main style={{ overflow: 'auto', padding: '24px 44px 36px' }}>
          {focus ? <ToolDetailCompact tool={focus} mcp={I.mcps.find(m => m.id === focus.mcp)}/>
                 : <EmptyDetail/>}
        </main>
      </div>
    </div>
  );
}

function MCPGroup({ mcp, tools, collapsed, onToggle, focusId, onPick }) {
  return (
    <div style={{ marginBottom: 2 }}>
      <button onClick={onToggle}
              style={{ width: '100%', display: 'grid',
                        gridTemplateColumns: '14px 18px 1fr auto auto',
                        gap: 8, alignItems: 'center',
                        padding: '8px 14px', textAlign: 'left',
                        background: 'transparent', border: 'none', cursor: 'pointer',
                        color: 'var(--sumi-2)' }}>
        <span className="mono" style={{ fontSize: 9, color: 'var(--sumi-3)',
                      transform: collapsed ? 'none' : 'rotate(90deg)',
                      transition: 'transform 0.15s' }}>▶</span>
        <span className="kanji" style={{ fontSize: 12, color: 'var(--shu)' }}>
          {mcp.kanji}
        </span>
        <span style={{ fontSize: 12 }}>{mcp.name}</span>
        {!mcp.installed && (
          <span style={{ fontSize: 9, color: 'var(--amber)',
                          letterSpacing: '0.12em', textTransform: 'uppercase' }}>
            off
          </span>
        )}
        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
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
  const kindColor = isAction ? "var(--shu)" : "var(--matcha)";
  const kindGlyph = isAction ? "作" : "問";
  return (
    <button onClick={onClick}
            style={{ width: '100%', display: 'grid',
                      gridTemplateColumns: '32px 14px 1fr',
                      gap: 6, padding: '5px 14px 5px 4px',
                      textAlign: 'left', background: 'transparent', border: 'none',
                      borderLeft: active ? '2px solid var(--shu)' : '2px solid transparent',
                      cursor: 'pointer' }}>
      <span/>
      <span className="kanji" style={{ fontSize: 10.5, color: kindColor }}>{kindGlyph}</span>
      <span className="mono" style={{ fontSize: 11,
                    color: active ? 'var(--sumi)' : 'var(--sumi-2)',
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
    ? { label: "action", color: "var(--shu)",    glyph: "作", hint: "performs an operation" }
    : { label: "query",  color: "var(--matcha)", glyph: "問", hint: "returns information" };

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
      <div style={{ marginBottom: 18 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                       marginBottom: 8, flexWrap: 'wrap' }}>
          <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
            {mcp.kanji} {mcp.name.toLowerCase()}
          </span>
          <span style={{ fontSize: 11, color: 'var(--sumi-4)' }}>·</span>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5,
                          padding: '2px 8px', fontSize: 9.5,
                          background: 'var(--paper-2)', border: 'var(--hairline)',
                          borderRadius: 3, color: kind.color,
                          letterSpacing: '0.14em', textTransform: 'uppercase' }}>
            <span className="kanji" style={{ fontSize: 11 }}>{kind.glyph}</span>
            {kind.label}
          </span>
        </div>
        <h2 className="mono" style={{ fontSize: 17, color: 'var(--sumi)',
                      fontWeight: 400, margin: 0 }}>
          {tool.name}
        </h2>
        <p style={{ fontSize: 13, color: 'var(--sumi-2)', margin: '6px 0 0',
                     lineHeight: 1.55, maxWidth: 700 }}>
          {tool.summary}
        </p>
      </div>

      {/* Inputs */}
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderRadius: 7, padding: '14px 16px', marginBottom: 14 }}>
        <div style={{ display: 'flex', alignItems: 'baseline', marginBottom: 10,
                       justifyContent: 'space-between' }}>
          <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                          textTransform: 'uppercase' }}>
            Inputs
          </span>
          <span style={{ fontSize: 10.5, color: 'var(--sumi-4)' }}>
            {kind.hint}
          </span>
        </div>

        {(tool.inputs || []).length === 0 ? (
          <div style={{ fontSize: 12, color: 'var(--sumi-3)', marginBottom: 10 }}>
            No inputs — just call it.
          </div>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)',
                         gap: '10px 18px' }}>
            {tool.inputs.map(i => (
              <InputRowS key={i.key} input={i}
                         value={values[i.key]}
                         onChange={v => setValues(s => ({ ...s, [i.key]: v }))}/>
            ))}
          </div>
        )}

        <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                       marginTop: 12, paddingTop: 10, borderTop: 'var(--hairline)' }}>
          <button onClick={runExample}
                  style={{ padding: '6px 14px', fontSize: 12,
                            background: 'var(--sumi)', color: 'var(--paper)',
                            border: 'none', borderRadius: 5,
                            cursor: 'pointer', letterSpacing: '0.04em' }}>
            {isAction ? "Run →" : "Query →"}
          </button>
          <span style={{ flex: 1 }}/>
          {status === "running" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>calling …</span>
          )}
          {status === "done" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--matcha)' }}>200 ok</span>
          )}
        </div>
      </div>

      {/* Response */}
      <div>
        <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 6 }}>
          Response {status === "idle" && "· preview"}
        </div>
        <pre style={{ margin: 0, padding: '12px 14px',
                       fontFamily: 'var(--font-mono)', fontSize: 11.5, lineHeight: 1.55,
                       background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${kind.color}`,
                       borderRadius: 5, color: 'var(--sumi-2)',
                       whiteSpace: 'pre-wrap', overflow: 'auto',
                       maxHeight: 360, opacity: status === "idle" ? 0.68 : 1 }}>
{status === "idle" ? tool.example?.response || "—" : response}
        </pre>
      </div>
    </div>
  );
}

function InputRowS({ input, value, onChange }) {
  const label = (
    <label style={{ fontSize: 11, color: 'var(--sumi-2)',
                     display: 'flex', alignItems: 'baseline', gap: 6 }}>
      <span>{input.label}</span>
      {input.required && <span style={{ color: 'var(--shu)' }}>*</span>}
      <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>
        {input.kind}
      </span>
    </label>
  );
  const fieldStyle = {
    padding: '6px 9px', fontSize: 12,
    border: '1px solid var(--paper-edge)', borderRadius: 4,
    background: 'var(--paper)', color: 'var(--sumi)',
    fontFamily: 'var(--font-mono)', outline: 'none'
  };
  let control;
  if (input.kind === "enum" || input.kind === "since") {
    control = (
      <select value={value ?? ""} onChange={e => onChange(e.target.value)} style={fieldStyle}>
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
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label}
      {control}
    </div>
  );
}

function EmptyDetail() {
  return (
    <div style={{ padding: 40, color: 'var(--sumi-4)', fontSize: 13,
                   textAlign: 'center' }}>
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
