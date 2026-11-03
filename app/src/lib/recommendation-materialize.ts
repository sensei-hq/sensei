// What to send when accepting a recommendation materializes it — read from the
// daemon's preview, the one source (review r1 M4): a file-class preview
// (`consent_required`) sends nothing extra; a rule-class one sends back the scope
// and tier the preview proposed. Never a literal a screen guessed.
import type { RecommendationPreview } from './api.js';

export interface MaterializeOptions {
    gov_scope?: string;
    enforcement?: string;
}

/** pending: actionitems */
export function materializeOptions(_preview: RecommendationPreview): MaterializeOptions {
    throw new Error('pending: actionitems');
}
