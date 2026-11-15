// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Takeaways from './Takeaways.svelte';
import type { Takeaway } from './takeaways';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const ITEMS: Takeaway[] = [
  { title: 'Break 2 cycles', detail: 'Break these first.', tone: 'warning', items: [{ text: 'senseid/api ⇄ senseid/db ⇄ senseid/tasks', value: 'cut senseid/api → senseid/db (3)', target: 'senseid/api', paths: ['senseid/api', 'senseid/db', 'senseid/tasks'], join: '⇄' }] },
  { title: '1 upward dependency', detail: 'Invert it.', tone: 'danger', items: [{ text: 'p/c → p/a', value: '9', target: 'p/c', paths: ['p/c', 'p/a'], join: '→' }] },
];

function render(props: Record<string, unknown> = {}) {
  const onfocus = vi.fn();
  target = document.body.appendChild(document.createElement('div'));
  app = mount(Takeaways, { target, props: { items: ITEMS, onfocus, label: 'Cycles', ...props } });
  flushSync();
  return onfocus;
}

// The user (2026-11-02): the four sections could not be selected and copied — they
// were List buttons. They are text now: a section per takeaway, open by default,
// the modules as text with an open action beside each, and a Copy action.
describe('Takeaways', () => {
  it('a section per takeaway, every one open: title, why, and the modules, as text', () => {
    render();
    const sections = [...target.querySelectorAll('details[data-takeaway]')] as HTMLDetailsElement[];
    expect(sections.map((s) => s.querySelector('summary')?.textContent?.trim())).toEqual(['Break 2 cycles', '1 upward dependency']);
    expect(sections.every((s) => s.open)).toBe(true);
    expect(target.textContent).toContain('Invert it.');
    // The text is not inside a button, so a selection copies it.
    const item = [...target.querySelectorAll('[data-takeaway-item]')].find((li) => li.textContent?.includes('p/c'))!;
    expect(item.querySelector('[data-takeaway-text]')?.closest('button')).toBeNull();
    expect(item.textContent).toContain('9');
  });

  it("each module opens from the action beside it", () => {
    const onfocus = render();
    const open = target.querySelector('[data-takeaway-item] button[aria-label="Focus p/c"]') as HTMLButtonElement
      ?? [...target.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Focus p/c') as HTMLButtonElement;
    open.click();
    expect(onfocus).toHaveBeenCalledWith('p/c');
  });

  it('Copy puts what to act on on the clipboard as markdown, and says so', async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.assign(navigator, { clipboard: { writeText } });
    render();
    (target.querySelector('[data-testid="codemap-takeaways-copy"]') as HTMLButtonElement).click();
    await Promise.resolve();
    flushSync();
    expect(writeText).toHaveBeenCalledOnce();
    const md = writeText.mock.calls[0][0] as string;
    expect(md.startsWith('## What to act on · Cycles')).toBe(true);
    expect(md).toContain('- p/c → p/a — 9');
    expect(target.querySelector('[data-testid="codemap-takeaways-copy"]')?.textContent).toContain('Copied');
  });

  it('a clean view says there is nothing to fix, and offers nothing to copy', () => {
    render({ items: [] });
    expect(target.textContent).toContain('Nothing flagged on this view.');
    expect(target.querySelector('[data-testid="codemap-takeaways-copy"]')).toBeNull();
  });

  // The user (2026-11-03): the panel was unreadable — 12px mono, paths broken
  // mid-word, a cycle as one long line. Each module reads as its name with its
  // folder dimmed, a cycle lists its members, the number sits on its own line,
  // and the action is a labelled button.
  it('each module reads as its name with its folder dimmed; a cycle lists its members; the number has its own line', () => {
    render();
    const cycle = target.querySelector('[data-takeaway-item]') as HTMLElement;
    const names = [...cycle.querySelectorAll('[data-path-name]')].map((n) => n.textContent);
    expect(names).toEqual(['api', 'db', 'tasks']);
    expect(cycle.querySelector('[data-path-folder]')?.textContent).toBe('senseid/');
    expect(cycle.querySelectorAll('[data-path]').length).toBe(3);
    expect(cycle.querySelector('[data-takeaway-value]')?.textContent).toBe('cut senseid/api → senseid/db (3)');
    const focus = cycle.querySelector('button[aria-label="Focus senseid/api"]') as HTMLButtonElement;
    expect(focus.textContent?.trim()).toBe('Focus');
    expect(target.querySelector('[data-takeaway-text].break-all, .break-all')).toBeNull();
  });

  // Review r1 F4: the name is what a reader scans for, so a long path gives up
  // its folder, never its name.
  it('a long path truncates its folder and keeps its name whole', () => {
    render();
    const path = target.querySelector('[data-path]') as HTMLElement;
    expect(path.querySelector('[data-path-name]')?.classList.contains('shrink-0')).toBe(true);
    expect(path.querySelector('[data-path-folder]')?.classList.contains('truncate')).toBe(true);
  });
});
