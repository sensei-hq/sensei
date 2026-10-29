<script lang="ts">
  // Observatory · Sessions — every workspace's sessions, through the one Sessions
  // screen. No route param, so the state is seeded once from the load; the
  // range chips refetch through it thereafter.
  import { untrack } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { SessionsDigestScreen } from '$lib/components/sessions';
  import { SessionsDigestState, sessionsFetcher } from '$lib/sessions-digest.svelte.js';
  import { sessionsCopy, type SessionRange } from '$lib/sessions-digest.js';

  let { data } = $props();

  const digest = new SessionsDigestState(
    sessionsFetcher(senseiApi(appState.port)),
    untrack(() => data.sessions),
    untrack(() => data.range as SessionRange),
    untrack(() => data.error),
    untrack(() => data.activity),
  );
</script>

<SessionsDigestScreen {digest} copy={sessionsCopy(null)} />
