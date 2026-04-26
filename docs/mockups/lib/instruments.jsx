// Instruments — the three-tab surface for MCP tools.
//
// An "instrument" is a tool exposed by an MCP server. Sensei ships some;
// Postgres, Stripe, GitHub, etc. ship others. Tools are either:
//   · action — does something (writes, triggers)
//   · query  — returns information
//
// The three tabs answer distinct questions:
//   具 Playground — what CAN these instruments do?  (interactive try)
//   録 Replay     — what DID the assistant do?      (per-session tool-call log)
//   照 Insights   — what SHOULD we change?          (usage + effectiveness)
//
// This file exports:
//   · InstrumentsShell     — shared chrome with tab nav
//   · InstrumentsPlayground — revised (MCP-as-app-chooser, flat list, kinds)
//   · InstrumentsReplay    — simplified (request/response only)
//   · InstrumentsInsights  — unchanged from the old MCPInsights
//   · InstrumentsApp       — connected host that switches tabs

const { useState: iUseS, useMemo: iUseM } = React;

// ═══════════════════════════════════════════════════════════════════════
// Shared shell
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsShell({ activeTab, onTab, embedded = false,
                             simple = false,
                             kanji, tagline, sub, chip, children }) {
  const tabs = [
    { id: "playground", kanji: "具", label: "Playground",
      hint: "what can these instruments do?" },
    { id: "replay",     kanji: "録", label: "Replay",
      hint: "what did the assistant do?" },
    { id: "insights",   kanji: "照", label: "Insights",
      hint: "what should we change?" }
  ];
  const chrome = !embedded;

  return (
    <div className="sensei" data-screen-label={`Instruments · ${activeTab}`}
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      {chrome && <TauriChrome title={`Sensei  先生  ·  instruments · ${activeTab}`}/>}

      {simple ? (
        /* Slim one-line hero for the simple layout — used when this
           component is its own destination in the observatory sidebar
           and the old tab strip is gone. */
        <div style={{ padding: '22px 44px 18px', borderBottom: 'var(--hairline)',
                       display: 'flex', alignItems: 'center', gap: 18, background: 'var(--paper)' }}>
          <div className="kanji" style={{ fontSize: 40, color: 'var(--shu)', lineHeight: 1 }}>
            {kanji || "具"}
          </div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                           textTransform: 'uppercase', marginBottom: 5 }}>
              Instruments · {activeTab}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              <h1 className="display" style={{ fontSize: 22, fontWeight: 400, margin: 0,
                                                color: 'var(--sumi)' }}>
                {tagline}
              </h1>
              {sub && (
                <p style={{ fontSize: 12.5, color: 'var(--sumi-2)', margin: 0,
                             maxWidth: 680, lineHeight: 1.55 }}>
                  {sub}
                </p>
              )}
            </div>
          </div>
          {chip}
        </div>
      ) : (
        <>
          {/* Full hero */}
          <div style={{ padding: '26px 56px 18px', display: 'flex',
                         alignItems: 'flex-end', gap: 20, borderBottom: 'var(--hairline)' }}>
            <div className="kanji" style={{ fontSize: 44, color: 'var(--shu)', lineHeight: 1 }}>
              {kanji || "具"}
            </div>
            <div style={{ flex: 1 }}>
              <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                             textTransform: 'uppercase', marginBottom: 6 }}>
                Instruments · {activeTab}
              </div>
              <h1 className="display" style={{ fontSize: 24, fontWeight: 400, margin: 0 }}>
                {tagline}
              </h1>
              {sub && (
                <p style={{ fontSize: 12.5, color: 'var(--sumi-2)', margin: '6px 0 0',
                             maxWidth: 680, lineHeight: 1.55 }}>
                  {sub}
                </p>
              )}
            </div>
            {chip}
          </div>

          {/* Tab nav */}
          <div style={{ padding: '0 56px', borderBottom: 'var(--hairline)',
                         display: 'flex', gap: 0, background: 'var(--paper)' }}>
            {tabs.map(t => {
              const on = t.id === activeTab;
              return (
                <button key={t.id} onClick={() => onTab && onTab(t.id)}
                        style={{ padding: '14px 18px 13px',
                                  display: 'flex', alignItems: 'center', gap: 10,
                                  background: 'transparent', border: 'none',
                                  borderBottom: on ? '2px solid var(--sumi)' : '2px solid transparent',
                                  marginBottom: -1,
                                  color: on ? 'var(--sumi)' : 'var(--sumi-3)',
                                  cursor: 'pointer' }}>
                  <span className="kanji" style={{ fontSize: 15,
                                color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{t.kanji}</span>
                  <span className="display" style={{ fontSize: 13 }}>{t.label}</span>
                  <span style={{ fontSize: 10.5, color: 'var(--sumi-4)' }}>· {t.hint}</span>
                </button>
              );
            })}
          </div>
        </>
      )}

      <div style={{ flex: 1, minHeight: 0, overflow: 'hidden', display: 'flex',
                     flexDirection: 'column' }}>
        {children}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// PLAYGROUND — redesigned
