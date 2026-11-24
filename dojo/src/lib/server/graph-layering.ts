// The dependency layering behind the dōjō's Layers and Cycles diagrams (#268):
// a faithful port of the daemon's `analysis::layering`, which the dōjō cannot
// call — cycles first (Tarjan), then longest-path ranks over the condensation,
// one greedy feedback order per cycle (Eades–Lin–Smyth), and a verdict per
// edge. Same inputs give the same outputs as the daemon; the specs pin the
// cases the Rust suite pins. Ported, not shared: the daemon is Rust, this is
// the dōjō's TypeScript, and a port with pinned cases beats an RPC the dōjō
// cannot make. If the algorithm changes upstream, this file changes with it.

export interface Dep {
	source: string;
	target: string;
	occurrences: number;
}

export type Conformance = 'down' | 'skip' | 'up' | 'level';

export interface Placed {
	id: string;
	/** 0 at the top, foundations below — the daemon's convention. */
	layer: number;
	/** Positional, not durable: renumbered per call. Never persisted. */
	component: number;
	/** Units sharing this component; 1 means "in no cycle". */
	component_size: number;
}

export interface ClassifiedDep extends Dep {
	conformance: Conformance;
	/** The one edge inside a cycle that is cheapest to cut. */
	weakest: boolean;
}

export interface Cycle {
	component: number;
	/** Sorted, so the caption is stable between calls. */
	members: string[];
	layer: number;
	/** The dependencies closing the loop, heaviest first. */
	inner: Dep[];
	/** The weakest link: fewest occurrences. Cheapest, not the back edge. */
	cut: Dep | null;
}

export interface Layering {
	units: Placed[];
	deps: ClassifiedDep[];
	/** Multi-member components only, by descending size then first member. */
	cycles: Cycle[];
	/** Unit-onto-itself deps: dropped from the graph, kept here (cohesion). */
	self_deps: Dep[];
	/** Deps naming a unit outside the universe: dropped AND counted. */
	dropped: Dep[];
	/** `max(layer) + 1`, or 0 with no units. */
	depth: number;
}

/** Byte-order string comparison, matching Rust's `Ord for String` on valid
 *  strings (UTF-8 preserves code-point order, as does UTF-16 comparison). */
function cmpStr(a: string, b: string): number {
	return a < b ? -1 : a > b ? 1 : 0;
}

type IxEdge = [s: number, t: number, w: number];

/** Rank a dependency graph: cycles first, then layers over the condensation.
 *  `units` is authoritative — a dep naming a unit outside it is DROPPED. */
export function analyse(units: string[], deps: Dep[]): Layering {
	const ids = [...new Set(units)].sort(cmpStr);
	const index = new Map(ids.map((s, i) => [s, i] as const));
	const n = ids.length;

	const { edges, self_deps, dropped } = partitionDeps(deps, index);
	const adjacency = adjacencyOf(n, edges);
	const componentOf = stronglyConnected(n, adjacency);
	const layerOf = rankCondensation(n, edges, componentOf);
	const sizes = componentSizes(componentOf);
	const placed = place(ids, componentOf, layerOf, sizes);
	const cuts = weakestLinks(edges, ids, componentOf);

	const feedbackPos = new Map<number, number>();
	const byComponent = new Map<number, number[]>();
	for (let u = 0; u < n; u++) {
		if (sizes[componentOf[u]] > 1) {
			const list = byComponent.get(componentOf[u]) ?? [];
			list.push(u);
			byComponent.set(componentOf[u], list);
		}
	}
	for (const members of byComponent.values()) {
		for (const [node, pos] of feedbackOrder(members, edges)) feedbackPos.set(node, pos);
	}
	const depsOut = classify(edges, ids, componentOf, layerOf, cuts, feedbackPos);
	const cycles = collapse(ids, componentOf, layerOf, sizes, edges, cuts);
	const depth = layerOf.length === 0 ? 0 : Math.max(...layerOf) + 1;
	return { units: placed, deps: depsOut, cycles, self_deps, dropped, depth };
}

function partitionDeps(deps: Dep[], index: Map<string, number>): { edges: IxEdge[]; self_deps: Dep[]; dropped: Dep[] } {
	const edges: IxEdge[] = [];
	const self_deps: Dep[] = [];
	const dropped: Dep[] = [];
	for (const d of deps) {
		const s = index.get(d.source);
		const t = index.get(d.target);
		if (s === undefined || t === undefined) dropped.push({ ...d });
		else if (s === t) self_deps.push({ ...d });
		else edges.push([s, t, d.occurrences]);
	}
	// Parallel edges fold: layering asks WHETHER one unit depends on another.
	// Occurrences sum — the cut heuristic weighs them.
	edges.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
	const folded: IxEdge[] = [];
	for (const [s, t, w] of edges) {
		const last = folded[folded.length - 1];
		if (last && last[0] === s && last[1] === t) last[2] += w;
		else folded.push([s, t, w]);
	}
	self_deps.sort((a, b) => cmpStr(a.source, b.source) || b.occurrences - a.occurrences);
	return { edges: folded, self_deps, dropped };
}

