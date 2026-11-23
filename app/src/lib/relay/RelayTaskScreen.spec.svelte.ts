// @vitest-environment jsdom
// The task view's open decision (#389): when the view re-reads to a different open
// decision, the card is drawn afresh for it, never the previous decision's form and values.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync, tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { RelayTaskState, type RelayTaskSource } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import RelayTaskScreen from './RelayTaskScreen.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const settle = async () => {
  flushSync();
  await tick();
  await Promise.resolve();
  flushSync();
};

function source(): RelayTaskSource {
  return {
    readTask: vi.fn(async () => ({ data: F.taskView(), error: null })),
    answer: vi.fn(async () => ({ data: { action_id: 'a1' }, error: null })),
    events: { subscribe: () => () => {} },
  } as unknown as RelayTaskSource;
}

const names = (c: HTMLElement) =>
  [...c.querySelectorAll('[data-decision-field]')].map((f) => f.getAttribute('data-decision-field'));

describe('RelayTaskScreen · the open decision', () => {
  it('a different open decision draws its own form, not the last one', async () => {
    const b = F.formDecision({ id: 'ask-b', question: 'Where does it ship?' });
    const a = F.formDecision({
      id: 'ask-a',
      question: 'Where do the logs go?',
      response: { schema: { type: 'object', properties: { path: { type: 'string' } }, required: ['path'] } },
    });
    const state = new RelayTaskState({ view: F.taskView({ decision: b }), error: null }, source(), 'c1', 't-header');
    const m = mountComponent(RelayTaskScreen as never, {
      state,
      viewId: 'relay-task',
      backHref: '/relay',
      context: 'Relay',
    } as never);
    cleanup.push(m.destroy);
    await settle();
    expect(names(m.container)).toEqual(['target', 'max_retries', 'ship']);

    state.view = F.taskView({ decision: a });
    await settle();
    expect(m.container.querySelector('[data-decision]')?.getAttribute('data-decision')).toBe('ask-a');
    expect(names(m.container)).toEqual(['path']);
  });
});
