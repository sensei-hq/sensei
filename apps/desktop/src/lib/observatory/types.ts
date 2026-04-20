// ─── Observatory Types ──────────────────────────────────────────────────────
// Data shapes for the session observatory. Components render these types.
// Load functions return them — dummy today, daemon API tomorrow.

// ─── Capability ─────────────────────────────────────────────────────────────

export type DataQuality = 'exact' | 'estimated' | 'unavailable';

export interface MetricValue<T = number> {
  value: T;
  quality: DataQuality;
  trackingUrl?: string;   // link to upstream FR when unavailable
  hint?: string;          // "Estimated from turn count"
}

// ─── Scope ──────────────────────────────────────────────────────────────────

export type ProjectState = 'active' | 'recent' | 'inactive' | 'archived';
export type SourceType = 'git' | 'unmanaged' | 'connector';

export interface SolutionSummary {
  id: string;
  name: string;
  description?: string;
  projects: ProjectSummaryRef[];
  state: ProjectState;
  metrics: ScopeMetrics;
}

export interface ProjectSummaryRef {
  id: string;
  name: string;
  role: string;
  sourceType: SourceType;
  state: ProjectState;
  indexedAt?: string;
  lastSessionAt?: string;
}

/** Metrics that exist at both solution and project scope */
export interface ScopeMetrics {
  period: { label: string; from: string; to: string };
  ftr: MetricValue;
  sessionCount: MetricValue;
  reworkRate: MetricValue;
  tokens: MetricValue;
  cost: MetricValue;
  toolAdherence: { mcp: number; fallback: number; total: number };
}

// ─── Global Overview ────────────────────────────────────────────────────────

export interface OverviewData {
  solutions: SolutionSummary[];
  globalMetrics: ScopeMetrics;
  quota: MetricValue<{ usedPct: number; daysRemaining: number | null }>;
}

// ─── Solution Dashboard ─────────────────────────────────────────────────────

export interface SolutionDashboardData {
  solution: SolutionSummary;
  metrics: ScopeMetrics;
  recentSessions: SessionSummary[];
  perProjectMetrics: Array<{ project: ProjectSummaryRef; metrics: ScopeMetrics }>;
}

// ─── Project Dashboard ──────────────────────────────────────────────────────

export interface ProjectDashboardData {
  projectId: string;
  projectName: string;
  solutionId: string;
  solutionName: string;
  metrics: ScopeMetrics;
  recentSessions: SessionSummary[];
  activeTask: { issue?: string; task?: string; phase?: string } | null;
}

// ─── Legacy alias (used by existing home page, will migrate) ────────────────

export interface DashboardData {
  period: { label: string; from: string; to: string };
  ftr: MetricValue;
  sessionCount: MetricValue;
  reworkRate: MetricValue;
  tokens: MetricValue;
  cost: MetricValue;
  toolAdherence: { mcp: number; fallback: number; total: number };
  recentSessions: SessionSummary[];
  quota: MetricValue<{ usedPct: number; daysRemaining: number | null }>;
  activeTask: { issue?: string; task?: string; phase?: string } | null;
}

// ─── Sessions ───────────────────────────────────────────────────────────────

export interface SessionSummary {
  id: string;
  task: string;
  project: string;
  startedAt: string;
  completedAt?: string;
  outcome: 'completed' | 'partial' | 'blocked' | null;
  ftr: number | null;
  turns: number;
  corrections: number;
  tokens: MetricValue;
  cost: MetricValue;
}

export interface SessionDetail extends SessionSummary {
  events: SessionEvent[];
  profilesApplied: ProfileApplication[];
  rulesChecked: RuleCheck[];
}

export interface SessionEvent {
  id: string;
  timestamp: string;
  type: 'turn' | 'tool_used' | 'revision_requested' | 'phase_transition' | 'mindset_applied' | 'persona_applied' | 'rule_checked';
  data: Record<string, unknown>;
  // Enriched fields (populated when available)
  toolName?: string;
  toolParams?: Record<string, unknown>;
  toolResponse?: MetricValue<string>;  // exact if cached, unavailable if not
  classification?: string;             // new_request, correction, continuation
  isMcp?: boolean;
}

