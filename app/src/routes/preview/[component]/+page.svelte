<script lang="ts">
  // One component's preview: drawn by the dev-only module the load hands over.
  import type { PageData } from './$types.js';

  let { data }: { data: PageData } = $props();
</script>

<data.page component={data.component} entries={data.entries} components={data.components} view={data.view} />
