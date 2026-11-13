// The task view's selectors (#425; CONTRACT §1e; COMPONENTS §3d TaskHeader, RunDetails,
// HandoffCard): from the task's read to what each part draws. Pure. A number nobody
// reported is null, and the part says "not reported", never 0 (D83).

import { hhmm, providerLabel, roundsLabel, taskStateLabel } from './format.js';
import { assistantLabel, type RelayTone } from './now-selectors.js';
import type { RelayRunDetails, RelayTaskView } from './types.js';

export interface RelayTaskHeaderView {
  back: { label: string; href: string };
  title: string;
  state: { label: string; tone: RelayTone };
  ticket: { provider: string; key: string; url: string | null } | null;
  facts: Array<{ k: string; v: string }>;
}

export interface RelayRunDetailsView {
  /** The folded line: tokens in, time working, subagents. */
  line: string;
  rows: Array<{ k: string; v: string | null }>;
  /** How many of the task's turns reported. */
  note: string;
}

export interface RelayHandoffView {
  label: string;
  brief: string;
  docs: string[];
}

export interface RelayTaskPartsView {
  header: RelayTaskHeaderView;
  run: RelayRunDetailsView;
  handoff: RelayHandoffView | null;
}

const STATE_TONE: Record<string, RelayTone> = {
  done: 'success',
  approved: 'success',
  working: 'accent',
  assigned: 'accent',
  in_review: 'warning',
  rework: 'warning',
  blocked: 'danger',
  interrupted: 'danger',
  cancelled: 'neutral',
  ready: 'neutral',
};

/** "2 h 22 min", "38 min". */
function duration(secs: number): string {
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins} min`;
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return m === 0 ? `${h} h` : `${h} h ${m} min`;
}

/** "3.1 M", "112 k", "640". */
function count(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, '')} M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)} k`;
  return String(n);
}

const num = (n: number | null) => (n == null ? null : new Intl.NumberFormat('en').format(n));

function runRows(r: RelayRunDetails): Array<{ k: string; v: string | null }> {
  return [
    { k: 'Repository', v: r.repository },
    { k: 'Branch', v: r.branch },
    { k: 'Commits', v: r.commits?.join(', ') ?? null },
    { k: 'Model', v: r.model },
    { k: 'Tokens in', v: num(r.tokens_in) },
    { k: 'Tokens out', v: num(r.tokens_out) },
    { k: 'Tokens cached', v: num(r.tokens_cache) },
    { k: 'Peak context', v: num(r.context_tokens) },
    { k: 'Compactions', v: num(r.compactions) },
    { k: 'Peak memory', v: r.peak_memory_mb == null ? null : `${r.peak_memory_mb} MB` },
    { k: 'Subagents', v: num(r.subagents) },
    { k: 'Working', v: r.working_secs == null ? null : duration(r.working_secs) },
    { k: 'Waiting for review', v: r.waiting_secs == null ? null : duration(r.waiting_secs) },
    {
      k: 'Files changed',
      v: r.files_changed == null ? null : `${r.files_changed}${r.files_outside_owned ? ` · ${r.files_outside_owned} outside owned paths` : ''}`,
    },
    { k: 'Gates run', v: num(r.gates_run) },
  ];
}

function runLine(r: RelayRunDetails): string {
  const parts: string[] = [];
  if (r.tokens_in != null) parts.push(`${count(r.tokens_in)} tokens in`);
  if (r.working_secs != null) parts.push(`${duration(r.working_secs)} working`);
  if (r.subagents != null) parts.push(`${r.subagents} ${r.subagents === 1 ? 'subagent' : 'subagents'}`);
  return parts.length ? parts.join(' · ') : 'not reported yet';
}

/** The task view's header, Run details and Handed off. */
export function toTaskView(view: RelayTaskView, o: { back: { label: string; href: string } }): RelayTaskPartsView {
  const t = view.task;
  const lane = t.lane;
  const start = t.assigned_at;
  const end = t.completed_at;
  const span =
    start && end ? ` · ${duration((new Date(end).getTime() - new Date(start).getTime()) / 1000)}` : '';
  const time = start ? `${hhmm(start)} → ${end ? hhmm(end) : 'now'}${span}` : 'unclaimed';
  return {
    header: {
      back: o.back,
      title: t.title,
      state: { label: taskStateLabel(t.state), tone: STATE_TONE[t.state] ?? 'neutral' },
      ticket: t.ticket ? { provider: providerLabel(t.ticket.provider), key: t.ticket.key, url: t.ticket.url ?? null } : null,
      facts: [
        { k: 'Lane', v: lane ? `${lane.title ?? lane.code} · ${lane.code}` : 'in the pool' },
        { k: 'Assistant', v: lane ? `${assistantLabel(lane.family, lane.version)} · ${lane.model ?? 'model not reported'}` : 'not reported' },
        { k: 'Time', v: time },
        { k: 'Rounds', v: roundsLabel(t.rounds) || 'none yet' },
      ],
    },
    run: {
      line: runLine(view.run),
      rows: runRows(view.run),
      note: `Totals over the task's turns · ${view.run.reported_turns} of ${view.run.turns} reported`,
    },
    handoff: view.handoff
      ? { label: `Handed off · ${hhmm(view.handoff.at)}`, brief: view.handoff.body_md, docs: view.handoff.docs }
      : null,
  };
}
