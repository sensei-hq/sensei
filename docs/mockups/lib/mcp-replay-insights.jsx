// MCP Replay & Insights — the second and third of the three MCP capabilities.
//
// Sits alongside MCPPlayground (lib/libraries.jsx). Each renders inside a
// shared MCPShell that provides the chrome, hero, and tab nav.
//
//   Playground → what CAN these tools do?
//   Replay     → what DID the assistant do?
//   Insights   → what SHOULD we change?

const { useState: mrUseS, useMemo: mrUseM } = React;

// ═════════════════════════════════════════════════════════════
// Shared shell — chrome + hero + tab nav
// The three views pass their own body + hero copy.
// ═════════════════════════════════════════════════════════════
function MCPShell({ activeTab, onTab, kanji, title, tagline, chip, sub, children }) {
  const tabs = [
    { id: "playground", kanji: "具", label: "Playground",
      hint: "what can these tools do?" },
    { id: "replay",     kanji: "録", label: "Replay",
      hint: "what did the assistant do?" },
    { id: "insights",   kanji: "健", label: "Health",
      hint: "what should we change?" }
  ];
  return (
    <div className="sensei" data-screen-label={`MCP · ${title}`}
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title={`Sensei  先生  ·  mcp · ${activeTab}`}/>

      {/* Hero */}
      <div style={{
 display: 'flex',
                     alignItems: 'flex-end', borderBottom: 'var(--hairline)'
}} className="gap-4 pt-5 pb-4 px-7" >
        <div className="kanji" style={{ fontSize: 40, color: 'var(--accent)', lineHeight: 1 }}>
          {kanji}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{
 fontSize: 11, letterSpacing: '0.18em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            MCP · {title}
          </div>
          <h1 className="display m-0" style={{ fontSize: 22, fontWeight: 400 }}>
            {tagline}
          </h1>
          {sub && (
            <p style={{
 fontSize: 13, color: 'var(--ink-2)',
                         maxWidth: 680, lineHeight: 1.55
}} className="mt-1 mb-0" >
              {sub}
            </p>
          )}
        </div>
        {chip}
      </div>

      {/* Tab nav */}
      <div style={{
 borderBottom: 'var(--hairline)',
                     display: 'flex', background: 'var(--paper)'
}} className="px-7 gap-0" >
        {tabs.map(t => {
          const on = t.id === activeTab;
          return (
            <button key={t.id} onClick={() => onTab && onTab(t.id)}
                    style={{
                              display: 'flex', alignItems: 'center',
                              background: 'transparent', border: 'none',
                              borderBottom: on ? '2px solid var(--ink)' : '2px solid transparent',
                              marginBottom: -1,
                              color: on ? 'var(--ink)' : 'var(--ink-3)',
                              cursor: 'pointer'
}} className="gap-2 py-3 px-4" >
              <span className="kanji" style={{ fontSize: 15,
                            color: on ? 'var(--accent)' : 'var(--ink-3)' }}>{t.kanji}</span>
              <span className="display" style={{ fontSize: 13 }}>{t.label}</span>
              <span style={{ fontSize: 11, color: 'var(--ink-4)' }}>· {t.hint}</span>
            </button>
          );
        })}
      </div>

      <div style={{ flex: 1, minHeight: 0, overflow: 'hidden', display: 'flex',
                     flexDirection: 'column' }}>
        {children}
      </div>
    </div>
  );
}

// ═════════════════════════════════════════════════════════════
// REPLAY — per-session tool-call timeline
// Left: session picker (scrollable). Right: ordered list of tool
// calls for the selected session, each showing request, response
// excerpt, duration, and whether the assistant used the result.
// ═════════════════════════════════════════════════════════════
function MCPReplay({ onTab = () => {} }) {
  const all = window.SENSEI_DATA.sessions;
  const signals = window.MCP_SIGNALS.sessions;
  const sessionIds = Object.keys(signals);
  const [pickedId, setPickedId] = mrUseS(sessionIds[0]);
  const [callFilter, setCallFilter] = mrUseS("all"); // all · used · partial · ignored
  const [focusCall, setFocusCall] = mrUseS(1);

  const sess = signals[pickedId];
  const sessMeta = all.find(s => s.id === pickedId) || {};

  const filteredCalls = sess.calls.filter(c =>
    callFilter === "all" ? true : c.usage === callFilter
  );
  const currentCall = sess.calls.find(c => c.i === focusCall) || sess.calls[0];

  // Small counts for the filter strip
  const counts = {
    all: sess.calls.length,
    used: sess.calls.filter(c => c.usage === "used").length,
    partial: sess.calls.filter(c => c.usage === "partial").length,
    ignored: sess.calls.filter(c => c.usage === "ignored").length
  };

  return (
    <MCPShell activeTab="replay" onTab={onTab}
              kanji="録"
              title="Replay"
              tagline="Every MCP call, in order."
              sub="Step through the tools the assistant reached for during a session — what it asked, what it got back, and whether the response actually moved the next turn."
              chip={
                <span className="mono py-1 px-2" style={{
 fontSize: 11, color: 'var(--ink-3)', border: 'var(--hairline)', borderRadius: 3
}}>
                  {sessionIds.length} sessions indexed
                </span>
              }>
      <div style={{ flex: 1, display: 'grid', gridTemplateColumns: '300px 1fr',
                     minHeight: 0, overflow: 'hidden' }}>

        {/* Session picker */}
        <aside style={{ overflow: 'auto', borderRight: 'var(--hairline)',
                         background: 'var(--paper-2)' }}>
          <div style={{
                         fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="pt-3 pb-2 px-3" >
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
 display: 'block', width: '100%', textAlign: 'left',
                                background: on ? 'var(--paper)' : 'transparent',
                                border: 'none',
                                borderLeft: on ? '2px solid var(--accent)' : '2px solid transparent',
                                cursor: 'pointer'
}} className="py-2 px-3" >
                <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-1" >
                  <span className="mono" style={{ fontSize: 11,
                                color: on ? 'var(--ink)' : 'var(--ink-2)' }}>{sid}</span>
                  <span style={{ fontSize: 11,
                                 color: m.ftr ? 'var(--success)' : 'var(--warning)',
                                 letterSpacing: '0.12em', textTransform: 'uppercase' }}>
                    {m.ftr ? "ftr" : `${m.corrections}c`}
                  </span>
                </div>
                <div style={{
 fontSize: 11, color: 'var(--ink-2)',
                               lineHeight: 1.4,
                               overflow: 'hidden', display: '-webkit-box',
                               WebkitLineClamp: 2, WebkitBoxOrient: 'vertical'
}} className="mt-1" >
                  {sg.title}
                </div>
                <div style={{ fontSize: 11, color: 'var(--ink-4)' }} className="mt-1" >
                  {m.project} · {sg.toolCallCount} calls · {m.duration || "–"}
                </div>
              </button>
            );
          })}
        </aside>

        {/* Detail */}
        <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-6" >
          {/* Session summary strip */}
          <div style={{
 display: 'grid',
                         gridTemplateColumns: 'auto auto auto auto auto 1fr',
                         alignItems: 'baseline', borderBottom: 'var(--hairline)'
}} className="gap-5 mb-4 pb-3" >
            <div>
              <div className="display mb-1" style={{ fontSize: 15 }}>{sess.title}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>{pickedId}</div>
            </div>
            <Stat label="turns"      value={sess.totalTurns}/>
            <Stat label="tool calls" value={sess.toolCallCount}/>
            <Stat label="corrections" value={sess.corrections} tone={sess.corrections === 0 ? "good" : "warn"}/>
            <Stat label="ftr"        value={sess.ftr ? "yes" : "no"} tone={sess.ftr ? "good" : "warn"}/>
            <span/>
          </div>

          {/* Call-filter strip */}
          <div style={{ display: 'flex' }} className="gap-1 mb-3" >
            {[
              { id: "all",     label: "all",      tone: "var(--ink-2)" },
              { id: "used",    label: "used",     tone: "var(--success)" },
              { id: "partial", label: "partial",  tone: "var(--warning)" },
              { id: "ignored", label: "ignored",  tone: "var(--accent)" }
            ].map(f => {
              const on = callFilter === f.id;
              return (
                <button key={f.id} onClick={() => setCallFilter(f.id)}
                        style={{
 fontSize: 11, borderRadius: 4,
                                  display: 'inline-flex', alignItems: 'center',
                                  background: on ? 'var(--ink)' : 'transparent',
                                  color: on ? 'var(--paper)' : f.tone
}} className="py-1 px-3 gap-1" >
                  <span style={{ width: 6, height: 6, borderRadius: '50%',
                                  background: f.tone, opacity: on ? 0.9 : 1 }}/>
                  {f.label}
                  <span className="mono" style={{ fontSize: 11,
                                color: on ? 'var(--paper)' : 'var(--ink-4)', opacity: 0.9 }}>
                    {counts[f.id]}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Timeline + detail */}
          <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1.4fr' }} className="gap-5" >
            {/* Left: timeline */}
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                             textTransform: 'uppercase'
}} className="mb-2" >
                timeline ({filteredCalls.length})
              </div>
              <div style={{
 display: 'flex', flexDirection: 'column',
                             position: 'relative'
}} className="gap-0" >
                {/* thin rail */}
                <div style={{ position: 'absolute', left: 24, top: 10, bottom: 10,
                               width: 1, background: 'var(--edge)' }}/>
                {filteredCalls.map(c => {
                  const on = focusCall === c.i;
                  const dot = usageColor(c.usage);
                  return (
                    <button key={c.i} onClick={() => setFocusCall(c.i)}
                            style={{
 display: 'grid',
                                      gridTemplateColumns: '28px 42px 1fr auto', alignItems: 'center',
                                      textAlign: 'left', borderRadius: 5,
                                      background: on ? 'var(--paper-2)' : 'transparent',
                                      border: on ? '1px solid var(--edge)' : '1px solid transparent',
                                      cursor: 'pointer'
}} className="gap-2 py-2 pl-3 pr-2" >
                      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                                    fontFeatureSettings: '"tnum"' }}>{c.i}</span>
                      <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)',
                                    fontFeatureSettings: '"tnum"' }}>{c.t}</span>
                      <span className="mono" style={{ fontSize: 11, color: 'var(--ink)',
                                    overflow: 'hidden', textOverflow: 'ellipsis',
                                    whiteSpace: 'nowrap' }}>
                        {shortName(c.tool)}
                      </span>
                      <span style={{ display: 'flex', alignItems: 'center' }} className="gap-1" >
                        <span style={{ width: 7, height: 7, borderRadius: '50%', background: dot }}/>
                        <span className="mono" style={{ fontSize: 11, color: 'var(--ink-4)',
                                      fontFeatureSettings: '"tnum"' }}>
                          {c.durationMs}ms
                        </span>
                      </span>
                    </button>
                  );
                })}
                {filteredCalls.length === 0 && (
                  <div style={{
 fontSize: 13, color: 'var(--ink-4)',
                                 textAlign: 'center'
}} className="py-4 px-3" >
                    No calls match.
                  </div>
                )}
              </div>
            </div>

            {/* Right: call detail */}
            <CallDetail call={currentCall}/>
          </div>
        </main>
      </div>
    </MCPShell>
  );
}