// MCP is the primary axis (top row, app-chooser). Tool list is flat
// within the selected MCP. Kind chips (all/action/query) + search.
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsPlayground({ activeTab = "playground", onTab = () => {},
                                  embedded = false, simple = false } = {}) {
  const I = window.INSTRUMENTS;
  const [mcpId, setMcpId] = iUseS("sensei");
  const [kind, setKind]   = iUseS("all");  // all · action · query
  const [q, setQ]         = iUseS("");
  const [focusId, setFocusId] = iUseS(null);

  const mcp = I.mcps.find(m => m.id === mcpId) || I.mcps[0];
  const mcpTools = I.tools.filter(t => t.mcp === mcpId);

  const ql = q.toLowerCase().trim();
  const filtered = mcpTools.filter(t =>
    (kind === "all" || t.kind === kind) &&
    (!ql || t.name.toLowerCase().includes(ql) || t.summary.toLowerCase().includes(ql))
  );
  const focus = filtered.find(t => t.id === focusId)
             || filtered[0]
             || mcpTools[0];

  return (
    <InstrumentsShell activeTab={activeTab} onTab={onTab} embedded={embedded} simple={simple}
      kanji={mcp.kanji}
      tagline={`${mcp.name} · ${mcp.tagline}`}
      sub={mcp.id === "sensei"
        ? "Sensei's own MCP — tools that run against your local index of code, libraries, patterns, and sessions. Any assistant with sensei attached can call them."
        : `${mcp.name} is an MCP server. Sensei lists the tools from its manifest and lets you try each one. Third-party MCPs aren't wrapped — sensei just surfaces them.`}
      chip={
        <div style={{ display: 'flex', gap: 12, fontSize: 11, color: 'var(--sumi-3)' }}>
          <Stat2 label="tools"   v={mcp.toolCount}/>
          <Stat2 label="actions" v={mcp.actionCount}/>
          <Stat2 label="queries" v={mcp.queryCount}/>
        </div>
      }>

      {/* MCP chooser — primary axis */}
      <div style={{ padding: '14px 56px', borderBottom: 'var(--hairline)',
                     background: 'var(--paper-2)',
                     display: 'flex', gap: 8, alignItems: 'center', overflow: 'auto' }}>
        <span style={{ fontSize: 9.5, letterSpacing: '0.16em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase', marginRight: 8, flexShrink: 0 }}>
          MCP
        </span>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {I.mcps.map(m => (
            <MCPPill key={m.id} mcp={m} on={m.id === mcpId}
                     onClick={() => { setMcpId(m.id); setFocusId(null); setKind("all"); setQ(""); }}/>
          ))}
          <button style={{ padding: '7px 12px', fontSize: 11.5,
                            color: 'var(--sumi-3)', border: '1px dashed var(--paper-edge)',
                            borderRadius: 5, background: 'transparent', cursor: 'pointer' }}>
            + Add MCP
          </button>
        </div>
      </div>

      {/* Kind + search strip */}
      <div style={{ padding: '10px 56px', borderBottom: 'var(--hairline)',
                     display: 'flex', gap: 10, alignItems: 'center' }}>
        <KindChip kanji="全" label="All"     on={kind === "all"}
                   count={mcpTools.length}         onClick={() => setKind("all")}/>
        <KindChip kanji="作" label="Actions" on={kind === "action"}
                   count={mcpTools.filter(t => t.kind === "action").length}
                   onClick={() => setKind("action")} tone="shu"/>
        <KindChip kanji="問" label="Queries" on={kind === "query"}
                   count={mcpTools.filter(t => t.kind === "query").length}
                   onClick={() => setKind("query")} tone="matcha"/>
        <span style={{ flex: 1 }}/>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8,
                       background: 'var(--paper-2)', borderRadius: 5,
                       padding: '6px 10px', border: 'var(--hairline)', minWidth: 260 }}>
          <span className="kanji" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>探</span>
          <input value={q} onChange={e => setQ(e.target.value)}
                 placeholder={`search ${mcp.name.toLowerCase()} tools…`}
                 style={{ border: 'none', outline: 'none', background: 'transparent',
                          fontSize: 12, flex: 1, color: 'var(--sumi)' }}/>
          {q && (
            <button onClick={() => setQ("")}
                    style={{ fontSize: 11, color: 'var(--sumi-4)' }}>×</button>
          )}
        </div>
      </div>

      {/* Two-pane list + detail */}
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '340px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>
        <aside style={{ overflow: 'auto', borderRight: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
          {filtered.length === 0 ? (
            <div style={{ padding: '26px 20px', textAlign: 'center',
                           fontSize: 12, color: 'var(--sumi-4)' }}>
              No tools match.
            </div>
          ) : (
            <div style={{ padding: '6px 0' }}>
              {filtered.map(t => (
                <ToolRowFlat key={t.id} tool={t}
                             active={focus && focus.id === t.id}
                             onClick={() => setFocusId(t.id)}/>
              ))}
            </div>
          )}
        </aside>

        <main style={{ overflow: 'auto', padding: '26px 44px 40px' }}>
          {focus
            ? <ToolDetailFlat tool={focus} mcp={mcp}/>
            : <EmptyFocus/>}
        </main>
      </div>
    </InstrumentsShell>
  );
}

function Stat2({ label, v }) {
  return (
    <span style={{ display: 'inline-flex', alignItems: 'baseline', gap: 5 }}>
      <span className="mono" style={{ fontSize: 12, color: 'var(--sumi)',
                    fontFeatureSettings: '"tnum"' }}>{v}</span>
      <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                      textTransform: 'uppercase' }}>{label}</span>
    </span>
  );
}

