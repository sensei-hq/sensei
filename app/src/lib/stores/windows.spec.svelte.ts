import { describe, it, expect, vi, beforeEach } from 'vitest';

// Use vi.hoisted so these variables are available inside the vi.mock factory (which gets hoisted)
const { mockSetFocus, mockOnce, MockWebviewWindow, mockEmit, mockEmitTo } = vi.hoisted(() => {
  const mockEmitTo = vi.fn().mockResolvedValue(undefined);
  const mockSetFocus = vi.fn().mockResolvedValue(undefined);
  const mockOnce = vi.fn().mockResolvedValue(() => undefined);
  const mockEmit = vi.fn().mockResolvedValue(undefined);
  const MockWebviewWindow = vi.fn().mockImplementation(function(this: any, label: string) {
    this.label = label;
    this.setFocus = mockSetFocus;
    this.once = mockOnce;
  });
  (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(null);
  return { mockSetFocus, mockOnce, MockWebviewWindow, mockEmit, mockEmitTo };
});

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: MockWebviewWindow,
}));

// `emit` reaches Tauri internals (needs `window`) — mock it so the store's
// native-menu sync is a no-op in unit tests and assertable.
vi.mock('@tauri-apps/api/event', () => ({
  emit: mockEmit,
  emitTo: mockEmitTo,
}));

import { isSettingsPath, openSettingsWindow, openWorkspaceWindow } from './windows.svelte.js';

describe('openWorkspaceWindow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(null);
  });

  it('strips hyphens from workspace id for Tauri label', async () => {
    await openWorkspaceWindow('abc-def-1234-5678', 'My Workspace');
    const label = (MockWebviewWindow as any).mock.calls[0][0] as string;
    expect(label.startsWith('workspace-')).toBe(true);
    // After 'workspace-' prefix, no hyphens from the UUID
    expect(label.slice('workspace-'.length)).not.toContain('-');
  });

  it('opens an opaque window with the overlay titlebar', async () => {
    await openWorkspaceWindow('abc-def-1234', 'My Workspace');
    const opts = (MockWebviewWindow as any).mock.calls[0][1];
    // A transparent window leaves the Linux GTK menu bar see-through.
    expect(opts.transparent).toBeFalsy();
    expect(opts.titleBarStyle).toBe('overlay');
    expect(opts.hiddenTitle).toBe(true);
  });

  it('sets focus on existing window instead of opening a new one', async () => {
    const fakeWin = { setFocus: mockSetFocus, once: mockOnce };
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(fakeWin);

    await openWorkspaceWindow('abc-def-1234', 'My Workspace');

    expect(mockSetFocus).toHaveBeenCalled();
    expect(MockWebviewWindow).not.toHaveBeenCalled();
  });

  it('emits the open windows to the native Window menu after opening one', async () => {
    await openWorkspaceWindow('feed-1111-2222-3333', 'Gateway');
    expect(mockEmit).toHaveBeenCalledWith(
      'sync-window-menu',
      expect.arrayContaining([expect.objectContaining({ title: 'Gateway' })]),
    );
  });
});

// Settings is a child window, as a Mac app's preferences are (user,
// 2026-10-27): ⌘, or the app menu opens it; one at a time.
describe('openSettingsWindow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(null);
  });

  it('opens one settings window at the page asked for', async () => {
    await openSettingsWindow('/settings/dojo');
    const [label, opts] = (MockWebviewWindow as any).mock.calls[0];
    expect(label).toBe('settings');
    expect(opts.url).toBe('/settings/dojo');
    expect(opts.titleBarStyle).toBe('overlay');
  });

  it('brings an open settings window forward, at the page asked for', async () => {
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue({ setFocus: mockSetFocus });
    await openSettingsWindow('/settings/dojo');
    expect(mockSetFocus).toHaveBeenCalled();
    expect(mockEmitTo).toHaveBeenCalledWith('settings', 'dev-navigate', '/settings/dojo');
    expect(MockWebviewWindow).not.toHaveBeenCalled();
  });
});

describe('isSettingsPath', () => {
  it('is the settings window’s, and nothing else', () => {
    expect(['/settings', '/settings/general'].map(isSettingsPath)).toEqual([true, true]);
    expect(['/', '/dojo/connections', '/settingsx', '/workspace/1'].map(isSettingsPath)).toEqual([false, false, false, false]);
  });
});
