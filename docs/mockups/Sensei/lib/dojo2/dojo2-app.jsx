// Dōjō (web app) — screens + wired flow for the work-first redesign.
// Every screen is composed ONLY from the shared kit (Button · Card · Row ·
// SectionHead · ListSection …, exported on window by dojo2-kit.jsx) — no
// per-screen hand-rolling. Two small screen-level compositions (PackRow,
// MemberRow) are built here from kit atoms; flagged in the summary.
//
// Exports (window): DojoApp2 (wired desktop) · DojoApp2Mobile (wired phone),
// plus the individual screen bodies for the static per-screen artboards.

const {
  AppShell, MobileShell, ListSection, SectionHead, Banner, StatBadge,
  Button, Chip, Icon, ClassChip, RoleTag, PhasePill, KanjiMark, EmptyState,
  DojoRow, ProjectRow, NeedsYouBand, RunCard, GateCard, DecisionCard, ChatThread,
  PlanOutline, allTasks, SubTabs, InboxRow, toInboxRow, toInboxRows, RUN_STATUS,
  LadderRung, RuleRow, ConflictCard, StanceDial,
} = window;
// No fixture alias on purpose: every screen reads through ZS_API
// (lib/data/dojo2-api.js), so pointing that module at real endpoints is
// the whole migration. A `const D2 = window.DOJO2` here would be the
// back door that makes the seam a fiction.

/* ── nav + tab definitions ─────────────────────────────── */
// Personal nav: one Inbox holds every in-flight session; approve / decide /
// chat are actions inside a session, not surfaces of their own.
const navYou = (needs) => [
  { group: "Work", items: [
    { id: "inbox", icon: "inbox", label: "Inbox", badge: needs || undefined },
    { id: "projects", icon: "folder", label: "Projects" },
  ]},
  { group: "Govern", items: [
    { id: "rules", icon: "scale", label: "Constitution" },
    { id: "packs", icon: "box", label: "Rule packs" },
  ]},
  { group: "Dōjōs", items: [
    { id: "dojos", icon: "users-group-two-rounded", label: "My dōjōs" },
    { id: "contributions", icon: "upload-square", label: "Contributions", badge: 1 },
  ]},
];
// Org nav: three destinations everyone sees, then one destination per zone of
// responsibility. Everything that used to be a rail item is a tab inside its
// zone — one level of nesting instead of fourteen top-level sections.
const ORG_GROUPS = {
  governance: { label: "Governance", icon: "scale", role: "maintainer", badge: 7, tabs: [
    { id: "triage", label: "Triage", icon: "inbox", badge: 5 },
    { id: "approvals", label: "Approvals", icon: "clipboard-check", badge: 2 },
    { id: "knowledge", label: "Knowledge", icon: "book-2" },
  ]},
  clients: { label: "Clients", icon: "case-round", role: "lead", badge: 1, tabs: [
    { id: "engagements", label: "Engagements", icon: "case-round" },
    { id: "incidents", label: "Incidents", icon: "shield-warning", badge: 1 },
    { id: "clientaudit", label: "Client audit", icon: "document-text" },
  ]},
  admin: { label: "Admin", icon: "shield-user", role: "admin", tabs: [
    { id: "members", label: "Members", icon: "users-group-rounded" },
    { id: "roles", label: "Roles", icon: "shield-check" },
    { id: "scopes", label: "Scopes", icon: "tuning-2" },
    { id: "identity", label: "Identity", icon: "key" },
    { id: "audit", label: "Audit", icon: "clipboard-list" },
    { id: "health", label: "Health", icon: "pulse" },
    { id: "billing", label: "Billing", icon: "card" },
  ]},
};
// Legacy deep links (a tab id) resolve to their zone.
const ORG_ZONE_OF = Object.keys(ORG_GROUPS).reduce((m, g) => {
  ORG_GROUPS[g].tabs.forEach(t => { m[t.id] = g; });
  return m;
}, {});
// Additive role rank — a zone shows when the viewer's role reaches its floor.
const ROLE_RANK = { developer: 0, maintainer: 1, lead: 2, admin: 3 };
function navForOrg(org) {
  const rank = ROLE_RANK[org && org.role] != null ? ROLE_RANK[org.role] : 0;
  const zones = Object.keys(ORG_GROUPS)
    .filter(g => rank >= ROLE_RANK[ORG_GROUPS[g].role])
    .map(g => ({ id: g, icon: ORG_GROUPS[g].icon, label: ORG_GROUPS[g].label, badge: ORG_GROUPS[g].badge }));
  return [
    { group: "Dōjō", items: [
      { id: "home", icon: "buildings-2", label: "Home" },
      { id: "ladder", icon: "scale", label: "Constitution" },
      { id: "teams", icon: "users-group-rounded", label: "Teams", badge: 8 },
      { id: "projects", icon: "folder", label: "Projects", badge: 4 },
    ]},
    ...(zones.length ? [{ group: "Manage", items: zones }] : []),
  ];
}
const tabsYou = (needs) => [
  { id: "inbox", icon: "inbox", label: "Inbox", badge: needs || undefined },
  { id: "projects", icon: "folder", label: "Projects" },
  { id: "rules", icon: "scale", label: "Rules" },
  { id: "dojos", icon: "users-group-two-rounded", label: "Dōjōs" },
];
const TABS_ORG = [
  { id: "home", icon: "buildings-2", label: "Home" },
  { id: "projects", icon: "folder", label: "Projects", badge: 4 },
  { id: "teams", icon: "users-group-rounded", label: "Teams", badge: 8 },
  { id: "ladder", icon: "scale", label: "Rules" },
];

// gap is a spacing-scale stop (Tailwind numbering, 4px x n), not a px value.
const Body = ({ children, gap = 6 }) => (
  <div className="p-8 flex flex-col" style={{ gap: `var(--space-${gap})` }}>{children}</div>
);
const BodyM = ({ children, gap = 4 }) => (
  <div className="p-4 flex flex-col" style={{ gap: `var(--space-${gap})` }}>{children}</div>
);

/* ── back header (drill-in) ────────────────────────────── */
function BackHead({ onBack, children }) {
  return (
    <Button variant="link" icon="alt-arrow-left" onClick={onBack}>{children || "Back"}</Button>
  );
}

/* ═══ PERSONAL SCREENS ═════════════════════════════════════ */

// urgency rank for ordering the needs-you band (gate → conflict → decision → review)
const URGENCY = { gate: 0, conflict: 1, decision: 2, review: 3 };
const NEEDS_PRIMARY_ACTION = { gate: "Approve", conflict: "Settle", decision: "Decide", review: "Approve" };
function orderNeeds(items) {
  return items.slice().sort((a, b) => (URGENCY[a.kind] - URGENCY[b.kind]) || 0);
}

// PROJECTS — the full list (personal or org).
function ScrProjects({ projects, showDojo = true, onOpenProject, eyebrow, title, mobile }) {
  const W = mobile ? BodyM : Body;
  return (
    <W>
      <SectionHead eyebrow={eyebrow} title={title} count={projects.length}
        right={<Button size="sm" variant="ghost" icon="tuning-2">Filter</Button>} />
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
        {projects.map(p => <ProjectRow key={p.id} p={p} onOpen={onOpenProject} showDojo={showDojo && !mobile} compact={mobile} />)}
      </div>
    </W>
  );
}

// (2) CONSTITUTION — stance dials + your effective personal constitution.
function ScrConstitution({ onGoPacks, mobile }) {
  const W = mobile ? BodyM : Body;
  const ladder = useAsync(() => ZS_API.getLadder(), "ladder", []).data || [];
  const stance = useAsync(() => ZS_API.getStance(), "stance", []).data || [];
  const personalRungs = ladder.filter(r => r.id === "personal" || r.id === "stack");
  return (
    <W>
      <SectionHead eyebrow="You · standing rules" title="Your constitution"
        right={!mobile && <Button size="sm" variant="ghost" icon="box" onClick={onGoPacks}>Rule packs →</Button>} />
      <Banner kanji="静" tone="neutral" title="Everything sensei keeps is derived and stays on your machine.">
        Your stance sets how far a session runs and what surfaces to a dōjō. A classification change alters which rules apply — never what leaves.
      </Banner>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)" }}>
        {stance.map(s => <StanceDial key={s.id} dial={s} />)}
      </div>
      <div>
        <div className="flex items-center gap-2 mb-3" >
          <Icon name="scale" size={17} color="var(--accent)" />
          <span className="zs-eyebrow font-semibold text-ink">Your effective constitution</span>
          <span className="zs-meta">— personal + stack · every project you touch</span>
        </div>
        <div className="flex flex-col gap-3">
          {personalRungs.map(r => <LadderRung key={r.id} rung={r} active={null} onSelect={() => {}} />)}
        </div>
      </div>
    </W>
  );
}

// (2b) RULE PACKS — adoptable rule bundles.
function PackRow({ pack }) {
  const [open, setOpen] = React.useState(false);
  const rules = pack.rules || [];
  return (
    <div className="border-b">
      <div className="flex items-center gap-4 py-3 px-4" >
        <Icon name="box" size={20} color="var(--accent)" style={{ marginTop: 0 }} />
        <div className="flex-1 min-w-0" >
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-ink">{pack.name}</span>
            <button
 onClick={() => setOpen(o => !o)}
 disabled={!rules.length}
 title={rules.length ? (open ? "Hide rules" : "Show the rules in this pack") : ""}
 className="flex items-center gap-1 border-0 p-0"
 style={{ background: "none", cursor: rules.length ? "pointer" : "default" }}>
              <Chip mono tone={open ? "var(--accent)" : "var(--ink-mute)"}>{pack.count} rules</Chip>
              {rules.length ? (
                <Icon name="alt-arrow-down" size={13} color="var(--ink-mute)"
                  style={{ transform: open ? "rotate(180deg)" : "none", transition: "transform .18s" }} />
              ) : null}
            </button>
          </div>
          <div className="zs-meta" style={{ marginTop: 2 }}>by {pack.by} · {pack.note}</div>
        </div>
        {pack.adopted
          ? <Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</Chip>
          : <Button size="sm" variant="ghost" icon="add-circle">Adopt</Button>}
      </div>
      {open && rules.length ? (
        <div className="bg-paper-mute border-t" style={{ padding: "var(--space-2) var(--space-4) var(--space-3)", paddingLeft: "calc(var(--space-4) + 20px + var(--space-4))" }}>
          <div className="zs-eyebrow text-ink-mute mb-2" >What this pack adds</div>
          <div className="flex flex-col gap-1" >
            {rules.map((r, i) => {
              const guard = r.tone === "guard";
              return (
                <div key={i} className="flex items-center gap-3 py-1 px-0" >
                  <Icon name={guard ? "shield-check" : "check-circle"} size={15}
                    color={guard ? "var(--danger)" : "var(--ink-mute)"} style={{ marginTop: 0, flexShrink: 0 }} />
                  <span className="text-sm text-ink flex-1 min-w-0" >{r.title}</span>
                  {guard ? <Chip mono tone="var(--danger)" soft="var(--danger-soft)" edge="var(--danger-edge)">guard</Chip> : null}
                </div>
              );
            })}
          </div>
        </div>
      ) : null}
    </div>
  );
}
function ScrRulePacks({ mobile }) {
  const W = mobile ? BodyM : Body;
  const packs = useAsync(() => ZS_API.getRulePacks(), "packs", []).data || [];
  const adopted = packs.filter(p => p.adopted), avail = packs.filter(p => !p.adopted);
  return (
    <W>
      <SectionHead eyebrow="Adopt · not a library" title="Rule packs"
        right={<Button size="sm" variant="ghost" icon="tuning-2">Browse all</Button>} />
      <Banner kanji="束" tone="neutral" title="Packs are bundles of rules you adopt into your constitution.">
        Adopting a pack adds its rules at the scope you choose. You can drop any single rule later — a pack is a starting point, not a lock.
      </Banner>
      <ListSection icon="check-circle" iconColor="var(--success)" title="Adopted" count={adopted.length} countTone="var(--success)">
        {adopted.map(p => <PackRow key={p.id} pack={p} />)}
      </ListSection>
      <ListSection icon="box" title="Available" count={avail.length}>
        {avail.map(p => <PackRow key={p.id} pack={p} />)}
      </ListSection>
    </W>
  );
}

