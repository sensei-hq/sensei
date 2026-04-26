// Setup wizard — mock data for development.
// Replace with real daemon API calls when endpoints are ready.

import type {
  AcpEntry, ScanFolder, ScanEvent, DiscoveredProject,
  DiscoveredLibrary, McpEntry, WizardState
} from './types.js';

export const MOCK_ACPS: AcpEntry[] = [
  { id: 'claude-code', name: 'Claude Code', version: '1.8.2', found: true,  path: '/Users/aiko/.claude/code' },
  { id: 'cursor',      name: 'Cursor',      version: '0.42',  found: true,  path: '/Applications/Cursor.app' },
  { id: 'zed',         name: 'Zed',         version: '0.148', found: false, path: null },
  { id: 'continue',    name: 'Continue',    version: null,    found: false, path: null },
];

export const MOCK_FOLDERS: ScanFolder[] = [
  { id: 'f1', path: '~/code/lumen',     note: 'monorepo root, 3 packages' },
  { id: 'f2', path: '~/code/brand-kit', note: 'docs + tokens' },
];

export const MOCK_SCAN_EVENTS: ScanEvent[] = [
  { t: 0,    level: 'info',    msg: 'scan started · 2 roots · 2 workers' },
  { t: 120,  level: 'discover', msg: '~/code/lumen · found git repo' },
  { t: 180,  level: 'discover', msg: '~/code/lumen/lumen-app · found git repo', parent: 'lumen' },
  { t: 240,  level: 'discover', msg: '~/code/lumen/lumen-canvas · found git repo', parent: 'lumen' },
  { t: 310,  level: 'discover', msg: '~/code/lumen/lumen-shell · found git repo', parent: 'lumen' },
  { t: 380,  level: 'queue',   msg: 'lumen-app · 842 files queued' },
  { t: 420,  level: 'queue',   msg: 'lumen-canvas · 614 files queued' },
  { t: 470,  level: 'queue',   msg: 'lumen-shell · 291 files queued' },
  { t: 880,  level: 'discover', msg: '~/code/brand-kit/brand-tokens · found git repo', parent: 'brand-kit' },
  { t: 940,  level: 'discover', msg: '~/code/brand-kit/brand-docs · found git repo', parent: 'brand-kit' },
  { t: 1080, level: 'process', msg: 'lumen-app · 842 / 842 processed · graph extracted' },
  { t: 1200, level: 'info',    msg: '2 projects detected · 5 repos · 1,821 files indexed' },
  { t: 1260, level: 'success', msg: 'scan complete · 12s' },
];

export const MOCK_PROJECTS: DiscoveredProject[] = [
  {
    id: 'lumen-studio', name: 'Lumen Studio', kanji: '工',
    path: '~/code/lumen', autoDetected: true, confidence: 'high',
    repos: [
      { id: 'lumen-app',    name: 'lumen-app',    path: '~/code/lumen/lumen-app',    files: 842, lang: 'TypeScript', suggestedRole: 'frontend' },
      { id: 'lumen-canvas', name: 'lumen-canvas', path: '~/code/lumen/lumen-canvas', files: 614, lang: 'Rust + TS',  suggestedRole: 'library' },
      { id: 'lumen-shell',  name: 'lumen-shell',  path: '~/code/lumen/lumen-shell',  files: 291, lang: 'Rust',       suggestedRole: 'infra' },
    ],
  },
  {
    id: 'brand-kit', name: 'Brand Kit', kanji: '紋',
    path: '~/code/brand-kit', autoDetected: true, confidence: 'medium',
    repos: [
      { id: 'brand-tokens', name: 'brand-tokens', path: '~/code/brand-kit/brand-tokens', files: 128, lang: 'JSON + TS', suggestedRole: 'library' },
      { id: 'brand-docs',   name: 'brand-docs',   path: '~/code/brand-kit/brand-docs',   files: 76,  lang: 'Markdown',  suggestedRole: 'docs' },
    ],
  },
];

