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
//   健 Health     — what SHOULD we change?          (usage + effectiveness)
//
// Renamed from "Insights" to "Health" to avoid colliding with the
// observatory's top-level Insights surface (the things sensei has
// noticed about your work). "Health" honestly names what this view is:
// a diagnostic of how your toolset is performing. Route, component,
// and tab id all renamed; an InstrumentsInsights alias remains on
// window for any older artboard markup that hasn't been touched yet.
//
// This file exports:
//   · InstrumentsShell     — shared chrome with tab nav
//   · InstrumentsPlayground — revised (MCP-as-app-chooser, flat list, kinds)
//   · InstrumentsReplay    — simplified (request/response only)
//   · InstrumentsHealth     — toolset health (usage + effectiveness)
//   · InstrumentsApp       — connected host that switches tabs

const { useState: iUseS, useMemo: iUseM } = React;

// ═══════════════════════════════════════════════════════════════════════
// Shared shell
// ═══════════════════════════════════════════════════════════════════════
function InstrumentsShell({ activeTab, onTab, embedded = false,
                             simple = false, subNav,
                             kanji, tagline, sub, chip, children }) {
  const tabs = [
    { id: "playground", kanji: "具", label: "Playground",
      hint: "what can these instruments do?" },
    { id: "replay",     kanji: "録", label: "Replay",
      hint: "what did the assistant do?" },
    { id: "health",     kanji: "健", label: "Health",
      hint: "what should we change?" }
  ];
  const chrome = !embedded;

  return (
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label={`Instruments · ${activeTab}`}
 >
      {chrome && <TauriChrome title={`Sensei  先生  ·  instruments · ${activeTab}`}/>}

      {simple ? (
        /* Slim one-line hero for the simple layout — used when this
           component is its own destination in the observatory sidebar
           and the old tab strip is gone. */
        <div className="gap-4 pt-6 pb-4 px-12 border-b flex items-center bg-paper" >
          <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>
            {kanji || "具"}
          </div>
          <div className="flex-1 min-w-0" >
            <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
              Instruments · {activeTab}
            </div>
            <div className="gap-1 flex flex-col" >
              <h1 className="display m-0 font-normal text-ink" style={{
 fontSize: 22 }}>
                {tagline}
              </h1>
              {sub && (
                <p style={{
 fontSize: 13,
 maxWidth: 680, lineHeight: 1.55
 }} className="m-0 text-ink-2" >
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
          <div className="gap-4 pt-6 pb-4 px-12 flex items-end border-b" >
            <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>
              {kanji || "具"}
            </div>
            <div className="flex-1" >
              <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
                Instruments · {activeTab}
              </div>
              <h1 className="display m-0 font-normal" style={{ fontSize: 22 }}>
                {tagline}
              </h1>
              {sub && (
                <p style={{
 fontSize: 13,
 maxWidth: 680, lineHeight: 1.55
 }} className="mt-1 mb-0 text-ink-2" >
                  {sub}
                </p>
              )}
            </div>
            {chip}
          </div>

          {/* Tab nav */}
          <div className="px-12 gap-0 border-b flex bg-paper" >
            {tabs.map(t => {
              const on = t.id === activeTab;
              return (
                <button key={t.id} onClick={() => onTab && onTab(t.id)}
 style={{
 borderBottom: on ? '2px solid var(--ink)' : '2px solid transparent',
 marginBottom: -1,
 color: on ? 'var(--ink)' : 'var(--ink-3)' }} className="gap-2 py-3 px-4 flex items-center bg-transparent border-0 cursor-pointer" >
                  <span className="kanji" style={{ fontSize: 15,
                                color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{t.kanji}</span>
                  <span className="display" style={{ fontSize: 13 }}>{t.label}</span>
                  <span className="text-ink-4" style={{ fontSize: 11 }}>· {t.hint}</span>
                </button>
              );
            })}
          </div>
        </>
      )}

      {subNav}

      <div className="flex-1 min-h-0 overflow-hidden flex flex-col" >
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
                                  embedded = false, simple = false, subNav } = {}) {
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
      subNav={subNav}
      kanji={mcp.kanji}
      tagline={`${mcp.name} · ${mcp.tagline}`}
      sub={mcp.id === "sensei"
        ? "Sensei's own MCP — tools that run against your local index of code, libraries, patterns, and sessions. Any assistant with sensei attached can call them."
        : `${mcp.name} is an MCP server. Sensei lists the tools from its manifest and lets you try each one. Third-party MCPs aren't wrapped — sensei just surfaces them.`}
      chip={
        <div style={{ fontSize: 11 }} className="gap-3 flex text-ink-3" >
          <Stat2 label="tools"   v={mcp.toolCount}/>
          <Stat2 label="actions" v={mcp.actionCount}/>
          <Stat2 label="queries" v={mcp.queryCount}/>
        </div>
      }>

      {/* MCP chooser — primary axis */}
      <div className="py-3 px-12 gap-2 border-b bg-paper-2 flex items-center overflow-auto" >
        <span style={{
 fontSize: 11, letterSpacing: '0.16em' }} className="mr-2 text-ink-3 uppercase shrink-0" >
          MCP
        </span>
        <div className="gap-1 flex flex-wrap" >
          {I.mcps.map(m => (
            <MCPPill key={m.id} mcp={m} on={m.id === mcpId}
                     onClick={() => { setMcpId(m.id); setFocusId(null); setKind("all"); setQ(""); }}/>
          ))}
          <button style={{
 fontSize: 11, border: '1px dashed var(--edge)',
 borderRadius: 5 }} className="py-2 px-3 text-ink-3 bg-transparent cursor-pointer" >
            + Add MCP
          </button>
        </div>
      </div>

      {/* Kind + search strip */}
      <div className="py-2 px-12 gap-2 border-b flex items-center" >
        <KindChip kanji="全" label="All"     on={kind === "all"}
                   count={mcpTools.length}         onClick={() => setKind("all")}/>
        <KindChip kanji="作" label="Actions" on={kind === "action"}
                   count={mcpTools.filter(t => t.kind === "action").length}
                   onClick={() => setKind("action")} tone="shu"/>
        <KindChip kanji="問" label="Queries" on={kind === "query"}
                   count={mcpTools.filter(t => t.kind === "query").length}
                   onClick={() => setKind("query")} tone="matcha"/>
        <span className="flex-1" />
        <div style={{ borderRadius: 5, minWidth: 260
 }} className="gap-2 py-1 px-2 flex items-center bg-paper-2 border border-paper-edge" >
          <span className="kanji text-ink-3" style={{ fontSize: 11 }}>探</span>
          <input className="border-0 bg-transparent flex-1 text-ink" value={q} onChange={e => setQ(e.target.value)}
 placeholder={`search ${mcp.name.toLowerCase()} tools…`}
 style={{ outline: 'none',
 fontSize: 13 }}/>
          {q && (
            <button className="text-ink-4" onClick={() => setQ("")}
 style={{ fontSize: 11 }}>×</button>
          )}
        </div>
      </div>

      {/* Two-pane list + detail */}
      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '340px 1fr' }}>
        <aside className="overflow-auto border-r bg-paper-2" >
          {filtered.length === 0 ? (
            <div style={{
 fontSize: 13 }} className="py-6 px-4 text-center text-ink-4" >
              No tools match.
            </div>
          ) : (
            <div className="py-1 px-0" >
              {filtered.map(t => (
                <ToolRowFlat key={t.id} tool={t}
                             active={focus && focus.id === t.id}
                             onClick={() => setFocusId(t.id)}/>
              ))}
            </div>
          )}
        </aside>

        <main className="pt-6 pb-8 px-12 overflow-auto" >
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
    <span className="gap-1 inline-flex items-baseline" >
      <span className="mono text-ink" style={{ fontSize: 13,
 fontFeatureSettings: '"tnum"' }}>{v}</span>
      <span className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</span>
    </span>
  );
}

function MCPPill({ mcp, on, onClick }) {
  return (
    <button onClick={onClick}
 style={{ fontSize: 13, borderRadius: 5,
 background: on ? 'var(--paper)' : 'transparent',
 border: on ? '1px solid var(--ink-4)' : '1px solid transparent',
 color: on ? 'var(--ink)' : 'var(--ink-2)' }} className="gap-2 py-2 px-3 inline-flex items-center cursor-pointer whitespace-nowrap" >
      <span className="kanji" style={{ fontSize: 13,
                    color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{mcp.kanji}</span>
      <span>{mcp.name}</span>
      <span className="mono" style={{ fontSize: 11,
                    color: on ? 'var(--ink-3)' : 'var(--ink-4)' }}>
        {mcp.toolCount}
      </span>
      {!mcp.installed && (
        <span className="text-warning uppercase" style={{ fontSize: 11,
 letterSpacing: '0.12em' }}>
          not installed
        </span>
      )}
    </button>
  );
}

function KindChip({ kanji, label, on, count, onClick, tone }) {
  const toneColor = tone === "shu"    ? "var(--accent)" :
                    tone === "matcha" ? "var(--success)" : "var(--ink-3)";
  return (
    <button onClick={onClick}
 style={{ fontSize: 11, borderRadius: 4,
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : 'var(--ink-2)' }} className="gap-1 py-1 px-3 inline-flex items-center border-0 cursor-pointer" >
      <span className="kanji" style={{ fontSize: 11,
                    color: on ? 'var(--paper)' : toneColor }}>{kanji}</span>
      <span>{label}</span>
      <span className="mono" style={{ fontSize: 11,
                    color: on ? 'var(--paper)' : 'var(--ink-4)' }}>
        {count}
      </span>
    </button>
  );
}

function ToolRowFlat({ tool, active, onClick }) {
  const kindGlyph = tool.kind === "action" ? "作" : "問";
  const kindColor = tool.kind === "action" ? "var(--accent)" : "var(--success)";
  return (
    <button onClick={onClick}
 style={{ gridTemplateColumns: '14px 1fr',
 background: active ? 'var(--paper)' : 'transparent',
 borderLeft: active ? '2px solid var(--accent)' : '2px solid transparent' }} className="gap-2 py-3 px-4 grid w-full text-left border-0 cursor-pointer" >
      <span className="kanji mt-1" style={{ fontSize: 13, color: kindColor }}>
        {kindGlyph}
      </span>
      <div>
        <div className="mono overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11,
 color: active ? 'var(--ink)' : 'var(--ink-2)' }}>
          {tool.name}
        </div>
        <div style={{
 fontSize: 11,
 lineHeight: 1.4, display: '-webkit-box',
 WebkitLineClamp: 2, WebkitBoxOrient: 'vertical'
 }} className="mt-1 text-ink-3 overflow-hidden" >
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
    ? { label: "action", color: "var(--accent)",    glyph: "作",
        hint: "performs an operation" }
    : { label: "query",  color: "var(--success)", glyph: "問",
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
      <div className="gap-3 mb-4 flex items-start" >
        <div className="flex-1" >
          <div className="gap-2 mb-1 flex items-center" >
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {mcp.kanji} {mcp.name.toLowerCase()}
            </span>
            <span className="text-ink-4" style={{ fontSize: 11 }}>·</span>
            <span style={{ fontSize: 11,
 borderRadius: 3, color: kindBadge.color,
 letterSpacing: '0.12em' }} className="gap-1 py-1 px-2 inline-flex items-center bg-paper-2 border border-paper-edge uppercase" >
              <span className="kanji" style={{ fontSize: 11 }}>{kindBadge.glyph}</span>
              {kindBadge.label}
            </span>
          </div>
          <h2 className="mono m-0 text-ink font-normal" style={{
 fontSize: 17 }}>
            {tool.name}
          </h2>
          <p style={{
 fontSize: 13,
 lineHeight: 1.55, maxWidth: 720
 }} className="mt-2 mb-0 text-ink-2" >
            {tool.summary}
          </p>
        </div>
      </div>

      {/* Inputs form */}
      <div style={{
 borderRadius: 7
 }} className="py-4 px-4 mb-4 bg-paper-2 border border-paper-edge" >
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
          Inputs
        </div>
        {(tool.inputs || []).length === 0 && (
          <div className="text-ink-3" style={{ fontSize: 13 }}>
            No inputs — just call it.
          </div>
        )}
        <div className="grid" style={{ gridTemplateColumns: 'repeat(2, 1fr)',
 gap: '12px 16px' }}>
          {(tool.inputs || []).map(i => (
            <InputRow key={i.key} input={i}
                      value={values[i.key]}
                      onChange={v => setValues(s => ({ ...s, [i.key]: v }))}/>
          ))}
        </div>

        <div className="gap-2 mt-3 pt-3 flex items-center border-t" >
          <button onClick={runExample}
 style={{
 fontSize: 13, borderRadius: 5, letterSpacing: '0.04em'
 }} className="py-2 px-3 bg-ink text-paper border-0 cursor-pointer" >
            {tool.kind === "action" ? "Run →" : "Query →"}
          </button>
          <span className="text-ink-4" style={{ fontSize: 11 }}>
            {kindBadge.hint}
          </span>
          <span className="flex-1" />
          {status === "running" && (
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              calling …
            </span>
          )}
          {status === "done" && (
            <span className="mono text-success" style={{ fontSize: 11 }}>
              200 ok
            </span>
          )}
        </div>
      </div>

      {/* Response */}
      <div>
        <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
          Response {status === "idle" && "· preview"}
        </div>
        <pre style={{
 fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.6,
 borderLeft: `2px solid ${kindBadge.color}`,
 borderRadius: 5,
 whiteSpace: 'pre-wrap',
 maxHeight: 380, opacity: status === "idle" ? 0.68 : 1
 }} className="py-3 px-4 m-0 bg-paper-2 border border-paper-edge text-ink-2 overflow-auto" >
{status === "idle" ? tool.example?.response || "—" : response}
        </pre>
        {status === "idle" && (
          <div style={{
 fontSize: 11 }} className="mt-1 text-ink-4 italic" >
            Example response. Click {tool.kind === "action" ? "Run" : "Query"} to invoke.
          </div>
        )}
      </div>
    </div>
  );
}

function InputRow({ input, value, onChange }) {
  const labelBlock = (
    <label style={{
 fontSize: 11 }} className="gap-1 text-ink-2 flex items-baseline" >
      <span>{input.label}</span>
      {input.required && <span className="text-accent" >*</span>}
      <span className="mono text-ink-4" style={{ fontSize: 11 }}>
        {input.kind}
      </span>
    </label>
  );

  let control;
  if (input.kind === "enum" || input.kind === "since") {
    control = (
      <select value={value ?? ""} onChange={e => onChange(e.target.value)}
              style={fieldStyle} className="gap-1" >
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
    <div className="flex flex-col" >
      {labelBlock}
      {control}
      {input.help && (
        <span className="text-ink-4" style={{ fontSize: 11 }}>{input.help}</span>
      )}
    </div>
  );
}

const fieldStyle = {
  padding: '8px 8px', fontSize: 13,
  border: '1px solid var(--edge)', borderRadius: 4,
  background: 'var(--paper)', color: 'var(--ink)',
  fontFamily: 'var(--font-mono)',
  outline: 'none'
};

function EmptyFocus() {
  return (
    <div style={{ fontSize: 13 }} className="p-8 text-ink-4 text-center" >
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
                              embedded = false, simple = false, subNav } = {}) {
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
      subNav={subNav}
      kanji="録"
      tagline="Every instrument call, in order."
      sub="Step through the tools the assistant reached for during a session. Pure request + response — what was asked, what came back, how long it took."
      chip={
        <span className="mono py-1 px-2 text-ink-3 border border-paper-edge" style={{
 fontSize: 11, borderRadius: 3
 }}>
          {sessionIds.length} sessions indexed
        </span>
      }>
      <div className="flex-1 grid min-h-0 overflow-hidden" style={{ gridTemplateColumns: '300px 1fr' }}>

        {/* Session picker */}
        <aside className="overflow-auto border-r bg-paper-2" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="pt-3 pb-2 px-3 text-ink-3 uppercase" >
            sessions
          </div>
          {sessionIds.map(sid => {
            const m = all.find(s => s.id === sid) || {};
            const sg = signals[sid];
            const on = pickedId === sid;
            return (
              <button key={sid}
 onClick={() => { setPickedId(sid); setFocusCall(1); }}
 style={{
 background: on ? 'var(--paper)' : 'transparent',
 borderLeft: on ? '2px solid var(--accent)' : '2px solid transparent' }} className="py-2 px-3 block w-full text-left border-0 cursor-pointer" >
                <div className="gap-1 flex items-baseline" >
                  <span className="mono" style={{ fontSize: 11,
                                color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{sid}</span>
                  <span className="uppercase" style={{ fontSize: 11,
 color: m.ftr ? 'var(--success)' : 'var(--warning)',
 letterSpacing: '0.12em' }}>
                    {m.ftr ? "ftr" : `${m.corrections}c`}
                  </span>
                </div>
                <div style={{
 fontSize: 11,
 lineHeight: 1.4, display: '-webkit-box',
 WebkitLineClamp: 2, WebkitBoxOrient: 'vertical'
 }} className="mt-1 text-ink-2 overflow-hidden" >
                  {sg.title}
                </div>
                <div style={{ fontSize: 11 }} className="mt-1 text-ink-4" >
                  {m.project} · {sg.toolCallCount} calls · {m.duration || "–"}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main className="pt-6 pb-8 px-8 overflow-auto" >
          {/* Session summary strip */}
          <div style={{
 gridTemplateColumns: 'auto auto auto auto 1fr' }} className="gap-6 mb-4 pb-3 grid items-baseline border-b" >
            <div>
              <div className="display mb-1" style={{ fontSize: 15 }}>{sess.title}</div>
              <div className="mono text-ink-3" style={{ fontSize: 11 }}>{pickedId}</div>
            </div>
            <StatR label="turns"      value={sess.totalTurns}/>
            <StatR label="tool calls" value={sess.toolCallCount}/>
            <StatR label="ftr"        value={sess.ftr ? "yes" : "no"}
                   tone={sess.ftr ? "good" : "warn"}/>
            <span/>
          </div>

          {/* Timeline + detail */}
          <div style={{ gridTemplateColumns: '0.95fr 1.4fr' }} className="gap-6 grid" >
            {/* Left: timeline */}
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
                timeline ({sess.calls.length})
              </div>
              <div className="gap-0 flex flex-col" >
                {sess.calls.map(c => {
                  const on = focusCall === c.i;
                  const kindFromName = inferKind(c.tool);
                  const dot = kindFromName === "action" ? "var(--accent)" : "var(--success)";
                  return (
                    <button key={c.i} onClick={() => setFocusCall(c.i)}
 style={{
 gridTemplateColumns: '22px 38px 1fr auto', borderRadius: 5,
 background: on ? 'var(--paper-2)' : 'transparent',
 border: on ? '1px solid var(--edge)' : '1px solid transparent' }} className="gap-2 py-2 pl-3 pr-2 grid items-center text-left cursor-pointer" >
                      <span className="mono text-ink-3" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>{c.i}</span>
                      <span className="mono text-ink-3" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>{c.t}</span>
                      <span className="mono text-ink overflow-hidden text-ellipsis whitespace-nowrap" style={{ fontSize: 11 }}>
                        {shortName(c.tool)}
                      </span>
                      <span className="gap-1 flex items-center" >
                        <span className="rounded-full" style={{ width: 7, height: 7, background: dot }}/>
                        <span className="mono text-ink-4" style={{ fontSize: 11,
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
    ? { label: "action", color: "var(--accent)",    glyph: "作" }
    : { label: "query",  color: "var(--success)", glyph: "問" };
  return (
    <div className="gap-3 flex flex-col" >
      <div className="pb-3 flex items-baseline justify-between border-b" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
            call #{call.i} · turn {call.turn} · {call.t}
          </div>
          <div className="mono text-ink" style={{ fontSize: 13 }}>
            {call.tool}
          </div>
        </div>
        <span style={{ fontSize: 11,
 borderRadius: 3, color: kindBadge.color,
 letterSpacing: '0.1em' }} className="gap-1 py-1 px-2 inline-flex items-center bg-paper-2 border border-paper-edge uppercase" >
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
        <div className="text-ink" style={{ ...preStyleR, borderLeft: `2px solid ${kindBadge.color}` }}>
          {call.responseSnippet}
        </div>
        <div style={{
 fontSize: 11 }} className="mt-1 text-ink-4 italic" >
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
      <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >{label}</div>
      {children}
    </div>
  );
}

const preStyleR = {
  margin: 0, padding: '12px 12px',
  fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
  background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 5,
  color: 'var(--ink-2)', whiteSpace: 'pre-wrap', overflow: 'auto'
};

function StatR({ label, value, tone }) {
  const color = tone === "good" ? "var(--success)" :
                tone === "warn" ? "var(--warning)" : "var(--ink)";
  return (
    <div>
      <div className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</div>
      <div className="display mt-1" style={{
 fontSize: 15, color,
                    fontFeatureSettings: '"tnum"'
}}>{value}</div>
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
function InstrumentsHealth({ activeTab = "health", onTab = () => {},
                              embedded = false, simple = false, subNav } = {}) {
  const I = window.MCP_SIGNALS.insights;
  const [window_, setWindow] = iUseS(I.window);
  const [focusTool, setFocusTool] = iUseS(null);
  const [activeMcp, setActiveMcp] = iUseS(null);   // null = MCP overview, else drill-down

  // Assemble the per-MCP rollup from meta + Sensei tools + third-party tools.
  const senseiTools = I.toolUsage.map(t => ({ ...t, mcp: t.mcp || "sensei" }));
  const allTools = [...senseiTools, ...(I.thirdPartyUsage || [])];
  const mcpRows = I.mcpMeta.map(m => {
    const tools = allTools.filter(t => t.mcp === m.id)
                          .slice().sort((a, b) => b.calls - a.calls);
    const invoked = tools.filter(t => t.calls > 0).length;
    const calls = tools.reduce((s, t) => s + t.calls, 0);
    const warn = tools.filter(t => t.verdict === "warn").length;
    const dormant = tools.filter(t => t.calls === 0).length;
    const wsum = calls || 1;
    const ftrDelta = tools.reduce((s, t) => s + t.ftrDelta * t.calls, 0) / wsum;
    return {
      ...m, tools, toolsTotal: tools.length, invoked, calls, warn, dormant, ftrDelta,
      coverage: tools.length ? invoked / tools.length : 0
    };
  });
  const drill = activeMcp ? mcpRows.find(m => m.id === activeMcp) : null;

  const enterMcp = (id) => { setActiveMcp(id); setFocusTool(null); };
  const exitMcp = () => { setActiveMcp(null); setFocusTool(null); };

  return (
    <InstrumentsShell activeTab={activeTab} onTab={onTab} embedded={embedded} simple={simple}
      subNav={subNav}
      kanji="健"
      tagline={drill
        ? `${drill.name} · which tools earn their keep`
        : "Which instruments earn their keep — and what to change."}
      sub={drill
        ? `${drill.invoked} of ${drill.toolsTotal} tools invoked this window · ${drill.calls.toLocaleString()} calls. Sorted by how often each is called; the split shows whether the assistant acted on the response.`
        : "Start at the server level: what share of each MCP's registered tools is the assistant actually invoking? Coverage alone isn't success — the signal is whether the tools that get called change what the assistant does. Open a server to see the per-tool breakdown."}
      chip={
        <div style={{ borderRadius: 5 }} className="gap-1 p-1 flex bg-paper-2 border border-paper-edge" >
          {["7d", "30d", "90d"].map(w => (
            <button key={w} onClick={() => setWindow(w)}
 style={{
 fontSize: 11, borderRadius: 3,
 background: window_ === w ? 'var(--paper)' : 'transparent',
 color: window_ === w ? 'var(--ink)' : 'var(--ink-3)' }} className="py-1 px-2 border-0 cursor-pointer" >{w}</button>
          ))}
        </div>
      }>

      <main className="pt-6 pb-8 px-12 overflow-auto" >
        {drill
          ? <McpDrilldown m={drill} focusTool={focusTool} setFocusTool={setFocusTool}
                          onBack={exitMcp}/>
          : <McpOverview I={I} mcpRows={mcpRows} onOpen={enterMcp}/>}
      </main>
    </InstrumentsShell>
  );
}

// ─── Level 1 — the MCP overview ────────────────────────────────────────
function McpOverview({ I, mcpRows, onOpen }) {
  const connected = mcpRows.filter(m => m.connected);
  const totalTools = connected.reduce((s, m) => s + m.toolsTotal, 0);
  const totalInvoked = connected.reduce((s, m) => s + m.invoked, 0);
  const overallCoverage = totalTools ? Math.round((totalInvoked / totalTools) * 100) : 0;

  // Signals split: warn/opportunity/win stay as their own actionable cards;
  // every dormant tool collapses into one summary card so the list stays short.
  const actionable = I.signals.filter(s => s.kind !== "unused");
  const dormantTools = connected
    .flatMap(m => m.tools.filter(t => t.calls === 0).map(t => shortName(t.tool)));

  return (
    <>
      <div style={{ gridTemplateColumns: 'repeat(4, 1fr)'
 }} className="gap-3 mb-6 grid" >
        <Kpi kanji="接" label="servers connected"
             value={`${connected.length} of ${mcpRows.length}`}
             hint={`${mcpRows.length - connected.length} available`}/>
        <Kpi kanji="具" label="tool coverage"
             value={`${overallCoverage}%`}
             hint={`${totalInvoked} of ${totalTools} invoked`}/>
        <Kpi kanji="計" label="total tool calls"
             value={I.deltas.totalCalls.toLocaleString()}
             delta={`${I.deltas.totalCallsTrend > 0 ? "+" : ""}${Math.round(I.deltas.totalCallsTrend * 100)}%`}
             deltaTone={I.deltas.totalCallsTrend > 0 ? "good" : "warn"}/>
        <Kpi kanji="一" label="first-try rate"
             value={`${Math.round(I.deltas.ftrThisWindow * 100)}%`}
             delta={`${I.deltas.ftrTrend > 0 ? "+" : ""}${Math.round(I.deltas.ftrTrend * 100)} pts`}
             deltaTone={I.deltas.ftrTrend > 0 ? "good" : "warn"}/>
      </div>

      <div className="mb-6" >
        <div className="mb-3 flex items-baseline justify-between" >
          <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
            Servers
            <span style={{ fontSize: 13 }} className="ml-2 text-ink-3" >
              · what share of each server's tools is in use
            </span>
          </h3>
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            open a server to drill in →
          </span>
        </div>
        <div style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))'
 }} className="gap-3 grid" >
          {mcpRows.map(m => <McpCard key={m.id} m={m} onOpen={onOpen}/>)}
        </div>
      </div>

      <div className="mb-6" >
        <div className="mb-3 flex items-baseline justify-between" >
          <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
            Signals
            <span style={{ fontSize: 13 }} className="ml-2 text-ink-3" >
              · what the data suggests you change
            </span>
          </h3>
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            {actionable.length} to act on
          </span>
        </div>
        <div style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))'
 }} className="gap-3 grid" >
          {actionable.map((s, i) => <SignalCard key={i} s={s}/>)}
          {dormantTools.length > 0 && <DormantSummary tools={dormantTools}/>}
        </div>
      </div>
    </>
  );
}

// One server card — headline is the % of its tools being invoked.
function McpCard({ m, onOpen }) {
  const pct = Math.round(m.coverage * 100);
  const disabled = !m.connected;
  return (
    <button onClick={() => !disabled && onOpen(m.id)}
 disabled={disabled}
 style={{ borderRadius: 8,
 cursor: disabled ? 'default' : 'pointer', opacity: disabled ? 0.55 : 1,
 transition: 'background 0.12s'
 }}
 onMouseEnter={(e) => { if (!disabled) e.currentTarget.style.background = 'var(--paper-3)'; }}
 onMouseLeave={(e) => { if (!disabled) e.currentTarget.style.background = 'var(--paper-2)'; }}
 className="py-3 px-4 gap-3 w-full text-left bg-paper-2 border border-paper-edge flex flex-col" >
      <div className="gap-2 flex items-start" >
        <span className="kanji text-accent text-center shrink-0" style={{ fontSize: 22, lineHeight: 1,
 width: 26 }}>{m.kanji}</span>
        <div className="flex-1 min-w-0" >
          <div className="text-ink" style={{ fontSize: 15 }}>{m.name}</div>
          <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
            {m.publisher} · {m.toolsTotal} tools
          </div>
        </div>
        {disabled && (
          <span className="mono text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.12em' }}>off</span>
        )}
      </div>

      {m.connected ? (
        <>
          <div className="gap-2 flex items-baseline" >
            <span className="display font-normal" style={{ fontSize: 28,
 color: pct >= 70 ? 'var(--ink)' : 'var(--warning)',
 fontFeatureSettings: '"tnum"' }}>{pct}%</span>
            <span className="text-ink-3" style={{ fontSize: 11 }}>
              of tools invoked · {m.invoked}/{m.toolsTotal}
            </span>
          </div>
          <CoverageBar invoked={m.invoked} total={m.toolsTotal}/>
          <div style={{ fontFeatureSettings: '"tnum"' }}
 className="gap-3 pt-2 mono flex border-t text-ink-3" >
            <span style={{ fontSize: 11 }}>{m.calls.toLocaleString()} calls</span>
            <span style={{ fontSize: 11, color: m.ftrDelta >= 0 ? 'var(--success)' : 'var(--warning)' }}>
              {m.ftrDelta >= 0 ? "+" : ""}{Math.round(m.ftrDelta * 100)} pts ftr
            </span>
            {m.warn > 0 && (
              <span className="text-warning" style={{ fontSize: 11 }}>{m.warn} warn</span>
            )}
            {m.dormant > 0 && (
              <span className="text-ink-4" style={{ fontSize: 11 }}>{m.dormant} dormant</span>
            )}
          </div>
        </>
      ) : (
        <div style={{ fontSize: 11, lineHeight: 1.5 }} className="pt-2 text-ink-3 border-t" >
          {m.note}
        </div>
      )}
    </button>
  );
}

// The invoked/idle coverage bar for a server card.
function CoverageBar({ invoked, total }) {
  return (
    <div className="flex overflow-hidden bg-paper-3" style={{ height: 6, borderRadius: 3, gap: 2 }}>
      {Array.from({ length: total }).map((_, i) => (
        <div className="flex-1" key={i} style={{
 background: i < invoked ? 'var(--accent)' : 'var(--paper-3)',
 opacity: i < invoked ? 1 : 0.5 }}/>
      ))}
    </div>
  );
}

// ─── Level 2 — the per-tool drill-down for one server ──────────────────
function McpDrilldown({ m, focusTool, setFocusTool, onBack }) {
  const maxCalls = Math.max(1, ...m.tools.map(t => t.calls));
  return (
    <>
      <button onClick={onBack}
 style={{
 fontSize: 13 }} className="p-0 mb-4 bg-transparent border-0 cursor-pointer text-ink-3" >
        ← all servers
      </button>

      <div style={{ gridTemplateColumns: '1.5fr 1fr' }} className="gap-6 grid" >
        {/* Calls-per-tool chart — which tools are called more often */}
        <div>
          <div className="mb-3 flex items-baseline justify-between" >
            <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
              Calls per tool
            </h3>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {m.calls.toLocaleString()} calls · this window
            </span>
          </div>
          <div style={{ borderRadius: 7 }} className="py-3 px-4 border border-paper-edge bg-paper" >
            <div className="gap-3 flex flex-col" >
              {m.tools.map(t => (
                <CallsBar key={t.tool} t={t} max={maxCalls}
                          active={focusTool === t.tool}
                          onFocus={() => setFocusTool(focusTool === t.tool ? null : t.tool)}/>
              ))}
            </div>
          </div>
        </div>

        {/* Server summary rail */}
        <div>
          <h3 className="display mt-0 mb-3 font-normal" style={{ fontSize: 15 }}>
            {m.name}
          </h3>
          <div style={{
 borderRadius: 7 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
            <div className="gap-2 mb-2 flex items-baseline" >
              <span className="display font-light text-ink" style={{ fontSize: 40, fontFeatureSettings: '"tnum"' }}>
                {Math.round(m.coverage * 100)}%
              </span>
              <span className="text-ink-3" style={{ fontSize: 11 }}>
                of tools invoked<br/>{m.invoked} of {m.toolsTotal}
              </span>
            </div>
            <CoverageBar invoked={m.invoked} total={m.toolsTotal}/>
            <div className="gap-1 mt-3 pt-3 border-t flex flex-col" >
              <SummaryLine label="tool calls" value={m.calls.toLocaleString()}/>
              <SummaryLine label="ftr delta"
                           value={`${m.ftrDelta >= 0 ? "+" : ""}${Math.round(m.ftrDelta * 100)} pts`}
                           tone={m.ftrDelta >= 0 ? 'var(--success)' : 'var(--warning)'}/>
              {m.warn > 0 && <SummaryLine label="warnings" value={m.warn} tone="var(--warning)"/>}
              {m.dormant > 0 && <SummaryLine label="dormant tools" value={m.dormant} tone="var(--ink-3)"/>}
            </div>
          </div>
          <div style={{ fontSize: 11, lineHeight: 1.6 }} className="mt-3 text-ink-2" >
            Click a bar to read why a tool looks healthy or off. The split under each bar
            is how often the assistant used, half-used, or ignored the response.
          </div>
        </div>
      </div>

      {/* Full per-tool table for this server */}
      <div className="mt-8" >
        <div className="mb-3 flex items-baseline justify-between" >
          <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
            Per-tool detail
          </h3>
          <span className="mono text-ink-3" style={{ fontSize: 11 }}>
            {m.tools.length} tools · sorted by calls
          </span>
        </div>
        <div className="border border-paper-edge overflow-hidden bg-paper" style={{ borderRadius: 7 }}>
          <ToolRowHeader/>
          {m.tools.map((t, idx) => (
            <ToolUsageRow key={t.tool} t={t}
                          focus={focusTool === t.tool}
                          onFocus={() => setFocusTool(focusTool === t.tool ? null : t.tool)}
                          last={idx === m.tools.length - 1}/>
          ))}
        </div>
      </div>
    </>
  );
}

// A single horizontal bar in the calls-per-tool chart.
function CallsBar({ t, max, active, onFocus }) {
  const pct = Math.round((t.calls / max) * 100);
  const dormant = t.calls === 0;
  const barColor = t.verdict === "warn" ? 'var(--warning)'
                 : dormant ? 'var(--paper-3)' : 'var(--accent)';
  return (
    <button onClick={onFocus} className="p-0 gap-1 w-full text-left bg-transparent border-0 cursor-pointer flex flex-col" >
      <div className="flex items-baseline justify-between" >
        <span className="mono" style={{ fontSize: 11,
                      color: dormant ? 'var(--ink-4)' : 'var(--ink)' }}>
          {shortName(t.tool)}
        </span>
        <span className="mono text-ink-3" style={{ fontSize: 11,
 fontFeatureSettings: '"tnum"' }}>{t.calls}</span>
      </div>
      <div className="bg-paper-3 overflow-hidden" style={{ height: 10, borderRadius: 3, outline: active ? '1px solid var(--accent)' : 'none',
 outlineOffset: 1 }}>
        <div className="h-full" style={{ width: `${Math.max(pct, dormant ? 0 : 3)}%`,
 background: barColor, transition: 'width 0.18s' }}/>
      </div>
      {active && (
        <div style={{ fontSize: 11, lineHeight: 1.5 }} className="mt-1 text-ink-2" >
          <span className="text-success" >{t.usedPct}% used</span>
          {t.partialPct > 0 && <span> · {t.partialPct}% partial</span>}
          {t.ignoredPct > 0 && <span className="text-accent" > · {t.ignoredPct}% ignored</span>}
          {t.note && <span> — {t.note}</span>}
        </div>
      )}
    </button>
  );
}

function SummaryLine({ label, value, tone = 'var(--ink)' }) {
  return (
    <div className="flex justify-between items-baseline" >
      <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.12em' }}>{label}</span>
      <span className="mono" style={{ fontSize: 13, color: tone,
                    fontFeatureSettings: '"tnum"' }}>{value}</span>
    </div>
  );
}

// Insights helpers — reused from before
function Kpi({ kanji, label, value, delta, deltaTone, hint, tone }) {
  const valueColor = tone === "warn" ? "var(--warning)" : "var(--ink)";
  return (
    <div style={{ borderRadius: 7
 }} className="py-3 px-3 bg-paper-2 border border-paper-edge" >
      <div className="gap-1 mb-1 flex items-center" >
        <span className="kanji text-accent" style={{ fontSize: 13 }}>{kanji}</span>
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</span>
      </div>
      <div className="display" style={{ fontSize: 22, color: valueColor,
                    fontFeatureSettings: '"tnum"' }}>{value}</div>
      <div style={{ fontSize: 11 }} className="mt-1 text-ink-3" >
        {delta && (
          <span style={{
 color: deltaTone === "good" ? "var(--success)" :
                                deltaTone === "warn" ? "var(--accent)" : "var(--ink-3)"
}} className="mr-1" >
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
    warn:        { border: "var(--warning)",  tintBg: "var(--warning-soft)",  label: "warn"  },
    opportunity: { border: "var(--success)", tintBg: "var(--success-soft)", label: "lift" },
    unused:      { border: "var(--ink-3)", tintBg: "var(--paper-2)",         label: "quiet" },
    win:         { border: "var(--accent)",    tintBg: "var(--accent-soft)",   label: "win"   }
  };
  const p = palette[s.kind] || palette.warn;
  return (
    <div style={{
 background: p.tintBg, borderLeft: `3px solid ${p.border}`,
 borderRadius: 5
 }} className="py-3 px-4 border border-paper-edge" >
      <div className="gap-2 mb-1 flex items-baseline" >
        <span className="kanji" style={{ fontSize: 13, color: p.border }}>{s.kanji}</span>
        <span className="uppercase" style={{ fontSize: 11, letterSpacing: '0.16em', color: p.border }}>{p.label}</span>
      </div>
      <div className="display mb-1" style={{ fontSize: 13 }}>{s.title}</div>
      <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
        {s.body}
      </div>
      <div className="mt-2 pt-2 border-t" >
        <button style={{
 fontSize: 11, color: p.border,
 letterSpacing: '0.04em'
 }} className="p-0 bg-transparent border-0 cursor-pointer" >
          {s.action} →
        </button>
      </div>
    </div>
  );
}

// One dormant-tools summary — collapses every 0-call tool into a single
// card with the first three names and an "N more" tail, so the actionable
// signals aren't buried under a wall of never-called tools.
function DormantSummary({ tools }) {
  const shown = tools.slice(0, 3);
  const rest = tools.length - shown.length;
  return (
    <div style={{ borderLeft: '3px solid var(--ink-3)',
 borderRadius: 5
 }} className="py-3 px-4 bg-paper-2 border border-paper-edge" >
      <div className="gap-2 mb-1 flex items-baseline" >
        <span className="kanji text-ink-3" style={{ fontSize: 13 }}>眠</span>
        <span className="text-ink-3 uppercase" style={{ fontSize: 11, letterSpacing: '0.16em' }}>dormant</span>
      </div>
      <div className="display mb-1" style={{ fontSize: 13 }}>
        {tools.length} tool{tools.length !== 1 ? "s" : ""} dormant
      </div>
      <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
        Registered but never called this window — likely undiscoverable or contradicted
        by a skill.
      </div>
      <div className="mono mt-2 text-ink-3" style={{ fontSize: 11,
 lineHeight: 1.6 }}>
        {shown.join(" · ")}{rest > 0 && <span className="text-ink-4" > · {rest} more</span>}
      </div>
      <div className="mt-2 pt-2 border-t" >
        <button style={{
 fontSize: 11,
 letterSpacing: '0.04em'
 }} className="p-0 text-ink-3 bg-transparent border-0 cursor-pointer" >
          Review dormant tools →
        </button>
      </div>
    </div>
  );
}

function ToolRowHeader() {
  return (
    <div style={{
 gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px' }} className="gap-3 py-2 px-3 grid bg-paper-2 border-b" >
      {["tool", "calls", "trend 14d", "usage split", "ftr Δ"].map(h => (
        <div className="text-ink-3 uppercase" key={h} style={{ fontSize: 11, letterSpacing: '0.14em' }}>{h}</div>
      ))}
    </div>
  );
}

function ToolUsageRow({ t, focus, onFocus, last }) {
  const verdict = {
    healthy:   { color: "var(--success)",  glyph: "✓" },
    ok:        { color: "var(--ink-3)",  glyph: "·" },
    warn:      { color: "var(--warning)",   glyph: "!" },
    underused: { color: "var(--ink-3)",  glyph: "·" },
    unused:    { color: "var(--accent)",     glyph: "○" }
  }[t.verdict] || { color: "var(--ink-3)", glyph: "·" };

  return (
    <div>
      <button onClick={onFocus}
 style={{
 gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px',
 background: focus ? 'var(--paper-2)' : 'transparent',
 borderBottom: last && !focus ? 'none' : 'var(--hairline)' }} className="gap-3 py-2 px-3 w-full grid border-0 text-left cursor-pointer items-center" >
        <div>
          <div className="gap-2 flex items-baseline" >
            <span className="kanji" style={{ fontSize: 11, color: verdict.color }}>
              {verdict.glyph}
            </span>
            <span className="mono text-ink" style={{ fontSize: 11 }}>
              {shortName(t.tool)}
            </span>
          </div>
        </div>
        <div className="mono text-ink-2" style={{ fontSize: 13,
 fontFeatureSettings: '"tnum"' }}>
          {t.calls}
        </div>
        <Sparkline data={t.trend}/>
        <UsageBar used={t.usedPct} partial={t.partialPct} ignored={t.ignoredPct}/>
        <div className="mono" style={{ fontSize: 11,
                      color: t.ftrDelta > 0 ? "var(--success)" :
                             t.ftrDelta < 0 ? "var(--warning)" : "var(--ink-3)",
                      fontFeatureSettings: '"tnum"' }}>
          {t.ftrDelta > 0 ? "+" : ""}{Math.round(t.ftrDelta * 100)} pts
        </div>
      </button>
      {focus && (
        <div style={{
 borderBottom: last ? 'none' : 'var(--hairline)' }} className="pt-2 pb-3 pl-12 pr-3 bg-paper-2" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
            verdict · {t.verdict}
          </div>
          <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.55 }}>
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
    <svg className="block" width={w} height={h} >
      <polyline points={pts} fill="none"
                stroke="var(--ink-3)" strokeWidth="1.3"/>
      <circle cx={w} cy={h - (data[data.length - 1] / max) * (h - 2) - 1}
              r="2" fill="var(--accent)"/>
    </svg>
  );
}

function UsageBar({ used, partial, ignored }) {
  return (
    <div>
      <div className="flex overflow-hidden bg-paper-3" style={{ height: 8, borderRadius: 2 }}>
        <div className="bg-success" style={{ width: `${used}%` }}/>
        <div className="bg-warning" style={{ width: `${partial}%` }}/>
        <div className="bg-accent" style={{ width: `${ignored}%` }}/>
      </div>
      <div style={{
 fontSize: 11,
 fontFeatureSettings: '"tnum"'
 }} className="gap-1 mt-1 flex text-ink-3" >
        <span>{used}% used</span>
        {ignored > 0 && <span className="text-accent" >{ignored}% ignored</span>}
      </div>
    </div>
  );
}

function ProjectUsageRow({ p }) {
  return (
    <div style={{ borderRadius: 5, gridTemplateColumns: '1fr auto' }} className="py-2 px-3 gap-2 bg-paper-2 border border-paper-edge grid items-baseline" >
      <div>
        <div className="text-ink" style={{ fontSize: 13 }}>{p.project}</div>
        <div className="mono mt-1 text-ink-3" style={{ fontSize: 11 }}>
          {p.sessions} sessions · {p.toolCalls} calls · top: {shortName(p.topTool)}
        </div>
      </div>
      <div className="text-right" >
        <div className="display" style={{ fontSize: 15,
                      color: p.ftr >= 0.7 ? 'var(--success)' :
                             p.ftr >= 0.5 ? 'var(--ink)' : 'var(--warning)',
                      fontFeatureSettings: '"tnum"' }}>
          {Math.round(p.ftr * 100)}%
        </div>
        <div className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>ftr</div>
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
  if (tab === "replay") return <InstrumentsReplay {...props}/>;
  if (tab === "health") return <InstrumentsHealth {...props}/>;
  return <InstrumentsPlayground {...props}/>;
}

Object.assign(window, {
  InstrumentsShell, InstrumentsPlayground, InstrumentsReplay, InstrumentsHealth,
  // back-compat alias — some older artboards still reference InstrumentsInsights
  InstrumentsInsights: InstrumentsHealth,
  InstrumentsApp
});