function CallDetail({ call }) {
  const badge = usageBadge(call.usage);
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-3" >
      <div style={{
 display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', borderBottom: 'var(--hairline)'
}} className="pb-3" >
        <div>
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            call #{call.i} · turn {call.turn} · {call.t}
          </div>
          <div className="mono" style={{ fontSize: 13, color: 'var(--ink)' }}>
            {call.tool}
          </div>
        </div>
        <span style={{
 display: 'inline-flex', alignItems: 'center', fontSize: 11,
                        background: 'var(--paper-2)', border: 'var(--hairline)',
                        borderRadius: 3, color: badge.color,
                        letterSpacing: '0.1em', textTransform: 'uppercase'
}} className="gap-1 py-1 px-2" >
          <span style={{ width: 6, height: 6, borderRadius: '50%',
                          background: badge.color }}/>
          {badge.label}
        </span>
      </div>

      {/* Request */}
      <CallPanel label="request">
        <pre style={preStyle}>
{JSON.stringify({ tool: call.tool, args: call.args }, null, 2)}
        </pre>
      </CallPanel>

      {/* Response */}
      <CallPanel label={`response · ${call.durationMs}ms`}>
        <div style={{ ...preStyle, borderLeft: '2px solid var(--accent)',
                       color: 'var(--ink)' }}>
          {call.responseSnippet}
        </div>
      </CallPanel>

      {/* Usage */}
      <CallPanel label="what the assistant did next">
        <div style={{ fontSize: 13, color: badge.color, lineHeight: 1.5 }}>
          <span className="display mr-2" style={{
 fontSize: 13, color: badge.color
}}>{badge.glyph}</span>
          {call.note || usageDefaultNote(call.usage)}
        </div>
      </CallPanel>
    </div>
  );
}

