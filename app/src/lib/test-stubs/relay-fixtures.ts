// Relay · test fixtures. The data lives beside the relay domain states in
// @sensei/state (#421), whose specs use it too; the app's specs import it from here,
// with the spy client below (a package file cannot import vitest).
import { vi } from 'vitest';
import type { RelayApi } from '$lib/relay-api.js';
import { CREW_ID, RUN_ID, now, ok, queue, taskView } from '../../../../packages/state/relay/fixtures.js';

export * from '../../../../packages/state/relay/fixtures.js';

/** A relay client whose every call is a spy; each test sets the answers it needs. */
export type FakeRelayApi = { [K in keyof RelayApi]: ReturnType<typeof vi.fn> & RelayApi[K] };

export function fakeRelayApi(): FakeRelayApi {
  return {
    tryGetNow: vi.fn(async () => ok(now())),
    tryGetQueue: vi.fn(async () => ok(queue())),
    tryGetTask: vi.fn(async () => ok(taskView())),
    trySendMessage: vi.fn(async () => ok({ message_id: 'msg-1' })),
    tryAnswerDecision: vi.fn(async () => ok({ action_id: 'act-answer' })),
    tryAddToBacklog: vi.fn(async () => ok({ item_id: 'item-new' })),
    tryAddTriage: vi.fn(async () => ok({ item_id: 'item-added' })),
    tryCreateRun: vi.fn(async () =>
      ok({ run_id: RUN_ID, planning_task_id: 't-plan', crew_id: CREW_ID, status: 'planning' as const }),
    ),
  } as FakeRelayApi;
}
