// ─── AssistantCard ────────────────────────────────────────────
// A fully CONTROLLED, presentational card for the wizard's
// Assistants step. It holds no state of its own — a parent owns the
// status of every part and hands it down, so one common reducer can
// drive any number of cards.
//
//   <AssistantCard
//     name="Claude"
//     logoId="claude"            // key into BrandMark registry
//     found={true}               // detected on this machine?
//     enabled={true}             // the per-assistant switch
//     parts={[                    // the registrable parts
//       { id:"plugins",  label:"plugins",  status:"done" },
//       { id:"skills",   label:"skills",   status:"done" },
//       { id:"agents",   label:"agents",   status:"error" },
//     ]}
//     error="~/.claude/agents — permission denied"   // consolidated, or null
//     onToggle={() => …}         // flip the switch (all-or-nothing)
//     onRetry={() => …}          // retry the failed parts
//   />
//
// Part status vocabulary:
//   "idle"        — empty ring (registered nothing yet / switch off)
//   "configuring" — spinner   (in progress)
//   "done"        — check     (registered)
//   "error"       — cross     (failed; see consolidated `error`)

const PART_STATUS = { IDLE: "idle", CONFIGURING: "configuring", DONE: "done", ERROR: "error" };

const _DANGER      = "var(--danger)";
const _DANGER_SOFT = "oklch(0.55 0.18 28 / 0.10)";
const _DANGER_EDGE = "oklch(0.55 0.18 28 / 0.32)";

// ── Glyphs ────────────────────────────────────────────────────
function ACheck({ size = 11, stroke = 2.4 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" style={{ display: "block" }}>
      <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor"
            strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  );
}
function ACross({ size = 11, stroke = 2.4 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" style={{ display: "block" }}>
      <path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5" fill="none" stroke="currentColor"
            strokeWidth={stroke} strokeLinecap="round"/>
    </svg>
  );
}
function ASpinner({ size = 11 }) {
  return (
    <svg className="zs-spin" width={size} height={size} viewBox="0 0 16 16" style={{ display: "block" }}>
      <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="2" strokeOpacity="0.25"/>
      <path d="M8 2 a6 6 0 0 1 6 6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
    </svg>
  );
}

// ── One status-only capability chip ───────────────────────────
function CapChip({ status = "idle", label }) {
  const skin = {
    idle:        { color: "var(--ink-4)",   bg: "transparent",         bd: "1px dashed var(--edge)" },
    configuring: { color: "var(--accent)",  bg: "var(--paper-3)",      bd: "1px solid var(--edge)" },
    done:        { color: "var(--success)", bg: "var(--success-soft)", bd: "1px solid oklch(0.62 0.08 160 / 0.30)" },
    error:       { color: _DANGER,          bg: _DANGER_SOFT,          bd: "1px solid " + _DANGER_EDGE },
  }[status] || {};
  const icon = {
    idle:        <span style={{ width: 8, height: 8, borderRadius: "50%", border: "1.5px solid var(--ink-4)" }}/>,
    configuring: <ASpinner size={11}/>,
    done:        <ACheck size={11}/>,
    error:       <ACross size={11}/>,
  }[status];
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 6,
      fontFamily: "var(--font-mono)", fontSize: 11,
      padding: "3px 10px 3px 8px", borderRadius: 999,
      color: skin.color, background: skin.bg, border: skin.bd,
      transition: "color var(--dur) var(--ease), background var(--dur) var(--ease), border-color var(--dur) var(--ease)",
    }}>
      {icon}
      {label}
    </span>
  );
}

