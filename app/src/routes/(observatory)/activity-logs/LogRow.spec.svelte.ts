// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { LogRow } from '$lib/types.js';
import LogRowHarness from './LogRow.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const NOW = Date.parse('2026-07-13T02:19:29.612354+00:00');

function makeRow(over: Partial<LogRow> = {}): LogRow {
  return {
    id: over.id ?? 'r1',
    level: over.level ?? 'info',
    source: over.source ?? null,
    module: 'module' in over ? (over.module ?? null) : 'tasks',
    logged_at: over.logged_at ?? '2026-07-13T02:19:29.612354+00:00',
    message: over.message ?? 'task_completed',
    context: over.context ?? { folder: 'Developer' },
    data: over.data ?? null,
    error: over.error ?? null,
  };
}

describe('LogRow', () => {
  it('renders level, module, message and a relative time', () => {
    const m = mountComponent(LogRowHarness, {
      row: makeRow({ level: 'warn', message: 'scan_slow' }),
      now: NOW + 120_000,
    });
    cleanup.push(m.destroy);
    const t = m.container.textContent ?? '';
    expect(t).toContain('warn');
    expect(t).toContain('tasks');
    expect(t).toContain('scan_slow');
    expect(t).toContain('2m ago');
  });

  it('renders an em dash when source is null', () => {
    const m = mountComponent(LogRowHarness, { row: makeRow({ source: null }), now: NOW });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-log-row-header]')?.textContent).toContain('—');
  });

  it('applies the severity token for the level chip', () => {
    const m = mountComponent(LogRowHarness, { row: makeRow({ level: 'error' }), now: NOW });
    cleanup.push(m.destroy);
    const chip = m.container.querySelector('[data-log-level-chip]') as HTMLElement;
    expect(chip.className).toContain('text-error');
    expect(chip.className).toContain('bg-error-soft');
  });

  it('shows the payload only when expanded, error section first', () => {
    const row = makeRow({ error: { message: 'boom' }, data: { n: 1 } });
    const collapsed = mountComponent(LogRowHarness, { row, now: NOW, expanded: false });
    cleanup.push(collapsed.destroy);
    expect(collapsed.container.querySelector('[data-log-payload]')).toBeNull();

    const expanded = mountComponent(LogRowHarness, { row, now: NOW, expanded: true });
    cleanup.push(expanded.destroy);
    const payload = expanded.container.querySelector('[data-log-payload]');
    expect(payload).not.toBeNull();
    const sections = [...expanded.container.querySelectorAll('[data-log-section]')].map((el) =>
      el.getAttribute('data-log-section'),
    );
    expect(sections[0]).toBe('error');
    expect(payload?.textContent).toContain('boom');
  });

  it('fires onToggle when a row with payload is clicked', () => {
    const onToggle = vi.fn();
    const m = mountComponent(LogRowHarness, { row: makeRow(), now: NOW, onToggle });
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-log-row-header]') as HTMLButtonElement).click();
    expect(onToggle).toHaveBeenCalledOnce();
  });

  it('disables the header (no expand affordance) when there is no payload', () => {
    const m = mountComponent(LogRowHarness, {
      row: makeRow({ context: {}, data: null, error: null }),
      now: NOW,
    });
    cleanup.push(m.destroy);
    const header = m.container.querySelector('[data-log-row-header]') as HTMLButtonElement;
    expect(header.disabled).toBe(true);
  });
});