function CallPanel({ label, children }) {
  return (
    <div>
      <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                     textTransform: 'uppercase'
}} className="mb-1" >{label}</div>
      {children}
    </div>
  );
}

const preStyle = {
  margin: 0, padding: '12px 12px',
  fontFamily: 'var(--font-mono)', fontSize: 11, lineHeight: 1.55,
  background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 5,
  color: 'var(--ink-2)', whiteSpace: 'pre-wrap', overflow: 'auto'
};

function Stat({ label, value, tone }) {
  const color = tone === "good" ? "var(--success)" :
                tone === "warn" ? "var(--warning)" : "var(--ink)";
  return (
    <div>
      <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                     textTransform: 'uppercase' }}>{label}</div>
      <div className="display mt-1" style={{
 fontSize: 15, color,
                    fontFeatureSettings: '"tnum"'
}}>{value}</div>
    </div>
  );
}

function shortName(fqn) { return fqn.replace(/^sensei\./, ""); }

function usageColor(u) {
  if (u === "used")    return "var(--success)";
  if (u === "partial") return "var(--warning)";
  if (u === "ignored") return "var(--accent)";
  return "var(--ink-3)";
}
function usageBadge(u) {
  if (u === "used")    return { label: "used",    color: "var(--success)", glyph: "✓" };
  if (u === "partial") return { label: "partial", color: "var(--warning)",  glyph: "◐" };
  if (u === "ignored") return { label: "ignored", color: "var(--accent)",    glyph: "✕" };
  return { label: u, color: "var(--ink-3)", glyph: "·" };
}
function usageDefaultNote(u) {
  if (u === "used")    return "Assistant referenced this result in the next turn.";
  if (u === "partial") return "Some of the response was used; other fields ignored.";
  if (u === "ignored") return "Response was clean but the assistant never referenced it in a subsequent turn.";
  return "—";
}