function MCPPill({ mcp, on, onClick }) {
  return (
    <button onClick={onClick}
            style={{ display: 'inline-flex', gap: 8, alignItems: 'center',
                      padding: '7px 12px', fontSize: 12, borderRadius: 5,
                      background: on ? 'var(--paper)' : 'transparent',
                      border: on ? '1px solid var(--sumi-4)' : '1px solid transparent',
                      color: on ? 'var(--sumi)' : 'var(--sumi-2)',
                      cursor: 'pointer', whiteSpace: 'nowrap' }}>
      <span className="kanji" style={{ fontSize: 13,
                    color: on ? 'var(--shu)' : 'var(--sumi-3)' }}>{mcp.kanji}</span>
      <span>{mcp.name}</span>
      <span className="mono" style={{ fontSize: 10.5,
                    color: on ? 'var(--sumi-3)' : 'var(--sumi-4)' }}>
        {mcp.toolCount}
      </span>
      {!mcp.installed && (
        <span style={{ fontSize: 9.5, color: 'var(--amber)',
                        letterSpacing: '0.12em', textTransform: 'uppercase' }}>
          not installed
        </span>
      )}
    </button>
  );
}

function KindChip({ kanji, label, on, count, onClick, tone }) {
  const toneColor = tone === "shu"    ? "var(--shu)" :
                    tone === "matcha" ? "var(--matcha)" : "var(--sumi-3)";
  return (
    <button onClick={onClick}
            style={{ display: 'inline-flex', gap: 6, alignItems: 'center',
                      padding: '6px 11px', fontSize: 11.5, borderRadius: 4,
                      background: on ? 'var(--sumi)' : 'transparent',
                      border: 'none',
                      color: on ? 'var(--paper)' : 'var(--sumi-2)', cursor: 'pointer' }}>
      <span className="kanji" style={{ fontSize: 11,
                    color: on ? 'var(--paper)' : toneColor }}>{kanji}</span>
      <span>{label}</span>
      <span className="mono" style={{ fontSize: 10,
                    color: on ? 'var(--paper)' : 'var(--sumi-4)' }}>
        {count}
      </span>
    </button>
  );
}

function ToolRowFlat({ tool, active, onClick }) {
  const kindGlyph = tool.kind === "action" ? "作" : "問";
  const kindColor = tool.kind === "action" ? "var(--shu)" : "var(--matcha)";
  return (
    <button onClick={onClick}
            style={{ display: 'grid', gridTemplateColumns: '14px 1fr',
                      gap: 10, padding: '11px 18px',
                      width: '100%', textAlign: 'left',
                      background: active ? 'var(--paper)' : 'transparent',
                      border: 'none',
                      borderLeft: active ? '2px solid var(--shu)' : '2px solid transparent',
                      cursor: 'pointer' }}>
      <span className="kanji" style={{ fontSize: 12, color: kindColor, marginTop: 2 }}>
        {kindGlyph}
      </span>
      <div>
        <div className="mono" style={{ fontSize: 11.5,
                      color: active ? 'var(--sumi)' : 'var(--sumi-2)',
                      overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {tool.name}
        </div>
        <div style={{ fontSize: 11, color: 'var(--sumi-3)', marginTop: 3,
                       lineHeight: 1.4,
                       overflow: 'hidden', display: '-webkit-box',
                       WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
          {tool.summary}
        </div>
      </div>
    </button>
  );
}

function ToolDetailFlat({ tool, mcp }) {
  const [values, setValues] = iUseS(() => Object.fromEntries(
    (tool.inputs || []).map(i => [i.key, i.default ?? ""])
  ));
  const [status, setStatus] = iUseS("idle"); // idle · running · done
  const [response, setResponse] = iUseS("");

  // Reset form when tool changes
  React.useEffect(() => {
    setValues(Object.fromEntries((tool.inputs || []).map(i => [i.key, i.default ?? ""])));
    setStatus("idle");
    setResponse("");
  }, [tool.id]);

  const kindBadge = tool.kind === "action"
    ? { label: "action", color: "var(--shu)",    glyph: "作",
        hint: "performs an operation" }
    : { label: "query",  color: "var(--matcha)", glyph: "問",
        hint: "returns information" };

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
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 14,
                     marginBottom: 20 }}>
        <div style={{ flex: 1 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
            <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi-3)' }}>
              {mcp.kanji} {mcp.name.toLowerCase()}
            </span>
            <span style={{ fontSize: 11.5, color: 'var(--sumi-4)' }}>·</span>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5,
                            padding: '2px 9px', fontSize: 10,
                            background: 'var(--paper-2)', border: 'var(--hairline)',
                            borderRadius: 3, color: kindBadge.color,
                            letterSpacing: '0.12em', textTransform: 'uppercase' }}>
              <span className="kanji" style={{ fontSize: 11 }}>{kindBadge.glyph}</span>
              {kindBadge.label}
            </span>
          </div>
          <h2 className="mono" style={{ fontSize: 17, color: 'var(--sumi)',
                        fontWeight: 400, margin: 0 }}>
            {tool.name}
          </h2>
          <p style={{ fontSize: 13, color: 'var(--sumi-2)', margin: '8px 0 0',
                       lineHeight: 1.55, maxWidth: 720 }}>
            {tool.summary}
          </p>
        </div>
      </div>

      {/* Inputs form */}
      <div style={{ background: 'var(--paper-2)', border: 'var(--hairline)',
                     borderRadius: 7, padding: '16px 18px', marginBottom: 16 }}>
        <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 10 }}>
          Inputs
        </div>
        {(tool.inputs || []).length === 0 && (
          <div style={{ fontSize: 12, color: 'var(--sumi-3)' }}>
            No inputs — just call it.
          </div>
        )}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)',
                       gap: '12px 20px' }}>
          {(tool.inputs || []).map(i => (
            <InputRow key={i.key} input={i}
                      value={values[i.key]}
                      onChange={v => setValues(s => ({ ...s, [i.key]: v }))}/>
          ))}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 10,
                       marginTop: 14, paddingTop: 12, borderTop: 'var(--hairline)' }}>
          <button onClick={runExample}
                  style={{ padding: '7px 14px', fontSize: 12,
                            background: 'var(--sumi)', color: 'var(--paper)',
                            border: 'none', borderRadius: 5,
                            cursor: 'pointer', letterSpacing: '0.04em' }}>
            {tool.kind === "action" ? "Run →" : "Query →"}
          </button>
          <span style={{ fontSize: 11, color: 'var(--sumi-4)' }}>
            {kindBadge.hint}
          </span>
          <span style={{ flex: 1 }}/>
          {status === "running" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>
              calling …
            </span>
          )}
          {status === "done" && (
            <span className="mono" style={{ fontSize: 11, color: 'var(--matcha)' }}>
              200 ok
            </span>
          )}
        </div>
      </div>

      {/* Response */}
      <div>
        <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                       textTransform: 'uppercase', marginBottom: 8 }}>
          Response {status === "idle" && "· preview"}
        </div>
        <pre style={{ margin: 0, padding: '14px 16px',
                       fontFamily: 'var(--font-mono)', fontSize: 11.5, lineHeight: 1.6,
                       background: 'var(--paper-2)', border: 'var(--hairline)',
                       borderLeft: `2px solid ${kindBadge.color}`,
                       borderRadius: 5, color: 'var(--sumi-2)',
                       whiteSpace: 'pre-wrap', overflow: 'auto',
                       maxHeight: 380, opacity: status === "idle" ? 0.68 : 1 }}>
{status === "idle" ? tool.example?.response || "—" : response}
        </pre>
        {status === "idle" && (
          <div style={{ fontSize: 10.5, color: 'var(--sumi-4)', marginTop: 6,
                         fontStyle: 'italic' }}>
            Example response. Click {tool.kind === "action" ? "Run" : "Query"} to invoke.
          </div>
        )}
      </div>
    </div>
  );
}

