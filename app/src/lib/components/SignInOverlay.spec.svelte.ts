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

  it('says it is reading rather than claiming emptiness before the first read', () => {
    // Opening from the menu starts the read and shows the overlay in the same
    // tick, so "No identities yet" would be visible over data not yet fetched.
    const m = mountComponent(SignInOverlay, { ...BASE, personas: [], loaded: false });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="sign-in-loading"]')).not.toBeNull();
    expect(root(m)!.textContent).not.toMatch(/no identities yet/i);
  });

  it('distinguishes a genuinely empty registry from a failure', () => {
    const m = mountComponent(SignInOverlay, { ...BASE, personas: [], error: null, loaded: true });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[role="alert"]')).toBeNull();
    expect(root(m)!.textContent).toMatch(/no identities yet/i);
  });

  it('moves focus into the dialog on open', () => {
    // Load-bearing, not cosmetic: Escape is handled on the container, so without
    // focus inside, the key goes to whatever was focused behind the overlay and
    // the dialog cannot be dismissed by keyboard. The e2e run caught this; the
    // old unit test did not, because it dispatched keydown onto the element.
    const m = mountComponent(SignInOverlay, {
      ...BASE,
      personas: [persona({ action: 'connect' })]
    });
    cleanup.push(m.destroy);
    const active = document.activeElement as HTMLElement;
    expect(root(m)!.contains(active)).toBe(true);
    // Prefers the first actionable control so a keyboard user can act at once.
    expect(active.tagName).toBe('BUTTON');
  });

  it('focuses the card itself when there is nothing to click', () => {
    // A list of healthy identities has no buttons. The card must still take
    // focus or Escape is dead.
    const m = mountComponent(SignInOverlay, BASE);
    cleanup.push(m.destroy);
    expect(root(m)!.contains(document.activeElement)).toBe(true);
  });

  it('closes on Escape pressed at wherever focus actually is', () => {
    const onClose = vi.fn();
    const m = mountComponent(SignInOverlay, { ...BASE, onClose });
    cleanup.push(m.destroy);

    // Dispatched on the ACTIVE element, which is what a real keypress does.
    // Bubbling to the container is the behaviour under test.
    document.activeElement!.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })
    );
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes on a backdrop click, but not on a click inside', () => {
    const onClose = vi.fn();
    const m = mountComponent(SignInOverlay, { ...BASE, onClose });
    cleanup.push(m.destroy);
    const scrim = root(m) as HTMLElement;

    scrim.click();
    expect(onClose).toHaveBeenCalledTimes(1);

    // A click on the card must NOT dismiss — losing the dialog mid-read is the
    // classic misimplementation of click-outside.
    (scrim.firstElementChild as HTMLElement).click();
    expect(onClose).toHaveBeenCalledTimes(1);
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
