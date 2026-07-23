// Dōjō (web app) — screens + wired flow for the work-first redesign.
// Every screen is composed ONLY from the shared kit (window.K2*) — no
// per-screen hand-rolling. Two small screen-level compositions (PackRow,
// MemberRow) are built here from kit atoms; flagged in the summary.
//
// Exports (window): DojoApp2 (wired desktop) · DojoApp2Mobile (wired phone),
// plus the individual screen bodies for the static per-screen artboards.

const {
  K2AppShell, K2MobileShell, K2ListSection, K2SectionHead, K2Banner, K2StatBadge,
  K2Btn, K2Chip, K2Icon, K2ClassChip, K2RoleTag, K2PhasePill, K2KanjiToken, K2EmptyState,
  K2MyDojoRow, K2ProjectRow, K2NeedsYouBand, K2RunCard, K2GateCard, K2DecisionCard, K2ChatThread,
  K2LadderRung, K2RuleRow, K2ConflictCard, K2StanceDial,
} = window;
const D2 = window.DOJO2;

/* ── nav + tab definitions ─────────────────────────────── */
const NAV_YOU = [
  { group: "Work", items: [
    { id: "work", icon: "widget-4", label: "Your work" },
    { id: "runs", icon: "eye", label: "Live runs", badge: 3 },
    { id: "projects", icon: "folder", label: "Projects" },
  ]},
  { group: "Govern", items: [
    { id: "rules", icon: "scale", label: "Constitution" },
    { id: "packs", icon: "box", label: "Rule packs" },
  ]},
  { group: "Relay", items: [
    { id: "approve", icon: "check-circle", label: "Approve", badge: 2 },
    { id: "decide", icon: "checklist-minimalistic", label: "Decide", badge: 2 },
    { id: "chat", icon: "chat-round-line", label: "Chat" },
  ]},
  { group: "Dōjōs", items: [
    { id: "dojos", icon: "users-group-two-rounded", label: "My dōjōs" },
    { id: "contributions", icon: "upload-square", label: "Contributions", badge: 1 },
  ]},
];
const NAV_ORG_BASE = [
  { group: "Overview", items: [
    { id: "home", icon: "buildings-2", label: "Home" },
    { id: "ladder", icon: "scale", label: "Constitution" },
    { id: "projects", icon: "folder", label: "Projects", badge: 4 },
  ]},
  { group: "Govern", role: "maintainer", items: [
    { id: "triage", icon: "inbox", label: "Triage", badge: 5 },
    { id: "approvals", icon: "clipboard-check", label: "Approvals", badge: 2 },
    { id: "knowledge", icon: "book-2", label: "Knowledge" },
  ]},
  { group: "Clients", role: "lead", items: [
    { id: "engagements", icon: "case-round", label: "Engagements" },
    { id: "incidents", icon: "shield-warning", label: "Incidents", badge: 1 },
    { id: "clientaudit", icon: "document-text", label: "Client audit" },
  ]},
  { group: "Admin", role: "admin", items: [
    { id: "members", icon: "users-group-rounded", label: "Members & Roles" },
    { id: "scopes", icon: "shield-check", label: "Scopes & policies" },
    { id: "identity", icon: "key", label: "Identity & SSO" },
    { id: "audit", icon: "clipboard-list", label: "Audit" },
    { id: "health", icon: "pulse", label: "Health / Monitor" },
    { id: "billing", icon: "card", label: "Plan & billing" },
  ]},
];
// Additive role rank — a group shows when the viewer's role reaches its floor.
const K2_ROLE_RANK = { developer: 0, maintainer: 1, lead: 2, admin: 3 };
function navForOrg(org) {
  const rank = K2_ROLE_RANK[org && org.role] != null ? K2_ROLE_RANK[org.role] : 0;
  return NAV_ORG_BASE.filter(g => !g.role || rank >= K2_ROLE_RANK[g.role]);
}
const TABS_YOU = [
  { id: "work", icon: "widget-4", label: "Work" },
  { id: "runs", icon: "eye", label: "Runs", badge: 3 },
  { id: "inbox", icon: "checklist-minimalistic", label: "Needs", badge: 4 },
  { id: "chat", icon: "chat-round-line", label: "Chat" },
];
const TABS_ORG = [
  { id: "home", icon: "buildings-2", label: "Home" },
  { id: "projects", icon: "folder", label: "Projects", badge: 4 },
  { id: "ladder", icon: "scale", label: "Rules" },
  { id: "members", icon: "users-group-rounded", label: "Members" },
];

const Body = ({ children, gap = 5 }) => (
  <div style={{ padding: "var(--space-6)", display: "flex", flexDirection: "column", gap: `var(--space-${gap})` }}>{children}</div>
);
const BodyM = ({ children, gap = 4 }) => (
  <div style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: `var(--space-${gap})` }}>{children}</div>
);

/* ── back header (drill-in) ────────────────────────────── */
function BackHead({ onBack, children }) {
  return (
    <button onClick={onBack} className="inline-flex items-center gap-1 text-sm text-ink-mute" style={{ background: "transparent", alignSelf: "flex-start" }}>
      <K2Icon name="alt-arrow-left" size={15} color="var(--ink-mute)" /> {children || "Back"}
    </button>
  );
}

/* ═══ PERSONAL SCREENS ═════════════════════════════════════ */

// urgency rank for ordering the needs-you band (gate → conflict → decision → review)
const K2_URGENCY = { gate: 0, conflict: 1, decision: 2, review: 3 };
const K2_PRIMARY = { gate: "Approve", conflict: "Settle", decision: "Decide", review: "Approve" };
function orderNeeds(items) {
  return items.slice().sort((a, b) => (K2_URGENCY[a.kind] - K2_URGENCY[b.kind]) || 0);
}

// (1) YOUR WORK — needs-you + live runs + active projects.
function ScrYourWork({ onOpenProject, onAct, resolved = {}, mobile }) {
  const W = mobile ? BodyM : Body;
  const runsWeek = D2.projects.reduce((a, p) => a + p.runsWeek, 0);
  const activeRuns = D2.runs.filter(r => r.state === "running").length;
  const Div = () => <span style={{ width: 1, height: 34, background: "var(--paper-edge)", flexShrink: 0 }} />;
  const ordered = orderNeeds(D2.needsYou);
  const openItems = ordered.filter(it => !resolved[it.id]);
  const next = openItems[0];
  return (
    <W>
      <K2SectionHead eyebrow="Monday · morning" title="Your work" />
      {next ? (
        <K2Banner kanji="即" tone="accent" title={"Start here — " + next.title}
          right={<K2Btn size="sm" icon={next.kind === "gate" || next.kind === "review" ? "check-circle" : next.kind === "conflict" ? "scale" : "checklist-minimalistic"}
            onClick={() => onAct && onAct(next, { id: next.kind === "conflict" ? "settle" : next.kind === "decision" ? "decide" : "approve" })}>{K2_PRIMARY[next.kind]}</K2Btn>}>
          {next.project} · {next.dojo} · {next.why}. {openItems.length - 1 > 0 ? (openItems.length - 1) + " more below." : "Then you're clear."}
        </K2Banner>
      ) : (
        <K2Banner kanji="静" tone="success" title="Nothing needs you.">Sessions are running within the rules you set.</K2Banner>
      )}
      {!mobile && (
        <div className="flex items-center zs-card-flush" style={{ padding: "var(--space-4) var(--space-6)", gap: "var(--space-7)" }}>
          <K2StatBadge n={runsWeek} label="runs this week" sub="↑ from 9" tone="var(--accent)" />
          <Div />
          <K2StatBadge n={openItems.length} label="need you" sub="gates · conflicts · reviews" />
          <Div />
          <K2StatBadge n="0" label="incidents" sub="30 days clean" tone="var(--success)" />
        </div>
      )}
      <K2NeedsYouBand items={ordered} onAct={onAct} resolved={resolved} mobile={mobile} />
      <K2ListSection icon="eye" iconColor="var(--success)" title="Live runs" count={activeRuns + " running"} countTone="var(--success)"
        right={<a href="#" className="mono text-xs text-ink-mute" style={{ textDecoration: "none" }}>session log →</a>}>
        {D2.runs.map(r => <K2RunCard key={r.id} run={r} flat stacked={mobile} />)}
      </K2ListSection>
      <K2ListSection icon="folder" title="Active projects" count={D2.projects.length}
        right={<K2Btn size="sm" variant="ghost" icon="tuning-2">Filter</K2Btn>}>
        {D2.projects.map(p => <K2ProjectRow key={p.id} p={p} onOpen={onOpenProject} showDojo={!mobile} compact={mobile} />)}
      </K2ListSection>
    </W>
  );
}

// PROJECTS — the full list (personal or org).
function ScrProjects({ projects, showDojo = true, onOpenProject, eyebrow, title, mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow={eyebrow} title={title} count={projects.length}
        right={<K2Btn size="sm" variant="ghost" icon="tuning-2">Filter</K2Btn>} />
      <div className="zs-card-flush" style={{ overflow: "hidden" }}>
        {projects.map(p => <K2ProjectRow key={p.id} p={p} onOpen={onOpenProject} showDojo={showDojo && !mobile} compact={mobile} />)}
      </div>
    </W>
  );
}

// (2) CONSTITUTION — stance dials + your effective personal constitution.
function ScrConstitution({ onGoPacks, mobile }) {
  const W = mobile ? BodyM : Body;
  const personalRungs = D2.ladder.filter(r => r.id === "personal" || r.id === "stack");
  return (
    <W>
      <K2SectionHead eyebrow="You · standing rules" title="Your constitution"
        right={!mobile && <K2Btn size="sm" variant="ghost" icon="box" onClick={onGoPacks}>Rule packs →</K2Btn>} />
      <K2Banner kanji="静" tone="neutral" title="Everything sensei keeps is derived and stays on your machine.">
        Your stance sets how far a session runs and what surfaces to a dōjō. A classification change alters which rules apply — never what leaves.
      </K2Banner>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)", gap: "var(--space-4)" }}>
        {D2.stance.map(s => <K2StanceDial key={s.id} dial={s} />)}
      </div>
      <div>
        <div className="flex items-center gap-2" style={{ marginBottom: "var(--space-3)" }}>
          <K2Icon name="scale" size={17} color="var(--accent)" />
          <span className="zs-eyebrow font-semibold text-ink">Your effective constitution</span>
          <span className="zs-meta">— personal + stack · every project you touch</span>
        </div>
        <div className="flex flex-col gap-3">
          {personalRungs.map(r => <K2LadderRung key={r.id} rung={r} active={null} onSelect={() => {}} />)}
        </div>
      </div>
    </W>
  );
}