function InputRow({ input, value, onChange }) {
  const labelBlock = (
    <label style={{ fontSize: 11, color: 'var(--sumi-2)',
                     display: 'flex', alignItems: 'baseline', gap: 6 }}>
      <span>{input.label}</span>
      {input.required && <span style={{ color: 'var(--shu)' }}>*</span>}
      <span className="mono" style={{ fontSize: 9.5, color: 'var(--sumi-4)' }}>
        {input.kind}
      </span>
    </label>
  );

  let control;
  if (input.kind === "enum" || input.kind === "since") {
    control = (
      <select value={value ?? ""} onChange={e => onChange(e.target.value)}
              style={fieldStyle}>
        {(input.options || []).map(o => <option key={o} value={o}>{o}</option>)}
      </select>
    );
  } else if (input.kind === "number") {
    control = (
      <input type="number" value={value ?? ""}
             onChange={e => onChange(e.target.value)}
             style={fieldStyle}/>
    );
  } else {
    control = (
      <input type="text" value={value ?? ""}
             onChange={e => onChange(e.target.value)}
             placeholder={input.placeholder || ""}
             style={fieldStyle}/>
    );
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
      {labelBlock}
      {control}
      {input.help && (
        <span style={{ fontSize: 10.5, color: 'var(--sumi-4)' }}>{input.help}</span>
      )}
    </div>
  );
}

const fieldStyle = {
  padding: '7px 10px', fontSize: 12,
  border: '1px solid var(--paper-edge)', borderRadius: 4,
  background: 'var(--paper)', color: 'var(--sumi)',
  fontFamily: 'var(--font-mono)',
  outline: 'none'
};

