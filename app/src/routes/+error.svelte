<script lang="ts">
  // A route that failed to load: the shared Notice at screen size, so it reads
  // like every other failure (docs/architecture/notice.md). Words: notice.js.
  import { page } from '$app/state';
  import { Notice, noticeForRouteError } from '@sensei/ui';

  const notice = $derived(noticeForRouteError(page.status, page.error?.message));
</script>

<Notice
  size="screen"
  {...notice}
  primary={{ label: 'Go to the observatory', href: '/', name: 'home' }}
  data-route-error={page.status}
/>
