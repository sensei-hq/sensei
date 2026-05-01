import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getModuleLogger } from './logger.js';

// Mock @tauri-apps/api/core before importing logger
vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
    mockInvoke.mockReset();
});

describe('getModuleLogger', () => {
    it('starts a session on first call', async () => {
        mockInvoke.mockResolvedValueOnce('sess-00000001'); // log_session_start
        const logger = getModuleLogger('wizard');
        await logger.ready;
        expect(mockInvoke).toHaveBeenCalledWith('log_session_start', expect.objectContaining({
            module: 'wizard',
        }));
    });

    it('returns the same instance for the same module', async () => {
        mockInvoke.mockResolvedValue('sess-00000001');
        const a = getModuleLogger('wizard');
        const b = getModuleLogger('wizard');
        expect(a).toBe(b);
    });

    it('returns different instances for different modules', async () => {
        mockInvoke.mockResolvedValue('sess-00000001');
        const a = getModuleLogger('wizard');
        const b = getModuleLogger('bootstrap');
        expect(a).not.toBe(b);
    });

    it('info() sends a log entry with level info', async () => {
        mockInvoke.mockResolvedValueOnce('sess-abc');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-info');
        await logger.ready;
        await logger.info('ui', 'prefs_load', 'Prefs loaded', { ms: 10 });
        const call = mockInvoke.mock.calls.find(([cmd]) => cmd === 'log_entry');
        expect(call).toBeDefined();
        const entry = (call![1] as any).entry;
        expect(entry.level).toBe('info');
        expect(entry.layer).toBe('ui');
        expect(entry.step).toBe('prefs_load');
        expect(entry.msg).toBe('Prefs loaded');
        expect(entry.data).toEqual({ ms: 10 });
    });

    it('error() sets err and stack from Error object', async () => {
        mockInvoke.mockResolvedValueOnce('sess-abc');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-error');
        await logger.ready;
        const e = new Error('connection refused');
        await logger.error('sidecar', 'daemon_invoke', 'Invoke failed', e);
        const call = mockInvoke.mock.calls.find(([cmd]) => cmd === 'log_entry');
        const entry = (call![1] as any).entry;
        expect(entry.level).toBe('error');
        expect(entry.err).toBe('connection refused');
        expect(entry.stack).toContain('Error');
    });

    it('close() calls log_session_end', async () => {
        mockInvoke.mockResolvedValueOnce('sess-close');
        mockInvoke.mockResolvedValue(undefined);
        const logger = getModuleLogger('test-close');
        await logger.ready;
        await logger.close();
        expect(mockInvoke).toHaveBeenCalledWith('log_session_end', { session_id: 'sess-close' });
    });
});