function EmptyFocus() {
  return (
    <div style={{ padding: 40, color: 'var(--sumi-4)', fontSize: 13,
                   textAlign: 'center' }}>
      Select a tool to inspect it.
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
// REPLAY — simplified
// Per-session timeline. Each call shows request + response.
// No "what assistant did next" semantic — just the transaction.
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsReplay({ activeTab = "replay", onTab = () => {},
                              embedded = false, simple = false } = {}) {
  const all = window.SENSEI_DATA.sessions;
  const signals = window.MCP_SIGNALS.sessions;
  const sessionIds = Object.keys(signals);
  const [pickedId, setPickedId] = iUseS(sessionIds[0]);
  const [focusCall, setFocusCall] = iUseS(1);

  const sess = signals[pickedId];
  const sessMeta = all.find(s => s.id === pickedId) || {};
  const currentCall = sess.calls.find(c => c.i === focusCall) || sess.calls[0];

  return (
    <InstrumentsShell activeTab={activeTab} onTab={onTab} embedded={embedded} simple={simple}
      kanji="録"
      tagline="Every instrument call, in order."
      sub="Step through the tools the assistant reached for during a session. Pure request + response — what was asked, what came back, how long it took."
      chip={
        <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                      padding: '5px 9px', border: 'var(--hairline)', borderRadius: 3 }}>
          {sessionIds.length} sessions indexed
        </span>
      }>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '300px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>

        {/* Session picker */}
        <aside style={{ overflow: 'auto', borderRight: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
          <div style={{ padding: '14px 14px 8px',
                         fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase' }}>
            sessions
          </div>
          {sessionIds.map(sid => {
            const m = all.find(s => s.id === sid) || {};
            const sg = signals[sid];
            const on = pickedId === sid;
            return (
              <button key={sid}
                      onClick={() => { setPickedId(sid); setFocusCall(1); }}
                      style={{ display: 'block', width: '100%', textAlign: 'left',
                                padding: '10px 14px',
                                background: on ? 'var(--paper)' : 'transparent',
                                border: 'none',
                                borderLeft: on ? '2px solid var(--shu)' : '2px solid transparent',
                                cursor: 'pointer' }}>
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 6 }}>
                  <span className="mono" style={{ fontSize: 10.5,
                                color: on ? 'var(--sumi)' : 'var(--sumi-2)' }}>{sid}</span>
                  <span style={{ fontSize: 9.5,
                                 color: m.ftr ? 'var(--matcha)' : 'var(--amber)',
                                 letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                    {m.ftr ? "ftr" : `${m.corrections}c`}
                  </span>
                </div>
                <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', marginTop: 3,
                               lineHeight: 1.4,
                               overflow: 'hidden', display: '-webkit-box',
                               WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' }}>
                  {sg.title}
                </div>
                <div style={{ fontSize: 10, color: 'var(--sumi-4)', marginTop: 3 }}>
                  {m.project} · {sg.toolCallCount} calls · {m.duration || "–"}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main style={{ overflow: 'auto', padding: '22px 40px 40px' }}>
          {/* Session summary strip */}
          <div style={{ display: 'grid',
                         gridTemplateColumns: 'auto auto auto auto 1fr',
                         alignItems: 'baseline', gap: 22, marginBottom: 18,
                         paddingBottom: 14, borderBottom: 'var(--hairline)' }}>
            <div>
              <div className="display" style={{ fontSize: 16, marginBottom: 2 }}>{sess.title}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{pickedId}</div>
            </div>
            <StatR label="turns"      value={sess.totalTurns}/>
            <StatR label="tool calls" value={sess.toolCallCount}/>
            <StatR label="ftr"        value={sess.ftr ? "yes" : "no"}
                   tone={sess.ftr ? "good" : "warn"}/>
            <span/>
          </div>

          {/* Timeline + detail */}
          <div style={{ display: 'grid', gridTemplateColumns: '0.95fr 1.4fr', gap: 26 }}>
            {/* Left: timeline */}
            <div>
              <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                             textTransform: 'uppercase', marginBottom: 10 }}>
                timeline ({sess.calls.length})
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
                {sess.calls.map(c => {
                  const on = focusCall === c.i;
                  const kindFromName = inferKind(c.tool);
                  const dot = kindFromName === "action" ? "var(--shu)" : "var(--matcha)";
                  return (
                    <button key={c.i} onClick={() => setFocusCall(c.i)}
                            style={{ display: 'grid',
                                      gridTemplateColumns: '22px 38px 1fr auto',
                                      gap: 8, alignItems: 'center',
                                      padding: '10px 10px 10px 12px',
                                      textAlign: 'left', borderRadius: 5,
                                      background: on ? 'var(--paper-2)' : 'transparent',
                                      border: on ? '1px solid var(--paper-edge)' : '1px solid transparent',
                                      cursor: 'pointer' }}>
                      <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                                    fontFeatureSettings: '"tnum"' }}>{c.i}</span>
                      <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)',
                                    fontFeatureSettings: '"tnum"' }}>{c.t}</span>
                      <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi)',
                                    overflow: 'hidden', textOverflow: 'ellipsis',
                                    whiteSpace: 'nowrap' }}>
                        {shortName(c.tool)}
                      </span>
                      <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        <span style={{ width: 7, height: 7, borderRadius: '50%', background: dot }}/>
                        <span className="mono" style={{ fontSize: 10, color: 'var(--sumi-4)',
                                      fontFeatureSettings: '"tnum"' }}>
                          {c.durationMs}ms
                        </span>
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Right: request + response */}
            <CallDetailSimple call={currentCall}/>
          </div>
        </main>
      </div>
    </InstrumentsShell>
  );
}

function CallDetailSimple({ call }) {
  const kind = inferKind(call.tool);
  const kindBadge = kind === "action"
    ? { label: "action", color: "var(--shu)",    glyph: "作" }
    : { label: "query",  color: "var(--matcha)", glyph: "問" };
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between',
                     paddingBottom: 12, borderBottom: 'var(--hairline)' }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            call #{call.i} · turn {call.turn} · {call.t}
          </div>
          <div className="mono" style={{ fontSize: 14, color: 'var(--sumi)' }}>
            {call.tool}
          </div>
        </div>
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6,
                        padding: '4px 10px', fontSize: 10.5,
                        background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 3, color: kindBadge.color,
                        letterSpacing: '0.1em', textTransform: 'uppercase' }}>
          <span className="kanji" style={{ fontSize: 11 }}>{kindBadge.glyph}</span>
          {kindBadge.label}
        </span>
      </div>

      <CallPanelR label="request">
        <pre style={preStyleR}>
{JSON.stringify({ tool: call.tool, args: call.args }, null, 2)}
        </pre>
      </CallPanelR>

      <CallPanelR label={`response · ${call.durationMs}ms`}>
        <div style={{ ...preStyleR, borderLeft: `2px solid ${kindBadge.color}`,
                       color: 'var(--sumi)' }}>
          {call.responseSnippet}
        </div>
        <div style={{ fontSize: 10.5, color: 'var(--sumi-4)', marginTop: 6,
                       fontStyle: 'italic' }}>
          {kind === "action"
            ? "Action response — describes what the call did."
            : "Query response — the data the assistant received."}
        </div>
      </CallPanelR>
    </div>
  );
}