// ═════════════════════════════════════════════════════════════
// INSIGHTS — aggregated usage + effectiveness
// Top: window controls + KPI strip.
// Middle: signals (warn/unused/opportunity/win) — the action list.
// Bottom: per-tool usage table with sparkline + usage split + FTR delta.
// Side: per-project adoption block.
// ═════════════════════════════════════════════════════════════
function MCPInsights({ onTab = () => {} }) {
  const I = window.MCP_SIGNALS.insights;
  const [window_, setWindow] = mrUseS(I.window);
  const [focusTool, setFocusTool] = mrUseS(null);

  return (
    <MCPShell activeTab="insights" onTab={onTab}
              kanji="照"
              title="Insights"
              tagline="Which tools earn their keep — and what to change."
              sub="Aggregated across every session in the window. Usage alone isn't success; the signal is whether the assistant DID something with the response, and whether sessions that touched the tool landed first-try more often than ones that didn't."
              chip={
                <div style={{
 display: 'flex', background: 'var(--paper-2)', borderRadius: 5, border: 'var(--hairline)'
}} className="gap-1 p-1" >
                  {["7d", "30d", "90d"].map(w => (
                    <button key={w} onClick={() => setWindow(w)}
                            style={{
 fontSize: 11, borderRadius: 3,
                                      background: window_ === w ? 'var(--paper)' : 'transparent',
                                      color: window_ === w ? 'var(--ink)' : 'var(--ink-3)',
                                      border: 'none', cursor: 'pointer'
}} className="py-1 px-2" >{w}</button>
                  ))}
                </div>
              }>

      <main style={{ overflow: 'auto' }} className="pt-5 pb-6 px-7" >
        {/* KPI strip */}
        <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)'
}} className="gap-3 mb-5" >
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

        {/* Signals — the action list */}
        <div className="mb-5" >
          <div style={{
 display: 'flex', alignItems: 'baseline',
                         justifyContent: 'space-between'
}} className="mb-3" >
            <h3 className="display m-0" style={{ fontSize: 15, fontWeight: 400 }}>
              Signals
              <span style={{ fontSize: 13, color: 'var(--ink-3)' }} className="ml-2" >
                · what the data suggests you change
              </span>
            </h3>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
              {I.signals.length} signals
            </span>
          </div>
          <div style={{
 display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))'
}} className="gap-3" >
            {I.signals.map((s, i) => <SignalCard key={i} s={s}/>)}
          </div>
        </div>

        {/* Usage table */}
        <div style={{ display: 'grid', gridTemplateColumns: '1.6fr 1fr' }} className="gap-5" >
          <div>
            <div style={{
 display: 'flex', alignItems: 'baseline',
                           justifyContent: 'space-between'
}} className="mb-3" >
              <h3 className="display m-0" style={{ fontSize: 15, fontWeight: 400 }}>
                Per-tool usage
              </h3>
              <span className="mono" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
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

          {/* By-project adoption */}
          <div>
            <h3 className="display mt-0 mb-3" style={{
 fontSize: 15, fontWeight: 400
}}>
              By project
            </h3>
            <div style={{ display: 'flex', flexDirection: 'column' }} className="gap-2" >
              {I.byProject.map(p => <ProjectUsageRow key={p.project} p={p}/>)}
            </div>

            <div style={{
                           background: 'var(--paper-2)', border: 'var(--hairline)',
                           borderRadius: 7
}} className="mt-5 py-3 px-4" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                             textTransform: 'uppercase'
}} className="mb-1" >
                how insights work
              </div>
              <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.6 }}>
                Sensei logs every MCP call its assistants make, pairs the response with
                the next turn, and asks: did the assistant cite the result, ignore it,
                or only use part of it? Roll that up across sessions and you see which
                tools actually change what the assistant does — and which deserve a rewrite.
              </div>
            </div>
          </div>
        </div>
      </main>
    </MCPShell>
  );
}

