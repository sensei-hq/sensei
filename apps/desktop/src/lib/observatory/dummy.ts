// ─── Dummy Data Factory ─────────────────────────────────────────────────────
// Realistic hand-crafted data for all observatory pages.
// Swap these for real API calls in load functions when wiring.

import type {
  DashboardData, SessionSummary, SessionDetail, SessionEvent,
  ProjectOverview, ComplexityHotspot, DeadCodeCandidate, DuplicateGroup, DocDriftItem,
  LibraryOverview, ToolInfo, ProfilesData, ProfileLever, ProfileSuggestion,
  BenchmarkComparison, BenchmarkTask, BenchmarkRun,
  MetricValue,
} from './types.js';

// ─── Helpers ────────────────────────────────────────────────────────────────

const exact = <T = number>(value: T): MetricValue<T> => ({ value, quality: 'exact' });
const estimated = <T = number>(value: T, hint: string): MetricValue<T> => ({ value, quality: 'estimated', hint });
const unavailable = <T = number>(url: string): MetricValue<T> => ({ value: 0 as T, quality: 'unavailable', trackingUrl: url });

const TOKEN_FR = 'https://github.com/anthropics/claude-code/issues/11008';
const QUOTA_FR = 'https://github.com/anthropics/claude-code/issues/50926';

// ─── Dashboard ──────────────────────────────────────────────────────────────

export function dashboardDummy(): DashboardData {
  return {
    period: { label: 'This Week', from: '2026-04-14', to: '2026-04-19' },
    ftr: exact(0.83),
    sessionCount: exact(12),
    reworkRate: exact(0.18),
    tokens: estimated(142000, 'Estimated from turn count'),
    cost: unavailable(TOKEN_FR),
    toolAdherence: { mcp: 34, fallback: 4, total: 38 },
    recentSessions: recentSessionsDummy(),
    quota: unavailable(QUOTA_FR),
    activeTask: { issue: '#97', task: 'Daemon metrics computation', phase: 'build' },
  };
}

function recentSessionsDummy(): SessionSummary[] {
  return [
    {
      id: 's-001', task: 'Daemon metrics (#97)', project: 'sensei-dev',
      startedAt: '2026-04-19T09:00:00Z', completedAt: '2026-04-19T09:12:00Z',
      outcome: 'completed', ftr: 1.0, turns: 8, corrections: 0,
      tokens: estimated(24000, 'Estimated'), cost: unavailable(TOKEN_FR),
    },
    {
      id: 's-002', task: 'Retire skills (#93)', project: 'sensei-dev',
      startedAt: '2026-04-19T09:15:00Z', completedAt: '2026-04-19T09:22:00Z',
      outcome: 'completed', ftr: 1.0, turns: 5, corrections: 0,
      tokens: estimated(16000, 'Estimated'), cost: unavailable(TOKEN_FR),
    },
    {
      id: 's-003', task: 'Mindsets & personas (#95)', project: 'sensei-dev',
      startedAt: '2026-04-19T08:00:00Z', completedAt: '2026-04-19T08:35:00Z',
      outcome: 'completed', ftr: 0.5, turns: 14, corrections: 3,
      tokens: estimated(52000, 'Estimated'), cost: unavailable(TOKEN_FR),
    },
    {
      id: 's-004', task: 'IR class edges (#96)', project: 'sensei-dev',
      startedAt: '2026-04-19T07:30:00Z', completedAt: '2026-04-19T07:42:00Z',
      outcome: 'completed', ftr: 1.0, turns: 6, corrections: 0,
      tokens: estimated(20000, 'Estimated'), cost: unavailable(TOKEN_FR),
    },
    {
      id: 's-005', task: 'Plugin hook debug', project: 'sensei-dev',
      startedAt: '2026-04-18T16:00:00Z',
      outcome: 'blocked', ftr: 0.0, turns: 3, corrections: 1,
      tokens: estimated(8000, 'Estimated'), cost: unavailable(TOKEN_FR),
    },
  ];
}

// ─── Session Detail ─────────────────────────────────────────────────────────

