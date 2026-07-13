import { describe, it, expect, vi } from 'vitest';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { PublishBatchOutcome, ShareReviewItem } from '$lib/types.js';
import {
  PUBLISH_CONFIRM_THRESHOLD,
  PublishBatchAction,
  dereferenceLabel,
  destinationChips,
  heldCount,
  isShippable,
  isUnbound,
  nextBatchLabel,
  outcomeSummary,
  partitionItems,
  publishButtonLabel,
  publishResultChip,
  requiresConfirm,
  shareStateChip,
  shippableCount,
} from './share-review-state.svelte.js';

const item = (over: Partial<ShareReviewItem> = {}): ShareReviewItem => ({
  memory_id: 'm1',
  type: 'principle',
  title: 'prefer small functions',
  body: 'keep units testable',
  attribution: { mode: 'anonymous', dereferenced: false },
  will_dereference: false,
  state: 'queued',
  ...over,
});

const outcome = (over: Partial<PublishBatchOutcome> = {}): PublishBatchOutcome => ({
  batch_id: 'b1',
  published: 2,
  held: 1,
  queued: 0,
  errored: 0,
  already_sent: 0,
  items: [],
  ...over,
});

describe('isShippable', () => {
  it('queued ships', () => expect(isShippable(item({ state: 'queued' }))).toBe(true));
  it('held does not ship', () => expect(isShippable(item({ state: 'held' }))).toBe(false));
  it('an unknown state does not ship', () =>
    expect(isShippable(item({ state: 'weird' }))).toBe(false));
});

describe('partitionItems', () => {
  it('splits queued from held, preserving order within each group', () => {
    const items = [
      item({ memory_id: 'a', state: 'queued' }),
      item({ memory_id: 'b', state: 'held' }),
      item({ memory_id: 'c', state: 'queued' }),
    ];
    const { shippable, held } = partitionItems(items);
    expect(shippable.map((i) => i.memory_id)).toEqual(['a', 'c']);
    expect(held.map((i) => i.memory_id)).toEqual(['b']);
  });
  it('handles an empty batch', () => {
    expect(partitionItems([])).toEqual({ shippable: [], held: [] });
  });
});

describe('shippableCount / heldCount', () => {
  const items = [
    item({ state: 'queued' }),
    item({ state: 'queued' }),
    item({ state: 'held' }),
  ];
  it('counts queued items', () => expect(shippableCount(items)).toBe(2));
  it('counts held items as the remainder', () => expect(heldCount(items)).toBe(1));
});

describe('shareStateChip', () => {
  it('queued reads info', () =>
    expect(shareStateChip('queued')).toEqual({
      bg: 'bg-info-soft',
      text: 'text-info',
      label: 'queued',
    }));
  it('held reads warning', () =>
    expect(shareStateChip('held')).toEqual({
      bg: 'bg-warning-soft',
      text: 'text-warning',
      label: 'held',
    }));
  it('degrades an unexpected state to muted ink, still labelled with the raw value', () =>
    expect(shareStateChip('weird')).toEqual({
      bg: 'bg-paper-mute',
      text: 'text-ink-mute',
      label: 'weird',
    }));
});

describe('dereferenceLabel', () => {
  it('marks client work as source-dropped', () =>
    expect(dereferenceLabel(item({ will_dereference: true }))).toBe('source dropped'));
  it('is null when the source is kept', () =>
    expect(dereferenceLabel(item({ will_dereference: false }))).toBeNull());
});

describe('destinationChips', () => {
  it('trims and drops blanks', () =>
    expect(destinationChips([' acme ', '', 'global'])).toEqual(['acme', 'global']));
  it('is empty for a nullish destination', () => expect(destinationChips(undefined)).toEqual([]));
});

describe('isUnbound', () => {
  it('true when there is no destination', () => expect(isUnbound([])).toBe(true));
  it('true for a nullish destination', () => expect(isUnbound(undefined)).toBe(true));
  it('false once a destination is routed', () => expect(isUnbound(['acme'])).toBe(false));
});

describe('nextBatchLabel', () => {
  it('takes the date portion of an ISO timestamp', () =>
    expect(nextBatchLabel('2026-07-13T12:34:56Z')).toBe('2026-07-13'));
  it('is empty for a missing timestamp', () => expect(nextBatchLabel(undefined)).toBe(''));
  it('is empty for an unparseable value', () => expect(nextBatchLabel('soon')).toBe(''));
});

