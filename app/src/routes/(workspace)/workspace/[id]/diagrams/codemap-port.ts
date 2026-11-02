// The app's Code map port: the daemon's codemap endpoints for one workspace,
// each failure carried as its reason (#288).
import { failureReason, type ApiResult, type ComplexityWire } from '$lib/api.js';
import type { CoChange, CodeMapPort, GroupBy, Level0, Level1, Level2, Read, Zones } from '@sensei/codemap';
import { formatPerKloc, metricGrade } from '$lib/metrics/metric-view.js';


export interface CodemapApi {
  tryGetCodemap: (id: string, groupBy: GroupBy) => Promise<ApiResult<Level0>>;
  tryGetCodemapModule: (id: string, module: string) => Promise<ApiResult<Level1>>;
  tryGetCodemapFile: (id: string, file: string) => Promise<ApiResult<Level2>>;
  tryGetZones: (id: string) => Promise<ApiResult<Zones>>;
  tryGetCochange: (id: string) => Promise<ApiResult<CoChange>>;
  tryGetComplexity: (id: string) => Promise<ApiResult<ComplexityWire>>;
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
    cochange: () => read(api.tryGetCochange(workspaceId())),
    // Graded by the Metrics page's own rule, so maintainability reads the same here.
    complexity: async () => {
      const r = await read(api.tryGetComplexity(workspaceId()));
      if (!r.ok) return r;
      const m = r.data.maintainability;
      return {
        ok: true,
        data: {
          ...r.data,
          maintainability: m && { ...m, grade: metricGrade('module_quality', m.value), label: formatPerKloc(m.value) },
        },
      };
    },
  };
}
