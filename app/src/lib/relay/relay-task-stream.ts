// Relay · the task route's socket (F8): transport owned by the route, never
// by shared state. Dispatches the shared {action, entity, data} contract
// into the state updaters — steps apply directly, task notifications
// re-read their task, unregister re-reads (its interrupts name no task),
// everything else is ignored. The dojo replays this same contract over
// Supabase realtime; only this dispatcher's source changes.
import { relayStream } from '$lib/relay-api.js';
import type { RelayTaskState } from '@sensei/state';

/** Subscribe the task route; returns the teardown (call on destroy). */
export function subscribeTaskStream(
  port: number,
  state: RelayTaskState,
  taskId: string,
): () => void {
  // One task's screen hears only its crew (#418); the Now screen spans every crew.
  const stream = relayStream(port, state.crewId);
  // Broadcast has no replay: the first open is the initial connect, later
  // opens are reconnects and re-read. (WebKit may delay `open` until the
  // first message — the covering re-read below makes that harmless.)
  let primed = false;
  stream.onopen = () => {
    if (!primed) {
      primed = true;
      return;
    }
    void state.reRead();
  };
  const unsub = stream.subscribe((event) => {
    if (event.entity === 'step' && event.data.task_id === taskId && event.data.step) {
      state.applyStep(event.data.task_id, event.data.step);
    } else if (event.entity === 'task' && event.data.task_id) {
      void state.applyTaskChanged(event.data.task_id);
    } else if (event.entity === 'relay' && event.data.op === 'unregister') {
      void state.reRead();
    }
  });
  // Cover the loader-to-subscribe gap: the loader read before the socket
  // existed (scan hydrates after subscribing for the same reason).
  void state.reRead();
  return () => {
    unsub();
    stream.destroy();
  };
}
