// The app's Code map port: the daemon's codemap endpoints for one workspace,
// each failure carried as its reason (#288).
import { failureReason, type ApiResult } from '$lib/api.js';
import type { CodeMapPort, GroupBy, Level0, Level1, Level2, Read, Zones } from '@sensei/codemap';

export interface CodemapApi {
  tryGetCodemap: (id: string, groupBy: GroupBy) => Promise<ApiResult<Level0>>;
  tryGetCodemapModule: (id: string, module: string) => Promise<ApiResult<Level1>>;
  tryGetCodemapFile: (id: string, file: string) => Promise<ApiResult<Level2>>;
  tryGetZones: (id: string) => Promise<ApiResult<Zones>>;
}

const read = async <T>(r: Promise<ApiResult<T>>): Promise<Read<T>> => {
  const res = await r;
  return res.ok ? { ok: true, data: res.data } : { ok: false, error: failureReason(res.error) };
};

export function codemapPort(api: CodemapApi, workspaceId: () => string): CodeMapPort {
  return {
    level0: (groupBy) => read(api.tryGetCodemap(workspaceId(), groupBy)),
    module: (id) => read(api.tryGetCodemapModule(workspaceId(), id)),
    file: (id) => read(api.tryGetCodemapFile(workspaceId(), id)),
    zones: () => read(api.tryGetZones(workspaceId())),
  };
}