function CallPanelR({ label, children }) {
  return (
    <div>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 6 }}>{label}</div>
      {children}
    </div>
  );
}

const preStyleR = {
  margin: 0, padding: '11px 14px',
  fontFamily: 'var(--font-mono)', fontSize: 11.5, lineHeight: 1.55,
  background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 5,
  color: 'var(--sumi-2)', whiteSpace: 'pre-wrap', overflow: 'auto'
};

function StatR({ label, value, tone }) {
  const color = tone === "good" ? "var(--matcha)" :
                tone === "warn" ? "var(--amber)" : "var(--sumi)";
  return (
    <div>
      <div style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                     textTransform: 'uppercase' }}>{label}</div>
      <div className="display" style={{ fontSize: 16, color,
                    fontFeatureSettings: '"tnum"', marginTop: 2 }}>{value}</div>
    </div>
  );
}

function shortName(fqn) { return fqn.replace(/^sensei\./, ""); }

// Replay data was authored before kinds existed. Infer from tool name.
// Known actions: sensei.pattern.promote. Everything else is a query.
const KNOWN_ACTIONS = new Set(["sensei.pattern.promote"]);
function inferKind(toolName) {
  return KNOWN_ACTIONS.has(toolName) ? "action" : "query";
}

// ═══════════════════════════════════════════════════════════════════════
// INSIGHTS — unchanged, re-exported with new label
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsInsights({ activeTab = "insights", onTab = () => {},
                                embedded = false, simple = false } = {}) {
  const I = window.MCP_SIGNALS.insights;
  const [window_, setWindow] = iUseS(I.window);
  const [focusTool, setFocusTool] = iUseS(null);

  return (
    <InstrumentsShell activeTab={activeTab} onTab={onTab} embedded={embedded} simple={simple}
      kanji="照"
      tagline="Which instruments earn their keep — and what to change."
      sub="Aggregated across every session in the window. Usage alone isn't success; the signal is whether the assistant DID something with the response, and whether sessions that touched the tool landed first-try more often than ones that didn't."
      chip={
        <div style={{ display: 'flex', gap: 2, background: 'var(--paper-2)',
                       padding: 2, borderRadius: 5, border: 'var(--hairline)' }}>
          {["7d", "30d", "90d"].map(w => (
            <button key={w} onClick={() => setWindow(w)}
                    style={{ padding: '4px 10px', fontSize: 11, borderRadius: 3,
                              background: window_ === w ? 'var(--paper)' : 'transparent',
                              color: window_ === w ? 'var(--sumi)' : 'var(--sumi-3)',
                              border: 'none', cursor: 'pointer' }}>{w}</button>
          ))}
        </div>
      }>

      <main style={{ overflow: 'auto', padding: '22px 56px 40px' }}>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)',
                       gap: 14, marginBottom: 22 }}>
          <Kpi kanji="録" label="sessions analyzed"
               value={I.sessionsAnalyzed} hint={window_}/>
          <Kpi kanji="計" label="total tool calls"
               value={I.deltas.totalCalls.toLocaleString()}
               delta={`${I.deltas.totalCallsTrend > 0 ? "+" : ""}${Math.round(I.deltas.totalCallsTrend * 100)}%`}
               deltaTone={I.deltas.totalCallsTrend > 0 ? "good" : "warn"}/>
          <Kpi kanji="一" label="first-try rate"
               value={`${Math.round(I.deltas.ftrThisWindow * 100)}%`}
               delta={`${I.deltas.ftrTrend > 0 ? "+" : ""}${Math.round(I.deltas.ftrTrend * 100)} pts`}
               deltaTone={I.deltas.ftrTrend > 0 ? "good" : "warn"}/>
          <Kpi kanji="警" label="tools with warnings"
               value={I.deltas.warnTools} hint="ignored · low usage"
               tone={I.deltas.warnTools > 0 ? "warn" : "neutral"}/>
          <Kpi kanji="眠" label="dormant tools"
               value={I.deltas.unusedTools} hint="0 calls this window"
               tone={I.deltas.unusedTools > 0 ? "warn" : "neutral"}/>
        </div>

        <div style={{ marginBottom: 26 }}>
          <div style={{ display: 'flex', alignItems: 'baseline',
                         justifyContent: 'space-between', marginBottom: 12 }}>
            <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0 }}>
              Signals
              <span style={{ fontSize: 12, color: 'var(--sumi-3)', marginLeft: 10 }}>
                · what the data suggests you change
              </span>
            </h3>
            <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
              {I.signals.length} signals
            </span>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))',
                         gap: 12 }}>
            {I.signals.map((s, i) => <SignalCard key={i} s={s}/>)}
          </div>
        </div>

        <div style={{ display: 'grid', gridTemplateColumns: '1.6fr 1fr', gap: 26 }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'baseline',
                           justifyContent: 'space-between', marginBottom: 12 }}>
              <h3 className="display" style={{ fontSize: 16, fontWeight: 400, margin: 0 }}>
                Per-tool usage
              </h3>
              <span className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)' }}>
                {I.toolUsage.length} tools · sorted by calls
              </span>
            </div>
            <div style={{ border: 'var(--hairline)', borderRadius: 7,
                           overflow: 'hidden', background: 'var(--paper)' }}>
              <ToolRowHeader/>
              {I.toolUsage.map((t, idx) => (
                <ToolUsageRow key={t.tool} t={t}
                              focus={focusTool === t.tool}
                              onFocus={() => setFocusTool(
                                focusTool === t.tool ? null : t.tool
                              )}
                              last={idx === I.toolUsage.length - 1}/>
              ))}
            </div>
          </div>

          <div>
            <h3 className="display" style={{ fontSize: 16, fontWeight: 400,
                          margin: '0 0 12px' }}>
              By project
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {I.byProject.map(p => <ProjectUsageRow key={p.project} p={p}/>)}
            </div>

            <div style={{ marginTop: 22, padding: '14px 16px',
                           background: 'var(--paper-2)', border: 'var(--hairline)',
                           borderRadius: 7 }}>
              <div style={{ fontSize: 10.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                             textTransform: 'uppercase', marginBottom: 6 }}>
                how insights work
              </div>
              <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.6 }}>
                Sensei logs every instrument call its assistants make, pairs the response
                with the next turn, and asks: did the assistant cite the result, ignore it,
                or only use part of it? Roll that up across sessions and you see which
                tools actually change what the assistant does — and which deserve a rewrite.
              </div>
            </div>
          </div>
        </div>
      </main>
    </InstrumentsShell>
  );
}

