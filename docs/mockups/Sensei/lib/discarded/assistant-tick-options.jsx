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
    <div className="flex flex-col gap-4" style={{ padding: "24px 24px 26px" }}>
      <KanjiHeader variant="h1" kanji="連" eyebrow="Assistants"
        title="Connect your assistants"
        description="One switch per assistant — it registers every part, or none. Flip a switch and watch each part settle to ✓, or ✗ if something fails."/>

      <div className="flex flex-col gap-2">
        {CHIP_ASSISTANTS.map(a => (
          <AssistantCard key={a.id} {...toCardProps(a, st[a.id], { toggle, retry })}/>
        ))}
      </div>

      <div className="flex justify-end">
        <button onClick={configureAll} className="zs-btn zs-btn-primary">
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
    <div className="flex flex-col gap-4" style={{ padding: "24px 24px 26px" }}>
      <KanjiHeader variant="h3" eyebrow="Reference" title="Part states"
        description="One status prop per part; the parent reducer drives them all."/>
      <div className="flex flex-col gap-4">
        {rows.map(r => (
          <div key={r.s} className="flex items-center" style={{ gap: 14 }}>
            <div className="shrink-0" style={{ width: 118 }}><CapChip label="agents" status={r.s}/></div>
            <div className="min-w-0" >
              <div className="text-sm text-ink whitespace-nowrap" >{r.t}</div>
              <div className="text-xs text-ink-mute whitespace-nowrap" >{r.d}</div>
            </div>
          </div>
        ))}
      </div>

      <div className="border-t pt-3">
        <div className="text-xs text-ink-mute mb-2">Brand marks (currentColor — adapt to theme)</div>
        <div className="flex" style={{ gap: 10 }}>
          {["claude", "zed", "opencode", "cursor", "openai"].map(id => (
            <div key={id} className="flex items-center justify-center border border-paper-edge bg-paper text-ink rounded"
                 style={{ width: 38, height: 38 }}>
              <BrandMark id={id} size={22}/>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

Object.assign(window, { ChipsLive, ChipsStates });
