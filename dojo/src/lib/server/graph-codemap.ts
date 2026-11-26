// The Code map's math for the dōjō (#268): a faithful port of the daemon's
// `analysis::codemap` L0/L1 pieces — roles from paths, role columns, block
// folding with hubs, focus scope — plus Martin zones over graph rows. The
// port (`graph-port.ts`) feeds these from `graph-reads.ts`; the components
// cannot tell the dōjō's answers from the daemon's. Cases mirror the Rust
// suite; `level0`/`level1`/`fileRole` are additionally diffed against the
// Rust implementation (see the slice commit).

export const BLOCK_BUDGET = 60;
export const COLUMN_BUDGET = 12;
const HUB_SHARE = 0.05;
const HUB_MIN_CALLERS = 5;
const HUB_REACH = 0.25;

export type Role = 'code' | 'test' | 'docs' | 'config' | 'generated';
export type RoleCounts = Record<Role, number>;
const ROLE_ORDER: Role[] = ['code', 'test', 'docs', 'config', 'generated'];

export interface ModuleComplexity {
	functions: number;
	complex: number;
}

export interface Module {
	id: string;
	label: string;
	group: string;
	pkg: string;
	files: number;
	symbols: number;
	roles: RoleCounts;
	complexity: ModuleComplexity | null;
}

export interface ModuleEdge {
	source: string;
	target: string;
	kind: string;
	count: number;
}

export interface Block {
	id: string;
	label: string;
	group: string;
	package: string | null;
	files: number;
	symbols: number;
	roles: RoleCounts;
	hub: boolean;
	weight_in: number;
	weight_out: number;
	folded: string[];
	complexity: (ModuleComplexity & { tier: 'danger' | 'warning' | 'neutral' | null }) | null;
}

export interface Band {
	source: string;
	target: string;
	weight: number;
	kinds: Record<string, number>;
}

export interface Level0Out {
	budget: { drawn: number; modules: number; bands: number; edges: number };
	blocks: Block[];
	bands: Band[];
}

export interface ModuleFile {
	id: string;
	path: string;
	symbols: number;
}

export interface Column {
	role: Role;
	files: ModuleFile[];
	more: number;
	more_symbols: number;
}

function cmpStr(a: string, b: string): number {
	return a < b ? -1 : a > b ? 1 : 0;
}

function zeroRoles(): RoleCounts {
	return { code: 0, test: 0, docs: 0, config: 0, generated: 0 };
}

function addRoles(a: RoleCounts, b: RoleCounts): RoleCounts {
	return { code: a.code + b.code, test: a.test + b.test, docs: a.docs + b.docs, config: a.config + b.config, generated: a.generated + b.generated };
}

/** What a file is, from its path — generated wins, since a generated test is
 *  not the developer's test. */
export function fileRole(path: string): Role {
	const lower = path.toLowerCase();
	const segments = lower.split('/');
	const dirs = segments.slice(0, -1);
	const name = segments[segments.length - 1] ?? '';
	const original = path.split('/').pop() ?? path;
	const inDir = (names: string[]): boolean => dirs.some((d) => names.includes(d));
	const ext = name.includes('.') ? (name.split('.').pop() as string) : '';

	if (
		inDir(['generated', '__generated__', 'gen', 'dist', 'target']) ||
		name.includes('.generated.') ||
		name.endsWith('.pb.go') ||
		name.endsWith('_pb2.py') ||
		name.endsWith('.g.dart') ||
		name.endsWith('.d.ts')
	) {
		return 'generated';
	}
	if (
		inDir(['tests', 'test', '__tests__', 'e2e', 'spec', 'specs']) ||
		name.includes('.spec.') ||
		name.includes('.test.') ||
		name.endsWith('_test.go') ||
		name.endsWith('_test.rs') ||
		name.endsWith('_tests.rs') ||
		(name.startsWith('test_') && ext === 'py') ||
		original.endsWith('Test.java') ||
		original.endsWith('Tests.cs') ||
		original.endsWith('Test.kt')
	) {
		return 'test';
	}
	if (['md', 'mdx', 'rst', 'adoc', 'txt'].includes(ext) || inDir(['docs', 'doc'])) return 'docs';
	if (
		['json', 'toml', 'yaml', 'yml', 'ini', 'cfg', 'conf', 'lock'].includes(ext) ||
		name.includes('.config.') ||
		name === 'makefile' ||
		name === 'dockerfile' ||
		name === '.env'
	) {
		return 'config';
	}
	return 'code';
}

