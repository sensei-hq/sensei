import { mount, unmount } from 'svelte';
import type { Component } from 'svelte';

export interface MountResult {
  container: HTMLElement;
  destroy: () => void;
}

/**
 * Mount a Svelte 5 component into a fresh detached container appended to <body>.
 * Reactive tests should mutate a reactive object (e.g. a `HealthState` instance)
 * passed as a prop — Svelte 5's fine-grained reactivity propagates automatically.
 * Tests must call `destroy()` (typically in afterEach).
 */
export function mountComponent<P extends Record<string, unknown>>(
  comp: Component<P>,
  props: P,
): MountResult {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const instance = mount(comp, { target: container, props });
  return {
    container,
    destroy: () => { unmount(instance); container.remove(); },
  };
}
