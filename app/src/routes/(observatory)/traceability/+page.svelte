<script lang="ts">
    import { PageHeader, EmptyState } from '$lib/components';
</script>

<!-- MOCK: stub page — flesh from docs/mockups/Sensei/lib/traceability.jsx (wire daemon API later) -->
<PageHeader
    variant="h2"
    eyebrow="Observatory · Document traceability"
    kanji="巻"
    title="Where the docs and the code disagree."
    description="Every doc-to-symbol link, checked nightly. Drift becomes visible here before someone reads stale docs and writes the wrong thing."
/>

<EmptyState
    kanji="巻"
    title="no drift detected yet"
    description="Doc-to-code links and their drift will appear here once this view is wired to the daemon."
/>
