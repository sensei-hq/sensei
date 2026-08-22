// One kanji per metric, for the signal rail's glyph column.
//
// The mockups (docs/mockups/Sensei/screenshots/01-metrics-inspector.png,
// 01-wk.png) show a glyph before every signal name. Eleven metrics are read
// straight off those screenshots and are the design's own choices. The rest
// postdate the mockups, so their glyphs are PROPOSED here — semantically derived
// to sit alongside the originals. They are collected in one map precisely so
// they are cheap to overrule: change the character, nothing else moves.
//
// A key with no entry returns null rather than a stand-in glyph. A plausible but
// wrong character is worse than none in a product whose visual language is
// carried by these marks, and the rail reserves the column either way so names
// stay aligned.

/** Glyphs taken from the mockups — the design's own assignments. */
const FROM_MOCKUP: Record<string, string> = {
	project_health: '健', // health
	churn_concentration: '紋', // crest/pattern — where change concentrates
	churn_rate: '流', // flow
	time_to_useful_result: '連', // continuity, elapsing
	throughput: '量', // quantity
	ftr: '果', // result/fruit
	rework_ratio: '戻', // return, going back
	rework_density: '密', // density
	duplication_ratio: '双', // paired, doubled
	memory_promotion: '覚', // to remember/become aware
	interruption_rate: '断', // to cut off
};

/**
 * Proposed glyphs for metrics added after the mockups were drawn. Each is chosen
 * to read the same way as the originals — one concrete character naming what the
 * metric measures, not an abbreviation of its English label.
 */
const PROPOSED: Record<string, string> = {
	context_pressure_rate: '圧', // pressure
	run_completion: '完', // completion
	spec_depth: '深', // depth
	spec_deviation_rate: '逸', // to stray from
	tokens_in_per_day: '入', // in
	tokens_out_per_day: '出', // out
	tokens_per_day: '費', // expenditure
	tokens_per_result: '価', // cost/worth — spend per result
	session_duration: '続', // to continue
	coverage: '覆', // to cover
	incomplete_analysis_llm_rate: '未', // not yet, unfinished
	incomplete_analysis_rate: '早', // premature — edited before reading
	module_quality: '保', // to maintain/preserve
	refuted_finding_rate: '否', // to deny
	false_crash_rate: '誤', // mistaken
	unused_tools: '休', // idle, at rest
};

const METRIC_KANJI: Record<string, string> = { ...FROM_MOCKUP, ...PROPOSED };

/** The glyph for a metric key, or null when none is assigned. */
export function metricKanji(key: string): string | null {
	return METRIC_KANJI[key] ?? null;
}

/** Every key that carries a glyph — used by tests to assert full registry cover. */
export function kanjiKeys(): string[] {
	return Object.keys(METRIC_KANJI);
}

/** True when the glyph came from the mockups rather than being proposed here. */
export function isMockupKanji(key: string): boolean {
	return key in FROM_MOCKUP;
}
