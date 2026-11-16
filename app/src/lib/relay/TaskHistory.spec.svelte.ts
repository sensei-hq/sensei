// @vitest-environment jsdom
// A task's history on rokkit's Timeline (catalogue rule 1; CONTRACT §1e.6): one dot per
// entry on one line, each dot in its entry's tone; receipts fold into their message.
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import { historyTone } from './relay-timeline.js';
import TaskHistory from './TaskHistory.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});
function mount() {
  const m = mountComponent(TaskHistory, { history: F.history(), lanes: F.laneRefs() });
  cleanup.push(m.destroy);
  return m.container.querySelector('[data-relay-section="history"]') as HTMLElement;
}

describe('TaskHistory', () => {
  it('draws the history on rokkit’s Timeline with plain dots', () => {
    const timeline = mount().querySelector('[data-timeline]');
    expect(timeline?.getAttribute('data-marker')).toBe('dot');
  });

  it('one item per entry, receipts folded into their message', () => {
    const entries = F.history().filter((a) => a.kind !== 'delivered' && a.kind !== 'read');
    const items = mount().querySelectorAll('[data-timeline-item]');
    expect(items).toHaveLength(entries.length);
    items.forEach((item, i) => {
      expect(item.querySelector(`[data-history-entry="${entries[i].id}"]`), entries[i].id).not.toBeNull();
    });
  });

  it('each dot is in its entry’s tone', () => {
    const root = mount();
    for (const action of F.history().filter((a) => a.kind !== 'delivered' && a.kind !== 'read')) {
      const item = root.querySelector(`[data-history-entry="${action.id}"]`)!.closest('[data-timeline-item]');
      expect(item?.getAttribute('data-tone'), action.id).toBe(historyTone(action));
    }
  });

  it('an entry draws no node of its own: the Timeline’s dot is the only one', () => {
    expect(mount().querySelector('[data-entry-node]')).toBeNull();
  });
});
