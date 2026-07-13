// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { SenseiApi } from '$lib/api.js';
import type { PublishBatchOutcome, ShareReviewBatch, ShareReviewItem } from '$lib/types.js';
import { PublishBatchAction } from './share-review-state.svelte.js';
import ShareReviewScreenHarness from './ShareReviewScreen.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const shareItem = (over: Partial<ShareReviewItem> = {}): ShareReviewItem => ({
  memory_id: 'm1',
  type: 'principle',
  title: 'prefer small functions',
  body: 'keep units testable',
  attribution: { mode: 'anonymous', dereferenced: false },
  will_dereference: false,
  state: 'queued',
  ...over,
});

const batchWith = (
  items: ShareReviewItem[],
  over: Partial<ShareReviewBatch> = {},
): ShareReviewBatch => ({
  batch_id: 'batch-1',
  destination: ['acme'],
  cadence: 'manual',
  items,
  ...over,
});

/** A PublishBatchAction over a hand-rolled mock api. */
function action(
  publishBatch = vi.fn().mockResolvedValue({ ok: true, data: undefined }),
  reload = vi.fn().mockResolvedValue(undefined),
): PublishBatchAction {
  return new PublishBatchAction({ publishBatch } as unknown as SenseiApi, reload);
}

describe('ShareReviewScreen — batch present', () => {
  const batch = batchWith([
    shareItem({
      memory_id: 'a',
      type: 'guard',
      title: 'Validate webhook signatures before parsing',
      state: 'queued',
      will_dereference: true,
      attribution: { mode: 'dereferenced', dereferenced: true },
    }),
    shareItem({
      memory_id: 'b',
      type: 'pattern',
      title: '',
      body: '',
      state: 'held',
    }),
  ]);

  it('renders the destination/cadence policy bar with the ship + held counts', () => {
    const m = mountComponent(ShareReviewScreenHarness, { batch, actions: action() });
    cleanup.push(m.destroy);

    const bar = m.container.querySelector('[data-policy-bar]');
    expect(bar).toBeTruthy();
    expect(m.container.querySelector('[data-destination]')?.textContent).toContain('acme');
    expect(m.container.querySelector('[data-cadence]')?.textContent).toContain('manual');
    expect(m.container.querySelector('[data-shippable-count]')?.textContent).toContain('1 to ship');
    expect(m.container.querySelector('[data-held-count]')?.textContent).toContain('1 held');
  });

  it('renders the shippable item with its title, attribution and dereference note', () => {
    const m = mountComponent(ShareReviewScreenHarness, { batch, actions: action() });
    cleanup.push(m.destroy);

    const row = m.container.querySelector('[data-share-item="a"]');
    expect(row).toBeTruthy();
    expect(row?.getAttribute('data-state')).toBe('queued');
    expect(row?.textContent).toContain('Validate webhook signatures before parsing');
    expect(row?.textContent).toContain('source dropped'); // will_dereference
    expect(row?.textContent).toContain('source withheld'); // dereferenced attribution
  });

  it('shows held items in a separate held section, never as publishable', () => {
    const m = mountComponent(ShareReviewScreenHarness, { batch, actions: action() });
    cleanup.push(m.destroy);

    const held = m.container.querySelector('[data-held-section]');
    expect(held).toBeTruthy();
    const heldRow = m.container.querySelector('[data-share-item="b"]');
    expect(heldRow?.getAttribute('data-state')).toBe('held');
    expect(heldRow?.textContent).toContain("won't ship this batch");
    // no per-item publish affordance exists anywhere on the screen
    expect(m.container.querySelector('[data-share-item="b"] button')).toBeNull();
  });

  it('offers a single Publish action labelled with the shippable count', () => {
    const m = mountComponent(ShareReviewScreenHarness, { batch, actions: action() });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('[data-action="publish"]');
    expect(btn?.textContent).toContain('Publish 1 to Dōjō');
    expect((btn as HTMLButtonElement).disabled).toBe(false);
  });
});

describe('ShareReviewScreen — publish', () => {
  it('publishes the batch, reloads, and reflects the post-publish outcome', async () => {
    const publishedOutcome: PublishBatchOutcome = {
      batch_id: 'batch-1',
      published: 2,
      held: 1,
      queued: 0,
      errored: 0,
      already_sent: 0,
      items: [
        { memory_id: 'a', kind: 'guard', result: 'published', seq: 1, remote_id: 'r-1' },
        { memory_id: 'b', kind: 'pattern', result: 'held_residual_risk' },
      ],
    };
    const publishBatch = vi.fn().mockResolvedValue({ ok: true, data: publishedOutcome });
    const reload = vi.fn().mockResolvedValue(undefined);
    const actions = action(publishBatch, reload);

    const batch = batchWith([
      shareItem({ memory_id: 'a', state: 'queued' }),
      shareItem({ memory_id: 'b', state: 'queued' }),
    ]);
    const m = mountComponent(ShareReviewScreenHarness, { batch, actions });
    cleanup.push(m.destroy);

    (m.container.querySelector('[data-action="publish"]') as HTMLButtonElement).click();

    // reload runs at the end of the success path (after the outcome is stored),
    // so waiting on it guarantees the whole publish resolved.
    await vi.waitFor(() => expect(reload).toHaveBeenCalledOnce());
    expect(publishBatch).toHaveBeenCalledWith('batch-1');

    await vi.waitFor(() => expect(m.container.querySelector('[data-outcome]')).toBeTruthy());
    expect(m.container.querySelector('[data-outcome-summary]')?.textContent).toContain('2 published');
    expect(m.container.querySelectorAll('[data-outcome-item]').length).toBe(2);
  });
});

describe('ShareReviewScreen — empty', () => {
  it('renders the empty state cleanly when there is no pending batch', () => {
    const m = mountComponent(ShareReviewScreenHarness, { batch: null, actions: action() });
    cleanup.push(m.destroy);

    expect(m.container.querySelector('[data-empty]')).toBeTruthy();
    expect(m.container.textContent).toContain('nothing queued to share');
    expect(m.container.querySelector('[data-policy-bar]')).toBeNull();
    expect(m.container.querySelector('[data-action="publish"]')).toBeNull();
  });
});
