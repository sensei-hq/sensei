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
    { id: "insights",   kanji: "照", label: "Insights",
      hint: "what should we change?" }
  ];
  return (
    <div className="sensei" data-screen-label={`MCP · ${title}`}
         style={{ width: '100%', height: '100%', display: 'flex', flexDirection: 'column',
                  background: 'var(--paper)', overflow: 'hidden' }}>
      <TauriChrome title={`Sensei  先生  ·  mcp · ${activeTab}`}/>

      {/* Hero */}
      <div style={{ padding: '26px 56px 18px', display: 'flex',
                     alignItems: 'flex-end', gap: 20, borderBottom: 'var(--hairline)' }}>
        <div className="kanji" style={{ fontSize: 44, color: 'var(--shu)', lineHeight: 1 }}>
          {kanji}
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontSize: 10.5, letterSpacing: '0.18em', color: 'var(--sumi-3)',
                         textTransform: 'uppercase', marginBottom: 6 }}>
            MCP · {title}
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
                         gridTemplateColumns: 'auto auto auto auto auto 1fr',
                         alignItems: 'baseline', gap: 22, marginBottom: 18,
                         paddingBottom: 14, borderBottom: 'var(--hairline)' }}>
            <div>
              <div className="display" style={{ fontSize: 16, marginBottom: 2 }}>{sess.title}</div>
              <div className="mono" style={{ fontSize: 11, color: 'var(--sumi-3)' }}>{pickedId}</div>
            </div>
            <Stat label="turns"      value={sess.totalTurns}/>
            <Stat label="tool calls" value={sess.toolCallCount}/>
            <Stat label="corrections" value={sess.corrections} tone={sess.corrections === 0 ? "good" : "warn"}/>
            <Stat label="ftr"        value={sess.ftr ? "yes" : "no"} tone={sess.ftr ? "good" : "warn"}/>
            <span/>
          </div>

          {/* Call-filter strip */}
          <div style={{ display: 'flex', gap: 4, marginBottom: 14 }}>
            {[
              { id: "all",     label: "all",      tone: "var(--sumi-2)" },
              { id: "used",    label: "used",     tone: "var(--matcha)" },
              { id: "partial", label: "partial",  tone: "var(--amber)" },
              { id: "ignored", label: "ignored",  tone: "var(--shu)" }
            ].map(f => {
              const on = callFilter === f.id;
              return (
                <button key={f.id} onClick={() => setCallFilter(f.id)}
                        style={{ padding: '5px 11px', fontSize: 11, borderRadius: 4,
                                  display: 'inline-flex', gap: 6, alignItems: 'center',
                                  background: on ? 'var(--sumi)' : 'transparent',
                                  color: on ? 'var(--paper)' : f.tone }}>
                  <span style={{ width: 6, height: 6, borderRadius: '50%',
                                  background: f.tone, opacity: on ? 0.9 : 1 }}/>
                  {f.label}
                  <span className="mono" style={{ fontSize: 10,
                                color: on ? 'var(--paper)' : 'var(--sumi-4)', opacity: 0.9 }}>
                    {counts[f.id]}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Timeline + detail */}
          <div style={{ display: 'grid', gridTemplateColumns: '1.1fr 1.4fr', gap: 24 }}>
            {/* Left: timeline */}
            <div>
              <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                             textTransform: 'uppercase', marginBottom: 10 }}>
                timeline ({filteredCalls.length})
              </div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 0,
                             position: 'relative' }}>
                {/* thin rail */}
                <div style={{ position: 'absolute', left: 24, top: 10, bottom: 10,
                               width: 1, background: 'var(--paper-edge)' }}/>
                {filteredCalls.map(c => {
                  const on = focusCall === c.i;
                  const dot = usageColor(c.usage);
                  return (
                    <button key={c.i} onClick={() => setFocusCall(c.i)}
                            style={{ display: 'grid',
                                      gridTemplateColumns: '28px 42px 1fr auto',
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
                {filteredCalls.length === 0 && (
                  <div style={{ padding: '18px 12px', fontSize: 12, color: 'var(--sumi-4)',
                                 textAlign: 'center' }}>
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
                        borderRadius: 3, color: badge.color,
                        letterSpacing: '0.1em', textTransform: 'uppercase' }}>
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
        <div style={{ ...preStyle, borderLeft: '2px solid var(--shu)',
                       color: 'var(--sumi)' }}>
          {call.responseSnippet}
        </div>
      </CallPanel>

      {/* Usage */}
      <CallPanel label="what the assistant did next">
        <div style={{ fontSize: 12, color: badge.color, lineHeight: 1.5 }}>
          <span className="display" style={{ fontSize: 13, color: badge.color,
                        marginRight: 8 }}>{badge.glyph}</span>
          {call.note || usageDefaultNote(call.usage)}
        </div>
      </CallPanel>
    </div>
  );
}

function CallPanel({ label, children }) {
  return (
    <div>
      <div style={{ fontSize: 10, letterSpacing: '0.14em', color: 'var(--sumi-3)',
                     textTransform: 'uppercase', marginBottom: 6 }}>{label}</div>
      {children}
    </div>
  );
}

const preStyle = {
  margin: 0, padding: '11px 14px',
  fontFamily: 'var(--font-mono)', fontSize: 11.5, lineHeight: 1.55,
  background: 'var(--paper-2)', border: 'var(--hairline)', borderRadius: 5,
  color: 'var(--sumi-2)', whiteSpace: 'pre-wrap', overflow: 'auto'
};

function Stat({ label, value, tone }) {
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

function usageColor(u) {
  if (u === "used")    return "var(--matcha)";
  if (u === "partial") return "var(--amber)";
  if (u === "ignored") return "var(--shu)";
  return "var(--sumi-3)";
}
function usageBadge(u) {
  if (u === "used")    return { label: "used",    color: "var(--matcha)", glyph: "✓" };
  if (u === "partial") return { label: "partial", color: "var(--amber)",  glyph: "◐" };
  if (u === "ignored") return { label: "ignored", color: "var(--shu)",    glyph: "✕" };
  return { label: u, color: "var(--sumi-3)", glyph: "·" };
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
        {/* KPI strip */}
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

        {/* Signals — the action list */}
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

        {/* Usage table */}
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

          {/* By-project adoption */}
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
        <span style={{ flex: 1 }}/>
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


Object.assign(window, { MCPShell, MCPReplay, MCPInsights });