export function sessionDetailDummy(id: string): SessionDetail {
  const summary = recentSessionsDummy().find(s => s.id === id) ?? recentSessionsDummy()[0];
  return {
    ...summary,
    events: sessionEventsDummy(),
    profilesApplied: [
      { name: 'Analyst', category: 'mindset', applied: true, appliedAt: '2026-04-19T09:00:30Z' },
      { name: 'Developer', category: 'mindset', applied: true, appliedAt: '2026-04-19T09:02:00Z' },
      { name: 'BAT', category: 'mindset', applied: true, appliedAt: '2026-04-19T09:10:00Z' },
      { name: 'Security Reviewer', category: 'mindset', applied: false },
      { name: 'AI Driven Developer', category: 'persona', applied: true, appliedAt: '2026-04-19T09:10:30Z' },
      { name: 'API Consumer', category: 'persona', applied: false },
    ],
    rulesChecked: [
      { rule: 'TDD — tests first', adhered: true },
      { rule: 'Zero errors — cargo test before and after', adhered: true },
      { rule: 'MCP preferred over grep', adhered: false, detail: 'Turn 5: used grep instead of search()' },
    ],
  };
}

function sessionEventsDummy(): SessionEvent[] {
  return [
    { id: 'e1', timestamp: '2026-04-19T09:00:00Z', type: 'turn', data: {}, classification: 'new_request' },
    { id: 'e2', timestamp: '2026-04-19T09:01:00Z', type: 'tool_used', data: { tool: 'search', query: 'compute_metrics' }, toolName: 'search', toolParams: { query: 'compute_metrics' }, isMcp: true, toolResponse: exact('Found 3 results: store.rs:924, routes.rs:1702, main.rs:397') },
    { id: 'e3', timestamp: '2026-04-19T09:02:00Z', type: 'tool_used', data: { tool: 'get_callers', name: 'insert_event' }, toolName: 'get_callers', toolParams: { name: 'insert_event' }, isMcp: true, toolResponse: exact('5 callers found') },
    { id: 'e4', timestamp: '2026-04-19T09:03:00Z', type: 'mindset_applied', data: { mindset: 'Developer' } },
    { id: 'e5', timestamp: '2026-04-19T09:04:00Z', type: 'turn', data: {}, classification: 'continuation' },
    { id: 'e6', timestamp: '2026-04-19T09:05:00Z', type: 'tool_used', data: { tool: 'Grep', pattern: 'fn list_events' }, toolName: 'Grep', isMcp: false, toolResponse: { value: '', quality: 'unavailable' as const, hint: 'Non-MCP tool — response not captured' } },
    { id: 'e7', timestamp: '2026-04-19T09:06:00Z', type: 'revision_requested', data: { reason: 'Use MCP search instead of Grep' } },
    { id: 'e8', timestamp: '2026-04-19T09:07:00Z', type: 'tool_used', data: { tool: 'search', query: 'list_events' }, toolName: 'search', isMcp: true, toolResponse: exact('Found 2 results: store.rs:880, routes.rs:1689') },
    { id: 'e9', timestamp: '2026-04-19T09:10:00Z', type: 'turn', data: {}, classification: 'new_request' },
    { id: 'e10', timestamp: '2026-04-19T09:11:00Z', type: 'rule_checked', data: { rule: 'Zero errors', adhered: true } },
    { id: 'e11', timestamp: '2026-04-19T09:12:00Z', type: 'turn', data: {}, classification: 'continuation' },
  ];
}

// ─── Projects ───────────────────────────────────────────────────────────────

