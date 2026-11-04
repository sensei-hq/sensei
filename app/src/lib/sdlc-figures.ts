// SDLC figures as shown: values in their units, or (when null) their reason's words.
// Shared by Tickets (a ticket's group) and Delivery Metrics (sprint and board rows).
// Spec: docs/spec/daemon/tracker-sdlc-metrics.md S6, S8; tracker-delivery-metrics.md D2.
import { compactDuration, compactTokens } from '$lib/sessions-digest.js';
import type { MetricReason, ProjectSdlc, SdlcReasons, SprintSdlc, TicketSdlc } from '$lib/types.js';

/** The reason domain the SDLC figures' codes live in (tracker-sdlc-metrics S6). */
export const SDLC_REASONS = 'sdlc_metrics';

/** Reason codes' words, by code. */
export type Words = Record<string, MetricReason>;

/** One SDLC figure as shown: its value, or (when null) its reason's summary. */
export interface SdlcFigure {
  key: string;
  label: string;
  text: string;
  /** The text is a reason's summary, not a value. */
  reason: boolean;
}

export interface SdlcRow {
  id: 'cost' | 'defects' | 'timing' | 'rework';
  label: string;
  figures: SdlcFigure[];
  note: string | null;
}

const pct = (x: number) => `${Math.round(x)}%`;
const money = (x: number) => `$${x.toFixed(2)}`;
const count = (x: number) => String(x);
const points = (x: number) => `${Number(x.toFixed(1))} pts`;
const hoursText = (h: number) => compactDuration(Math.round(h * 60));
const minutesText = (m: number) => compactDuration(Math.round(m));

/** A figure's value, or its reason's summary. A code the vocabulary has no words for
 *  is a fault and says so, never passing for "no data". */
function figure(
  key: string,
  label: string,
  value: number | null,
  format: (x: number) => string,
  reasons: SdlcReasons,
  words: Words,
): SdlcFigure {
  if (value != null) return { key, label, text: format(value), reason: false };
  const code = reasons[key];
  if (code == null) return { key, label, text: '—', reason: false };
  return { key, label, text: reasonText(code, words), reason: true };
}

/** A reason code's summary, for a figure that is not one of a row's (a sprint's
 *  missing replay). A code with no words says so. */
export function reasonText(code: string, words: Words): string {
  return words[code]?.summary ?? 'Reason not recognised';
}

/** A ticket's SDLC group (S8): cost, defects, timing, rework. */
export function ticketSdlcRows(s: TicketSdlc, words: Words): SdlcRow[] {
  const f = (key: keyof TicketSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, s[key] as number | null, format, s.reasons, words);
  const unknown = s.runs_unknown ?? 0;
  // While runs are unknown every defect count is a lower bound (S6): an unknown run can
  // hide a build or test failure, an escape needs one, and review defects include escapes.
  const atLeast = (x: number) => (unknown > 0 ? `≥ ${x}` : String(x));
  const holds = (m: number) => minutesText(m) + (s.on_hold_count ? ` ×${s.on_hold_count}` : '');
  return [
    {
      id: 'cost',
      label: 'Cost',
      figures: [
        f('ai_token_cost', 'AI token cost', money),
        f('cost_per_prompt_cycle', 'per prompt cycle', money),
        f('effort_cost', 'effort cost', money),
      ],
      note: null,
    },
    {
      id: 'defects',
      label: 'Defects',
      figures: [
        f('compile_defects', 'compile', atLeast),
        f('test_defects', 'test', atLeast),
        f('review_defects', 'review', atLeast),
        f('escaped_task_defects', 'escaped task', atLeast),
      ],
      note: unknown > 0 ? `${unknown} run${unknown === 1 ? '' : 's'} with unknown outcome` : null,
    },
    {
      id: 'timing',
      label: 'Timing',
      figures: [
        f('execution_hours', 'execution time', hoursText),
        f('lead_hours', 'lead time', hoursText),
        f('on_hold_minutes', 'on hold', holds),
        f('on_hold_ratio', 'on-hold ratio', pct),
      ],
      note: null,
    },
    {
      id: 'rework',
      label: 'Rework',
      figures: [
        f('rework_minutes', 'rework time', minutesText),
        f('rework_cycles', 'rework cycles', count),
        f('post_review_rework_ratio', 'post-review rework', pct),
      ],
      note: null,
    },
  ];
}

/** A sprint's seven figures (S6): six percents, and velocity in points. */
export function sprintSdlcFigures(s: SprintSdlc, words: Words): SdlcFigure[] {
  const f = (key: keyof SprintSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, s[key] as number | null, format, s.reasons, words);
  return [
    f('commitment_reliability', 'commitment reliability', pct),
    f('total_completion', 'total completion', pct),
    f('scope_change', 'scope change', pct),
    f('carry_over_rate', 'carry-over', pct),
    f('unplanned_work_ratio', 'unplanned work', pct),
    f('velocity', 'velocity', points),
    f('points_commitment_reliability', 'points commitment', pct),
  ];
}

/** A board's project figures (S6). */
export function projectSdlcFigures(p: ProjectSdlc, words: Words): SdlcFigure[] {
  const f = (key: keyof ProjectSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, p[key] as number | null, format, p.reasons, words);
  return [
    f('predictability', 'predictability', points),
    f('tokens_per_ticket', 'tokens per ticket', compactTokens),
    f('prompts_per_ticket', 'prompts per ticket', (x) => x.toFixed(1)),
  ];
}

const SPRINT_WINDOW_REASONS: Record<string, string> = {
  not_started: 'Not started. Usage counts from the sprint’s start.',
  no_dates: 'Jira has no dates for this sprint, so its sessions can’t be placed in it.',
};

/** Why a sprint has no usage window (R7 `reason`), in words; an unknown code as itself. */
export function sprintWindowReason(code: string): string {
  return SPRINT_WINDOW_REASONS[code] ?? code;
}
