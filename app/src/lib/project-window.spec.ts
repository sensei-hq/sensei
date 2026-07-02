import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { openProjectWindow } from './project-window.js';

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('openProjectWindow', () => {
  it('invokes the open_project_window command with the projectId', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await openProjectWindow('abc-123');
    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith('open_project_window', {
      projectId: 'abc-123',
    });
  });

  it('rejects an empty projectId before invoking Tauri', async () => {
    await expect(openProjectWindow('')).rejects.toThrow(/projectId required/);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('surfaces the Tauri error to the caller', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('boom'));
    await expect(openProjectWindow('x')).rejects.toThrow('boom');
  });
});
