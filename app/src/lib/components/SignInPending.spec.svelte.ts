// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './SignInPending.harness.svelte';

// #300: a sign-in opens the system browser. If that browser is signed in to
// GitHub as someone else, the daemon refuses the callback — so the user must be
// told which account to use, and given the link for a private window.
let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const URL_ = 'https://dojo.test/authorize?x=1';

describe('SignInPending', () => {
  it('names the GitHub account the sign-in expects', () => {
    const m = mountComponent(Harness, { pending: { label: 'sensei-hq-org', expectedLogin: 'sensei-hq-org', url: URL_ } });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('sign in to GitHub as sensei-hq-org');
  });

  it('for a first connect, asks for the account that belongs to this identity', () => {
    const m = mountComponent(Harness, { pending: { label: 'joelthomas8847', expectedLogin: null, url: URL_ } });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('joelthomas8847');
    expect(m.container.textContent).not.toContain('sign in to GitHub as null');
  });

  it('copies the exact link for a private window', async () => {
    const copy = vi.fn(async () => {});
    const m = mountComponent(Harness, { pending: { label: 'x', expectedLogin: 'x', url: URL_ }, copy });
    cleanup.push(m.destroy);
    const button = m.container.querySelector('[data-testid="sign-in-copy-link"]') as HTMLButtonElement;
    button.click();
    await new Promise((r) => setTimeout(r, 0));
    expect(copy).toHaveBeenCalledWith(URL_);
    expect(m.container.textContent).toContain('Copied');
  });

  it('renders nothing when no sign-in is in flight', () => {
    const m = mountComponent(Harness, { pending: null });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="sign-in-pending"]')).toBeNull();
  });
});
