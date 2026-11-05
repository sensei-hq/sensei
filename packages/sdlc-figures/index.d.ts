/** The reason domain the SDLC figures' codes live in. */
export declare const SDLC_REASONS: 'sdlc_metrics';

/** One figure as shown: its value, or (when null) its reason's summary. */
export interface SdlcFigure {
	key: string;
	label: string;
	text: string;
	/** The text is a reason's summary, not a value. */
	reason: boolean;
}

/** Reason codes' words, by code. */
export type Words = Record<string, { summary: string }>;

/** A figure's reason code, by figure key. */
export type SdlcReasons = Record<string, string>;

export declare function pct(x: number): string;
export declare function points(x: number): string;
export declare function reasonText(code: string, words: Words): string;
export declare function sdlcFigure(
	key: string,
	label: string,
	value: number | null | undefined,
	format: (x: number) => string,
	reasons: SdlcReasons,
	words: Words
): SdlcFigure;

export declare const SPRINT_FIGURES: ReadonlyArray<readonly [string, string, (x: number) => string]>;

/** The sprint figures a SprintSdlc carries, and its reasons. */
export interface SprintFigureValues {
	commitment_reliability: number | null;
	total_completion: number | null;
	scope_change: number | null;
	carry_over_rate: number | null;
	unplanned_work_ratio: number | null;
	velocity: number | null;
	points_commitment_reliability: number | null;
	reasons: SdlcReasons;
}

export declare function sprintSdlcFigures(s: SprintFigureValues, words: Words): SdlcFigure[];

export declare function compactDuration(mins: number | null): string;
export declare function compactTokens(tokens: number | null | undefined): string;
export declare function activeMinutes(durationSecs: number | null | undefined): number | null;
export declare function apiCostText(cost: number | null | undefined, partial: boolean): string;
