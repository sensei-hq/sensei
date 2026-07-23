// Shapes the dojo2 kit rows bind to. Ported from the mockup fixtures
// (docs/mockups/Sensei/lib/data/dojo2-data.js) so screens and specs share one
// self-describing contract. Kept flat and optional-heavy to match the mock; the
// wire types will refine these when the screens land.

/** A project in flight — the row workhorse (dojo2-data `projects[]`). */
export interface KitProject {
	id: string;
	name: string;
	repo: string;
	/** company · client · personal · community. */
	classification: string;
	/** watch · notice · adopt. */
	phase: string;
	/** Compact "how long ago" the last run was (e.g. "8m"). */
	lastRun?: string;
	/** Count of things in this project that need the viewer; 0 = none. */
	needs?: number;
	/** Owning dōjō display name (shown when `showDojo`). */
	dojoName?: string | null;
	/** A short human note about the project's current signal. */
	note?: string | null;
	/** Sparkline series for the recent run cadence. */
	spark?: number[];
	/** Sessions run against this project this week (dojo2-data `runsWeek`). */
	runsWeek?: number;
}

/** A dōjō membership row (dojo2-data `dojos[]`). */
export interface KitDojo {
	slug: string;
	kanji: string;
	name: string;
	/** employer · client · community · personal. */
	kind: string;
	/** developer · maintainer · lead · admin. */
	role: string;
	/** The dōjō's public route (e.g. "sensei-hq.com/acme"). */
	route: string;
	members: number;
	projects: number;
	/** Count of things in this dōjō that need the viewer; 0 = none. */
	needs?: number;
}

/** The viewer (dojo2-data `me`). */
export interface KitMe {
	name: string;
	handle?: string;
	avatar?: string;
}

/** An org context — a dōjō the viewer has stepped into (dojo2-data `dojos[]`). */
export interface KitOrg {
	slug: string;
	kanji: string;
	name: string;
	kind: string;
	role: string;
	route: string;
}

/** A nav item inside a NavPane group or a TabBar (icon OR kanji + label). */
export interface KitNavItem {
	id: string;
	label: string;
	icon?: string;
	kanji?: string;
	/** Optional count badge. */
	badge?: number;
}

/** A titled group of nav items in the NavPane. */
export interface KitNavGroup {
	group?: string;
	items: KitNavItem[];
}

// ── Governance plane (dojo2-data `ladder` · `conflicts` · `stance`) ─────────

/** A single rule on the constitution ladder (dojo2-data `ladder[].rules[]`). */
export interface KitRule {
	/** Brand glyph for the rule's intent. */
	kanji: string;
	/** The rule text. */
	text: string;
	/** ★ non-negotiable — a hard lock no narrower scope can relax. */
	hard?: boolean;
	/** The rung the rule entered from (shown when `showLevel`). */
	level?: string;
}

/** One rung of the constitution ladder — a scope + its rules (dojo2-data `ladder[]`). */
export interface KitLadderRung {
	id: string;
	/** Scope glyph (社 · 客 · 己 · 件 · 技). */
	kanji: string;
	/** Scope label — Company · Client · Personal · Project · Stack. */
	scope: string;
	/** The named entity at this scope (e.g. "Acme Corp"). */
	name: string;
	/** A short line on what the rung governs. */
	caption: string;
	/** `accent` tints the rung (a switched-on client rung); otherwise neutral ink. */
	tone?: string;
	rules?: KitRule[];
}

/** One side of a settled conflict — the losing/winning rule (dojo2-data `conflicts[].loser`). */
export interface KitConflictSide {
	/** The scope level the rule came from. */
	level: string;
	text: string;
}

/** A conflict the ladder settled — topic · loser → winner · why (dojo2-data `conflicts[]`). */
export interface KitConflict {
	id: string;
	topic: string;
	loser: KitConflictSide;
	winner: KitConflictSide;
	why: string;
	/** A ★ non-negotiable decided it — no narrower scope can relax it. */
	locked?: boolean;
}

/** An adoptable rule-pack bundle (dojo2-data `rulePacks[]`). Adopting adds its
 *  rules to the constitution at a chosen scope — a starting point, not a lock. */
export interface KitRulePack {
	id: string;
	/** Brand glyph for the pack's intent. */
	kanji: string;
	name: string;
	/** Who authors the pack (e.g. "Acme · platform"). */
	by: string;
	/** How many rules the pack carries. */
	count: number;
	/** Whether the viewer has adopted the pack into their constitution. */
	adopted: boolean;
	/** A short line on what the pack covers. */
	note: string;
}

/** One section of a dōjō's OWN authored constitution (dojo2-data
 *  `orgConstitution[slug][]`). A dōjō authors rules at the scopes it owns —
 *  company-wide, per team, per stack (stacks also adopt rule packs). This is
 *  NOT the resolution ladder (that appears only at project-preview time): it is
 *  the maintainer-authoring surface where the dōjō adds/edits/toggles its rules. */
