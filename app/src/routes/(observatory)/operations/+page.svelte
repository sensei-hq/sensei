<script lang="ts">
  // Observatory · Operations — the one live surface for background work:
  // Now, Queue, Failures, Schedules, Pipelines, Sync. The sections read the
  // ops singletons; the page only attaches them, so navigation never loses
  // their state.
  import { PageHeader, Screen } from '$lib/components';
  import OpsNowSection from '$lib/components/operations/OpsNowSection.svelte';
  import OpsQueueSection from '$lib/components/operations/OpsQueueSection.svelte';
  import OpsFailuresSection from '$lib/components/operations/OpsFailuresSection.svelte';
  import OpsSchedulesSection from '$lib/components/operations/OpsSchedulesSection.svelte';
  import OpsPipelinesSection from '$lib/components/operations/OpsPipelinesSection.svelte';
  import OpsSyncSection from '$lib/components/operations/OpsSyncSection.svelte';
  import { railIcon } from '../observatory-nav';
  import { jobsState, schedulesState, syncState } from '$lib/ops.js';

  $effect(() => {
    void jobsState.attach();
    void schedulesState.attach();
    void syncState.attach();
  });
</script>

<Screen data-operations>
  {#snippet header()}
    <PageHeader
      kanji="運"
      icon={railIcon('/operations')}
      eyebrow="Operations · Background work"
      title="Every job, queue, and schedule, live."
      description="What is running now, what waits, what failed and why, when each worker runs next, and when every entity last agreed with the dōjō."
    />
  {/snippet}
  {#snippet children()}
    <OpsNowSection />
    <OpsQueueSection />
    <OpsFailuresSection />
    <OpsSchedulesSection />
    <OpsPipelinesSection />
    <OpsSyncSection />
  {/snippet}
</Screen>
