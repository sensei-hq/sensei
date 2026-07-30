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
// Exports (window, K2-prefixed): shell — K2TopBar · K2OrgSwitcher · K2NavPane
//   · K2ContextHeader · K2AppShell · K2MobileShell · K2TabBar. primitives —
//   K2KanjiToken · K2Chip · K2ClassChip · K2RoleTag · K2PhasePill · K2SectionHead
//   · K2Banner · K2StatBadge · K2Spark · K2Enso · K2ConfidenceBar · K2EmptyState
//   · K2MyDojoRow · K2ProjectRow · K2Btn. governance — K2LadderRung · K2RuleRow
//   · K2ConflictCard · K2StanceDial. relay — K2RunCard · K2GateCard
//   · K2NeedsYouBand · K2NeedsRow · K2DecisionCard · K2ChatThread.

const { useState: k2S } = React;

/* ─── shared vocab maps (the only place tones are decided) ── */
const K2_CLASS = {
  company:   { kanji: "社", label: "company",   tone: "var(--ink-soft)",  soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  client:    { kanji: "客", label: "client",    tone: "var(--accent)",    soft: "var(--accent-soft)", edge: "var(--accent-edge)" },
  personal:  { kanji: "己", label: "personal",  tone: "var(--ink-mute)",  soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  community: { kanji: "群", label: "community", tone: "var(--success)",   soft: "var(--success-soft)", edge: "var(--success-edge)" },
};
const K2_PHASE = {
  watch:  { kanji: "観", label: "watch",  tone: "var(--ink-mute)", dot: "var(--ink-mute)", step: 1 },
  notice: { kanji: "察", label: "notice", tone: "var(--warning)",  dot: "var(--warning)",  step: 2 },
  adopt:  { kanji: "覚", label: "adopt",  tone: "var(--success)",  dot: "var(--success)",  step: 3 },
};
const K2_ROLE = {
  developer:  { kanji: "士", label: "developer" },
  maintainer: { kanji: "掟", label: "maintainer" },
  lead:       { kanji: "客", label: "lead" },
  admin:      { kanji: "任", label: "admin" },
};
const K2_KIND = {
  employer:  { kanji: "社", tone: "var(--ink-soft)" },
  client:    { kanji: "客", tone: "var(--accent)" },
  community: { kanji: "群", tone: "var(--success)" },
  personal:  { kanji: "己", tone: "var(--ink-mute)" },
};

// Solar Icons (bold-duotone) via Iconify, tinted with ?color= — the accessible
// working-icon layer. Rendered as <img> for reliable paint everywhere. Kanji is
// kept for brand/aesthetic marks (brand, sensei voice, ladder scopes, identity).
const K2_HEX = {
  "var(--accent)": "#A83D1F", "var(--success)": "#2F7D5B", "var(--warning)": "#B87514",
  "var(--ink)": "#2A2925", "var(--ink-soft)": "#514c44", "var(--ink-mute)": "#8f887c",
  "var(--paper)": "#F6F3ED", "var(--danger)": "#A3302A",
};
function K2Icon({ name, size = 18, color, style }) {
  const hex = K2_HEX[color] || (color && color[0] === "#" ? color : "#8f887c");
  const url = `https://api.iconify.design/solar:${name}-bold-duotone.svg?color=${encodeURIComponent(hex)}`;
  return <img src={url} width={size} height={size} alt="" aria-hidden="true" style={{ display: "inline-block", flexShrink: 0, ...style }} />;
}

/* ═══ PRIMITIVES ═══════════════════════════════════════════ */

// A single functional kanji beside nothing else — the brand mark unit.
function K2KanjiToken({ char, size = "base", tone = "var(--accent)", w, style }) {
  const px = { xs: 11, sm: 13, base: 15, lg: 17, xl: 22, "2xl": 28, "3xl": 40, "4xl": 56 }[size] || size;
  return <span className="kanji" style={{ fontSize: px, lineHeight: 1, color: tone, width: w, textAlign: w ? "center" : undefined, flexShrink: 0, ...style }}>{char}</span>;
}

// Generic pill. Kanji optional. Tone/soft/edge default to a neutral chip.
function K2Chip({ kanji, icon, children, tone = "var(--ink-mute)", soft = "var(--paper-mute)", edge = "transparent", mono, style }) {
  return (
    <span className={"inline-flex items-center gap-1 rounded-full text-xs " + (mono ? "mono" : "")}
      style={{ color: tone, background: soft, border: "1px solid " + edge, padding: "3px 9px", whiteSpace: "nowrap", letterSpacing: mono ? ".04em" : undefined, ...style }}>
      {icon && <K2Icon name={icon} size={13} color={tone} />}{kanji && <span className="kanji" style={{ fontSize: 12 }}>{kanji}</span>}{children}
    </span>
  );
}

// Classification chip — company · client · personal · community.
function K2ClassChip({ kind }) {
  const c = K2_CLASS[kind] || K2_CLASS.company;
  return (
    <span className="inline-flex items-center gap-1 rounded-full text-xs" style={{ color: c.tone, background: c.soft, border: "1px solid " + c.edge, padding: "3px 9px", whiteSpace: "nowrap" }}>
      <span className="rounded-full" style={{ width: 6, height: 6, background: c.tone, flexShrink: 0 }} />{c.label}
    </span>
  );
}

// Additive role tag.
const K2_ROLE_ICON = { developer: "code-2", maintainer: "scale", lead: "case-round", admin: "shield-user" };
function K2RoleTag({ role, muted }) {
  const r = K2_ROLE[role]; if (!r) return null;
  const c = muted ? "var(--ink-mute)" : "var(--accent)";
  return (
    <span className="mono inline-flex items-center gap-1 rounded-full text-xs"
      style={{ color: c, background: muted ? "var(--paper-mute)" : "var(--accent-soft)",
        border: "1px solid " + (muted ? "var(--paper-edge)" : "var(--accent-edge)"), padding: "3px 10px", whiteSpace: "nowrap" }}>
      <K2Icon name={K2_ROLE_ICON[role] || "user"} size={13} color={c} />{r.label}
    </span>
  );
}

// Phase pill with a 3-step track (watch → notice → adopt).
function K2PhasePill({ phase }) {
  const p = K2_PHASE[phase] || K2_PHASE.watch;
  return (
    <span className="inline-flex items-center gap-2 rounded-full text-xs" style={{ border: "1px solid var(--paper-edge)", background: "var(--paper)", padding: "3px 10px 3px 8px", whiteSpace: "nowrap" }}>
      <span className="flex items-center" style={{ gap: 3 }}>
        {[1, 2, 3].map(n => <span key={n} className="rounded-full" style={{ width: 5, height: 5, background: n <= p.step ? p.dot : "var(--paper-edge)" }} />)}
      </span>
      <span style={{ color: p.tone }}>{p.label}</span>
    </span>
  );
}

// Eyebrow + title + optional kanji + right slot. The one section header.
function K2SectionHead({ kanji, eyebrow, title, count, right, style }) {
  return (
    <div className="flex items-baseline gap-3 border-b" style={{ paddingBottom: "var(--space-3)", ...style }}>
      {kanji && <K2KanjiToken char={kanji} size="lg" tone="var(--ink-mute)" />}
      <div style={{ minWidth: 0 }}>
        {eyebrow && <div className="zs-eyebrow font-semibold mb-1">{eyebrow}</div>}
        <h2 className="display font-normal tracking-tight text-xl" style={{ margin: 0, lineHeight: 1.1 }}>{title}</h2>
      </div>
      {count != null && <span className="mono text-xs text-ink-faint">{count}</span>}
      <span className="flex-1" />
      {right}
    </div>
  );
}

// Notice band. tone: neutral | accent | success | warning.
function K2Banner({ kanji, tone = "neutral", title, children, right }) {
  const map = {
    neutral: { bg: "var(--paper-soft)", edge: "var(--paper-edge)", k: "var(--ink-mute)" },
    accent:  { bg: "var(--accent-soft)", edge: "var(--accent-edge)", k: "var(--accent)" },
    success: { bg: "var(--success-soft)", edge: "var(--success-edge)", k: "var(--success)" },
    warning: { bg: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)", k: "var(--warning)" },
  }[tone];
  return (
    <div className="flex items-start gap-3 rounded-lg" style={{ background: map.bg, border: "1px solid " + map.edge, padding: "var(--space-3) var(--space-4)" }}>
      {kanji && <K2KanjiToken char={kanji} size="lg" tone={map.k} style={{ marginTop: 1 }} />}
      <div className="flex-1" style={{ minWidth: 0 }}>
        {title && <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.3 }}>{title}</div>}
        {children && <div className="zs-body-sm" style={{ marginTop: title ? 2 : 0 }}>{children}</div>}
      </div>
      {right}
    </div>
  );
}

