// @vitest-environment jsdom
//
// The one modal shell (scrim, Escape, click-outside, focus in, focus kept in,
// focus back). Extracted when a third modal appeared, so its keyboard contract
// is tested once here instead of once per dialog.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './ModalShell.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function setup(open = true) {
  const ctl = $state({ open });
  const onClose = vi.fn();
  const m = mountComponent(Harness, { ctl, onClose });
  cleanup.push(m.destroy);
  const q = (id: string) => m.container.querySelector(`[data-testid="${id}"]`) as HTMLElement;
  const root = () => m.container.querySelector('[data-component="harness-modal"]') as HTMLElement | null;
  return { ctl, onClose, q, root };
}

const key = (k: string, shift = false) =>
  document.activeElement!.dispatchEvent(
    new KeyboardEvent('keydown', { key: k, shiftKey: shift, bubbles: true, cancelable: true }),
  );

describe('ModalShell', () => {
  it('renders nothing while closed', () => {
    expect(setup(false).root()).toBeNull();
  });

  it('is a named modal dialog', () => {
    const el = setup().root()!;
    expect(el.getAttribute('role')).toBe('dialog');
    expect(el.getAttribute('aria-modal')).toBe('true');
    expect(el.getAttribute('aria-labelledby')).toBe('harness-title');
  });

  it('focuses the first enabled button on open', () => {
    const { q } = setup();
    expect(document.activeElement).toBe(q('first'));
  });

  it('keeps Tab inside: past the last control it wraps to the first, and back', () => {
    const { q } = setup();
    q('last').focus();
    key('Tab');
    expect(document.activeElement).toBe(q('first'));
    key('Tab', true);
    expect(document.activeElement).toBe(q('last'));
  });

  it('closes on Escape from wherever focus is, and on a backdrop click only', () => {
    const { onClose, root } = setup();
    key('Escape');
    expect(onClose).toHaveBeenCalledTimes(1);
    (root()!.firstElementChild as HTMLElement).click();
    expect(onClose).toHaveBeenCalledTimes(1);
    root()!.click();
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('gives focus back to what had it before opening', () => {
    const { ctl, q } = setup(false);
    q('opener').focus();
    ctl.open = true;
    flushSync();
    expect(document.activeElement).toBe(q('first'));
    ctl.open = false;
    flushSync();
    expect(document.activeElement).toBe(q('opener'));
  });
});
