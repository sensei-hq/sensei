<script lang="ts">
  // The development preview's index: drawn by the dev-only module the load hands over.
  import type { PageData } from './$types.js';

  let { data }: { data: PageData } = $props();
</script>

<data.page components={data.components} />