// (2b) RULE PACKS — adoptable rule bundles.
function PackRow({ pack }) {
  return (
    <div className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
      <K2Icon name="box" size={20} color="var(--accent)" style={{ marginTop: 0 }} />
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-ink">{pack.name}</span>
          <K2Chip mono tone="var(--ink-mute)">{pack.count} rules</K2Chip>
        </div>
        <div className="zs-meta" style={{ marginTop: 2 }}>by {pack.by} · {pack.note}</div>
      </div>
      {pack.adopted
        ? <K2Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</K2Chip>
        : <K2Btn size="sm" variant="ghost" icon="add-circle">Adopt</K2Btn>}
    </div>
  );
}
function ScrRulePacks({ mobile }) {
  const W = mobile ? BodyM : Body;
  const adopted = D2.rulePacks.filter(p => p.adopted), avail = D2.rulePacks.filter(p => !p.adopted);
  return (
    <W>
      <K2SectionHead eyebrow="Adopt · not a library" title="Rule packs"
        right={<K2Btn size="sm" variant="ghost" icon="tuning-2">Browse all</K2Btn>} />
      <K2Banner kanji="束" tone="neutral" title="Packs are bundles of rules you adopt into your constitution.">
        Adopting a pack adds its rules at the scope you choose. You can drop any single rule later — a pack is a starting point, not a lock.
      </K2Banner>
      <K2ListSection icon="check-circle" iconColor="var(--success)" title="Adopted" count={adopted.length} countTone="var(--success)">
        {adopted.map(p => <PackRow key={p.id} pack={p} />)}
      </K2ListSection>
      <K2ListSection icon="box" title="Available" count={avail.length}>
        {avail.map(p => <PackRow key={p.id} pack={p} />)}
      </K2ListSection>
    </W>
  );
}

// (3/7) PROJECT CONSTITUTION PREVIEW — the resolved ladder for one project.
function previewRungs(p) {
  const byId = Object.fromEntries(D2.ladder.map(r => [r.id, r]));
  let ids;
  if (p.classification === "personal") ids = ["personal", "project", "stack"];
  else if (p.classification === "client") ids = ["company", "client", "personal", "project", "stack"];
  else ids = ["company", "personal", "project", "stack"];
  return ids.map(id => {
    const r = byId[id];
    if (id === "project") return { ...r, name: p.name };
    return r;
  });
}
function ScrProjectPreview({ project, onBack, mobile }) {
  const W = mobile ? BodyM : Body;
  const p = project || D2.projects[0];
  const rungs = previewRungs(p);
  const [active, setActive] = React.useState("project");
  const [view, setView] = React.useState("layer"); // layer | consolidated
  const locks = rungs.reduce((a, r) => a + (r.rules || []).filter(x => x.hard).length, 0);
  const showConflicts = p.classification !== "personal";
  const discarded = showConflicts ? D2.conflicts.map(c => c.loser.text) : [];
  const effective = rungs.flatMap(r => (r.rules || []).map(x => ({ ...x, level: r.scope })))
    .filter(x => !discarded.includes(x.text));
  const Toggle = () => (
    <div className="flex" style={{ gap: 2, background: "var(--paper-mute)", padding: 2, borderRadius: "var(--radius)" }}>
      {[["layer", "By layer"], ["consolidated", "Consolidated"]].map(([id, lab]) => (
        <button key={id} onClick={() => setView(id)} className={"text-xs rounded " + (view === id ? "bg-paper text-ink border-1px" : "text-ink-mute")}
          style={{ padding: "4px 10px", border: view === id ? undefined : "1px solid transparent" }}>{lab}</button>
      ))}
    </div>
  );
  return (
    <W>
      {onBack && <BackHead onBack={onBack}>Back to projects</BackHead>}
      <K2SectionHead kanji="件" eyebrow={"Before you start · " + p.repo} title={p.name}
        right={<div className="flex items-center gap-2"><K2ClassChip kind={p.classification} /><K2PhasePill phase={p.phase} /></div>} />
      <K2Banner kanji="観" tone={p.classification === "client" ? "accent" : "neutral"}
        title={p.classification === "client" ? "Client engagement — sources are dereferenced." : "These rules resolve here, most-specific wins."}>
        {rungs.length} scopes compose this constitution · {effective.length} rules apply · {locks} locked (★){discarded.length ? " · " + discarded.length + " discarded by the ladder" : ""}.
      </K2Banner>
      <div>
        <div className="flex items-center gap-2" style={{ marginBottom: "var(--space-3)" }}>
          <K2Icon name="layers-minimalistic" size={17} color="var(--accent)" />
          <span className="zs-eyebrow font-semibold text-ink">{view === "layer" ? "The ladder — broad → specific" : "Consolidated constitution"}</span>
          <span className="zs-meta">{view === "layer" ? "tap a rung to focus" : effective.length + " effective rules"}</span>
          <span className="flex-1" />
          <Toggle />
        </div>
        {view === "layer" ? (
          <div className="flex flex-col gap-3">
            {rungs.map(r => <K2LadderRung key={r.id} rung={r} active={active} onSelect={setActive} />)}
          </div>
        ) : (
          <div className="zs-card-flush" style={{ overflow: "hidden" }}>
            {effective.map((r, i) => <K2RuleRow key={i} rule={r} showLevel onJump={(rule) => {
              const hit = rungs.find(x => x.scope === rule.level);
              if (hit) { setActive(hit.id); setView("layer"); }
            }} />)}
          </div>
        )}
      </div>
      {showConflicts && (
        <div>
          <div className="flex items-center gap-2" style={{ marginBottom: "var(--space-3)" }}>
            <K2Icon name="danger-triangle" size={17} color="var(--warning)" />
            <span className="zs-eyebrow font-semibold text-ink">Discarded by the ladder</span>
            <span className="zs-meta">what the resolution eliminated, and why</span>
          </div>
          <div className="flex flex-col gap-3">
            {D2.conflicts.map(c => <K2ConflictCard key={c.id} conflict={c} />)}
          </div>
        </div>
      )}
    </W>
  );
}

/* ═══ RELAY SCREENS ════════════════════════════════════════ */

function ScrRelayWatch({ onOpenProject, mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow="Relay · watch" title="Live runs" count={D2.runs.length}
        right={<a href="#" className="mono text-xs text-ink-mute" style={{ textDecoration: "none" }}>session log →</a>} />
      <div className="zs-card-flush" style={{ overflow: "hidden" }}>
        {D2.runs.map(r => <K2RunCard key={r.id} run={r} flat stacked={mobile} />)}
      </div>
    </W>
  );
}
function ScrRelayApprove({ mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow="Relay · approve" title="Commands waiting on you" count={D2.gates.length} />
      {D2.gates.map(g => <K2GateCard key={g.id} gate={g} />)}
    </W>
  );
}
function ScrRelayDecide({ mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow="Relay · decide" title="Rules to sign off" count={D2.decisions.length} />
      {D2.decisions.map(d => <K2DecisionCard key={d.id} decision={d} />)}
      <K2Banner kanji="静" tone="neutral" title="That's everything.">When there is nothing to decide, sensei stays quiet.</K2Banner>
    </W>
  );
}
function ScrRelayChat({ mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow={"Relay · chat · " + D2.chat.session} title={D2.chat.project} />
      <div className="zs-card-flush" style={{ padding: "var(--space-5)" }}>
        <K2ChatThread thread={D2.chat.thread} me={D2.me} />
      </div>
      <div className="zs-input" style={{ height: 42 }}>
        <K2Icon name="chat-round-line" size={16} color="var(--ink-mute)" />
        <span className="text-ink-faint text-sm">reply to sensei…</span>
      </div>
    </W>
  );
}

/* ═══ ORG SCREENS ══════════════════════════════════════════ */

// (5) ORG HOME — projects in the dōjō's jurisdiction + org needs.
function ScrOrgHome({ org, onOpenProject, onAct, resolved, mobile }) {
  const W = mobile ? BodyM : Body;
  const projs = (D2.orgProjects[org.slug] || D2.orgProjects.acme).map(p => ({
    ...p, repo: org.slug + "/" + p.name, note: p.team + " · " + p.maintainers + " maintainers", lastRun: p.runsWeek + "/wk",
  }));
  const orgNeeds = D2.needsYou.slice(0, 2);
  const Div = () => <span style={{ width: 1, height: 34, background: "var(--paper-edge)", flexShrink: 0 }} />;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · jurisdiction"} title={projs.length + " projects under this dōjō"} />
      {!mobile && (
        <div className="flex items-center zs-card-flush" style={{ padding: "var(--space-4) var(--space-6)", gap: "var(--space-7)" }}>
          <K2StatBadge n={org.members} label="members" />
          <Div />
          <K2StatBadge n={org.needs} label="need a maintainer" sub="across this jurisdiction" tone="var(--accent)" />
          <Div />
          <K2StatBadge n={org.projects} label="projects" sub="in flight" />
        </div>
      )}
      <K2NeedsYouBand items={orgNeeds} onAct={onAct} resolved={resolved} title="Needs a maintainer" mobile={mobile} />
      <K2ListSection icon="folder" title="Projects" count={projs.length}
        right={<K2Btn size="sm" variant="ghost" icon="tuning-2">By team</K2Btn>}>
        {projs.map(p => <K2ProjectRow key={p.id} p={p} onOpen={onOpenProject} showDojo={false} compact={mobile} />)}
      </K2ListSection>
    </W>
  );
}