// (3/7) PROJECT CONSTITUTION PREVIEW — the resolved ladder for one project.
function previewRungs(p, ladder) {
  const byId = Object.fromEntries((ladder || []).map(r => [r.id, r]));
  let ids;
  if (p.classification === "personal") ids = ["personal", "project", "stack"];
  else if (p.classification === "client") ids = ["company", "client", "personal", "project", "stack"];
  else ids = ["company", "personal", "project", "stack"];
  return ids.map(id => {
    const r = byId[id];
    if (!r) return null;                       // ladder not loaded yet
    if (id === "project") return { ...r, name: p.name };
    return r;
  }).filter(Boolean);
}
function ScrProjectPreview({ project, onBack, mobile }) {
  const W = mobile ? BodyM : Body;
  const ladder = useAsync(() => ZS_API.getLadder(), "ladder", []).data || [];
  const projects = useAsync(() => ZS_API.getProjects(), "projects", []).data || [];
  const conflicts = useAsync(() => ZS_API.getConflicts(), "conflicts", []).data || [];
  const p = project || projects[0];
  const rungs = p ? previewRungs(p, ladder) : [];
  const [active, setActive] = React.useState("project");
  const [view, setView] = React.useState("layer"); // layer | consolidated
  const ready = !!p && rungs.length > 0;
  const locks = rungs.reduce((a, r) => a + (r.rules || []).filter(x => x.hard).length, 0);
  if (!ready) return <W><EmptyState kanji="静" title="Still listening.">Resolving this project’s constitution.</EmptyState></W>;
  const showConflicts = p && p.classification !== "personal";
  const discarded = showConflicts ? conflicts.map(c => c.loser.text) : [];
  const effective = rungs.flatMap(r => (r.rules || []).map(x => ({ ...x, level: r.scope })))
    .filter(x => !discarded.includes(x.text));
  const Toggle = () => (
    <SegmentedControl value={view} onPick={setView}
      options={[{ id: "layer", label: "By layer" }, { id: "consolidated", label: "Consolidated" }]} />
  );
  return (
    <W>
      {onBack && <BackHead onBack={onBack}>Back to projects</BackHead>}
      <SectionHead kanji="件" eyebrow={"Before you start · " + p.repo} title={p.name}
        right={<div className="flex items-center gap-2"><ClassChip kind={p.classification} /><PhasePill phase={p.phase} /></div>} />
      <Banner kanji="観" tone={p.classification === "client" ? "accent" : "neutral"}
        title={p.classification === "client" ? "Client engagement — sources are dereferenced." : "These rules resolve here, most-specific wins."}>
        {rungs.length} scopes compose this constitution · {effective.length} rules apply · {locks} locked (★){discarded.length ? " · " + discarded.length + " discarded by the ladder" : ""}.
      </Banner>
      <div>
        <div className="flex items-center gap-2 mb-3" >
          <Icon name="layers-minimalistic" size={17} color="var(--accent)" />
          <span className="zs-eyebrow font-semibold text-ink">{view === "layer" ? "The ladder — broad → specific" : "Consolidated constitution"}</span>
          <span className="zs-meta">{view === "layer" ? "tap a rung to focus" : effective.length + " effective rules"}</span>
          <span className="flex-1" />
          <Toggle />
        </div>
        {view === "layer" ? (
          <div className="flex flex-col gap-3">
            {rungs.map(r => <LadderRung key={r.id} rung={r} active={active} onSelect={setActive} />)}
          </div>
        ) : (
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
            {effective.map((r, i) => <RuleRow key={i} rule={r} showLevel onJump={(rule) => {
              const hit = rungs.find(x => x.scope === rule.level);
              if (hit) { setActive(hit.id); setView("layer"); }
            }} />)}
          </div>
        )}
      </div>
      {showConflicts && (
        <div>
          <div className="flex items-center gap-2 mb-3" >
            <Icon name="danger-triangle" size={17} color="var(--warning)" />
            <span className="zs-eyebrow font-semibold text-ink">Discarded by the ladder</span>
            <span className="zs-meta">what the resolution eliminated, and why</span>
          </div>
          <div className="flex flex-col gap-3">
            {conflicts.map(c => <ConflictCard key={c.id} conflict={c} />)}
          </div>
        </div>
      )}
    </W>
  );
}

/* ═══ INBOX — every in-flight session, one list ════════════ */

// What a session is waiting on you for: the asks it raised and can't
// answer itself. Each names the task it blocks and is answered in the
// session detail.
//
// `asks` is passed in, never read from a global — that's what lets the
// same function work against a fixture today and GET /you/runs/:id/asks
// tomorrow. Callers get the list from ZS_API.getAsks().
function pendingAsksFor(run, asks) {
  return (asks || []).filter(a => a.run === run.id);
}
const ASK_KIND = {
  approval: { kanji: "認", label: "approval" },
  choice:   { kanji: "岐", label: "decision" },
  recovery: { kanji: "阻", label: "recovery" },
  clarification: { kanji: "問", label: "clarification" },
};

const INBOX_FILTERS = [
  { id: "needs", label: "Needs you" },
  { id: "running", label: "Running" },
  { id: "done", label: "Finished" },
  { id: "all", label: "All" },
];

function ScrInbox({ startRun, answered = {}, onAnswer, mobile }) {
  const [openId, setOpen] = React.useState(startRun || null);
  const [filter, setFilter] = React.useState("needs");
  const answer = (item, verdict) => onAnswer && onAnswer(item, verdict);
  // GET /you/inbox + GET /you/asks — one call each, never per row.
  const inbox = useAsync(() => ZS_API.getInbox(), "inbox", []);
  const askList = useAsync(() => ZS_API.getAsks(), "asks", []);
  const runs = inbox.data || [];
  const asks = askList.data || [];
  const pending = (run) => pendingAsksFor(run, asks).filter(it => !answered[it.id]);
  const rows = toInboxRows(runs, pending);
  const shown = rows.filter(r =>
    filter === "all" ? true :
    filter === "needs" ? (r.needs > 0 || r.attention) :
    filter === "running" ? r.status === "running" :
    r.status === "done");
  const needTotal = rows.reduce((a, r) => a + r.needs, 0);

  // Phone: the list pushes to a detail. Desktop: they sit side by side.
  if (mobile) {
    const open = openId && runs.find(r => r.id === openId);
    if (open) return <RunDetail run={open} asks={asks} onBack={() => setOpen(null)} answered={answered} onAnswer={answer} mobile />;
    return (
      <BodyM>
        <InboxHead needTotal={needTotal} total={runs.length} filter={filter} setFilter={setFilter} />
        <InboxList rows={shown} onOpen={(r) => setOpen(r.id)} mobile />
      </BodyM>
    );
  }
  const selId = (openId && shown.some(r => r.run.id === openId) ? openId : (shown[0] && shown[0].run.id)) || null;
  const sel = selId && runs.find(r => r.id === selId);
  return (
    <div className="grid items-stretch min-h-full" style={{ gridTemplateColumns: "minmax(340px, 400px) minmax(0, 1fr)" }}>
      <div className="border-r" >
        <div className="flex flex-col gap-4 py-8 px-6 sticky" style={{ top: 0 }}>
          <InboxHead needTotal={needTotal} total={runs.length} filter={filter} setFilter={setFilter} />
          <InboxList rows={shown} selId={selId} onOpen={(r) => setOpen(r.id)} compact />
          <div className="zs-meta">Sorted by what waits on you — then stalled or blocked, then running, then finished.</div>
        </div>
      </div>
      <div className="min-w-0" >
        {sel
          ? <RunDetail key={sel.id} run={sel} asks={asks} answered={answered} onAnswer={answer} embedded />
          : <div className="p-8" ><EmptyState kanji="空" title="Nothing here.">No session matches that view right now.</EmptyState></div>}
      </div>
    </div>
  );
}

function InboxHead({ needTotal, total, filter, setFilter }) {
  return (
    <React.Fragment>
      <SectionHead eyebrow="You · in flight" title="Inbox" count={total}
        right={needTotal > 0 ? <span className="mono text-xs text-accent">{needTotal} need you</span> : null} />
      <SubTabs tabs={INBOX_FILTERS} active={filter} onPick={setFilter} />
    </React.Fragment>
  );
}
function InboxList({ rows, selId, onOpen, mobile, compact }) {
  if (!rows.length) return <EmptyState kanji="空" title="Nothing here.">No session matches that view right now.</EmptyState>;
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      {rows.map(r => <InboxRow key={r.run.id} row={r} selected={r.run.id === selId} onOpen={onOpen} />)}
    </div>
  );
}

// One ask, answered the way sensei asks it: numbered options, or type your own.
function AskCard({ ask: it, run, verdict, onAnswer, onShowInPlan }) {
  const [reply, setReply] = React.useState("");
  const k = ASK_KIND[it.kind] || ASK_KIND.choice;
  const choices = it.options || [];
  const n = parseInt(reply.trim(), 10);
  const picked = String(n) === reply.trim() && n >= 1 && n <= choices.length ? choices[n - 1] : null;
  const ready = !!reply.trim();
  const submit = () => { if (ready) onAnswer(it, picked ? "Answered · " + picked : "Answered · “" + reply.trim() + "”"); };
  if (verdict) return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4 flex items-start gap-3 " >
      <KanjiMark char="了" size="lg" tone="var(--success)" w={22} />
      <div className="min-w-0 flex-1" >
        <div className="text-sm text-ink-mute">{it.question}</div>
        <div className="text-sm text-ink" style={{ marginTop: 2 }}>{verdict}</div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{run.id} resumed from {it.task} · {it.taskTitle}</div>
      </div>
    </div>
  );
  return (
    <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
      <div className="flex items-start gap-3 p-4 bg-accent-soft border-b" >
        <KanjiMark char={k.kanji} size="lg" tone="var(--accent)" w={22} />
        <div className="min-w-0 flex-1" >
          <div className="flex items-center gap-2 flex-wrap" >
            <span className="zs-eyebrow font-semibold text-accent" >{k.label}</span>
            {it.severity === "blocking" && <Chip mono tone="var(--accent)" soft="var(--paper)" edge="var(--accent-edge)">blocking</Chip>}
            <span className="flex-1" />
            <span className="mono text-xs text-ink-faint">{it.age}</span>
          </div>
          <div className="text-sm font-medium text-ink" style={{ marginTop: 3, lineHeight: 1.4 }}>{it.question}</div>
          <div className="zs-body-sm" style={{ marginTop: 2 }}>{it.context}</div>
          <Button variant="link" size="sm" onClick={() => onShowInPlan && onShowInPlan(it.task)}
            style={{ marginTop: 4 }}>holds {it.task} · {it.taskTitle} →</Button>
        </div>
      </div>
      <div className="flex flex-col gap-2" style={{ padding: "var(--space-3) var(--space-4) var(--space-4)" }}>
        <div className="flex flex-col" style={{ gap: 2 }}>
          {choices.map((o, i) => {
            return (
              <NumberedChoice key={o} n={i + 1} label={o} selected={picked === o}
                onPick={() => setReply(String(i + 1))} />
            );
          })}
        </div>
        <div className="flex items-center gap-2">
          <div className="zs-input flex-1 min-w-0" style={{ height: 38 }}>
            <span className="mono text-xs text-ink-faint shrink-0" >›</span>
            <input value={reply} onChange={e => setReply(e.target.value)} onKeyDown={e => { if (e.key === "Enter") submit(); }}
 placeholder={"type 1–" + choices.length + ", or tell sensei what to do instead…"}
 className="text-sm text-ink flex-1 min-w-0 bg-transparent border-0" style={{ outline: "none" }} />
          </div>
          <Button size="sm" icon="check-circle" onClick={submit} style={ready ? null : { opacity: 0.45 }}>Send answer</Button>
        </div>
        <span className="zs-meta">{ready
          ? (picked ? picked + " · " : "Your own instruction · ") + run.id + " resumes from " + it.task + "."
          : "The run holds here until you answer."}</span>
      </div>
    </div>
  );
}

// The session detail — two tabs: what needs answering, and the plan. It opens
// on whichever is true: an ask if the run raised one, the plan if it didn't.
function RunDetail({ run, asks = [], onBack, answered = {}, onAnswer, embedded, mobile }) {
  const W = mobile ? BodyM : Body;
  const [selTask, setSelTask] = React.useState(null);
  const pr = planProgress(run.plan || []);
  const items = pendingAsksFor(run, asks);
  const openItems = items.filter(it => !answered[it.id]);
  const [tab, setTab] = React.useState(openItems.length ? "needs" : "plan");
  React.useEffect(() => { setTab(openItems.length ? "needs" : "plan"); setSelTask(null); }, [run.id]);
  const rowInfo = toInboxRow(run, openItems.length);
  const st = RUN_STATUS[rowInfo.status] || RUN_STATUS.waiting;
  const tone = openItems.length ? "var(--accent)" : st.tone;
  const task = allTasks(run.plan || []).find(t => t.id === selTask);
  const thread = useAsync(() => ZS_API.getThread(run.id), "thread:" + run.id, []).data;
  const me = useAsync(() => ZS_API.getMe(), "me", {}).data || {};
  const goal = run.plan && !Array.isArray(run.plan) ? run.plan.goal : null;
  const showInPlan = (id) => { setSelTask(id); setTab("plan"); };
  const tabs = [
    { id: "needs", label: "Needs you", icon: "checklist-minimalistic", badge: openItems.length || undefined },
    { id: "plan", label: "Plan", icon: "layers-minimalistic" },
  ];
  return (
    <W>
      {!embedded && <BackHead onBack={onBack}>Inbox</BackHead>}
      <div className="flex items-baseline gap-3">
        <KanjiMark char="観" size="lg" tone={tone} />
        <div className="flex-1 min-w-0" >
          <div className="zs-eyebrow text-ink-mute">Session · {run.id}</div>
          <div className="zs-h3 text-ink" style={{ marginTop: 2 }}>{run.task}</div>
          <div className="mono text-xs text-ink-mute" style={{ marginTop: 3 }}>{run.project} · {run.assistant} · {run.elapsed} · {run.edits} edits · last activity {run.last}</div>
        </div>
        <Chip mono tone={st.tone} soft={st.soft}>{st.label}</Chip>
      </div>
      <div style={{ maxWidth: mobile ? "100%" : 560 }}>
        <div className="flex items-center justify-between mb-2" >
          <span className="mono text-xs text-ink-soft">Phase {pr.stage} of {pr.stages} · {pr.stageName}</span>
          <span className="mono text-xs text-ink-soft">{pr.done}/{pr.total} tasks · {pr.pct}%</span>
        </div>
        <PlanBar pct={pr.pct} tone={tone} />
      </div>

      <div className="border-b pb-3" >
        <SubTabs tabs={tabs} active={tab} onPick={setTab} />
      </div>

      {tab === "needs" ? (
        items.length ? (
          <div className="flex flex-col gap-3" >
            {items.map(it => (
              <AskCard key={it.id} ask={it} run={run} verdict={answered[it.id]} onAnswer={onAnswer} onShowInPlan={showInPlan} />
            ))}
          </div>
        ) : (
          <EmptyState kanji="静" title={
            rowInfo.attention === "stalled" ? "Quiet for " + run.last + " — no heartbeat."
              : rowInfo.status === "done" ? "Finished." : "Nothing waits on you."}>
            {rowInfo.status === "done"
              ? "The session note is written. Anything worth keeping is offered to your dōjō, never sent."
              : "sensei keeps going and surfaces only what it can't decide alone."}
          </EmptyState>
        )
      ) : (
        <ListSection icon="layers-minimalistic" iconColor="var(--ink-mute)" title="Plan"
          count={pr.stages + " phases · " + pr.total + " tasks"}
          right={<span className="mono text-xs text-ink-faint">tap a task for detail</span>}>
          {goal && <div className="zs-meta border-b py-3 px-4" >Goal · {goal}</div>}
          <PlanOutline plan={run.plan} mobile={mobile} onSelect={t => setSelTask(t.id)} selectedId={selTask} />
          {task && (
            <div className="border-t py-3 px-4 bg-paper" >
              <div className="flex items-center gap-2">
                <KanjiMark char={task.is_gate ? "認" : task.state === "failed" ? "阻" : "刻"} size="base" tone={task.is_gate || task.state === "failed" ? "var(--accent)" : "var(--ink-mute)"} />
                <span className="text-sm font-medium text-ink flex-1 min-w-0" >{task.title}</span>
                <span className="mono text-xs text-ink-faint">{task.id}</span>
              </div>
              <div className="zs-body-sm" style={{ marginTop: 4 }}>
                {(TASK_STATE[task.state] || {}).label}
                {task.agent ? " · " + task.agent : ""}{task.model ? " · " + task.model : ""}
                {task.deps.length ? " · waits on " + task.deps.join(", ") : ""}
                {task.spec_ref ? " · " + task.spec_ref : ""}
                {task.summary ? " · " + task.summary : ""}.
                {items.some(it => it.task === task.id && !answered[it.id])
                  ? " This task raised a question — answer it in Needs you."
                  : " Sensei applies the run's constitution to this task on your machine; the dōjō only sees what you let it."}
              </div>
            </div>
          )}
        </ListSection>
      )}

      <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr" }}>
        <ListSection icon="history" iconColor="var(--ink-mute)" title="Activity"
          right={<a href="#" className="mono text-xs text-ink-mute no-underline" >full log →</a>}>
          <RunActivity feed={run.feed || []} />
        </ListSection>
        <ListSection icon="chat-round-line" iconColor="var(--ink-mute)" title="Conversation">
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4 flex flex-col gap-3" >
            {thread && thread.length
              ? <ChatThread thread={thread} me={me} />
              : <div className="zs-body-sm">Nothing said in this session yet. Ask sensei anything about the plan above.</div>}
            <div className="zs-input" style={{ height: 42 }}>
              <Icon name="chat-round-line" size={16} color="var(--ink-mute)" />
              <span className="text-ink-faint text-sm">reply to sensei…</span>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button size="sm" variant="ghost" icon="pause">Pause run</Button>
              <Button size="sm" variant="ghost" icon="eye">Open in Observatory</Button>
            </div>
          </div>
        </ListSection>
      </div>
    </W>
  );
}