export function projectOverviewDummy(repoId: string): ProjectOverview {
  return {
    repoId,
    name: 'sensei-dev',
    path: '/Users/Jerry/Developer/sensei-dev',
    stack: ['rust', 'typescript', 'svelte'],
    indexedAt: '2026-04-19T08:00:00Z',
    symbols: { functions: 486, types: 124 },
    edges: 1847,
    complexityHotspots: [
      { name: 'mcp_call_tool', file: 'crates/senseid/src/api/routes.rs', line: 1149, complexity: 42, action: { label: 'Investigate complexity', prompt: 'The function mcp_call_tool in crates/senseid/src/api/routes.rs has cyclomatic complexity 42. Analyze and suggest how to decompose it.', severity: 'warning' } },
      { name: 'process_file', file: 'crates/senseid/src/tasks/handlers.rs', line: 298, complexity: 28, action: { label: 'Review handler', prompt: 'Review process_file handler in tasks/handlers.rs (complexity 28) — can the match arms be extracted into separate functions?', severity: 'info' } },
    ],
    deadCodeCandidates: [
      { name: 'get_call_flow', file: 'crates/senseid/src/indexer/graph.rs', line: 688, kind: 'function', callerCount: 0, action: { label: 'Check if used', prompt: 'The function get_call_flow in graph.rs has 0 callers. Is it dead code or called via HTTP route? Investigate and remove if unused.', severity: 'info' } },
    ],
    duplicates: [
      { signature: 'fn ftrClass(ftr: number) -> string', instances: [{ name: 'ftrClass', file: 'routes/(app)/p/[id]/+page.svelte', line: 21 }, { name: 'ftrClass', file: 'routes/(app)/s/[id]/+page.svelte', line: 130 }, { name: 'ftrClass', file: 'routes/(app)/s/[id]/sessions/+page.svelte', line: 22 }], action: { label: 'Extract to $lib', prompt: 'Extract the duplicated ftrClass function from 3 page components into $lib/utils.ts and update all imports.', severity: 'warning' } },
    ],
    docDrift: [
      { docPath: 'docs/design/01-daemon/architecture.md', changedTarget: 'crates/senseid/src/api/routes.rs', edgeType: 'COVERS', action: { label: 'Review doc', prompt: 'The architecture doc at docs/design/01-daemon/architecture.md references routes.rs which has been modified. Check if the doc is still accurate.', severity: 'info' } },
    ],
  };
}

// ─── Libraries ──────────────────────────────────────────────────────────────

export function librariesDummy(): LibraryOverview[] {
  return [
    { name: 'rokkit', sectionCount: 8, indexedAt: '2026-04-15T10:00:00Z', staleDays: 4, isStale: false, usedInSessions: 6, repos: ['sensei-dev'] },
    { name: 'kavach', sectionCount: 5, indexedAt: '2026-03-01T10:00:00Z', staleDays: 49, isStale: true, usedInSessions: 2, repos: ['sensei-dev'] },
    { name: 'hono', sectionCount: 12, indexedAt: '2026-04-10T10:00:00Z', staleDays: 9, isStale: false, usedInSessions: 0, repos: [] },
  ];
}

// ─── Tools ──────────────────────────────────────────────────────────────────

export function toolsDummy(): ToolInfo[] {
  return [
    { name: 'search', description: 'Find functions and types by name', params: ['query', 'repoId'], usageCount: 34, errorCount: 0, lastUsed: '2026-04-19T09:07:00Z' },
    { name: 'get_callers', description: 'Who calls this function?', params: ['name', 'repoId'], usageCount: 18, errorCount: 0, lastUsed: '2026-04-19T09:02:00Z' },
    { name: 'get_callees', description: 'What does this function call?', params: ['name', 'repoId'], usageCount: 12, errorCount: 0, lastUsed: '2026-04-18T16:00:00Z' },
    { name: 'get_patterns', description: 'Find files by framework pattern', params: ['pattern', 'repoId'], usageCount: 8, errorCount: 1, lastUsed: '2026-04-18T14:00:00Z' },
    { name: 'get_lib_docs', description: 'Library documentation', params: ['name', 'component'], usageCount: 6, errorCount: 0, lastUsed: '2026-04-17T10:00:00Z' },
    { name: 'get_metrics', description: 'Project quality metrics', params: ['repoId'], usageCount: 2, errorCount: 0, lastUsed: '2026-04-19T09:12:00Z' },
    { name: 'get_communities', description: 'Architecture clusters', params: ['repoId'], usageCount: 3, errorCount: 0 },
    { name: 'get_project_summary', description: 'Project stats and metadata', params: ['repoId'], usageCount: 12, errorCount: 0 },
    { name: 'search_lib_docs', description: 'Search across all library docs', params: ['query'], usageCount: 4, errorCount: 0 },
    { name: 'get_doc_drift', description: 'Docs out of sync with code', params: ['repoId'], usageCount: 1, errorCount: 0 },
  ];
}