// Insights helpers — reused from before
function Kpi({ kanji, label, value, delta, deltaTone, hint, tone }) {
  const valueColor = tone === "warn" ? "var(--amber)" : "var(--sumi)";
  return (
    <div style={{ padding: '12px 14px', background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 7 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
        <span className="kanji" style={{ fontSize: 12, color: 'var(--shu)' }}>{kanji}</span>
        <span style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                        textTransform: 'uppercase' }}>{label}</span>
      </div>
      <div className="display" style={{ fontSize: 22, color: valueColor,
                    fontFeatureSettings: '"tnum"' }}>{value}</div>
      <div style={{ marginTop: 2, fontSize: 10.5, color: 'var(--sumi-3)' }}>
        {delta && (
          <span style={{ color: deltaTone === "good" ? "var(--matcha)" :
                                deltaTone === "warn" ? "var(--shu)" : "var(--sumi-3)",
                         marginRight: 6 }}>
            {delta}
          </span>
        )}
        {hint && <span>{hint}</span>}
      </div>
    </div>
  );
}

function SignalCard({ s }) {
  const palette = {
    warn:        { border: "var(--amber)",  tintBg: "rgba(194,141,68,0.06)",  label: "warn"  },
    opportunity: { border: "var(--matcha)", tintBg: "rgba(122,158,98,0.06)", label: "lift" },
    unused:      { border: "var(--sumi-3)", tintBg: "var(--paper-2)",         label: "quiet" },
    win:         { border: "var(--shu)",    tintBg: "rgba(196,80,53,0.06)",   label: "win"   }
  };
  const p = palette[s.kind] || palette.warn;
  return (
    <div style={{ padding: '14px 16px', background: p.tintBg,
                   border: 'var(--hairline)', borderLeft: `3px solid ${p.border}`,
                   borderRadius: 5 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 6 }}>
        <span className="kanji" style={{ fontSize: 14, color: p.border }}>{s.kanji}</span>
        <span style={{ fontSize: 9.5, letterSpacing: '0.16em', color: p.border,
                        textTransform: 'uppercase' }}>{p.label}</span>
      </div>
      <div className="display" style={{ fontSize: 13.5, marginBottom: 4 }}>{s.title}</div>
      <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
        {s.body}
      </div>
      <div style={{ marginTop: 10, paddingTop: 10, borderTop: 'var(--hairline)' }}>
        <button style={{ fontSize: 11, color: p.border, padding: 0,
                          background: 'transparent', border: 'none', cursor: 'pointer',
                          letterSpacing: '0.04em' }}>
          {s.action} →
        </button>
      </div>
    </div>
  );
}

function ToolRowHeader() {
  return (
    <div style={{ display: 'grid',
                   gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px',
                   gap: 12, padding: '10px 14px',
                   background: 'var(--paper-2)', borderBottom: 'var(--hairline)' }}>
      {["tool", "calls", "trend 14d", "usage split", "ftr Δ"].map(h => (
        <div key={h} style={{ fontSize: 9.5, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                               textTransform: 'uppercase' }}>{h}</div>
      ))}
    </div>
  );
}

