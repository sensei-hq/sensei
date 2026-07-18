<script lang="ts">
	import { Button } from '@rokkit/ui';
	import { env } from '$env/dynamic/public';
	import {
		subscribeToPush,
		pushSupported,
		postNotificationPrefs,
		type SubscribeResult
	} from '$lib/relay-push';

	// Relay "enable push notifications" header action (relay P4.3, CLIENT half).
	// A progressive, graceful toggle: it reads the browser's push capability and
	// Notification.permission on mount and renders the right affordance — a
	// "Notify me" button when it can ask, or a muted status line when the browser
	// can't do push / has already granted / has blocked it. Delegates the actual
	// SW-register → PushManager.subscribe → store flow to subscribeToPush (which
	// never throws on the "can't" paths, returning a typed SubscribeResult), then
	// opts the caller into the events they'd want. SSR-safe: every browser-only
	// read is behind pushSupported(), and the initial state is seeded once (no
	// $effect) so it resolves 'unsupported' under SSR and corrects on hydration.
	let { tenantKey, accessToken }: { tenantKey: string; accessToken: string | null } = $props();

	type ToggleState = 'idle' | 'enabled' | 'denied' | 'unsupported' | 'error';

	// Resolve the starting state from the live browser capability + permission.
	// Pure over browser globals: pushSupported() guards navigator/window so this
	// is SSR-safe (returns 'idle' server-side, and the $state seed re-runs on the
	// client render). No $effect needed — enable() then owns further transitions.
	function initialState(): ToggleState {
		if (!pushSupported()) return 'unsupported';
		if (Notification.permission === 'granted') return 'enabled';
		if (Notification.permission === 'denied') return 'denied';
		return 'idle';
	}

	const seed = initialState();
	let toggleState = $state<ToggleState>(seed);
	let status = $state(seed === 'enabled' ? 'Notifications on' : '');
	let busy = $state(false);

	// Map the non-ok SubscribeResult reason onto our display state. 'no-vapid-key'
	// is a config gap, not a user-facing capability — surface it as a plain error.
	function stateForReason(reason: Exclude<SubscribeResult, { ok: true }>['reason']): ToggleState {
		switch (reason) {
			case 'unsupported':
				return 'unsupported';
			case 'denied':
				return 'denied';
			default:
				return 'error';
		}
	}

	async function enable() {
		if (busy) return;
		busy = true;
		try {
			const res = await subscribeToPush(tenantKey, env.PUBLIC_VAPID_KEY, { accessToken });
			if (res.ok) {
				// A prefs failure shouldn't undo a good subscription — the browser is
				// subscribed either way. Count it as enabled, but soft-warn.
				try {
					await postNotificationPrefs(
						tenantKey,
						{ approvals: true, stalls: true, crashed: true },
						{ accessToken }
					);
					status = 'Notifications on';
				} catch {
					status = 'Notifications on (preferences not saved)';
				}
				toggleState = 'enabled';
			} else {
				toggleState = stateForReason(res.reason);
				status = res.message;
			}
		} finally {
			busy = false;
		}
	}
</script>

{#if toggleState === 'enabled'}
	<span class="text-ink-mute text-xs" style="white-space: nowrap">
		<span class="kanji text-accent text-xs">鈴</span>
		{status || 'Notifications on'}
	</span>
{:else if toggleState === 'unsupported'}
	<span class="text-ink-faint text-xs" style="white-space: nowrap">Notifications not supported here</span>
{:else if toggleState === 'denied'}
	<span class="text-ink-faint text-xs" style="white-space: nowrap">
		Notifications blocked — enable them in your browser settings
	</span>
{:else}
	<div class="flex flex-col items-end" style="gap: 4px">
		<Button variant="primary" size="sm" onclick={enable} disabled={busy}>
			<span class="kanji text-xs">鈴</span>
			{busy ? 'Enabling…' : 'Notify me'}
		</Button>
		{#if toggleState === 'error' && status}
			<span class="text-danger text-xs" style="white-space: nowrap">{status}</span>
		{/if}
	</div>
{/if}
