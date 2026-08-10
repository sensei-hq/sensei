// ─── Torii + Seiki product page content — from docs/mockups/Sensei/hq/torii-seiki.jsx ───
// Two clients for an org's models, both built on Gateway: Torii (member
// workspace) and Seiki (admin/governance portal).

export interface Client {
  id: 'torii' | 'seiki';
  kanji: string;
  gloss: string;
  name: string;
  category: string;
  tagline: string;
  blurb: string;
  surfaces: string[];
  meta: string[];
  status: string;
}

export interface Capability {
  kanji: string;
  title: string;
  text: string;
}

export interface Plane {
  label: string;
  kanji: string;
  text: string;
}

export interface DetailRow {
  term: string;
  detail: string;
}

export const GATEWAY_URL = 'https://gateway.sensei-hq.com';

export const CLIENTS: Client[] = [
  {
    id: 'torii', kanji: '門', gloss: 'Mon · the gate',
    name: 'Torii', category: 'Desktop · Member workspace',
    tagline: 'The gate your team walks through.',
    blurb: 'Everything a member needs and nothing they don’t. Ask a model, keep a library, try things in the playground — and always see where the answer actually ran.',
    surfaces: ['Workspace', 'Ask', 'Library', 'Playground', 'Activity', 'Settings'],
    meta: ['macOS · Windows · Linux', 'Tauri', 'Works offline'],
    status: 'Beta',
  },
  {
    id: 'seiki', kanji: '社', gloss: 'Sha · the sanctuary',
    name: 'Seiki', category: 'Web · Governance plane',
    tagline: 'The sanctuary behind it.',
    blurb: 'The quiet room where the rules are kept. Every request, every provider, every fallback chain and every budget — set once, cascading down your org, without standing over anyone’s shoulder.',
    surfaces: ['Overview', 'Requests & audit', 'Organization', 'Models', 'Routing', 'Connections', 'Governance', 'Budgets'],
    meta: ['Self-hosted', 'SSO · SCIM', 'Full audit trail'],
    status: 'Beta',
  },
];

export const PLANES: Plane[] = [
  { label: 'On your device', kanji: '手', text: 'Local models, local context, nothing leaves the machine. Marked as such on every answer.' },
  { label: 'Via the gateway', kanji: '関', text: 'Routed, logged and budgeted through your own deployment — in the region you chose.' },
];

export const CAPABILITIES: Capability[] = [
  { kanji: '路', title: 'Fallback chains', text: 'Order your models once — primary, then the next, then the local one. Gateway walks the chain when a provider stalls, and Seiki shows you how often it had to.' },
  { kanji: '繋', title: 'Connections', text: 'Bring your own keys for every provider and router. Strategos holds the credentials; your apps hold one address.' },
  { kanji: '具', title: 'MCP & tools', text: 'Register MCP servers — stdio on the desktop, http for shared ones — and allow-list tools per role and space.' },
  { kanji: '鍵', title: 'Programmatic access', text: 'Scoped keys turn the same gateway into an endpoint your own services can call, with usage attributed back to a team.' },
  { kanji: '階', title: 'Hierarchy & budgets', text: 'Org, department, team, person — your real structure. Permissions follow it, and so do spend caps, cascading downward.' },
  { kanji: '器', title: 'Devices & offline', text: 'Local models run on the machine in front of you. When the gateway is unreachable, work continues and syncs later.' },
];

export const PRIVACY_ROWS: DetailRow[] = [
  { term: 'Prompts and responses', detail: 'Only to the provider you routed them to.' },
  { term: 'Audit log', detail: 'Never. It lives in your database.' },
  { term: 'Usage telemetry', detail: 'Never. There is none to send.' },
  { term: 'Licence check', detail: 'Once, at install. Offline after that.' },
];

export const GATEWAY_ROWS: DetailRow[] = [
  { term: 'Fallback chains', detail: 'Primary, secondary, local — in order, on failure.' },
  { term: 'Budget control', detail: 'Hard ceilings per key, per team, per period.' },
  { term: 'One interface', detail: 'Every provider behind a single Rust API.' },
  { term: 'Rust, embedded', detail: 'A library in your service, not another hop.' },
];

export const TS_NAV_LINKS = [
  ['#clients', 'The pair'],
  ['#planes', 'Two planes'],
  ['#capabilities', 'Capabilities'],
  ['#gateway', 'Gateway'],
] as const;
