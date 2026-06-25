<script lang="ts">
    import { PageHeader, EmptyState } from '$lib/components';
</script>

<!-- MOCK: stub page — flesh from docs/mockups/Sensei/lib/impact.jsx (wire daemon API later) -->
<PageHeader
    variant="h2"
    eyebrow="Observatory · Change impact"
    kanji="果"
    title="Did sensei's advice actually work?"
    description="Each accepted recommendation gets a measurement window — FTR, corrections, tool usage and session duration, compared before vs after."
/>

<EmptyState
    kanji="果"
    title="no measurements yet"
    description="Accepted recommendations and their before/after results will appear here once this view is wired to the daemon."
/>