function adjacencyOf(n: number, edges: IxEdge[]): number[][] {
	const adj: number[][] = Array.from({ length: n }, () => []);
	for (const [s, t] of edges) adj[s].push(t);
	return adj;
}

/** Tarjan's SCC, iterative — recursion overflows on the deep graphs code
 *  reliably produces. Component indices follow completion order over the
 *  sorted units, so identical graphs number identically. */
function stronglyConnected(n: number, adjacency: number[][]): number[] {
	const UNVISITED = -1;
	const indexOf = new Array<number>(n).fill(UNVISITED);
	const low = new Array<number>(n).fill(0);
	const onStack = new Array<boolean>(n).fill(false);
	const component = new Array<number>(n).fill(UNVISITED);
	const stack: number[] = [];
	let nextIndex = 0;
	let nextComponent = 0;

	for (let root = 0; root < n; root++) {
		if (indexOf[root] !== UNVISITED) continue;
		const call: Array<[node: number, walked: number]> = [[root, 0]];
		indexOf[root] = nextIndex;
		low[root] = nextIndex;
		nextIndex++;
		stack.push(root);
		onStack[root] = true;

		while (call.length > 0) {
			const frame = call[call.length - 1];
			const v = frame[0];
			if (frame[1] < adjacency[v].length) {
				const w = adjacency[v][frame[1]];
				frame[1]++;
				if (indexOf[w] === UNVISITED) {
					indexOf[w] = nextIndex;
					low[w] = nextIndex;
					nextIndex++;
					stack.push(w);
					onStack[w] = true;
					call.push([w, 0]);
				} else if (onStack[w]) {
					low[v] = Math.min(low[v], indexOf[w]);
				}
				continue;
			}
			if (low[v] === indexOf[v]) {
				for (;;) {
					const w = stack.pop() as number;
					onStack[w] = false;
					component[w] = nextComponent;
					if (w === v) break;
				}
				nextComponent++;
			}
			call.pop();
			if (call.length > 0) {
				const parent = call[call.length - 1][0];
				low[parent] = Math.min(low[parent], low[v]);
			}
		}
	}
	return component;
}

/** Longest path from a source over the condensation: 0 where nothing depends
 *  on the unit, one below the deepest depender. One Kahn pass — each
 *  component finalises after its dependers, so the max is settled on read. */
function rankCondensation(n: number, edges: IxEdge[], componentOf: number[]): number[] {
	const count = componentOf.length === 0 ? 0 : Math.max(...componentOf) + 1;
	const succ: number[][] = Array.from({ length: count }, () => []);
	const inDegree = new Array<number>(count).fill(0);
	const seen = new Set<string>();
	for (const [s, t] of edges) {
		const [cs, ct] = [componentOf[s], componentOf[t]];
		if (cs === ct || seen.has(`${cs}>${ct}`)) continue;
		seen.add(`${cs}>${ct}`);
		succ[cs].push(ct);
		inDegree[ct]++;
	}
	const layer = new Array<number>(count).fill(0);
	const queue: number[] = [];
	for (let c = 0; c < count; c++) if (inDegree[c] === 0) queue.push(c);
	while (queue.length > 0) {
		const c = queue.shift() as number;
		for (const d of succ[c]) {
			layer[d] = Math.max(layer[d], layer[c] + 1);
			inDegree[d]--;
			if (inDegree[d] === 0) queue.push(d);
		}
	}
	return Array.from({ length: n }, (_, u) => layer[componentOf[u]]);
}

function componentSizes(componentOf: number[]): number[] {
	const count = componentOf.length === 0 ? 0 : Math.max(...componentOf) + 1;
	const sizes = new Array<number>(count).fill(0);
	for (const c of componentOf) sizes[c]++;
	return sizes;
}

function place(ids: string[], componentOf: number[], layerOf: number[], sizes: number[]): Placed[] {
	return ids.map((id, u) => ({ id, layer: layerOf[u], component: componentOf[u], component_size: sizes[componentOf[u]] }));
}

/** Cheapest edge to cut per multi-member component: fewest occurrences (the
 *  mockup's rule), ties on unit names so the cut is stable. */
function weakestLinks(edges: IxEdge[], ids: string[], componentOf: number[]): Map<number, [s: number, t: number]> {
	const best = new Map<number, { w: number; s: string; t: string; si: number; ti: number }>();
	for (const [s, t, w] of edges) {
		if (componentOf[s] !== componentOf[t]) continue;
		const cand = { w, s: ids[s], t: ids[t], si: s, ti: t };
		const cur = best.get(componentOf[s]);
		if (!cur || cand.w < cur.w || (cand.w === cur.w && (cmpStr(cand.s, cur.s) < 0 || (cand.s === cur.s && cmpStr(cand.t, cur.t) < 0)))) {
			best.set(componentOf[s], cand);
		}
	}
	return new Map([...best].map(([c, b]) => [c, [b.si, b.ti] as [number, number]]));
}

