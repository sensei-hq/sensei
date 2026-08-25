import { describe, it, expect } from 'vitest';
import { metricKanji, kanjiKeys, isMockupKanji } from './metric-kanji.js';

// The rail draws one glyph per signal, so the map has to cover every metric the
// daemon can return. This list is the registry as of the metric-rating work
// (sensei.metrics, 27 rows) — when a metric is added, this test is what says
// "give it a glyph" instead of shipping a gap in the rail.
const REGISTRY = [
	'context_pressure_rate',
	'ftr',
	'rework_ratio',
	'run_completion',
	'spec_depth',
	'spec_deviation_rate',
	'tokens_in_per_day',
	'tokens_out_per_day',
	'tokens_per_day',
	'tokens_per_result',
	'session_duration',
	'throughput',
	'time_to_useful_result',
	'churn_concentration',
	'churn_rate',
	'coverage',
	'duplication_ratio',
	'incomplete_analysis_llm_rate',
	'incomplete_analysis_rate',
	'module_quality',
	'refuted_finding_rate',
	'rework_density',
	'false_crash_rate',
	'interruption_rate',
	'memory_promotion',
	'unused_tools',
	'project_health',
];

/** The eleven the mockups actually specify — these are not ours to change. */
const FROM_MOCKUP: Array<[string, string]> = [
	['project_health', '健'],
	['churn_concentration', '紋'],
	['churn_rate', '流'],
	['time_to_useful_result', '連'],
	['throughput', '量'],
	['ftr', '果'],
	['rework_ratio', '戻'],
	['rework_density', '密'],
	['duplication_ratio', '双'],
	['memory_promotion', '覚'],
	['interruption_rate', '断'],
];

describe('metricKanji', () => {
	it.each(REGISTRY)('%s has a glyph', (key) => {
		expect(metricKanji(key)).toMatch(/^\p{Script=Han}$/u);
	});

	it.each(FROM_MOCKUP)('%s keeps the mockup glyph %s', (key, glyph) => {
		expect(metricKanji(key)).toBe(glyph);
		expect(isMockupKanji(key)).toBe(true);
	});

	it('assigns a distinct glyph to every metric', () => {
		// Two metrics sharing a mark makes the rail ambiguous at a glance.
		const glyphs = kanjiKeys().map((k) => metricKanji(k));
		expect(new Set(glyphs).size).toBe(glyphs.length);
	});

	it('returns null for an unknown key rather than inventing a mark', () => {
		expect(metricKanji('not_a_metric')).toBeNull();
		expect(metricKanji('')).toBeNull();
		expect(isMockupKanji('not_a_metric')).toBe(false);
	});

	it('does not reuse a section glyph the metrics screens already spend', () => {
		// 観 signals rail, 察 what-moved, 具 tools-used, 測 the screen itself.
		// Reusing one on a metric row would read as a heading, not a signal.
		const sectionGlyphs = ['観', '察', '具', '測'];
		const assigned = kanjiKeys().map((k) => metricKanji(k));
		for (const g of sectionGlyphs) expect(assigned).not.toContain(g);
	});
});
