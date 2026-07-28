import { vibe } from '@rokkit/states';
import { ColorModeManager, resolveMode, type ColorMode } from '@rokkit/app';

// Color-mode (dark/light/system) for the whole dōjō. rokkit's ColorModeManager bridges
// the 3-way UI choice to vibe's 2-way light/dark store, resolving 'system' via
// prefers-color-scheme. We own the abstract choice here (persisted) so 'system' survives
// reloads — vibe/themable only persist the resolved light/dark. Wire once in the root
// layout: `setColorMode(loadColorMode())` + `colorMode.listen()` on mount.
const KEY = 'dojo-color-mode';

export const colorMode = new ColorModeManager(vibe, 'system');

/** The persisted abstract choice (system unless the user picked light/dark). */
export function loadColorMode(): ColorMode {
	if (typeof localStorage === 'undefined') return 'system';
	const v = localStorage.getItem(KEY);
	return v === 'light' || v === 'dark' || v === 'system' ? v : 'system';
}

/** Set + persist the mode. Also forces vibe.mode to the resolved value directly, since
 *  the manager's setter no-ops when the abstract value is unchanged (e.g. re-applying
 *  'system' on load must still resolve the current OS preference onto vibe). */
export function setColorMode(mode: ColorMode) {
	colorMode.mode = mode;
	vibe.mode = resolveMode(mode);
	if (typeof localStorage !== 'undefined') localStorage.setItem(KEY, mode);
}
