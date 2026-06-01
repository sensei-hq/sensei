// ─── Assistants step · stateful container ────────────────────
// All state lives HERE in one place. Each <AssistantCard> is fully
// controlled — this container maps the common state onto its props.
// The card component lives in lib/assistant-card.jsx.

const { useState, useRef, useCallback } = React;

// ── Fixtures ──────────────────────────────────────────────────
const CHIP_ASSISTANTS = [
  { id: "claude",   name: "Claude",   caps: [
    { id: "plugins", label: "plugins" }, { id: "skills", label: "skills" },
    { id: "commands", label: "commands" }, { id: "agents", label: "agents" },
  ], found: true },
  { id: "zed",      name: "Zed",      caps: [{ id: "mcp", label: "mcp server" }], found: true },
  { id: "opencode", name: "OpenCode", caps: [{ id: "mcp", label: "mcp server" }], found: true },
  { id: "cursor",   name: "Cursor",   caps: [{ id: "mcp", label: "mcp server" }], found: false },
];

// One part fails on the first attempt to exercise the error path;
// succeeds on retry so recovery is visible.
const FLAKY = { "claude/agents": "~/.claude/agents — permission denied" };

// State: { [id]: { enabled, caps:{[capId]: status}, errs:{[capId]: msg} } }
function initState() {
  const o = {};
  for (const a of CHIP_ASSISTANTS) {
    o[a.id] = { enabled: a.found, caps: {}, errs: {} };
    for (const c of a.caps) o[a.id].caps[c.id] = a.found ? "done" : "idle";
  }
  return o;
}

function useAssistants() {
  const [st, setSt] = useState(initState);
  const timers = useRef({});
  const attempts = useRef({});

  const resolvePart = (aId, capId, delay) => {
    const key = aId + "/" + capId;
    clearTimeout(timers.current[key]);
    timers.current[key] = setTimeout(() => {
      const n = attempts.current[key] || 0;
      const willFail = !!FLAKY[key] && n === 0;
      attempts.current[key] = n + 1;
      setSt(p => ({
        ...p,
        [aId]: {
          ...p[aId],
          caps: { ...p[aId].caps, [capId]: willFail ? "error" : "done" },
          errs: { ...p[aId].errs, [capId]: willFail ? FLAKY[key] : null },
        },
      }));
    }, delay);
  };

  const runAssistant = useCallback((aId) => {
    const a = CHIP_ASSISTANTS.find(x => x.id === aId);
    setSt(p => {
      const caps = {}, errs = {};
      for (const c of a.caps) { caps[c.id] = "configuring"; errs[c.id] = null; }
      return { ...p, [aId]: { ...p[aId], caps, errs } };
    });
    a.caps.forEach((c, i) => resolvePart(aId, c.id, 600 + i * 280 + Math.random() * 250));
  }, []);

  const toggle = (aId) => {
    const a = CHIP_ASSISTANTS.find(x => x.id === aId);
    if (!a.found) return;
    if (!st[aId].enabled) { setSt(p => ({ ...p, [aId]: { ...p[aId], enabled: true } })); runAssistant(aId); }
    else setSt(p => {
      const caps = {}, errs = {};
      for (const k in p[aId].caps) { caps[k] = "idle"; errs[k] = null; }
      return { ...p, [aId]: { enabled: false, caps, errs } };
    });
  };

  const retry = (aId) => {
    const a = CHIP_ASSISTANTS.find(x => x.id === aId);
    const failed = a.caps.filter(c => st[aId].caps[c.id] === "error");
    setSt(p => {
      const caps = { ...p[aId].caps };
      failed.forEach(c => { caps[c.id] = "configuring"; });
      return { ...p, [aId]: { ...p[aId], caps } };
    });
    failed.forEach((c, i) => resolvePart(aId, c.id, 500 + i * 250));
  };

  const configureAll = () => {
    CHIP_ASSISTANTS.forEach(a => {
      if (!st[a.id].enabled) return;
      if (!a.caps.every(c => st[a.id].caps[c.id] === "done")) runAssistant(a.id);
    });
  };

  return { st, toggle, retry, configureAll };
}

