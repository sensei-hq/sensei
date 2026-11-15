// The route refuses an unknown label with a 400, and the table refuses it with
// its CHECK. Both name the same set — and this pins that, by reading the
// table's CHECK out of the DDL, so a label added in one place and not the
// other fails here instead of drifting.
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { GRAPH_LABELS } from '$lib/graph-labels';

const DDL = resolve(
	import.meta.dirname,
	'../../../../../../../database/ddl/table/dojo/repository_graph_branches.ddl'
);

function checkLabels(): string[] {
	const text = readFileSync(DDL, 'utf8');
	const m = /check\s*\(\s*label\s+is\s+null\s+or\s+label\s+in\s*\(([^)]+)\)/i.exec(text);
	if (!m) throw new Error('the label CHECK is gone from repository_graph_branches.ddl');
	return [...m[1].matchAll(/'([^']+)'/g)].map((x) => x[1]).sort();
}

describe('graph branch labels', () => {
	it('the route and the table know the same labels', () => {
		expect([...GRAPH_LABELS].sort()).toEqual(checkLabels());
	});
});
