// @vitest-environment jsdom
//
// The overlay replaced a strip along the title bar. Its job is to show a LIST —
// an install has several personas, each a separate GitHub identity with its own
// credential — and to be absent otherwise.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import SignInOverlay from './SignInOverlay.svelte';
import type { PersonaWire } from '$lib/personas.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const persona = (over: Partial<PersonaWire> = {}): PersonaWire => ({
  label: 'sensei-hq-org',
  githubLogin: 'sensei-hq-org',
  sessionSlot: 'default',
  connected: true,
  forgeToken: { state: 'active', expiresAt: 1_788_204_679 },
  action: 'none',
  ...over
});

const BASE = {
  open: true,
  personas: [persona()],
  actionLabel: () => 'Sign in',
  describe: () => '7.0 hours left'
};

const root = (m: { container: HTMLElement }) =>
  m.container.querySelector('[data-component="sign-in-overlay"]');
const rows = (m: { container: HTMLElement }) =>
  Array.from(m.container.querySelectorAll('[data-persona]'));

describe('SignInOverlay', () => {
  it('renders nothing when closed', () => {
    const m = mountComponent(SignInOverlay, { ...BASE, open: false });
    cleanup.push(m.destroy);
    expect(root(m)).toBeNull();
  });

  it('lists one row per identity', () => {
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [persona(), persona({ label: 'jerrythomas', githubLogin: null, action: 'connect' })]
    });
    cleanup.push(m.destroy);
    expect(rows(m)).toHaveLength(2);
    // The display name falls back to the label when GitHub has never been asked.
    expect(rows(m)[1].textContent).toContain('jerrythomas');
  });

  it('shows "ready" instead of a button for a healthy identity', () => {
    // A button here invites a needless sign-in, which rotates a working
    // credential for nothing.
    const m = mountComponent(SignInOverlay, BASE);
    cleanup.push(m.destroy);
    expect(rows(m)[0].querySelector('button')).toBeNull();
    expect(rows(m)[0].textContent).toContain('ready');
  });

  it('shows the action button only for identities that need one', () => {
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [persona({ action: 'signIn' })],
      actionLabel: () => 'Sign in again'
    });
    cleanup.push(m.destroy);
    const btn = rows(m)[0].querySelector('button') as HTMLButtonElement;
    expect(btn.textContent).toContain('Sign in again');
  });

  it('reports which persona was clicked, not just that a click happened', () => {
    // The overlay is a list. A callback with no argument would leave the caller
    // guessing which identity to sign in.
    const onSignIn = vi.fn();
    const target = persona({ label: 'jerrythomas', action: 'connect' });
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [persona(), target],
      onSignIn
    });
    cleanup.push(m.destroy);
    (rows(m)[1].querySelector('button') as HTMLButtonElement).click();
    expect(onSignIn).toHaveBeenCalledWith(target);
  });

  it('disables only the row whose sign-in is in flight', () => {
    const busy = persona({ label: 'a', action: 'connect' });
    const idle = persona({ label: 'b', action: 'connect' });
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [busy, idle],
      isBusy: (p: PersonaWire) => p.label === 'a'
    });
    cleanup.push(m.destroy);
    expect((rows(m)[0].querySelector('button') as HTMLButtonElement).disabled).toBe(true);
    expect((rows(m)[1].querySelector('button') as HTMLButtonElement).disabled).toBe(false);
  });

  it('shows an error instead of an empty list when the read failed', () => {
    // "No identities" invites the user to connect one — the wrong action
    // against a registry that is merely unreachable.
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [],
      error: 'could not read the identity list'
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[role="alert"]')?.textContent).toMatch(/could not read/i);
    expect(rows(m)).toHaveLength(0);
  });

  it('distinguishes a genuinely empty registry from a failure', () => {
    const m = mountComponent(SignInOverlay, { ...BASE, personas: [], error: null });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[role="alert"]')).toBeNull();
    expect(root(m)!.textContent).toMatch(/no identities yet/i);
  });

  it('closes on Escape and on a backdrop click, but not on a click inside', () => {
    const onClose = vi.fn();
    const m = mountComponent(SignInOverlay, { ...BASE, onClose });
    cleanup.push(m.destroy);
    const scrim = root(m) as HTMLElement;

    scrim.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(onClose).toHaveBeenCalledTimes(1);

    scrim.click();
    expect(onClose).toHaveBeenCalledTimes(2);

    // A click on the card must NOT dismiss — losing the dialog mid-read is the
    // classic misimplementation of click-outside.
    (scrim.firstElementChild as HTMLElement).click();
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('says that each identity opens its own window', () => {
    // A separate window is a deliberate, visible choice: a shared browser
    // session answers as whoever is already signed in to GitHub.
    const m = mountComponent(SignInOverlay, BASE);
    cleanup.push(m.destroy);
    expect(root(m)!.textContent).toMatch(/separate window/i);
  });

  it('is announced as a modal dialog with a name', () => {
    const m = mountComponent(SignInOverlay, BASE);
    cleanup.push(m.destroy);
    const el = root(m)!;
    expect(el.getAttribute('role')).toBe('dialog');
    expect(el.getAttribute('aria-modal')).toBe('true');
    const labelled = el.getAttribute('aria-labelledby');
    expect(labelled).toBeTruthy();
    expect(m.container.querySelector(`#${labelled}`)?.textContent).toBeTruthy();
  });
});