/** Greedy linear arrangement (Eades–Lin–Smyth): backwards edges in this order
 *  are a feedback arc set. Deterministic — ties break on member index. The
 *  order is NOT a layer and must never become one. */
function feedbackOrder(members: number[], edges: IxEdge[]): Map<number, number> {
	const inside = new Set(members);
	const within: Array<[s: number, t: number]> = edges
		.filter(([s, t]) => s !== t && inside.has(s) && inside.has(t))
		.map(([s, t]) => [s, t]);

	const alive = new Set(members);
	const front: number[] = [];
	const back: number[] = [];
	const degrees = (): Map<number, [out: number, inn: number]> => {
		const out = new Map<number, [number, number]>([...alive].map((m) => [m, [0, 0]]));
		for (const [s, t] of within) {
			if (alive.has(s) && alive.has(t)) {
				const ds = out.get(s) as [number, number];
				const dt = out.get(t) as [number, number];
				ds[0]++;
				dt[1]++;
			}
		}
		return out;
	};
	const ascending = () => [...alive].sort((a, b) => a - b);

	while (alive.size > 0) {
		for (;;) {
			const deg = degrees();
			const sink = ascending().find((m) => (deg.get(m) as [number, number])[0] === 0);
			if (sink !== undefined) {
				alive.delete(sink);
				back.push(sink);
				continue;
			}
			const source = ascending().find((m) => (deg.get(m) as [number, number])[1] === 0);
			if (source !== undefined) {
				alive.delete(source);
				front.push(source);
				continue;
			}
			break;
		}
		if (alive.size === 0) break;
		// Cyclic here: take the node whose out-degree most exceeds its
		// in-degree (ties: smallest index), costing the fewest back edges.
		const deg = degrees();
		let pick = -1;
		let pickKey = Number.NEGATIVE_INFINITY;
		for (const m of ascending()) {
			const [o, i] = deg.get(m) as [number, number];
			const key = (o - i) * 1048576 - m;
			if (key > pickKey) {
				pickKey = key;
				pick = m;
			}
		}
		alive.delete(pick);
		front.push(pick);
	}
	back.reverse();
	const order = [...front, ...back];
	return new Map(order.map((node, pos) => [node, pos]));
}

function classify(
	edges: IxEdge[],
	ids: string[],
	componentOf: number[],
	layerOf: number[],
	cuts: Map<number, [number, number]>,
	feedbackPos: Map<number, number>
): ClassifiedDep[] {
	return edges.map(([s, t, w]) => {
		const same = componentOf[s] === componentOf[t];
		let conformance: Conformance;
		if (same) {
			// Inside a cycle ranks agree, so the feedback order names the
			// arrow closing the loop — the call breaking the downward flow.
			const a = feedbackPos.get(s);
			const b = feedbackPos.get(t);
			conformance = a !== undefined && b !== undefined && a > b ? 'up' : 'level';
		} else {
			const d = layerOf[t] - layerOf[s];
			conformance = d === 1 ? 'down' : d > 1 ? 'skip' : d === 0 ? 'level' : 'up';
		}
		const cut = cuts.get(componentOf[s]);
		return { source: ids[s], target: ids[t], occurrences: w, conformance, weakest: same && !!cut && cut[0] === s && cut[1] === t };
	});
}

function collapse(
	ids: string[],
	componentOf: number[],
	layerOf: number[],
	sizes: number[],
	edges: IxEdge[],
	cuts: Map<number, [number, number]>
): Cycle[] {
	const byComponent = new Map<number, { members: string[]; layer: number }>();
	ids.forEach((id, u) => {
		if (sizes[componentOf[u]] > 1) {
			const entry = byComponent.get(componentOf[u]) ?? { members: [], layer: 0 };
			entry.members.push(id);
			entry.layer = layerOf[u];
			byComponent.set(componentOf[u], entry);
		}
	});
	const cycles: Cycle[] = [...byComponent].map(([component, { members, layer }]) => {
		members.sort(cmpStr);
		const inner: Dep[] = edges
			.filter(([s, t]) => componentOf[s] === component && componentOf[t] === component)
			.map(([s, t, w]) => ({ source: ids[s], target: ids[t], occurrences: w }));
		inner.sort((a, b) => b.occurrences - a.occurrences || cmpStr(a.source, b.source) || cmpStr(a.target, b.target));
		const cutIx = cuts.get(component);
		const cut = cutIx ? (inner.find((d) => d.source === ids[cutIx[0]] && d.target === ids[cutIx[1]]) ?? null) : null;
		return { component, members, layer, inner, cut };
	});
	cycles.sort((a, b) => b.members.length - a.members.length || cmpStr(a.members[0], b.members[0]));
	return cycles;
}
