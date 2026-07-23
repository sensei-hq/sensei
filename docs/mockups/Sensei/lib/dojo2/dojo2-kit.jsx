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

// A dōjō membership row — org identity + your role.
function K2MyDojoRow({ dojo, onOpen }) {
  const kind = K2_KIND[dojo.kind] || K2_KIND.employer;
  return (
    <button onClick={() => onOpen && onOpen(dojo)} className="flex items-center gap-4 w-full text-left border-b"
      style={{ padding: "var(--space-4)", background: "transparent" }}>
      <span className="kanji" style={{ fontSize: 26, lineHeight: 1, color: kind.tone, width: 34, textAlign: "center", flexShrink: 0 }}>{dojo.kanji}</span>
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="flex items-center gap-2">
          <span className="text-base font-medium text-ink">{dojo.name}</span>
          <K2Chip mono tone="var(--ink-mute)">{dojo.kind}</K2Chip>
        </div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{dojo.route} · {dojo.members} members · {dojo.projects} projects</div>
      </div>
      {dojo.needs > 0 && <K2Chip icon="bell" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{dojo.needs} need you</K2Chip>}
      <K2RoleTag role={dojo.role} />
      <span className="text-ink-faint" style={{ fontSize: 18 }}>→</span>
    </button>
  );
}

// A project row — name · classification · phase · signal. The list workhorse.
function K2ProjectRow({ p, onOpen, showDojo = true, compact = false }) {
  if (compact) {
    return (
      <button onClick={() => onOpen && onOpen(p)} className="flex items-center gap-3 w-full text-left border-b" style={{ padding: "var(--space-3) var(--space-4)", background: "transparent" }}>
        <K2Icon name="folder" size={18} color="var(--accent)" />
        <div style={{ minWidth: 0, flex: 1 }}>
          <div className="text-sm font-medium text-ink" style={{ lineHeight: 1.2, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.name}</div>
          <div className="mono text-xs text-ink-faint" style={{ marginTop: 1, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{p.repo}</div>
        </div>
        {p.needs > 0 && <K2Chip icon="bell" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{p.needs}</K2Chip>}
        <K2PhasePill phase={p.phase} />
        <span className="mono text-xs text-ink-faint" style={{ width: 34, textAlign: "right", flexShrink: 0 }}>{p.lastRun}</span>
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

// A live-run card. `flat` drops the card chrome so it reads as a row inside a
// single flush card (matching the project list) instead of a stack of cards.
function K2RunCard({ run, onOpen, flat, stacked }) {
  const live = run.state === "running";
  const statusPill = (
    <span className={"inline-flex items-center gap-1 text-xs rounded-full " + (live ? "text-success bg-success-soft" : "text-warning bg-warning-soft")} style={{ border: "1px solid " + (live ? "var(--success-edge)" : "oklch(0.72 0.12 75 / 0.30)"), padding: "3px 10px" }}>
      <span className="rounded-full" style={{ width: 6, height: 6, background: live ? "var(--success)" : "var(--warning)" }} />{live ? "running" : "waiting"}
    </span>
  );
  if (stacked) {
    return (
      <button onClick={() => onOpen && onOpen(run)} className="flex flex-col w-full text-left border-b" style={{ padding: "var(--space-3) var(--space-4)", gap: "var(--space-2)", background: "transparent" }}>
        <div className="flex items-center gap-2">
          <K2Icon name="eye" size={20} color={live ? "var(--success)" : "var(--warning)"} />
          <span className="text-sm font-medium text-ink flex-1" style={{ lineHeight: 1.3 }}>{run.task}</span>
        </div>
        <div className="mono text-xs text-ink-faint">{run.project} · {run.id} · {run.assistant}</div>
        <div className="flex items-center gap-2">
          {statusPill}
          {run.gate && <K2Chip icon="command" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">gate</K2Chip>}
          <span className="flex-1" />
          <span className="mono text-xs text-ink-mute">{run.elapsed} · {run.edits} edits</span>
        </div>
      </button>
    );
  }
  const cls = flat ? "flex items-center gap-4 w-full text-left border-b" : "zs-card-flush flex items-center gap-4 w-full text-left";
  return (
    <button onClick={() => onOpen && onOpen(run)} className={cls} style={{ padding: flat ? "var(--space-3) var(--space-4)" : "var(--space-4)", background: "transparent" }}>
      <K2Icon name="eye" size={22} color={live ? "var(--success)" : "var(--warning)"} />
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-ink">{run.task}</span>
          {run.gate && <K2Chip icon="command" tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">gate waiting</K2Chip>}
        </div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{run.project} · {run.id} · {run.assistant}</div>
      </div>
      <div className="flex flex-col items-end" style={{ gap: 3 }}>
        {statusPill}
        <span className="mono text-xs text-ink-mute">{run.elapsed} · {run.edits} edits</span>
      </div>
    </button>
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

Object.assign(window, {
  K2_CLASS, K2_PHASE, K2_ROLE, K2_KIND,
  K2KanjiToken, K2Icon, K2Chip, K2ClassChip, K2RoleTag, K2PhasePill, K2SectionHead, K2Banner, K2StatBadge, K2Spark, K2Enso, K2ConfidenceBar, K2EmptyState, K2Btn, K2MyDojoRow, K2ProjectRow,
  K2OrgSwitcher, K2TopBar, K2ContextHeader, K2NavPane, K2AppShell, K2TabBar, K2MobileShell, K2ListSection,
  K2LadderRung, K2RuleRow, K2ConflictCard, K2StanceDial,
  K2RunCard, K2GateCard, K2NeedsYouBand, K2NeedsRow, K2DecisionCard, K2ChatThread,
});
