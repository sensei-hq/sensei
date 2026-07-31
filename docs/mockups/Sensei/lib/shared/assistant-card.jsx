// ─── AssistantCard ────────────────────────────────────────────
// A fully CONTROLLED, presentational card for the wizard's
// Assistants step. It holds no state of its own — a parent owns the
// status of every part and hands it down, so one common reducer can
// drive any number of cards.
//
//   <AssistantCard
//     name="Claude"  logoId="claude"  found  enabled
//     parts={[{ id:"agents", label:"agents", status:"error" }, …]}
//     error="…permission denied"      // consolidated, or null
//     onToggle={…}  onRetry={…}
//   />
//
// Part status: "idle" | "configuring" | "done" | "error"
//   idle → empty ring · configuring → spinner · done → ✓ · error → ✗
//
// STYLING CONVENTION (design-system):
//   • Colors come from semantic utility classes only — text-ink /
//     -soft / -mute / -faint, bg-paper / -soft / -mute, border-*.
//     No raw var() or oklch in this file.
//   • Type comes from the scale (text-xs/-sm/-base, font-semibold).
//   • Inline style is reserved for geometry the scale doesn't model:
//     fixed control dimensions, asymmetric chip padding, the toggle
//     knob position, transitions and opacity.

const _XFADE = "color var(--dur) var(--ease), background var(--dur) var(--ease), border-color var(--dur) var(--ease)";

// Per-status presentation: chip skin classes + glyph.
const CHIP_SKIN = {
  idle:        "text-ink-faint border border-paper-edge border-dashed border-ink-faint",
  configuring: "text-accent bg-paper-mute border border-paper-edge",
  done:        "text-success bg-success-soft border border-success-edge",
  error:       "text-danger bg-danger-soft border border-danger-edge",
};
const STATUS_TEXT = {
  configuring: "text-accent",
  error:       "text-danger",
  done:        "text-success",
};

// ── Glyphs ────────────────────────────────────────────────────
function ACheck({ size = 11, stroke = 2.4 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className="block">
      <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor"
            strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  );
}
function ACross({ size = 11, stroke = 2.4 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" className="block">
      <path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5" fill="none" stroke="currentColor"
            strokeWidth={stroke} strokeLinecap="round"/>
    </svg>
  );
}
function ASpinner({ size = 11 }) {
  return (
    <svg className="zs-spin block" width={size} height={size} viewBox="0 0 16 16">
      <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" strokeWidth="2" strokeOpacity="0.25"/>
      <path d="M8 2 a6 6 0 0 1 6 6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
    </svg>
  );
}

// ── One status-only capability chip ───────────────────────────
function CapChip({ status = "idle", label }) {
  const icon = {
    idle:        <span className="rounded-full border border-ink-faint" style={{ width: 8, height: 8 }}/>,
    configuring: <ASpinner size={11}/>,
    done:        <ACheck size={11}/>,
    error:       <ACross size={11}/>,
  }[status];
  return (
    <span className={"inline-flex items-center mono text-xs rounded-full " + (CHIP_SKIN[status] || "")}
          style={{ gap: 6, padding: "4px 12px 4px 8px", whiteSpace: "nowrap", transition: _XFADE }}>
      {icon}
      {label}
    </span>
  );
}

// ── Header status label (derived from props) ──────────────────
function headerStatus({ found, enabled, parts }) {
  const geo = { whiteSpace: "nowrap", flexShrink: 0 };
  if (!found)
    return <span className="inline-flex items-center gap-1 text-xs text-ink-mute italic" style={{ ...geo }}>not found</span>;
  if (!enabled)
    return <span className="mono inline-flex items-center gap-1 text-xs text-ink-mute" style={geo}>off</span>;
  if (parts.some(p => p.status === "configuring"))
    return <span className="mono inline-flex items-center gap-1 text-xs text-accent" style={geo}><ASpinner size={10}/> configuring…</span>;
  if (parts.some(p => p.status === "error"))
    return <span className="mono inline-flex items-center gap-1 text-xs text-danger" style={geo}><ACross size={10}/> failed</span>;
  if (parts.length && parts.every(p => p.status === "done"))
    return <span className="mono inline-flex items-center gap-1 text-xs text-success" style={geo}><ACheck size={11}/> configured</span>;
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
    <div className={"flex flex-col gap-3 p-4 rounded-lg border border-paper-edge "
                    + (found ? "bg-paper-soft" : "")}
         style={{ opacity: found ? 1 : 0.6 }}>
      {/* Header row: icon · title · status · switch */}
      <div className="flex items-center gap-3">
        <div className={"flex items-center justify-center border border-paper-edge bg-paper rounded "
                        + (enabled ? "text-accent" : (found ? "text-ink" : "text-ink-faint"))}
             style={{ width: 34, height: 34, flexShrink: 0 }}>
          <BrandMark id={logoId} letter={(name || "?")[0]} size={21}/>
        </div>

        <span className="text-base font-semibold">{name}</span>

        <div className="flex items-center gap-3 ml-auto">
          {headerStatus({ found, enabled, parts })}
          <button onClick={onToggle} disabled={!found || busy}
            role="switch" aria-checked={enabled} aria-label={"Enable " + name}
            className={"rounded-full " + (enabled ? "bg-accent" : "bg-paper-mute border border-paper-edge")}
            style={{
              width: 38, height: 22, flexShrink: 0, padding: 0,
              position: "relative", cursor: found && !busy ? "pointer" : "default",
              opacity: busy ? 0.6 : 1, transition: "background var(--dur) var(--ease)",
            }}>
            <span className="rounded-full bg-paper shadow-sm absolute"
 style={{ top: 3, left: enabled ? 19 : 3,
 width: 16, height: 16, transition: "left var(--dur) var(--ease)" }}/>
          </button>
        </div>
      </div>

      {/* Parts — full width below the header */}
      <div className="flex flex-wrap gap-2">
        {parts.map(p => (
          <CapChip key={p.id} label={p.label} status={enabled ? p.status : "idle"}/>
        ))}
      </div>

      {/* One consolidated error — chips stay above so you can see which failed */}
      {showError && (
        <div className="flex items-start gap-2 px-3 py-2 rounded bg-danger-soft border border-danger-edge">
          <span className="text-danger shrink-0" style={{ marginTop: 1 }}><ACross size={11}/></span>
          <div className="flex-1 text-sm text-ink-soft min-w-0" style={{ lineHeight: 1.45 }}>
            Couldn’t configure {name} — <span className="mono text-danger">{error}</span>
          </div>
          {onRetry && (
            <button onClick={onRetry} className="mono text-xs text-accent cursor-pointer shrink-0"
 style={{ marginTop: 1 }}>
              Retry →
            </button>
          )}
        </div>
      )}
    </div>
  );
}

Object.assign(window, { AssistantCard, CapChip });
