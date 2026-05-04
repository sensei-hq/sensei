import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

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

export async function openProjectWindow(projectId: string, projectName: string): Promise<void> {
  const label = `project-${projectId.replace(/-/g, '')}`;

  // If window already open, bring to front
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return;
  }

  openWindowsState.set(projectId, { projectId, label, projectName });

  const win = new WebviewWindow(label, {
    url: `/project/${projectId}`,
    title: `Sensei · ${projectName}`,
    width: 1200,
    height: 820,
    minWidth: 900,
    minHeight: 600,
    decorations: false,
  });

  await win.once('tauri://destroyed', () => {
    openWindowsState.delete(projectId);
  });
}
