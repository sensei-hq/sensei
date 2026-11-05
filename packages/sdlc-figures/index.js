// SDLC figures as sensei shows them, in both the desktop app and the dōjō
// (spec docs/spec/daemon/tracker-sdlc-metrics.md S6, S8;
// tracker-delivery-metrics.md D2, D5): a value in its unit, or — when the figure is
// null — its reason's words. A real 0 is a value, never a reason or "—".

/** @typedef {{ key: string, label: string, text: string, reason: boolean }} SdlcFigure One figure as shown: its value, or (when null) its reason's summary. */
/** @typedef {Record<string, { summary: string }>} Words Reason codes' words, by code. */
/** @typedef {Record<string, string>} SdlcReasons A figure's reason code, by figure key. */
/**
 * The sprint figures a SprintSdlc carries, and its reasons.
 * @typedef {{
 *   commitment_reliability: number | null,
 *   total_completion: number | null,
 *   scope_change: number | null,
 *   carry_over_rate: number | null,
 *   unplanned_work_ratio: number | null,
 *   velocity: number | null,
 *   points_commitment_reliability: number | null,
 *   reasons: SdlcReasons
 * }} SprintFigureValues
 */

/** The reason domain the SDLC figures' codes live in (tracker-sdlc-metrics S6). */
export const SDLC_REASONS = 'sdlc_metrics';

/**
 * A whole percent: "80%".
 * @param {number} x
 * @returns {string}
 */
export const pct = (x) => `${Math.round(x)}%`;

/**
 * Points to one decimal: "21 pts", "12.5 pts".
 * @param {number} x
 * @returns {string}
 */
export const points = (x) => `${Number(x.toFixed(1))} pts`;

/**
 * A reason code's summary; a code with no words is a fault and says so.
 * @param {string} code
 * @param {Words} words
 * @returns {string}
 */
export function reasonText(code, words) {
	return words[code]?.summary ?? 'Reason not recognised';
}

/**
 * A figure's value, or its reason's summary; "—" when it has neither.
 * @param {string} key
 * @param {string} label
 * @param {number | null | undefined} value
 * @param {(x: number) => string} format
 * @param {SdlcReasons} reasons
 * @param {Words} words
 * @returns {SdlcFigure}
 */
export function sdlcFigure(key, label, value, format, reasons, words) {
	if (value != null) return { key, label, text: format(value), reason: false };
	const code = reasons[key];
	if (code == null) return { key, label, text: '—', reason: false };
	return { key, label, text: reasonText(code, words), reason: true };
}

/**
 * A sprint's seven figures, in order: six percents, and velocity in points.
 * @type {ReadonlyArray<readonly [keyof Omit<SprintFigureValues, 'reasons'>, string, (x: number) => string]>}
 */
export const SPRINT_FIGURES = [
	['commitment_reliability', 'commitment reliability', pct],
	['total_completion', 'total completion', pct],
	['scope_change', 'scope change', pct],
	['carry_over_rate', 'carry-over', pct],
	['unplanned_work_ratio', 'unplanned work', pct],
	['velocity', 'velocity', points],
	['points_commitment_reliability', 'points commitment', pct]
];

/**
 * A sprint's seven figures (S6) from its SprintSdlc.
 * @param {SprintFigureValues} s
 * @param {Words} words
 * @returns {SdlcFigure[]}
 */
export function sprintSdlcFigures(s, words) {
	return SPRINT_FIGURES.map(([key, label, format]) =>
		sdlcFigure(key, label, s[key], format, s.reasons, words)
	);
}