function Kpi({ kanji, label, value, delta, deltaTone, hint, tone }) {
  const valueColor = tone === "warn" ? "var(--warning)" : "var(--ink)";
  return (
    <div style={{
 background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 7
}} className="py-3 px-3" >
      <div style={{ display: 'flex', alignItems: 'center' }} className="gap-1 mb-1" >
        <span className="kanji" style={{ fontSize: 13, color: 'var(--accent)' }}>{kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                        textTransform: 'uppercase' }}>{label}</span>
      </div>
      <div className="display" style={{ fontSize: 22, color: valueColor,
                    fontFeatureSettings: '"tnum"' }}>{value}</div>
      <div style={{ fontSize: 11, color: 'var(--ink-3)' }} className="mt-1" >
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
    warn:        { border: "var(--warning)",  tintBg: "rgba(194,141,68,0.06)",  label: "warn"  },
    opportunity: { border: "var(--success)", tintBg: "rgba(122,158,98,0.06)", label: "lift" },
    unused:      { border: "var(--ink-3)", tintBg: "var(--paper-2)",         label: "quiet" },
    win:         { border: "var(--accent)",    tintBg: "rgba(196,80,53,0.06)",   label: "win"   }
  };
  const p = palette[s.kind] || palette.warn;
  return (
    <div style={{
 background: p.tintBg,
                   border: 'var(--hairline)', borderLeft: `3px solid ${p.border}`,
                   borderRadius: 5
}} className="py-3 px-4" >
      <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2 mb-1" >
        <span className="kanji" style={{ fontSize: 13, color: p.border }}>{s.kanji}</span>
        <span style={{ fontSize: 11, letterSpacing: '0.16em', color: p.border,
                        textTransform: 'uppercase' }}>{p.label}</span>
        <span style={{ flex: 1 }}/>
      </div>
      <div className="display mb-1" style={{ fontSize: 13 }}>{s.title}</div>
      <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55 }}>
        {s.body}
      </div>
      <div style={{ borderTop: 'var(--hairline)' }} className="mt-2 pt-2" >
        <button style={{
 fontSize: 11, color: p.border,
                          background: 'transparent', border: 'none', cursor: 'pointer',
                          letterSpacing: '0.04em'
}} className="p-0" >
          {s.action} →
        </button>
      </div>
    </div>
  );
}