export interface KitConstitutionSection {
	id: string;
	/** Scope glyph (社 · 組 · 技). */
	kanji: string;
	/** The scope label (e.g. "Company-wide" · "Payments" · "Postgres"). */
	scope: string;
	/** The authoring group the section sits under — Company · Teams · Stacks. */
	group: string;
	/** A short line on what the section governs. */
	caption: string;
	/** Adopted rule-pack names (stacks only); absent for company/team scopes. */
	packs?: string[];
	rules?: KitRule[];
}

/** The org-context stat row for the org home (dojo2-data `dojos[]` counts). The
 *  jurisdiction summary — members, items needing a maintainer, projects in flight. */
export interface KitOrgStats {
	members: number;
	/** Items across this jurisdiction that need a maintainer; 0 = none. */
	needs: number;
	/** Projects in flight under this dōjō. */
	projects: number;
}

/** A stance axis — a labelled discrete slider (dojo2-data `stance[]`). */
export interface KitStanceDial {
	/** autonomy · sharing · review. */
	id: string;
	kanji: string;
	label: string;
	caption: string;
	/** Ordered level labels; the slider snaps between them. */
	levels: string[];
	/** Index of the current level within `levels`. */
	value: number;
}

// ── Relay plane (dojo2-data `runs` · `gates` · `needsYou` · `decisions` · `chat`) ─

/** A live session in progress (dojo2-data `runs[]`). */
export interface KitRun {
	id: string;
	project: string;
	assistant: string;
	/** running · waiting. */
	state: string;
	task: string;
	/** Compact elapsed time (e.g. "38m"). */
	elapsed: string;
	edits: number;
	/** A command is waiting on a gate. */
	gate?: boolean;
}

/** A command awaiting approve/deny (dojo2-data `gates[]`). */
export interface KitGate {
	id: string;
	project: string;
	/** The command line to run. */
	cmd: string;
	/** command · secret · … */
	kind: string;
	/** guarded · high. */
	risk: string;
	why: string;
	/** Owning session id. */
	session: string;
	/** Compact "how long ago" (e.g. "3m"). */
	age: string;
}

/** A "blocked on you" item in the cross-dōjō needs-you band (dojo2-data `needsYou[]`). */
export interface KitNeed {
	id: string;
	/** gate · conflict · decision · review — drives the icon + action set. */
	kind: string;
	title: string;
	project: string;
	/** Owning dōjō display name. */
	dojo: string;
	why: string;
	age: string;
}

/** A rule to sign off, with options (dojo2-data `decisions[]`). */
export interface KitDecision {
	id: string;
	project: string;
	title: string;
	/** The choices; the first is the primary CTA. */
	options: string[];
	/** The evidence line under the title. */
	context: string;
	age: string;
}

// ── Contributions (dojo2-data `contributions`) ─────────────────────────────

/** Something you shared upstream and its fate (dojo2-data `contributions.mine[]`).
 *  You propose; a maintainer decides — client work anonymizes before it leaves. */
export interface KitContribution {
	/** Brand glyph for the shared item's intent. */
	kanji: string;
	title: string;
	/** The destination dōjō the contribution landed in. */
	dest: string;
	/** The scope it was proposed at (e.g. "Stack · Rust"). */
	scope: string;
	/** approved · pending · declined — drives the status chip. */
	status: string;
	/** Compact "how long ago" (e.g. "2d"). */
	when: string;
	/** A short line on the outcome / where it sits. */
	note: string;
	/** Client work — anonymized before it left; carries a shield marker. */
	client?: boolean;
}

/** A downstream item approved for you from a dōjō (dojo2-data
 *  `contributions.downstream[]`). Never silently merged — you pin it in. */
export interface KitDownstream {
	kanji: string;
	title: string;
	/** The dōjō it came from. */
	from: string;
	scope: string;
	when: string;
	/** Whether you've adopted it locally; unadopted rows are "new" + pinnable. */
	adopted: boolean;
	/** guard · pattern · skill — the kind of thing shared. */
	kind: string;
}

/** The contributions stat row (dojo2-data `contributions.stat`). */
export interface KitContribStat {
	approved: number;
	pending: number;
	/** Lifetime devs the viewer's contributions have helped. */
	helped: number;
}

/** One turn in a sensei-speaks-rarely thread (dojo2-data `chat.thread[]`). */
export interface KitChatTurn {
	/** `sensei` for the mentor voice; anything else is the viewer ("me"). */
	who: string;
	/** Brand glyph for the sensei voice (default 先). */
	kanji?: string;
	text: string;
	/** Compact "how long ago" (e.g. "38m"). */
	when: string;
}

// ── Maintainer + lead consoles (dojo2-data `consoles`) ──────────────────────

/** A candidate learning waiting on a maintainer in the triage queue (dojo2-data
 *  `consoles.triage[].items[]`). Ranked by confidence within its scope group. */
