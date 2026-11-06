// SDLC figures as shown: values in their units, or (when null) their reason's words.
// Shared by Tickets (a ticket's group) and Delivery Metrics (sprint and board rows).
// Spec: docs/spec/daemon/tracker-sdlc-metrics.md S6, S8; tracker-delivery-metrics.md D2.
import { compactDuration, compactTokens } from '$lib/sessions-digest.js';
import type { MetricReason, ProjectSdlc, TicketSdlc } from '$lib/types.js';
import {
  SDLC_REASONS,
  points,
  pct,
  reasonText,
  sdlcFigure,
  sprintSdlcFigures,
  type SdlcFigure,
} from '@sensei/sdlc-figures';

export { SDLC_REASONS, reasonText, sprintSdlcFigures, type SdlcFigure };

/** Reason codes' words, by code. */
export type Words = Record<string, MetricReason>;

export interface SdlcRow {
  id: 'cost' | 'defects' | 'timing' | 'rework';
  label: string;
  figures: SdlcFigure[];
  note: string | null;
}

const money = (x: number) => `$${x.toFixed(2)}`;
const count = (x: number) => String(x);
const hoursText = (h: number) => compactDuration(Math.round(h * 60));
const minutesText = (m: number) => compactDuration(Math.round(m));

/** A ticket's SDLC group (S8): cost, defects, timing, rework. */
export function ticketSdlcRows(s: TicketSdlc, words: Words): SdlcRow[] {
  const f = (key: keyof TicketSdlc & string, label: string, format: (x: number) => string) =>
    sdlcFigure(key, label, s[key] as number | null, format, s.reasons, words);
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

/** A board's project figures (S6). */
export function projectSdlcFigures(p: ProjectSdlc, words: Words): SdlcFigure[] {
  const f = (key: keyof ProjectSdlc & string, label: string, format: (x: number) => string) =>
    sdlcFigure(key, label, p[key] as number | null, format, p.reasons, words);
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
