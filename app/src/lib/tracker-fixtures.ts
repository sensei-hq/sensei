// Test fixtures for the tracker screens' specs (Tickets, Integrations): one copy of
// the wire shapes, so a new field is added once.
import type { MetricReason, ProjectSdlc, SprintSdlc, TicketSdlc, TrackerUsage } from '$lib/types.js';

/** R7 usage: nothing known unless named. */
export const usage = (over: Partial<TrackerUsage> = {}): TrackerUsage => ({
  sessions: 0,
  by_source: { branch: 0, prompt: 0 },
  shared: 0,
  active_secs: null,
  active_n: 0,
  tokens_fresh: null,
  tokens_fresh_n: 0,
  cache_read: null,
  cache_read_n: 0,
  cache_write: null,
  cache_write_n: 0,
  tokens_out: null,
  tokens_out_n: 0,
  tokens_reasoning: null,
  tokens_reasoning_n: 0,
  metered_cost: null,
  metered_cost_n: 0,
  api_cost: null,
  api_cost_n: 0,
  api_cost_partial: 0,
  claude_secs: null,
  claude_n: 0,
  user_secs: null,
  user_n: 0,
  wall_secs: null,
  wall_n: 0,
  first_at: null,
  last_at: null,
  ...over,
});

const TICKET_KEYS = [
  'worked_minutes', 'prompts', 'ai_token_cost', 'cost_per_prompt_cycle', 'escaped_task_defects',
  'compile_defects', 'test_defects', 'review_fixes', 'review_defects', 'total_defects',
  'testing_efficiency', 'review_efficiency', 'build_runs', 'runs_unknown', 'unclassified_failures',
  'build_failure_rate', 'mean_build_recovery_minutes', 'build_unrecovered', 'execution_hours',
  'lead_hours', 'on_hold_minutes', 'on_hold_count', 'on_hold_ratio', 'rework_minutes',
  'rework_cycles', 'post_review_rework_ratio', 'estimated_points', 'expected_minutes',
  'time_variance_minutes', 'effort_cost', 'rework_cost',
] as const;

/** A ticket with no linked sessions: every figure null, each with `no_sessions`. */
export const ticketSdlc = (over: Partial<TicketSdlc> = {}): TicketSdlc => ({
  ...(Object.fromEntries(TICKET_KEYS.map((k) => [k, null])) as Record<(typeof TICKET_KEYS)[number], null>),
  work_started_at: null,
  submitted_at: null,
  submitted_via: null,
  on_hold_by_reason: null,
  reasons: Object.fromEntries(TICKET_KEYS.map((k) => [k, 'no_sessions'])),
  ...over,
});

/** A sprint whose history is not read yet. */
export const sprintSdlc = (over: Partial<SprintSdlc> = {}): SprintSdlc => ({
  commitment_reliability: null,
  total_completion: null,
  scope_change: null,
  carry_over_rate: null,
  unplanned_work_ratio: null,
  velocity: null,
  points_commitment_reliability: null,
  facts: null,
  reasons: Object.fromEntries(
    ['commitment_reliability', 'total_completion', 'scope_change', 'carry_over_rate',
      'unplanned_work_ratio', 'velocity', 'points_commitment_reliability'].map((k) => [k, 'history_pending']),
  ),
  ...over,
});

/** A board with no ticket sessions and too few sprints. */
export const projectSdlc = (over: Partial<ProjectSdlc> = {}): ProjectSdlc => ({
  predictability: null,
  tokens_per_ticket: null,
  prompts_per_ticket: null,
  tickets_with_sessions: 0,
  sessions_without_tokens: 0,
  reasons: { predictability: 'too_few_sprints', tokens_per_ticket: 'no_usage', prompts_per_ticket: 'no_usage' },
  ...over,
});

const word = (code: string, summary: string): MetricReason => ({
  code, kind: 'normal', precedence: 0, summary, detail: '', remedy: null, actor: null,
});

/** The `sdlc_metrics` words the fixtures' codes need. */
export const WORDS: Record<string, MetricReason> = Object.fromEntries(
  [
    word('no_sessions', 'No AI sessions linked'),
    word('history_pending', 'Ticket history not read yet'),
    word('too_few_sprints', 'Too few sprints'),
    word('no_usage', 'No ticket has AI sessions'),
    word('not_derived', 'Signals not derived yet'),
  ].map((w) => [w.code, w]),
);