export interface KitTriageCandidate {
	id: string;
	/** Brand glyph for the candidate's intent. */
	kanji: string;
	title: string;
	/** Where it came from (e.g. "6 sessions · 3 repos"). */
	origin: string;
	/** Confidence 0..1 — drives the bar + the detail enso. */
	conf: number;
	/** Ladder conflicts this candidate raises (0 = none). */
	conflicts: number;
	/** Near-duplicate candidates it overlaps (0 = none). */
	dups: number;
	/** high · safety · normal · low — high/safety show an impact chip + a
	 *  second-approval note. */
	impact: string;
}

/** A scope-grouped batch of triage candidates (dojo2-data `consoles.triage[]`). */
export interface KitTriageGroup {
	/** The scope the candidates share (e.g. "Payments"). */
	scope: string;
	items: KitTriageCandidate[];
}

/** The detail for the selected triage candidate (dojo2-data
 *  `consoles.candidateDetail`). Shown in the desktop right pane. */
export interface KitCandidateDetail {
	/** The one-line learning the candidate proposes. */
	learning: string;
	/** Why it surfaced. */
	cause: string;
	/** Where it surfaced. */
	context: string;
	/** The supporting sessions/events. */
	evidence: string[];
	/** The ladder conflict this candidate settles (loser → winner). */
	conflict: { loser: string; winner: string };
	/** The scope chips it could distribute to; the second is the recommended one. */
	scopes: string[];
}

/** A high-impact candidate awaiting a second maintainer's signature (dojo2-data
 *  `consoles.approvals[]`). */
export interface KitApproval {
	id: string;
	kanji: string;
	title: string;
	/** The scope it would publish to. */
	scope: string;
	/** Who gave the first approval. */
	first: string;
	/** Compact "how long ago" the first approval landed. */
	when: string;
	/** high · safety — drives the impact chip tone. */
	impact: string;
}

/** A published-knowledge row — adopted or pending prune (dojo2-data
 *  `consoles.knowledge.active[]` / `.pending[]`). */
export interface KitKnowledgeRow {
	kanji: string;
	title: string;
	/** The scope it publishes to. */
	scope: string;
	/** Adoption reach (e.g. "6 repos") — active rows only. */
	adopted?: string;
	/** Age / prune line (e.g. "adopted 3mo" · "unused 84d"). */
	age: string;
}

/** A shareable extension in the knowledge catalog (dojo2-data
 *  `consoles.knowledge.catalog[]`). */
export interface KitCatalogItem {
	kanji: string;
	title: string;
	/** agent · command · skill — drives the row icon + the kind chip. */
	kind: string;
	scope: string;
}

/** The published-knowledge library + prune policy + catalog (dojo2-data
 *  `consoles.knowledge`). */
export interface KitKnowledge {
	/** The prune-policy line (e.g. "Prune after 90 days unused"). */
	prunePolicy: string;
	active: KitKnowledgeRow[];
	pending: KitKnowledgeRow[];
	catalog: KitCatalogItem[];
}

/** A client engagement in the register (dojo2-data `consoles.engagements[]`).
 *  Lessons cross the boundary anonymized; the client/repo/source never do. */
export interface KitEngagement {
	id: string;
	kanji: string;
	/** The client display name. */
	client: string;
	/** The projects in the engagement. */
	projects: string;
	/** Lessons kept (crossed the boundary anonymized). */
	lessons: number;
	/** Identifiers/source stripped before anything left. */
	dropped: number;
	/** How long the engagement has run (e.g. "7mo"). */
	since: string;
	/** active · … */
	status: string;
}

/** The confidentiality model — what crosses the boundary and an anonymized
 *  example (dojo2-data `consoles.confidentiality`). */
export interface KitConfidentiality {
	/** What travels upstream (the lesson, anonymized). */
	kept: string[];
	/** What is stripped before anything leaves. */
	dropped: string[];
	/** A raw → anonymized code example. */
	example: { raw: string; stripped: string };
}

/** A confidentiality incident (dojo2-data `consoles.incidents[]`). */
export interface KitIncident {
	id: string;
	kanji: string;
	title: string;
	/** The client the incident touched. */
	client: string;
	/** contained · resolved · open — drives the state dot tone. */
	state: string;
	/** Compact "how long ago" (e.g. "3d"). */
	when: string;
	/** high · medium — drives the severity chip tone. */
	severity: string;
}

/** An immutable client-audit ledger entry (dojo2-data `consoles.clientAudit[]`).
 *  Append-only proof that confidentiality held, entry by entry. */
export interface KitClientAuditRow {
	/** The wall-clock/relative timestamp (e.g. "10:42" · "Yesterday"). */
	t: string;
	kanji: string;
	/** What happened (e.g. "Lesson shared upstream"). */
	event: string;
	/** The specifics (e.g. "idempotency pattern · anonymized"). */
	detail: string;
	/** The client the entry concerns. */
	client: string;
	/** Whether confidentiality held (true) or a contribution was blocked (false). */
	ok: boolean;
}
