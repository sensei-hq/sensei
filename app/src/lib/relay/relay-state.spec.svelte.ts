// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import type { ApiResult } from '$lib/api.js';
import type { RelayCreateRun, RelayCreatedRun } from '$lib/relay-api.js';
import { RelayState } from './relay-state.svelte.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';

// Queue cutover (phase 4): the person picks backlog items and runs a sprint
// from the screen. One write per press; the daemon's rows are read back,
// never invented; a refusal says why and leaves the selection as it was.

const reads = (now = F.now()) => ({ now, nowError: null, queue: F.queue(), queueError: null });

const sprintBody = (): Omit<RelayCreateRun, 'item_ids' | 'crew_id'> => ({
  title: 'Sprint 15: Queue screen',
  goal: 'Run a sprint from the Relay screen',
  repo_key: null,
  workspace_id: null,
  config: {
    base_checkout: '/w/sensei',
    base_branch: 'develop',
    verify_cmd: 'make test-fast',
    auto_merge: true,
  },
});

describe('RelayState · sprint selection', () => {
  it('toggleSelected adds then removes; clearSelection empties', () => {
    const s = new RelayState(reads(), F.fakeRelayApi(), {});
    expect(s.selectedCount).toBe(0);
    s.toggleSelected('item-a');
    s.toggleSelected('item-b');
    expect(s.selectedCount).toBe(2);
    expect(s.isSelected('item-a')).toBe(true);
    s.toggleSelected('item-a');
    expect(s.selectedCount).toBe(1);
    expect(s.isSelected('item-a')).toBe(false);
    s.clearSelection();
    expect(s.selectedCount).toBe(0);
  });

  it("runs and floatingReviews derive from now in the daemon's order; null without a read", () => {
    const s = new RelayState(reads(F.sprintNow()), F.fakeRelayApi(), {});
    expect(s.runs?.map((r) => r.run_id)).toEqual([F.OTHER_RUN_ID, F.RUN_ID]);
    expect(s.floatingReviews?.map((r) => r.task_id)).toEqual(['t-claim-handlers', 't-ddl-deltas']);
    // D83: no read is not an empty band — the screen draws its failure notice instead.
    const bare = new RelayState(
      { now: null, nowError: null, queue: null, queueError: null },
      F.fakeRelayApi(),
      {},
    );
    expect(bare.runs).toBeNull();
    expect(bare.floatingReviews).toBeNull();
  });
});

describe('RelayState · createSprint', () => {
  it('is single-flight, names the crew, re-reads now + queue on success and clears the selection', async () => {
    const api = F.fakeRelayApi();
    const d = F.deferred<ApiResult<RelayCreatedRun>>();
    api.tryCreateRun.mockReturnValue(d.promise);
    const s = new RelayState(reads(), api, {});
    s.toggleSelected('item-a');
    s.toggleSelected('item-b');
    const first = s.createSprint(sprintBody());
    const second = s.createSprint(sprintBody());
    expect(api.tryCreateRun).toHaveBeenCalledTimes(1);
    const body = api.tryCreateRun.mock.calls[0][0];
    expect(body.item_ids).toEqual(['item-a', 'item-b']);
    // The screen's crew, named: the daemon never has to pick among several.
    expect(body.crew_id).toBe(F.CREW_ID);
    expect(body.title).toBe('Sprint 15: Queue screen');
    expect(body.config.auto_merge).toBe(true);
    d.resolve(F.ok({ run_id: F.RUN_ID, planning_task_id: 't-plan', crew_id: F.CREW_ID, status: 'planning' }));
    expect(await first).toBe(F.RUN_ID);
    expect(await second).toBe(F.RUN_ID);
    expect(api.tryGetNow).toHaveBeenCalled();
    expect(api.tryGetQueue).toHaveBeenCalled();
    expect(s.selectedCount).toBe(0);
    expect(s.sprintError).toBeNull();
    expect(s.creatingSprint).toBe(false);
  });

  it('keeps the selection and surfaces the reason on a refusal', async () => {
    const api = F.fakeRelayApi();
    api.tryCreateRun.mockResolvedValue(F.fail(409, 'more than one active crew: name crew_id'));
    const s = new RelayState(reads(), api, {});
    s.toggleSelected('item-a');
    s.creatingSprint = true;
    const run = await s.createSprint(sprintBody());
    expect(run).toBeNull();
    expect(s.sprintError).toContain('more than one active crew');
    expect(s.selectedCount).toBe(1);
    expect(s.creatingSprint).toBe(true);
    expect(api.tryGetNow).not.toHaveBeenCalled();
  });

  it('refuses an empty selection without a call', async () => {
    const api = F.fakeRelayApi();
    const s = new RelayState(reads(), api, {});
    const run = await s.createSprint(sprintBody());
    expect(run).toBeNull();
    expect(s.sprintError).toBeTruthy();
    expect(api.tryCreateRun).not.toHaveBeenCalled();
  });
});
