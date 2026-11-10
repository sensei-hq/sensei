// Relay · one task's state (D90; CONTRACT §1e, `relay.task_history`).
//
// A task is read by (crew, task): another crew's task, or another user's, is a
// 404 and the screen says "not found" — nothing broke, and Retry would ask the
// same question again. Everything else that fails is a fault with a Retry.
import { failureReason, type ApiError } from '$lib/api.js';
import type { RelayApi, RelayDecision, RelayTaskStep, RelayTaskView } from '$lib/relay-api.js';

export interface TaskReads {
  view: RelayTaskView | null;
  error: ApiError | null;
}

export class RelayTaskState {
  readonly api: RelayApi;
  readonly crewId: string;
  readonly taskId: string;

  view = $state<RelayTaskView | null>(null);
  error = $state<ApiError | null>(null);
  reading = $state(false);

  #inFlight: Promise<void> | null = null;

  constructor(reads: TaskReads, api: RelayApi, crewId: string, taskId: string) {
    this.api = api;
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

  /** A streamed step lands directly (F8): new keys append, restated keys
   *  update in place. Anything for another task, or before the view reads,
   *  is ignored — the stream is crew-wide, the state is one task's. */
  applyStep(taskId: string, step: RelayTaskStep): void {
    if (taskId !== this.taskId || !this.view) return;
    const at = this.view.steps.findIndex((s) => s.key === step.key);
    if (at === -1) this.view.steps.push(step);
    else this.view.steps[at] = step;
  }

  /** A streamed task notification (F8): re-read, but only when the task is
   *  this one. (Op-only events like unregister re-read through `reRead`,
   *  routed by the route — they name no task.) */
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
