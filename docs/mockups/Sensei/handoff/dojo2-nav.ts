// Personal + org IA as of the Inbox redesign.
// Replaces: Live runs · Approve · Decide · Chat (four relay sections) with one
// Inbox, and the 14-section org rail with 3 destinations + 3 role-gated zones.
//
// The rule behind both: a *destination* is a place you go; answering an ask is
// an action ON a session, not a place. Asks are answered in the session detail
// (`/you/inbox/[run_id]`, "Needs you" tab), never in their own section.

export type NavBadge = number | undefined;

export interface NavItem {
  id: string;
  icon: string;
  label: string;
  href: string;
  badge?: NavBadge;
}

export interface NavGroup {
  group: string;
  role?: OrgRole;        // minimum role; omit for "everyone"
  items: NavItem[];
}

/* ── personal ─────────────────────────────────────────────── */

// `needs` = pending asks across all in-flight runs (see inboxNeedsCount).
export const navYou = (needs: number): NavGroup[] => [
  {
    group: 'Work',
    items: [
      { id: 'inbox', icon: 'inbox', label: 'Inbox', href: '/you/inbox', badge: needs || undefined },
      { id: 'projects', icon: 'folder', label: 'Projects', href: '/you/projects' },
    ],
  },
  {
    group: 'Govern',
    items: [
      { id: 'rules', icon: 'scale', label: 'Constitution', href: '/you/constitution' },
      { id: 'packs', icon: 'box', label: 'Rule packs', href: '/you/packs' },
    ],
  },
  {
    group: 'Dōjōs',
    items: [
      { id: 'dojos', icon: 'users-group-two-rounded', label: 'My dōjōs', href: '/you/dojos' },
      { id: 'contributions', icon: 'graph-up', label: 'Contributions', href: '/you/contributions' },
    ],
  },
];

// Phone tab bar — same destinations, four slots.
export const tabsYou = (needs: number): NavItem[] => [
  { id: 'inbox', icon: 'inbox', label: 'Inbox', href: '/you/inbox', badge: needs || undefined },
  { id: 'projects', icon: 'folder', label: 'Projects', href: '/you/projects' },
  { id: 'rules', icon: 'scale', label: 'Rules', href: '/you/constitution' },
  { id: 'dojos', icon: 'users-group-two-rounded', label: 'Dōjōs', href: '/you/dojos' },
];

// Retired personal sections → where they went. Keep as redirects.
export const RETIRED_YOU: Record<string, string> = {
  runs: '/you/inbox',                    // "Live runs" IS the inbox
  approve: '/you/inbox?filter=needs',    // approvals are asks on a session
  decide: '/you/inbox?filter=needs',     // decisions are asks on a session
  chat: '/you/inbox?filter=needs',       // a reply is an answer to an ask
  work: '/you/inbox',                    // the "Needs you" landing band
};

/* ── org ──────────────────────────────────────────────────── */

export type OrgRole = 'developer' | 'maintainer' | 'lead' | 'admin';

// Additive: a zone shows when the viewer's role reaches its floor.
export const ROLE_RANK: Record<OrgRole, number> = {
  developer: 0,
  maintainer: 1,
  lead: 2,
  admin: 3,
};

export interface OrgZone {
  id: string;
  icon: string;
  label: string;
  role: OrgRole;
  tabs: { id: string; label: string; icon: string; badge?: NavBadge }[];
}

// Everything that used to be its own rail item is a tab inside its zone —
// one level of nesting instead of fourteen top-level sections.
export const ORG_ZONES: OrgZone[] = [
  {
    id: 'governance', icon: 'scale', label: 'Governance', role: 'maintainer',
    tabs: [
      { id: 'triage', label: 'Triage', icon: 'inbox' },
      { id: 'approvals', label: 'Approvals', icon: 'clipboard-check' },
      { id: 'knowledge', label: 'Knowledge', icon: 'book-2' },
    ],
  },
  {
    id: 'clients', icon: 'case-round', label: 'Clients', role: 'lead',
    tabs: [
      { id: 'engagements', label: 'Engagements', icon: 'case-round' },
      { id: 'incidents', label: 'Incidents', icon: 'shield-warning' },
      { id: 'clientaudit', label: 'Client audit', icon: 'document-text' },
    ],
  },
  {
    id: 'admin', icon: 'shield-user', label: 'Admin', role: 'admin',
    tabs: [
      { id: 'members', label: 'Members', icon: 'users-group-rounded' },
      { id: 'roles', label: 'Roles', icon: 'shield-check' },
      { id: 'scopes', label: 'Scopes', icon: 'tuning-2' },
      { id: 'identity', label: 'Identity', icon: 'key' },
      { id: 'audit', label: 'Audit', icon: 'clipboard-list' },
      { id: 'health', label: 'Health', icon: 'pulse' },
      { id: 'billing', label: 'Billing', icon: 'card' },
    ],
  },
];

// Legacy deep links (a bare tab id) resolve to zone + tab.
export const ORG_ZONE_OF: Record<string, string> = Object.fromEntries(
  ORG_ZONES.flatMap((z) => z.tabs.map((t) => [t.id, z.id])),
);

export const navForOrg = (
  slug: string,
  role: OrgRole,
  badges: Record<string, number> = {},
): NavGroup[] => {
  const rank = ROLE_RANK[role] ?? 0;
  const zones = ORG_ZONES.filter((z) => rank >= ROLE_RANK[z.role]).map((z) => ({
    id: z.id,
    icon: z.icon,
    label: z.label,
    href: `/${slug}/${z.id}`,
    badge: badges[z.id] || undefined,   // rolls up its tabs' counts
  }));
  return [
    {
      group: 'Dōjō',
      items: [
        { id: 'home', icon: 'buildings-2', label: 'Home', href: `/${slug}` },
        { id: 'ladder', icon: 'scale', label: 'Constitution', href: `/${slug}/constitution` },
        { id: 'projects', icon: 'folder', label: 'Projects', href: `/${slug}/projects`, badge: badges.projects || undefined },
      ],
    },
    ...(zones.length ? [{ group: 'Manage', items: zones }] : []),
  ];
};