/* ═══ ORG SCREENS ══════════════════════════════════════════ */

// (5) ORG HOME — projects in the dōjō's jurisdiction + org needs.
function ScrOrgHome({ org, onOpenProject, onAct, resolved, mobile }) {
  const W = mobile ? BodyM : Body;
  const projs = useAsync(() => ZS_API.getProjects(org.slug), "orgprojects:" + org.slug, []).data || [];
  const orgNeeds = (useAsync(() => ZS_API.getOrgNeeds(), "orgneeds", []).data || []).slice(0, 2);
  const Div = () => <span className="bg-paper-edge shrink-0" style={{ width: 1, height: 34 }} />;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · jurisdiction"} title={projs.length + " projects under this dōjō"} />
      {!mobile && (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden flex items-center py-4 px-8 gap-12" >
          <StatBadge n={org.members} label="members" />
          <Div />
          <StatBadge n={org.needs} label="need a maintainer" sub="across this jurisdiction" tone="var(--accent)" />
          <Div />
          <StatBadge n={org.projects} label="projects" sub="in flight" />
        </div>
      )}
      <NeedsYouBand items={orgNeeds} onAct={onAct} resolved={resolved} title="Needs a maintainer" mobile={mobile} />
      <ListSection icon="folder" title="Projects" count={projs.length}
        right={<Button size="sm" variant="ghost" icon="tuning-2">By team</Button>}>
        {projs.map(p => <ProjectRow key={p.id} p={p} onOpen={onOpenProject} showDojo={false} compact={mobile} />)}
      </ListSection>
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
    <div className="absolute flex items-center justify-center p-8" style={{ inset: 0, zIndex: 80, background: "oklch(0.22 0.012 50 / 0.28)" }} onClick={onClose}>
      <div className="bg-paper rounded-lg shadow-lg max-w-full overflow-hidden" style={{ width: 520 }} onClick={e => e.stopPropagation()}>
        <div className="flex items-center gap-2 border-b py-4 px-6" >
          <Icon name={rule ? "pen-2" : "add-circle"} size={18} color="var(--accent)" />
          <span className="display text-lg tracking-tight text-ink">{rule ? "Edit rule" : "New rule"}</span>
          <span className="flex-1" />
          <span className="zs-meta">{scope.scope} · {scope.name}</span>
        </div>
        <div className="p-6 flex flex-col gap-4" >
          <div>
            <div className="zs-eyebrow font-semibold text-ink-mute mb-2">Rule</div>
            <textarea value={text} onChange={e => setText(e.target.value)} rows={3} placeholder="State the rule as an instruction sensei can follow…"
 className="text-sm text-ink w-full bg-paper-2 border border-paper-edge rounded p-3" style={{ resize: "none", fontFamily: "var(--font-ui)", outline: "none", lineHeight: 1.5 }} />
          </div>
          <div>
            <div className="zs-eyebrow font-semibold text-ink-mute mb-2">Family</div>
            <div className="flex flex-wrap gap-2">
              {RULE_FAMILIES.map(f => {
                const on = fam === f.k;
                return (
                  <button key={f.k} onClick={() => setFam(f.k)} className={"inline-flex items-center gap-2 rounded-full text-sm py-2 px-3 " + (on ? "bg-accent-soft" : "bg-paper-2 border border-paper-edge")}
                    style={{ border: on ? "1px solid var(--accent)" : undefined, color: on ? "var(--accent)" : "var(--ink-soft)" }}>
                    <span className="kanji" style={{ fontSize: 14 }}>{f.k}</span>{f.label}
                  </button>
                );
              })}
            </div>
          </div>
          <button onClick={() => setHard(h => !h)} className="flex items-center gap-3 text-left rounded-lg py-3 px-4" style={{ background: hard ? "var(--accent-soft)" : "var(--paper-2)", border: "1px solid " + (hard ? "var(--accent-edge)" : "var(--paper-edge)") }}>
            <span style={{ fontSize: 16, color: hard ? "var(--accent)" : "var(--ink-faint)" }}>★</span>
            <div className="flex-1">
              <div className="text-sm font-medium text-ink">Non-negotiable</div>
              <div className="zs-meta">Locks the rule — no narrower scope can relax it.</div>
            </div>
            <span className="rounded-full relative shrink-0" style={{ width: 34, height: 20, background: hard ? "var(--accent)" : "var(--paper-mute)", transition: "background .15s" }}>
              <span className="rounded-full absolute bg-paper" style={{ top: 2, left: hard ? 16 : 2, width: 16, height: 16, transition: "left .15s" }} />
            </span>
          </button>
        </div>
        <div className="flex gap-2 border-t py-4 px-6 bg-paper-2" >
          <span className="flex-1" />
          <Button size="sm" variant="ghost" onClick={onClose}>Cancel</Button>
          <Button size="sm" icon="check-circle" onClick={onClose}>{rule ? "Save rule" : "Add rule"}</Button>
        </div>
      </div>
    </div>
  );
}