/** L1: one module's files in columns by role; an empty role has no column. */
export function level1(files: ModuleFile[]): Column[] {
	const byRole = new Map<Role, ModuleFile[]>();
	for (const f of files) {
		const role = fileRole(f.path);
		byRole.set(role, [...(byRole.get(role) ?? []), f]);
	}
	return ROLE_ORDER.filter((r) => byRole.has(r)).map((role) => {
		const fs = (byRole.get(role) as ModuleFile[]).sort((a, b) => b.symbols - a.symbols || cmpStr(a.path, b.path));
		const rest = fs.slice(COLUMN_BUDGET);
		return {
			role,
			files: fs.slice(0, COLUMN_BUDGET),
			more: rest.length,
			more_symbols: rest.reduce((n, f) => n + f.symbols, 0)
		};
	});
}

export function foldId(group: string): string {
	return `fold:${group}`;
}

export interface Focused {
	modules: Module[];
	edges: ModuleEdge[];
	/** The kept modules that are not focused, sorted. */
	neighbours: string[];
}

/** Narrow modules and edges to `members` and their direct neighbours. Runs
 *  before `level0`, so nothing in the neighbourhood is folded away. */
export function focusScope(modules: Module[], edges: ModuleEdge[], members: Set<string>): Focused {
	const known = new Set(modules.map((m) => m.id));
	const keptEdges = edges.filter(
		(e) => (members.has(e.source) || members.has(e.target)) && known.has(e.source) && known.has(e.target)
	);
	const neighbours = [
		...new Set(keptEdges.flatMap((e) => [e.source, e.target]).filter((id) => !members.has(id)))
	].sort(cmpStr);
	const keptModules = modules.filter((m) => members.has(m.id) || neighbours.includes(m.id));
	return { modules: keptModules, edges: keptEdges, neighbours };
}

/** The modules a Group-by segment holds: every module whose group it is. */
export function segmentMembers(modules: Module[], segment: string): Set<string> {
	return new Set(modules.filter((m) => m.group === segment).map((m) => m.id));
}

function complexityTier(functions: number, complex: number): 'danger' | 'warning' | 'neutral' | null {
	if (functions === 0) return null;
	if (complex / functions > 0.2) return 'danger';
	if (complex / functions > 0.1) return 'warning';
	return 'neutral';
}

/** L0 from module-level nodes and edges: hubs aside, the largest drawn,
 *  the smallest folded per group, one band per drawn pair. */
export function level0(modules: Module[], edges: ModuleEdge[], budget: number): Level0Out {
	const between = edges.filter((e) => e.source !== e.target);
	const total = between.reduce((n, e) => n + e.count, 0);

	const inbound = new Map<string, number>();
	const callers = new Map<string, Set<string>>();
	for (const e of between) {
		inbound.set(e.target, (inbound.get(e.target) ?? 0) + e.count);
		if (!callers.has(e.target)) callers.set(e.target, new Set());
		callers.get(e.target)?.add(e.source);
	}
	const minCallers = Math.max(HUB_MIN_CALLERS, Math.ceil(HUB_REACH * modules.length));
	const isHub = (id: string): boolean =>
		total > 0 && (inbound.get(id) ?? 0) > HUB_SHARE * total && (callers.get(id)?.size ?? 0) >= minCallers;

	const ranked = modules.filter((m) => !isHub(m.id)).sort((a, b) => b.symbols - a.symbols || cmpStr(a.id, b.id));
	const hubs = modules.length - ranked.length;
	const room = Math.max(0, budget - hubs);
	let keep = Math.min(ranked.length, room);
	const foldsFor = (k: number): number => new Set(ranked.slice(k).map((m) => m.group)).size;
	while (keep > 0 && keep + foldsFor(keep) > room) keep--;

	const keptIds = new Set(ranked.slice(0, keep).map((m) => m.id));
	const drawnAs = new Map<string, string>();
	for (const m of modules) {
		const folded = !isHub(m.id) && !keptIds.has(m.id);
		drawnAs.set(m.id, folded ? foldId(m.group) : m.id);
	}

	const blocks = new Map<string, Block>();
	for (const m of modules) {
		const id = drawnAs.get(m.id) as string;
		let block = blocks.get(id);
		if (!block) {
			block = {
				id,
				label: id === m.id ? m.label : '',
				group: m.group,
				package: m.pkg,
				files: 0,
				symbols: 0,
				roles: zeroRoles(),
				hub: isHub(m.id),
				weight_in: 0,
				weight_out: 0,
				folded: [],
				complexity: null
			};
			blocks.set(id, block);
		}
		block.files += m.files;
		block.symbols += m.symbols;
		block.roles = addRoles(block.roles, m.roles);
		if (block.package !== m.pkg) block.package = null;
		if (m.complexity) {
			const sum = block.complexity ?? { functions: 0, complex: 0, tier: null };
			sum.functions += m.complexity.functions;
			sum.complex += m.complexity.complex;
			sum.tier = complexityTier(sum.functions, sum.complex);
			block.complexity = sum;
		}
		if (block.id !== m.id) block.folded.push(m.id);
	}

	const bands = new Map<string, Band>();
	for (const e of between) {
		const s = drawnAs.get(e.source);
		const t = drawnAs.get(e.target);
		if (s === undefined || t === undefined) continue;
		const bs = blocks.get(s);
		const bt = blocks.get(t);
		if (bs) bs.weight_out += e.count;
		if (bt) bt.weight_in += e.count;
		if (s === t || isHub(e.source) || isHub(e.target)) continue;
		const key = `${s}\u0000${t}`;
		let band = bands.get(key);
		if (!band) {
			band = { source: s, target: t, weight: 0, kinds: {} };
			bands.set(key, band);
		}
		band.weight += e.count;
		band.kinds[e.kind] = (band.kinds[e.kind] ?? 0) + e.count;
	}

	const blockList = [...blocks.values()];
	for (const b of blockList) {
		if (b.folded.length > 0) {
			b.folded.sort(cmpStr);
			b.label = `+${b.folded.length} more`;
		}
	}
	blockList.sort(
		(a, b) =>
			cmpStr(a.group, b.group) ||
			// Folds before plain blocks within the group (the daemon's
			// reversed emptiness comparison, stated plainly).
			(a.folded.length > 0 ? 0 : 1) - (b.folded.length > 0 ? 0 : 1) ||
			b.symbols - a.symbols ||
			cmpStr(a.id, b.id)
	);
	const bandList = [...bands.values()].sort((a, b) => cmpStr(a.source, b.source) || cmpStr(a.target, b.target));
	return { budget: { drawn: blockList.length, modules: modules.length, bands: bandList.length, edges: total }, blocks: blockList, bands: bandList };
}

