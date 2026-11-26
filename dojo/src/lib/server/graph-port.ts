// The dōjō's CodeMapPort (#268): the exact `@sensei/codemap` wire shapes over
// one branch's graph rows, so the same components draw the dōjō's diagrams
// and the app's. The daemon computes these in Rust; the dōjō cannot call it,
// so this wires `graph-reads.ts` through the faithful ports (`graph-layering`
// for ranks and verdicts, `graph-codemap` for blocks, bands, columns, zones).
//
// Mapping, stated once:
// - A module is a folder (`folder_path`); unplaced nodes (no file) belong to
//   no module. The daemon's modules are modal-per-file and finer — folder
//   grain is what the synced rows support, and every screen says module.
// - Bands aggregate resolved `calls`/`imports` edges by folder pair, weighted
//   by row count — the daemon's `sum(occurrences)` over placed edges.
// - Complexity is not synced: blocks carry none, `complexity()` answers
//   ok:false. Commit history is not synced: `cochange()` answers commits: 0.
// - Files are keyed by path (graph_files has no uuid); L2 takes the path.
import type { CodeMapPort, GroupBy, Focus, Read, Level0, Level1, Level2, Zones, CoChange, ComplexityTree } from '@sensei/codemap';
import type { BranchGraph, GraphNode } from './graph-reads';
import { analyse } from './graph-layering';
import {
	BLOCK_BUDGET,
	fileRole,
	focusScope,
	level0,
	level1,
	librariesSummary,
	segmentMembers,
	zones as zonePoints,
	type Module,
	type ModuleEdge,
	type RoleCounts
} from './graph-codemap';

const KINDS = ['calls', 'imports'];
const NEIGHBOUR_BUDGET = 7;
const LIBRARIES_TOP = 5;
const COCHANGE_MIN = 5;
const TYPE_KINDS = new Set(['class', 'interface', 'struct', 'enum', 'type', 'trait']);

function zeroRoles(): RoleCounts {
	return { code: 0, test: 0, docs: 0, config: 0, generated: 0 };
}

function labelOf(folder: string): string {
	if (folder === '') return '(root)';
	const parts = folder.split('/');
	return parts[parts.length - 1] || folder;
}

function packageOf(folder: string): string {
	if (folder === '') return '(root)';
	return folder.split('/')[0];
}

interface Prepared {
	modules: Module[];
	edges: ModuleEdge[];
	ranked: ReturnType<typeof analyse>;
	byId: Map<string, GraphNode>;
	filesOf: Map<string, string[]>;
	nodesOfFile: Map<string, GraphNode[]>;
}

function prepare(g: BranchGraph): Prepared {
	const byId = new Map(g.nodes.map((n) => [n.id, n]));
	const filesOf = new Map<string, string[]>();
	const nodesOfFile = new Map<string, GraphNode[]>();
	const symbolsOf = new Map<string, number>();
	for (const n of g.nodes) {
		if (n.file_path === null) continue;
		if (!filesOf.has(n.folder_path)) filesOf.set(n.folder_path, []);
		if (!filesOf.get(n.folder_path)?.includes(n.file_path)) filesOf.get(n.folder_path)?.push(n.file_path);
		if (!nodesOfFile.has(n.file_path)) nodesOfFile.set(n.file_path, []);
		nodesOfFile.get(n.file_path)?.push(n);
		symbolsOf.set(n.folder_path, (symbolsOf.get(n.folder_path) ?? 0) + 1);
	}
	const modules: Module[] = [...filesOf.keys()].map((folder) => {
		const roles = zeroRoles();
		for (const f of filesOf.get(folder) ?? []) roles[fileRole(f)]++;
		return {
			id: folder,
			label: labelOf(folder),
			group: packageOf(folder),
			pkg: packageOf(folder),
			files: filesOf.get(folder)?.length ?? 0,
			symbols: symbolsOf.get(folder) ?? 0,
			roles,
			complexity: null
		};
	});
	// Resolved calls/imports by folder pair, one row of weight each — the
	// daemon's placed-edges-only module edge set.
	const bandWeight = new Map<string, { source: string; target: string; kind: string; count: number }>();
	for (const e of g.edges) {
		if (e.target_id === null || !KINDS.includes(e.kind)) continue;
		const s = byId.get(e.source_id);
		const t = byId.get(e.target_id);
		if (!s || !t || s.file_path === null || t.file_path === null) continue;
		const key = `${s.folder_path}\u0000${t.folder_path}\u0000${e.kind}`;
		const cur = bandWeight.get(key) ?? { source: s.folder_path, target: t.folder_path, kind: e.kind, count: 0 };
		cur.count++;
		bandWeight.set(key, cur);
	}
	const edges: ModuleEdge[] = [...bandWeight.values()];
	const ranked = analyse(
		modules.map((m) => m.id),
		edges.map((e) => ({ source: e.source, target: e.target, occurrences: e.count }))
	);
	return { modules, edges, ranked, byId, filesOf, nodesOfFile };
}

