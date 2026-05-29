// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Remedy from './Remedy.svelte';
import type { Remedy as RemedyT } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const fixture = (over: Partial<RemedyT> = {}): RemedyT => ({
  message: 'Run the script in your terminal.',
  script: 'brew install sensei-hq/tap/sensei',
  url: null, ...over,
});

describe('Remedy', () => {
  it('renders message and script verbatim', () => {
    const r = fixture();
    const m = mountComponent(Remedy, { remedy: r });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain(r.message);
    const pre = m.container.querySelector('pre');
    expect(pre?.textContent).toBe(r.script);
  });

  it('Copy button writes the script to clipboard via injected writeText', async () => {
    const writeText = vi.fn(async () => undefined);
    const m = mountComponent(Remedy, { remedy: fixture(), writeText });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('button[data-action="copy"]') as HTMLButtonElement;
    btn.click();
    await tick();
    await tick();
    expect(writeText).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith('brew install sensei-hq/tap/sensei');
  });

  it('Copy button shows "Copied ✓" feedback after a successful write', async () => {
    const writeText = vi.fn(async () => undefined);
    const m = mountComponent(Remedy, { remedy: fixture(), writeText });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('button[data-action="copy"]') as HTMLButtonElement;
    expect(btn.textContent?.trim()).toBe('Copy script');
    btn.click();
    await tick();
    await tick();
    expect(btn.getAttribute('data-state')).toBe('copied');
    expect(btn.textContent?.trim()).toContain('Copied');
  });

  it('Copy button shows "Copy failed" feedback when writeText rejects', async () => {
    const writeText = vi.fn(async () => { throw new Error('clipboard blocked'); });
    const m = mountComponent(Remedy, { remedy: fixture(), writeText });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('button[data-action="copy"]') as HTMLButtonElement;
    btn.click();
    await tick();
    await tick();
    expect(btn.getAttribute('data-state')).toBe('failed');
    expect(btn.textContent?.toLowerCase()).toContain('failed');
  });

  it('Verify button calls onVerify', () => {
    const onVerify = vi.fn();
    const m = mountComponent(Remedy, { remedy: fixture(), onVerify });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="verify"]') as HTMLButtonElement).click();
    expect(onVerify).toHaveBeenCalledTimes(1);
  });

  it('script and message are user-selectable (carry select-text class)', () => {
    // Fallback for users whose webview rejects the clipboard API — they can
    // still highlight the text and copy manually.
    const m = mountComponent(Remedy, { remedy: fixture() });
    cleanup.push(m.destroy);
    const pre = m.container.querySelector('pre');
    expect(pre?.className).toContain('select-text');
    const message = m.container.querySelector('[data-remedy-message]');
    expect(message?.className).toContain('select-text');
  });

  it('renders a link only when remedy.url is non-null', () => {
    const m1 = mountComponent(Remedy, { remedy: fixture({ url: null }) });
    cleanup.push(m1.destroy);
    expect(m1.container.querySelector('a[data-role="remedy-url"]')).toBeNull();

    const m2 = mountComponent(Remedy, {
      remedy: fixture({ url: 'https://brew.sh' }),
    });
    cleanup.push(m2.destroy);
    const a = m2.container.querySelector('a[data-role="remedy-url"]') as HTMLAnchorElement;
    expect(a?.href).toContain('brew.sh');
  });
});