// Rule add/edit editor — overlay card composed from kit atoms.
const RULE_FAMILIES = [
  { k: "守", label: "guard" }, { k: "紋", label: "pattern" }, { k: "理", label: "principle" },
  { k: "検", label: "review" }, { k: "技", label: "stack" }, { k: "盾", label: "shield" },
];
function RuleEditor({ scope, rule, onClose }) {
  const [text, setText] = React.useState(rule ? rule.text : "");
  const [fam, setFam] = React.useState(rule ? rule.kanji : "守");
  const [hard, setHard] = React.useState(rule ? !!rule.hard : false);
  return (
    <div style={{ position: "absolute", inset: 0, zIndex: 80, display: "flex", alignItems: "center", justifyContent: "center", padding: "var(--space-6)", background: "oklch(0.22 0.012 50 / 0.28)" }} onClick={onClose}>
      <div className="bg-paper rounded-lg shadow-lg" style={{ width: 520, maxWidth: "100%", overflow: "hidden" }} onClick={e => e.stopPropagation()}>
        <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-4) var(--space-5)" }}>
          <K2Icon name={rule ? "pen-2" : "add-circle"} size={18} color="var(--accent)" />
          <span className="display text-lg tracking-tight text-ink">{rule ? "Edit rule" : "New rule"}</span>
          <span className="flex-1" />
          <span className="zs-meta">{scope.scope} · {scope.name}</span>
        </div>
        <div style={{ padding: "var(--space-5)", display: "flex", flexDirection: "column", gap: "var(--space-4)" }}>
          <div>
            <div className="zs-eyebrow font-semibold text-ink-mute mb-2">Rule</div>
            <textarea value={text} onChange={e => setText(e.target.value)} rows={3} placeholder="State the rule as an instruction sensei can follow…"
              className="text-sm text-ink" style={{ width: "100%", resize: "none", background: "var(--paper-2)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-3)", fontFamily: "var(--font-ui)", outline: "none", lineHeight: 1.5 }} />
          </div>
          <div>
            <div className="zs-eyebrow font-semibold text-ink-mute mb-2">Family</div>
            <div className="flex flex-wrap gap-2">
              {RULE_FAMILIES.map(f => {
                const on = fam === f.k;
                return (
                  <button key={f.k} onClick={() => setFam(f.k)} className={"inline-flex items-center gap-2 rounded-full text-sm " + (on ? "bg-accent-soft" : "bg-paper-2 border-1px")}
                    style={{ padding: "var(--space-2) var(--space-3)", border: on ? "1px solid var(--accent)" : undefined, color: on ? "var(--accent)" : "var(--ink-soft)" }}>
                    <span className="kanji" style={{ fontSize: 14 }}>{f.k}</span>{f.label}
                  </button>
                );
              })}
            </div>
          </div>
          <button onClick={() => setHard(h => !h)} className="flex items-center gap-3 text-left rounded-lg" style={{ padding: "var(--space-3) var(--space-4)", background: hard ? "var(--accent-soft)" : "var(--paper-2)", border: "1px solid " + (hard ? "var(--accent-edge)" : "var(--paper-edge)") }}>
            <span style={{ fontSize: 16, color: hard ? "var(--accent)" : "var(--ink-faint)" }}>★</span>
            <div className="flex-1">
              <div className="text-sm font-medium text-ink">Non-negotiable</div>
              <div className="zs-meta">Locks the rule — no narrower scope can relax it.</div>
            </div>
            <span className="rounded-full" style={{ width: 34, height: 20, background: hard ? "var(--accent)" : "var(--paper-mute)", position: "relative", flexShrink: 0, transition: "background .15s" }}>
              <span className="rounded-full" style={{ position: "absolute", top: 2, left: hard ? 16 : 2, width: 16, height: 16, background: "var(--paper)", transition: "left .15s" }} />
            </span>
          </button>
        </div>
        <div className="flex gap-2 border-t" style={{ padding: "var(--space-4) var(--space-5)", background: "var(--paper-2)" }}>
          <span className="flex-1" />
          <K2Btn size="sm" variant="ghost" onClick={onClose}>Cancel</K2Btn>
          <K2Btn size="sm" icon="check-circle" onClick={onClose}>{rule ? "Save rule" : "Add rule"}</K2Btn>
        </div>
      </div>
    </div>
  );
}