// ─── Profiles ───────────────────────────────────────────────────────────────

export function profilesDummy(): ProfilesData {
  return {
    levers: [
      { name: 'Analyst', category: 'mindset', type: 'core', sessionsApplied: 12, ftrImpact: exact(0.15), tokenImpact: estimated(0.08, 'Estimated'), verdict: 'keep', verdictReason: 'Worth the token cost — catches scope issues early' },
      { name: 'BAT', category: 'mindset', type: 'core', sessionsApplied: 10, ftrImpact: exact(0.22), tokenImpact: estimated(0.12, 'Estimated'), verdict: 'keep', verdictReason: 'Biggest quality gain — catches integration issues' },
      { name: 'Developer', category: 'mindset', type: 'core', sessionsApplied: 12, ftrImpact: exact(0.05), tokenImpact: estimated(0.03, 'Estimated'), verdict: 'keep', verdictReason: 'Low cost, steady quality lift' },
      { name: 'UX Designer', category: 'mindset', type: 'specialist', sessionsApplied: 2, ftrImpact: exact(0.0), tokenImpact: estimated(0.04, 'Estimated'), verdict: 'review', verdictReason: 'Low usage — only relevant for UI tasks' },
      { name: 'Security Reviewer', category: 'mindset', type: 'specialist', sessionsApplied: 0, ftrImpact: exact(0.0), tokenImpact: estimated(0.0, 'Estimated'), verdict: 'unused', verdictReason: 'Never triggered — remove or lower threshold?' },
      { name: 'AI Driven Developer', category: 'persona', sessionsApplied: 5, ftrImpact: exact(0.08), tokenImpact: estimated(0.06, 'Estimated'), verdict: 'keep', verdictReason: 'Validates from user perspective' },
      { name: 'Plugin Developer', category: 'persona', sessionsApplied: 3, ftrImpact: exact(0.04), tokenImpact: estimated(0.05, 'Estimated'), verdict: 'review', verdictReason: 'Marginal impact — review questions' },
      { name: 'TDD rule', category: 'rule', sessionsApplied: 12, ftrImpact: exact(0.18), tokenImpact: estimated(0.15, 'Estimated'), verdict: 'keep', verdictReason: 'High quality lift — tests-first catches bugs early' },
      { name: 'MCP preferred', category: 'rule', sessionsApplied: 12, ftrImpact: exact(0.10), tokenImpact: estimated(-0.05, 'Estimated'), verdict: 'keep', verdictReason: 'Saves tokens AND improves quality' },
      { name: 'rokkit lib docs', category: 'library', sessionsApplied: 6, ftrImpact: exact(0.10), tokenImpact: estimated(-0.05, 'Estimated'), verdict: 'keep', verdictReason: 'Prevents incorrect API usage' },
    ],
    suggestions: [
      { type: 'add_persona', reason: '60% of corrections in last 5 sessions were about missing user perspective on CLI output', action: { label: 'Create CLI User persona', prompt: '/sensei:persona add cli-user', severity: 'info' } },
      { type: 'remove_lever', reason: 'Security Reviewer mindset applied 0 times in 12 sessions', action: { label: 'Remove Security Reviewer', prompt: 'Remove .sensei/mindsets/security-reviewer.md — never triggers for this project. Re-add if you start handling auth or external input.', severity: 'info' } },
    ],
  };
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

export function benchmarksDummy(): BenchmarkComparison[] {
  const task: BenchmarkTask = { id: 'b1', name: 'Fix parser bug', prompt: 'Fix the null pointer exception in src/parser.rs', expectedOutcome: 'Tests pass, no regression' };
  return [
    {
      task,
      baseline: { id: 'r1', task, config: 'baseline', outcome: 'completed', turns: 12, corrections: 3, tokens: estimated(36000, 'Estimated'), durationSeconds: 240 },
      withSensei: { id: 'r2', task, config: 'with-sensei', outcome: 'completed', turns: 7, corrections: 0, tokens: estimated(28000, 'Estimated'), durationSeconds: 150 },
      improvement: { ftr: 0.5, turns: -5, tokens: -8000 },
    },
  ];
}
