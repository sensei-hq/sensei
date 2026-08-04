import { describe, it, expect, vi, beforeEach } from 'vitest';

// Use vi.hoisted so these variables are available inside the vi.mock factory (which gets hoisted)
const { mockSetFocus, mockOnce, MockWebviewWindow, mockEmit } = vi.hoisted(() => {
  const mockSetFocus = vi.fn().mockResolvedValue(undefined);
  const mockOnce = vi.fn().mockResolvedValue(() => undefined);
  const mockEmit = vi.fn().mockResolvedValue(undefined);
  const MockWebviewWindow = vi.fn().mockImplementation(function(this: any, label: string) {
    this.label = label;
    this.setFocus = mockSetFocus;
    this.once = mockOnce;
  });
  (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(null);
  return { mockSetFocus, mockOnce, MockWebviewWindow, mockEmit };
});

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: MockWebviewWindow,
}));

// `emit` reaches Tauri internals (needs `window`) — mock it so the store's
// native-menu sync is a no-op in unit tests and assertable.
vi.mock('@tauri-apps/api/event', () => ({
  emit: mockEmit,
}));

import { openProjectWindow } from './windows.svelte.js';

describe('openProjectWindow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(null);
  });

  it('strips hyphens from project id for Tauri label', async () => {
    await openProjectWindow('abc-def-1234-5678', 'My Project');
    const label = (MockWebviewWindow as any).mock.calls[0][0] as string;
    expect(label.startsWith('project-')).toBe(true);
    // After 'project-' prefix, no hyphens from the UUID
    expect(label.slice('project-'.length)).not.toContain('-');
  });

  it('sets focus on existing window instead of opening a new one', async () => {
    const fakeWin = { setFocus: mockSetFocus, once: mockOnce };
    (MockWebviewWindow as any).getByLabel = vi.fn().mockResolvedValue(fakeWin);

    await openProjectWindow('abc-def-1234', 'My Project');

    expect(mockSetFocus).toHaveBeenCalled();
    expect(MockWebviewWindow).not.toHaveBeenCalled();
  });

  it('emits the open windows to the native Window menu after opening one', async () => {
    await openProjectWindow('feed-1111-2222-3333', 'Gateway');
    expect(mockEmit).toHaveBeenCalledWith(
      'sync-window-menu',
      expect.arrayContaining([expect.objectContaining({ title: 'Gateway' })]),
    );
  });
});