// (6) CONSTITUTION — the dōjō authors its OWN rules, by section
// (company-wide · teams · stacks, packs per stack). Not the resolution ladder.
function ScrOrgLadder({ org, mobile }) {
  const sections = D2.orgConstitution[org.slug] || D2.orgConstitution.acme;
  const [active, setActive] = React.useState(sections[0].id);
  const sec = sections.find(s => s.id === active) || sections[0];
  const [inc, setInc] = React.useState({});
  const [editing, setEditing] = React.useState(null);
  const [showEx, setShowEx] = React.useState(false);
  const isIn = (i) => inc[active + i] !== false;
  const excluded = (sec.rules || []).filter((r, i) => !isIn(i)).length;
  const groups = ["Company", "Teams", "Stacks"];
  const W = mobile ? BodyM : Body;
  return (
    <div style={{ position: "relative" }}>
      <W>
        <K2SectionHead eyebrow={org.name + " · governance"} title="Constitution"
          right={<K2Btn size="sm" icon="add-circle" onClick={() => setEditing({})}>New rule</K2Btn>} />
        <K2Banner kanji="掟" tone="neutral" title="The dōjō authors its rules by scope — company-wide, per team, per stack.">
          Stacks also adopt rule packs. These are the dōjō's own rules; how they combine with your personal and a project's rules is resolved — and shown — when you open a project.
        </K2Banner>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "252px 1fr", gap: "var(--space-4)", alignItems: "start" }}>
          <div className="flex flex-col gap-4">
            {groups.map(g => {
              const items = sections.filter(s => s.group === g);
              if (!items.length) return null;
              return (
                <div key={g}>
                  <div className="zs-eyebrow font-semibold text-ink-mute px-2 mb-2">{g}</div>
                  <div className="flex flex-col gap-2">
                    {items.map(s => (
                      <button key={s.id} onClick={() => setActive(s.id)}
                        className={"flex items-center gap-3 w-full text-left rounded-lg " + (active === s.id ? "bg-accent-soft" : "bg-paper-2 border-1px")}
                        style={{ padding: "var(--space-3)", border: active === s.id ? "1px solid var(--accent)" : undefined }}>
                        <K2KanjiToken char={s.kanji} size="lg" tone={active === s.id ? "var(--accent)" : "var(--ink-soft)"} w={24} />
                        <div className="flex-1" style={{ minWidth: 0 }}>
                          <div className="text-sm font-medium text-ink" style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>{s.scope}</div>
                          <div className="zs-meta">{(s.rules || []).length} rules{s.packs && s.packs.length ? " · " + s.packs.length + " pack" : ""}</div>
                        </div>
                      </button>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
          <div className="flex flex-col gap-4">
            {sec.packs && (
              <div className="zs-card-flush" style={{ overflow: "hidden" }}>
                <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
                  <K2Icon name="box" size={16} color="var(--accent)" />
                  <span className="zs-eyebrow font-semibold text-ink">Rule packs for this stack</span>
                  <span className="flex-1" />
                  <K2Btn size="sm" variant="ghost" icon="add-circle">Adopt pack</K2Btn>
                </div>
                {sec.packs.length ? sec.packs.map((p, i) => (
                  <div key={i} className="flex items-center gap-3 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
                    <K2Icon name="box" size={17} color="var(--accent)" />
                    <span className="text-sm text-ink flex-1">{p}</span>
                    <K2Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</K2Chip>
                  </div>
                )) : <div className="zs-meta" style={{ padding: "var(--space-3) var(--space-4)" }}>No pack adopted — this stack runs on its own rules and the company baseline.</div>}
              </div>
            )}
            <div className="zs-card-flush" style={{ overflow: "hidden" }}>
              <div className="flex items-center gap-2 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
                <K2KanjiToken char={sec.kanji} size="base" tone="var(--accent)" />
                <span className="zs-eyebrow font-semibold text-ink">{sec.scope} · rules</span>
                <span className="flex-1" />
                {excluded > 0 && (
                  <button onClick={() => setShowEx(s => !s)} className="inline-flex items-center gap-1 mono text-xs text-ink-mute" style={{ background: "transparent" }}>
                    <K2Icon name={showEx ? "eye-closed" : "eye"} size={14} color="var(--ink-mute)" />{showEx ? "Hide" : "Show"} {excluded} excluded
                  </button>
                )}
                <K2Btn size="sm" variant="ghost" icon="add-circle" onClick={() => setEditing({})}>Add</K2Btn>
              </div>
              {(sec.rules || []).map((r, i) => ({ r, i })).filter(({ i }) => showEx || isIn(i)).map(({ r, i }) => (
                <K2RuleRow key={i} rule={r} showLevel={false} included={isIn(i)}
                  onToggle={() => setInc(s => ({ ...s, [active + i]: isIn(i) ? false : true }))}
                  onEdit={() => setEditing({ rule: r })} />
              ))}
              {!showEx && excluded > 0 && (
                <div className="zs-meta" style={{ padding: "var(--space-2) var(--space-4)", background: "var(--paper)" }}>{excluded} excluded rule{excluded > 1 ? "s" : ""} hidden · consolidated view</div>
              )}
            </div>
          </div>
        </div>
      </W>
      {editing && <RuleEditor scope={sec} rule={editing.rule} onClose={() => setEditing(null)} />}
    </div>
  );
}

// (8) ROLE SURFACES — members / policies / audit (admin).
function MemberRow({ m }) {
  return (
    <div className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
      <window.Avatar name={m.name} size={30} />
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-ink">{m.name}</span>
          {m.you && <K2Chip tone="var(--ink-mute)">you</K2Chip>}
        </div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>git: {m.git} · {m.scopes}</div>
      </div>
      <span className="zs-meta">{m.active}</span>
      <K2RoleTag role={m.role} />
      <K2Icon name="alt-arrow-right" size={16} color="var(--ink-faint)" />
    </div>
  );
}
function ScrRoleSurfaces({ org, tab = "members", mobile }) {
  const W = mobile ? BodyM : Body;
  const tabs = [
    { id: "members", label: "Members & Roles", icon: "users-group-rounded", eyebrow: "admin", title: "Members & Roles" },
    { id: "policies", label: "Policies", icon: "shield-check", eyebrow: "admin", title: "Role policies" },
    { id: "audit", label: "Audit", icon: "clipboard-list", eyebrow: "admin", title: "Audit log" },
  ];
  const [t, setT] = React.useState(tab);
  React.useEffect(() => { setT(tab); }, [tab]);
  const cur = tabs.find(x => x.id === t) || tabs[0];
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · " + cur.eyebrow} title={cur.title}
        right={<K2Btn size="sm" icon="add-circle">{t === "members" ? "Invite" : t === "policies" ? "New policy" : "Export"}</K2Btn>} />
      <div className="flex gap-2">
        {tabs.map(x => (
          <button key={x.id} onClick={() => setT(x.id)}
            className={"inline-flex items-center gap-2 rounded text-sm " + (t === x.id ? "bg-paper-soft border-1px text-ink" : "text-ink-mute")}
            style={{ padding: "var(--space-2) var(--space-3)", border: t === x.id ? undefined : "1px solid transparent" }}>
            <K2Icon name={x.icon} size={15} color={t === x.id ? "var(--accent)" : "var(--ink-mute)"} />{x.label}
          </button>
        ))}
      </div>
      {t === "members" && (
        <div className="zs-card-flush" style={{ overflow: "hidden" }}>
          {D2.members.map((m, i) => <MemberRow key={i} m={m} />)}
        </div>
      )}
      {t === "policies" && (
        <div className="flex flex-col gap-3">
          <K2Banner kanji="規" tone="neutral" title="Roles are additive and derived from git.">developer → maintainer → lead → admin. A role only ever adds capability.</K2Banner>
          {Object.values(D2.roles).map((r, i) => (
            <div key={i} className="zs-card-flush flex items-center gap-4" style={{ padding: "var(--space-4)" }}>
              <K2KanjiToken char={r.kanji} size="xl" tone="var(--accent)" w={30} />
              <div className="flex-1"><div className="text-sm font-medium text-ink">{r.label}</div><div className="zs-meta">{r.note}</div></div>
              <K2Btn size="sm" variant="ghost" icon="tuning-2">Edit policy</K2Btn>
            </div>
          ))}
        </div>
      )}
      {t === "audit" && (
        <div className="zs-card-flush" style={{ overflow: "hidden" }}>
          {D2.chat.thread.map((x, i) => (
            <div key={i} className="flex items-center gap-3 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
              <K2Icon name="clipboard-list" size={17} color="var(--ink-mute)" />
              <span className="text-sm text-ink flex-1">{x.who === "sensei" ? "sensei" : D2.me.name} · {x.text.slice(0, 60)}…</span>
              <span className="mono text-xs text-ink-faint">{x.when}</span>
            </div>
          ))}
        </div>
      )}
    </W>
  );
}

/* ═══ CONTRIBUTIONS · SCOPES · BILLING (ported) ════════════ */

// CONTRIBUTIONS (personal) — what you shared upstream + what's approved for you.
const K2_CSTAT = {
  approved: { tone: "var(--success)", soft: "var(--success-soft)", edge: "var(--success-edge)", label: "approved", icon: "check-circle" },
  pending:  { tone: "var(--accent)",  soft: "var(--accent-soft)",  edge: "var(--accent-edge)",  label: "in triage", icon: "hourglass" },
  declined: { tone: "var(--danger)",  soft: "var(--danger-soft)",  edge: "var(--danger-edge)",  label: "declined", icon: "close-circle" },
};
function ContribRow({ c }) {
  const s = K2_CSTAT[c.status] || K2_CSTAT.pending;
  return (
    <div className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
      <K2KanjiToken char={c.kanji} size="lg" tone="var(--accent)" w={22} />
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{c.title}</div>
        <div className="flex items-center gap-2" style={{ marginTop: 2, flexWrap: "wrap" }}>
          <K2Chip icon={c.client ? "shield-check" : undefined} tone={c.client ? "var(--accent)" : "var(--ink-mute)"} soft={c.client ? "var(--accent-soft)" : "var(--paper-mute)"} edge={c.client ? "var(--accent-edge)" : "transparent"}>{c.dest}</K2Chip>
          <span className="mono text-xs text-ink-faint">{c.scope} · {c.note}</span>
        </div>
      </div>
      <K2Chip icon={s.icon} tone={s.tone} soft={s.soft} edge={s.edge}>{s.label}</K2Chip>
      <span className="mono text-xs text-ink-faint" style={{ width: 28, textAlign: "right" }}>{c.when}</span>
    </div>
  );
}
function DownstreamRow({ it }) {
  return (
    <div className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
      <K2KanjiToken char={it.kanji} size="lg" tone="var(--accent)" w={22} />
      <div className="flex-1" style={{ minWidth: 0 }}>
        <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{it.title}</div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{it.from} · {it.scope} · {it.when} ago</div>
      </div>
      {it.adopted
        ? <K2Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</K2Chip>
        : <K2Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">new</K2Chip>}
      {!it.adopted && <K2Btn size="sm" variant="ghost" icon="pin">Pin</K2Btn>}
    </div>
  );
}
function ScrContributions({ mobile }) {
  const W = mobile ? BodyM : Body;
  const { mine, downstream, stat } = D2.contributions;
  return (
    <W>
      <K2SectionHead eyebrow="You · sharing" title="Contributions"
        right={!mobile && <div className="flex items-center gap-4">
          <K2StatBadge n={stat.approved} label="approved" tone="var(--success)" />
          <K2StatBadge n={stat.pending} label="in triage" tone="var(--accent)" />
          <K2StatBadge n={stat.helped} label="devs helped" sub="lifetime" />
        </div>} />
      <K2Banner kanji="共" tone="neutral" title="You propose; a maintainer decides — nothing publishes without their named approval.">
        You share from a session's ready-to-share lane; it lands in the bound dōjō's triage queue. Client work is anonymized before it leaves — the lesson travels, the client never does.
      </K2Banner>
      <K2ListSection icon="upload-square" title="What you've shared" count={mine.length}>
        {mine.map((c, i) => <ContribRow key={i} c={c} />)}
      </K2ListSection>
      <K2ListSection icon="download-square" iconColor="var(--success)" title="Approved for you" count={downstream.length}
        right={<span className="zs-meta">from your dōjōs · never silently merged</span>}>
        {downstream.map((it, i) => <DownstreamRow key={i} it={it} />)}
      </K2ListSection>
    </W>
  );
}

// SCOPES (org admin) — who owns/triages each scope's queue.
function ScrScopes({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const rows = D2.scopeOwners[org.slug] || D2.scopeOwners.acme;
  const groups = ["Company", "Teams", "Stacks"];
  const unowned = rows.filter(r => !r.owner).length;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · admin"} title="Scopes & policies"
        right={<K2Btn size="sm" icon="add-circle">New scope</K2Btn>} />
      <K2Banner kanji="規" tone={unowned ? "warning" : "neutral"} title="Every scope has a named owner who triages its queue.">
        {unowned ? unowned + " scope has no owner — its queue routes to a fallback maintainer so nothing stalls. Assign an owner to give it an SLA." : "Anything unowned routes to a fallback maintainer so nothing stalls."}
      </K2Banner>
      {groups.map(g => {
        const items = rows.filter(r => r.group === g); if (!items.length) return null;
        return (
          <K2ListSection key={g} icon={g === "Company" ? "buildings-2" : g === "Teams" ? "users-group-rounded" : "layers-minimalistic"} title={g} count={items.length}>
            {items.map((r, i) => (
              <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
                <div className="flex-1" style={{ minWidth: 0 }}>
                  <div className="text-sm font-medium text-ink">{r.scope}</div>
                  <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.queue} in queue · SLA {r.sla}</div>
                </div>
                {r.owner ? (
                  <div className="flex items-center gap-2">
                    <window.Avatar name={r.owner} size={24} />
                    <span className="text-sm text-ink">{r.owner}</span>
                    <K2RoleTag role={r.role} muted />
                  </div>
                ) : <K2Chip icon="danger-triangle" tone="var(--warning)" soft="var(--warning-soft)" edge="oklch(0.72 0.12 75 / 0.30)">unowned · fallback</K2Chip>}
                <K2Btn size="sm" variant="ghost" icon="tuning-2">{r.owner ? "Reassign" : "Assign"}</K2Btn>
              </div>
            ))}
          </K2ListSection>
        );
      })}
    </W>
  );
}

// BILLING (org admin) — plan & seats. Free where public/personal; paid where shared.
function ScrBilling({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const b = D2.billing;
  const monthly = b.seatsActive * b.perSeat;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · plan & billing"} title="Plan & billing"
        right={<K2Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{b.plan}</K2Chip>} />
      <K2Banner kanji="円" tone="neutral" title="Free where public or personal; paid where private and shared.">
        Seats bill per active contributor. The desktop app, the global collective, bring-your-own-key inference, and read-only membership are always free — you pay to coordinate a group, never for tokens.
      </K2Banner>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)", gap: "var(--space-4)" }}>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)", border: "1px solid var(--accent)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">Current plan</div>
          <div className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 300, letterSpacing: "-0.01em" }}>{b.plan}</div>
          <div className="zs-body-sm" style={{ marginTop: 2 }}>Renews {b.renews} · <span className="mono">${b.perSeat}</span>/contributor/mo</div>
        </div>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">Billable seats</div>
          <K2StatBadge n={b.seatsActive} label="active contributors" sub={b.seatsReadonly + " read-only free"} />
        </div>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">This month</div>
          <K2StatBadge n={"$" + monthly} label={b.seatsActive + " × $" + b.perSeat} sub="updated live" tone="var(--accent)" />
        </div>
      </div>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)", gap: "var(--space-4)" }}>
        {b.tiers.map(t => (
          <div key={t.id} className="zs-card-flush" style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-2)",
            background: t.dark ? "var(--ink)" : undefined, border: t.current ? "1px solid var(--accent)" : undefined }}>
            <div className="flex items-center gap-2">
              <K2KanjiToken char={t.kanji} size="lg" tone={t.dark ? "var(--accent)" : t.current ? "var(--accent)" : "var(--ink-mute)"} />
              <span className="text-sm font-semibold" style={{ color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.name}</span>
              {t.current && <K2Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)" style={{ marginLeft: "auto" }}>current</K2Chip>}
            </div>
            <div>
              <span className="display" style={{ fontSize: "var(--text-xl)", fontWeight: 300, color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.price}</span>
              <span className="mono text-xs" style={{ color: t.dark ? "var(--on-primary-mute)" : "var(--ink-faint)", marginLeft: 4 }}>{t.sub}</span>
            </div>
            <div className="flex flex-col gap-1">
              {t.lines.map((l, i) => (
                <div key={i} className="flex gap-2" style={{ fontSize: "var(--text-xs)", lineHeight: 1.45, color: t.dark ? "var(--on-primary-soft)" : "var(--ink-soft)" }}>
                  <span className="text-accent" style={{ flexShrink: 0 }}>·</span>{l}
                </div>
              ))}
            </div>
            {!t.current && <button className="zs-btn zs-btn-sm bg-paper border-1px" style={{ marginTop: "auto", justifyContent: "center" }}>{t.id === "free" ? "Downgrade" : "Contact sales"}</button>}
          </div>
        ))}
      </div>
      <K2ListSection icon="chat-round-line" title="Relay — free for individuals, paid where it's shared">
        {b.relayRows.map((r, i) => (
          <div key={i} className="flex items-center gap-3 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <span className="text-sm text-ink-soft flex-1">{r.label}</span>
            <K2Chip tone={r.free ? "var(--success)" : "var(--accent)"} soft={r.free ? "var(--success-soft)" : "var(--accent-soft)"} edge={r.free ? "var(--success-edge)" : "var(--accent-edge)"}>{r.free ? "free · individuals" : "paid · team"}</K2Chip>
          </div>
        ))}
      </K2ListSection>
      <K2ListSection icon="bill-list" title="Invoices" count={b.invoices.length}>
        {b.invoices.map((iv, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <span className="mono text-xs text-ink-soft flex-1">{iv.d}</span>
            <span className="mono text-sm text-ink">{iv.amt}</span>
            <K2Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">{iv.s}</K2Chip>
          </div>
        ))}
      </K2ListSection>
    </W>
  );
}

