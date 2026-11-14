// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import QueueRow from './QueueRow.svelte';

// Queue cutover (phase 4): a backlog row can be picked for a sprint — a
// checkbox before the title, only when the screen says the row is
// selectable (triage rows never are).

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount(over: Record<string, unknown> = {}) {
  const m = mountComponent(QueueRow, { id: 'b1', title: 'Billing build', kind: 'feature', ...over });
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-queue-row]') as HTMLElement;
  return { root };
}

describe('QueueRow · selectable', () => {
  it('draws no checkbox by default', () => {
    const { root } = mount();
    expect(root.querySelector('[data-select]')).toBeNull();
  });

  it('a selectable row toggles through its checkbox and reflects the selection', () => {
    const ontoggle = vi.fn();
    const { root } = mount({ selectable: true, selected: true, ontoggle });
    const box = root.querySelector('[data-select]') as HTMLInputElement;
    expect(box.checked).toBe(true);
    expect(root.dataset.selected).toBe('true');
    box.click();
    expect(ontoggle).toHaveBeenCalledTimes(1);
  });
});
