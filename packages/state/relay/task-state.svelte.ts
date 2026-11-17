// Relay · one task's state (D90; CONTRACT §1e, `relay.task_history`).
//
// A task is read by (crew, task): another crew's task, or another user's, is a
// 404 and the screen says "not found" — nothing broke, and Retry would ask the
// same question again. Everything else that fails is a fault with a Retry.
import { failureReason, type ApiError } from '../result.js';
import type { RelayApi } from './client.js';
import type { RelayEvents } from './relay-state.svelte.js';
import type { RelayDecision, RelayStreamEvent, RelayTaskStep, RelayTaskView } from './types.js';

export interface TaskReads {
  view: RelayTaskView | null;
  error: ApiError | null;
}

export class RelayTaskState {
  readonly api: RelayApi;
  readonly crewId: string;
  readonly taskId: string;
  /** The crew's stream (#418): the daemon's socket on the desktop, the realtime channel on the dōjō. */
  readonly events: RelayEvents;

  view = $state<RelayTaskView | null>(null);
  error = $state<ApiError | null>(null);
  reading = $state(false);

  #inFlight: Promise<void> | null = null;

  constructor(reads: TaskReads, api: RelayApi, crewId: string, taskId: string, events: RelayEvents) {
    this.api = api;
    this.events = events;
    this.crewId = crewId;
    this.taskId = taskId;
    this.view = reads.view;
    this.error = reads.error;
  }

  /** A task outside this user's crews is not there — a refusal, not a fault. */
  get notFound(): boolean {
    return this.error?.status === 404;
  }

  /** Why the task could not be read, for the fault Notice. */
  get reason(): string | null {
    return this.error ? failureReason(this.error) : null;
  }

  async retry(): Promise<void> {
    if (this.#inFlight) return this.#inFlight;
    this.reading = true;
    this.#inFlight = (async () => {
      try {
        const answer = await this.api.tryGetTask(this.crewId, this.taskId);
        if (answer.ok) {
          this.view = answer.data;
          this.error = null;
        } else {
          this.error = answer.error;
        }
      } finally {
        this.reading = false;
        this.#inFlight = null;
      }
    })();
    return this.#inFlight;
  }

  /** Answer the task's open decision: its reason when refused, else null. */
  async answer(decision: RelayDecision, option: string): Promise<string | null> {
    const answer = await this.api.tryAnswerDecision({
      decision_id: decision.id,
      option,
      note: null,
      surface: 'desktop',
    });
    if (!answer.ok) return failureReason(answer.error);
    await this.reRead();
    return null;
  }

  // ── The stream ──

  /** Subscribe to the crew's stream; returns the teardown (call on destroy). Each open
   *  reads the task: the loader's read predates the first, and a broadcast replays
   *  nothing missed while the stream was down. */
  connect(): () => void {
    return this.events(
      (event) => this.apply(event),
      () => void this.reRead(),
    );
  }

  /** One stream event (F8): a step for this task lands directly, a task event for this
   *  task re-reads it, and an unregister — whose interrupts name no task — re-reads. The
   *  stream is crew-wide; anything else names nothing this state holds. */
  apply(event: RelayStreamEvent): void {
    const { entity, data } = event;
    if (entity === 'step' && data.task_id && data.step) this.applyStep(data.task_id, data.step);
    else if (entity === 'task' && data.task_id) void this.applyTaskChanged(data.task_id);
    else if (entity === 'relay' && data.op === 'unregister') void this.reRead();
  }

  /** A streamed step lands directly (F8): new keys append, restated keys
   *  update in place. Anything for another task, or before the view reads,
   *  is ignored — the stream is crew-wide, the state is one task's. */
  applyStep(taskId: string, step: RelayTaskStep): void {
    if (taskId !== this.taskId || !this.view) return;
    const at = this.view.steps.findIndex((s) => s.key === step.key);
    if (at === -1) this.view.steps.push(step);
    else this.view.steps[at] = step;
  }

  /** A streamed task notification (F8): re-read, but only when the task is this one. */
  async applyTaskChanged(taskId: string): Promise<void> {
    if (taskId !== this.taskId) return;
    await this.reRead();
  }

  /** Read the task again after a write. A failed re-read is a fault, not a no-op:
   *  the view it would have replaced is stale (D83, N8). */
  async reRead(): Promise<void> {
    const answer = await this.api.tryGetTask(this.crewId, this.taskId);
    if (answer.ok) {
      this.view = answer.data;
      this.error = null;
    } else {
      this.error = answer.error;
    }
  }
}
