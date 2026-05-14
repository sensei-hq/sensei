// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Header from './Header.svelte';
import type { Platform, HealthStatus } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Header', () => {
  it.each([
    ['macos',   'macOS'],
    ['linux',   'Linux'],
    ['windows', 'Windows'],
  ] as const)('renders platform label for %s', (platform, label) => {
    const m = mountComponent(Header, { platform: platform as Platform, status: 'checking' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain(label);
  });

  it('renders the "ok" headline', () => {
    const m = mountComponent(Header, { platform: 'macos' as Platform, status: 'ok' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('The foundation');
    expect(m.container.textContent).toContain('holds');
  });

  it('renders the "resolving" headline', () => {
    const m = mountComponent(Header, { platform: 'macos' as Platform, status: 'resolving' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Setting up your.*foundation/);
  });

  it('renders the "needs-action" headline', () => {
    const m = mountComponent(Header, { platform: 'macos' as Platform, status: 'needs-action' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('One last');
  });

  it('renders the "checking" headline', () => {
    const m = mountComponent(Header, { platform: 'macos' as Platform, status: 'checking' as HealthStatus });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toMatch(/Checking the.*foundation/i);
  });
});
