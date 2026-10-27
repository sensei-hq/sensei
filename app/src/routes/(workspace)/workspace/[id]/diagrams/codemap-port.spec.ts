import { describe, expect, it, vi } from 'vitest';
import { codemapPort } from './codemap-port';

describe('codemapPort', () => {
  it('reads the workspace it is given, and carries a failure as its reason', async () => {
    const api = {
      tryGetCodemap: vi.fn().mockResolvedValue({ ok: true, data: { blocks: [] } }),
      tryGetCodemapModule: vi.fn().mockResolvedValue({ ok: false, error: { status: 503, message: 'Service Unavailable' } }),
      tryGetCodemapFile: vi.fn(),
    };
    const port = codemapPort(api, () => 'ws1');
    expect(await port.level0('folders')).toEqual({ ok: true, data: { blocks: [] } });
    expect(api.tryGetCodemap).toHaveBeenCalledWith('ws1', 'folders');
    expect(await port.module('p/a')).toEqual({ ok: false, error: 'Service Unavailable' });
    expect(api.tryGetCodemapModule).toHaveBeenCalledWith('ws1', 'p/a');
  });
});