export interface ZoneInput {
	component: string;
	types: number;
	abstract: number;
}

export interface ZoneEdge {
	source: string;
	target: string;
}

export interface ZonePoint {
	component: string;
	types: number;
	abstractTypes: number;
	ca: number;
	ce: number;
	abstractness: number | null;
	instability: number | null;
	distance: number | null;
	zone: 'main' | 'pain' | 'useless' | 'risk' | null;
}

/** Martin's abstractness against instability, one point per component — the
 *  daemon's `component_zones` over graph rows. Ca/Ce count DISTINCT partners;
 *  a self-edge is cohesion, not coupling; Ce includes library packages.
 *  NULL where the denominator is empty — never 0-as-maximal. */
export function zones(components: ZoneInput[], edges: ZoneEdge[], libraries: Set<string>): ZonePoint[] {
	const known = new Set(components.map((c) => c.component));
	const caPartners = new Map<string, Set<string>>();
	const ceInternal = new Map<string, Set<string>>();
	const ceExternal = new Map<string, Set<string>>();
	for (const { source, target } of edges) {
		if (!known.has(source) || source === target) continue;
		if (known.has(target)) {
			if (!caPartners.has(target)) caPartners.set(target, new Set());
			caPartners.get(target)?.add(source);
			if (!ceInternal.has(source)) ceInternal.set(source, new Set());
			ceInternal.get(source)?.add(target);
		} else if (libraries.has(target)) {
			if (!ceExternal.has(source)) ceExternal.set(source, new Set());
			ceExternal.get(source)?.add(target);
		}
	}
	return components.map(({ component, types, abstract }) => {
		const ca = caPartners.get(component)?.size ?? 0;
		const ce = (ceInternal.get(component)?.size ?? 0) + (ceExternal.get(component)?.size ?? 0);
		const abstractness = types > 0 ? abstract / types : null;
		const instability = ca + ce > 0 ? ce / (ca + ce) : null;
		const distance = abstractness !== null && instability !== null ? Math.abs(abstractness + instability - 1) : null;
		const zone =
			distance === null || abstractness === null || instability === null
				? null
				: distance > 0.4
					? abstractness + instability < 1
						? 'pain'
						: 'useless'
					: distance > 0.25
						? 'risk'
						: 'main';
		return { component, types, abstractTypes: abstract, ca, ce, abstractness, instability, distance, zone };
	});
}

/** Referenced libraries as L0 draws them: how many, and the most used. */
export function librariesSummary(rows: Array<{ name: string; uses: number }>, top: number): { count: number; top: string[] } {
	const ranked = [...rows].sort((a, b) => b.uses - a.uses || cmpStr(a.name, b.name));
	return { count: rows.length, top: ranked.slice(0, top).map((r) => r.name) };
}
