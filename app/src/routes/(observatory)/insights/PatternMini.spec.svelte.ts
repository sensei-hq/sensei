// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import PatternMiniHarness from './PatternMini.harness.svelte';
import type { PatternCardVm } from './insights-board.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const pattern = (over: Partial<PatternCardVm> = {}): PatternCardVm => ({
  id: 'pat1',
  name: 'rule-candidates',
  family: null,
  instanceCount: 8,
  lifecycle: 'suggested',
  ...over,
});

describe('PatternMini', () => {
  it('renders a suggested pattern as a candidate to promote (warning edge)', () => {
    const m = mountComponent(PatternMiniHarness, { pattern: pattern() });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="pattern-mini"]');
    expect(el?.getAttribute('data-lifecycle')).toBe('suggested');
    expect(m.container.textContent).toContain('rule-candidates');
    expect(m.container.textContent).toContain('candidate to promote');
    expect(el?.className).toMatch(/border-l-warning/);
  });

  it('renders a rule pattern as an adopted rule (success edge)', () => {
    const m = mountComponent(PatternMiniHarness, { pattern: pattern({ lifecycle: 'rule' }) });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="pattern-mini"]');
    expect(m.container.textContent).toContain('adopted rule');
    expect(el?.className).toMatch(/border-l-success/);
  });
});
