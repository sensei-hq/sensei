// What to send when accepting a recommendation materializes it — read from the
// daemon's preview, the one source (review r1 M4): a file-class preview
// (`consent_required`) sends nothing extra; a rule-class one sends back the scope
// and tier the preview proposed. Never a literal a screen guessed.
import type { RecommendationPreview } from './api.js';

export interface MaterializeOptions {
    gov_scope?: string;
    enforcement?: string;
}

export function materializeOptions(preview: RecommendationPreview): MaterializeOptions {
    if (preview.consent_required) return {};
    const options: MaterializeOptions = {};
    if (preview.gov_scope) options.gov_scope = preview.gov_scope;
    if (preview.enforcement) options.enforcement = preview.enforcement;
    return options;
}
