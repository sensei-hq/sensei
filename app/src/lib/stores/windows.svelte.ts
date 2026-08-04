import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { emit } from '@tauri-apps/api/event';

export interface OpenWindow {
  projectId: string;
  label: string;
  projectName: string;
}

let openWindowsState = $state<Map<string, OpenWindow>>(new Map());

export const openWindows = {
  get all(): OpenWindow[] { return [...openWindowsState.values()]; },
  has(projectId: string): boolean { return openWindowsState.has(projectId); },
};

/** Push the current open project windows to the native menu (Rust rebuilds the
 *  Window submenu). macOS doesn't auto-list Tauri windows, so the menu only
 *  reflects reality if we emit on every open/close. */
function syncWindowMenu(): void {
  void emit(
    'sync-window-menu',
    openWindows.all.map((w) => ({ label: w.label, title: w.projectName })),
  );
}

export async function openProjectWindow(projectId: string, projectName: string): Promise<void> {
  const label = `project-${projectId.replace(/-/g, '')}`;

  // If window already open, bring to front
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }

  openWindowsState.set(projectId, { projectId, label, projectName });

  let win: WebviewWindow;
  try {
    win = new WebviewWindow(label, {
      url: `/project/${projectId}`,
      title: `Sensei · ${projectName}`,
      width: 1200,
      height: 820,
      minWidth: 900,
      minHeight: 600,
      // Match the main window's chrome (tauri.conf.json): a transparent overlay
      // titlebar with the macOS traffic lights floating top-left and no OS title
      // text. `hiddenTitle` + `titleBarStyle: 'overlay'` require decorations to
      // stay on — `decorations: false` (the old value) can't render the overlay
      // titlebar, which left the traffic lights overlapping the content.
      titleBarStyle: 'overlay',
      hiddenTitle: true,
      transparent: true,
    });
  } catch (err) {
    openWindowsState.delete(projectId);
    throw err;
  }
  syncWindowMenu(); // the window exists now — add it to the native Window menu

  await win.once('tauri://destroyed', () => {
    openWindowsState.delete(projectId);
    syncWindowMenu(); // window gone — drop it from the Window menu
  });
}