function ToolUsageRow({ t, focus, onFocus, last }) {
  const verdict = {
    healthy:   { color: "var(--matcha)",  glyph: "✓" },
    ok:        { color: "var(--sumi-3)",  glyph: "·" },
    warn:      { color: "var(--amber)",   glyph: "!" },
    underused: { color: "var(--sumi-3)",  glyph: "·" },
    unused:    { color: "var(--shu)",     glyph: "○" }
  }[t.verdict] || { color: "var(--sumi-3)", glyph: "·" };

  return (
    <div>
      <button onClick={onFocus}
              style={{ width: '100%', display: 'grid',
                        gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px',
                        gap: 12, padding: '10px 14px',
                        background: focus ? 'var(--paper-2)' : 'transparent',
                        border: 'none',
                        borderBottom: last && !focus ? 'none' : 'var(--hairline)',
                        textAlign: 'left', cursor: 'pointer',
                        alignItems: 'center' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <span className="kanji" style={{ fontSize: 11, color: verdict.color }}>
              {verdict.glyph}
            </span>
            <span className="mono" style={{ fontSize: 11.5, color: 'var(--sumi)' }}>
              {shortName(t.tool)}
            </span>
          </div>
        </div>
        <div className="mono" style={{ fontSize: 12, color: 'var(--sumi-2)',
                      fontFeatureSettings: '"tnum"' }}>
          {t.calls}
        </div>
        <Sparkline data={t.trend}/>
        <UsageBar used={t.usedPct} partial={t.partialPct} ignored={t.ignoredPct}/>
        <div className="mono" style={{ fontSize: 11.5,
                      color: t.ftrDelta > 0 ? "var(--matcha)" :
                             t.ftrDelta < 0 ? "var(--amber)" : "var(--sumi-3)",
                      fontFeatureSettings: '"tnum"' }}>
          {t.ftrDelta > 0 ? "+" : ""}{Math.round(t.ftrDelta * 100)} pts
        </div>
      </button>
      {focus && (
        <div style={{ padding: '10px 14px 14px 46px',
                       borderBottom: last ? 'none' : 'var(--hairline)',
                       background: 'var(--paper-2)' }}>
          <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 4 }}>
            verdict · {t.verdict}
          </div>
          <div style={{ fontSize: 11.5, color: 'var(--sumi-2)', lineHeight: 1.55 }}>
            {t.note}
          </div>
        </div>
      )}
    </div>
  );
}

function Sparkline({ data }) {
  const max = Math.max(1, ...data);
  const w = 110, h = 22, step = w / (data.length - 1 || 1);
  const pts = data.map((v, i) => `${i * step},${h - (v / max) * (h - 2) - 1}`).join(" ");
  return (
    <svg width={w} height={h} style={{ display: 'block' }}>
      <polyline points={pts} fill="none"
                stroke="var(--sumi-3)" strokeWidth="1.3"/>
      <circle cx={w} cy={h - (data[data.length - 1] / max) * (h - 2) - 1}
              r="2" fill="var(--shu)"/>
    </svg>
  );
}

function UsageBar({ used, partial, ignored }) {
  return (
    <div>
      <div style={{ height: 8, display: 'flex', borderRadius: 2, overflow: 'hidden',
                     background: 'var(--paper-3)' }}>
        <div style={{ width: `${used}%`, background: 'var(--matcha)' }}/>
        <div style={{ width: `${partial}%`, background: 'var(--amber)' }}/>
        <div style={{ width: `${ignored}%`, background: 'var(--shu)' }}/>
      </div>
      <div style={{ display: 'flex', gap: 6, marginTop: 3,
                     fontSize: 9.5, color: 'var(--sumi-3)',
                     fontFeatureSettings: '"tnum"' }}>
        <span>{used}% used</span>
        {ignored > 0 && <span style={{ color: 'var(--shu)' }}>{ignored}% ignored</span>}
      </div>
    </div>
  );
}

function ProjectUsageRow({ p }) {
  return (
    <div style={{ padding: '10px 12px', background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 5,
                   display: 'grid', gridTemplateColumns: '1fr auto',
                   gap: 10, alignItems: 'baseline' }}>
      <div>
        <div style={{ fontSize: 12.5, color: 'var(--sumi)' }}>{p.project}</div>
        <div className="mono" style={{ fontSize: 10.5, color: 'var(--sumi-3)', marginTop: 2 }}>
          {p.sessions} sessions · {p.toolCalls} calls · top: {shortName(p.topTool)}
        </div>
      </div>
      <div style={{ textAlign: 'right' }}>
        <div className="display" style={{ fontSize: 15,
                      color: p.ftr >= 0.7 ? 'var(--matcha)' :
                             p.ftr >= 0.5 ? 'var(--sumi)' : 'var(--amber)',
                      fontFeatureSettings: '"tnum"' }}>
          {Math.round(p.ftr * 100)}%
        </div>
        <div style={{ fontSize: 9, letterSpacing: '0.14em', color: 'var(--sumi-4)',
                       textTransform: 'uppercase' }}>ftr</div>
      </div>
    </div>
  );
}


// ═══════════════════════════════════════════════════════════════════════
// HOST — connected app with tab state
// Use this when Instruments is mounted inside the Observatory or the
// canvas as a single artboard. Switching tabs swaps the body.
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsApp({ initialTab = "playground", embedded = false } = {}) {
  const [tab, setTab] = iUseS(initialTab);
  const props = { activeTab: tab, onTab: setTab, embedded };
  if (tab === "replay")   return <InstrumentsReplay {...props}/>;
  if (tab === "insights") return <InstrumentsInsights {...props}/>;
  return <InstrumentsPlayground {...props}/>;
}

Object.assign(window, {
  InstrumentsShell, InstrumentsPlayground, InstrumentsReplay, InstrumentsInsights,
  InstrumentsApp
});