// Meaningful number + label — sensei's small, specific stat.
function K2StatBadge({ n, label, sub, tone = "var(--ink)" }) {
  return (
    <div className="flex flex-col" style={{ gap: 2 }}>
      <div className="display font-light tracking-tight" style={{ fontSize: "var(--text-2xl)", lineHeight: 1, color: tone }}>{n}</div>
      <div className="text-xs font-medium text-ink">{label}</div>
      {sub && <div className="zs-meta">{sub}</div>}
    </div>
  );
}

// Sparkline wrapper (uses primitives.jsx Sparkline).
function K2Spark({ data, w = 84, h = 24, color = "var(--accent)", fill = "var(--accent-soft)" }) {
  return <window.Sparkline data={data} width={w} height={h} color={color} fill={fill} />;
}
// Ensō ring wrapper.
function K2Enso(props) { return <window.EnsoRing {...props} />; }

// Confidence bar — labelled 0..1.
function K2ConfidenceBar({ v, w = 96, showN = true }) {
  const tone = v >= 0.85 ? "var(--success)" : v >= 0.7 ? "var(--accent)" : "var(--warning)";
  return (
    <div className="flex items-center gap-2">
      <div className="bg-paper-mute rounded-full" style={{ width: w, height: 4, overflow: "hidden" }}>
        <div className="rounded-full" style={{ width: (v * 100) + "%", height: "100%", background: tone }} />
      </div>
      {showN && <span className="mono text-xs text-ink-soft">{Math.round(v * 100)}</span>}
    </div>
  );
}

// Empty state — kanji anchor, one landing line, a calm second sentence.
function K2EmptyState({ kanji = "空", title = "Still listening.", children, action }) {
  return (
    <div className="flex flex-col items-center text-center" style={{ padding: "var(--space-8) var(--space-6)", gap: "var(--space-3)" }}>
      <K2KanjiToken char={kanji} size="3xl" tone="var(--ink-faint)" />
      <div className="display font-normal text-lg text-ink" style={{ letterSpacing: "-0.01em" }}>{title}</div>
      {children && <div className="zs-body-sm" style={{ maxWidth: 380, margin: 0 }}>{children}</div>}
      {action && <div style={{ marginTop: "var(--space-2)" }}>{action}</div>}
    </div>
  );
}

// Canonical button on zs-btn. variant: primary | ghost | danger. size sm|md.
function K2Btn({ variant = "primary", size = "md", kanji, icon, children, onClick, style, title }) {
  const cls = "zs-btn " + (size === "sm" ? "zs-btn-sm " : "") + (variant === "primary" ? "zs-btn-primary" : variant === "ghost" ? "bg-paper border-1px" : "");
  const skin = variant === "danger" ? { background: "var(--danger)", color: "var(--paper)", borderColor: "var(--danger)" } : null;
  const kc = variant === "danger" ? "var(--paper)" : variant === "primary" ? "var(--paper)" : "var(--accent)";
  return (
    <button title={title} onClick={onClick} className={cls} style={{ justifyContent: "center", ...skin, ...style }}>
      {icon && <K2Icon name={icon} size={size === "sm" ? 15 : 16} color={kc} />}{kanji && !icon && <span className="kanji" style={{ color: kc }}>{kanji}</span>}{children}
    </button>
  );
}

// A dōjō membership row — org identity + your role. Same shape on phone and
// desktop: kanji mark, name + kind, route, then role and what needs you.
function K2MyDojoRow({ dojo, onOpen, mobile }) {
  const kind = K2_KIND[dojo.kind] || K2_KIND.employer;
  return (
    <button onClick={() => onOpen && onOpen(dojo)} className="w-full text-left border-b"
      style={{ display: "grid", gridTemplateColumns: "30px minmax(0, 1fr)", gap: "var(--space-3)",
        padding: "var(--space-3) var(--space-4)", background: "transparent", alignItems: "start" }}>
      <span className="kanji" style={{ fontSize: 24, lineHeight: 1.1, color: kind.tone, textAlign: "center" }}>{dojo.kanji}</span>
      <span style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: 3 }}>
        <span className="flex items-baseline gap-2">
          <span className="text-sm font-medium text-ink" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{dojo.name}</span>
          <span className="mono text-xs text-ink-mute">{dojo.kind}</span>
          <span className="flex-1" />
          {dojo.needs > 0 && <span className="mono text-xs text-accent font-semibold" style={{ flexShrink: 0 }}>{dojo.needs} need you</span>}
        </span>
        <span className="mono text-xs text-ink-faint" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{dojo.route}</span>
        <span className="flex items-center gap-3" style={{ marginTop: 1 }}>
          <K2RoleTag role={dojo.role} />
          <span className="mono text-xs text-ink-faint" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{dojo.members} members · {dojo.projects} projects</span>
          {!mobile && <span className="text-ink-faint" style={{ fontSize: 16, marginLeft: "auto" }}>→</span>}
        </span>
      </span>
    </button>
  );
}

// A project row — name · classification · phase · signal. The list workhorse.
function K2ProjectRow({ p, onOpen, showDojo = true, compact = false }) {
  if (compact) {
    return (
      <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b"
        style={{ display: "grid", gridTemplateColumns: "22px minmax(0, 1fr)", gap: "var(--space-3)",
          padding: "var(--space-4)", background: "transparent", alignItems: "start" }}>
        <K2Icon name="folder" size={18} color="var(--accent)" style={{ marginTop: 2 }} />
        <span style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
          <span style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: 2 }}>
            <span className="flex items-baseline gap-2">
              <span className="text-base font-medium text-ink" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.name}</span>
              <span className="flex-1" />
              {p.needs > 0 && <span className="mono text-xs text-accent font-semibold" style={{ flexShrink: 0 }}>{p.needs} need{p.needs === 1 ? "s" : ""} you</span>}
            </span>
            <span className="mono text-xs text-ink-faint" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{p.repo}</span>
          </span>
          <span className="flex items-center gap-2" style={{ flexWrap: "wrap" }}>
            <K2ClassChip kind={p.classification} />
            <K2PhasePill phase={p.phase} />
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
    <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b"
      style={{ display: "grid", gridTemplateColumns: cols, alignItems: "center", gap: "var(--space-3)", padding: "var(--space-3) var(--space-4)", background: "transparent" }}>
      <K2Icon name="folder" size={18} color="var(--accent)" />
      <div style={{ minWidth: 0 }}>
        <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.2, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.name}</div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.repo}</div>
      </div>
      <span className="flex items-center"><K2ClassChip kind={p.classification} /></span>
      {showDojo && <span className="text-xs text-ink-mute" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.dojoName || ""}</span>}
      <span className="text-xs text-ink-mute" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.note || ""}</span>
      <span className="flex items-center" style={{ justifyContent: "flex-start" }}>{p.spark ? <K2Spark data={p.spark} /> : null}</span>
      <span className="flex items-center">{p.needs > 0 ? <K2Chip icon="bell" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{p.needs}</K2Chip> : null}</span>
      <span className="flex items-center justify-center"><K2PhasePill phase={p.phase} /></span>
      <span className="mono text-xs text-ink-faint" style={{ textAlign: "right" }}>{p.lastRun}</span>
    </button>
  );
}

/* ═══ SHELL ════════════════════════════════════════════════ */

// A titled list section: icon + eyebrow title + count + right slot, over a
// single flush card holding the rows. The one recipe for every "header + rows"
// block (live runs, active projects, members, packs…).
function K2ListSection({ icon, iconColor = "var(--accent)", title, count, countTone, right, children, style }) {
  return (
    <div style={style}>
      <div className="flex items-center gap-2" style={{ marginBottom: "var(--space-3)" }}>
        {icon && <K2Icon name={icon} size={17} color={iconColor} />}
        <span className="zs-eyebrow font-semibold text-ink">{title}</span>
        {count != null && <span className="mono text-xs" style={{ color: countTone || "var(--ink-faint)" }}>{count}</span>}
        <span className="flex-1" />
        {right}
      </div>
      <div className="zs-card-flush" style={{ overflow: "hidden" }}>{children}</div>
    </div>
  );
}