/* ═══ ORG ROLE CONSOLES (Govern · Clients · Admin) ════════ */

const K2_IMPACT = {
  high:   { tone: "var(--accent)", soft: "var(--accent-soft)", edge: "var(--accent-edge)" },
  safety: { tone: "var(--danger)", soft: "var(--danger-soft)", edge: "var(--danger-edge)" },
  normal: { tone: "var(--ink-mute)", soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  low:    { tone: "var(--ink-mute)", soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
};

// 1 · TRIAGE (maintainer) — ranked candidate queue + candidate detail.
function ScrTriage({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const groups = D2.consoles.triage;
  const all = groups.flatMap(g => g.items);
  const [sel, setSel] = React.useState(all[0].id);
  const cur = all.find(c => c.id === sel) || all[0];
  const d = D2.consoles.candidateDetail;
  const total = all.length;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · govern"} title="Triage" count={total}
        right={<K2Btn size="sm" variant="ghost" icon="tuning-2">My scopes</K2Btn>} />
      <K2Banner kanji="門" tone="neutral" title="Candidate learnings waiting on a maintainer, grouped by scope and ranked by confidence.">
        You decide what becomes shared knowledge. High-impact and safety candidates need a second approval before they publish.
      </K2Banner>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1.1fr 1fr", gap: "var(--space-4)", alignItems: "start" }}>
        <div className="flex flex-col gap-4">
          {groups.map(g => (
            <K2ListSection key={g.scope} icon="layers-minimalistic" title={g.scope} count={g.items.length}>
              {g.items.map(c => {
                const im = K2_IMPACT[c.impact] || K2_IMPACT.normal;
                const on = c.id === sel;
                return (
                  <button key={c.id} onClick={() => setSel(c.id)} className={"flex items-center gap-3 w-full text-left border-b " + (on ? "bg-accent-soft" : "")} style={{ padding: "var(--space-3) var(--space-4)", background: on ? undefined : "transparent" }}>
                    <K2KanjiToken char={c.kanji} size="base" tone="var(--accent)" w={20} />
                    <div className="flex-1" style={{ minWidth: 0 }}>
                      <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{c.title}</div>
                      <div className="flex items-center gap-2" style={{ marginTop: 3, flexWrap: "wrap" }}>
                        <span className="mono text-xs text-ink-faint">{c.origin}</span>
                        {c.conflicts > 0 && <K2Chip icon="danger-triangle" tone="var(--warning)" soft="var(--warning-soft)" edge="oklch(0.72 0.12 75 / 0.30)">{c.conflicts} conflict</K2Chip>}
                        {c.dups > 0 && <K2Chip tone="var(--ink-mute)">{c.dups} dup</K2Chip>}
                        {(c.impact === "high" || c.impact === "safety") && <K2Chip tone={im.tone} soft={im.soft} edge={im.edge}>{c.impact}</K2Chip>}
                      </div>
                    </div>
                    <K2ConfidenceBar v={c.conf} w={56} />
                  </button>
                );
              })}
            </K2ListSection>
          ))}
        </div>
        {!mobile && (
          <div className="zs-card-flush" style={{ overflow: "hidden", position: "sticky", top: 0 }}>
            <div className="flex items-center gap-3 border-b" style={{ padding: "var(--space-4)" }}>
              <K2KanjiToken char={cur.kanji} size="xl" tone="var(--accent)" />
              <div className="flex-1"><div className="zs-eyebrow font-semibold text-ink-mute">candidate</div><div className="text-sm font-medium text-ink">{cur.title}</div></div>
              <K2Enso progress={cur.conf} size={54} label={Math.round(cur.conf * 100) + ""} />
            </div>
            <div style={{ padding: "var(--space-4)", display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
              <div><div className="zs-eyebrow text-ink-mute mb-1">Learning</div><div className="text-sm text-ink">{d.learning}</div></div>
              <div><div className="zs-eyebrow text-ink-mute mb-1">Cause</div><div className="zs-body-sm">{d.cause}</div></div>
              <div><div className="zs-eyebrow text-ink-mute mb-1">Evidence</div>
                <div className="flex flex-col gap-1">{d.evidence.map((e, i) => <div key={i} className="mono text-xs text-ink-soft">· {e}</div>)}</div></div>
              {cur.conflicts > 0 && (
                <div className="rounded" style={{ background: "var(--warning-soft)", border: "1px solid oklch(0.72 0.12 75 / 0.30)", padding: "var(--space-3)" }}>
                  <div className="zs-eyebrow mb-1" style={{ color: "var(--warning)" }}>Conflict — the ladder settles it</div>
                  <div className="text-xs text-ink-soft" style={{ textDecoration: "line-through" }}>{d.conflict.loser}</div>
                  <div className="text-sm text-ink font-medium" style={{ marginTop: 2 }}>{d.conflict.winner}</div>
                </div>
              )}
              <div><div className="zs-eyebrow text-ink-mute mb-1">Distribution scope</div>
                <div className="flex flex-wrap gap-2">{d.scopes.map((s, i) => <K2Chip key={i} tone={i === 1 ? "var(--accent)" : "var(--ink-mute)"} soft={i === 1 ? "var(--accent-soft)" : "var(--paper-mute)"} edge={i === 1 ? "var(--accent-edge)" : "var(--paper-edge)"}>{s}</K2Chip>)}</div></div>
            </div>
            <div className="flex flex-wrap gap-2 border-t" style={{ padding: "var(--space-3) var(--space-4)", background: "var(--paper)" }}>
              <K2Btn size="sm" icon="check-circle">Approve</K2Btn>
              <K2Btn size="sm" variant="ghost" icon="pen-2">Revise</K2Btn>
              <span className="flex-1" />
              <K2Btn size="sm" variant="ghost" icon="close-circle">Decline</K2Btn>
            </div>
            {(cur.impact === "high" || cur.impact === "safety") && (
              <div className="border-t zs-meta" style={{ padding: "var(--space-2) var(--space-4)", background: "var(--paper)" }}>Approving sends this to a second maintainer before it publishes.</div>
            )}
          </div>
        )}
      </div>
    </W>
  );
}

// 2 · APPROVALS (maintainer) — second-approval queue.
function ScrApprovals({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const items = D2.consoles.approvals;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · govern"} title="Approvals" count={items.length} />
      <K2Banner kanji="承" tone="neutral" title="A second maintainer signs off high-impact and safety-relevant candidates.">
        One approval proposes; a second publishes. Nothing safety-relevant reaches a machine on a single signature.
      </K2Banner>
      {items.length ? (
        <div className="zs-card-flush" style={{ overflow: "hidden" }}>
          {items.map(a => {
            const im = K2_IMPACT[a.impact] || K2_IMPACT.high;
            return (
              <div key={a.id} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
                <K2KanjiToken char={a.kanji} size="lg" tone="var(--accent)" w={22} />
                <div className="flex-1" style={{ minWidth: 0 }}>
                  <div className="text-sm text-ink">{a.title}</div>
                  <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{a.scope} · first approval: {a.first} · {a.when}</div>
                </div>
                <K2Chip tone={im.tone} soft={im.soft} edge={im.edge}>{a.impact}</K2Chip>
                <K2Btn size="sm" variant="ghost" icon="eye">Review</K2Btn>
                <K2Btn size="sm" icon="check-circle">Approve</K2Btn>
              </div>
            );
          })}
        </div>
      ) : <div className="zs-card-flush"><K2EmptyState kanji="静" title="Nothing awaiting a second look.">High-impact candidates land here for a second signature. The queue is clear.</K2EmptyState></div>}
    </W>
  );
}

// 3 · KNOWLEDGE (+ catalog) (maintainer) — published library + prune + catalog.
const K2_EXT_ICON = { agent: "user-hands", command: "command", skill: "star" };
function ScrKnowledge({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const k = D2.consoles.knowledge;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · govern"} title="Knowledge"
        right={<div className="zs-input text-sm" style={{ height: 32, padding: "0 10px", width: "auto" }}><K2Icon name="alt-arrow-down" size={14} color="var(--ink-mute)" /><span className="text-ink-soft" style={{ whiteSpace: "nowrap" }}>{k.prunePolicy}</span></div>} />
      <K2Banner kanji="蔵" tone="neutral" title="The published library the dōjō has adopted — and what's due to be pruned.">
        Adopted knowledge distributes to every machine in scope. Unused teachings age out by the prune policy so the shared mind stays lean.
      </K2Banner>
      <K2ListSection icon="check-circle" iconColor="var(--success)" title="Active" count={k.active.length}>
        {k.active.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <K2KanjiToken char={r.kanji} size="lg" tone="var(--accent)" w={22} />
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink">{r.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.scope} · {r.adopted} · {r.age}</div></div>
            <K2Btn size="sm" variant="ghost" icon="pen-2">Edit</K2Btn>
          </div>
        ))}
      </K2ListSection>
      <K2ListSection icon="trash-bin-minimalistic" title="Pending prune" count={k.pending.length}>
        {k.pending.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <K2KanjiToken char={r.kanji} size="lg" tone="var(--ink-mute)" w={22} />
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink-soft">{r.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.scope} · {r.age}</div></div>
            <K2Btn size="sm" variant="ghost" icon="restart">Keep</K2Btn>
          </div>
        ))}
      </K2ListSection>
      <K2ListSection icon="widget-4" title="Catalog · skills, agents & commands" count={k.catalog.length}>
        {k.catalog.map((c, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <K2Icon name={K2_EXT_ICON[c.kind] || "widget-4"} size={18} color="var(--accent)" />
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink">{c.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{c.scope}</div></div>
            <K2Chip mono tone="var(--ink-mute)">{c.kind}</K2Chip>
          </div>
        ))}
      </K2ListSection>
    </W>
  );
}

