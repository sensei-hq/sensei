// The dōjō's client-side CodeMapPort (#268): the /v1/you/graph reads behind
// the port interface, so the shared CodeMap state and components run
// unchanged. Transport failures are carried as their reason (#288's rule);
// cochange/complexity never fetch — no history and no Measures are synced
// for any repository, so both answers are local and static.
import type { CodeMapPort, CoChange, ComplexityTree, Focus, GroupBy, Level0, Level1, Level2, Read, Zones } from '@sensei/codemap';
import { getGraphFile, getGraphLevel0, getGraphModule, getGraphZones, type GraphReadOpts } from './client-data';

async function read<T>(r: Promise<Read<T>>): Promise<Read<T>> {
	try {
		return await r;
	} catch (e) {
		return { ok: false, error: e instanceof Error ? e.message : 'the request failed' };
	}
}

export function graphPortClient(ctx: () => GraphReadOpts): CodeMapPort {
	return {
		level0: (groupBy: GroupBy, focus: Focus | null) => read(getGraphLevel0({ ...ctx(), groupBy, focus })),
		module: (id: string) => read(getGraphModule({ ...ctx(), id })),
		file: (id: string) => read(getGraphFile({ ...ctx(), id })),
		zones: () => read(getGraphZones({ ...ctx() })),
		cochange: async (): Promise<Read<CoChange>> => ({
			ok: true,
			data: { hidden: [], pairs: 0, commits: 0, minTogether: 5 }
		}),
		complexity: async (): Promise<Read<ComplexityTree>> => ({
			ok: false,
			error: 'complexity Measures are not synced to the dōjō'
		})
	};
}