// Org switcher popover. context "you" or an org slug. Lists "your work" +
// every membership. Purely presentational click state for the kit.
function K2OrgSwitcher({ context = "you", org, dojos = [], onPick }) {
  const [open, setOpen] = k2S(false);
  const isYou = context === "you";
  return (
    <div style={{ position: "relative" }}>
      <button onClick={() => setOpen(o => !o)}
        className={"inline-flex items-center gap-2 bg-paper-soft rounded " + (open ? "border-accent" : "border-1px")}
        style={{ padding: "var(--space-1) var(--space-3)", minHeight: 32, border: open ? "1px solid var(--accent)" : undefined }}>
        <span className="kanji" style={{ fontSize: 14, color: isYou ? "var(--accent)" : (K2_KIND[org?.kind]?.tone || "var(--accent)") }}>{isYou ? "携" : org?.kanji}</span>
        <span className="text-sm text-ink">{isYou ? "Your work" : org?.name}</span>
        {!isYou && <span className="mono text-xs text-ink-faint uppercase" style={{ letterSpacing: ".08em" }}>{org?.kind}</span>}
        <span className="text-xs text-ink-mute">▾</span>
      </button>
      {open && (
        <div className="bg-paper border-1px rounded-lg shadow-lg" style={{ position: "absolute", top: "calc(100% + 6px)", left: 0, width: 320, zIndex: 60, overflow: "hidden" }}>
          <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-2) var(--space-3)" }}>
            <K2KanjiToken char="探" size="sm" tone="var(--ink-mute)" />
            <span className="flex-1 text-sm text-ink-faint">Switch context…</span>
            <span className="mono text-xs text-ink-faint bg-paper-mute rounded-sm" style={{ padding: "3px 7px" }}>⌘K</span>
          </div>
          <button onClick={() => { setOpen(false); onPick && onPick("you"); }}
            className={"flex items-center gap-3 w-full text-left border-b " + (isYou ? "bg-accent-soft" : "")}
            style={{ padding: "var(--space-3)" }}>
            <K2KanjiToken char="携" size="base" tone="var(--accent)" w={20} />
            <div className="flex-1" style={{ minWidth: 0 }}>
              <div className="text-sm text-ink font-medium">Your work</div>
              <div className="text-xs text-ink-mute">every dōjō · nothing to switch</div>
            </div>
            {isYou && <span className="text-sm text-accent">✓</span>}
          </button>
          <div className="zs-eyebrow font-semibold text-ink-mute" style={{ padding: "var(--space-3) var(--space-3) var(--space-1)" }}>My dōjōs</div>
          <div style={{ maxHeight: 260, overflow: "auto", paddingBottom: 4 }}>
            {dojos.map(d => {
              const on = !isYou && org?.slug === d.slug;
              return (
                <button key={d.slug} onClick={() => { setOpen(false); onPick && onPick(d.slug); }}
                  className={"flex items-center gap-3 w-full text-left " + (on ? "bg-paper-soft" : "")} style={{ padding: "var(--space-2) var(--space-3)" }}>
                  <span className="kanji" style={{ fontSize: 14, color: K2_KIND[d.kind]?.tone, width: 20, textAlign: "center", flexShrink: 0 }}>{d.kanji}</span>
                  <div className="flex-1" style={{ minWidth: 0 }}>
                    <div className="text-sm text-ink" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{d.name}</div>
                    <div className="mono text-xs text-ink-faint">{K2_ROLE[d.role]?.label} · {d.kind}</div>
                  </div>
                  {d.needs > 0 && <span className="mono text-xs font-semibold bg-accent rounded-full" style={{ color: "var(--paper)", padding: "0 7px", lineHeight: "16px" }}>{d.needs}</span>}
                  {on && <span className="text-sm text-accent">✓</span>}
                </button>
              );
            })}
          </div>
          <button className="flex items-center gap-2 w-full text-left border-t text-sm text-ink-soft" style={{ padding: "var(--space-3)" }}>
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
function K2TopBar({ context, org, dojos, me, onPick, needsCount = 0, onNeeds }) {
  const isOrg = context === "org";
  return (
    <div className="flex items-center gap-3 border-b bg-paper px-4" style={{ height: 56, flexShrink: 0, borderTop: isOrg ? "2px solid var(--accent)" : undefined }}>
      <div className="flex items-baseline gap-2" style={{ flexShrink: 0 }}>
        <span className="kanji text-accent" style={{ fontSize: 20, lineHeight: 1 }}>結</span>
        <span className="display text-lg tracking-tight">Dōjō</span>
      </div>
      <K2OrgSwitcher context={context} org={org} dojos={dojos} onPick={onPick} />
      {isOrg && <span className="mono text-xs text-ink-mute bg-paper-soft rounded-full" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px" }}>{org.route}</span>}
      {isOrg && <K2RoleTag role={org.role} />}
      <span className="flex-1" />
      <div className="zs-input text-sm" style={{ width: 240, height: 34, padding: "0 12px" }}>
        <K2KanjiToken char="探" size="sm" tone="var(--ink-mute)" />
        <span className="text-ink-faint" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>search…</span>
      </div>
      <button onClick={onNeeds} title={needsCount ? needsCount + " need you" : "Nothing needs you"} className="flex items-center justify-center rounded-full"
        style={{ position: "relative", width: 34, height: 34, flexShrink: 0, background: needsCount ? "var(--accent-soft)" : "transparent", border: "1px solid " + (needsCount ? "var(--accent-edge)" : "var(--paper-edge)") }}>
        <K2Icon name="bell" size={17} color={needsCount ? "var(--accent)" : "var(--ink-mute)"} />
        {needsCount > 0 && <span className="mono" style={{ position: "absolute", top: -5, right: -5, minWidth: 16, height: 16, padding: "0 4px", borderRadius: 8, background: "var(--accent)", color: "var(--paper)", fontSize: 10, lineHeight: "16px", textAlign: "center", fontWeight: 600 }}>{needsCount}</span>}
      </button>
      <window.Avatar name={me?.name || "You"} size={30} />
    </div>
  );
}

// Context header — the band under the top bar that tells you WHERE you are.
// Personal ("you") vs an org. This is the only piece that changes shape.
function K2ContextHeader({ context, org, me }) {
  if (context === "you") {
    return (
      <div className="flex items-center gap-3 border-b bg-paper-soft px-4" style={{ height: 46, flexShrink: 0 }}>
        <K2KanjiToken char="携" size="lg" tone="var(--accent)" />
        <span className="text-sm font-medium text-ink">Your work</span>
        <span className="text-sm text-ink-mute">— everything in flight, across every dōjō</span>
        <span className="flex-1" />
        <span className="zs-meta">{me?.name}</span>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-3 px-4" style={{ height: 46, flexShrink: 0, background: "var(--paper-soft)", borderBottom: "var(--hairline)", borderTop: "2px solid var(--accent)" }}>
      <K2KanjiToken char={org.kanji} size="lg" tone={K2_KIND[org.kind]?.tone || "var(--accent)"} />
      <span className="display text-lg tracking-tight" style={{ whiteSpace: "nowrap" }}>{org.name}</span>
      <span className="mono text-xs text-ink-mute bg-paper rounded-full" style={{ border: "1px solid var(--paper-edge)", padding: "3px 10px" }}>{org.route}</span>
      <span className="flex-1" />
      <K2RoleTag role={org.role} />
    </div>
  );
}

// Left nav — grouped items + version footer. Groups: [{group, items:[{id,kanji,label,badge}]}].
function K2NavPane({ groups = [], active, onNav, width = 222 }) {
  return (
    <aside className="flex flex-col border-r bg-paper-soft" style={{ width, flexShrink: 0, padding: "var(--space-4) var(--space-3)", overflow: "auto" }}>
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
                  <span className="flex items-center justify-center" style={{ width: 16 }}>{it.icon ? <K2Icon name={it.icon} size={17} color={on ? "var(--accent)" : "var(--ink-mute)"} /> : <span className={"kanji " + (on ? "text-accent" : "text-ink-mute")} style={{ fontSize: 14 }}>{it.kanji}</span>}</span>
                  <span style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{it.label}</span>
                  {it.badge != null
                    ? <span className="mono text-xs font-semibold bg-accent rounded-full" style={{ color: "var(--paper)", padding: "0 7px", lineHeight: "16px" }}>{it.badge}</span>
                    : <span />}
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <span className="flex-1" style={{ minHeight: 12 }} />
      <div className="mono border-t" style={{ padding: "var(--space-3) var(--space-2) 0", fontSize: "var(--text-xs)", color: "var(--ink-faint)", display: "flex", alignItems: "center", gap: "var(--space-1)" }}>
        <span className="kanji">結</span>Dōjō v0.4.2
      </div>
    </aside>
  );
}

// The one shell. context "you" | "org". nav groups + context header differ;
// everything else identical. Pass `main` (or children) for content.
function K2AppShell({ label, context = "you", org, dojos = [], me, nav = [], active, onNav, onPick, needsCount, onNeeds, children }) {
  return (
    <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <K2TopBar context={context} org={org} dojos={dojos} me={me} onPick={onPick} needsCount={needsCount} onNeeds={onNeeds} />
      <div className="flex" style={{ flex: 1, minHeight: 0 }}>
        <K2NavPane groups={nav} active={active} onNav={onNav} />
        <div className="flex-1" style={{ minWidth: 0, overflow: "auto" }}>{children}</div>
      </div>
    </div>
  );
}

// Mobile bottom tab bar.
function K2TabBar({ tabs = [], active, onNav }) {
  return (
    <div className="grid border-t bg-paper" style={{ flexShrink: 0, gridTemplateColumns: `repeat(${tabs.length}, 1fr)` }}>
      {tabs.map(it => {
        const on = active === it.id;
        return (
          <button key={it.id} onClick={() => onNav && onNav(it.id)} className={"flex flex-col items-center gap-1 " + (on ? "text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-1) var(--space-3)", position: "relative" }}>
            {it.icon ? <K2Icon name={it.icon} size={20} color={on ? "var(--accent)" : "var(--ink-mute)"} /> : <span className={"kanji " + (on ? "text-accent" : "text-ink-mute")} style={{ fontSize: 17 }}>{it.kanji}</span>}
            <span className={"text-xs " + (on ? "font-semibold" : "font-normal")} style={{ whiteSpace: "nowrap" }}>{it.label}</span>
            {it.badge != null && <span className="mono text-xs font-semibold bg-accent rounded-full" style={{ color: "var(--paper)", position: "absolute", top: 4, left: "50%", marginLeft: 6, padding: "0 6px", lineHeight: "14px" }}>{it.badge}</span>}
          </button>
        );
      })}
    </div>
  );
}

// Mobile shell — condensed top bar (context) + scrolling main + bottom tabs.
function K2MobileShell({ label, context = "you", org, me, tabs = [], active, onNav, children }) {
  const isYou = context === "you";
  return (
    <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", overflow: "hidden", background: "var(--paper)" }}>
      <div className="flex items-center gap-3 border-b bg-paper" style={{ flexShrink: 0, padding: "var(--space-3) var(--space-4)", borderTop: isYou ? "none" : "2px solid var(--accent)" }}>
        <span className="kanji text-accent" style={{ fontSize: 19, lineHeight: 1 }}>{isYou ? "結" : org?.kanji}</span>
        <div className="flex-1" style={{ minWidth: 0 }}>
          <div className="text-sm font-semibold text-ink" style={{ lineHeight: 1.1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{isYou ? "Your work" : org?.name}</div>
          <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{isYou ? "every dōjō" : K2_ROLE[org?.role]?.label + " · " + org?.kind}</div>
        </div>
        <button title="Search" className="flex items-center justify-center rounded-full bg-paper-soft border-1px" style={{ width: 34, height: 34, flexShrink: 0 }}>
          <img src="https://api.iconify.design/solar:magnifer-bold-duotone.svg?color=%23736c60" width="18" height="18" alt="Search" />
        </button>
        <window.Avatar name={me?.name || "You"} size={28} />
      </div>
      <div className="flex flex-col" style={{ flex: 1, minHeight: 0, overflow: "auto" }}>{children}</div>
      <K2TabBar tabs={tabs} active={active} onNav={onNav} />
    </div>
  );
}

/* ═══ GOVERNANCE ═══════════════════════════════════════════ */

// One rung of the constitution ladder — scope identity + its rules + lock count.
function K2LadderRung({ rung, active, onSelect, showRules = true }) {
  const on = active === rung.id;
  const tone = rung.tone === "accent" ? "var(--accent)" : "var(--ink-soft)";
  const locks = (rung.rules || []).filter(r => r.hard).length;
  return (
    <div className="rounded-lg" style={{ border: on ? "1px solid var(--accent)" : "var(--hairline)", background: on ? "var(--accent-soft)" : "var(--paper-2)", overflow: "hidden" }}>
      <button onClick={() => onSelect && onSelect(rung.id)} className="flex items-center gap-3 w-full text-left" style={{ padding: "var(--space-3) var(--space-4)", background: "transparent" }}>
        <span className="kanji" style={{ fontSize: 22, lineHeight: 1, color: tone, width: 28, textAlign: "center", flexShrink: 0 }}>{rung.kanji}</span>
        <div className="flex-1" style={{ minWidth: 0 }}>
          <div className="flex items-center gap-2">
            <span className="zs-eyebrow font-semibold" style={{ color: tone }}>{rung.scope}</span>
            <span className="text-sm font-medium text-ink">{rung.name}</span>
          </div>
          <div className="zs-meta" style={{ marginTop: 1 }}>{rung.caption}</div>
        </div>
        <span className="mono text-xs text-ink-faint">{(rung.rules || []).length} rules</span>
        {locks > 0 && <K2Chip icon="lock-keyhole" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{locks} locked</K2Chip>}
      </button>
      {showRules && (
        <div className="border-t" style={{ background: "var(--paper)" }}>
          {(rung.rules || []).map((r, i) => <K2RuleRow key={i} rule={r} />)}
        </div>
      )}
    </div>
  );
}

// A rule row — include toggle · rule · level pill · ★ non-negotiable · edit.
function K2RuleRow({ rule, onToggle, included = true, showLevel, onEdit, onJump }) {
  return (
    <div className="flex items-center gap-3 border-b" style={{ padding: "var(--space-2) var(--space-4)" }}>
      {onToggle ? (
        <button onClick={() => onToggle(rule)} className="rounded-sm" style={{ width: 16, height: 16, flexShrink: 0, border: "1px solid " + (included ? "var(--accent)" : "var(--ink-faint)"), background: included ? "var(--accent)" : "transparent", color: "var(--paper)", fontSize: 11, lineHeight: "14px", textAlign: "center" }}>{included ? "✓" : ""}</button>
      ) : (
        <span className="kanji text-ink-mute" style={{ fontSize: 13, width: 16, textAlign: "center", flexShrink: 0 }}>{rule.kanji}</span>
      )}
      <span className="text-sm text-ink flex-1" style={{ lineHeight: 1.35, opacity: included ? 1 : 0.5 }}>{rule.text}</span>
      {showLevel && rule.level && (onJump
        ? <button onClick={() => onJump(rule)} title={"Jump to " + rule.level} className="mono inline-flex items-center gap-1 rounded-full text-xs" style={{ color: "var(--ink-mute)", background: "var(--paper-mute)", border: "1px solid var(--paper-edge)", padding: "3px 9px", cursor: "pointer", whiteSpace: "nowrap" }}>{rule.level}<K2Icon name="arrow-right-up" size={12} color="var(--ink-mute)" /></button>
        : <K2Chip mono tone="var(--ink-mute)">{rule.level}</K2Chip>)}
      {rule.hard && <span className="inline-flex items-center gap-1 text-xs text-accent" style={{ whiteSpace: "nowrap" }}><span style={{ fontSize: 12 }}>★</span>non-negotiable</span>}
      {onEdit && <button onClick={() => onEdit(rule)} title="Edit rule" className="flex items-center" style={{ background: "transparent", flexShrink: 0 }}><K2Icon name="pen-2" size={15} color="var(--ink-mute)" /></button>}
    </div>
  );
}

// Conflict card — topic · winner · why, with a lock marker when a ★ decided it.
function K2ConflictCard({ conflict }) {
  const c = conflict;
  return (
    <div className="zs-card-flush" style={{ overflow: "hidden" }}>
      <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
        <K2Icon name="danger-triangle" size={17} color="var(--warning)" />
        <span className="text-sm font-medium text-ink flex-1">{c.topic}</span>
        {c.locked ? <K2Chip icon="lock-keyhole" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">locked</K2Chip>
          : <K2Chip tone="var(--ink-mute)">settled</K2Chip>}
      </div>
      <div className="flex" style={{ padding: "var(--space-4)", gap: "var(--space-3)" }}>
        <div className="flex-1 rounded" style={{ padding: "var(--space-3)", background: "var(--paper-mute)", opacity: 0.7 }}>
          <div className="zs-eyebrow text-ink-mute mb-1">{c.loser.level} · yields</div>
          <div className="text-sm text-ink-soft" style={{ textDecoration: "line-through", textDecorationColor: "var(--ink-faint)" }}>{c.loser.text}</div>
        </div>
        <div className="flex items-center" style={{ color: "var(--ink-faint)" }}>→</div>
        <div className="flex-1 rounded" style={{ padding: "var(--space-3)", background: "var(--success-soft)", border: "1px solid var(--success-edge)" }}>
          <div className="zs-eyebrow mb-1" style={{ color: "var(--success)" }}>{c.winner.level} · wins</div>
          <div className="text-sm text-ink font-medium">{c.winner.text}</div>
        </div>
      </div>
      <div className="border-t zs-body-sm" style={{ padding: "var(--space-3) var(--space-4)", background: "var(--paper)" }}>{c.why}</div>
    </div>
  );
}

// Stance dial — a labelled discrete slider (autonomy / sharing / review).
function K2StanceDial({ dial, onChange }) {
  const [v, setV] = k2S(dial.value);
  const n = dial.levels.length;
  return (
    <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
      <div className="flex items-center gap-2 mb-1">
        <K2Icon name={{ autonomy: "cpu-bolt", sharing: "share-circle", review: "checklist-minimalistic" }[dial.id] || "settings"} size={17} color="var(--accent)" />
        <span className="text-sm font-medium text-ink">{dial.label}</span>
        <span className="flex-1" />
        <span className="mono text-xs text-accent">{dial.levels[v]}</span>
      </div>
      <div className="zs-meta mb-3">{dial.caption}</div>
      <div className="flex items-center" style={{ gap: 0 }}>
        {dial.levels.map((lv, i) => (
          <React.Fragment key={i}>
            <button onClick={() => { setV(i); onChange && onChange(dial.id, i); }} title={lv}
              className="rounded-full" style={{ width: 14, height: 14, flexShrink: 0, border: "2px solid " + (i <= v ? "var(--accent)" : "var(--paper-edge)"), background: i <= v ? "var(--accent)" : "var(--paper)" }} />
            {i < n - 1 && <span style={{ flex: 1, height: 2, background: i < v ? "var(--accent)" : "var(--paper-edge)" }} />}
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
function K2SubTabs({ tabs = [], active, onPick }) {
  return (
    <div className="flex gap-2" style={{ flexWrap: "wrap" }}>
      {tabs.map(x => {
        const on = x.id === active;
        return (
          <button key={x.id} onClick={() => onPick && onPick(x.id)}
            className={"inline-flex items-center gap-2 rounded text-sm " + (on ? "bg-paper-soft border-1px text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-3)", border: on ? undefined : "1px solid transparent" }}>
            {x.icon && <K2Icon name={x.icon} size={15} color={on ? "var(--accent)" : "var(--ink-mute)"} />}
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
function K2RunCard({ run, onOpen, onAct, flat, stacked, selected }) {
  const f = k2RunFlag(run);
  const pr = k2PlanProgress(run.plan || []);
  const plan = k2Phases(run.plan || []);
  const tasks = plan.flatMap(s => s.tasks || []);
  const nowTask = tasks.find(t => t.state === "active")
    || tasks.find(t => t.is_gate || t.state === "needs_review" || t.state === "failed");
  const nowLine = (
    <span className="text-xs text-ink-mute" style={{ display: "block", marginTop: 3 }}>
      <span style={{ color: f.tone, fontWeight: 600 }}>{f.label}</span>
      {nowTask ? <span> · Now: {nowTask.title || nowTask.name}</span> : null}
    </span>
  );
  const meta = <span className="mono text-xs text-ink-faint">{run.project} · {run.id} · {run.elapsed}</span>;
  const progress = plan.length ? (
    <span style={{ display: "block" }}>
      <K2PlanBar pct={pr.pct} tone={f.tone} />
      <span className="flex items-center gap-2" style={{ marginTop: 5 }}>
        <span className="mono text-xs text-ink-faint">stage {pr.stage}/{pr.stages} · {pr.pct}%</span>
        <K2PlanPips plan={plan} showCaption={false} />
      </span>
    </span>
  ) : null;
  const cta = f.act
    ? <K2Btn size="sm" variant="primary" onClick={(e) => { e && e.stopPropagation && e.stopPropagation(); onAct ? onAct(run, f) : onOpen && onOpen(run); }}>{f.cta}</K2Btn>
    : <K2Btn size="sm" variant="ghost" onClick={(e) => { e && e.stopPropagation && e.stopPropagation(); onOpen && onOpen(run); }}>Watch →</K2Btn>;

  if (stacked) {
    return (
      <div className="flex flex-col w-full border-b" style={{ background: selected ? "var(--paper-mute)" : "transparent" }}>
        <div className="flex" style={{ minWidth: 0 }}>
          <span style={{ width: 3, flexShrink: 0, background: f.act ? f.tone : "transparent" }} />
          <button onClick={() => onOpen && onOpen(run)} className="flex flex-col flex-1 text-left"
            style={{ padding: "var(--space-3) var(--space-4) var(--space-2)", gap: "var(--space-2)", background: "transparent", minWidth: 0 }}>
            <span style={{ minWidth: 0 }}>
              <span className="text-sm font-medium text-ink" style={{ display: "block", lineHeight: 1.3 }}>{run.task}</span>
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
      <span style={{ width: 3, flexShrink: 0, background: f.act ? f.tone : "transparent" }} />
      <button onClick={() => onOpen && onOpen(run)} className="flex items-center gap-4 flex-1 text-left"
        style={{ padding: flat ? "var(--space-3) var(--space-4)" : "var(--space-4)", background: "transparent", minWidth: 0 }}>
        <K2Icon name="eye" size={22} color={f.tone} />
        <span className="flex-1" style={{ minWidth: 0 }}>
          <span className="flex items-center gap-2">
            <span className="text-sm font-medium text-ink">{run.task}</span>
            <K2Chip mono tone="var(--ink-mute)">{run.assistant}</K2Chip>
          </span>
          {nowLine}
          <span style={{ display: "block", marginTop: 3 }}>{meta}</span>
        </span>
        {progress ? <span style={{ width: 168, flexShrink: 0 }}>{progress}</span> : null}
      </button>
      <span className="flex items-center" style={{ paddingRight: "var(--space-4)", flexShrink: 0 }}>{cta}</span>
    </div>
  );
}

// ── inbox ──────────────────────────────────────────────
// One row per in-flight session. Status, progress, why it's surfaced, and how
// long since it last said anything. Everything answerable lives in the detail.
const K2_STATUS = {
  running: { label: "running", tone: "var(--success)", soft: "var(--success-soft)" },
  waiting: { label: "waiting", tone: "var(--ink-soft)", soft: "var(--paper-mute)" },
  paused:  { label: "paused",  tone: "var(--ink-mute)", soft: "var(--paper-mute)" },
  stalled: { label: "stalled", tone: "var(--warning)", soft: "var(--warning-soft)" },
  blocked: { label: "blocked", tone: "var(--warning)", soft: "var(--warning-soft)" },
  failed:  { label: "failed",  tone: "var(--danger)",  soft: "var(--danger-soft)" },
  done:    { label: "done",    tone: "var(--ink-mute)", soft: "var(--paper-mute)" },
};
// Roll a run + its pending items into an inbox row.
function k2InboxRow(run, needs = 0) {
  const pr = k2PlanProgress(run.plan || []);
  const states = k2Tasks(run.plan || []).map(t => t.state);
  let status = run.state;
  if (status !== "done" && states.includes("failed")) status = "failed";
  else if (run.stale) status = "stalled";
  else if (status !== "done" && !states.includes("active") && states.includes("blocked")) status = "blocked";
  const attention = needs > 0 ? "gate" : (status === "stalled" || status === "blocked" || status === "failed") ? status : null;
  const rank = needs > 0 ? 0 : attention ? 1 : status === "running" ? 2 : status === "done" ? 4 : 3;
  return { run, needs, status, attention, rank, done: pr.done, total: pr.total, pct: pr.pct };
}
function k2InboxRows(runs = [], pendingFor) {
  return runs.map(r => k2InboxRow(r, pendingFor ? pendingFor(r).length : 0))
    .sort((a, b) => a.rank - b.rank);
}
function K2InboxRow({ row, selected, onOpen }) {
  const s = K2_STATUS[row.status] || K2_STATUS.waiting;
  const r = row.run;
  const attn = row.needs > 0 ? "var(--accent)" : row.attention ? s.tone : null;
  const why = row.needs > 0
    ? row.needs + (row.needs === 1 ? " needs you" : " need you")
    : row.attention === "stalled" ? "no heartbeat"
    : row.attention === "blocked" ? "blocked on a task"
    : row.attention === "failed" ? "a task failed" : null;
  return (
    <div className="border-b" style={{ background: selected ? "var(--paper-mute)" : "transparent" }}>
      <button onClick={() => onOpen && onOpen(r)} className="w-full text-left"
        style={{ display: "grid", gridTemplateColumns: "10px minmax(0, 1fr)", gap: "var(--space-3)",
          padding: "var(--space-3) var(--space-4)", background: "transparent" }}>
        <span className="rounded-full" style={{ width: 7, height: 7, marginTop: 6,
          background: attn || (row.status === "running" ? s.tone : "transparent"),
          border: attn || row.status === "running" ? "none" : "1px solid var(--ink-faint)" }} />
        <span style={{ minWidth: 0, display: "flex", flexDirection: "column", gap: 3 }}>
          <span className="flex items-baseline gap-2">
            <span className="mono text-xs text-ink-mute" style={{ minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.project}</span>
            <span className="flex-1" />
            <span className="mono text-xs text-ink-faint" style={{ flexShrink: 0 }}>{r.last}</span>
          </span>
          <span className={"text-sm " + (row.status === "done" ? "text-ink-mute" : "text-ink font-medium")}
            style={{ lineHeight: 1.35, display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>{r.task}</span>
          <span className="flex items-center gap-2" style={{ marginTop: 1 }}>
            <span className="text-xs" style={{ color: attn || "var(--ink-mute)", fontWeight: attn ? 600 : 400, whiteSpace: "nowrap" }}>
              {why || s.label}
            </span>
            <span className="flex-1" />
            <K2PlanPips plan={r.plan} showCaption={false} />
            <span className="mono text-xs text-ink-faint" style={{ flexShrink: 0 }}>{row.done}/{row.total}</span>
          </span>
        </span>
      </button>
    </div>
  );
}

// A gate card — command awaiting approve / deny.
function K2GateCard({ gate, onApprove, onDeny }) {
  const high = gate.risk === "high";
  return (
    <div className="zs-card-flush" style={{ overflow: "hidden" }}>
      <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
        <K2Icon name="command" size={18} color="var(--accent)" />
        <span className="text-sm font-medium text-ink flex-1">{gate.project}</span>
        <K2Chip mono tone={high ? "var(--danger)" : "var(--warning)"} soft={high ? "var(--danger-soft)" : "var(--warning-soft)"} edge={high ? "var(--danger-edge)" : "oklch(0.72 0.12 75 / 0.30)"}>{gate.risk}</K2Chip>
        <span className="mono text-xs text-ink-faint">{gate.age}</span>
      </div>
      <div style={{ padding: "var(--space-4)" }}>
        <div className="mono text-sm text-ink bg-paper-mute rounded" style={{ padding: "var(--space-3)", border: "var(--hairline)", overflowX: "auto" }}>$ {gate.cmd}</div>
        <div className="zs-body-sm" style={{ marginTop: "var(--space-2)" }}>{gate.why} · session {gate.session}</div>
        <div className="flex flex-wrap gap-2" style={{ marginTop: "var(--space-3)" }}>
          <K2Btn size="sm" icon="check-circle" onClick={onApprove}>Approve once</K2Btn>
          <K2Btn size="sm" variant="ghost" onClick={onApprove}>Always allow</K2Btn>
          <span className="flex-1" />
          <K2Btn size="sm" variant="ghost" icon="close-circle" onClick={onDeny}>Deny</K2Btn>
        </div>
      </div>
    </div>
  );
}

// The needs-you band — a header + rows of things waiting on the viewer.
const K2_NEEDS_TONE = {
  gate:     { icon: "command",         label: "approve", tone: "var(--accent)" },
  conflict: { icon: "danger-triangle", label: "settle",  tone: "var(--warning)" },
  decision: { icon: "checklist-minimalistic", label: "decide", tone: "var(--accent)" },
  review:   { icon: "clipboard-check", label: "review",  tone: "var(--ink-soft)" },
};
// Action sets per needs-kind — the band is a remote control, so you act here.
const K2_NEEDS_ACTIONS = {
  gate:     [{ id: "approve", label: "Approve", icon: "check-circle", primary: true }, { id: "deny", label: "Deny", icon: "close-circle" }],
  conflict: [{ id: "settle", label: "Settle", icon: "scale", primary: true }],
  decision: [{ id: "decide", label: "Decide", icon: "checklist-minimalistic", primary: true }],
  review:   [{ id: "approve", label: "Approve", icon: "check-circle", primary: true }, { id: "deny", label: "Decline", icon: "close-circle" }],
};
function K2NeedsActions({ item, onAct, size = "sm" }) {
  const acts = K2_NEEDS_ACTIONS[item.kind] || K2_NEEDS_ACTIONS.decision;
  return (
    <div className="flex items-center gap-2" style={{ flexShrink: 0 }}>
      {acts.map(a => (
        <K2Btn key={a.id} size={size} variant={a.primary ? "primary" : "ghost"} icon={a.icon}
          onClick={(e) => { if (e && e.stopPropagation) e.stopPropagation(); onAct && onAct(item, a); }}>{a.label}</K2Btn>
      ))}
    </div>
  );
}
function K2NeedsResolved({ label }) {
  return (
    <span className="inline-flex items-center gap-1 text-xs" style={{ color: "var(--success)", whiteSpace: "nowrap", flexShrink: 0 }}>
      <K2Icon name="check-circle" size={15} color="var(--success)" />{label}
    </span>
  );
}
function K2NeedsRow({ item, onOpen, onAct, resolved, stacked }) {
  const t = K2_NEEDS_TONE[item.kind] || K2_NEEDS_TONE.decision;
  const done = resolved && resolved[item.id];
  if (stacked) {
    return (
      <div className="flex flex-col w-full border-b" style={{ padding: "var(--space-3) var(--space-4)", gap: "var(--space-2)", opacity: done ? 0.7 : 1 }}>
        <button onClick={() => onOpen && onOpen(item)} className="flex items-center gap-2 text-left" style={{ background: "transparent" }}>
          <K2Icon name={t.icon} size={18} color={t.tone} />
          <span className="text-sm font-medium text-ink flex-1" style={{ lineHeight: 1.3 }}>{item.title}</span>
        </button>
        <div className="zs-meta">{item.project} · {item.dojo} · {item.why}</div>
        <div className="flex items-center gap-2">
          {done ? <K2NeedsResolved label={done} />
            : <K2NeedsActions item={item} onAct={onAct} />}
          <span className="flex-1" />
          <span className="mono text-xs text-ink-faint">{item.age}</span>
        </div>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-3 w-full border-b" style={{ padding: "var(--space-3) var(--space-4)", opacity: done ? 0.7 : 1 }}>
      <button onClick={() => onOpen && onOpen(item)} className="flex items-center gap-3 flex-1 text-left" style={{ minWidth: 0, background: "transparent" }}>
        <span className="flex items-center justify-center" style={{ width: 22, flexShrink: 0 }}><K2Icon name={t.icon} size={19} color={t.tone} /></span>
        <div className="flex-1" style={{ minWidth: 0 }}>
          <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.25 }}>{item.title}</div>
          <div className="zs-meta" style={{ marginTop: 1 }}>{item.project} · {item.dojo} · {item.why}</div>
        </div>
      </button>
      {done ? <K2NeedsResolved label={done} />
        : <K2NeedsActions item={item} onAct={onAct} />}
      <span className="mono text-xs text-ink-faint" style={{ width: 30, textAlign: "right", flexShrink: 0 }}>{item.age}</span>
    </div>
  );
}
function K2NeedsYouBand({ items = [], onOpen, onAct, resolved, title = "Needs you", mobile }) {
  const open = items.filter(it => !(resolved && resolved[it.id])).length;
  return (
    <div className="rounded-lg" style={{ border: "1px solid var(--accent-edge)", background: "var(--paper-2)", overflow: "hidden" }}>
      <div className="flex items-center gap-2" style={{ padding: "var(--space-3) var(--space-4)", background: "var(--accent-soft)", borderBottom: "1px solid var(--accent-edge)" }}>
        <K2Icon name="bell" size={17} color="var(--accent)" />
        <span className="zs-eyebrow font-semibold" style={{ color: "var(--accent)" }}>{title}</span>
        <span className="mono text-xs" style={{ color: "var(--accent)" }}>{open}</span>
        <span className="flex-1" />
        {!mobile && <span className="zs-meta">{open ? "act here — nothing routes you away" : "nothing else is blocked on you"}</span>}
      </div>
      {items.length ? items.map(it => <K2NeedsRow key={it.id} item={it} onOpen={onOpen} onAct={onAct} resolved={resolved} stacked={mobile} />)
        : <K2EmptyState kanji="静" title="Nothing needs you.">Sessions run within the rules you set. sensei will surface only what it can't decide alone.</K2EmptyState>}
    </div>
  );
}

// A decision card — sign off with options.
function K2DecisionCard({ decision, onChoose }) {
  const d = decision;
  return (
    <div className="zs-card-flush" style={{ overflow: "hidden" }}>
      <div className="flex items-start gap-3" style={{ padding: "var(--space-4)" }}>
        <K2Icon name="checklist-minimalistic" size={22} color="var(--accent)" />
        <div className="flex-1" style={{ minWidth: 0 }}>
          <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.3 }}>{d.title}</div>
          <div className="zs-body-sm" style={{ marginTop: 3 }}>{d.project} · {d.context}</div>
        </div>
        <span className="mono text-xs text-ink-faint">{d.age}</span>
      </div>
      <div className="flex flex-wrap gap-2 border-t" style={{ padding: "var(--space-3) var(--space-4)", background: "var(--paper)" }}>
        {d.options.map((o, i) => (
          <K2Btn key={i} size="sm" variant={i === 0 ? "primary" : "ghost"} icon={i === 0 ? "check-circle" : undefined} onClick={() => onChoose && onChoose(o)}>{o}</K2Btn>
        ))}
      </div>
    </div>
  );
}

// A chat thread — sensei speaks rarely; the viewer replies.
function K2ChatThread({ thread = [], me }) {
  return (
    <div className="flex flex-col" style={{ gap: "var(--space-4)" }}>
      {thread.map((m, i) => {
        const mine = m.who !== "sensei";
        return (
          <div key={i} className="flex" style={{ gap: "var(--space-3)", flexDirection: mine ? "row-reverse" : "row" }}>
            {mine ? <window.Avatar name={me?.name || "You"} size={28} />
              : <K2KanjiToken char={m.kanji || "先"} size="lg" tone="var(--accent)" style={{ marginTop: 2 }} />}
            <div style={{ maxWidth: 460 }}>
              <div className="rounded-lg" style={{ padding: "var(--space-3) var(--space-4)", background: mine ? "var(--paper-mute)" : "var(--paper-2)", border: "var(--hairline)" }}>
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
const K2_NODE = {
  done:         { icon: "check-circle",              label: "done",         tone: "var(--ink-mute)",  bg: "var(--paper-2)",      edge: "var(--paper-edge)" },
  active:       { icon: "play-circle",               label: "active",       tone: "var(--success)",   bg: "var(--success-soft)", edge: "var(--success-edge)" },
  needs_review: { icon: "shield-warning",            label: "needs review", tone: "var(--accent)",    bg: "var(--accent-soft)",  edge: "var(--accent-edge)" },
  blocked:      { icon: "lock-keyhole-minimalistic", label: "blocked",      tone: "var(--warning)",   bg: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)" },
  failed:       { icon: "close-circle",              label: "failed",       tone: "var(--danger)",    bg: "var(--danger-soft)",  edge: "var(--danger-edge)" },
  skipped:      { icon: "forward",                   label: "skipped",      tone: "var(--ink-faint)", bg: "var(--paper-2)",      edge: "var(--paper-edge)" },
  pending:      { icon: "clock-circle",              label: "pending",      tone: "var(--ink-faint)", bg: "transparent",         edge: "var(--paper-edge)", dashed: true },
};
// Older authored plans used stage/queued/running/gate — read them too.
const K2_STATE_ALIAS = { queued: "pending", running: "active", gate: "needs_review" };
// Normalize any plan into phases of tasks. Accepts the authored object shape
// and the legacy array-of-stages shape.
function k2Phases(plan) {
  const phases = Array.isArray(plan) ? plan : (plan && plan.phases) || [];
  return phases.map((p, i) => ({
    id: p.id || "p" + i,
    title: p.title || p.name || "Phase " + (i + 1),
    tasks: (p.tasks || []).map(t => ({
      ...t,
      title: t.title || t.name,
      summary: t.summary || t.meta,
      state: K2_STATE_ALIAS[t.state] || t.state || "pending",
      deps: t.deps || [],
    })),
  }));
}
function k2Tasks(plan) { return k2Phases(plan).flatMap(p => p.tasks); }
// Phase rolls up to the most urgent state among its tasks.
function k2StageState(stage) {
  const s = (stage.tasks || []).map(t => K2_STATE_ALIAS[t.state] || t.state);
  return ["failed", "needs_review", "blocked", "active", "pending"].find(k => s.includes(k)) || "done";
}

// One task node.
function K2PlanNode({ task, onSelect, selected }) {
  const n = K2_NODE[K2_STATE_ALIAS[task.state] || task.state] || K2_NODE.pending;
  return (
    <button onClick={() => onSelect && onSelect(task)}
      className="flex items-start gap-2 w-full text-left rounded"
      style={{ padding: "var(--space-2) var(--space-3)", background: n.bg,
        border: (n.dashed ? "1px dashed " : "1px solid ") + (selected ? n.tone : n.edge),
        boxShadow: selected ? "inset 0 0 0 1px " + n.tone : "none" }}>
      <K2Icon name={n.icon} size={16} color={n.tone} style={{ marginTop: 1 }} />
      <span style={{ minWidth: 0, flex: 1 }}>
        <span className={"text-sm " + (task.state === "pending" ? "text-ink-mute" : "text-ink")}
          style={{ display: "block", lineHeight: 1.35 }}>{task.title || task.name}</span>
        {task.summary || task.meta ? <span className="zs-meta" style={{ display: "block", marginTop: 2 }}>{task.summary || task.meta}</span> : null}
      </span>
    </button>
  );
}

// The stage column: header + its tasks, wired parallel (fan bracket) or
// sequential (arrow chain).
function K2PlanStage({ stage, index, onSelect, selectedId }) {
  const par = stage.mode ? stage.mode === "parallel" : (stage.tasks || []).filter(t => !(t.deps || []).length).length > 1;
  const st = K2_NODE[k2StageState(stage)] || K2_NODE.pending;
  const tasks = stage.tasks || [];
  return (
    <div className="flex flex-col" style={{ gap: "var(--space-2)", minWidth: 150, flex: "1 1 0" }}>
      <div className="flex items-center gap-2" style={{ paddingBottom: "var(--space-2)", borderBottom: "var(--hairline)" }}>
        <span className="mono text-xs text-ink-faint">{String(index + 1).padStart(2, "0")}</span>
        <span className="text-sm font-medium text-ink flex-1" style={{ minWidth: 0 }}>{stage.title || stage.name}</span>
        <span className="rounded-full" style={{ width: 6, height: 6, background: st.tone, flexShrink: 0 }} />
      </div>
      <div className="flex items-center gap-1" style={{ marginBottom: 2 }}>
        <K2Icon name={par ? "transfer-horizontal" : "arrow-right"} size={13} color="var(--ink-faint)" />
        <span className="mono text-xs text-ink-faint">{par ? "parallel · all at once" : "sequential · in order"}</span>
      </div>
      {par ? (
        <div className="flex" style={{ gap: "var(--space-2)" }}>
          <span style={{ width: 1, background: "var(--paper-edge)", flexShrink: 0, borderRadius: 1 }} />
          <div className="flex flex-col" style={{ gap: "var(--space-2)", flex: 1, minWidth: 0 }}>
            {tasks.map(t => <K2PlanNode key={t.id} task={t} onSelect={onSelect} selected={selectedId === t.id} />)}
          </div>
        </div>
      ) : (
        <div className="flex flex-col" style={{ gap: 0 }}>
          {tasks.map((t, i) => (
            <React.Fragment key={t.id}>
              {i > 0 && (
                <span className="flex items-center" style={{ height: 16, paddingLeft: "var(--space-4)" }}>
                  <K2Icon name="alt-arrow-down" size={13} color="var(--ink-faint)" />
                </span>
              )}
              <K2PlanNode task={t} onSelect={onSelect} selected={selectedId === t.id} />
            </React.Fragment>
          ))}
        </div>
      )}
    </div>
  );
}

// The whole plan. Stages flow left→right on desktop (wrapping to a second row
// rather than scrolling out of sight), top→bottom on mobile.
function K2PlanGraph({ plan = [], mobile, onSelect, selectedId, legend = true }) {
  const stages = k2Phases(plan);
  const arrow = (i) => mobile
    ? <div key={"a" + i} className="flex justify-center" style={{ padding: "var(--space-1) 0" }}>
        <K2Icon name="alt-arrow-down" size={16} color="var(--ink-faint)" /></div>
    : <div key={"a" + i} className="flex items-center" style={{ paddingTop: 30, flexShrink: 0 }}>
        <K2Icon name="alt-arrow-right" size={16} color="var(--ink-faint)" /></div>;
  return (
    <div className="flex flex-col" style={{ gap: "var(--space-4)" }}>
      <div className={mobile ? "flex flex-col" : "flex flex-wrap"} style={{
        gap: mobile ? 0 : "var(--space-3)", alignItems: "stretch", rowGap: mobile ? 0 : "var(--space-5)" }}>
        {stages.map((s, i) => [
          i > 0 ? arrow(i) : null,
          <K2PlanStage key={s.id} stage={s} index={i} onSelect={onSelect} selectedId={selectedId} />,
        ])}
      </div>
      {legend && (
        <div className="flex flex-wrap items-center gap-4" style={{ paddingTop: "var(--space-3)", borderTop: "var(--hairline)" }}>
          {Object.keys(K2_NODE).map(k => (
            <span key={k} className="flex items-center gap-1">
              <K2Icon name={K2_NODE[k].icon} size={13} color={K2_NODE[k].tone} />
              <span className="mono text-xs text-ink-mute">{K2_NODE[k].label}</span>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

// The plan outline — phases with their tasks, seq-numbered the way the segment
// projection orders them. Per task: title · state · agent · model · spec_ref.
function K2PlanOutline({ plan = [], onSelect, selectedId, mobile }) {
  const phases = k2Phases(plan);
  return (
    <div className="flex flex-col">
      {phases.map((p, pi) => {
        const st = K2_NODE[k2StageState(p)] || K2_NODE.pending;
        return (
          <div key={p.id} className={pi ? "border-t" : ""}>
            <div className="flex items-center gap-3" style={{ padding: "var(--space-3) var(--space-4)", background: "var(--paper-mute)" }}>
              <span className="text-sm font-medium text-ink flex-1" style={{ minWidth: 0 }}>{p.title}</span>
              <span className="mono text-xs" style={{ color: st.tone }}>{st.label}</span>
              <span className="mono text-xs text-ink-faint">{p.tasks.filter(t => t.state === "done" || t.state === "skipped").length}/{p.tasks.length}</span>
            </div>
            {p.tasks.map(t => {
              const n = K2_NODE[t.state] || K2_NODE.pending;
              const on = selectedId === t.id;
              const labels = [t.agent, t.model].filter(Boolean).join(" · ");
              return (
                <button key={t.id} onClick={() => onSelect && onSelect(t)}
                  className={"flex w-full text-left border-t " + (mobile ? "flex-col gap-2" : "items-center gap-3")}
                  style={{ padding: "var(--space-3) var(--space-4)", paddingLeft: mobile ? "var(--space-4)" : "var(--space-6)",
                    background: on ? "var(--paper-mute)" : "transparent" }}>
                  <span className={mobile ? "flex items-center gap-3 w-full" : "flex items-center gap-3 flex-1"} style={{ minWidth: 0 }}>
                    <K2Icon name={n.icon} size={16} color={n.tone} />
                    <span style={{ minWidth: 0, flex: 1 }}>
                      <span className="flex items-center gap-2" style={{ flexWrap: "wrap" }}>
                        <span className={"text-sm " + (t.state === "pending" || t.state === "skipped" ? "text-ink-mute" : "text-ink")}>{t.title}</span>
                        {t.is_gate && <K2Chip mono tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{t.gate_severity === "advisory" ? "gate · advisory" : "gate · blocking"}</K2Chip>}
                      </span>
                      {(labels || t.spec_ref || t.summary) && (
                        <span className="mono text-xs text-ink-faint" style={{ display: "block", marginTop: 3, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: mobile ? "normal" : "nowrap" }}>
                          {[labels, t.spec_ref, t.summary].filter(Boolean).join(" · ")}
                        </span>
                      )}
                    </span>
                  </span>
                  <span className={mobile ? "flex items-center gap-3" : "flex items-center gap-3"} style={{ flexShrink: 0 }}>
                    {t.deps.length > 0 && <span className="mono text-xs text-ink-faint">waits on {t.deps.join(", ")}</span>}
                    <span className="mono text-xs" style={{ color: n.tone, width: mobile ? undefined : 88, textAlign: "right" }}>{n.label}</span>
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
function k2PlanProgress(plan = []) {
  const phases = k2Phases(plan);
  const tasks = phases.flatMap(s => s.tasks);
  const total = tasks.length || 1;
  const done = tasks.filter(t => t.state === "done" || t.state === "skipped").length;
  const running = tasks.filter(t => t.state === "active").length;
  const liveIdx = phases.findIndex(s => k2StageState(s) !== "done");
  const stage = liveIdx === -1 ? phases.length : liveIdx + 1;
  return { done, total, running, stage, stages: phases.length,
    pct: Math.round(((done + running * 0.5) / total) * 100),
    stageName: (phases[liveIdx] || phases[phases.length - 1] || {}).title || "" };
}

// What this run wants from you right now — drives stripe, label and CTA.
function k2RunFlag(run) {
  const tasks = k2Tasks(run.plan || []);
  const states = tasks.map(t => t.state);
  const gated = tasks.some(t => t.is_gate && t.state !== "done" && t.state !== "skipped");
  if (gated || states.includes("needs_review") || run.gate) return { key: "gate", label: "Needs approval", tone: "var(--accent)", cta: "Approve", act: true };
  if (states.includes("failed")) return { key: "failed", label: "Task failed", tone: "var(--danger)", cta: "Review", act: true };
  if (states.includes("blocked")) return { key: "blocked", label: "Blocked", tone: "var(--warning)", cta: "Unblock", act: true };
  if (run.state === "running") return { key: "running", label: "Running", tone: "var(--success)" };
  return { key: "waiting", label: "Waiting", tone: "var(--warning)" };
}

// A thin progress rail.
function K2PlanBar({ pct, tone = "var(--ink)", style }) {
  return (
    <span style={{ display: "block", height: 5, borderRadius: "var(--radius-sm)", background: "var(--paper-mute)", overflow: "hidden", ...style }}>
      <span style={{ display: "block", width: Math.max(2, pct) + "%", height: "100%", background: tone, borderRadius: "var(--radius-sm)" }} />
    </span>
  );
}

// The run's activity feed — what sensei actually did, newest first.
function K2RunActivity({ feed = [] }) {
  return (
    <div className="zs-card-flush" style={{ overflow: "hidden" }}>
      {feed.map((e, i) => (
        <div key={i} className="flex items-center gap-3" style={{ padding: "var(--space-3) var(--space-4)", borderBottom: i < feed.length - 1 ? "var(--hairline)" : "none" }}>
          <K2Icon name={e.icon} size={16} color={e.tone} />
          <span className="text-sm text-ink-soft flex-1" style={{ minWidth: 0 }}>{e.text}</span>
          <span className="mono text-xs text-ink-faint">{e.at}</span>
        </div>
      ))}
    </div>
  );
}

// Compact stage strip for a run row — one segment per stage, tinted by roll-up
// state, with a task-count caption. Reads as "where in the plan is this run".
function K2PlanPips({ plan = [], showCaption = true }) {
  plan = k2Phases(plan);
  if (!plan.length) return null;
  const total = plan.reduce((a, s) => a + (s.tasks || []).length, 0);
  const done = plan.reduce((a, s) => a + (s.tasks || []).filter(t => t.state === "done").length, 0);
  return (
    <span className="flex items-center gap-2" style={{ minWidth: 0 }}>
      <span className="flex items-center" style={{ gap: 3 }}>
        {plan.map(s => {
          const n = K2_NODE[k2StageState(s)] || K2_NODE.pending;
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
  K2_CLASS, K2_PHASE, K2_ROLE, K2_KIND, K2_NODE, K2_STATE_ALIAS, k2StageState, k2Phases, k2Tasks,
  K2PlanGraph, K2PlanStage, K2PlanNode, K2PlanOutline, K2PlanPips, K2PlanBar, K2RunActivity, k2PlanProgress, k2RunFlag,
  K2KanjiToken, K2Icon, K2Chip, K2ClassChip, K2RoleTag, K2PhasePill, K2SectionHead, K2Banner, K2StatBadge, K2Spark, K2Enso, K2ConfidenceBar, K2EmptyState, K2Btn, K2MyDojoRow, K2ProjectRow,
  K2OrgSwitcher, K2TopBar, K2ContextHeader, K2NavPane, K2AppShell, K2TabBar, K2MobileShell, K2ListSection,
  K2LadderRung, K2RuleRow, K2ConflictCard, K2StanceDial,
  K2RunCard, K2GateCard, K2NeedsYouBand, K2NeedsRow, K2DecisionCard, K2ChatThread,
  K2_STATUS, k2InboxRow, k2InboxRows, K2InboxRow, K2SubTabs,
});