export const MOCK_LIBRARIES: DiscoveredLibrary[] = [
  { id: 'axum',  name: 'axum',  version: '0.7.5',  lang: 'Rust',       usage: 42, source: 'Cargo.toml', docs: 'indexed', why: 'widely used but no MCP available · sensei wraps' },
  { id: 'yjs',   name: 'yjs',   version: '13.6.8', lang: 'TypeScript', usage: 18, source: 'package.json', docs: 'partial', why: 'CRDT lib · no MCP · sensei wraps' },
  { id: 'sqlx',  name: 'sqlx',  version: '0.7.3',  lang: 'Rust',       usage: 21, source: 'Cargo.toml', docs: 'indexed', why: 'used with postgres-mcp · sensei indexes Rust-side API' },
];

export const MOCK_MCPS: McpEntry[] = [
  { id: 'postgres-mcp', name: 'PostgreSQL MCP', publisher: 'supabase',     kind: 'data',    kanji: '庫', summary: 'Query schema, introspect tables, explain plans.', trigger: ['PostgreSQL'], tools: 14, verified: true,  installed: true,  recommended: true },
  { id: 'redis-mcp',    name: 'Redis MCP',      publisher: 'redis labs',   kind: 'data',    kanji: '速', summary: 'Inspect keys, check TTLs, run diagnostics.',      trigger: ['Redis'],      tools: 9,  verified: true,  installed: false, recommended: true },
  { id: 'stripe-mcp',   name: 'Stripe MCP',     publisher: 'stripe',       kind: 'api',     kanji: '銀', summary: 'List prices, inspect customers, dry-run webhooks.', trigger: ['Stripe'],    tools: 18, verified: true,  installed: false, recommended: true },
  { id: 'github-mcp',   name: 'GitHub MCP',     publisher: 'github',       kind: 'devtool', kanji: '貢', summary: 'Search code, list PRs, read issues, check CI.',    trigger: ['GitHub'],    tools: 23, verified: true,  installed: false, recommended: true },
  { id: 'sentry-mcp',   name: 'Sentry MCP',     publisher: 'sentry.io',    kind: 'service', kanji: '哨', summary: 'Pull recent errors, inspect stack frames.',        trigger: [],            tools: 11, verified: true,  installed: false, recommended: false },
  { id: 'playwright-mcp', name: 'Playwright MCP', publisher: 'microsoft', kind: 'devtool', kanji: '試', summary: 'Run browser tests, capture screenshots.',           trigger: [],            tools: 7,  verified: true,  installed: false, recommended: false },
];

export const MOCK_STACK = {
  languages:  ['Rust', 'TypeScript'],
  frameworks: ['axum', 'sqlx', 'React', 'Tauri'],
  runtimes:   ['tokio 1.36', 'Node 20'],
  services:   ['PostgreSQL', 'Redis', 'Stripe', 'GitHub'],
};

/** Empty state for production — populated from daemon on mount. */
export function createEmptyState(): WizardState {
  return {
    acps: {},
    acpList: [],
    folders: [],
    scanStarted: false,
    scanDone: false,
    scanTick: 0,
    scanEvents: [],
    projects: [],
    roles: {},
    libraries: {},
    libExtras: [],
    mcps: {},
    detectedStack: { languages: [], frameworks: [], runtimes: [], services: [] },
  };
}

/** Full mock state for when daemon is unreachable. */
export function createMockState(): WizardState {
  return {
    acps: Object.fromEntries(MOCK_ACPS.map(a => [a.id, a.found])),
    acpList: MOCK_ACPS,
    folders: [...MOCK_FOLDERS],
    scanStarted: false,
    scanDone: false,
    scanTick: 0,
    scanEvents: [],
    projects: MOCK_PROJECTS.map(p => ({ ...p, confirmed: true })),
    roles: Object.fromEntries(
      MOCK_PROJECTS.flatMap(p => p.repos.map(r => [r.id, r.suggestedRole]))
    ),
    libraries: Object.fromEntries(MOCK_LIBRARIES.map(l => [l.id, true])),
    libExtras: [],
    mcps: Object.fromEntries(MOCK_MCPS.map(m => [m.id, m.installed || m.recommended])),
    detectedStack: MOCK_STACK,
  };
}
