// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import ComplexityLens from './ComplexityLens.svelte';
import type { ComplexityTree } from './types';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const TREE: ComplexityTree = {
  total: { functions: 40, complex: 6, maxCyclo: 41, files: 5, tier: 'warning' },
  maintainability: { value: 0.0048, grade: 'B', label: '4.8 / 1,000 lines', date: '2026-11-01' },
  modules: [{
    id: 'p/a', label: 'p/a', kind: 'module', functions: 30, complex: 6, maxCyclo: 41, files: 3, tier: 'warning',
    children: [{ id: 'a/src', label: 'src', kind: 'folder', functions: 30, complex: 6, maxCyclo: 41, files: 3, tier: 'warning',
      children: [{ id: 'a/src/big.rs', label: 'big.rs', kind: 'file', functions: 12, complex: 5, maxCyclo: 41, files: 1, tier: 'danger', children: [] }] }],
  }],
};

function render(data: ComplexityTree | null = TREE) {
  const onfocus = vi.fn();
  target = document.body.appendChild(document.createElement('div'));
  app = mount(ComplexityLens, { target, props: { data, zoom: 1, onfocus } });
  flushSync();
  return onfocus;
}

// The user (2026-11-02): complexity as a tree, with the numbers at module and
// folder levels and an overall complexity and maintainability rating.
describe('ComplexityLens', () => {
  it('leads with the overall reading: share over cyclomatic 10, highest, files, maintainability grade', () => {
    render();
    const head = target.querySelector('[data-testid="complexity-overall"]')!.textContent!;
    expect(head).toContain('6 of 40 functions over cyclomatic 10');
    expect(head).toContain('15%');
    expect(head).toContain('41');
    expect(target.querySelector('[data-testid="complexity-grade"]')?.textContent?.trim()).toBe('B');
    expect(head).toContain('4.8 / 1,000 lines');
  });

  it('is a rokkit Tree whose rows carry their own numbers', () => {
    render();
    expect(target.querySelector('[data-testid="codemap-complexity"] [data-tree]')).not.toBeNull();
    const row = target.querySelector('[data-complexity-row="p/a"]')!;
    expect(row.textContent).toContain('6 of 30');
    expect(row.textContent).toContain('max 41');
  });

  it('says so when nothing was measured, and when maintainability is not computed', () => {
    render({ ...TREE, total: { functions: 0, complex: 0, maxCyclo: 0, files: 0, tier: null }, modules: [], maintainability: null });
    expect(target.textContent).toContain('Not measured yet');
  });

  // Review r1 F5: under a focus, "not measured yet" is true of the focus only.
  it('under a focus with nothing measured, it names the focus', () => {
    target = document.body.appendChild(document.createElement('div'));
    const empty = { total: { functions: 0, complex: 0, maxCyclo: 0, files: 0, tier: null }, modules: [], maintainability: null };
    app = mount(ComplexityLens, { target, props: { data: empty, zoom: 1, onfocus: vi.fn(), focus: 'senseid/db' } });
    flushSync();
    expect(target.textContent).toContain('senseid/db has no measured files');
  });

  it('a failed read says why and offers Retry, never Reading…', () => {
    const onretry = vi.fn();
    target = document.body.appendChild(document.createElement('div'));
    app = mount(ComplexityLens, { target, props: { data: null, zoom: 1, onfocus: vi.fn(), failed: 'Bad Gateway', onretry } });
    flushSync();
    expect(target.textContent).not.toContain('Reading');
    expect(target.textContent).toContain('Bad Gateway');
    expect([...target.querySelectorAll('button')].some((b) => b.textContent?.trim() === 'Retry')).toBe(true);
  });

  // Review G6: the lens colours a row by the tier the daemon sent, not by its own
  // thresholds — so its red rows are the metric's outliers.
  it('colours a module by the tier it was sent, not its share', () => {
    const tree: ComplexityTree = {
      total: { functions: 10, complex: 3, maxCyclo: 20, files: 1, tier: 'danger' }, maintainability: null,
      modules: [{ id: 'p/z', label: 'p/z', kind: 'module', functions: 10, complex: 3, maxCyclo: 20, files: 1, tier: 'warning', children: [] }],
    };
    render(tree);
    const row = target.querySelector('[data-complexity-row="p/z"]') as HTMLElement;
    expect(row.querySelector('.text-warning')).not.toBeNull();
    expect(row.querySelector('.text-danger')).toBeNull();
  });

  // Review r2 L5: a row with no tier is not drawn red, whatever its share.
  it('a row the daemon gives no tier is not drawn red', () => {
    const tree: ComplexityTree = {
      total: { functions: 10, complex: 9, maxCyclo: 40, files: 1, tier: 'danger' }, maintainability: null,
      modules: [{ id: 'p/n', label: 'p/n', kind: 'module', functions: 10, complex: 9, maxCyclo: 40, files: 1, tier: null, children: [] }],
    };
    render(tree);
    const row = target.querySelector('[data-complexity-row="p/n"]') as HTMLElement;
    expect(row.querySelector('.text-danger')).toBeNull();
    // Review r3 R3-3: no default tier either — it reads as not measured.
    expect(row.querySelector('.text-warning')).toBeNull();
    expect(row.textContent).toContain('not measured');
  });
});
