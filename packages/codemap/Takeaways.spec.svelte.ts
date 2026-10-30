// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Takeaways from './Takeaways.svelte';
import type { Takeaway } from './takeaways';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const ITEMS: Takeaway[] = [
  { title: 'Break 2 cycles', detail: 'Break these first.', tone: 'warning', items: [{ text: 'p/a ⇄ p/b', value: '', target: 'p/a' }] },
  { title: '1 upward dependency', detail: 'Invert it.', tone: 'danger', items: [{ text: 'p/c → p/a', value: '9', target: 'p/c' }] },
];

describe('Takeaways', () => {
  it('an accordion of what to act on: titles without numbers, each opening onto why and the modules', () => {
    const onopen = vi.fn();
    target = document.body.appendChild(document.createElement('div'));
    app = mount(Takeaways, { target, props: { items: ITEMS, onopen } });
    flushSync();
    const groups = [...target.querySelectorAll('[data-list-group]')] as HTMLButtonElement[];
    expect(groups.map((g) => g.querySelector('[data-item-label]')?.textContent?.trim())).toEqual(['Break 2 cycles', '1 upward dependency']);
    expect(target.textContent).not.toContain('Invert it.');
    groups[1].click();
    flushSync();
    expect(target.textContent).toContain('Invert it.');
    const item = [...target.querySelectorAll('[data-list-item]')].find((b) => b.textContent?.includes('p/c → p/a')) as HTMLElement;
    expect(item.textContent).toContain('9');
    item.click();
    flushSync();
    expect(onopen).toHaveBeenCalledWith('p/c');
  });
});
