// Dōjō (web app) — shared component kit for the work-first redesign.
// Build every screen from THIS file; no per-screen hand-rolling. One AppShell
// serves both the personal ("you") context and an org context — only the nav
// items and the context header differ.
//
// Styling: semantic utility classes for color + type (text-ink-*, bg-paper-*,
// text-xs/sm, zs-eyebrow, zs-meta) and zs-* components; inline style only for
// geometry the scale doesn't model. Token-only → theme-free (light + dark).
//
// Reuses generic graphics from primitives.jsx: Sparkline · EnsoRing · Avatar.
//
// Exports (on window, named the way the dev will write them — no prefix,
// so the kit reads like an import list): shell — TopBar · OrgSwitcher · NavPane
//   · ContextHeader · AppShell · MobileShell · TabBar. primitives —
//   KanjiMark · Chip · ClassChip · RoleTag · PhasePill · SectionHead
//   · Banner · StatBadge · Spark · Enso · ConfidenceBar · EmptyState
//   · DojoRow · ProjectRow · Button. governance — LadderRung · RuleRow
//   · ConflictCard · StanceDial. relay — RunCard · GateCard
//   · NeedsYouBand · NeedsRow · DecisionCard · ChatThread.


/* ─── shared vocab maps (the only place tones are decided) ── */
const CLASSIFICATION = {
  company:   { kanji: "社", label: "company",   tone: "var(--ink-soft)",  soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  client:    { kanji: "客", label: "client",    tone: "var(--accent)",    soft: "var(--accent-soft)", edge: "var(--accent-edge)" },
  personal:  { kanji: "己", label: "personal",  tone: "var(--ink-mute)",  soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  community: { kanji: "群", label: "community", tone: "var(--success)",   soft: "var(--success-soft)", edge: "var(--success-edge)" },
};
const PHASE = {
  watch:  { kanji: "観", label: "watch",  tone: "var(--ink-mute)", dot: "var(--ink-mute)", step: 1 },
  notice: { kanji: "察", label: "notice", tone: "var(--warning)",  dot: "var(--warning)",  step: 2 },
  adopt:  { kanji: "覚", label: "adopt",  tone: "var(--success)",  dot: "var(--success)",  step: 3 },
};
const ROLE = {
  developer:  { kanji: "士", label: "developer" },
  maintainer: { kanji: "掟", label: "maintainer" },
  lead:       { kanji: "客", label: "lead" },
  admin:      { kanji: "任", label: "admin" },
};
const DOJO_KIND = {
  employer:  { kanji: "社", tone: "var(--ink-soft)" },
  client:    { kanji: "客", tone: "var(--accent)" },
  community: { kanji: "群", tone: "var(--success)" },
  personal:  { kanji: "己", tone: "var(--ink-mute)" },
};

// Solar Icons (bold-duotone) via Iconify, tinted with ?color= — the accessible
// working-icon layer. Rendered as <img> for reliable paint everywhere. Kanji is
// kept for brand/aesthetic marks (brand, sensei voice, ladder scopes, identity).
const ICON_HEX = {
  "var(--accent)": "#A83D1F", "var(--success)": "#2F7D5B", "var(--warning)": "#B87514",
  "var(--ink)": "#2A2925", "var(--ink-soft)": "#514c44", "var(--ink-mute)": "#8f887c",
  "var(--paper)": "#F6F3ED", "var(--danger)": "#A3302A",
};
function Icon({ name, size = 18, color, style }) {
  const hex = ICON_HEX[color] || (color && color[0] === "#" ? color : "#8f887c");
  const url = `https://api.iconify.design/solar:${name}-bold-duotone.svg?color=${encodeURIComponent(hex)}`;
  return <img className="inline-block shrink-0" src={url} width={size} height={size} alt="" aria-hidden="true" style={{ ...style }} />;
}

/* ═══ PRIMITIVES ═══════════════════════════════════════════ */

// A single functional kanji beside nothing else — the brand mark unit.
function KanjiMark({ char, size = "base", tone = "var(--accent)", w, style }) {
  const px = { xs: 11, sm: 13, base: 15, lg: 17, xl: 22, "2xl": 28, "3xl": 40, "4xl": 56 }[size] || size;
  return <span className="kanji shrink-0" style={{ fontSize: px, lineHeight: 1, color: tone, width: w, textAlign: w ? "center" : undefined, ...style }}>{char}</span>;
}

// Generic pill. Kanji optional. Tone/soft/edge default to a neutral chip.
function Chip({ kanji, icon, children, tone = "var(--ink-mute)", soft = "var(--paper-mute)", edge = "transparent", mono, style }) {
  return (
    <span className={"inline-flex items-center gap-1 rounded-full text-xs " + (mono ? "mono" : "")}
      style={{ color: tone, background: soft, border: "1px solid " + edge, padding: "3px 9px", whiteSpace: "nowrap", letterSpacing: mono ? ".04em" : undefined, ...style }}>
      {icon && <Icon name={icon} size={13} color={tone} />}{kanji && <span className="kanji" style={{ fontSize: 12 }}>{kanji}</span>}{children}
    </span>
  );
}

// Classification chip — company · client · personal · community.
function ClassChip({ kind }) {
  const c = CLASSIFICATION[kind] || CLASSIFICATION.company;
  return (
    <span className="inline-flex items-center gap-1 rounded-full text-xs whitespace-nowrap" style={{ color: c.tone, background: c.soft, border: "1px solid " + c.edge, padding: "3px 9px" }}>
      <span className="rounded-full shrink-0" style={{ width: 6, height: 6, background: c.tone }} />{c.label}
    </span>
  );
}

// Additive role tag.
const ROLE_ICON = { developer: "code-2", maintainer: "scale", lead: "case-round", admin: "shield-user" };
function RoleTag({ role, muted }) {
  const r = ROLE[role]; if (!r) return null;
  const c = muted ? "var(--ink-mute)" : "var(--accent)";
  return (
    <span className="mono inline-flex items-center gap-1 rounded-full text-xs whitespace-nowrap"
 style={{ color: c, background: muted ? "var(--paper-mute)" : "var(--accent-soft)",
 border: "1px solid " + (muted ? "var(--paper-edge)" : "var(--accent-edge)"), padding: "3px 10px" }}>
      <Icon name={ROLE_ICON[role] || "user"} size={13} color={c} />{r.label}
    </span>
  );
}

// Phase pill with a 3-step track (watch → notice → adopt).
function PhasePill({ phase }) {
  const p = PHASE[phase] || PHASE.watch;
  return (
    <span className="inline-flex items-center gap-2 rounded-full text-xs bg-paper whitespace-nowrap" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px 3px 8px" }}>
      <span className="flex items-center" style={{ gap: 3 }}>
        {[1, 2, 3].map(n => <span key={n} className="rounded-full" style={{ width: 5, height: 5, background: n <= p.step ? p.dot : "var(--paper-edge)" }} />)}
      </span>
      <span style={{ color: p.tone }}>{p.label}</span>
    </span>
  );
}

// Eyebrow + title + optional kanji + right slot. The one section header.
function SectionHead({ kanji, eyebrow, title, count, right, style }) {
  return (
    <div className="flex items-baseline gap-3 border-b pb-3" style={{ ...style }}>
      {kanji && <KanjiMark char={kanji} size="lg" tone="var(--ink-mute)" />}
      <div className="min-w-0" >
        {eyebrow && <div className="zs-eyebrow font-semibold mb-1">{eyebrow}</div>}
        <h2 className="display font-normal tracking-tight text-xl m-0" style={{ lineHeight: 1.1 }}>{title}</h2>
      </div>
      {count != null && <span className="mono text-xs text-ink-faint">{count}</span>}
      <span className="flex-1" />
      {right}
    </div>
  );
}

// Notice band. tone: neutral | accent | success | warning.
function Banner({ kanji, tone = "neutral", title, children, right }) {
  const map = {
    neutral: { bg: "var(--paper-soft)", edge: "var(--paper-edge)", k: "var(--ink-mute)" },
    accent:  { bg: "var(--accent-soft)", edge: "var(--accent-edge)", k: "var(--accent)" },
    success: { bg: "var(--success-soft)", edge: "var(--success-edge)", k: "var(--success)" },
    warning: { bg: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)", k: "var(--warning)" },
  }[tone];
  return (
    <div className="flex items-start gap-3 rounded-lg py-3 px-4" style={{ background: map.bg, border: "1px solid " + map.edge }}>
      {kanji && <KanjiMark char={kanji} size="lg" tone={map.k} style={{ marginTop: 1 }} />}
      <div className="flex-1 min-w-0" >
        {title && <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.3 }}>{title}</div>}
        {children && <div className="zs-body-sm" style={{ marginTop: title ? 2 : 0 }}>{children}</div>}
      </div>
      {right}
    </div>
  );
}

// Meaningful number + label — sensei's small, specific stat.
function StatBadge({ n, label, sub, tone = "var(--ink)" }) {
  return (
    <div className="flex flex-col" style={{ gap: 2 }}>
      <div className="display font-light tracking-tight text-2xl" style={{ lineHeight: 1, color: tone }}>{n}</div>
      <div className="text-xs font-medium text-ink">{label}</div>
      {sub && <div className="zs-meta">{sub}</div>}
    </div>
  );
}

// Sparkline wrapper (uses primitives.jsx Sparkline).
function Spark({ data, w = 84, h = 24, color = "var(--accent)", fill = "var(--accent-soft)" }) {
  return <window.Sparkline data={data} width={w} height={h} color={color} fill={fill} />;
}
// Ensō ring wrapper.
function Enso(props) { return <window.EnsoRing {...props} />; }

