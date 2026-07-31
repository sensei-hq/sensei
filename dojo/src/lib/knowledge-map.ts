// Pure wire→kit mapper for the maintainer Knowledge screen (dojo
// `/org/[slug]/knowledge`). Takes the `client-data.ts` `KnowledgeLibraryWire`
// (published `dojo.artifacts` partitioned into active/pending/catalog + the prune
// window) and projects it onto the presentational `KitKnowledge` the shipped
// `ScrKnowledge` already declares — so the screen renders real data with no
// component change. Reuses the shared `kindKanji` / `relativeAge` triage helpers;
// side-effect-free, `now` injected for a deterministic age.
import type { KnowledgeArtifactWire, KnowledgeLibraryWire } from './client-data';
import { kindKanji, relativeAge } from './triage/view';
import type { KitKnowledge, KitKnowledgeRow, KitCatalogItem } from './components/kit/types';

/**
 * Readable label for an artifact's `scope` jsonb ({company|team|project|stack}).
 * Defensive: reads the common keys, falls back to "Unscoped"; never throws.
 */
export function scopeLabel(scope: unknown): string {
	if (scope && typeof scope === 'object') {
		const s = scope as Record<string, unknown>;
		if (typeof s.team === 'string' && s.team) return `Team · ${s.team}`;
		if (typeof s.project === 'string' && s.project) return `Project · ${s.project}`;
		if (typeof s.stack === 'string' && s.stack) return `Stack · ${s.stack}`;
		if (s.company) return typeof s.company === 'string' && s.company !== 'true' ? `Company · ${s.company}` : 'Company';
	}
	return 'Unscoped';
}

/** An active-library row: kind glyph · title · scope · adoption reach · published age. */
function activeRow(a: KnowledgeArtifactWire, now: Date): KitKnowledgeRow {
	return {
		kanji: kindKanji(a.kind),
		title: a.title,
		scope: scopeLabel(a.scope),
		adopted: `${a.adopted_count} ${a.adopted_count === 1 ? 'repo' : 'repos'}`,
		age: `published ${relativeAge(a.created_at, now)}`
	};
}

/** A pending-prune row: no adoption reach; the age reads how long it's sat unused. */
function pendingRow(a: KnowledgeArtifactWire, now: Date): KitKnowledgeRow {
	return {
		kanji: kindKanji(a.kind),
		title: a.title,
		scope: scopeLabel(a.scope),
		age: `unused ${relativeAge(a.created_at, now)}`
	};
}

/** A catalog (extension) row: kind glyph · title · kind chip · scope. */
function catalogRow(a: KnowledgeArtifactWire): KitCatalogItem {
	return { kanji: kindKanji(a.kind), title: a.title, kind: a.kind, scope: scopeLabel(a.scope) };
}

/** KnowledgeLibraryWire → KitKnowledge. Pure. */
export function toKitKnowledge(lib: KnowledgeLibraryWire, now: Date = new Date()): KitKnowledge {
	return {
		prunePolicy:
			lib.retention_days != null ? `Prune after ${lib.retention_days} days unused` : 'No prune policy',
		active: lib.active.map((a) => activeRow(a, now)),
		pending: lib.pending.map((a) => pendingRow(a, now)),
		catalog: lib.catalog.map(catalogRow)
	};
}
