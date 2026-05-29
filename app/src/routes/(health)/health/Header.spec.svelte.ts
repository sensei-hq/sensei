// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Header from './Header.svelte';
import type { HealthStatus } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Header', () => {
  it('renders the Sensei wordmark', () => {
    const m = mountComponent(Header, { status: 'checking' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Sensei');
    // The kanji 先生 lives in the wordmark
    expect(m.container.textContent).toContain('先生');
  });

  it.each([
    ['checking',     'starting'],
    ['resolving',    'setting up'],
    ['needs-action', 'needs your hand'],
    ['ok',           'ready'],
  ] as const)('eyebrow for status=%s is "%s"', (status, eyebrow) => {
    const m = mountComponent(Header, { status: status as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent?.toLowerCase()).toContain(eyebrow);
  });

  it('renders the "ok" headline', () => {
    const m = mountComponent(Header, { status: 'ok' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('The foundation');
    expect(m.container.textContent).toContain('holds');
  });

  it('renders the "resolving" headline', () => {
    const m = mountComponent(Header, { status: 'resolving' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Setting up your.*foundation/);
  });

  it('renders the "needs-action" headline', () => {
    const m = mountComponent(Header, { status: 'needs-action' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('One last');
  });

  it('renders the "checking" headline', () => {
    const m = mountComponent(Header, { status: 'checking' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Checking the.*foundation/i);
  });

  it('renders a sub-copy paragraph beneath the headline', () => {
    // Sub-copy varies per state but every state has one — assert presence.
    const m = mountComponent(Header, { status: 'checking' as HealthStatus });
    cleanup.push(m.destroy);
    const sub = m.container.querySelector('[data-sub]');
    expect(sub).not.toBeNull();
    expect(sub?.textContent?.trim().length).toBeGreaterThan(0);
  });
});