// (6) CONSTITUTION — the dōjō authors its OWN rules, by section
// (company-wide · teams · stacks, packs per stack). Not the resolution ladder.
function ScrOrgLadder({ org, mobile }) {
  const sections = useAsync(() => ZS_API.getOrgConstitution(org.slug), "orgrules:" + org.slug, []).data || [];
  const [active, setActive] = React.useState(null);
  const [inc, setInc] = React.useState({});
  const [editing, setEditing] = React.useState(null);
  const [showEx, setShowEx] = React.useState(false);
  const sec = sections.find(s => s.id === active) || sections[0];
  // Every hook above, guard below — an early return placed among the
  // hooks changes their count between renders and React throws.
  if (!sec) return <Body><EmptyState kanji="静" title="Still listening.">This dōjō hasn’t authored a constitution yet.</EmptyState></Body>;
  const isIn = (i) => inc[active + i] !== false;
  const excluded = (sec.rules || []).filter((r, i) => !isIn(i)).length;
  const groups = ["Company", "Teams", "Stacks"];
  const W = mobile ? BodyM : Body;
  return (
    <div className="relative" >
      <W>
        <SectionHead eyebrow={org.name + " · governance"} title="Constitution"
          right={<Button size="sm" icon="add-circle" onClick={() => setEditing({})}>New rule</Button>} />
        <Banner kanji="掟" tone="neutral" title="The dōjō authors its rules by scope — company-wide, per team, per stack.">
          Stacks also adopt rule packs. These are the dōjō's own rules; how they combine with your personal and a project's rules is resolved — and shown — when you open a project.
        </Banner>
        <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "252px 1fr" }}>
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
                        className={"flex items-center gap-3 w-full text-left rounded-lg " + (active === s.id ? "bg-accent-soft" : "bg-paper-2 border border-paper-edge")}
                        style={{ padding: "var(--space-3)", border: active === s.id ? "1px solid var(--accent)" : undefined }}>
                        <KanjiMark char={s.kanji} size="lg" tone={active === s.id ? "var(--accent)" : "var(--ink-soft)"} w={24} />
                        <div className="flex-1 min-w-0" >
                          <div className="text-sm font-medium text-ink whitespace-nowrap overflow-hidden text-ellipsis" >{s.scope}</div>
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
              <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
                <div className="flex items-center gap-2 border-b py-3 px-4" >
                  <Icon name="box" size={16} color="var(--accent)" />
                  <span className="zs-eyebrow font-semibold text-ink">Rule packs for this stack</span>
                  <span className="flex-1" />
                  <Button size="sm" variant="ghost" icon="add-circle">Adopt pack</Button>
                </div>
                {sec.packs.length ? sec.packs.map((p, i) => (
                  <div key={i} className="flex items-center gap-3 border-b py-3 px-4" >
                    <Icon name="box" size={17} color="var(--accent)" />
                    <span className="text-sm text-ink flex-1">{p}</span>
                    <Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</Chip>
                  </div>
                )) : <div className="zs-meta py-3 px-4" >No pack adopted — this stack runs on its own rules and the company baseline.</div>}
              </div>
            )}
            <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
              <div className="flex items-center gap-2 border-b py-3 px-4" >
                <KanjiMark char={sec.kanji} size="base" tone="var(--accent)" />
                <span className="zs-eyebrow font-semibold text-ink">{sec.scope} · rules</span>
                <span className="flex-1" />
                {excluded > 0 && (
                  <Button variant="link" size="sm" icon={showEx ? "eye-closed" : "eye"}
                    onClick={() => setShowEx(s => !s)}>{showEx ? "Hide" : "Show"} {excluded} excluded</Button>
                )}
                <Button size="sm" variant="ghost" icon="add-circle" onClick={() => setEditing({})}>Add</Button>
              </div>
              {(sec.rules || []).map((r, i) => ({ r, i })).filter(({ i }) => showEx || isIn(i)).map(({ r, i }) => (
                <RuleRow key={i} rule={r} showLevel={false} included={isIn(i)}
                  onToggle={() => setInc(s => ({ ...s, [active + i]: isIn(i) ? false : true }))}
                  onEdit={() => setEditing({ rule: r })} />
              ))}
              {!showEx && excluded > 0 && (
                <div className="zs-meta py-2 px-4 bg-paper" >{excluded} excluded rule{excluded > 1 ? "s" : ""} hidden · consolidated view</div>
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
    <div className="flex items-center gap-4 border-b py-3 px-4" >
      <window.Avatar name={m.name} size={30} />
      <div className="flex-1 min-w-0" >
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-ink">{m.name}</span>
          {m.you && <Chip tone="var(--ink-mute)">you</Chip>}
        </div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>git: {m.git} · {m.scopes}</div>
      </div>
      <span className="zs-meta">{m.active}</span>
      <RoleTag role={m.role} />
      <Icon name="alt-arrow-right" size={16} color="var(--ink-faint)" />
    </div>
  );
}
function ScrRoleSurfaces({ org, tab = "members", hideTabs, mobile }) {
  const W = mobile ? BodyM : Body;
  const members = useAsync(() => ZS_API.getMembers(), "members", []).data || [];
  const roles = useAsync(() => ZS_API.getRoles(), "roles", {}).data || {};
  const me = useAsync(() => ZS_API.getMe(), "me", {}).data || {};
  const auditTrail = useAsync(() => ZS_API.getThread("s-2891"), "audit", []).data || [];
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
      <SectionHead eyebrow={org.name + " · " + cur.eyebrow} title={cur.title}
        right={<Button size="sm" icon="add-circle">{t === "members" ? "Invite" : t === "policies" ? "New policy" : "Export"}</Button>} />
      {!hideTabs && <SubTabs tabs={tabs} active={t} onPick={setT} />}
      {t === "members" && (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {members.map((m, i) => <MemberRow key={i} m={m} />)}
        </div>
      )}
      {t === "policies" && (
        <div className="flex flex-col gap-3">
          <Banner kanji="規" tone="neutral" title="Roles are additive and derived from git.">developer → maintainer → lead → admin. A role only ever adds capability.</Banner>
          {Object.values(roles).map((r, i) => (
            <div key={i} className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4 flex items-center gap-4 " >
              <KanjiMark char={r.kanji} size="xl" tone="var(--accent)" w={30} />
              <div className="flex-1"><div className="text-sm font-medium text-ink">{r.label}</div><div className="zs-meta">{r.note}</div></div>
              <Button size="sm" variant="ghost" icon="tuning-2">Edit policy</Button>
            </div>
          ))}
        </div>
      )}
      {t === "audit" && (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {auditTrail.map((x, i) => (
            <div key={i} className="flex items-center gap-3 border-b py-3 px-4" >
              <Icon name="clipboard-list" size={17} color="var(--ink-mute)" />
              <span className="text-sm text-ink flex-1">{x.who === "sensei" ? "sensei" : me.name} · {x.text.slice(0, 60)}…</span>
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
const CONTRIBUTION_STATUS = {
  approved: { tone: "var(--success)", soft: "var(--success-soft)", edge: "var(--success-edge)", label: "approved", icon: "check-circle" },
  pending:  { tone: "var(--accent)",  soft: "var(--accent-soft)",  edge: "var(--accent-edge)",  label: "in triage", icon: "hourglass" },
  declined: { tone: "var(--danger)",  soft: "var(--danger-soft)",  edge: "var(--danger-edge)",  label: "declined", icon: "close-circle" },
};
function ContribRow({ c }) {
  const s = CONTRIBUTION_STATUS[c.status] || CONTRIBUTION_STATUS.pending;
  return (
    <div className="flex items-center gap-4 border-b py-3 px-4" >
      <KanjiMark char={c.kanji} size="lg" tone="var(--accent)" w={22} />
      <div className="flex-1 min-w-0" >
        <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{c.title}</div>
        <div className="flex items-center gap-2 flex-wrap" style={{ marginTop: 2 }}>
          <Chip icon={c.client ? "shield-check" : undefined} tone={c.client ? "var(--accent)" : "var(--ink-mute)"} soft={c.client ? "var(--accent-soft)" : "var(--paper-mute)"} edge={c.client ? "var(--accent-edge)" : "transparent"}>{c.dest}</Chip>
          <span className="mono text-xs text-ink-faint">{c.scope} · {c.note}</span>
        </div>
      </div>
      <Chip icon={s.icon} tone={s.tone} soft={s.soft} edge={s.edge}>{s.label}</Chip>
      <span className="mono text-xs text-ink-faint text-right" style={{ width: 28 }}>{c.when}</span>
    </div>
  );
}
function DownstreamRow({ it }) {
  return (
    <div className="flex items-center gap-4 border-b py-3 px-4" >
      <KanjiMark char={it.kanji} size="lg" tone="var(--accent)" w={22} />
      <div className="flex-1 min-w-0" >
        <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{it.title}</div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{it.from} · {it.scope} · {it.when} ago</div>
      </div>
      {it.adopted
        ? <Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">adopted</Chip>
        : <Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">new</Chip>}
      {!it.adopted && <Button size="sm" variant="ghost" icon="pin">Pin</Button>}
    </div>
  );
}
function ScrContributions({ mobile }) {
  const W = mobile ? BodyM : Body;
  const c = useAsync(() => ZS_API.getContributions(), "contributions", {}).data || {};
  const { mine = [], downstream = [], stat = {} } = c;
  return (
    <W>
      <SectionHead eyebrow="You · sharing" title="Contributions"
        right={!mobile && <div className="flex items-center gap-4">
          <StatBadge n={stat.approved} label="approved" tone="var(--success)" />
          <StatBadge n={stat.pending} label="in triage" tone="var(--accent)" />
          <StatBadge n={stat.helped} label="devs helped" sub="lifetime" />
        </div>} />
      <Banner kanji="共" tone="neutral" title="You propose; a maintainer decides — nothing publishes without their named approval.">
        You share from a session's ready-to-share lane; it lands in the bound dōjō's triage queue. Client work is anonymized before it leaves — the lesson travels, the client never does.
      </Banner>
      <ListSection icon="upload-square" title="What you've shared" count={mine.length}>
        {mine.map((c, i) => <ContribRow key={i} c={c} />)}
      </ListSection>
      <ListSection icon="download-square" iconColor="var(--success)" title="Approved for you" count={downstream.length}
        right={<span className="zs-meta">from your dōjōs · never silently merged</span>}>
        {downstream.map((it, i) => <DownstreamRow key={i} it={it} />)}
      </ListSection>
    </W>
  );
}

// SCOPES (org admin) — who owns/triages each scope's queue.
function ScrScopes({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const rows = useAsync(() => ZS_API.getScopeOwners(org.slug), "scopes:" + org.slug, []).data || [];
  const groups = ["Company", "Teams", "Stacks"];
  const unowned = rows.filter(r => !r.owner).length;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · admin"} title="Scopes & policies"
        right={<Button size="sm" icon="add-circle">New scope</Button>} />
      <Banner kanji="規" tone={unowned ? "warning" : "neutral"} title="Every scope has a named owner who triages its queue.">
        {unowned ? unowned + " scope has no owner — its queue routes to a fallback maintainer so nothing stalls. Assign an owner to give it an SLA." : "Anything unowned routes to a fallback maintainer so nothing stalls."}
      </Banner>
      {groups.map(g => {
        const items = rows.filter(r => r.group === g); if (!items.length) return null;
        return (
          <ListSection key={g} icon={g === "Company" ? "buildings-2" : g === "Teams" ? "users-group-rounded" : "layers-minimalistic"} title={g} count={items.length}>
            {items.map((r, i) => (
              <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
                <div className="flex-1 min-w-0" >
                  <div className="text-sm font-medium text-ink">{r.scope}</div>
                  <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.queue} in queue · SLA {r.sla}</div>
                </div>
                {r.owner ? (
                  <div className="flex items-center gap-2">
                    <window.Avatar name={r.owner} size={24} />
                    <span className="text-sm text-ink">{r.owner}</span>
                    <RoleTag role={r.role} muted />
                  </div>
                ) : <Chip icon="danger-triangle" tone="var(--warning)" soft="var(--warning-soft)" edge="oklch(0.72 0.12 75 / 0.30)">unowned · fallback</Chip>}
                <Button size="sm" variant="ghost" icon="tuning-2">{r.owner ? "Reassign" : "Assign"}</Button>
              </div>
            ))}
          </ListSection>
        );
      })}
    </W>
  );
}

// BILLING (org admin) — plan & seats. Free where public/personal; paid where shared.
function ScrBilling({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const b = useAsync(() => ZS_API.getBilling(), "billing", null).data;
  if (!b) return <W><EmptyState kanji="静" title="Still listening.">Billing hasn’t answered yet.</EmptyState></W>;
  const monthly = b.seatsActive * b.perSeat;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · plan & billing"} title="Plan & billing"
        right={<Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{b.plan}</Chip>} />
      <Banner kanji="円" tone="neutral" title="Free where public or personal; paid where private and shared.">
        Seats bill per active contributor. The desktop app, the global collective, bring-your-own-key inference, and read-only membership are always free — you pay to coordinate a group, never for tokens.
      </Banner>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" style={{ border: "1px solid var(--accent)" }}>
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">Current plan</div>
          <div className="display text-2xl font-light" style={{ letterSpacing: "-0.01em" }}>{b.plan}</div>
          <div className="zs-body-sm" style={{ marginTop: 2 }}>Renews {b.renews} · <span className="mono">${b.perSeat}</span>/contributor/mo</div>
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">Billable seats</div>
          <StatBadge n={b.seatsActive} label="active contributors" sub={b.seatsReadonly + " read-only free"} />
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="zs-eyebrow font-semibold text-ink-mute mb-1">This month</div>
          <StatBadge n={"$" + monthly} label={b.seatsActive + " × $" + b.perSeat} sub="updated live" tone="var(--accent)" />
        </div>
      </div>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr" : "repeat(3, 1fr)" }}>
        {b.tiers.map(t => (
          <div key={t.id} className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4 flex flex-col gap-2" style={{
 background: t.dark ? "var(--ink)" : undefined, border: t.current ? "1px solid var(--accent)" : undefined }}>
            <div className="flex items-center gap-2">
              <KanjiMark char={t.kanji} size="lg" tone={t.dark ? "var(--accent)" : t.current ? "var(--accent)" : "var(--ink-mute)"} />
              <span className="text-sm font-semibold" style={{ color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.name}</span>
              {t.current && <Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)" style={{ marginLeft: "auto" }}>current</Chip>}
            </div>
            <div>
              <span className="display text-xl font-light" style={{ color: t.dark ? "var(--paper)" : "var(--ink)" }}>{t.price}</span>
              <span className="mono text-xs" style={{ color: t.dark ? "var(--on-primary-mute)" : "var(--ink-faint)", marginLeft: 4 }}>{t.sub}</span>
            </div>
            <div className="flex flex-col gap-1">
              {t.lines.map((l, i) => (
                <div key={i} className="flex gap-2 text-xs" style={{ lineHeight: 1.45, color: t.dark ? "var(--on-primary-soft)" : "var(--ink-soft)" }}>
                  <span className="text-accent shrink-0" >·</span>{l}
                </div>
              ))}
            </div>
            {!t.current && <Button variant="ghost" size="sm" style={{ marginTop: "auto" }}>{t.id === "free" ? "Downgrade" : "Contact sales"}</Button>}
          </div>
        ))}
      </div>
      <ListSection icon="chat-round-line" title="Relay — free for individuals, paid where it's shared">
        {b.relayRows.map((r, i) => (
          <div key={i} className="flex items-center gap-3 border-b py-3 px-4" >
            <span className="text-sm text-ink-soft flex-1">{r.label}</span>
            <Chip tone={r.free ? "var(--success)" : "var(--accent)"} soft={r.free ? "var(--success-soft)" : "var(--accent-soft)"} edge={r.free ? "var(--success-edge)" : "var(--accent-edge)"}>{r.free ? "free · individuals" : "paid · team"}</Chip>
          </div>
        ))}
      </ListSection>
      <ListSection icon="bill-list" title="Invoices" count={b.invoices.length}>
        {b.invoices.map((iv, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <span className="mono text-xs text-ink-soft flex-1">{iv.d}</span>
            <span className="mono text-sm text-ink">{iv.amt}</span>
            <Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">{iv.s}</Chip>
          </div>
        ))}
      </ListSection>
    </W>
  );
}

/* ═══ ORG ROLE CONSOLES (Govern · Clients · Admin) ════════ */

const IMPACT = {
  high:   { tone: "var(--accent)", soft: "var(--accent-soft)", edge: "var(--accent-edge)" },
  safety: { tone: "var(--danger)", soft: "var(--danger-soft)", edge: "var(--danger-edge)" },
  normal: { tone: "var(--ink-mute)", soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
  low:    { tone: "var(--ink-mute)", soft: "var(--paper-mute)", edge: "var(--paper-edge)" },
};

// 1 · TRIAGE (maintainer) — ranked candidate queue + candidate detail.
function ScrTriage({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const groups = useAsync(() => ZS_API.getConsole("triage"), "triage", []).data || [];
  const all = groups.flatMap(g => g.items);
  const [sel, setSel] = React.useState(null);
  const cur = all.find(c => c.id === sel) || all[0];
  const d = useAsync(() => ZS_API.getConsole("candidateDetail"), "candidate", null).data;
  const total = all.length;
  if (!cur || !d) return <W><EmptyState kanji="静" title="Still listening.">The triage queue hasn’t answered yet.</EmptyState></W>;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · govern"} title="Triage" count={total}
        right={<Button size="sm" variant="ghost" icon="tuning-2">My scopes</Button>} />
      <Banner kanji="門" tone="neutral" title="Candidate learnings waiting on a maintainer, grouped by scope and ranked by confidence.">
        You decide what becomes shared knowledge. High-impact and safety candidates need a second approval before they publish.
      </Banner>
      <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "1.1fr 1fr" }}>
        <div className="flex flex-col gap-4">
          {groups.map(g => (
            <ListSection key={g.scope} icon="layers-minimalistic" title={g.scope} count={g.items.length}>
              {g.items.map(c => {
                const im = IMPACT[c.impact] || IMPACT.normal;
                const on = c.id === sel;
                return (
                  <button key={c.id} onClick={() => setSel(c.id)} className={"flex items-center gap-3 w-full text-left border-b " + (on ? "bg-accent-soft" : "")} style={{ padding: "var(--space-3) var(--space-4)", background: on ? undefined : "transparent" }}>
                    <KanjiMark char={c.kanji} size="base" tone="var(--accent)" w={20} />
                    <div className="flex-1 min-w-0" >
                      <div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{c.title}</div>
                      <div className="flex items-center gap-2 flex-wrap" style={{ marginTop: 3 }}>
                        <span className="mono text-xs text-ink-faint">{c.origin}</span>
                        {c.conflicts > 0 && <Chip icon="danger-triangle" tone="var(--warning)" soft="var(--warning-soft)" edge="oklch(0.72 0.12 75 / 0.30)">{c.conflicts} conflict</Chip>}
                        {c.dups > 0 && <Chip tone="var(--ink-mute)">{c.dups} dup</Chip>}
                        {(c.impact === "high" || c.impact === "safety") && <Chip tone={im.tone} soft={im.soft} edge={im.edge}>{c.impact}</Chip>}
                      </div>
                    </div>
                    <ConfidenceBar v={c.conf} w={56} />
                  </button>
                );
              })}
            </ListSection>
          ))}
        </div>
        {!mobile && (
          <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden sticky" style={{ top: 0 }}>
            <div className="flex items-center gap-3 border-b p-4" >
              <KanjiMark char={cur.kanji} size="xl" tone="var(--accent)" />
              <div className="flex-1"><div className="zs-eyebrow font-semibold text-ink-mute">candidate</div><div className="text-sm font-medium text-ink">{cur.title}</div></div>
              <Enso progress={cur.conf} size={54} label={Math.round(cur.conf * 100) + ""} />
            </div>
            <div className="p-4 flex flex-col gap-3" >
              <div><div className="zs-eyebrow text-ink-mute mb-1">Learning</div><div className="text-sm text-ink">{d.learning}</div></div>
              <div><div className="zs-eyebrow text-ink-mute mb-1">Cause</div><div className="zs-body-sm">{d.cause}</div></div>
              <div><div className="zs-eyebrow text-ink-mute mb-1">Evidence</div>
                <div className="flex flex-col gap-1">{d.evidence.map((e, i) => <div key={i} className="mono text-xs text-ink-soft">· {e}</div>)}</div></div>
              {cur.conflicts > 0 && (
                <div className="rounded bg-warning-soft p-3" style={{ border: "1px solid oklch(0.72 0.12 75 / 0.30)" }}>
                  <div className="zs-eyebrow mb-1 text-warning" >Conflict — the ladder settles it</div>
                  <div className="text-xs text-ink-soft" style={{ textDecoration: "line-through" }}>{d.conflict.loser}</div>
                  <div className="text-sm text-ink font-medium" style={{ marginTop: 2 }}>{d.conflict.winner}</div>
                </div>
              )}
              <div><div className="zs-eyebrow text-ink-mute mb-1">Distribution scope</div>
                <div className="flex flex-wrap gap-2">{d.scopes.map((s, i) => <Chip key={i} tone={i === 1 ? "var(--accent)" : "var(--ink-mute)"} soft={i === 1 ? "var(--accent-soft)" : "var(--paper-mute)"} edge={i === 1 ? "var(--accent-edge)" : "var(--paper-edge)"}>{s}</Chip>)}</div></div>
            </div>
            <div className="flex flex-wrap gap-2 border-t py-3 px-4 bg-paper" >
              <Button size="sm" icon="check-circle">Approve</Button>
              <Button size="sm" variant="ghost" icon="pen-2">Revise</Button>
              <span className="flex-1" />
              <Button size="sm" variant="ghost" icon="close-circle">Decline</Button>
            </div>
            {(cur.impact === "high" || cur.impact === "safety") && (
              <div className="border-t zs-meta py-2 px-4 bg-paper" >Approving sends this to a second maintainer before it publishes.</div>
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
  const items = useAsync(() => ZS_API.getConsole("approvals"), "approvals", []).data || [];
  return (
    <W>
      <SectionHead eyebrow={org.name + " · govern"} title="Approvals" count={items.length} />
      <Banner kanji="承" tone="neutral" title="A second maintainer signs off high-impact and safety-relevant candidates.">
        One approval proposes; a second publishes. Nothing safety-relevant reaches a machine on a single signature.
      </Banner>
      {items.length ? (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
          {items.map(a => {
            const im = IMPACT[a.impact] || IMPACT.high;
            return (
              <div key={a.id} className="flex items-center gap-4 border-b py-3 px-4" >
                <KanjiMark char={a.kanji} size="lg" tone="var(--accent)" w={22} />
                <div className="flex-1 min-w-0" >
                  <div className="text-sm text-ink">{a.title}</div>
                  <div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{a.scope} · first approval: {a.first} · {a.when}</div>
                </div>
                <Chip tone={im.tone} soft={im.soft} edge={im.edge}>{a.impact}</Chip>
                <Button size="sm" variant="ghost" icon="eye">Review</Button>
                <Button size="sm" icon="check-circle">Approve</Button>
              </div>
            );
          })}
        </div>
      ) : <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden"><EmptyState kanji="静" title="Nothing awaiting a second look.">High-impact candidates land here for a second signature. The queue is clear.</EmptyState></div>}
    </W>
  );
}

// 3 · KNOWLEDGE (+ catalog) (maintainer) — published library + prune + catalog.
const EXTENSION_ICON = { agent: "user-hands", command: "command", skill: "star" };
function ScrKnowledge({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const k = useAsync(() => ZS_API.getConsole("knowledge"), "knowledge", null).data;
  if (!k) return <W><EmptyState kanji="静" title="Still listening.">The library hasn’t answered yet.</EmptyState></W>;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · govern"} title="Knowledge"
        right={<div className="zs-input text-sm w-auto" style={{ height: 32, padding: "0 10px" }}><Icon name="alt-arrow-down" size={14} color="var(--ink-mute)" /><span className="text-ink-soft whitespace-nowrap" >{k.prunePolicy}</span></div>} />
      <Banner kanji="蔵" tone="neutral" title="The published library the dōjō has adopted — and what's due to be pruned.">
        Adopted knowledge distributes to every machine in scope. Unused teachings age out by the prune policy so the shared mind stays lean.
      </Banner>
      <ListSection icon="check-circle" iconColor="var(--success)" title="Active" count={k.active.length}>
        {k.active.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <KanjiMark char={r.kanji} size="lg" tone="var(--accent)" w={22} />
            <div className="flex-1 min-w-0" ><div className="text-sm text-ink">{r.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.scope} · {r.adopted} · {r.age}</div></div>
            <Button size="sm" variant="ghost" icon="pen-2">Edit</Button>
          </div>
        ))}
      </ListSection>
      <ListSection icon="trash-bin-minimalistic" title="Pending prune" count={k.pending.length}>
        {k.pending.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <KanjiMark char={r.kanji} size="lg" tone="var(--ink-mute)" w={22} />
            <div className="flex-1 min-w-0" ><div className="text-sm text-ink-soft">{r.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.scope} · {r.age}</div></div>
            <Button size="sm" variant="ghost" icon="restart">Keep</Button>
          </div>
        ))}
      </ListSection>
      <ListSection icon="widget-4" title="Catalog · skills, agents & commands" count={k.catalog.length}>
        {k.catalog.map((c, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <Icon name={EXTENSION_ICON[c.kind] || "widget-4"} size={18} color="var(--accent)" />
            <div className="flex-1 min-w-0" ><div className="text-sm text-ink">{c.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{c.scope}</div></div>
            <Chip mono tone="var(--ink-mute)">{c.kind}</Chip>
          </div>
        ))}
      </ListSection>
    </W>
  );
}

// 4 · ENGAGEMENTS (lead) — client register + confidentiality model.
function ScrEngagements({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const c = useAsync(() => ZS_API.getConsole("confidentiality"), "confid", null).data;
  const engagements = useAsync(() => ZS_API.getConsole("engagements"), "engagements", []).data || [];
  if (!c) return <W><EmptyState kanji="静" title="Still listening.">The client register hasn’t answered yet.</EmptyState></W>;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · clients"} title="Engagements" count={engagements.length}
        right={<Button size="sm" icon="add-circle">New engagement</Button>} />
      <Banner kanji="盾" tone="accent" title="Share the lesson, never the source.">
        Findings from a client project are anonymized before they leave the machine — the pattern travels upstream; the client, repo and code never do.
      </Banner>
      <ListSection icon="case-round" title="Client engagements" count={engagements.length}>
        {engagements.map(e => (
          <div key={e.id} className="flex items-center gap-4 border-b py-3 px-4" >
            <KanjiMark char={e.kanji} size="lg" tone="var(--accent)" w={22} />
            <div className="flex-1 min-w-0" ><div className="text-sm font-medium text-ink">{e.client}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{e.projects} · since {e.since}</div></div>
            <div className="text-right"><div className="mono text-sm text-ink">{e.lessons}</div><div className="zs-meta">lessons kept</div></div>
            <div className="text-right"><div className="mono text-sm text-ink-mute">{e.dropped}</div><div className="zs-meta">stripped</div></div>
            <Button size="sm" variant="ghost" icon="document">Audit</Button>
          </div>
        ))}
      </ListSection>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="zs-eyebrow font-semibold text-ink-mute mb-3">What crosses the boundary</div>
          <div className="flex flex-col gap-2">
            {c.kept.map((x, i) => <div key={i} className="flex items-center gap-2 text-sm text-ink"><Icon name="check-circle" size={15} color="var(--success)" />{x}</div>)}
            {c.dropped.map((x, i) => <div key={i} className="flex items-center gap-2 text-sm text-ink-soft"><Icon name="close-circle" size={15} color="var(--danger)" />{x}</div>)}
          </div>
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="zs-eyebrow font-semibold text-ink-mute mb-3">Anonymized before it leaves</div>
          <div className="mono text-xs bg-paper-mute border border-paper-edge rounded p-3 text-ink-soft" style={{ textDecoration: "line-through" }}>{c.example.raw}</div>
          <div className="flex items-center justify-center text-ink-faint p-1" >↓</div>
          <div className="mono text-xs bg-success-soft rounded p-3 text-ink" style={{ border: "1px solid var(--success-edge)" }}>{c.example.stripped}</div>
        </div>
      </div>
    </W>
  );
}

// 5 · INCIDENTS (lead) — confidentiality containment.
const SEVERITY = { high: { tone: "var(--danger)", soft: "var(--danger-soft)", edge: "var(--danger-edge)" }, medium: { tone: "var(--warning)", soft: "var(--warning-soft)", edge: "oklch(0.72 0.12 75 / 0.30)" } };
const INCIDENT_STATE = { contained: "var(--warning)", resolved: "var(--success)", open: "var(--danger)" };
function ScrIncidents({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const incidents = useAsync(() => ZS_API.getConsole("incidents"), "incidents", []).data || [];
  return (
    <W>
      <SectionHead eyebrow={org.name + " · clients"} title="Incidents" count={incidents.length}
        right={<Button size="sm" icon="add-circle">Report</Button>} />
      <Banner kanji="盾" tone="warning" title="Contain a near-leak fast.">
        Leak-guard holds anything that looks like client source before it leaves. Log the containment, set retention, and control client read-access here.
      </Banner>
      <ListSection icon="shield-warning" title="Confidentiality incidents" count={incidents.length}>
        {incidents.map(it => {
          const sv = SEVERITY[it.severity] || SEVERITY.medium;
          return (
            <div key={it.id} className="flex items-center gap-4 border-b py-3 px-4" >
              <KanjiMark char={it.kanji} size="lg" tone={sv.tone} w={22} />
              <div className="flex-1 min-w-0" ><div className="text-sm text-ink">{it.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{it.client} · {it.when}</div></div>
              <Chip tone={sv.tone} soft={sv.soft} edge={sv.edge}>{it.severity}</Chip>
              <span className="inline-flex items-center gap-1 text-xs" style={{ color: INCIDENT_STATE[it.state] }}><span className="rounded-full" style={{ width: 6, height: 6, background: INCIDENT_STATE[it.state] }} />{it.state}</span>
              <Button size="sm" variant="ghost" icon="alt-arrow-right">Open</Button>
            </div>
          );
        })}
      </ListSection>
      <div className="flex flex-wrap gap-2">
        <Chip icon="lock-keyhole" tone="var(--ink-mute)">Retention · 1 year</Chip>
        <Chip icon="eye-closed" tone="var(--ink-mute)">Client read-access · off</Chip>
      </div>
    </W>
  );
}

// 6 · CLIENT AUDIT (lead) — immutable confidentiality ledger.
function ScrClientAudit({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const ledger = useAsync(() => ZS_API.getConsole("clientAudit"), "clientAudit", []).data || [];
  return (
    <W>
      <SectionHead eyebrow={org.name + " · clients"} title="Client audit trail"
        right={<div className="flex gap-2"><Button size="sm" variant="ghost" icon="tuning-2">Filter</Button><Button size="sm" variant="ghost" icon="download-minimalistic">Export</Button></div>} />
      <Banner kanji="録" tone="neutral" title="An immutable ledger of exactly what left and what was stripped.">
        Distinct from the admin action-audit — this proves confidentiality held for each client, entry by entry. Append-only, exportable as CSV or JSON.
      </Banner>
      <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden" >
        {ledger.map((r, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <KanjiMark char={r.kanji} size="base" tone={r.ok ? "var(--ink-mute)" : "var(--danger)"} w={20} />
            <div className="flex-1 min-w-0" ><div className="text-sm text-ink">{r.event}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{r.detail}</div></div>
            <Chip tone="var(--ink-mute)">{r.client}</Chip>
            {r.ok ? <Icon name="check-circle" size={16} color="var(--success)" /> : <Icon name="shield-warning" size={16} color="var(--danger)" />}
            <span className="mono text-xs text-ink-faint text-right" style={{ width: 64 }}>{r.t}</span>
          </div>
        ))}
      </div>
      <div className="flex flex-wrap gap-2">
        <Chip icon="lock-keyhole" tone="var(--ink-mute)">Retention · 7 years</Chip>
        <Chip icon="eye" tone="var(--ink-mute)">Client read-access · Globex on</Chip>
      </div>
    </W>
  );
}

// 7 · IDENTITY & SSO (admin).
function ScrIdentity({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const id = useAsync(() => ZS_API.getConsole("identity"), "identity", null).data;
  if (!id) return <W><EmptyState kanji="静" title="Still listening.">Identity settings haven’t answered yet.</EmptyState></W>;
  return (
    <W>
      <SectionHead eyebrow={org.name + " · admin"} title="Identity & SSO" />
      <Banner kanji="鍵" tone="neutral" title="Connect org identity — SSO, provisioning, and how git maps to members.">
        Roles derive from these mappings; SSO and SCIM keep membership in step with your directory automatically.
      </Banner>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="flex items-center gap-2 mb-2"><Icon name="key" size={17} color="var(--accent)" /><span className="zs-eyebrow font-semibold text-ink-mute">Identity provider</span></div>
          <div className="flex items-center gap-2"><span className="text-lg font-medium text-ink">{id.idp.name}</span><Chip mono tone="var(--ink-mute)">{id.idp.protocol}</Chip><Chip icon="check-circle" tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">{id.idp.status}</Chip></div>
          <div className="mono text-xs text-ink-faint" style={{ marginTop: 4 }}>{id.idp.domain}</div>
          <div className="mt-3" ><Button size="sm" variant="ghost" icon="tuning-2">Configure</Button></div>
        </div>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="flex items-center gap-2 mb-2"><Icon name="refresh-circle" size={17} color="var(--accent)" /><span className="zs-eyebrow font-semibold text-ink-mute">SCIM provisioning</span></div>
          <div className="flex items-center gap-2"><span className="text-sm text-ink">{id.scim ? "Enabled — members sync from your directory" : "Disabled"}</span></div>
          <div className="mt-3" ><Button size="sm" variant="ghost" icon="tuning-2">Manage</Button></div>
        </div>
      </div>
      <ListSection icon="link-circle" title="Identity mappings" count={id.mappings.length}
        right={<Button size="sm" icon="add-circle">Add mapping</Button>}>
        {id.mappings.map((m, i) => (
          <div key={i} className="flex items-center gap-4 border-b py-3 px-4" >
            <div className="flex-1 min-w-0" ><div className="text-sm text-ink">{m.source}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>→ {m.to}</div></div>
            <span className="mono text-xs text-ink-mute">{m.count} members</span>
            <Button size="sm" variant="ghost" icon="pen-2">Edit</Button>
          </div>
        ))}
      </ListSection>
    </W>
  );
}

// 8 · HEALTH / MONITOR (admin).
function ScrHealth({ org, mobile }) {
  const W = mobile ? BodyM : Body;
  const h = useAsync(() => ZS_API.getConsole("health"), "health", null).data;
  if (!h) return <W><EmptyState kanji="静" title="Still listening.">The monitor hasn’t answered yet.</EmptyState></W>;
  const max = Math.max(...h.contribVsApprove.flatMap(x => [x.c, x.a]));
  return (
    <W>
      <SectionHead eyebrow={org.name + " · admin"} title="Health / Monitor" />
      <Banner kanji="観" tone="neutral" title="The shared mind's vital signs, fed by the audit trail.">
        Throughput, adoption and leak-guard at a glance. Anomalies surface as alerts before they become incidents.
      </Banner>
      <div className="grid gap-4" style={{ gridTemplateColumns: mobile ? "1fr 1fr" : "repeat(4, 1fr)" }}>
        {h.signals.map((s, i) => (
          <div key={i} className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
            <div className="flex items-center gap-2 mb-2"><KanjiMark char={s.kanji} size="base" tone={s.tone} /><span className="zs-meta">{s.label}</span></div>
            <StatBadge n={s.n} label="" sub={s.sub} tone={s.tone} />
          </div>
        ))}
      </div>
      <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "1.3fr 1fr" }}>
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4" >
          <div className="flex items-center gap-2 mb-3"><span className="zs-eyebrow font-semibold text-ink-mute">Contributions vs approvals</span><span className="flex-1" /><Chip tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">contrib</Chip><Chip tone="var(--success)" soft="var(--success-soft)" edge="var(--success-edge)">approved</Chip></div>
          <div className="flex items-end gap-4 py-0 px-2" style={{ height: 130 }}>
            {h.contribVsApprove.map((x, i) => (
              <div key={i} className="flex flex-col items-center gap-2 flex-1" >
                <div className="flex items-end" style={{ gap: 4, height: 100 }}>
                  <div className="bg-accent" style={{ width: 14, height: (x.c / max * 100) + "%", borderRadius: "3px 3px 0 0" }} />
                  <div className="bg-success" style={{ width: 14, height: (x.a / max * 100) + "%", borderRadius: "3px 3px 0 0" }} />
                </div>
                <span className="mono text-xs text-ink-faint">{x.wk}</span>
              </div>
            ))}
          </div>
        </div>
        <ListSection icon="bell" title="Leak-guard & anomalies" count={h.alerts.length}>
          {h.alerts.map((a, i) => (
            <div key={i} className="flex items-start gap-3 border-b py-3 px-4" >
              <KanjiMark char={a.kanji} size="base" tone={a.sev === "warning" ? "var(--warning)" : "var(--success)"} w={20} />
              <div className="flex-1 min-w-0" ><div className="text-sm text-ink" style={{ lineHeight: 1.3 }}>{a.title}</div><div className="mono text-xs text-ink-faint" style={{ marginTop: 1 }}>{a.detail} · {a.when}</div></div>
            </div>
          ))}
        </ListSection>
      </div>
    </W>
  );
}

/* ═══ SIGN-IN (adopted from the Dōjō console) ═════════════ */
function GhMark({ size = 18, color = "var(--paper)" }) {
  return (
    <svg className="shrink-0" width={size} height={size} viewBox="0 0 24 24" fill={color} aria-hidden="true" >
      <path d="M12 .5C5.7.5.5 5.7.5 12c0 5.1 3.3 9.4 7.9 10.9.6.1.8-.2.8-.5v-1.9c-3.2.7-3.9-1.5-3.9-1.5-.5-1.3-1.3-1.7-1.3-1.7-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.7 1.3 3.4 1 .1-.8.4-1.3.7-1.6-2.6-.3-5.3-1.3-5.3-5.8 0-1.3.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.4 11.4 0 0 1 6 0C17 5 18 5.3 18 5.3c.6 1.7.2 2.9.1 3.2.8.8 1.2 1.8 1.2 3.1 0 4.5-2.7 5.5-5.3 5.8.4.4.8 1.1.8 2.2v3.3c0 .3.2.6.8.5 4.6-1.5 7.9-5.8 7.9-10.9C23.5 5.7 18.3.5 12 .5z"/>
    </svg>
  );
}
function ScrSignIn({ label = "Sign in", mobile = false, onContinue }) {
  const [selfHost, setSelfHost] = React.useState(false);
  return (
    <div className="sensei w-full h-full flex bg-paper" data-screen-label={label} style={{ flexDirection: mobile ? "column" : "row", overflow: mobile ? "auto" : "hidden" }}>
      {/* left · welcome + what sensei is */}
      <div className="shrink-0 flex flex-col" style={{ width: mobile ? "100%" : "57%", padding: mobile ? "var(--space-6)" : "var(--space-12)", overflow: mobile ? "visible" : "auto",
 background: "linear-gradient(160deg, var(--accent-soft) 0%, var(--paper-soft) 60%)", borderRight: mobile ? "none" : "var(--hairline)", borderBottom: mobile ? "var(--hairline)" : "none" }}>
        <div className="flex items-center gap-2">
          <span className="kanji text-accent text-2xl" style={{ lineHeight: 1 }}>結</span>
          <span className="display text-xl" style={{ letterSpacing: "-0.01em" }}>Dōjō</span>
          <span className="mono text-ink-mute bg-paper text-xs border border-paper-edge rounded-full py-1 px-2" >dojo.sensei-hq.com</span>
        </div>
        <div className="mt-12" >
          <div className="zs-eyebrow font-semibold text-ink-mute mb-2" >先生 Sensei</div>
          <h1 className="display font-light m-0" style={{ fontSize: mobile ? "var(--text-2xl)" : "var(--text-3xl)", letterSpacing: "-0.02em", lineHeight: 1.08 }}>
            A quiet companion{mobile ? " " : <br/>}for your{mobile ? " " : <br/>}AI-assisted work.
          </h1>
          <p className="text-ink-soft text-sm" style={{ lineHeight: 1.6, margin: "var(--space-4) 0 0", maxWidth: 440 }}>
            Sensei watches your coding sessions on this machine and surfaces the patterns you're too close to see. Yours alone by default — a dōjō is optional, for when you want to share what you learn with a team.
          </p>
        </div>
        <div className="grid gap-3" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr 1fr", marginTop: mobile ? "var(--space-6)" : "var(--space-8)" }}>
          {[["観", "Watches your sessions", "locally · on this machine"],
            ["己", "Your rules & guardrails", "yours to set and edit"],
            ["盾", "Nothing leaves", "unless you choose to share"]].map(([k, t, s]) => (
            <div key={k} className="bg-paper border border-paper-edge rounded-lg p-4 items-center gap-3" style={{ display: mobile ? "flex" : "block" }}>
              <span className="kanji text-accent text-xl shrink-0" >{k}</span>
              <div>
                <div className="text-ink text-sm font-semibold" style={{ marginTop: mobile ? 0 : "var(--space-2)", lineHeight: 1.3 }}>{t}</div>
                <div className="text-ink-mute text-xs mt-1" >{s}</div>
              </div>
            </div>
          ))}
        </div>
        <div className="bg-paper flex items-center gap-3 mt-3 border border-paper-edge rounded-lg p-4" >
          <span className="kanji text-ink-mute text-xl" >空</span>
          <div className="flex-1 min-w-0" >
            <div className="text-ink text-sm" >Sign in and sensei picks up where you left off.</div>
            <div className="text-ink-mute text-xs mt-1" >No team, no setup required. <span className="italic" >Still listening.</span></div>
          </div>
        </div>
        <div style={{ flex: mobile ? "none" : 1 }} />
        <div className="text-ink-faint text-xs mt-6" style={{ lineHeight: 1.5 }}>
          Local-first · yours by default · join or create a dōjō later only to share with a team.
        </div>
      </div>
      {/* right · sign-in options */}
      <div className="flex items-center justify-center flex-1 min-w-0" style={{ padding: mobile ? "var(--space-6)" : "var(--space-8)" }}>
        <div className="max-w-full" style={{ width: 364 }}>
          <h2 className="display text-2xl font-normal m-0" style={{ letterSpacing: "-0.015em", lineHeight: 1.1 }}>Sign in to continue</h2>
          <p className="text-ink-mute text-sm" style={{ lineHeight: 1.55, margin: "var(--space-2) 0 var(--space-6)" }}>
            GitHub brings your organizations and roles automatically. No GitHub? Use a magic link.
          </p>
          <Button size="lg" full onClick={onContinue}>
            <GhMark size={18} color="var(--paper)" /> Continue with GitHub
          </Button>
          <div className="text-ink-faint text-xs text-center mt-2" >
            Derives your orgs &amp; roles from GitHub — and matches your repos.
          </div>
          <div className="flex items-center gap-3 my-4 mx-0" >
            <span className="flex-1 bg-paper-edge" style={{ height: 1 }} />
            <span className="mono text-ink-faint text-xs" style={{ letterSpacing: ".1em" }}>OR</span>
            <span className="flex-1 bg-paper-edge" style={{ height: 1 }} />
          </div>
          <label className="zs-eyebrow font-semibold text-ink-mute block mb-2" >Work email</label>
          <div className="zs-input mb-2" >
            <input className="border-0 bg-transparent w-full text-ink" placeholder="you@company.com" defaultValue="" style={{ outline: "none", font: "inherit" }} />
          </div>
          <Button variant="ghost" full icon="letter" onClick={onContinue}>Email me a magic link</Button>
          <div className="text-ink-faint text-xs text-center mt-2" >For organizations not on GitHub.</div>
          <div className="mt-6 pt-4" style={{ borderTop: "1px solid var(--paper-edge)" }}>
            {!selfHost ? (
              <button onClick={() => setSelfHost(true)} className="flex items-center justify-center gap-2 text-ink-soft w-full border-0 cursor-pointer text-sm" style={{ background: "none", font: "inherit" }}>
                <Icon name="server" size={15} color="var(--ink-mute)" />
                Connecting to a self-hosted dōjō? <span className="text-accent">Enter its URL →</span>
              </button>
            ) : (
              <div>
                <label className="zs-eyebrow font-semibold text-ink-mute block mb-2" >Self-hosted dōjō URL</label>
                <div className="flex gap-2">
                  <div className="zs-input flex-1" >
                    <input className="border-0 bg-transparent w-full text-ink" defaultValue="dojo.acme.internal" style={{ outline: "none", font: "inherit" }} />
                  </div>
                  <Button variant="ghost">Connect</Button>
                </div>
                <div className="text-ink-faint text-xs mt-2" style={{ lineHeight: 1.5 }}>
                  Same sign-in — your server authenticates you through GitHub (or an email magic link) on its own domain.
                </div>
              </div>
            )}
          </div>
          <div className="text-ink-faint text-xs text-center mt-6" style={{ lineHeight: 1.5 }}>
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
  const dojos = useAsync(() => ZS_API.getMyDojos(), "dojos", []).data || [];
  const byKind = (k) => dojos.filter(d => d.kind === k);
  const groups = [["employer", "Employer"], ["client", "Clients"], ["community", "Communities"]];
  return (
    <W>
      <SectionHead eyebrow="You · membership" title="My dōjōs" count={dojos.length}
        right={<Button size="sm" icon="add-circle">Create or join</Button>} />
      <Banner kanji="結" tone="neutral" title="A dōjō is how a team shares what it learns.">
        You belong to {dojos.length} — your role in each is derived from git and only ever adds capability. Working solo needs none of them.
      </Banner>
      {dojos.length ? groups.map(([k, lab]) => {
        const items = byKind(k); if (!items.length) return null;
        return (
          <ListSection key={k} icon={k === "employer" ? "buildings-2" : k === "client" ? "case-round" : "users-group-two-rounded"} title={lab} count={items.length}>
            {items.map(d => <DojoRow key={d.slug} dojo={d} onOpen={onOpen} mobile={mobile} />)}
          </ListSection>
        );
      }) : (
        <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden">
          <EmptyState kanji="空" title="No dōjōs yet.">You can work entirely solo — your rules and projects stay on your machine. Join a dōjō when a team wants to share what it learns.
            <div style={{ marginTop: 12 }}><Button size="sm" icon="add-circle">Create or join a dōjō</Button></div>
          </EmptyState>
        </div>
      )}
    </W>
  );
}

/* ═══ TEAMS — the collective view ══════════════════════════ */

// A metric with its trend. Numbers here are meaningful, not decorative: a
// first-try rate and corrections-per-session are the two a team acts on.
function TeamMetric({ label, value, unit, delta, deltaGood = "down", sub }) {
  if (value == null) return (
    <div className="flex flex-col" style={{ gap: 2 }}>
      <span className="zs-eyebrow text-ink-mute">{label}</span>
      <span className="mono text-lg text-ink-faint">—</span>
      <span className="zs-meta">nothing sent yet</span>
    </div>
  );
  const good = delta == null ? null : (deltaGood === "up" ? delta > 0 : delta < 0);
  return (
    <div className="flex flex-col" style={{ gap: 2 }}>
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

const INFLOW_KIND = {
  learning:   { label: "learning",   tone: "var(--accent)" },
  memory:     { label: "memory",     tone: "var(--ink-mute)" },
  instrument: { label: "instrument", tone: "var(--ink-mute)" },
  metric:     { label: "counts",     tone: "var(--ink-faint)" },
};
const INFLOW_STATE = {
  adopted:  { label: "adopted",   tone: "var(--success)",   soft: "var(--success-soft)" },
  triage:   { label: "in triage", tone: "var(--accent)",    soft: "var(--accent-soft)" },
  declined: { label: "declined",  tone: "var(--ink-mute)",  soft: "var(--paper-mute)" },
  counted:  { label: "counted",   tone: "var(--ink-faint)", soft: "var(--paper-mute)" },
};

function InflowRow({ it }) {
  const k = INFLOW_KIND[it.kind] || INFLOW_KIND.learning;
  const st = INFLOW_STATE[it.state] || INFLOW_STATE.triage;
  return (
    <div className="border-b grid gap-3 py-3 px-4 items-start" style={{ gridTemplateColumns: "22px minmax(0, 1fr) auto" }}>
      <KanjiMark char={it.kanji} size="lg" tone={k.tone} w={20} />
      <div className="min-w-0" >
        <div className="text-sm text-ink" style={{ lineHeight: 1.35 }}>{it.title}</div>
        <div className="mono text-xs text-ink-faint" style={{ marginTop: 2 }}>{k.label} · {it.by} · {it.team}{it.project !== "—" ? " · " + it.project : ""} · {it.when}</div>
        <div className="zs-meta" style={{ marginTop: 2 }}>{it.note}</div>
      </div>
      <Chip mono tone={st.tone} soft={st.soft}>{st.label}</Chip>
    </div>
  );
}

const PEOPLE_COLS = "minmax(0, 1.6fr) 96px 78px 96px 72px 52px";
const TEAM_COLS = "minmax(0, 1.4fr) 76px 92px 108px 84px 84px";

// One developer's contribution to the collective. The numbers are the same two
// the team is judged on, so they compare honestly.
function PersonRow({ p, onOpen, mobile }) {
  if (mobile) return (
    <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b grid gap-3 p-4 bg-transparent items-start"
 style={{ gridTemplateColumns: "26px minmax(0, 1fr)" }}>
      <window.Avatar name={p.name} size={26} />
      <span className="min-w-0 flex flex-col gap-2" >
        <span className="flex flex-col" style={{ gap: 2 }}>
          <span className="flex items-baseline gap-2">
            <span className="text-sm font-medium text-ink">{p.name}</span>
            {p.you && <span className="mono text-xs text-accent">you</span>}
            <span className="flex-1" />
            <span className="mono text-sm text-ink">{p.ftr}%</span>
          </span>
          <span className="mono text-xs text-ink-faint">{p.sessions} sessions · {p.sent} sent · {p.adopted} adopted · {p.last}</span>
        </span>
        <RoleTag role={p.role} muted />
      </span>
    </button>
  );
  return (
    <button onClick={() => onOpen && onOpen(p)} className="w-full text-left border-b grid gap-3 py-3 px-4 bg-transparent items-center"
 style={{ gridTemplateColumns: PEOPLE_COLS }}>
      <span className="flex items-center gap-3 min-w-0" >
        <window.Avatar name={p.name} size={24} />
        <span className="text-sm text-ink min-w-0 overflow-hidden text-ellipsis whitespace-nowrap" >{p.name}</span>
        {p.you && <span className="mono text-xs text-accent">you</span>}
      </span>
      <span><RoleTag role={p.role} muted /></span>
      <span className="mono text-xs text-ink-mute">{p.sessions} sess.</span>
      <span className="flex items-center gap-2">
        <span className="mono text-sm text-ink">{p.ftr}%</span>
        <PlanBar pct={p.ftr} tone={p.ftr >= 80 ? "var(--success)" : p.ftr >= 70 ? "var(--ink)" : "var(--warning)"} style={{ flex: 1 }} />
      </span>
      <span className="mono text-xs text-ink-mute">{p.sent}→{p.adopted}</span>
      <span className="mono text-xs text-ink-faint text-right" >{p.last}</span>
    </button>
  );
}

function PeopleHead() {
  return (
    <div className="border-b grid gap-3 py-2 px-4 bg-paper-mute" style={{ gridTemplateColumns: PEOPLE_COLS }}>
      {["developer", "role", "sessions", "first-try", "sent→kept", "last"].map((h, i) => (
        <span key={h} className="zs-eyebrow text-ink-mute" style={i === 5 ? { textAlign: "right" } : null}>{h}</span>
      ))}
    </div>
  );
}

function PersonPanel({ team, p, onBack, mobile }) {
  const W = mobile ? BodyM : Body;
  const kept = p.sent ? Math.round((p.adopted / p.sent) * 100) : 0;
  const feedRows = useAsync(() => ZS_API.getTeamInflow(), "inflow", []);
  const feed = (feedRows.data || []).filter(f => f.by === p.name);
  return (
    <W>
      <BackHead onBack={onBack}>{team.name}</BackHead>
      <div className="flex items-center gap-4">
        <window.Avatar name={p.name} size={44} />
        <div className="flex-1 min-w-0" >
          <div className="zs-eyebrow text-ink-mute">{team.name} · developer</div>
          <div className="zs-h3 text-ink" style={{ marginTop: 2 }}>{p.name}{p.you ? " · you" : ""}</div>
          <div className="mono text-xs text-ink-mute" style={{ marginTop: 3 }}>last session {p.last} · {p.sessions} this week</div>
        </div>
        <RoleTag role={p.role} />
      </div>
      <div className="zs-card grid gap-6" style={{ gridTemplateColumns: mobile ? "1fr 1fr" : "repeat(4, 1fr)" }}>
        <TeamMetric label="First-try rate" value={p.ftr} unit="%" sub="landed without a correction" />
        <TeamMetric label="Corrections" value={p.corrections} unit="/session" sub={"team average " + team.corrections} />
        <TeamMetric label="Sent up" value={p.sent} sub={p.adopted + " adopted · " + kept + "% kept"} />
        <TeamMetric label="Sessions" value={p.sessions} sub="this week" />
      </div>
      <Banner kanji="観" tone="neutral" title="These are counts, not transcripts.">
        sensei runs on {p.you ? "your own" : p.name.split(" ")[0] + "’s"} machine. The dōjō sees what was sent — a count, a learning, a memory — never the code, the prompts, or the session itself.
      </Banner>
      <ListSection icon="upload-square" iconColor="var(--accent)" title="What they sent up" count={feed.length}>
        {feed.length ? feed.map(f => <InflowRow key={f.id} it={f} />)
          : <EmptyState kanji="空" title="Nothing sent this week.">Their sensei is still watching. Sharing is theirs to offer, never taken.</EmptyState>}
      </ListSection>
    </W>
  );
}

function TeamDetail({ team, startPerson, onBack, mobile }) {
  const W = mobile ? BodyM : Body;
  const [person, setPerson] = React.useState(() => team.people.find(p => p.name === startPerson) || null);
  React.useEffect(() => { setPerson(team.people.find(p => p.name === startPerson) || null); }, [startPerson, team.id]);
  if (person) return <PersonPanel team={team} p={person} onBack={() => setPerson(null)} mobile={mobile} />;
  const inflowRows = useAsync(() => ZS_API.getTeamInflow(team.name), "inflow:" + team.name, []);
  const inflow = inflowRows.data || [];
  const m = team.memory;
  return (
    <W>
      <BackHead onBack={onBack}>Teams</BackHead>
      <div className="flex items-baseline gap-3">
        <KanjiMark char={team.kanji} size="lg" tone="var(--accent)" />
        <div className="flex-1 min-w-0" >
          <div className="zs-eyebrow text-ink-mute">Team</div>
          <div className="zs-h3 text-ink" style={{ marginTop: 2 }}>{team.name}</div>
          <div className="mono text-xs text-ink-mute" style={{ marginTop: 3 }}>{team.caption} · {team.members} developers{team.lead ? " · lead " + team.lead : " · no lead"}</div>
        </div>
        {team.spark && <Spark data={team.spark} w={100} h={28} />}
      </div>
      <div className="zs-card grid gap-6" style={{ gridTemplateColumns: mobile ? "1fr 1fr" : "repeat(4, 1fr)" }}>
        <TeamMetric label="First-try rate" value={team.ftr} unit="%" delta={team.ftrDelta} deltaGood="up" sub="landed without a correction" />
        <TeamMetric label="Corrections" value={team.corrections} unit="/session" delta={team.correctionsDelta} sub="lower is calmer" />
        <TeamMetric label="Sessions" value={team.sessions || null} sub="this week" />
        <TeamMetric label="Under the rungs" value={team.governed || null} unit="%" sub="sessions running the dōjō constitution" />
      </div>
      {team.people.length ? (
        <React.Fragment>
          <ListSection icon="users-group-rounded" title="By developer" count={team.people.length}
            right={<span className="zs-meta">first-try rate · sent → adopted</span>}>
            {!mobile && <PeopleHead />}
            {team.people.map(p => <PersonRow key={p.name} p={p} onOpen={setPerson} mobile={mobile} />)}
          </ListSection>
          <div className="grid gap-4 items-start" style={{ gridTemplateColumns: mobile ? "1fr" : "1fr 1fr" }}>
            <ListSection icon="book-2" title="Shared memory" right={<span className="zs-meta">what the team holds</span>}>
              <div className="bg-paper-soft border border-paper-edge rounded-lg overflow-hidden p-4 grid gap-6 " style={{ gridTemplateColumns: "1fr 1fr" }}>
                <TeamMetric label="Learnings" value={m.learnings} sub="patterns kept" />
                <TeamMetric label="Memories" value={m.memories} sub="decisions & gotchas" />
                <TeamMetric label="Team rules" value={m.rules} sub="authored at this scope" />
                <TeamMetric label="Instruments" value={m.instruments} sub="skills · agents · commands" />
              </div>
            </ListSection>
            <ListSection icon="upload-square" iconColor="var(--accent)" title="Inflow" count={inflow.length}
              right={<span className="zs-meta">{team.inflow.triage} in triage</span>}>
              {inflow.length ? inflow.map(f => <InflowRow key={f.id} it={f} />)
                : <EmptyState kanji="空" title="Nothing sent yet.">Sharing is each developer's to offer.</EmptyState>}
            </ListSection>
          </div>
        </React.Fragment>
      ) : (
        <EmptyState kanji="空" title="No sensei has reported here yet.">
          {team.members} developers sit on this team, but none has bound a project or sent anything up. Bind a project to the team scope and the counts start arriving.
        </EmptyState>
      )}
    </W>
  );
}

function TeamRow({ t, onOpen, mobile }) {
  const quiet = t.ftr == null;
  if (mobile) return (
    <button onClick={() => onOpen(t)} className="w-full text-left border-b grid gap-3 p-4 bg-transparent items-start"
 style={{ gridTemplateColumns: "22px minmax(0, 1fr)" }}>
      <KanjiMark char={t.kanji} size="lg" tone={quiet ? "var(--ink-faint)" : "var(--accent)"} w={20} />
      <span className="min-w-0 flex flex-col gap-2" >
        <span className="flex flex-col" style={{ gap: 2 }}>
          <span className="flex items-baseline gap-2">
            <span className="text-base font-medium text-ink">{t.name}</span>
            <span className="flex-1" />
            <span className="mono text-sm text-ink">{quiet ? "—" : t.ftr + "%"}</span>
          </span>
          <span className="mono text-xs text-ink-faint">{t.caption}</span>
        </span>
        <span className="flex items-center gap-2 flex-wrap" >
          <span className="mono text-xs text-ink-mute">{t.members} devs · {t.sessions} sessions</span>
          {t.inflow.triage > 0 && <Chip mono tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{t.inflow.triage} in triage</Chip>}
        </span>
      </span>
    </button>
  );
  return (
    <button onClick={() => onOpen(t)} className="w-full text-left border-b grid gap-3 py-3 px-4 bg-transparent items-center"
 style={{ gridTemplateColumns: TEAM_COLS }}>
      <span className="flex items-center gap-3 min-w-0" >
        <KanjiMark char={t.kanji} size="base" tone={quiet ? "var(--ink-faint)" : "var(--accent)"} w={18} />
        <span className="min-w-0" >
          <span className="text-sm font-medium text-ink block" >{t.name}</span>
          <span className="mono text-xs text-ink-faint block overflow-hidden text-ellipsis whitespace-nowrap" >{t.caption}</span>
        </span>
      </span>
      <span className="mono text-xs text-ink-mute">{t.members} devs</span>
      <span className="mono text-xs text-ink-mute">{t.sessions ? t.sessions + " sess." : "—"}</span>
      <span className="flex items-center gap-2">
        {quiet ? <span className="mono text-sm text-ink-faint">—</span> : <React.Fragment>
          <span className="mono text-sm text-ink">{t.ftr}%</span>
          <PlanBar pct={t.ftr} tone={t.ftr >= 80 ? "var(--success)" : "var(--ink)"} style={{ flex: 1 }} />
        </React.Fragment>}
      </span>
      <span className="mono text-xs text-ink-mute">{t.inflow.sent}→{t.inflow.adopted}</span>
      <span className="flex items-center justify-end">
        {t.inflow.triage > 0
          ? <Chip mono tone="var(--accent)" soft="var(--accent-soft)" edge="var(--accent-edge)">{t.inflow.triage} triage</Chip>
          : <span className="mono text-xs text-ink-faint">clear</span>}
      </span>
    </button>
  );
}

// TEAMS — collective metrics per team, a drill-down to the developer who sent
// each thing, and the inflow from every local sensei that reports here.
function ScrTeams({ org, startTeam, startTeamTab, startPerson, mobile }) {
  const W = mobile ? BodyM : Body;
  const teamList = useAsync(() => ZS_API.getTeams(org.slug), "teams:" + org.slug, []);
  const teams = teamList.data || [];
  const [tab, setTab] = React.useState(startTeamTab || "teams");
  const [openId, setOpenId] = React.useState(startTeam || null);
  const [openPerson, setOpenPerson] = React.useState(startPerson || null);
  const sharingRows = useAsync(() => ZS_API.getSharingSettings(), "sharing", []);
  const shareDefs = sharingRows.data || [];
  const [sharing, setSharing] = React.useState({});
  const isOn = (row) => (sharing[row.label] !== undefined ? sharing[row.label] : row.on);
  const allInflowRows = useAsync(() => ZS_API.getTeamInflow(), "inflow:all", []);
  const allInflow = allInflowRows.data || [];
  const open = openId && teams.find(t => t.id === openId);
  if (open) return <TeamDetail team={open} startPerson={openPerson}
    onBack={() => { setOpenId(null); setOpenPerson(null); }} mobile={mobile} />;

  const reporting = teams.filter(t => t.sessions > 0);
  const people = teams.flatMap(t => t.people.map(p => ({ ...p, team: t.name })));
  const sessions = teams.reduce((a, t) => a + t.sessions, 0);
  const ftr = Math.round(reporting.reduce((a, t) => a + t.ftr * t.sessions, 0) / (sessions || 1));
  const triage = teams.reduce((a, t) => a + t.inflow.triage, 0);
  const tabs = [
    { id: "teams", label: "Teams", icon: "users-group-rounded" },
    { id: "people", label: "People", icon: "user-hands" },
    { id: "inflow", label: "Inflow", icon: "upload-square", badge: triage || undefined },
  ];
  return (
    <W>
      <SectionHead eyebrow={org.name + " · collective"} title="Teams" count={teams.length}
        right={!mobile && <div className="flex items-center gap-6">
          <StatBadge n={ftr + "%"} label="first-try" sub="weighted" />
          <StatBadge n={sessions} label="sessions" sub="this week" />
          <StatBadge n={triage} label="in triage" tone="var(--accent)" />
        </div>} />
      <SubTabs tabs={tabs} active={tab} onPick={setTab} />

      {tab === "teams" && (
        <React.Fragment>
          <Banner kanji="組" tone="neutral" title="A team is where solo practice becomes shared practice.">
            Each developer runs sensei on their own machine. What they send up — a learning, a memory, an instrument, a count — rolls up here, and drills back down to the person who sent it.
          </Banner>
          <ListSection icon="users-group-rounded" title="Teams in this dōjō" count={teams.length}
            right={<span className="zs-meta">first-try rate · sent → adopted</span>}>
            {!mobile && (
              <div className="border-b grid gap-3 py-2 px-4 bg-paper-mute" style={{ gridTemplateColumns: TEAM_COLS }}>
                {["team", "size", "sessions", "first-try", "sent→kept", "queue"].map((h, i) => (
                  <span key={h} className="zs-eyebrow text-ink-mute" style={i === 5 ? { textAlign: "right" } : null}>{h}</span>
                ))}
              </div>
            )}
            {teams.map(t => <TeamRow key={t.id} t={t} onOpen={(x) => setOpenId(x.id)} mobile={mobile} />)}
          </ListSection>
        </React.Fragment>
      )}

      {tab === "people" && (
        <React.Fragment>
          <Banner kanji="士" tone="neutral" title="Every developer, across every team.">
            Sorted by first-try rate. The number is a mirror, not a leaderboard — a low rate usually means a missing rule, not a careless developer.
          </Banner>
          <ListSection icon="user-hands" title="Developers" count={people.length}>
            {!mobile && <PeopleHead />}
            {people.slice().sort((a, b) => b.ftr - a.ftr).map(p => (
              <PersonRow key={p.team + p.name} p={p} mobile={mobile}
                onOpen={() => { setOpenPerson(p.name); setOpenId((teams.find(t => t.name === p.team) || {}).id); }} />
            ))}
          </ListSection>
        </React.Fragment>
      )}

      {tab === "inflow" && (
        <React.Fragment>
          <Banner kanji="共" tone="neutral" title="Nothing arrives that a developer didn’t send.">
            sensei is local-first. Sharing is a switch on each machine, and client work is anonymized before it leaves — the lesson travels, the client never does.
          </Banner>
          <ListSection icon="tuning-2" title="What a local sensei sends" count={shareDefs.length}
            right={<span className="zs-meta">the developer’s switch · shown here, set locally</span>}>
            {shareDefs.map((sh, i) => {
              const on = isOn(sh, i);
              return (
                <Row key={sh.label} align="center">
                  <KanjiMark char={sh.kanji} size="base" tone={on ? "var(--accent)" : "var(--ink-faint)"} w={18} />
                  <div className="flex-1 min-w-0">
                    <div className={"text-sm " + (on ? "text-ink" : "text-ink-mute")}>{sh.label}</div>
                    <div className="zs-meta" style={{ marginTop: 1 }}>{sh.note}</div>
                  </div>
                  <Toggle on={on} onToggle={() => setSharing(v => ({ ...v, [sh.label]: !on }))} label={sh.label} />
                </Row>
              );
            })}
          </ListSection>
          <ListSection icon="upload-square" iconColor="var(--accent)" title="Arrived this week" count={allInflow.length}
            right={<span className="zs-meta">{triage} waiting on a maintainer</span>}>
            {allInflow.map(f => <InflowRow key={f.id} it={f} />)}
          </ListSection>
        </React.Fragment>
      )}
    </W>
  );
}

/* ═══ ORG ZONES — one destination, tabs inside ═════════════ */
function ScrOrgZone({ org, zone, tab, onTab, openProject, mobile }) {
  const g = ORG_GROUPS[zone];
  const t = tab || g.tabs[0].id;
  const W = mobile ? BodyM : Body;
  const screen = ({
    triage: () => <ScrTriage org={org} mobile={mobile} />,
    approvals: () => <ScrApprovals org={org} mobile={mobile} />,
    knowledge: () => <ScrKnowledge org={org} mobile={mobile} />,
    engagements: () => <ScrEngagements org={org} mobile={mobile} />,
    incidents: () => <ScrIncidents org={org} mobile={mobile} />,
    clientaudit: () => <ScrClientAudit org={org} mobile={mobile} />,
    members: () => <ScrRoleSurfaces org={org} tab="members" hideTabs mobile={mobile} />,
    roles: () => <ScrRoleSurfaces org={org} tab="policies" hideTabs mobile={mobile} />,
    audit: () => <ScrRoleSurfaces org={org} tab="audit" hideTabs mobile={mobile} />,
    scopes: () => <ScrScopes org={org} mobile={mobile} />,
    identity: () => <ScrIdentity org={org} mobile={mobile} />,
    health: () => <ScrHealth org={org} mobile={mobile} />,
    billing: () => <ScrBilling org={org} mobile={mobile} />,
  })[t];
  return (
    <div>
      <div style={{ padding: mobile ? "var(--space-4) var(--space-4) 0" : "var(--space-8) var(--space-8) 0" }}>
        <SubTabs tabs={g.tabs} active={t} onPick={onTab} />
      </div>
      {screen ? screen() : null}
    </div>
  );
}

/* ═══ WIRED APP — desktop ══════════════════════════════════ */
function DojoApp2({ label, start = "you", startNav, startProject, startRun, startTeam, startTeamTab, startPerson }) {
  const [signedIn, setSignedIn] = React.useState(start !== "signin");
  const [ctx, setCtx] = React.useState(start === "you" || start === "signin" ? "you" : start);   // "you" | org slug
  const dojos = useAsync(() => ZS_API.getMyDojos(), "dojos", []).data || [];
  const me = useAsync(() => ZS_API.getMe(), "me", {}).data || {};
  const org = dojos.find(d => d.slug === ctx) || null;
  const myProjects = useAsync(() => ZS_API.getProjects(), "projects", []).data || [];
  const orgProjects = useAsync(() => ZS_API.getProjects(org && org.slug), "orgprojects:" + (org ? org.slug : "-"), []).data || [];
  const zoneOf = startNav && ORG_ZONE_OF[startNav];
  const [nav, setNav] = React.useState(zoneOf || startNav || (org ? "home" : "inbox"));
  const [zoneTab, setZoneTab] = React.useState(zoneOf ? { [zoneOf]: startNav } : {});
  const pickTab = (z, id) => setZoneTab(m => ({ ...m, [z]: id }));
  const [proj, setProj] = React.useState(startProject || null);

  const onPick = (slug) => {
    if (slug === "you") { setCtx("you"); setNav("inbox"); }
    else { setCtx(slug); setNav("home"); }
    setProj(null);
  };
  const openProject = (p) => setProj(p);
  const back = () => setProj(null);
  const [resolved, setResolved] = React.useState({});
  const onAct = (item, action) => setResolved(r => ({ ...r, [item.id]: action.id === "deny" ? "denied" : action.id === "settle" ? "settled" : action.id === "decide" ? "decided" : "approved" }));
  const [answered, setAnswered] = React.useState({});
  const onAnswer = (item, verdict) => setAnswered(a => ({ ...a, [item.id]: verdict }));
  const askCount = useAsync(() => ZS_API.getAsks(), "asks", []);
  const inboxNeeds = (askCount.data || []).filter(it => !answered[it.id]).length;
  const orgNeeds = useAsync(() => ZS_API.getOrgNeeds(), "orgneeds", []).data || [];
  const needsCount = org ? orgNeeds.filter(it => !resolved[it.id]).length : inboxNeeds;
  const onNeeds = () => { setProj(null); setNav(org ? "home" : "inbox"); };

  let body;
  if (!signedIn) return <ScrSignIn label={label} onContinue={() => { setSignedIn(true); setCtx("you"); setNav("inbox"); }} />;
  if (proj) body = <ScrProjectPreview project={proj} onBack={back} />;
  else if (!org) {
    body = ({
      inbox: <ScrInbox key={startRun || "inbox"} startRun={startRun} answered={answered} onAnswer={onAnswer} />,
      projects: <ScrProjects projects={myProjects} onOpenProject={openProject} eyebrow="Everything in flight" title="Your projects" />,
      dojos: <ScrMyDojos onOpen={(d) => onPick(d.slug)} />,
      contributions: <ScrContributions />,
      rules: <ScrConstitution onGoPacks={() => setNav("packs")} />,
      packs: <ScrRulePacks />,
    })[nav] || <ScrInbox answered={answered} onAnswer={onAnswer} />;
  } else {
    const projs = orgProjects;
    body = ORG_GROUPS[nav]
      ? <ScrOrgZone org={org} zone={nav} tab={zoneTab[nav]} onTab={(id) => pickTab(nav, id)} />
      : ({
        home: <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} />,
        ladder: <ScrOrgLadder org={org} />,
        teams: <ScrTeams org={org} startTeam={startTeam} startTeamTab={startTeamTab} startPerson={startPerson} />,
        projects: <ScrProjects projects={projs} showDojo={false} onOpenProject={openProject} eyebrow={org.name + " · jurisdiction"} title="Projects" />,
      })[nav] || <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} />;
  }

  return (
    <AppShell label={label} context={org ? "org" : "you"} org={org || dojos[0]} dojos={dojos} me={me}
      needsCount={needsCount} onNeeds={onNeeds}
      nav={org ? navForOrg(org) : navYou(inboxNeeds)} active={proj ? null : nav} onNav={(id) => { setProj(null); setNav(id); }} onPick={onPick}>
      {body}
    </AppShell>
  );
}

/* ═══ WIRED APP — mobile ═══════════════════════════════════ */
function DojoApp2Mobile({ label, start = "you", startTab, startRun, startTeam }) {
  const [signedIn, setSignedIn] = React.useState(start !== "signin");
  const [ctx, setCtx] = React.useState(start === "you" || start === "signin" ? "you" : start);
  const dojos = useAsync(() => ZS_API.getMyDojos(), "dojos", []).data || [];
  const me = useAsync(() => ZS_API.getMe(), "me", {}).data || {};
  const org = dojos.find(d => d.slug === ctx) || null;
  const myProjects = useAsync(() => ZS_API.getProjects(), "projects", []).data || [];
  const orgProjects = useAsync(() => ZS_API.getProjects(org && org.slug), "orgprojects:" + (org ? org.slug : "-"), []).data || [];
  const [tab, setTab] = React.useState(startTab || (org ? "home" : "inbox"));
  const [proj, setProj] = React.useState(null);
  const openProject = (p) => setProj(p);
  const back = () => setProj(null);
  const [resolved, setResolved] = React.useState({});
  const onAct = (item, action) => setResolved(r => ({ ...r, [item.id]: action.id === "deny" ? "denied" : action.id === "settle" ? "settled" : action.id === "decide" ? "decided" : "approved" }));
  const [answered, setAnswered] = React.useState({});
  const onAnswer = (item, verdict) => setAnswered(a => ({ ...a, [item.id]: verdict }));
  const askCount = useAsync(() => ZS_API.getAsks(), "asks", []);
  const inboxNeeds = (askCount.data || []).filter(it => !answered[it.id]).length;

  let body;
  if (!signedIn) return <ScrSignIn label={label} mobile onContinue={() => { setSignedIn(true); setCtx("you"); setTab("inbox"); }} />;
  if (proj) body = <ScrProjectPreview project={proj} onBack={back} mobile />;
  else if (!org) {
    body = ({
      inbox: <ScrInbox key={startRun || "inbox"} startRun={startRun} answered={answered} onAnswer={onAnswer} mobile />,
      projects: <ScrProjects projects={myProjects} onOpenProject={openProject} eyebrow="Everything in flight" title="Your projects" mobile />,
      rules: <ScrConstitution onGoPacks={() => setTab("rules")} mobile />,
      dojos: <ScrMyDojos onOpen={(d) => { setCtx(d.slug); setTab("home"); }} mobile />,
    })[tab] || <ScrInbox answered={answered} onAnswer={onAnswer} mobile />;
  } else {
    const projs = orgProjects;
    body = ({
      home: <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />,
      projects: <ScrProjects projects={projs} showDojo={false} onOpenProject={openProject} eyebrow={org.name} title="Projects" mobile />,
      ladder: <ScrOrgLadder org={org} mobile />,
      teams: <ScrTeams org={org} startTeam={startTeam} mobile />,
    })[tab] || <ScrOrgHome org={org} onOpenProject={openProject} onAct={onAct} resolved={resolved} mobile />;
  }

  return (
    <MobileShell label={label} context={org ? "org" : "you"} org={org || dojos[0]} me={me}
      tabs={org ? TABS_ORG : tabsYou(inboxNeeds)} active={proj ? null : tab} onNav={(id) => { setProj(null); setTab(id); }}>
      {body}
    </MobileShell>
  );
}

Object.assign(window, {
  DojoApp2, DojoApp2Mobile, ScrSignIn, ScrMyDojos, ScrContributions, ScrScopes, ScrBilling,
  ScrTriage, ScrApprovals, ScrKnowledge, ScrEngagements, ScrIncidents, ScrClientAudit, ScrIdentity, ScrHealth,
  ScrProjects, ScrConstitution, ScrRulePacks, ScrProjectPreview,
  ScrInbox, RunDetail, pendingAsksFor, ScrOrgZone, ORG_GROUPS, ScrTeams, TeamDetail, PersonPanel,
  ScrOrgHome, ScrOrgLadder, ScrRoleSurfaces,
});
