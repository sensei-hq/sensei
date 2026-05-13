// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Remedy from './Remedy.svelte';
import type { Remedy as RemedyT } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const fixture = (over: Partial<RemedyT> = {}): RemedyT => ({
  message: 'Run the script in your terminal.',
  script: 'brew bundle --file=https://example/Brewfile',
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

  it('Copy button calls onCopyScript', () => {
    const onCopyScript = vi.fn();
    const m = mountComponent(Remedy, { remedy: fixture(), onCopyScript });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="copy"]') as HTMLButtonElement).click();
    expect(onCopyScript).toHaveBeenCalledTimes(1);
  });

  it('Recheck button calls onRecheck', () => {
    const onRecheck = vi.fn();
    const m = mountComponent(Remedy, { remedy: fixture(), onRecheck });
    cleanup.push(m.destroy);
    (m.container.querySelector('button[data-action="recheck"]') as HTMLButtonElement).click();
    expect(onRecheck).toHaveBeenCalledTimes(1);
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