// 4 · ENGAGEMENTS (lead) — client register + confidentiality model.
function ScrEngagements({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const c = D2.consoles.confidentiality;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · clients"} title="Engagements" count={D2.consoles.engagements.length}
        right={<K2Btn size="sm" icon="add-circle">New engagement</K2Btn>} />
      <K2Banner kanji="盾" tone="accent" title="Share the lesson, never the source.">
        Findings from a client project are anonymized before they leave the machine — the pattern travels upstream; the client, repo and code never do.
      </K2Banner>
      <K2ListSection icon="case-round" title="Client engagements" count={D2.consoles.engagements.length}>
        {D2.consoles.engagements.map(e => (
          <div key={e.id} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <K2KanjiToken char={e.kanji} size="lg" tone="var(--accent)" w={22} />
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm font-medium text-ink">{e.client}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{e.projects} · since {e.since}</div></div>
            <div className="text-right"><div className="mono text-sm text-ink">{e.lessons}</div><div className="zs-meta">lessons kept</div></div>
            <div className="text-right"><div className="mono text-sm text-ink-mute">{e.dropped}</div><div className="zs-meta">stripped</div></div>
            <K2Btn size="sm" variant="ghost" icon="document">Audit</K2Btn>
          </div>
        ))}
      </K2ListSection>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: "var(--space-4)" }}>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-3">What crosses the boundary</div>
          <div className="flex flex-col gap-2">
            {c.kept.map((x, i) => <div key={i} className="flex items-center gap-2 text-sm text-ink"><K2Icon name="check-circle" size={15} color="var(--success)" />{x}</div>)}
            {c.dropped.map((x, i) => <div key={i} className="flex items-center gap-2 text-sm text-ink-soft"><K2Icon name="close-circle" size={15} color="var(--danger)" />{x}</div>)}
          </div>
        </div>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-3">Anonymized before it leaves</div>
          <div className="mono text-xs" style={{ background: "var(--paper-mute)", border: "var(--hairline)", borderRadius: "var(--radius)", padding: "var(--space-3)", color: "var(--ink-soft)", textDecoration: "line-through" }}>{c.example.raw}</div>
          <div className="flex items-center justify-center" style={{ color: "var(--ink-faint)", padding: "var(--space-1)" }}>↓</div>
          <div className="mono text-xs" style={{ background: "var(--success-soft)", border: "1px solid var(--success-edge)", borderRadius: "var(--radius)", padding: "var(--space-3)", color: "var(--ink)" }}>{c.example.stripped}</div>
        </div>
      </div>
    </W>
  );
}

// 5 · INCIDENTS (lead) — confidentiality containment.
const K2_SEV = { high: { tone: "var(--danger)", soft: "var(--danger-soft)", edge: "var(--danger-edge)" }, medium: { tone: "var(--warning)", soft: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)" } };
const K2_ISTATE = { contained: "var(--warning)", resolved: "var(--success)", open: "var(--danger)" };
function ScrIncidents({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · clients"} title="Incidents" count={D2.consoles.incidents.length}
        right={<K2Btn size="sm" icon="add-circle">Report</K2Btn>} />
      <K2Banner kanji="盾" tone="warning" title="Contain a near-leak fast.">
        Leak-guard holds anything that looks like client source before it leaves. Log the containment, set retention, and control client read-access here.
      </K2Banner>
      <K2ListSection icon="shield-warning" title="Confidentiality incidents" count={D2.consoles.incidents.length}>
        {D2.consoles.incidents.map(it => {
          const sv = K2_SEV[it.severity] || K2_SEV.medium;
          return (
            <div key={it.id} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
              <K2KanjiToken char={it.kanji} size="lg" tone={sv.tone} w={22} />
              <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink">{it.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{it.client} · {it.when}</div></div>
              <K2Chip tone={sv.tone} soft={sv.soft} edge={sv.edge}>{it.severity}</K2Chip>
              <span className="inline-flex items-center gap-1 text-xs" style={{ color: K2_ISTATE[it.state] }}><span className="rounded-full" style={{ width: 6, height: 6, background: K2_ISTATE[it.state] }} />{it.state}</span>
              <K2Btn size="sm" variant="ghost" icon="alt-arrow-right">Open</K2Btn>
            </div>
          );
        })}
      </K2ListSection>
      <div className="flex flex-wrap gap-2">
        <K2Chip icon="lock-keyhole" tone="var(--ink-mute)">Retention · 1 year</K2Chip>
        <K2Chip icon="eye-closed" tone="var(--ink-mute)">Client read-access · off</K2Chip>
      </div>
    </W>
  );
}

// 6 · CLIENT AUDIT (lead) — immutable confidentiality ledger.
function ScrClientAudit({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · clients"} title="Client audit trail"
        right={<div className="flex gap-2"><K2Btn size="sm" variant="ghost" icon="tuning-2">Filter</K2Btn><K2Btn size="sm" variant="ghost" icon="download-minimalistic">Export</K2Btn></div>} />
      <K2Banner kanji="録" tone="neutral" title="An immutable ledger of exactly what left and what was stripped.">
        Distinct from the admin action-audit — this proves confidentiality held for each client, entry by entry. Append-only, exportable as CSV or JSON.
      </K2Banner>
      <div className="zs-card-flush" style={{ overflow: "hidden" }}>
        {D2.consoles.clientAudit.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <K2KanjiToken char={r.kanji} size="base" tone={r.ok ? "var(--ink-mute)" : "var(--danger)"} w={20} />
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink">{r.event}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.detail}</div></div>
            <K2Chip tone="var(--ink-mute)">{r.client}</K2Chip>
            {r.ok ? <K2Icon name="check-circle" size={16} color="var(--success)" /> : <K2Icon name="shield-warning" size={16} color="var(--danger)" />}
            <span className="mono text-xs text-ink-faint" style={{ width: 64, textAlign: "right" }}>{r.t}</span>
          </div>
        ))}
      </div>
      <div className="flex flex-wrap gap-2">
        <K2Chip icon="lock-keyhole" tone="var(--ink-mute)">Retention · 7 years</K2Chip>
        <K2Chip icon="eye" tone="var(--ink-mute)">Client read-access · Globex on</K2Chip>
      </div>
    </W>
  );
}

// 7 · IDENTITY & SSO (admin).
function ScrIdentity({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const id = D2.consoles.identity;
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · admin"} title="Identity & SSO" />
      <K2Banner kanji="鍵" tone="neutral" title="Connect org identity — SSO, provisioning, and how git maps to members.">
        Roles derive from these mappings; SSO and SCIM keep membership in step with your directory automatically.
      </K2Banner>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr", gap: "var(--space-4)" }}>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="flex items-center gap-2 mb-2"><K2Icon name="key" size={17} color="var(--accent)" /><span className="zs-eyebrow font-semibold text-ink-mute">Identity provider</span></div>
          <div className="flex items-center gap-2"><span className="text-lg font-medium text-ink">{id.idp.name}</span><K2Chip mono tone="var(--ink-mute)">{id.idp.protocol}</K2Chip><K2Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">{id.idp.status}</K2Chip></div>
          <div className="mono text-xs text-ink-faint" style={{ marginTop: 4 }}>{id.idp.domain}</div>
          <div style={{ marginTop: "var(--space-3)" }}><K2Btn size="sm" variant="ghost" icon="tuning-2">Configure</K2Btn></div>
        </div>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="flex items-center gap-2 mb-2"><K2Icon name="refresh-circle" size={17} color="var(--accent)" /><span className="zs-eyebrow font-semibold text-ink-mute">SCIM provisioning</span></div>
          <div className="flex items-center gap-2"><span className="text-sm text-ink">{id.scim ? "Enabled — members sync from your directory" : "Disabled"}</span></div>
          <div style={{ marginTop: "var(--space-3)" }}><K2Btn size="sm" variant="ghost" icon="tuning-2">Manage</K2Btn></div>
        </div>
      </div>
      <K2ListSection icon="link-circle" title="Identity mappings" count={id.mappings.length}
        right={<K2Btn size="sm" icon="add-circle">Add mapping</K2Btn>}>
        {id.mappings.map((m, i) => (
          <div key={i} className="flex items-center gap-4 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
            <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink">{m.source}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>→ {m.to}</div></div>
            <span className="mono text-xs text-ink-mute">{m.count} members</span>
            <K2Btn size="sm" variant="ghost" icon="pen-2">Edit</K2Btn>
          </div>
        ))}
      </K2ListSection>
    </W>
  );
}

