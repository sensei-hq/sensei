<script lang="ts">
  // Workspace window · Sessions — this workspace's sessions, through the one
  // Sessions screen the Observatory uses.
  //
  // Re-created whenever the loaded workspace changes — NOT untracked. This is an
  // `[id]` route, so SvelteKit reuses the component when only the param
  // changes: seeding the state once froze the fetcher's workspace id and the rows
  // to the first workspace mounted, and workspace B rendered workspace A's sessions.
  // Constructing the state only assigns (no fetch), and the range chips mutate
  // the instance rather than `data`, so a 30d/90d pick survives.
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { SessionsDigestScreen } from '$lib/components/sessions';
  import { SessionsDigestState, sessionsFetcher } from '$lib/sessions-digest.svelte.js';
  import { sessionsCopy, type SessionRange } from '$lib/sessions-digest.js';

  let { data } = $props();

  const digest = $derived.by(
    () =>
      new SessionsDigestState(
        sessionsFetcher(senseiApi(appState.port), data.workspaceId),
        data.sessions,
        data.range as SessionRange,
        data.error,
        data.activity,
      ),
  );
  const copy = $derived(sessionsCopy(data.workspace?.name ?? 'workspace'));
</script>

<SessionsDigestScreen {digest} {copy} testid="workspace-sessions" />