export interface ProfileApplication {
  name: string;
  category: 'mindset' | 'persona';
  applied: boolean;
  appliedAt?: string;
}

export interface RuleCheck {
  rule: string;
  adhered: boolean;
  detail?: string;
}

// ─── Projects ───────────────────────────────────────────────────────────────

export interface ProjectOverview {
  repoId: string;
  name: string;
  path: string;
  stack: string[];
  indexedAt?: string;
  symbols: { functions: number; types: number };
  edges: number;
  complexityHotspots: ComplexityHotspot[];
  deadCodeCandidates: DeadCodeCandidate[];
  duplicates: DuplicateGroup[];
  docDrift: DocDriftItem[];
}

export interface ComplexityHotspot {
  name: string;
  file: string;
  line: number;
  complexity: number;
  action: ActionRecipe;
}

export interface DeadCodeCandidate {
  name: string;
  file: string;
  line: number;
  kind: string;
  callerCount: number;
  action: ActionRecipe;
}

export interface DuplicateGroup {
  signature: string;
  instances: Array<{ name: string; file: string; line: number }>;
  action: ActionRecipe;
}

export interface DocDriftItem {
  docPath: string;
  changedTarget: string;
  edgeType: string;
  action: ActionRecipe;
}

// ─── Action Recipes ─────────────────────────────────────────────────────────
// Every insight has an action. This is the shape.

export interface ActionRecipe {
  label: string;              // "Investigate dead code"
  prompt: string;             // clipboard prompt for Claude Code
  severity: 'info' | 'warning' | 'error';
}

// ─── Libraries ──────────────────────────────────────────────────────────────

export interface LibraryOverview {
  name: string;
  sectionCount: number;
  indexedAt: string;
  staleDays: number;
  isStale: boolean;
  usedInSessions: number;
  repos: string[];
}

export interface LibraryDetail {
  name: string;
  sections: Array<{ title: string; component: string; summary: string }>;
  indexedAt: string;
}

// ─── Tools ──────────────────────────────────────────────────────────────────

export interface ToolInfo {
  name: string;
  description: string;
  params: string[];
  usageCount: number;
  errorCount: number;
  lastUsed?: string;
}

export interface ToolSimulation {
  tool: string;
  params: Record<string, string>;
  response: unknown;
  durationMs: number;
}

// ─── Profiles (mindsets, personas, rules) ────────────────────────────────────

export interface ProfileLever {
  name: string;
  category: 'mindset' | 'persona' | 'rule' | 'library';
  type?: string;                         // core, specialist (mindsets)
  sessionsApplied: number;
  ftrImpact: MetricValue;               // +15% FTR
  tokenImpact: MetricValue;             // +8% tokens
  verdict: 'keep' | 'review' | 'unused' | 'remove';
  verdictReason: string;
}

export interface ProfileSuggestion {
  type: 'add_persona' | 'add_mindset' | 'remove_lever' | 'adjust_rule';
  reason: string;                        // "60% of corrections about UX"
  action: ActionRecipe;
}

export interface ProfilesData {
  levers: ProfileLever[];
  suggestions: ProfileSuggestion[];
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

export interface BenchmarkTask {
  id: string;
  name: string;
  prompt: string;
  expectedOutcome: string;
}

export interface BenchmarkRun {
  id: string;
  task: BenchmarkTask;
  config: string;                        // "with-sensei" | "baseline"
  outcome: 'completed' | 'partial' | 'failed';
  turns: number;
  corrections: number;
  tokens: MetricValue;
  durationSeconds: number;
}

export interface BenchmarkComparison {
  task: BenchmarkTask;
  baseline: BenchmarkRun | null;
  withSensei: BenchmarkRun | null;
  improvement: { ftr: number; turns: number; tokens: number } | null;
}
