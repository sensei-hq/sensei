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
    <div className="sensei w-full h-full flex flex-col bg-paper overflow-hidden" data-screen-label={`MCP · ${title}`}
 >
      <TauriChrome title={`Sensei  先生  ·  mcp · ${activeTab}`}/>

      {/* Hero */}
      <div className="gap-4 pt-6 pb-4 px-12 flex items-end border-b" >
        <div className="kanji text-accent" style={{ fontSize: 40, lineHeight: 1 }}>
          {kanji}
        </div>
        <div className="flex-1" >
          <div style={{
 fontSize: 11, letterSpacing: '0.18em' }} className="mb-1 text-ink-3 uppercase" >
            MCP · {title}
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

      <div className="flex-1 min-h-0 overflow-hidden flex flex-col" >
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
 gridTemplateColumns: 'auto auto auto auto auto 1fr' }} className="gap-6 mb-4 pb-3 grid items-baseline border-b" >
            <div>
              <div className="display mb-1" style={{ fontSize: 15 }}>{sess.title}</div>
              <div className="mono text-ink-3" style={{ fontSize: 11 }}>{pickedId}</div>
            </div>
            <Stat label="turns"      value={sess.totalTurns}/>
            <Stat label="tool calls" value={sess.toolCallCount}/>
            <Stat label="corrections" value={sess.corrections} tone={sess.corrections === 0 ? "good" : "warn"}/>
            <Stat label="ftr"        value={sess.ftr ? "yes" : "no"} tone={sess.ftr ? "good" : "warn"}/>
            <span/>
          </div>

          {/* Call-filter strip */}
          <div className="gap-1 mb-3 flex" >
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
 background: on ? 'var(--ink)' : 'transparent',
 color: on ? 'var(--paper)' : f.tone
 }} className="py-1 px-3 gap-1 inline-flex items-center" >
                  <span className="rounded-full" style={{ width: 6, height: 6,
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
          <div style={{ gridTemplateColumns: '1.1fr 1.4fr' }} className="gap-6 grid" >
            {/* Left: timeline */}
            <div>
              <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-2 text-ink-3 uppercase" >
                timeline ({filteredCalls.length})
              </div>
              <div className="gap-0 flex flex-col relative" >
                {/* thin rail */}
                <div className="absolute" style={{ left: 24, top: 10, bottom: 10,
 width: 1, background: 'var(--edge)' }}/>
                {filteredCalls.map(c => {
                  const on = focusCall === c.i;
                  const dot = usageColor(c.usage);
                  return (
                    <button key={c.i} onClick={() => setFocusCall(c.i)}
 style={{
 gridTemplateColumns: '28px 42px 1fr auto', borderRadius: 5,
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
                {filteredCalls.length === 0 && (
                  <div style={{
 fontSize: 13 }} className="py-4 px-3 text-ink-4 text-center" >
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
 borderRadius: 3, color: badge.color,
 letterSpacing: '0.1em' }} className="gap-1 py-1 px-2 inline-flex items-center bg-paper-2 border border-paper-edge uppercase" >
          <span className="rounded-full" style={{ width: 6, height: 6,
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
        <div className="text-ink" style={{ ...preStyle, borderLeft: '2px solid var(--accent)' }}>
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
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >{label}</div>
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
      <div className="text-ink-4 uppercase" style={{ fontSize: 11, letterSpacing: '0.14em' }}>{label}</div>
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
        {/* KPI strip */}
        <div style={{ gridTemplateColumns: 'repeat(5, 1fr)'
 }} className="gap-3 mb-6 grid" >
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
        <div className="mb-6" >
          <div className="mb-3 flex items-baseline justify-between" >
            <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
              Signals
              <span style={{ fontSize: 13 }} className="ml-2 text-ink-3" >
                · what the data suggests you change
              </span>
            </h3>
            <span className="mono text-ink-3" style={{ fontSize: 11 }}>
              {I.signals.length} signals
            </span>
          </div>
          <div style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))'
 }} className="gap-3 grid" >
            {I.signals.map((s, i) => <SignalCard key={i} s={s}/>)}
          </div>
        </div>

        {/* Usage table */}
        <div style={{ gridTemplateColumns: '1.6fr 1fr' }} className="gap-6 grid" >
          <div>
            <div className="mb-3 flex items-baseline justify-between" >
              <h3 className="display m-0 font-normal" style={{ fontSize: 15 }}>
                Per-tool usage
              </h3>
              <span className="mono text-ink-3" style={{ fontSize: 11 }}>
                {I.toolUsage.length} tools · sorted by calls
              </span>
            </div>
            <div className="border border-paper-edge overflow-hidden bg-paper" style={{ borderRadius: 7 }}>
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
            <h3 className="display mt-0 mb-3 font-normal" style={{
 fontSize: 15 }}>
              By project
            </h3>
            <div className="gap-2 flex flex-col" >
              {I.byProject.map(p => <ProjectUsageRow key={p.project} p={p}/>)}
            </div>

            <div style={{
 borderRadius: 7
 }} className="mt-6 py-3 px-4 bg-paper-2 border border-paper-edge" >
              <div style={{
 fontSize: 11, letterSpacing: '0.14em' }} className="mb-1 text-ink-3 uppercase" >
                how insights work
              </div>
              <div className="text-ink-2" style={{ fontSize: 11, lineHeight: 1.6 }}>
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
        <span className="flex-1" />
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


Object.assign(window, { MCPShell, MCPReplay, MCPInsights });
