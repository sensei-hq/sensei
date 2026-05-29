// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Footer from './Footer.svelte';
import type { Platform } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Footer', () => {
  it('renders the sensei version', () => {
    const m = mountComponent(Footer, { version: '0.2.14', platform: 'macos' as Platform });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('sensei');
    expect(m.container.textContent).toContain('0.2.14');
  });

  it.each([
    ['macos',   'macOS'],
    ['linux',   'Linux'],
    ['windows', 'Windows'],
  ] as const)('renders platform label for %s', (platform, label) => {
    const m = mountComponent(Footer, { version: '0.2.14', platform: platform as Platform });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain(label);
  });

  it('renders the dev label when version is empty', () => {
    // Empty version is the cold-load placeholder before any payload arrives.
    // Footer should still render coherently — show "dev" instead of blank.
    const m = mountComponent(Footer, { version: '', platform: 'macos' as Platform });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('dev');
  });
});