// 8 · HEALTH / MONITOR (admin).
function ScrHealth({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const h = D2.consoles.health;
  const max = Math.max(...h.contribVsApprove.flatMap(x => [x.c, x.a]));
  return (
    <W>
      <K2SectionHead eyebrow={org.name + " · admin"} title="Health / Monitor" />
      <K2Banner kanji="観" tone="neutral" title="The shared mind's vital signs, fed by the audit trail.">
        Throughput, adoption and leak-guard at a glance. Anomalies surface as alerts before they become incidents.
      </K2Banner>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr 1fr" : "repeat(4, 1fr)", gap: "var(--space-4)" }}>
        {h.signals.map((s, i) => (
          <div key={i} className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
            <div className="flex items-center gap-2 mb-2"><K2KanjiToken char={s.kanji} size="base" tone={s.tone} /><span className="zs-meta">{s.label}</span></div>
            <K2StatBadge n={s.n} label="" sub={s.sub} tone={s.tone} />
          </div>
        ))}
      </div>
      <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1.3fr 1fr", gap: "var(--space-4)", alignItems: "start" }}>
        <div className="zs-card-flush" style={{ padding: "var(--space-4)" }}>
          <div className="flex items-center gap-2 mb-3"><span className="zs-eyebrow font-semibold text-ink-mute">Contributions vs approvals</span><span className="flex-1" /><K2Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">contrib</K2Chip><K2Chip tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">approved</K2Chip></div>
          <div className="flex items-end" style={{ gap: "var(--space-4)", height: 130, padding: "0 var(--space-2)" }}>
            {h.contribVsApprove.map((x, i) => (
              <div key={i} className="flex flex-col items-center gap-2" style={{ flex: 1 }}>
                <div className="flex items-end" style={{ gap: 4, height: 100 }}>
                  <div style={{ width: 14, height: (x.c / max * 100) + "%", background: "var(--accent)", borderRadius: "3px 3px 0 0" }} />
                  <div style={{ width: 14, height: (x.a / max * 100) + "%", background: "var(--success)", borderRadius: "3px 3px 0 0" }} />
                </div>
                <span className="mono text-xs text-ink-faint">{x.wk}</span>
              </div>
            ))}
          </div>
        </div>
        <K2ListSection icon="bell" title="Leak-guard & anomalies" count={h.alerts.length}>
          {h.alerts.map((a, i) => (
            <div key={i} className="flex items-start gap-3 border-b" style={{ padding: "var(--space-3) var(--space-4)" }}>
              <K2KanjiToken char={a.kanji} size="base" tone={a.sev === "warning" ? "var(--warning)" : "var(--success)"} w={20} />
              <div className="flex-1" style={{ minWidth: 0 }}><div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{a.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{a.detail} · {a.when}</div></div>
            </div>
          ))}
        </K2ListSection>
      </div>
    </W>
  );
}

/* ═══ SIGN-IN (adopted from the Dōjō console) ═════════════ */
function GhMark({ size = 18, color = "var(--paper)" }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill={color} aria-hidden="true" style={{ flexShrink: 0 }}>
      <path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.5v-1.9c-3.2.7-3.9-1.5-3.9-1.5-.5-1.3-1.3-1.7-1.3-1.7-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.8 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.4 11.4 0 0 1 6 0C17 5 18 5.3 18 5.3c.6 1.7.2 2.9.1 3.2.8.8 1.2 1.8 1.2 3.1 0 4.5-2.7 5.5-5.3 5.8.4.4.8 1.1.8 2.2v3.3c0 .3.2.6.8.5 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.3.5 12 .5z"/>
    </svg>
  );
}
function ScrSignIn({ label = "Sign in", mobile = false, onContinue }) {
  const [selfHost, setSelfHost] = React.useState(false);
  return (
    <div className="sensei" data-screen-label={label} style={{ width: "100%", height: "100%", display: "flex", flexDirection: mobile ? "column" : "row", overflow: mobile ? "auto" : "hidden", background: "var(--paper)" }}>
      {/* left · welcome + what sensei is */}
      <div style={{ width: mobile ? "100%" : "57%", flexShrink: 0, padding: mobile ? "var(--space-5)" : "var(--space-7)", display: "flex", flexDirection: "column", overflow: mobile ? "visible" : "auto",
        background: "linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)", borderRight: mobile ? "none" : "var(--hairline)", borderBottom: mobile ? "var(--hairline)" : "none" }}>
        <div className="flex items-center gap-2">
          <span className="kanji text-accent" style={{ fontSize: "var(--text-2xl)", lineHeight: 1 }}>結</span>
          <span className="display" style={{ fontSize: "var(--text-xl)", letterSpacing: "-0.01em" }}>Dōjō</span>
          <span className="mono text-ink-mute bg-paper" style={{ fontSize: "var(--text-xs)", border: "var(--hairline)", borderRadius: "var(--radius-full)", padding: "var(--space-1) var(--space-2)" }}>dojo.sensei-hq.com</span>
        </div>
        <div style={{ marginTop: "var(--space-7)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute" style={{ marginBottom: "var(--space-2)" }}>先生 Sensei</div>
          <h1 className="display" style={{ fontSize: mobile ? "var(--text-2xl)" : "var(--text-3xl)", fontWeight: 300, letterSpacing: "-0.02em", margin: 0, lineHeight: 1.08 }}>
            A quiet companion{mobile ? " " : <br/>}for your{mobile ? " " : <br/>}AI-assisted work.
          </h1>
          <p className="text-ink-soft" style={{ fontSize: "var(--text-sm)", lineHeight: 1.6, margin: "var(--space-4) 0 0", maxWidth: 440 }}>
            Sensei watches your coding sessions on this machine and surfaces the patterns you're too close to see. Yours alone by default — a dōjō is optional, for when you want to share what you learn with a team.
          </p>
        </div>
        <div style={{ display: "grid", gridTemplateColumns: mobile ? "1fr" : "1fr 1fr 1fr", gap: "var(--space-3)", marginTop: mobile ? "var(--space-5)" : "var(--space-6)" }}>
          {[["観", "Watches your sessions", "locally · on this machine"],
            ["己", "Your rules & guardrails", "yours to set and edit"],
            ["盾", "Nothing leaves", "unless you choose to share"]].map(([k, t, s]) => (
            <div key={k} className="bg-paper" style={{ border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)", display: mobile ? "flex" : "block", alignItems: "center", gap: "var(--space-3)" }}>
              <span className="kanji text-accent" style={{ fontSize: "var(--text-xl)", flexShrink: 0 }}>{k}</span>
              <div>
                <div className="text-ink" style={{ fontSize: "var(--text-sm)", fontWeight: 600, marginTop: mobile ? 0 : "var(--space-2)", lineHeight: 1.3 }}>{t}</div>
                <div className="text-ink-mute" style={{ fontSize: "var(--text-xs)", marginTop: "var(--space-1)" }}>{s}</div>
              </div>
            </div>
          ))}
        </div>
        <div className="bg-paper flex items-center gap-3" style={{ marginTop: "var(--space-3)", border: "var(--hairline)", borderRadius: "var(--radius-lg)", padding: "var(--space-4)" }}>
          <span className="kanji text-ink-mute" style={{ fontSize: "var(--text-xl)" }}>空</span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div className="text-ink" style={{ fontSize: "var(--text-sm)" }}>Sign in and sensei picks up where you left off.</div>
            <div className="text-ink-mute" style={{ fontSize: "var(--text-xs)", marginTop: "var(--space-1)" }}>No team, no setup required. <span style={{ fontStyle: "italic" }}>Still listening.</span></div>
          </div>
        </div>
        <div style={{ flex: mobile ? "none" : 1 }} />
        <div className="text-ink-faint" style={{ fontSize: "var(--text-xs)", marginTop: "var(--space-5)", lineHeight: 1.5 }}>
          Local-first · yours by default · join or create a dōjō later only to share with a team.
        </div>
      </div>
      {/* right · sign-in options */}
      <div className="flex items-center justify-center" style={{ flex: 1, minWidth: 0, padding: mobile ? "var(--space-5)" : "var(--space-6)" }}>
        <div style={{ width: 364, maxWidth: "100%" }}>
          <h2 className="display" style={{ fontSize: "var(--text-2xl)", fontWeight: 400, letterSpacing: "-0.015em", margin: 0, lineHeight: 1.1 }}>Sign in to continue</h2>
          <p className="text-ink-mute" style={{ fontSize: "var(--text-sm)", lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-5)" }}>
            GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
          </p>
          <button onClick={onContinue} className="zs-btn zs-btn-primary zs-btn-lg" style={{ width: "100%", justifyContent: "center" }}>
            <GhMark size={18} color="var(--paper)" /> Continue with GitHub
          </button>
          <div className="text-ink-faint" style={{ fontSize: "var(--text-xs)", textAlign: "center", marginTop: "var(--space-2)" }}>
            Derives your orgs &amp; roles from GitHub — and matches your repos.
          </div>
          <div className="flex items-center gap-3" style={{ margin: "var(--space-4) 0" }}>
            <span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} />
            <span className="mono text-ink-faint" style={{ fontSize: "var(--text-xs)", letterSpacing: ".1em" }}>OR</span>
            <span style={{ flex: 1, height: 1, background: "var(--paper-edge)" }} />
          </div>
          <label className="zs-eyebrow font-semibold text-ink-mute" style={{ display: "block", marginBottom: "var(--space-2)" }}>Work email</label>
          <div className="zs-input" style={{ marginBottom: "var(--space-2)" }}>
            <input placeholder="you@company.com" defaultValue="" style={{ border: "none", background: "transparent", outline: "none", width: "100%", font: "inherit", color: "var(--ink)" }} />
          </div>
          <button onClick={onContinue} className="zs-btn bg-paper border-1px" style={{ width: "100%", justifyContent: "center" }}>
            <K2Icon name="letter" size={16} color="var(--accent)" /> Email me a magic link
          </button>
          <div className="text-ink-faint" style={{ fontSize: "var(--text-xs)", textAlign: "center", marginTop: "var(--space-2)" }}>For organizations not on GitHub.</div>
          <div style={{ marginTop: "var(--space-5)", paddingTop: "var(--space-4)", borderTop: "1px solid var(--paper-edge)" }}>
            {!selfHost ? (
              <button onClick={() => setSelfHost(true)} className="flex items-center justify-center gap-2 text-ink-soft" style={{ width: "100%", background: "none", border: "none", cursor: "pointer", font: "inherit", fontSize: "var(--text-sm)" }}>
                <K2Icon name="server" size={15} color="var(--ink-mute)" />
                Connecting to a self-hosted dōjō? <span className="text-accent">Enter its URL →</span>
              </button>
            ) : (
              <div>
                <label className="zs-eyebrow font-semibold text-ink-mute" style={{ display: "block", marginBottom: "var(--space-2)" }}>Self-hosted dōjō URL</label>
                <div className="flex gap-2">
                  <div className="zs-input" style={{ flex: 1 }}>
                    <input defaultValue="dojo.acme.internal" style={{ border: "none", background: "transparent", outline: "none", width: "100%", font: "inherit", color: "var(--ink)" }} />
                  </div>
                  <button className="zs-btn bg-paper border-1px" style={{ whiteSpace: "nowrap" }}>Connect</button>
                </div>
                <div className="text-ink-faint" style={{ fontSize: "var(--text-xs)", marginTop: "var(--space-2)", lineHeight: 1.5 }}>
                  Same sign-in — your server authenticates you through GitHub (or an email magic link) on its own domain.
                </div>
              </div>
            )}
          </div>
          <div className="text-ink-faint" style={{ fontSize: "var(--text-xs)", textAlign: "center", marginTop: "var(--space-5)", lineHeight: 1.5 }}>
            One sign-in for the hosted SaaS and any self-hosted dōjō.
          </div>
        </div>
      </div>
    </div>
  );
}

// MY DŌJŌS — register / view the dōjōs you belong to (+ your role in each).
function ScrMyDojos({ onOpen, mobile }) {
  const W = mobile ? BodyM : Body;
  const byKind = (k) => D2.dojos.filter(d => d.kind === k);
  const groups = [["employer", "Employer"], ["client", "Clients"], ["community", "Communities"]];
  return (
    <W>
      <K2SectionHead eyebrow="You · membership" title="My dōjōs" count={D2.dojos.length}
        right={<K2Btn size="sm" icon="add-circle">Create or join</K2Btn>} />
      <K2Banner kanji="結" tone="neutral" title="A dōjō is how a team shares what it learns.">
        You belong to {D2.dojos.length} — your role in each is derived from git and only ever adds capability. Working solo needs none of them.
      </K2Banner>
      {D2.dojos.length ? groups.map(([k, lab]) => {
        const items = byKind(k); if (!items.length) return null;
        return (
          <K2ListSection key={k} icon={k === "employer" ? "buildings-2" : k === "client" ? "case-round" : "users-group-two-rounded"} title={lab} count={items.length}>
            {items.map(d => <K2MyDojoRow key={d.slug} dojo={d} onOpen={onOpen} />)}
          </K2ListSection>
        );
      }) : (
        <div className="zs-card-flush">
          <K2EmptyState kanji="空" title="No dōjōs yet.">You can work entirely solo — your rules and projects stay on your machine. Join a dōjō when a team wants to share what it learns.
            <div style={{ marginTop: 12 }}><K2Btn size="sm" icon="add-circle">Create or join a dōjō</K2Btn></div>
          </K2EmptyState>
        </div>
      )}
    </W>
  );
}

/* ═══ WIRED APP — desktop ══════════════════════════════════ */
function DojoApp2({ label, start = "you", startNav, startProject }) {
  const [signedIn, setSignedIn] = React.useState(start !== "signin");
  const [ctx, setCtx] = React.useState(start === "you" || start === "signin" ? "you" : start);   // "you" | org slug
  const org = D2.dojos.find(d => d.slug === ctx) || null;
  const [nav, setNav] = React.useState(startNav || (org ? "home" : "work"));
  const [proj, setProj] = React.useState(startProject || null);

  const onPick = (slug) => {
    if (slug === "you") { setCtx("you"); setNav("work"); }
    else { setCtx(slug); setNav("home"); }
    setProj(null);
  };
  const openProject = (p) => setProj(p);
  const back = () => setProj(null);
  const [resolved, setResolved] = React.useState({});
  const onAct = (item, action) => setResolved(r => ({ ...r, [item.id]: action.id === "deny" ? "denied" : action.id === "settle" ? "settled" : action.id === "decide" ? "decided" : "approved" }));
  const needsCount = D2.needsYou.filter(it => !resolved[it.id]).length;
  const onNeeds = () => { setProj(null); setNav(org ? "home" : "work"); };

  let body;
  if (!signedIn) return <ScrSignIn label={label} onContinue={() => { setSignedIn(true); setCtx("you"); setNav("work"); }} />;
  if (proj) body = <ScrProjectPreview project={proj} onBack={back} />;
  else if (!org) {
    body = ({
      work: <ScrYourWork onOpenProject={openProject} onAct={onAct} resolved={resolved} />,
      runs: <ScrRelayWatch />,
      projects: <ScrProjects projects={D2.projects} onOpenProject={openProject} eyebrow="Everything in flight" title="Your projects" />,
      dojos: <ScrMyDojos onOpen={(d) => onPick(d.slug)} />,
      contributions: <ScrContributions />,
      rules: <ScrConstitution onGoPacks={() => setNav("packs")} />,
      packs: <ScrRulePacks />,
      approve: <ScrRelayApprove />,
      decide: <ScrRelayDecide />,
      chat: <ScrRelayChat />,
    })[nav] || <ScrYourWork onOpenProject={openProject} onAct={onAct} resolved={resolved} />;
  } else {
    const projs = (D2.orgProjects[org.slug] || D2.orgProjects.acme).map(p => ({ ...p, repo: org.slug + "/" + p.name, note: p.team, lastRun: p.runsWeek + "/wk" }));
    body = ({
      home: <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} />,
      ladder: <ScrOrgLadder org={org} />,
      projects: <ScrProjects projects={projs} showDojo={false} onOpenProject={openProject} eyebrow={org.name + " · jurisdiction"} title="Projects" />,
      triage: <ScrTriage org={org} />,
      approvals: <ScrApprovals org={org} />,
      knowledge: <ScrKnowledge org={org} />,
      engagements: <ScrEngagements org={org} />,
      incidents: <ScrIncidents org={org} />,
      clientaudit: <ScrClientAudit org={org} />,
      members: <ScrRoleSurfaces org={org} tab="members" />,
      scopes: <ScrScopes org={org} />,
      identity: <ScrIdentity org={org} />,
      audit: <ScrRoleSurfaces org={org} tab="audit" />,
      health: <ScrHealth org={org} />,
      billing: <ScrBilling org={org} />,
    })[nav] || <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} />;
  }

  return (
    <K2AppShell label={label} context={org ? "org" : "you"} org={org || D2.dojos[0]} dojos={D2.dojos} me={D2.me}
      needsCount={needsCount} onNeeds={onNeeds}
      nav={org ? navForOrg(org) : NAV_YOU} active={proj ? null : nav} onNav={(id) => { setProj(null); setNav(id); }} onPick={onPick}>
      {body}
    </K2AppShell>
  );
}

/* ═══ WIRED APP — mobile ═══════════════════════════════════ */
function DojoApp2Mobile({ label, start = "you", startTab }) {
  const [signedIn, setSignedIn] = React.useState(start !== "signin");
  const [ctx, setCtx] = React.useState(start === "you" || start === "signin" ? "you" : start);
  const org = D2.dojos.find(d => d.slug === ctx) || null;
  const [tab, setTab] = React.useState(startTab || (org ? "home" : "work"));
  const [proj, setProj] = React.useState(null);
  const openProject = (p) => setProj(p);
  const back = () => setProj(null);
  const [resolved, setResolved] = React.useState({});
  const onAct = (item, action) => setResolved(r => ({ ...r, [item.id]: action.id === "deny" ? "denied" : action.id === "settle" ? "settled" : action.id === "decide" ? "decided" : "approved" }));

  let body;
  if (!signedIn) return <ScrSignIn label={label} mobile onContinue={() => { setSignedIn(true); setCtx("you"); setTab("work"); }} />;
  if (proj) body = <ScrProjectPreview project={proj} onBack={back} mobile />;
  else if (!org) {
    body = ({
      work: <ScrYourWork onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />,
      runs: <ScrRelayWatch mobile />,
      inbox: <ScrRelayDecide mobile />,
      chat: <ScrRelayChat mobile />,
    })[tab] || <ScrYourWork onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />;
  } else {
    const projs = (D2.orgProjects[org.slug] || D2.orgProjects.acme).map(p => ({ ...p, repo: org.slug + "/" + p.name, note: p.team, lastRun: p.runsWeek + "/wk" }));
    body = ({
      home: <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />,
      projects: <ScrProjects projects={projs} showDojo={false} onOpenProject={openProject} eyebrow={org.name} title="Projects" mobile />,
      ladder: <ScrOrgLadder org={org} mobile />,
      members: <ScrRoleSurfaces org={org} tab="members" mobile />,
    })[tab] || <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />;
  }

  return (
    <K2MobileShell label={label} context={org ? "org" : "you"} org={org || D2.dojos[0]} me={D2.me}
      tabs={org ? TABS_ORG : TABS_YOU} active={proj ? null : tab} onNav={(id) => { setProj(null); setTab(id); }}>
      {body}
    </K2MobileShell>
  );
}

Object.assign(window, {
  DojoApp2, DojoApp2Mobile, ScrSignIn, ScrMyDojos, ScrContributions, ScrScopes, ScrBilling,
  ScrTriage, ScrApprovals, ScrKnowledge, ScrEngagements, ScrIncidents, ScrClientAudit, ScrIdentity, ScrHealth,
  ScrYourWork, ScrProjects, ScrConstitution, ScrRulePacks, ScrProjectPreview,
  ScrRelayWatch, ScrRelayApprove, ScrRelayDecide, ScrRelayChat,
  ScrOrgHome, ScrOrgLadder, ScrRoleSurfaces,
});