function ToolRowHeader() {
  return (
    <div style={{
 display: 'grid',
                   gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px',
                   background: 'var(--paper-2)', borderBottom: 'var(--hairline)'
}} className="gap-3 py-2 px-3" >
      {["tool", "calls", "trend 14d", "usage split", "ftr Δ"].map(h => (
        <div key={h} style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                               textTransform: 'uppercase' }}>{h}</div>
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
 width: '100%', display: 'grid',
                        gridTemplateColumns: '1.8fr 56px 120px 1.2fr 80px',
                        background: focus ? 'var(--paper-2)' : 'transparent',
                        border: 'none',
                        borderBottom: last && !focus ? 'none' : 'var(--hairline)',
                        textAlign: 'left', cursor: 'pointer',
                        alignItems: 'center'
}} className="gap-3 py-2 px-3" >
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline' }} className="gap-2" >
            <span className="kanji" style={{ fontSize: 11, color: verdict.color }}>
              {verdict.glyph}
            </span>
            <span className="mono" style={{ fontSize: 11, color: 'var(--ink)' }}>
              {shortName(t.tool)}
            </span>
          </div>
        </div>
        <div className="mono" style={{ fontSize: 13, color: 'var(--ink-2)',
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
                       borderBottom: last ? 'none' : 'var(--hairline)',
                       background: 'var(--paper-2)'
}} className="pt-2 pb-3 pl-7 pr-3" >
          <div style={{
 fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-3)',
                         textTransform: 'uppercase'
}} className="mb-1" >
            verdict · {t.verdict}
          </div>
          <div style={{ fontSize: 11, color: 'var(--ink-2)', lineHeight: 1.55 }}>
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
                stroke="var(--ink-3)" strokeWidth="1.3"/>
      <circle cx={w} cy={h - (data[data.length - 1] / max) * (h - 2) - 1}
              r="2" fill="var(--accent)"/>
    </svg>
  );
}

function UsageBar({ used, partial, ignored }) {
  return (
    <div>
      <div style={{ height: 8, display: 'flex', borderRadius: 2, overflow: 'hidden',
                     background: 'var(--paper-3)' }}>
        <div style={{ width: `${used}%`, background: 'var(--success)' }}/>
        <div style={{ width: `${partial}%`, background: 'var(--warning)' }}/>
        <div style={{ width: `${ignored}%`, background: 'var(--accent)' }}/>
      </div>
      <div style={{
 display: 'flex',
                     fontSize: 11, color: 'var(--ink-3)',
                     fontFeatureSettings: '"tnum"'
}} className="gap-1 mt-1" >
        <span>{used}% used</span>
        {ignored > 0 && <span style={{ color: 'var(--accent)' }}>{ignored}% ignored</span>}
      </div>
    </div>
  );
}

function ProjectUsageRow({ p }) {
  return (
    <div style={{
 background: 'var(--paper-2)',
                   border: 'var(--hairline)', borderRadius: 5,
                   display: 'grid', gridTemplateColumns: '1fr auto', alignItems: 'baseline'
}} className="py-2 px-3 gap-2" >
      <div>
        <div style={{ fontSize: 13, color: 'var(--ink)' }}>{p.project}</div>
        <div className="mono mt-1" style={{ fontSize: 11, color: 'var(--ink-3)' }}>
          {p.sessions} sessions · {p.toolCalls} calls · top: {shortName(p.topTool)}
        </div>
      </div>
      <div style={{ textAlign: 'right' }}>
        <div className="display" style={{ fontSize: 15,
                      color: p.ftr >= 0.7 ? 'var(--success)' :
                             p.ftr >= 0.5 ? 'var(--ink)' : 'var(--warning)',
                      fontFeatureSettings: '"tnum"' }}>
          {Math.round(p.ftr * 100)}%
        </div>
        <div style={{ fontSize: 11, letterSpacing: '0.14em', color: 'var(--ink-4)',
                       textTransform: 'uppercase' }}>ftr</div>
      </div>
    </div>
  );
}


Object.assign(window, { MCPShell, MCPReplay, MCPInsights });
