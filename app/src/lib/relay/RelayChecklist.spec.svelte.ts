// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { RelayTaskStep } from '$lib/relay-api.js';
import RelayChecklist from './RelayChecklist.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// F8: the steps checklist — completed checked, the current open step
// highlighted, skipped struck with its reason.

const step = (key: string, state: RelayTaskStep['state'], reason: string | null = null): RelayTaskStep => ({
  key,
  title: `Step ${key}`,
  state,
  reason,
  updated_at: '2026-11-06T10:11:00Z',
});

const stepsOf = (c: HTMLElement): HTMLElement[] =>
  [...c.querySelectorAll('[data-step]')] as HTMLElement[];

describe('RelayChecklist', () => {
  it('checks done, highlights the first open as current, strikes skipped with reason', () => {
    const m = mountComponent(RelayChecklist, {
      steps: [step('a', 'done'), step('b', 'open'), step('c', 'skipped', 'Not this slice'), step('d', 'open')],
    });
    cleanup.push(m.destroy);
    const rows = stepsOf(m.container);
    expect(rows).toHaveLength(4);
    expect(rows[0].getAttribute('data-step-state')).toBe('done');
    expect(rows[0].querySelector('[data-step-check]')).not.toBeNull();
    expect(rows[1].getAttribute('data-step-current')).toBe('true');
    expect(rows[3].getAttribute('data-step-current')).toBeNull();
    expect(rows[2].getAttribute('data-step-state')).toBe('skipped');
    expect(rows[2].textContent).toContain('Not this slice');
  });

  it('reads empty honestly when the lane posted nothing', () => {
    const m = mountComponent(RelayChecklist, { steps: [] });
    cleanup.push(m.destroy);
    expect(stepsOf(m.container)).toHaveLength(0);
    expect(m.container.textContent).toContain('No steps posted yet');
  });
});