describe('publishButtonLabel', () => {
  it('names the count when there is something to ship', () =>
    expect(publishButtonLabel(2)).toBe('Publish 2 to Dōjō'));
  it('reads nothing-to-publish at zero', () =>
    expect(publishButtonLabel(0)).toBe('Nothing to publish'));
});

describe('requiresConfirm', () => {
  it('does not require confirmation at or below the threshold', () => {
    expect(requiresConfirm(PUBLISH_CONFIRM_THRESHOLD)).toBe(false);
    expect(requiresConfirm(1)).toBe(false);
  });
  it('requires confirmation above the threshold', () =>
    expect(requiresConfirm(PUBLISH_CONFIRM_THRESHOLD + 1)).toBe(true));
});

describe('publishResultChip', () => {
  it('published reads success', () =>
    expect(publishResultChip('published')).toMatchObject({ text: 'text-success', label: 'published' }));
  it('held_residual_risk reads warning and labels held', () =>
    expect(publishResultChip('held_residual_risk')).toMatchObject({
      text: 'text-warning',
      label: 'held',
    }));
  it('error reads danger', () =>
    expect(publishResultChip('error')).toMatchObject({ text: 'text-danger', label: 'error' }));
  it('queued_retry reads info with a human label', () =>
    expect(publishResultChip('queued_retry')).toMatchObject({
      text: 'text-info',
      label: 'queued for retry',
    }));
  it('no_destination reads warning', () =>
    expect(publishResultChip('no_destination')).toMatchObject({ text: 'text-warning' }));
  it('degrades an unknown result to muted ink echoing the raw tag', () =>
    expect(publishResultChip('teleported')).toEqual({
      bg: 'bg-paper-mute',
      text: 'text-ink-mute',
      label: 'teleported',
    }));
});

describe('outcomeSummary', () => {
  it('joins only the non-zero counters', () =>
    expect(outcomeSummary(outcome({ published: 2, held: 1, queued: 0 }))).toBe(
      '2 published · 1 held',
    ));
  it('reads nothing-published when every counter is zero', () =>
    expect(
      outcomeSummary(
        outcome({ published: 0, held: 0, queued: 0, errored: 0, already_sent: 0 }),
      ),
    ).toBe('nothing published'));
});

// ── PublishBatchAction ────────────────────────────────────────────────────────
// Hand-rolled mock api — only publishBatch is exercised; cast to SenseiApi since
// the controller touches nothing else.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  const ok: ApiResult<PublishBatchOutcome> = { ok: true, data: outcome() };
  return {
    publishBatch: vi.fn().mockResolvedValue(ok),
    ...overrides,
  } as unknown as SenseiApi;
}

describe('PublishBatchAction', () => {
  it('publishes, stores the outcome, reloads, resolves true and clears busy + error', async () => {
    const data = outcome({ published: 3 });
    const publishBatch = vi.fn().mockResolvedValue({ ok: true, data });
    const reload = vi.fn().mockResolvedValue(undefined);
    const actions = new PublishBatchAction(mockApi({ publishBatch }), reload);

    const ok = await actions.publish('b1');
    expect(ok).toBe(true);
    expect(publishBatch).toHaveBeenCalledWith('b1');
    expect(reload).toHaveBeenCalledOnce();
    expect(actions.outcome).toEqual(data);
    expect(actions.busy).toBe(false);
    expect(actions.error).toBeNull();
  });

  it('a failed publish surfaces the wire error, does not reload, resolves false', async () => {
    const publishBatch = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 409, message: "batch not 'approved'" } });
    const reload = vi.fn();
    const actions = new PublishBatchAction(mockApi({ publishBatch }), reload);

    const ok = await actions.publish('b1');
    expect(ok).toBe(false);
    expect(actions.error).toBe("batch not 'approved'");
    expect(actions.outcome).toBeNull();
    expect(reload).not.toHaveBeenCalled();
    expect(actions.busy).toBe(false);
  });

  it('is busy for the duration of the call and refuses a concurrent publish', async () => {
    let resolveCall: (v: ApiResult<PublishBatchOutcome>) => void = () => {};
    const publishBatch = vi.fn().mockReturnValue(
      new Promise<ApiResult<PublishBatchOutcome>>((r) => {
        resolveCall = r;
      }),
    );
    const actions = new PublishBatchAction(mockApi({ publishBatch }), vi.fn());

    const pending = actions.publish('b1');
    expect(actions.busy).toBe(true);
    // a second publish while in flight is refused without a second call
    expect(await actions.publish('b1')).toBe(false);
    expect(publishBatch).toHaveBeenCalledOnce();

    resolveCall({ ok: true, data: outcome() });
    await pending;
    expect(actions.busy).toBe(false);
  });
});
