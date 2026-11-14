// The dōjō's remembered screen panels (D87, CONTRACT §4). ScreenShell stores
// nothing; the dōjō keeps every view's PanelState in ONE map keyed by view id, in
// this browser's localStorage. It is a per-browser convenience: a missing, failed
// or unreadable read means first visit (everything open), never an error on the
// screen. A per-user server row can replace it later without changing a screen.

/** Which bands of a view are open; a missing key is open. */
export type PanelState = Partial<Record<'info' | 'stats' | 'filters' | 'side', boolean>>;

export const STORAGE_KEY = 'sensei.dojo.views';

function readAll(): Record<string, PanelState> {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		const parsed: unknown = raw ? JSON.parse(raw) : {};
		return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? (parsed as Record<string, PanelState>) : {};
	} catch {
		return {}; // private window, blocked storage or unparseable JSON: a first visit
	}
}

/** The view's panels as the user last left them; `{}` (all open) on a first visit. */
export function panelsFor(viewId: string): PanelState {
	return readAll()[viewId] ?? {};
}

/** Save the view's whole PanelState (a toggle writes every band the view has). */
export function savePanels(viewId: string, next: PanelState): void {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...readAll(), [viewId]: next }));
	} catch {
		// Not saved in this browser: the screen still works, it just won't remember.
	}
}