// Confidence bar — labelled 0..1.
function ConfidenceBar({ v, w = 96, showN = true }) {
  const tone = v >= 0.85 ? "var(--success)" : v >= 0.7 ? "var(--accent)" : "var(--warning)";
  return (
    <div className="flex items-center gap-2">
      <div className="bg-paper-mute rounded-full overflow-hidden" style={{ width: w, height: 4 }}>
        <div className="rounded-full h-full" style={{ width: (v * 100) + "%", background: tone }} />
      </div>
      {showN && <span className="mono text-xs text-ink-soft">{Math.round(v * 100)}</span>}
    </div>
  );
}

// Empty state — kanji anchor, one landing line, a calm second sentence.
function EmptyState({ kanji = "空", title = "Still listening.", children, action }) {
  return (
    <div className="flex flex-col items-center text-center py-16 px-8 gap-3" >
      <KanjiMark char={kanji} size="3xl" tone="var(--ink-faint)" />
      <div className="display font-normal text-lg text-ink" style={{ letterSpacing: "-0.01em" }}>{title}</div>
      {children && <div className="zs-body-sm m-0" style={{ maxWidth: 380 }}>{children}</div>}
      {action && <div className="mt-2" >{action}</div>}
    </div>
  );
}

// ═══ DATA ACCESS ════════════════════════════════════════════════════
// Screens read through ZS_API (lib/data/dojo2-api.js), never from the
// fixture global, so swapping in real endpoints doesn't touch any JSX.
// This hook is the whole integration surface: it holds {data, loading,
// error} and re-runs when its key changes.
//
// The quiet-while-loading behaviour is deliberate — the system's empty
// state is content ("Still listening."), not a skeleton shimmer.
function useAsync(fn, key, fallback) {
  const [state, setState] = React.useState({ data: fallback, loading: true, error: null });
  React.useEffect(() => {
    let live = true;
    setState(s => ({ data: s.data, loading: true, error: null }));
    Promise.resolve()
      .then(fn)
      .then(d => { if (live) setState({ data: d, loading: false, error: null }); })
      .catch(e => { if (live) setState({ data: fallback, loading: false, error: e }); });
    return () => { live = false; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return state;
}

// ═══ THE SHARED LEAVES ═════════════════════════════════════════════
// One Button, one Card family, one Row, one Segmented control, one
// Metric. Every variant is an enumerated prop — never a class string
// passed in from outside, because the variant list IS the contract with
// the dev. If you're hand-styling one of these, the answer is a new
// variant here, once.

// THE button. variant: primary | ghost | danger | link. size: sm|md|lg.
// `link` is the bare-text affordance (back links, inline toggles) that
// used to be hand-rolled as <button className="text-sm text-ink-mute">.
// `full` stretches to the container, for form and sign-in buttons.
// Focus states come from the zs-btn rule in tokens.css — part of this
// contract, not an afterthought.
function Button({ variant = "primary", size = "md", kanji, icon, children, onClick, style, title, full, disabled, type }) {
  const kc = variant === "danger" || variant === "primary" ? "var(--paper)"
    : variant === "link" ? "var(--ink-mute)" : "var(--accent)";
  if (variant === "link") {
    return (
      <button type={type} title={title} onClick={onClick} disabled={disabled}
        className={"inline-flex items-center gap-1 bg-transparent self-start p-0 "
          + (size === "sm" ? "text-xs " : "text-sm ") + (disabled ? "text-ink-faint" : "text-ink-mute")}
        style={style}>
        {icon && <Icon name={icon} size={size === "sm" ? 13 : 15} color={disabled ? "var(--ink-faint)" : kc} />}
        {kanji && !icon && <span className="kanji" style={{ color: kc }}>{kanji}</span>}
        {children}
      </button>
    );
  }
  const cls = "zs-btn justify-center "
    + (size === "sm" ? "zs-btn-sm " : size === "lg" ? "zs-btn-lg " : "")
    + (full ? "w-full " : "")
    + (variant === "primary" ? "zs-btn-primary" : variant === "ghost" ? "bg-paper border border-paper-edge" : "");
  const skin = variant === "danger"
    ? { background: "var(--danger)", color: "var(--paper)", borderColor: "var(--danger)" } : null;
  return (
    <button type={type} title={title} onClick={onClick} disabled={disabled} className={cls}
      style={{ ...skin, ...(disabled ? { opacity: 0.45 } : null), ...style }}>
      {icon && <Icon name={icon} size={size === "sm" ? 15 : 16} color={kc} />}
      {kanji && !icon && <span className="kanji" style={{ color: kc }}>{kanji}</span>}
      {children}
    </button>
  );
}

// THE card. tone: paper (default) | accent | ink | success | warning.
// pad: false (flush — the card holds its own rows) | true | "sm".
// `selected` draws the accent ring that used to be an inline border.
// A card never carries a shadow — the system separates with hairlines,
// and that rule is enforced by not offering the option.
function Card({ tone = "paper", pad = false, selected, overflow = true, children, className = "", style, onClick }) {
  const TONE = {
    paper:   { bg: null, edge: null },
    accent:  { bg: "var(--accent-soft)",  edge: "var(--accent-edge)" },
    ink:     { bg: "var(--ink)",          edge: "var(--ink)" },
    success: { bg: "var(--success-soft)", edge: "var(--success-edge)" },
    warning: { bg: "var(--warning-soft)", edge: "var(--warning-edge)" },
  };
  const t = TONE[tone] || TONE.paper;
  const cls = "zs-card-flush "
    + (pad === "sm" ? "p-3 " : pad ? "p-4 " : "")
    + (overflow ? "overflow-hidden " : "") + className;
  const Tag = onClick ? "button" : "div";
  return (
    <Tag onClick={onClick} className={cls + (onClick ? " w-full text-left" : "")}
      style={{ background: t.bg || undefined,
        borderColor: selected ? "var(--accent)" : (t.edge || undefined), ...style }}>{children}</Tag>
  );
}

// THE row inside a card. Hairline-separated, optionally clickable.
// `cols` sets grid-template-columns — the one piece of geometry the scale
// can't model; omit it for a flex row.
// `gap` is a scale stop, mapped through a literal table rather than
// concatenated: a build-time Uno scans source text, so `"gap-" + gap`
// would generate nothing. Any dynamic utility needs its full class name
// to appear literally somewhere.
const GAP_CLASS = { 0: "gap-0", 1: "gap-1", 2: "gap-2", 3: "gap-3", 4: "gap-4", 6: "gap-6", 8: "gap-8" };
function Row({ cols, gap = 3, pad = "py-3 px-4", onClick, selected, align = "center", last, children, className = "", style }) {
  const Tag = onClick ? "button" : "div";
  const base = (cols ? "grid " : "flex ") + (GAP_CLASS[gap] || "gap-3") + " " + pad + " "
    + (align === "start" ? "items-start " : align === "baseline" ? "items-baseline " : "items-center ")
    + (last ? "" : "border-b ")
    + (onClick ? "w-full text-left bg-transparent " : "") + className;
  return (
    <Tag onClick={onClick} className={base}
      style={{ gridTemplateColumns: cols, background: selected ? "var(--paper-mute)" : undefined, ...style }}>
      {children}
    </Tag>
  );
}

// THE segmented control — the two/three-way view switch that was
// hand-rolled in three screens. options: [{id, label, icon}].
// gap-0.5/p-0.5 are Uno's fractional stops (2px) — not raw px.
function SegmentedControl({ options = [], value, onPick, size = "sm" }) {
  return (
    <div className="flex bg-paper-mute rounded gap-0.5 p-0.5">
      {options.map(o => {
        const on = o.id === value;
        return (
          <button key={o.id} onClick={() => onPick && onPick(o.id)}
            className={"inline-flex items-center gap-2 rounded py-1 px-2.5 " + (size === "sm" ? "text-xs " : "text-sm ")
              + (on ? "bg-paper text-ink border border-paper-edge" : "text-ink-mute bg-transparent border border-paper-edge border-transparent")}>
            {o.icon && <Icon name={o.icon} size={14} color={on ? "var(--accent)" : "var(--ink-mute)"} />}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

// THE pick-one pill row (rule families, filters). Distinct from
// SegmentedControl: these wrap, carry a kanji, and read as chips not a switch.
function PillChoice({ options = [], value, onPick }) {
  return (
    <div className="flex flex-wrap gap-2">
      {options.map(o => {
        const on = o.id === value;
        return (
          <button key={o.id} onClick={() => onPick && onPick(o.id)}
            className={"inline-flex items-center gap-2 rounded-full text-sm py-2 px-3 " + (on ? "bg-accent-soft" : "bg-paper-2 border border-paper-edge")}
            style={{ border: on ? "1px solid var(--accent)" : undefined, color: on ? "var(--accent)" : "var(--ink-soft)" }}>
            {o.kanji && <span className="kanji" style={{ fontSize: 14 }}>{o.kanji}</span>}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

// THE numbered answer option — sensei asks with numbers, you click or
// type the number. Used by every ask in the inbox.
function NumberedChoice({ n, label, selected, onPick }) {
  return (
    <button onClick={onPick} className="flex items-center gap-3 text-left rounded py-2 px-3"
      style={{ background: selected ? "var(--paper-mute)" : "transparent",
        border: "1px solid " + (selected ? "var(--accent)" : "transparent") }}>
      <span className="mono text-xs shrink-0" style={{ color: selected ? "var(--accent)" : "var(--ink-faint)" }}>{n}</span>
      <span className={"text-sm " + (selected ? "text-ink" : "text-ink-soft")}>{label}</span>
    </button>
  );
}

// THE toggle. A track and a thumb, nothing else. Used for every on/off
// switch (sharing, feature flags), so a switch never gets hand-rolled.
function Toggle({ on, onToggle, label }) {
  return (
    <button onClick={onToggle} role="switch" aria-checked={!!on} aria-label={label}
      className="rounded-full shrink-0 border-0 p-0.5"
      style={{ width: 38, height: 22,
        background: on ? "var(--accent)" : "var(--paper-mute)",
        transition: "background var(--dur) var(--ease)" }}>
      <span className="rounded-full block bg-paper" style={{ width: 18, height: 18,
        transform: on ? "translateX(16px)" : "translateX(0)",
        transition: "transform var(--dur) var(--ease)" }} />
    </button>
  );
}

// THE metric — eyebrow, number, optional trend and caption. `deltaGood`
// says which direction is good, so a falling corrections-per-session
// reads green while a falling first-try rate reads amber.
function Metric({ label, value, unit, delta, deltaGood = "down", sub, empty = "nothing sent yet" }) {
  if (value == null) return (
    <div className="flex flex-col gap-0.5">
      <span className="zs-eyebrow text-ink-mute">{label}</span>
      <span className="mono text-lg text-ink-faint">—</span>
      <span className="zs-meta">{empty}</span>
    </div>
  );
  const good = delta == null ? null : (deltaGood === "up" ? delta > 0 : delta < 0);
  return (
    <div className="flex flex-col gap-0.5">
      <span className="zs-eyebrow text-ink-mute">{label}</span>
      <span className="flex items-baseline gap-2">
        <span className="mono text-2xl text-ink">{value}<span className="text-sm text-ink-mute">{unit}</span></span>
        {delta != null && delta !== 0 && (
          <span className="mono text-xs" style={{ color: good ? "var(--success)" : "var(--warning)" }}>
            {delta > 0 ? "↑" : "↓"}{Math.abs(delta)}{unit === "%" ? "pp" : ""}
          </span>
        )}
      </span>
      {sub && <span className="zs-meta">{sub}</span>}
    </div>
  );
}

// A strip of metrics in a card — the one recipe for a stat row.
// Column counts use Uno's grid-cols-* rather than an inline repeat().
const COL_CLASS = { 1: "grid-cols-1", 2: "grid-cols-2", 3: "grid-cols-3", 4: "grid-cols-4" };
function MetricRow({ items = [], mobile }) {
  const n = Math.min(items.length, 4);
  return (
    <div className={"zs-card grid gap-6 " + (mobile ? COL_CLASS[2] : (COL_CLASS[n] || COL_CLASS[4]))}>
      {items.map((m, i) => <Metric key={m.label || i} {...m} />)}
    </div>
  );
}

// A dōjō membership row — org identity + your role. Same shape on phone and
// desktop: kanji mark, name + kind, route, then role and what needs you.
function DojoRow({ dojo, onOpen, mobile }) {
  const kind = DOJO_KIND[dojo.kind] || DOJO_KIND.employer;
  return (
    <button onClick={() => onOpen && onOpen(dojo)} className="w-full text-left border-b grid gap-3 py-3 px-4 bg-transparent items-start"
 style={{ gridTemplateColumns: "30px minmax(0, 1fr)" }}>
      <span className="kanji text-center" style={{ fontSize: 24, lineHeight: 1.1, color: kind.tone }}>{dojo.kanji}</span>
      <span className="min-w-0 flex flex-col" style={{ gap: 3 }}>
        <span className="flex items-baseline gap-2">
          <span className="text-sm font-medium text-ink min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{dojo.name}</span>
          <span className="mono text-xs text-ink-mute">{dojo.kind}</span>
          <span className="flex-1" />
          {dojo.needs > 0 && <span className="mono text-xs text-accent font-semibold shrink-0" >{dojo.needs} need you</span>}
        </span>
        <span className="mono text-xs text-ink-faint overflow-hidden text-ellipsis whitespace-nowrap" >{dojo.route}</span>
        <span className="flex items-center gap-3" style={{ marginTop: 1 }}>
          <RoleTag role={dojo.role} />
          <span className="mono text-xs text-ink-faint min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{dojo.members} members · {dojo.projects} projects</span>
          {!mobile && <span className="text-ink-faint" style={{ fontSize: 16, marginLeft: "auto" }}>→</span>}
        </span>
      </span>
    </button>
  );
}

// A project row — name · classification · phase · signal. The list workhorse.
function ProjectRow({ p, onOpen, showDojo = true, compact = false }) {
  if (compact) {
    return (
      <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b grid gap-3 p-4 bg-transparent items-start"
 style={{ gridTemplateColumns: "22px minmax(0, 1fr)" }}>
        <Icon name="folder" size={18} color="var(--accent)" style={{ marginTop: 2 }} />
        <span className="min-w-0 flex flex-col gap-2" >
          <span className="min-w-0 flex flex-col" style={{ gap: 2 }}>
            <span className="flex items-baseline gap-2">
              <span className="text-base font-medium text-ink min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{p.name}</span>
              <span className="flex-1" />
              {p.needs > 0 && <span className="mono text-xs text-accent font-semibold shrink-0" >{p.needs} need{p.needs === 1 ? "s" : ""} you</span>}
            </span>
            <span className="mono text-xs text-ink-faint overflow-hidden text-ellipsis whitespace-nowrap" >{p.repo}</span>
          </span>
          <span className="flex items-center gap-2 flex-wrap" >
            <ClassChip kind={p.classification} />
            <PhasePill phase={p.phase} />
            <span className="mono text-xs text-ink-faint">last run {p.lastRun}</span>
          </span>
        </span>
      </button>
    );
  }
  const cols = showDojo
    ? "22px minmax(140px,1.3fr) 100px 108px minmax(0,1.7fr) 84px 36px 108px 34px"
    : "22px minmax(140px,1.3fr) 100px minmax(0,1.7fr) 84px 36px 108px 34px";
  return (
    <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b grid items-center gap-3 py-3 px-4 bg-transparent"
 style={{ gridTemplateColumns: cols }}>
      <Icon name="folder" size={18} color="var(--accent)" />
      <div className="min-w-0" >
        <div className="text-sm font-medium text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ lineHeight: 1.2 }}>{p.name}</div>
        <div className="mono text-xs text-ink-faint whitespace-nowrap overflow-hidden text-ellipsis" style={{ marginTop: 1 }}>{p.repo}</div>
      </div>
      <span className="flex items-center"><ClassChip kind={p.classification} /></span>
      {showDojo && <span className="text-xs text-ink-mute whitespace-nowrap overflow-hidden text-ellipsis" >{p.dojoName || ""}</span>}
      <span className="text-xs text-ink-mute whitespace-nowrap overflow-hidden text-ellipsis" >{p.note || ""}</span>
      <span className="flex items-center justify-start" >{p.spark ? <Spark data={p.spark} /> : null}</span>
      <span className="flex items-center">{p.needs > 0 ? <Chip icon="bell" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{p.needs}</Chip> : null}</span>
      <span className="flex items-center justify-center"><PhasePill phase={p.phase} /></span>
      <span className="mono text-xs text-ink-faint text-right" >{p.lastRun}</span>
    </button>
  );
}

/* ═══ SHELL ════════════════════════════════════════════════ */

// A titled list section: icon + eyebrow title + count + right slot, over a
// single flush card holding the rows. The one recipe for every "header + rows"
// block (live runs, active projects, members, packs…).
function ListSection({ icon, iconColor = "var(--accent)", title, count, countTone, right, children, style }) {
  return (
    <div style={style}>
      <div className="flex items-center gap-2 mb-3" >
        {icon && <Icon name={icon} size={17} color={iconColor} />}
        <span className="zs-eyebrow font-semibold text-ink">{title}</span>
        {count != null && <span className="mono text-xs" style={{ color: countTone || "var(--ink-faint)" }}>{count}</span>}
        <span className="flex-1" />
        {right}
      </div>
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >{children}</div>
    </div>
  );
}

// Org switcher popover. context "you" or an org slug. Lists "your work" +
// every membership. Purely presentational click state for the kit.
function OrgSwitcher({ context = "you", org, dojos = [], onPick }) {
  const [open, setOpen] = React.useState(false);
  const isYou = context === "you";
  return (
    <div className="relative" >
      <button onClick={() => setOpen(o => !o)}
        className={"inline-flex items-center gap-2 bg-paper-soft rounded " + (open ? "border-accent" : "border border-paper-edge")}
        style={{ padding: "var(--space-1) var(--space-3)", minHeight: 32, border: open ? "1px solid var(--accent)" : undefined }}>
        <span className="kanji" style={{ fontSize: 14, color: isYou ? "var(--accent)" : (DOJO_KIND[org?.kind]?.tone || "var(--accent)") }}>{isYou ? "携" : org?.kanji}</span>
        <span className="text-sm text-ink">{isYou ? "Your work" : org?.name}</span>
        {!isYou && <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".08em" }}>{org?.kind}</span>}
        <span className="text-xs text-ink-mute">▾</span>
      </button>
      {open && (
        <div className="bg-paper border border-paper-edge rounded-lg shadow-lg absolute overflow-hidden" style={{ top: "calc(100% + 6px)", left: 0, width: 320, zIndex: 60 }}>
          <div className="flex items-center gap-2 border-b py-2 px-3" >
            <KanjiMark char="探" size="sm" tone="var(--ink-mute)" />
            <span className="flex-1 text-sm text-ink-faint">Switch context…</span>
            <span className="mono text-xs text-ink-faint bg-paper-mute rounded-sm" style={{ padding: "3px 7px" }}>⌘K</span>
          </div>
          <button onClick={() => { setOpen(false); onPick && onPick("you"); }}
            className={"flex items-center gap-3 w-full text-left border-b " + (isYou ? "bg-accent-soft" : "")}
            style={{ padding: "var(--space-3)" }}>
            <KanjiMark char="携" size="base" tone="var(--accent)" w={20} />
            <div className="flex-1 min-w-0" >
              <div className="text-sm text-ink font-medium">Your work</div>
              <div className="text-xs text-ink-mute">every dōjō · nothing to switch</div>
            </div>
            {isYou && <span className="text-sm text-accent">✓</span>}
          </button>
          <div className="zs-eyebrow font-semibold text-ink-mute" style={{ padding: "var(--space-3) var(--space-3) var(--space-1)" }}>My dōjōs</div>
          <div className="overflow-auto" style={{ maxHeight: 260, paddingBottom: 4 }}>
            {dojos.map(d => {
              const on = !isYou && org?.slug === d.slug;
              return (
                <button key={d.slug} onClick={() => { setOpen(false); onPick && onPick(d.slug); }}
                  className={"flex items-center gap-3 w-full text-left " + (on ? "bg-paper-soft" : "")} style={{ padding: "var(--space-2) var(--space-3)" }}>
                  <span className="kanji text-center shrink-0" style={{ fontSize: 14, color: DOJO_KIND[d.kind]?.tone, width: 20 }}>{d.kanji}</span>
                  <div className="flex-1 min-w-0" >
                    <div className="text-sm text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{d.name}</div>
                    <div className="mono text-xs text-ink-faint">{ROLE[d.role]?.label} · {d.kind}</div>
                  </div>
                  {d.needs > 0 && <span className="mono text-xs font-semibold bg-accent rounded-full text-paper" style={{ padding: "0 7px", lineHeight: "16px" }}>{d.needs}</span>}
                  {on && <span className="text-sm text-accent">✓</span>}
                </button>
              );
            })}
          </div>
          <button className="flex items-center gap-2 w-full text-left border-t text-sm text-ink-soft p-3" >
            <span className="text-accent">＋</span> Create or join a dōjō
          </button>
        </div>
      )}
    </div>
  );
}

// Top bar: brand · org switcher · search · avatar. Shared by both contexts.
// In an org, an accent top-rule + route chip + role tag carry the "you've
// stepped into a managed place" signal — no second rail needed.
function TopBar({ context, org, dojos, me, onPick, needsCount = 0, onNeeds }) {
  const isOrg = context === "org";
  return (
    <div className="flex items-center gap-3 border-b bg-paper px-4 shrink-0" style={{ height: 56, borderTop: isOrg ? "2px solid var(--accent)" : undefined }}>
      <div className="flex items-baseline gap-2 shrink-0" >
        <span className="kanji text-accent" style={{ fontSize: 20, lineHeight: 1 }}>結</span>
        <span className="display text-lg tracking-tight">Dōjō</span>
      </div>
      <OrgSwitcher context={context} org={org} dojos={dojos} onPick={onPick} />
      {isOrg && <span className="mono text-xs text-ink-mute bg-paper-soft rounded-full" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px" }}>{org.route}</span>}
      {isOrg && <RoleTag role={org.role} />}
      <span className="flex-1" />
      <div className="zs-input text-sm" style={{ width: 240, height: 34, padding: "0 12px" }}>
        <KanjiMark char="探" size="sm" tone="var(--ink-mute)" />
        <span className="text-ink-faint whitespace-nowrap overflow-hidden text-ellipsis" >search…</span>
      </div>
      <button onClick={onNeeds} title={needsCount ? needsCount + " need you" : "Nothing needs you"} className="flex items-center justify-center rounded-full relative shrink-0"
 style={{ width: 34, height: 34, background: needsCount ? "var(--accent-soft)" : "transparent", border: "1px solid " + (needsCount ? "var(--accent-edge)" : "var(--paper-edge)") }}>
        <Icon name="bell" size={17} color={needsCount ? "var(--accent)" : "var(--ink-mute)"} />
        {needsCount > 0 && <span className="mono absolute bg-accent text-paper text-center font-semibold" style={{ top: -5, right: -5, minWidth: 16, height: 16, padding: "0 4px", borderRadius: 8, fontSize: 10, lineHeight: "16px" }}>{needsCount}</span>}
      </button>
      <window.Avatar name={me?.name || "You"} size={30} />
    </div>
  );
}

// Context header — the band under the top bar that tells you WHERE you are.
// Personal ("you") vs an org. This is the only piece that changes shape.
function ContextHeader({ context, org, me }) {
  if (context === "you") {
    return (
      <div className="flex items-center gap-3 border-b bg-paper-soft px-4 shrink-0" style={{ height: 46 }}>
        <KanjiMark char="携" size="lg" tone="var(--accent)" />
        <span className="text-sm font-medium text-ink">Your work</span>
        <span className="text-sm text-ink-mute">— everything in flight, across every dōjō</span>
        <span className="flex-1" />
        <span className="zs-meta">{me?.name}</span>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-3 px-4 shrink-0 bg-paper-soft border-b" style={{ height: 46, borderTop: "2px solid var(--accent)" }}>
      <KanjiMark char={org.kanji} size="lg" tone={DOJO_KIND[org.kind]?.tone || "var(--accent)"} />
      <span className="display text-lg tracking-tight whitespace-nowrap" >{org.name}</span>
      <span className="mono text-xs text-ink-mute bg-paper rounded-full" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px" }}>{org.route}</span>
      <span className="flex-1" />
      <RoleTag role={org.role} />
    </div>
  );
}

// Left nav — grouped items + version footer. Groups: [{group, items:[{id,kanji,label,badge}]}].
function NavPane({ groups = [], active, onNav, width = 222 }) {
  return (
    <aside className="flex flex-col border-r bg-paper-soft shrink-0 py-4 px-3 overflow-auto" style={{ width }}>
      {groups.map((grp, gi) => (
        <div key={grp.group || gi} className="mb-4">
          {grp.group && <div className="zs-eyebrow font-semibold px-2 mb-2 text-ink-mute">{grp.group}</div>}
          <div className="flex flex-col gap-1">
            {grp.items.map(it => {
              const on = active === it.id;
              return (
                <button key={it.id} onClick={() => onNav && onNav(it.id)}
                  className={"w-full text-left rounded text-sm " + (on ? "bg-paper text-ink" : "text-ink-soft")}
                  style={{ display: "grid", gridTemplateColumns: "auto 1fr auto", alignItems: "center", gap: "var(--space-2)",
                    padding: "var(--space-2)", border: on ? "1px solid var(--paper-edge)" : "1px solid transparent" }}>
                  <span className="flex items-center justify-center" style={{ width: 16 }}>{it.icon ? <Icon name={it.icon} size={17} color={on ? "var(--accent)" : "var(--ink-mute)"} /> : <span className={"kanji " + (on ? "text-accent" : "text-ink-mute")} style={{ fontSize: 14 }}>{it.kanji}</span>}</span>
                  <span className="whitespace-nowrap overflow-hidden text-ellipsis" >{it.label}</span>
                  {it.badge != null
                    ? <span className="mono text-xs font-semibold bg-accent rounded-full text-paper" style={{ padding: "0 7px", lineHeight: "16px" }}>{it.badge}</span>
                    : <span />}
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <span className="flex-1" style={{ minHeight: 12 }} />
      <div className="mono border-t text-xs text-ink-faint flex items-center gap-1" style={{ padding: "var(--space-3) var(--space-2) 0" }}>
        <span className="kanji">結</span>Dōjō v0.4.2
      </div>
    </aside>
  );
}

// The one shell. context "you" | "org". nav groups + context header differ;
// everything else identical. Pass `main` (or children) for content.
function AppShell({ label, context = "you", org, dojos = [], me, nav = [], active, onNav, onPick, needsCount, onNeeds, children }) {
  return (
    <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label={label} >
      <TopBar context={context} org={org} dojos={dojos} me={me} onPick={onPick} needsCount={needsCount} onNeeds={onNeeds} />
      <div className="flex flex-1 min-h-0" >
        <NavPane groups={nav} active={active} onNav={onNav} />
        <div className="flex-1 min-w-0 overflow-auto" >{children}</div>
      </div>
    </div>
  );
}

// Mobile bottom tab bar.
function TabBar({ tabs = [], active, onNav }) {
  return (
    <div className="grid border-t bg-paper shrink-0" style={{ gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}>
      {tabs.map(it => {
        const on = active === it.id;
        return (
          <button key={it.id} onClick={() => onNav && onNav(it.id)} className={"flex flex-col items-center gap-1 " + (on ? "text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-1) var(--space-3)", position: "relative" }}>
            {it.icon ? <Icon name={it.icon} size={20} color={on ? "var(--accent)" : "var(--ink-mute)"} /> : <span className={"kanji " + (on ? "text-accent" : "text-ink-mute")} style={{ fontSize: 17 }}>{it.kanji}</span>}
            <span className={"text-xs " + (on ? "font-semibold" : "font-normal")} style={{ whiteSpace: "nowrap" }}>{it.label}</span>
            {it.badge != null && <span className="mono text-xs font-semibold bg-accent rounded-full text-paper absolute" style={{ top: 4, left: "50%", marginLeft: 6, padding: "0 6px", lineHeight: "14px" }}>{it.badge}</span>}
          </button>
        );
      })}
    </div>
  );
}

// Mobile shell — condensed top bar (context) + scrolling main + bottom tabs.
function MobileShell({ label, context = "you", org, me, tabs = [], active, onNav, children }) {
  const isYou = context === "you";
  return (
    <div className="sensei w-full h-full flex flex-col overflow-hidden bg-paper" data-screen-label={label} >
      <div className="flex items-center gap-3 border-b bg-paper shrink-0 py-3 px-4" style={{ borderTop: isYou ? "none" : "2px solid var(--accent)" }}>
        <span className="kanji text-accent" style={{ fontSize: 19, lineHeight: 1 }}>{isYou ? "結" : org?.kanji}</span>
        <div className="flex-1 min-w-0" >
          <div className="text-sm font-semibold text-ink whitespace-nowrap overflow-hidden text-ellipsis" style={{ lineHeight: 1.1 }}>{isYou ? "Your work" : org?.name}</div>
          <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{isYou ? "every dōjō" : ROLE[org?.role]?.label + " · " + org?.kind}</div>
        </div>
        <button title="Search" className="flex items-center justify-center rounded-full bg-paper-soft border border-paper-edge shrink-0" style={{ width: 34, height: 34 }}>
          <img src="https://api.iconify.design/solar:magnifer-bold-duotone.svg?color=%23736c60" width="18" height="18" alt="Search" />
        </button>
        <window.Avatar name={me?.name || "You"} size={28} />
      </div>
      <div className="flex flex-col flex-1 min-h-0 overflow-auto" >{children}</div>
      <TabBar tabs={tabs} active={active} onNav={onNav} />
    </div>
  );
}

/* ═══ GOVERNANCE ═══════════════════════════════════════════ */

// One rung of the constitution ladder — scope identity + its rules + lock count.
function LadderRung({ rung, active, onSelect, showRules = true }) {
  const on = active === rung.id;
  const tone = rung.tone === "accent" ? "var(--accent)" : "var(--ink-soft)";
  const locks = (rung.rules || []).filter(r => r.hard).length;
  return (
    <div className="rounded-lg overflow-hidden" style={{ border: on ? "1px solid var(--accent)" : "var(--hairline)", background: on ? "var(--accent-soft)" : "var(--paper-2)" }}>
      <button onClick={() => onSelect && onSelect(rung.id)} className="flex items-center gap-3 w-full text-left py-3 px-4 bg-transparent" >
        <span className="kanji text-center shrink-0" style={{ fontSize: 22, lineHeight: 1, color: tone, width: 28 }}>{rung.kanji}</span>
        <div className="flex-1 min-w-0" >
          <div className="flex items-center gap-2">
            <span className="zs-eyebrow font-semibold" style={{ color: tone }}>{rung.scope}</span>
            <span className="text-sm font-medium text-ink">{rung.name}</span>
          </div>
          <div className="zs-meta" style={{ marginTop: 1 }}>{rung.caption}</div>
        </div>
        <span className="mono text-xs text-ink-faint">{(rung.rules || []).length} rules</span>
        {locks > 0 && <Chip icon="lock-keyhole" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{locks} locked</Chip>}
      </button>
      {showRules && (
        <div className="border-t bg-paper" >
          {(rung.rules || []).map((r, i) => <RuleRow key={i} rule={r} />)}
        </div>
      )}
    </div>
  );
}

// A rule row — include toggle · rule · level pill · ★ non-negotiable · edit.
function RuleRow({ rule, onToggle, included = true, showLevel, onEdit, onJump }) {
  return (
    <div className="flex items-center gap-3 border-b py-2 px-4" >
      {onToggle ? (
        <button onClick={() => onToggle(rule)} className="rounded-sm shrink-0 text-paper text-center" style={{ width: 16, height: 16, border: "1px solid " + (included ? "var(--accent)" : "var(--ink-faint)"), background: included ? "var(--accent)" : "transparent", fontSize: 11, lineHeight: "14px" }}>{included ? "✓" : ""}</button>
      ) : (
        <span className="kanji text-ink-mute text-center shrink-0" style={{ fontSize: 13, width: 16 }}>{rule.kanji}</span>
      )}
      <span className="text-sm text-ink flex-1" style={{ lineHeight: 1.35, opacity: included ? 1 : 0.5 }}>{rule.text}</span>
      {showLevel && rule.level && (onJump
        ? <button onClick={() => onJump(rule)} title={"Jump to " + rule.level} className="mono inline-flex items-center gap-1 rounded-full text-xs text-ink-mute bg-paper-mute cursor-pointer whitespace-nowrap" style={{ border: "1px solid var(--paper-edge)", padding: "3px 9px" }}>{rule.level}<Icon name="arrow-right-up" size={12} color="var(--ink-mute)" /></button>
        : <Chip mono tone="var(--ink-mute)">{rule.level}</Chip>)}
      {rule.hard && <span className="inline-flex items-center gap-1 text-xs text-accent whitespace-nowrap" ><span style={{ fontSize: 12 }}>★</span>non-negotiable</span>}
      {onEdit && <button onClick={() => onEdit(rule)} title="Edit rule" className="flex items-center bg-transparent shrink-0" ><Icon name="pen-2" size={15} color="var(--ink-mute)" /></button>}
    </div>
  );
}

// Conflict card — topic · winner · why, with a lock marker when a ★ decided it.
function ConflictCard({ conflict }) {
  const c = conflict;
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      <div className="flex items-center gap-2 border-b py-3 px-4" >
        <Icon name="danger-triangle" size={17} color="var(--warning)" />
        <span className="text-sm font-medium text-ink flex-1">{c.topic}</span>
        {c.locked ? <Chip icon="lock-keyhole" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">locked</Chip>
          : <Chip tone="var(--ink-mute)">settled</Chip>}
      </div>
      <div className="flex p-4 gap-3" >
        <div className="flex-1 rounded p-3 bg-paper-mute" style={{ opacity: 0.7 }}>
          <div className="zs-eyebrow text-ink-mute mb-1">{c.loser.level} · yields</div>
          <div className="text-sm text-ink-soft" style={{ textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>{c.loser.text}</div>
        </div>
        <div className="flex items-center text-ink-faint" >→</div>
        <div className="flex-1 rounded p-3 bg-success-soft" style={{ border: "1px solid var(--success-edge)" }}>
          <div className="zs-eyebrow mb-1 text-success" >{c.winner.level} · wins</div>
          <div className="text-sm text-ink font-medium">{c.winner.text}</div>
        </div>
      </div>
      <div className="border-t zs-body-sm py-3 px-4 bg-paper" >{c.why}</div>
    </div>
  );
}

// Stance dial — a labelled discrete slider (autonomy / sharing / review).
function StanceDial({ dial, onChange }) {
  const [v, setV] = React.useState(dial.value);
  const n = dial.levels.length;
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
      <div className="flex items-center gap-2 mb-1">
        <Icon name={{ autonomy: "cpu-bolt", sharing: "share-circle", review: "checklist-minimalistic" }[dial.id] || "settings"} size={17} color="var(--accent)" />
        <span className="text-sm font-medium text-ink">{dial.label}</span>
        <span className="flex-1" />
        <span className="mono text-xs text-accent">{dial.levels[v]}</span>
      </div>
      <div className="zs-meta mb-3">{dial.caption}</div>
      <div className="flex items-center gap-0" >
        {dial.levels.map((lv, i) => (
          <React.Fragment key={i}>
            <button onClick={() => { setV(i); onChange && onChange(dial.id, i); }} title={lv}
 className="rounded-full shrink-0" style={{ width: 14, height: 14, border: "2px solid " + (i <= v ? "var(--accent)" : "var(--paper-edge)"), background: i <= v ? "var(--accent)" : "var(--paper)" }} />
            {i < n - 1 && <span className="flex-1" style={{ height: 2, background: i < v ? "var(--accent)" : "var(--paper-edge)" }} />}
          </React.Fragment>
        ))}
      </div>
      <div className="flex justify-between" style={{ marginTop: 6 }}>
        <span className="text-xs text-ink-faint">{dial.levels[0]}</span>
        <span className="text-xs text-ink-faint">{dial.levels[n - 1]}</span>
      </div>
    </div>
  );
}

/* ═══ RELAY ════════════════════════════════════════════════ */

// In-page tab strip — one level of nesting under a nav destination, so the
// rail stays short. tabs: [{id, label, icon, badge}].
function SubTabs({ tabs = [], active, onPick }) {
  return (
    <div className="flex gap-2 flex-wrap" >
      {tabs.map(x => {
        const on = x.id === active;
        return (
          <button key={x.id} onClick={() => onPick && onPick(x.id)}
            className={"inline-flex items-center gap-2 rounded text-sm " + (on ? "bg-paper-soft border border-paper-edge text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-3)", border: on ? undefined : "1px solid transparent" }}>
            {x.icon && <Icon name={x.icon} size={15} color={on ? "var(--accent)" : "var(--ink-mute)"} />}
            {x.label}
            {x.badge ? <span className="mono text-xs" style={{ color: on ? "var(--accent)" : "var(--ink-faint)" }}>{x.badge}</span> : null}
          </button>
        );
      })}
    </div>
  );
}

// A live-run card. `flat` drops the card chrome so it reads as a row inside a
// single flush card (matching the project list) instead of a stack of cards.
function RunCard({ run, onOpen, onAct, flat, stacked, selected }) {
  const f = runFlag(run);
  const pr = planProgress(run.plan || []);
  const plan = toPhases(run.plan || []);
  const tasks = plan.flatMap(s => s.tasks || []);
  const nowTask = tasks.find(t => t.state === "active")
    || tasks.find(t => t.is_gate || t.state === "needs_review" || t.state === "failed");
  const nowLine = (
    <span className="text-xs text-ink-mute block" style={{ marginTop: 3 }}>
      <span className="font-semibold" style={{ color: f.tone }}>{f.label}</span>
      {nowTask ? <span> · Now: {nowTask.title || nowTask.name}</span> : null}
    </span>
  );
  const meta = <span className="mono text-xs text-ink-faint">{run.project} · {run.id} · {run.elapsed}</span>;
  const progress = plan.length ? (
    <span className="block" >
      <PlanBar pct={pr.pct} tone={f.tone} />
      <span className="flex items-center gap-2" style={{ marginTop: 5 }}>
        <span className="mono text-xs text-ink-faint">stage {pr.stage}/{pr.stages} · {pr.pct}%</span>
        <PlanPips plan={plan} showCaption={false} />
      </span>
    </span>
  ) : null;
  const cta = f.act
    ? <Button size="sm" variant="primary" onClick={(e) => { e && e.stopPropagation && e.stopPropagation(); onAct ? onAct(run, f) : onOpen && onOpen(run); }}>{f.cta}</Button>
    : <Button size="sm" variant="ghost" onClick={(e) => { e && e.stopPropagation && e.stopPropagation(); onOpen && onOpen(run); }}>Watch →</Button>;

  if (stacked) {
    return (
      <div className="flex flex-col w-full border-b" style={{ background: selected ? "var(--paper-mute)" : "transparent" }}>
        <div className="flex min-w-0" >
          <span className="shrink-0" style={{ width: 3, background: f.act ? f.tone : "transparent" }} />
          <button onClick={() => onOpen && onOpen(run)} className="flex flex-col flex-1 text-left gap-2 bg-transparent min-w-0"
 style={{ padding: "var(--space-3) var(--space-4) var(--space-2)" }}>
            <span className="min-w-0" >
              <span className="text-sm font-medium text-ink block" style={{ lineHeight: 1.3 }}>{run.task}</span>
              {nowLine}
            </span>
            {meta}
            {progress}
          </button>
        </div>
        <div className="flex" style={{ padding: "0 var(--space-4) var(--space-3)", paddingLeft: "calc(var(--space-4) + 3px)" }}>{cta}</div>
      </div>
    );
  }
  const cls = flat ? "flex w-full text-left border-b" : "zs-card-flush flex w-full text-left";
  return (
    <div className={cls} style={{ background: selected ? "var(--paper-mute)" : "transparent" }}>
      <span className="shrink-0" style={{ width: 3, background: f.act ? f.tone : "transparent" }} />
      <button onClick={() => onOpen && onOpen(run)} className="flex items-center gap-4 flex-1 text-left bg-transparent min-w-0"
 style={{ padding: flat ? "var(--space-3) var(--space-4)" : "var(--space-4)" }}>
        <Icon name="eye" size={22} color={f.tone} />
        <span className="flex-1 min-w-0" >
          <span className="flex items-center gap-2">
            <span className="text-sm font-medium text-ink">{run.task}</span>
            <Chip mono tone="var(--ink-mute)">{run.assistant}</Chip>
          </span>
          {nowLine}
          <span className="block" style={{ marginTop: 3 }}>{meta}</span>
        </span>
        {progress ? <span className="shrink-0" style={{ width: 168 }}>{progress}</span> : null}
      </button>
      <span className="flex items-center pr-4 shrink-0" >{cta}</span>
    </div>
  );
}

// ── inbox ──────────────────────────────────────────────
// One row per in-flight session. Status, progress, why it's surfaced, and how
// long since it last said anything. Everything answerable lives in the detail.
const RUN_STATUS = {
  running: { label: "running", tone: "var(--success)", soft: "var(--success-soft)" },
  waiting: { label: "waiting", tone: "var(--ink-soft)", soft: "var(--paper-mute)" },
  paused:  { label: "paused",  tone: "var(--ink-mute)", soft: "var(--paper-mute)" },
  stalled: { label: "stalled", tone: "var(--warning)", soft: "var(--warning-soft)" },
  blocked: { label: "blocked", tone: "var(--warning)", soft: "var(--warning-soft)" },
  failed:  { label: "failed",  tone: "var(--danger)",  soft: "var(--danger-soft)" },
  done:    { label: "done",    tone: "var(--ink-mute)", soft: "var(--paper-mute)" },
};
// Roll a run + its pending items into an inbox row.
function toInboxRow(run, needs = 0) {
  const pr = planProgress(run.plan || []);
  const states = allTasks(run.plan || []).map(t => t.state);
  let status = run.state;
  if (status !== "done" && states.includes("failed")) status = "failed";
  else if (run.stale) status = "stalled";
  else if (status !== "done" && !states.includes("active") && states.includes("blocked")) status = "blocked";
  const attention = needs > 0 ? "gate" : (status === "stalled" || status === "blocked" || status === "failed") ? status : null;
  const rank = needs > 0 ? 0 : attention ? 1 : status === "running" ? 2 : status === "done" ? 4 : 3;
  return { run, needs, status, attention, rank, done: pr.done, total: pr.total, pct: pr.pct };
}
function toInboxRows(runs = [], pendingFor) {
  return runs.map(r => toInboxRow(r, pendingFor ? pendingFor(r).length : 0))
    .sort((a, b) => a.rank - b.rank);
}
function InboxRow({ row, selected, onOpen }) {
  const s = RUN_STATUS[row.status] || RUN_STATUS.waiting;
  const r = row.run;
  const attn = row.needs > 0 ? "var(--accent)" : row.attention ? s.tone : null;
  const why = row.needs > 0
    ? row.needs + (row.needs === 1 ? " needs you" : " need you")
    : row.attention === "stalled" ? "no heartbeat"
    : row.attention === "blocked" ? "blocked on a task"
    : row.attention === "failed" ? "a task failed" : null;
  return (
    <div className="border-b" style={{ background: selected ? "var(--paper-mute)" : "transparent" }}>
      <button onClick={() => onOpen && onOpen(r)} className="w-full text-left grid gap-3 py-3 px-4 bg-transparent"
 style={{ gridTemplateColumns: "10px minmax(0, 1fr)" }}>
        <span className="rounded-full" style={{ width: 7, height: 7, marginTop: 6,
          background: attn || (row.status === "running" ? s.tone : "transparent"),
          border: attn || row.status === "running" ? "none" : "1px solid var(--ink-faint)" }} />
        <span className="min-w-0 flex flex-col" style={{ gap: 3 }}>
          <span className="flex items-baseline gap-2">
            <span className="mono text-xs text-ink-mute min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{r.project}</span>
            <span className="flex-1" />
            <span className="mono text-xs text-ink-faint shrink-0" >{r.last}</span>
          </span>
          <span className={"text-sm " + (row.status === "done" ? "text-ink-mute" : "text-ink font-medium")}
            style={{ lineHeight: 1.35, display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{r.task}</span>
          <span className="flex items-center gap-2" style={{ marginTop: 1 }}>
            <span className="text-xs whitespace-nowrap" style={{ color: attn || "var(--ink-mute)", fontWeight: attn ? 600 : 400 }}>
              {why || s.label}
            </span>
            <span className="flex-1" />
            <PlanPips plan={r.plan} showCaption={false} />
            <span className="mono text-xs text-ink-faint shrink-0" >{row.done}/{row.total}</span>
          </span>
        </span>
      </button>
    </div>
  );
}

// A gate card — command awaiting approve / deny.
function GateCard({ gate, onApprove, onDeny }) {
  const high = gate.risk === "high";
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      <div className="flex items-center gap-2 border-b py-3 px-4" >
        <Icon name="command" size={18} color="var(--accent)" />
        <span className="text-sm font-medium text-ink flex-1">{gate.project}</span>
        <Chip mono tone={high ? "var(--danger)" : "var(--warning)"} soft={high ? "var(--danger-soft)" : "var(--warning-soft)"} edge={high ? "var(--danger-edge)" : "oklch(0.72 0.12 75 / 0.30)"}>{gate.risk}</Chip>
        <span className="mono text-xs text-ink-faint">{gate.age}</span>
      </div>
      <div className="p-4" >
        <div className="mono text-sm text-ink bg-paper-mute rounded p-3 border border-paper-edge overflow-x-auto" >$ {gate.cmd}</div>
        <div className="zs-body-sm mt-2" >{gate.why} · session {gate.session}</div>
        <div className="flex flex-wrap gap-2 mt-3" >
          <Button size="sm" icon="check-circle" onClick={onApprove}>Approve once</Button>
          <Button size="sm" variant="ghost" onClick={onApprove}>Always allow</Button>
          <span className="flex-1" />
          <Button size="sm" variant="ghost" icon="close-circle" onClick={onDeny}>Deny</Button>
        </div>
      </div>
    </div>
  );
}

// The needs-you band — a header + rows of things waiting on the viewer.
const NEEDS_TONE = {
  gate:     { icon: "command",         label: "approve", tone: "var(--accent)" },
  conflict: { icon: "danger-triangle", label: "settle",  tone: "var(--warning)" },
  decision: { icon: "checklist-minimalistic", label: "decide", tone: "var(--accent)" },
  review:   { icon: "clipboard-check", label: "review",  tone: "var(--ink-soft)" },
};
// Action sets per needs-kind — the band is a remote control, so you act here.
const NEEDS_ACTIONS = {
  gate:     [{ id: "approve", label: "Approve", icon: "check-circle", primary: true }, { id: "deny", label: "Deny", icon: "close-circle" }],
  conflict: [{ id: "settle", label: "Settle", icon: "scale", primary: true }],
  decision: [{ id: "decide", label: "Decide", icon: "checklist-minimalistic", primary: true }],
  review:   [{ id: "approve", label: "Approve", icon: "check-circle", primary: true }, { id: "deny", label: "Decline", icon: "close-circle" }],
};
function NeedsActions({ item, onAct, size = "sm" }) {
  const acts = NEEDS_ACTIONS[item.kind] || NEEDS_ACTIONS.decision;
  return (
    <div className="flex items-center gap-2 shrink-0" >
      {acts.map(a => (
        <Button key={a.id} size={size} variant={a.primary ? "primary" : "ghost"} icon={a.icon}
          onClick={(e) => { if (e && e.stopPropagation) e.stopPropagation(); onAct && onAct(item, a); }}>{a.label}</Button>
      ))}
    </div>
  );
}
function NeedsResolved({ label }) {
  return (
    <span className="inline-flex items-center gap-1 text-xs text-success whitespace-nowrap shrink-0" >
      <Icon name="check-circle" size={15} color="var(--success)" />{label}
    </span>
  );
}
function NeedsRow({ item, onOpen, onAct, resolved, stacked }) {
  const t = NEEDS_TONE[item.kind] || NEEDS_TONE.decision;
  const done = resolved && resolved[item.id];
  if (stacked) {
    return (
      <div className="flex flex-col w-full border-b py-3 px-4 gap-2" style={{ opacity: done ? 0.7 : 1 }}>
        <button onClick={() => onOpen && onOpen(item)} className="flex items-center gap-2 text-left bg-transparent" >
          <Icon name={t.icon} size={18} color={t.tone} />
          <span className="text-sm font-medium text-ink flex-1" style={{ lineHeight: 1.3 }}>{item.title}</span>
        </button>
        <div className="zs-meta">{item.project} · {item.dojo} · {item.why}</div>
        <div className="flex items-center gap-2">
          {done ? <NeedsResolved label={done} />
            : <NeedsActions item={item} onAct={onAct} />}
          <span className="flex-1" />
          <span className="mono text-xs text-ink-faint">{item.age}</span>
        </div>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-3 w-full border-b py-3 px-4" style={{ opacity: done ? 0.7 : 1 }}>
      <button onClick={() => onOpen && onOpen(item)} className="flex items-center gap-3 flex-1 text-left min-w-0 bg-transparent" >
        <span className="flex items-center justify-center shrink-0" style={{ width: 22 }}><Icon name={t.icon} size={19} color={t.tone} /></span>
        <div className="flex-1 min-w-0" >
          <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.25 }}>{item.title}</div>
          <div className="zs-meta" style={{ marginTop: 1 }}>{item.project} · {item.dojo} · {item.why}</div>
        </div>
      </button>
      {done ? <NeedsResolved label={done} />
        : <NeedsActions item={item} onAct={onAct} />}
      <span className="mono text-xs text-ink-faint text-right shrink-0" style={{ width: 30 }}>{item.age}</span>
    </div>
  );
}
function NeedsYouBand({ items = [], onOpen, onAct, resolved, title = "Needs you", mobile }) {
  const open = items.filter(it => !(resolved && resolved[it.id])).length;
  return (
    <div className="rounded-lg bg-paper-2 overflow-hidden" style={{ border: "1px solid var(--accent-edge)" }}>
      <div className="flex items-center gap-2 py-3 px-4 bg-accent-soft" style={{ borderBottom: "1px solid var(--accent-edge)" }}>
        <Icon name="bell" size={17} color="var(--accent)" />
        <span className="zs-eyebrow font-semibold text-accent" >{title}</span>
        <span className="mono text-xs text-accent" >{open}</span>
        <span className="flex-1" />
        {!mobile && <span className="zs-meta">{open ? "act here — nothing routes you away" : "nothing else is blocked on you"}</span>}
      </div>
      {items.length ? items.map(it => <NeedsRow key={it.id} item={it} onOpen={onOpen} onAct={onAct} resolved={resolved} stacked={mobile} />)
        : <EmptyState kanji="静" title="Nothing needs you.">Sessions run within the rules you set. sensei will surface only what it can't decide alone.</EmptyState>}
    </div>
  );
}

// A decision card — sign off with options.
function DecisionCard({ decision, onChoose }) {
  const d = decision;
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      <div className="flex items-start gap-3 p-4" >
        <Icon name="checklist-minimalistic" size={22} color="var(--accent)" />
        <div className="flex-1 min-w-0" >
          <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.3 }}>{d.title}</div>
          <div className="zs-body-sm" style={{ marginTop: 3 }}>{d.project} · {d.context}</div>
        </div>
        <span className="mono text-xs text-ink-faint">{d.age}</span>
      </div>
      <div className="flex flex-wrap gap-2 border-t py-3 px-4 bg-paper" >
        {d.options.map((o, i) => (
          <Button key={i} size="sm" variant={i === 0 ? "primary" : "ghost"} icon={i === 0 ? "check-circle" : undefined} onClick={() => onChoose && onChoose(o)}>{o}</Button>
        ))}
      </div>
    </div>
  );
}

// A chat thread — sensei speaks rarely; the viewer replies.
function ChatThread({ thread = [], me }) {
  return (
    <div className="flex flex-col gap-4" >
      {thread.map((m, i) => {
        const mine = m.who !== "sensei";
        return (
          <div key={i} className="flex gap-3" style={{ flexDirection: mine ? "row-reverse" : "row" }}>
            {mine ? <window.Avatar name={me?.name || "You"} size={28} />
              : <KanjiMark char={m.kanji || "先"} size="lg" tone="var(--accent)" style={{ marginTop: 2 }} />}
            <div style={{ maxWidth: 460 }}>
              <div className="rounded-lg py-3 px-4 border border-paper-edge" style={{ background: mine ? "var(--paper-mute)" : "var(--paper-2)" }}>
                <div className="text-sm text-ink" style={{ lineHeight: 1.5 }}>{m.text}</div>
              </div>
              <div className={"zs-meta " + (mine ? "text-right" : "")} style={{ marginTop: 3 }}>{mine ? (me?.name || "you") : "sensei"} · {m.when}</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

/* ═══ RELAY · PLAN GRAPH ═══════════════════════════════════ */

// The authored plan is { goal, phases: [{ title, tasks }] }; a task carries
// id · title · agent · model · spec_ref · summary · state · deps · is_gate.
// Parallelism is derived from deps (tasks whose deps are all satisfied run
// together), never authored. These are the seven task states.
const TASK_STATE = {
  done:         { icon: "check-circle",              label: "done",         tone: "var(--ink-mute)",  bg: "var(--paper-2)",      edge: "var(--paper-edge)" },
  active:       { icon: "play-circle",               label: "active",       tone: "var(--success)",   bg: "var(--success-soft)", edge: "var(--success-edge)" },
  needs_review: { icon: "shield-warning",            label: "needs review", tone: "var(--accent)",    bg: "var(--accent-soft)",  edge: "var(--accent-edge)" },
  blocked:      { icon: "lock-keyhole-minimalistic", label: "blocked",      tone: "var(--warning)",   bg: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)" },
  failed:       { icon: "close-circle",              label: "failed",       tone: "var(--danger)",    bg: "var(--danger-soft)",  edge: "var(--danger-edge)" },
  skipped:      { icon: "forward",                   label: "skipped",      tone: "var(--ink-faint)", bg: "var(--paper-2)",      edge: "var(--paper-edge)" },
  pending:      { icon: "clock-circle",              label: "pending",      tone: "var(--ink-faint)", bg: "transparent",         edge: "var(--paper-edge)", dashed: true },
};
// Older authored plans used stage/queued/running/gate — read them too.
const TASK_STATE_ALIAS = { queued: "pending", running: "active", gate: "needs_review" };
// Normalize any plan into phases of tasks. Accepts the authored object shape
// and the legacy array-of-stages shape.
function toPhases(plan) {
  const phases = Array.isArray(plan) ? plan : (plan && plan.phases) || [];
  return phases.map((p, i) => ({
    id: p.id || "p" + i,
    title: p.title || p.name || "Phase " + (i + 1),
    tasks: (p.tasks || []).map(t => ({
      ...t,
      title: t.title || t.name,
      summary: t.summary || t.meta,
      state: TASK_STATE_ALIAS[t.state] || t.state || "pending",
      deps: t.deps || [],
    })),
  }));
}
function allTasks(plan) { return toPhases(plan).flatMap(p => p.tasks); }
// Phase rolls up to the most urgent state among its tasks.
function phaseState(stage) {
  const s = (stage.tasks || []).map(t => TASK_STATE_ALIAS[t.state] || t.state);
  return ["failed", "needs_review", "blocked", "active", "pending"].find(k => s.includes(k)) || "done";
}

// One task node.
function PlanNode({ task, onSelect, selected }) {
  const n = TASK_STATE[TASK_STATE_ALIAS[task.state] || task.state] || TASK_STATE.pending;
  return (
    <button onClick={() => onSelect && onSelect(task)}
 className="flex items-start gap-2 w-full text-left rounded py-2 px-3"
 style={{ background: n.bg,
 border: (n.dashed ? "1px dashed " : "1px solid ") + (selected ? n.tone : n.edge),
 boxShadow: selected ? "inset 0 0 0 1px " + n.tone : "none" }}>
      <Icon name={n.icon} size={16} color={n.tone} style={{ marginTop: 1 }} />
      <span className="min-w-0 flex-1" >
        <span className={"text-sm " + (task.state === "pending" ? "text-ink-mute" : "text-ink")}
          style={{ display: "block", lineHeight: 1.35 }}>{task.title || task.name}</span>
        {task.summary || task.meta ? <span className="zs-meta block" style={{ marginTop: 2 }}>{task.summary || task.meta}</span> : null}
      </span>
    </button>
  );
}

// The stage column: header + its tasks, wired parallel (fan bracket) or
// sequential (arrow chain).
function PlanStage({ stage, index, onSelect, selectedId }) {
  const par = stage.mode ? stage.mode === "parallel" : (stage.tasks || []).filter(t => !(t.deps || []).length).length > 1;
  const st = TASK_STATE[phaseState(stage)] || TASK_STATE.pending;
  const tasks = stage.tasks || [];
  return (
    <div className="flex flex-col gap-2" style={{ minWidth: 150, flex: "1 1 0" }}>
      <div className="flex items-center gap-2 pb-2 border-b" >
        <span className="mono text-xs text-ink-faint">{String(index + 1).padStart(2, "0")}</span>
        <span className="text-sm font-medium text-ink flex-1 min-w-0" >{stage.title || stage.name}</span>
        <span className="rounded-full shrink-0" style={{ width: 6, height: 6, background: st.tone }} />
      </div>
      <div className="flex items-center gap-1" style={{ marginBottom: 2 }}>
        <Icon name={par ? "transfer-horizontal" : "arrow-right"} size={13} color="var(--ink-faint)" />
        <span className="mono text-xs text-ink-faint">{par ? "parallel · all at once" : "sequential · in order"}</span>
      </div>
      {par ? (
        <div className="flex gap-2" >
          <span className="bg-paper-edge shrink-0" style={{ width: 1, borderRadius: 1 }} />
          <div className="flex flex-col gap-2 flex-1 min-w-0" >
            {tasks.map(t => <PlanNode key={t.id} task={t} onSelect={onSelect} selected={selectedId === t.id} />)}
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-0" >
          {tasks.map((t, i) => (
            <React.Fragment key={t.id}>
              {i > 0 && (
                <span className="flex items-center pl-4" style={{ height: 16 }}>
                  <Icon name="alt-arrow-down" size={13} color="var(--ink-faint)" />
                </span>
              )}
              <PlanNode task={t} onSelect={onSelect} selected={selectedId === t.id} />
            </React.Fragment>
          ))}
        </div>
      )}
    </div>
  );
}

// The whole plan. Stages flow left→right on desktop (wrapping to a second row
// rather than scrolling out of sight), top→bottom on mobile.
function PlanGraph({ plan = [], mobile, onSelect, selectedId, legend = true }) {
  const stages = toPhases(plan);
  const arrow = (i) => mobile
    ? <div key={"a" + i} className="flex justify-center py-1 px-0" >
        <Icon name="alt-arrow-down" size={16} color="var(--ink-faint)" /></div>
    : <div key={"a" + i} className="flex items-center shrink-0" style={{ paddingTop: 30 }}>
        <Icon name="alt-arrow-right" size={16} color="var(--ink-faint)" /></div>;
  return (
    <div className="flex flex-col gap-4" >
      <div className={mobile ? "flex flex-col" : "flex flex-wrap"} style={{
        gap: mobile ? 0 : "var(--space-3)", alignItems: "stretch", rowGap: mobile ? 0 : "var(--space-6)" }}>
        {stages.map((s, i) => [
          i > 0 ? arrow(i) : null,
          <PlanStage key={s.id} stage={s} index={i} onSelect={onSelect} selectedId={selectedId} />,
        ])}
      </div>
      {legend && (
        <div className="flex flex-wrap items-center gap-4 pt-3 border-t" >
          {Object.keys(TASK_STATE).map(k => (
            <span key={k} className="flex items-center gap-1">
              <Icon name={TASK_STATE[k].icon} size={13} color={TASK_STATE[k].tone} />
              <span className="mono text-xs text-ink-mute">{TASK_STATE[k].label}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

// The plan outline — phases with their tasks, seq-numbered the way the segment
// projection orders them. Per task: title · state · agent · model · spec_ref.
function PlanOutline({ plan = [], onSelect, selectedId, mobile }) {
  const phases = toPhases(plan);
  return (
    <div className="flex flex-col">
      {phases.map((p, pi) => {
        const st = TASK_STATE[phaseState(p)] || TASK_STATE.pending;
        return (
          <div key={p.id} className={pi ? "border-t" : ""}>
            <div className="flex items-center gap-3 py-3 px-4 bg-paper-mute" >
              <span className="text-sm font-medium text-ink flex-1 min-w-0" >{p.title}</span>
              <span className="mono text-xs" style={{ color: st.tone }}>{st.label}</span>
              <span className="mono text-xs text-ink-faint">{p.tasks.filter(t => t.state === "done" || t.state === "skipped").length}/{p.tasks.length}</span>
            </div>
            {p.tasks.map(t => {
              const n = TASK_STATE[t.state] || TASK_STATE.pending;
              const on = selectedId === t.id;
              const labels = [t.agent, t.model].filter(Boolean).join(" · ");
              return (
                <button key={t.id} onClick={() => onSelect && onSelect(t)}
                  className={"flex w-full text-left border-t " + (mobile ? "flex-col gap-2" : "items-center gap-3")}
                  style={{ padding: "var(--space-3) var(--space-4)", paddingLeft: mobile ? "var(--space-4)" : "var(--space-8)",
                    background: on ? "var(--paper-mute)" : "transparent" }}>
                  <span className={mobile ? "flex items-center gap-3 w-full" : "flex items-center gap-3 flex-1"} style={{ minWidth: 0 }}>
                    <Icon name={n.icon} size={16} color={n.tone} />
                    <span className="min-w-0 flex-1" >
                      <span className="flex items-center gap-2 flex-wrap" >
                        <span className={"text-sm " + (t.state === "pending" || t.state === "skipped" ? "text-ink-mute" : "text-ink")}>{t.title}</span>
                        {t.is_gate && <Chip mono tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{t.gate_severity === "advisory" ? "gate · advisory" : "gate · blocking"}</Chip>}
                      </span>
                      {(labels || t.spec_ref || t.summary) && (
                        <span className="mono text-xs text-ink-faint block overflow-hidden text-ellipsis" style={{ marginTop: 3, whiteSpace: mobile ? "normal" : "nowrap" }}>
                          {[labels, t.spec_ref, t.summary].filter(Boolean).join(" · ")}
                        </span>
                      )}
                    </span>
                  </span>
                  <span className={mobile ? "flex items-center gap-3" : "flex items-center gap-3"} style={{ flexShrink: 0 }}>
                    {t.deps.length > 0 && <span className="mono text-xs text-ink-faint">waits on {t.deps.join(", ")}</span>}
                    <span className="mono text-xs text-right" style={{ color: n.tone, width: mobile ? undefined : 88 }}>{n.label}</span>
                  </span>
                </button>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}

// Roll the plan up into progress: which stage is live, how far along.
function planProgress(plan = []) {
  const phases = toPhases(plan);
  const tasks = phases.flatMap(s => s.tasks);
  const total = tasks.length || 1;
  const done = tasks.filter(t => t.state === "done" || t.state === "skipped").length;
  const running = tasks.filter(t => t.state === "active").length;
  const liveIdx = phases.findIndex(s => phaseState(s) !== "done");
  const stage = liveIdx === -1 ? phases.length : liveIdx + 1;
  return { done, total, running, stage, stages: phases.length,
    pct: Math.round(((done + running * 0.5) / total) * 100),
    stageName: (phases[liveIdx] || phases[phases.length - 1] || {}).title || "" };
}

// What this run wants from you right now — drives stripe, label and CTA.
function runFlag(run) {
  const tasks = allTasks(run.plan || []);
  const states = tasks.map(t => t.state);
  const gated = tasks.some(t => t.is_gate && t.state !== "done" && t.state !== "skipped");
  if (gated || states.includes("needs_review") || run.gate) return { key: "gate", label: "Needs approval", tone: "var(--accent)", cta: "Approve", act: true };
  if (states.includes("failed")) return { key: "failed", label: "Task failed", tone: "var(--danger)", cta: "Review", act: true };
  if (states.includes("blocked")) return { key: "blocked", label: "Blocked", tone: "var(--warning)", cta: "Unblock", act: true };
  if (run.state === "running") return { key: "running", label: "Running", tone: "var(--success)" };
  return { key: "waiting", label: "Waiting", tone: "var(--warning)" };
}

// A thin progress rail.
function PlanBar({ pct, tone = "var(--ink)", style }) {
  return (
    <span className="block rounded-sm bg-paper-mute overflow-hidden" style={{ height: 5, ...style }}>
      <span className="block h-full rounded-sm" style={{ width: Math.max(2, pct) + "%", background: tone }} />
    </span>
  );
}

// The run's activity feed — what sensei actually did, newest first.
function RunActivity({ feed = [] }) {
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      {feed.map((e, i) => (
        <div key={i} className="flex items-center gap-3 py-3 px-4" style={{ borderBottom: i < feed.length - 1 ? "var(--hairline)" : "none" }}>
          <Icon name={e.icon} size={16} color={e.tone} />
          <span className="text-sm text-ink-soft flex-1 min-w-0" >{e.text}</span>
          <span className="mono text-xs text-ink-faint">{e.at}</span>
        </div>
      ))}
    </div>
  );
}

// Compact stage strip for a run row — one segment per stage, tinted by roll-up
// state, with a task-count caption. Reads as "where in the plan is this run".
function PlanPips({ plan = [], showCaption = true }) {
  plan = toPhases(plan);
  if (!plan.length) return null;
  const total = plan.reduce((a, s) => a + (s.tasks || []).length, 0);
  const done = plan.reduce((a, s) => a + (s.tasks || []).filter(t => t.state === "done").length, 0);
  return (
    <span className="flex items-center gap-2 min-w-0" >
      <span className="flex items-center" style={{ gap: 3 }}>
        {plan.map(s => {
          const n = TASK_STATE[phaseState(s)] || TASK_STATE.pending;
          const par = (s.tasks || []).filter(t => !(t.deps || []).length).length > 1;
          return (
            <span key={s.id} title={s.name + " · " + s.mode} className="flex" style={{ gap: 1 }}>
              {(par ? [0, 1] : [0]).map(k => (
                <span key={k} className="rounded-full" style={{ width: par ? 5 : 12, height: 5,
                  background: n.dashed ? "transparent" : n.tone,
                  border: n.dashed ? "1px solid var(--paper-edge)" : "none" }} />
              ))}
            </span>
          );
        })}
      </span>
      {showCaption && <span className="mono text-xs text-ink-faint">{done}/{total} tasks</span>}
    </span>
  );
}

Object.assign(window, {
  CLASSIFICATION, PHASE, ROLE, DOJO_KIND, TASK_STATE, TASK_STATE_ALIAS, phaseState, toPhases, allTasks,
  PlanGraph, PlanStage, PlanNode, PlanOutline, PlanPips, PlanBar, RunActivity, planProgress, runFlag,
  KanjiMark, Icon, Chip, ClassChip, RoleTag, PhasePill, SectionHead, Banner, StatBadge, Spark, Enso, ConfidenceBar, EmptyState, Button, Card, Row, SegmentedControl, PillChoice, NumberedChoice,
  Metric, MetricRow, Toggle, DojoRow, ProjectRow,
  OrgSwitcher, TopBar, ContextHeader, NavPane, AppShell, TabBar, MobileShell, ListSection,
  LadderRung, RuleRow, ConflictCard, StanceDial,
  RunCard, GateCard, NeedsYouBand, NeedsRow, DecisionCard, ChatThread,
  RUN_STATUS, toInboxRow, toInboxRows, InboxRow, SubTabs, useAsync,
});
