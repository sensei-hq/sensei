// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import GateRowHarness from './GateRow.harness.svelte';
import type { Component } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

function makeGate(overrides: Partial<Component> = {}): Component {
  return {
    id: 'postgres',
    label: 'PostgreSQL',
    detail: 'storage · @16',
    note: null,
    status: 'pending',
    version: null,
    installingVerb: 'installing',
    description: 'A still pond where memories settle.',
    ...overrides,
  };
}

describe('GateRow', () => {
  it('renders kanji numeral, name, detail, description', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate(), numeral: '二' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('二');
    expect(m.container.textContent).toContain('PostgreSQL');
    expect(m.container.textContent).toContain('storage · @16');
    expect(m.container.textContent).toContain('A still pond where memories settle.');
  });

  it('shows description in italic with ink-soft', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate(), numeral: '二' });
    cleanup.push(m.destroy);
    const desc = m.container.querySelector('[data-component="gate-row-description"]') as HTMLElement;
    expect(desc.className).toMatch(/\bitalic\b/);
    expect(desc.className).toMatch(/\btext-ink-soft\b/);
  });

  it('numeral is success-colored when status="ready"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'ready' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-success\b/);
  });

  it('numeral is accent-colored when status="failed"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'failed' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-accent\b/);
  });

  it('numeral is muted when status="pending"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'pending' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-ink-faint\b/);
  });

  it('row dims when pending', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'pending' }), numeral: '一' });
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-component="gate-row"]') as HTMLElement;
    expect(row.style.opacity).toBe('0.5');
  });

  it('passes installingVerb as StatusIndicator label when status="installing"', () => {
    const m = mountComponent(GateRowHarness, {
      gate: makeGate({ status: 'installing', installingVerb: 'configuring' }),
      numeral: '五',
    });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('configuring');
  });

  it('shows version when present', () => {
    const m = mountComponent(GateRowHarness, {
      gate: makeGate({ version: '16.4', status: 'ready' }),
      numeral: '二',
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('16.4');
  });
});