export function createGraphPort(read: () => Promise<BranchGraph>): CodeMapPort {
	let prepared: Promise<Prepared> | null = null;
	const load = (): Promise<Prepared> => (prepared ??= read().then(prepare));

	async function level0read(groupBy: GroupBy, focus: Focus | null): Promise<Read<Level0>> {
		const p = await load();
		const layerOf = new Map(p.ranked.units.map((u) => [u.id, u.layer]));
		const modules: Module[] = p.modules.map((m) => ({
			...m,
			group: groupBy === 'folders' ? m.pkg : (layerOf.has(m.id) ? `L${layerOf.get(m.id)}` : 'unplaced')
		}));
		let scoped = { modules, edges: p.edges };
		let focusJson: Level0['focus'] = null;
		if (focus) {
			const members =
				focus.kind === 'module'
					? new Set(modules.some((m) => m.id === focus.id) ? [focus.id] : [])
					: segmentMembers(modules, focus.id);
			if (members.size === 0) return { ok: false, error: `no ${focus.kind} ${focus.id} on this branch` };
			const f = focusScope(modules, p.edges, members);
			scoped = { modules: f.modules, edges: f.edges };
			focusJson = { kind: focus.kind, id: focus.id, members: [...members].sort(), neighbours: f.neighbours };
		}
		const l0 = level0(scoped.modules, scoped.edges, BLOCK_BUDGET);
		const direction = new Map(p.ranked.deps.map((d) => [`${d.source}\u0000${d.target}`, d.conformance]));
		const g = await read();
		const unplaced = g.edges.filter((e) => e.external && KINDS.includes(e.kind)).length;
		const libUses = new Map<string, number>();
		for (const e of g.edges) {
			if (!e.external || !e.target_name) continue;
			libUses.set(e.target_name, (libUses.get(e.target_name) ?? 0) + 1);
		}
		const cycles = focusJson
			? p.ranked.cycles.filter((c) => c.members.some((m) => (focusJson?.members ?? []).includes(m)))
			: p.ranked.cycles;
		return {
			ok: true,
			data: {
				level: 'module',
				groupBy,
				blocks: l0.blocks.map((b) => ({
					id: b.id,
					label: b.label,
					group: b.group,
					package: b.package,
					files: b.files,
					symbols: b.symbols,
					roles: b.roles,
					hub: b.hub,
					weight_in: b.weight_in,
					weight_out: b.weight_out,
					folded: b.folded,
					complexity: b.complexity
				})),
				bands: l0.bands.map((b) => ({ ...b, direction: direction.get(`${b.source}\u0000${b.target}`) ?? null })),
				budget: l0.budget,
				cycles,
				coverage: { unplaced },
				libraries: librariesSummary([...libUses].map(([name, uses]) => ({ name, uses })), LIBRARIES_TOP),
				focus: focusJson
			}
		};
	}

	async function moduleread(id: string): Promise<Read<Level1>> {
		const p = await load();
		const files = p.filesOf.get(id);
		if (!files) return { ok: false, error: `no module ${id} on this branch` };
		const side = (outbound: boolean) =>
			p.ranked.deps
				.filter((d) => (outbound ? d.source === id : d.target === id))
				.map((d) => ({ module: outbound ? d.target : d.source, weight: d.occurrences, direction: d.conformance }))
				.sort((a, b) => b.weight - a.weight);
		return {
			ok: true,
			data: {
				level: 'file',
				module: id,
				columns: level1(
					files.map((path) => ({ id: path, path, symbols: p.nodesOfFile.get(path)?.length ?? 0 }))
				),
				dependsOn: side(true),
				usedBy: side(false)
			}
		};
	}

	async function fileread(path: string): Promise<Read<Level2>> {
		const p = await load();
		const g = await read();
		const mine = p.nodesOfFile.get(path);
		if (!mine) return { ok: false, error: `no file ${path} on this branch` };
		const inside = new Set(mine.map((n) => n.id));
		const symbols = [...mine]
			.sort((a, b) => (a.line_start ?? Number.MAX_SAFE_INTEGER) - (b.line_start ?? Number.MAX_SAFE_INTEGER) || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
			.map((n) => ({ id: n.id, name: n.name, kind: n.kind, line: n.line_start }));
		// Placed `calls` across the file boundary, heaviest symbol first —
		// the daemon's neighbour query, including its ORDER BY.
		const side = (inbound: boolean) => {
			const weight = new Map<string, number>();
			for (const e of g.edges) {
				if (e.kind !== 'calls') continue;
				const inner = inbound ? e.target_id : e.source_id;
				const outer = inbound ? e.source_id : e.target_id;
				if (inner === null || outer === null || !inside.has(inner) || inside.has(outer)) continue;
				weight.set(outer, (weight.get(outer) ?? 0) + 1);
			}
			const list = [...weight]
				.map(([id, w]) => ({ node: p.byId.get(id) as GraphNode, weight: w }))
				.sort((a, b) => b.weight - a.weight || (a.node.name < b.node.name ? -1 : a.node.name > b.node.name ? 1 : 0));
			return {
				list: list.slice(0, NEIGHBOUR_BUDGET).map(({ node, weight }) => ({ id: node.id, name: node.name, file: node.file_path, weight })),
				total: list.length
			};
		};
		const callers = side(true);
		const callees = side(false);
		return {
			ok: true,
			data: {
				level: 'symbol',
				file: path,
				path,
				role: fileRole(path),
				symbols,
				callers: callers.list,
				callersTotal: callers.total,
				callees: callees.list,
				calleesTotal: callees.total
			}
		};
	}

	async function zonesread(): Promise<Read<Zones>> {
		const p = await load();
		const g = await read();
		// Abstract by declaration, as `component_zones`: interface/trait, or a
		// class something implements/extends.
		const implemented = new Set<string>();
		for (const e of g.edges) {
			if ((e.kind === 'implements' || e.kind === 'extends') && e.target_id !== null) implemented.add(e.target_id);
		}
		const typesOf = new Map<string, { types: number; abstract: number }>();
		for (const n of g.nodes) {
			if (n.file_path === null || !TYPE_KINDS.has(n.kind)) continue;
			const cur = typesOf.get(n.folder_path) ?? { types: 0, abstract: 0 };
			cur.types++;
			if (n.kind === 'interface' || n.kind === 'trait' || (n.kind === 'class' && implemented.has(n.id))) cur.abstract++;
			typesOf.set(n.folder_path, cur);
		}
		const libraries = new Set<string>();
		for (const e of g.edges) {
			// A keyless external names a library; a keyed-but-unresolved edge
			// names an unfiled internal file — not a library.
			if (e.external && e.target_folder === null && e.target_name !== null) libraries.add(e.target_name);
		}
		const zoneEdges = g.edges
			.filter((e) => KINDS.includes(e.kind))
			.flatMap((e) => {
				const s = p.byId.get(e.source_id);
				if (!s || s.file_path === null) return [];
				if (e.target_id !== null) {
					const t = p.byId.get(e.target_id);
					if (!t || t.file_path === null) return [];
					return [{ source: s.folder_path, target: t.folder_path }];
				}
				if (e.target_name !== null && libraries.has(e.target_name)) return [{ source: s.folder_path, target: e.target_name }];
				return [];
			});
		const points = zonePoints(
			p.modules.map((m) => ({ component: m.id, types: typesOf.get(m.id)?.types ?? 0, abstract: typesOf.get(m.id)?.abstract ?? 0 })),
			zoneEdges,
			libraries
		);
		const placed = points.map((pt) => pt.distance).filter((d): d is number => d !== null);
		const unplaceable = points.filter((pt) => pt.distance === null).length;
		return {
			ok: true,
			data: {
				// The wire carries null abstractness/instability for unplaceable
				// points — exactly what the daemon sends against this same
				// type. `zonesView` filters by zone before any arithmetic, so
				// the lens never touches a null.
				points: points as unknown as Zones['points'],
				meanDistance: placed.length > 0 ? placed.reduce((a, b) => a + b, 0) / placed.length : null,
				coverage: { points: points.length, unplaceable }
			}
		};
	}

	return {
		level0: level0read,
		module: moduleread,
		file: fileread,
		zones: zonesread,
		cochange: async (): Promise<Read<CoChange>> =>
			({ ok: true, data: { hidden: [], pairs: 0, commits: 0, minTogether: COCHANGE_MIN } }),
		complexity: async (): Promise<Read<ComplexityTree>> =>
			({ ok: false, error: 'complexity Measures are not synced to the dōjō' })
	};
}
