import { describe, expect, it, vi } from 'vitest';
import { codemapPort } from './codemap-port';

describe('codemapPort', () => {
  it('reads the workspace it is given, and carries a failure as its reason', async () => {
    const api = {
      tryGetCodemap: vi.fn().mockResolvedValue({ ok: true, data: { blocks: [] } }),
      tryGetCodemapModule: vi.fn().mockResolvedValue({ ok: false, error: { status: 503, message: 'Service Unavailable' } }),
      tryGetCodemapFile: vi.fn(),
      tryGetZones: vi.fn().mockResolvedValue({ ok: true, data: { points: [] } }),
      tryGetCochange: vi.fn().mockResolvedValue({ ok: true, data: { hidden: [] } }),
      tryGetComplexity: vi.fn(),
    };
    const port = codemapPort(api, () => 'ws1');
    expect(await port.level0('folders')).toEqual({ ok: true, data: { blocks: [] } });
    expect(api.tryGetCodemap).toHaveBeenCalledWith('ws1', 'folders');
    expect(await port.module('p/a')).toEqual({ ok: false, error: 'Service Unavailable' });
    expect(api.tryGetCodemapModule).toHaveBeenCalledWith('ws1', 'p/a');
    expect(await port.zones()).toEqual({ ok: true, data: { points: [] } });
    expect(api.tryGetZones).toHaveBeenCalledWith('ws1');
    expect(await port.cochange()).toEqual({ ok: true, data: { hidden: [] } });
    expect(api.tryGetCochange).toHaveBeenCalledWith('ws1');
  });

  // The Complexity tree's maintainability reading is graded here, by the app's one
  // grading rule (metricGrade), so the shared drawing never carries a second copy.
  it('grades the Complexity tree’s maintainability reading, and keeps none as none', async () => {
    const tree = { total: { functions: 1, complex: 0, maxCyclo: 3, files: 1 }, modules: [] };
    const api = {
      tryGetCodemap: vi.fn(), tryGetCodemapModule: vi.fn(), tryGetCodemapFile: vi.fn(), tryGetZones: vi.fn(), tryGetCochange: vi.fn(),
      tryGetComplexity: vi.fn()
        .mockResolvedValueOnce({ ok: true, data: { ...tree, maintainability: { value: 0.0048, date: '2026-11-01' } } })
        .mockResolvedValueOnce({ ok: true, data: { ...tree, maintainability: null } }),
    };
    const port = codemapPort(api, () => 'ws1');
    const graded = await port.complexity();
    expect(api.tryGetComplexity).toHaveBeenCalledWith('ws1');
    expect(graded.ok && graded.data.maintainability).toEqual({ value: 0.0048, date: '2026-11-01', grade: 'B', label: '4.8 / 1,000 lines' });
    const none = await port.complexity();
    expect(none.ok && none.data.maintainability).toBeNull();
  });
});
