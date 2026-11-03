import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { emit, emitTo } from '@tauri-apps/api/event';
export { isSettingsPath } from '$lib/settings-path.js';

export interface OpenWindow {
  workspaceId: string;
  label: string;
  workspaceName: string;
}

let openWindowsState = $state<Map<string, OpenWindow>>(new Map());

export const openWindows = {
  get all(): OpenWindow[] { return [...openWindowsState.values()]; },
  has(workspaceId: string): boolean { return openWindowsState.has(workspaceId); },
};

/** Push the current open workspace windows to the native menu (Rust rebuilds the
 *  Window submenu). macOS doesn't auto-list Tauri windows, so the menu only
 *  reflects reality if we emit on every open/close. */
function syncWindowMenu(): void {
  void emit(
    'sync-window-menu',
    openWindows.all.map((w) => ({ label: w.label, title: w.workspaceName })),
  );
}

export async function openWorkspaceWindow(workspaceId: string, workspaceName: string): Promise<void> {
  const label = `workspace-${workspaceId.replace(/-/g, '')}`;

  // If window already open, bring to front
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }

  openWindowsState.set(workspaceId, { workspaceId, label, workspaceName });

  let win: WebviewWindow;
  try {
    win = new WebviewWindow(label, {
      url: `/workspace/${workspaceId}`,
      title: `Sensei · ${workspaceName}`,
      width: 1200,
      height: 820,
      minWidth: 900,
      minHeight: 600,
      // Match the main window's chrome (tauri.conf.json): an overlay titlebar
      // with the macOS traffic lights floating top-left and no OS title text.
      // `hiddenTitle` + `titleBarStyle: 'overlay'` require decorations to stay
      // on — `decorations: false` (the old value) can't render the overlay
      // titlebar, which left the traffic lights overlapping the content.
      // Not `transparent`: only the main window gets vibrancy, and on Linux a
      // transparent window leaves the GTK menu bar see-through.
      titleBarStyle: 'overlay',
      hiddenTitle: true,
    });
  } catch (err) {
    openWindowsState.delete(workspaceId);
    throw err;
  }
  syncWindowMenu(); // the window exists now — add it to the native Window menu

  await win.once('tauri://destroyed', () => {
    openWindowsState.delete(workspaceId);
    syncWindowMenu(); // window gone — drop it from the Window menu
  });
}

/** The settings window's label: one at a time. */
const SETTINGS_LABEL = 'settings';

/** Open Settings as its own window at `path` (user, 2026-10-27: ⌘, or the app
 *  menu's Preferences…, as in a Mac app), or bring the open one forward. */
export async function openSettingsWindow(path = '/settings/general'): Promise<void> {
  const existing = await WebviewWindow.getByLabel(SETTINGS_LABEL);
  if (existing) {
    await existing.setFocus();
    // The root layout's `dev-navigate` listener turns this into a goto.
    await emitTo(SETTINGS_LABEL, 'dev-navigate', path);
    return;
  }
  new WebviewWindow(SETTINGS_LABEL, {
    url: path,
    title: 'Sensei · Settings',
    width: 1000,
    height: 720,
    minWidth: 760,
    minHeight: 520,
    // The main and workspace windows' chrome: overlay titlebar, no OS title.
    titleBarStyle: 'overlay',
    hiddenTitle: true,
  });
}
