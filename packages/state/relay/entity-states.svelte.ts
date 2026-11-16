// The relay domain's entity states (#421; design 2026-11-12 §2b): one
// ReactiveStageContext per entity the stream names, each keyed by `id`. A row the
// daemon keys otherwise (a run by `run_id`, a task by `task_id`, a tracker ticket
// with no item yet) gets its `id` here, and loses it again when the screen's
// `relay.now` shape is assembled (`relay-state.svelte.ts`).

import { ReactiveStageContext } from '../stage.svelte.js';
import type {
  RelayBacklogItem,
  RelayCompletedTask,
  RelayDecision,
  RelayFloatingReview,
  RelayLane,
  RelayPersonMessage,
  RelayRunSummary,
  RelayTriageItem,
} from './types.js';

/** An open run (sprint), by its id. */
export type RelayRunRow = RelayRunSummary & { id: string };

/** A task the crew view lists outside a lane: a report waiting for a reviewer, or one done today. */
export type RelayTaskRow =
  | (RelayFloatingReview & { id: string; stage: 'floating' })
  | (RelayCompletedTask & { id: string; stage: 'completed' });

/** A queue row: triage (a tracker ticket or a manual item) or the backlog. */
export type RelayItemRow =
  | (RelayTriageItem & { id: string; stage: 'triage' })
  | (RelayBacklogItem & { id: string; stage: 'backlog' });

export class LanesState extends ReactiveStageContext<RelayLane> {}
export class DecisionsState extends ReactiveStageContext<RelayDecision> {}
export class MessagesState extends ReactiveStageContext<RelayPersonMessage> {}

export class RunsState extends ReactiveStageContext<RelayRunRow> {
  static row(run: RelayRunSummary): RelayRunRow {
    return { ...run, id: run.run_id };
  }
}

export class TasksState extends ReactiveStageContext<RelayTaskRow> {
  static rows(floating: RelayFloatingReview[], completed: RelayCompletedTask[]): RelayTaskRow[] {
    return [
      ...floating.map((t) => ({ ...t, id: t.task_id, stage: 'floating' as const })),
      ...completed.map((t) => ({ ...t, id: t.task_id, stage: 'completed' as const })),
    ];
  }

  get floating(): RelayFloatingReview[] {
    return this.items.filter((t) => t.stage === 'floating').map(({ id: _id, stage: _stage, ...t }) => t as RelayFloatingReview);
  }

  get completed(): RelayCompletedTask[] {
    return this.items.filter((t) => t.stage === 'completed').map(({ id: _id, stage: _stage, ...t }) => t as RelayCompletedTask);
  }
}

export class ItemsState extends ReactiveStageContext<RelayItemRow> {
  /** A tracker ticket has no item until it is added to the backlog: it is keyed by its tracker and key. */
  static triageId(item: RelayTriageItem): string {
    return item.item_id ?? `${item.ticket.provider}:${item.ticket.key}`;
  }

  static rows(triage: RelayTriageItem[], backlog: RelayBacklogItem[]): RelayItemRow[] {
    return [
      ...triage.map((t) => ({ ...t, id: ItemsState.triageId(t), stage: 'triage' as const }) as RelayItemRow),
      ...backlog.map((b) => ({ ...b, id: b.item_id, stage: 'backlog' as const })),
    ];
  }

  get triage(): RelayTriageItem[] {
    return this.items.filter((i) => i.stage === 'triage').map(({ id: _id, stage: _stage, ...t }) => t as RelayTriageItem);
  }

  get backlog(): RelayBacklogItem[] {
    return this.items.filter((i) => i.stage === 'backlog').map(({ id: _id, stage: _stage, ...b }) => b as RelayBacklogItem);
  }
}

/** A run row without the `id` this state keyed it by: the `relay.now` shape. */
export const runSummary = ({ id: _id, ...run }: RelayRunRow): RelayRunSummary => run;
