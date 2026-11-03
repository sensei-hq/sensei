// packages/ui/state-defaults.js — what a failed screen says to do, provided once
// from each app's root (like glyph-mode.js). The desktop app names the daemon;
// the dōjō names itself. StateView reads it so no screen has to repeat it and no
// failure is left without a next step.

import { getContext, setContext } from 'svelte';

const STATE_CONTEXT = Symbol.for('sensei.state-defaults');

/**
 * Provide the defaults to every StateView below. Call once, during the root
 * layout's init.
 * @param {{ remedy?: string, service?: string }} defaults  `service` names the far
 *        end of every read ("the sensei daemon", "the dōjō").
 * @returns {void}
 */
export function setStateDefaults(defaults) {
  setContext(STATE_CONTEXT, defaults);
}

/**
 * The defaults provided above, or none (StateView then uses DEFAULT_REMEDY).
 * Call during component init.
 * @returns {{ remedy?: string, service?: string }}
 */
export function useStateDefaults() {
  return getContext(STATE_CONTEXT) ?? {};
}
