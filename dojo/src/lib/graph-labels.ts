// The environment labels a tracked graph branch may carry.
//
// One list for the route (which refuses anything outside it as a 400) and the
// screen (which offers it as the label picker). The table enforces the same
// set with its CHECK — not by convention: graph-branches-labels.spec.ts reads
// the CHECK out of the DDL and fails when the two disagree.
export const GRAPH_LABELS = ['production', 'testing', 'development'] as const;

/** One of the labels `GRAPH_LABELS` knows. */
export type GraphLabel = (typeof GRAPH_LABELS)[number];