// Map common state → AssistantCard props for one assistant.
function toCardProps(a, s, handlers) {
  const parts = a.caps.map(c => ({ id: c.id, label: c.label, status: s.enabled ? s.caps[c.id] : "idle" }));
  const firstErr = a.caps.find(c => s.caps[c.id] === "error");
  return {
    name: a.name, logoId: a.id, found: a.found, enabled: s.enabled, parts,
    error: s.enabled && firstErr ? s.errs[firstErr.id] : null,
    onToggle: () => handlers.toggle(a.id),
    onRetry: () => handlers.retry(a.id),
  };
}

// ── Live container ────────────────────────────────────────────
function ChipsLive() {
  const { st, toggle, retry, configureAll } = useAssistants();
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16, padding: "24px 24px 26px" }}>
      <div>
        <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 500 }}>連 · Assistants</div>
        <div className="display" style={{ fontSize: 26, fontWeight: 300, marginTop: 6, letterSpacing: "-0.02em" }}>Connect your assistants</div>
        <p style={{ fontSize: 13, lineHeight: 1.55, color: "var(--ink-3)", marginTop: 6, maxWidth: 420 }}>
          One switch per assistant — it registers every part, or none. Flip a switch and watch each part settle to ✓, or ✗ if something fails.
        </p>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
        {CHIP_ASSISTANTS.map(a => (
          <AssistantCard key={a.id} {...toCardProps(a, st[a.id], { toggle, retry })}/>
        ))}
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 2 }}>
        <button onClick={configureAll}
          style={{ fontSize: 13, fontFamily: "var(--font-ui)", fontWeight: 500, background: "var(--ink)", color: "var(--paper)", padding: "9px 18px", borderRadius: 7, cursor: "pointer" }}>
          Configure &amp; Continue →
        </button>
      </div>
    </div>
  );
}

// ── States reference (static) ─────────────────────────────────
function ChipsStates() {
  const rows = [
    { s: "idle",        t: "Empty",       d: "Switch off / nothing yet" },
    { s: "configuring", t: "Configuring", d: "Spinner — in progress" },
    { s: "done",        t: "Registered",  d: "Check — part is set up" },
    { s: "error",       t: "Failed",      d: "Cross — see message" },
  ];
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14, padding: "24px 24px 26px" }}>
      <div>
        <div style={{ fontSize: 11, letterSpacing: "0.18em", textTransform: "uppercase", color: "var(--ink-3)", fontWeight: 500 }}>Reference</div>
        <div className="display" style={{ fontSize: 20, fontWeight: 400, marginTop: 4 }}>Part states</div>
        <p style={{ fontSize: 12, color: "var(--ink-3)", marginTop: 5, maxWidth: 280, lineHeight: 1.5 }}>
          One <span className="mono">status</span> prop per part; the parent reducer drives them all.
        </p>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        {rows.map(r => (
          <div key={r.s} style={{ display: "flex", alignItems: "center", gap: 14 }}>
            <div style={{ width: 118, flexShrink: 0 }}><CapChip label="agents" status={r.s}/></div>
            <div style={{ minWidth: 0 }}>
              <div style={{ fontSize: 13, color: "var(--ink)", whiteSpace: "nowrap" }}>{r.t}</div>
              <div style={{ fontSize: 12, color: "var(--ink-3)", whiteSpace: "nowrap" }}>{r.d}</div>
            </div>
          </div>
        ))}
      </div>

      <div style={{ borderTop: "var(--hairline)", paddingTop: 14, marginTop: 2 }}>
        <div style={{ fontSize: 12, color: "var(--ink-3)", marginBottom: 10 }}>Brand marks (currentColor — adapt to theme)</div>
        <div style={{ display: "flex", gap: 10 }}>
          {["claude", "zed", "opencode", "cursor", "openai"].map(id => (
            <div key={id} style={{
              width: 38, height: 38, borderRadius: 8, color: "var(--ink)",
              background: "var(--paper)", border: "var(--hairline)",
              display: "flex", alignItems: "center", justifyContent: "center",
            }}>
              <BrandMark id={id} size={22}/>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { ChipsLive, ChipsStates });