// ── Header status label (derived from props) ──────────────────
function headerStatus({ found, enabled, parts }) {
  const sBase = { fontSize: 11, display: "inline-flex", alignItems: "center", gap: 4, whiteSpace: "nowrap", flexShrink: 0 };
  if (!found)   return <span style={{ ...sBase, fontFamily: "var(--font-ui)", fontStyle: "italic", color: "var(--ink-3)" }}>not found</span>;
  if (!enabled) return <span className="mono" style={{ ...sBase, color: "var(--ink-3)" }}>off</span>;
  if (parts.some(p => p.status === "configuring"))
    return <span className="mono" style={{ ...sBase, color: "var(--accent)", gap: 5 }}><ASpinner size={10}/> configuring…</span>;
  if (parts.some(p => p.status === "error"))
    return <span className="mono" style={{ ...sBase, color: _DANGER }}><ACross size={10}/> failed</span>;
  if (parts.length && parts.every(p => p.status === "done"))
    return <span className="mono" style={{ ...sBase, color: "var(--success)" }}><ACheck size={11}/> configured</span>;
  return null;
}

// ── The card ──────────────────────────────────────────────────
function AssistantCard({
  name, logoId, found = true, enabled = false,
  parts = [], error = null, onToggle, onRetry,
}) {
  const busy = enabled && parts.some(p => p.status === "configuring");
  const showError = enabled && !!error && !busy;

  return (
    <div style={{
      display: "flex", flexDirection: "column", gap: 11,
      border: found ? "var(--hairline)" : "1px dashed var(--edge)",
      borderRadius: 10, background: found ? "var(--paper-2)" : "transparent",
      opacity: found ? 1 : 0.6, padding: "15px 18px",
    }}>
      {/* Header row: icon · title · status · switch */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <div style={{
          width: 34, height: 34, borderRadius: 8, flexShrink: 0,
          background: "var(--paper)", border: "var(--hairline)",
          display: "flex", alignItems: "center", justifyContent: "center",
          color: enabled || !found ? "var(--ink)" : "var(--ink-2)",
        }}>
          <BrandMark id={logoId} letter={(name || "?")[0]} size={21}/>
        </div>

        <span style={{ fontSize: 15, fontWeight: 600 }}>{name}</span>

        <div style={{ marginLeft: "auto", display: "flex", alignItems: "center", gap: 12 }}>
          {headerStatus({ found, enabled, parts })}
          <button onClick={onToggle} disabled={!found || busy}
            role="switch" aria-checked={enabled} aria-label={"Enable " + name}
            style={{
              width: 38, height: 22, borderRadius: 999, flexShrink: 0, padding: 0,
              background: enabled ? "var(--ink)" : "var(--paper-3)",
              border: enabled ? "none" : "var(--hairline)",
              position: "relative", cursor: found && !busy ? "pointer" : "default",
              opacity: busy ? 0.6 : 1,
              transition: "background var(--dur) var(--ease)",
            }}>
            <span style={{
              position: "absolute", top: 3, left: enabled ? 19 : 3,
              width: 16, height: 16, borderRadius: "50%", background: "var(--paper)",
              boxShadow: "var(--shadow-sm)", transition: "left var(--dur) var(--ease)",
            }}/>
          </button>
        </div>
      </div>

      {/* Parts — full width below the header */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        {parts.map(p => (
          <CapChip key={p.id} label={p.label} status={enabled ? p.status : "idle"}/>
        ))}
      </div>

      {/* One consolidated error — chips stay above so you can see which failed */}
      {showError && (
        <div style={{
          display: "flex", alignItems: "flex-start", gap: 8,
          padding: "9px 11px", borderRadius: 7,
          background: _DANGER_SOFT, border: "1px solid " + _DANGER_EDGE,
        }}>
          <span style={{ color: _DANGER, marginTop: 1, flexShrink: 0 }}><ACross size={11}/></span>
          <div style={{ minWidth: 0, flex: 1, fontSize: 12.5, color: "var(--ink-2)", lineHeight: 1.45 }}>
            Couldn’t configure {name} — <span className="mono" style={{ color: _DANGER }}>{error}</span>
          </div>
          {onRetry && (
            <button onClick={onRetry}
              style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--accent)", cursor: "pointer", flexShrink: 0, marginTop: 1 }}>
              Retry →
            </button>
          )}
        </div>
      )}
    </div>
  );
}

Object.assign(window, { AssistantCard, CapChip, PART_STATUS, ACheck, ACross, ASpinner });
