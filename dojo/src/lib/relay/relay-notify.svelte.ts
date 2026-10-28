// "Notify me" — relay push notifications for the inbox (relay P4.3, client half;
// in the inbox header since 2026-10-28). The inbox spans every dōjō you belong
// to and a push subscription is stored per dōjō, so enabling registers this
// browser with each. Everything the toggle shows is decided here (audit
// 2026-10-28, item 13: the state machine lived in the component).
import type { SubscribeResult } from '$lib/relay-push';

export interface RelayNotifyDeps {
	supported: () => boolean;
	permission: () => NotificationPermission;
	subscribe: (tenantKey: string) => Promise<SubscribeResult>;
	/** Opt the caller into the events they would want; throws when not saved. */
	postPrefs: (tenantKey: string) => Promise<void>;
}

type Phase = 'idle' | 'enabled' | 'denied' | 'unsupported' | 'error';

export type NotifyView =
	| { kind: 'on'; text: string }
	| { kind: 'status'; text: string }
	| { kind: 'button'; buttonLabel: string; busy: boolean; error: string | null };

export class RelayNotify {
	phase = $state<Phase>('idle');
	message = $state<string | null>(null);
	busy = $state(false);
	#deps: RelayNotifyDeps;

	constructor(deps: RelayNotifyDeps) {
		this.#deps = deps;
		// SSR-safe: `supported` guards the browser globals, so the server renders
		// 'unsupported' and the client corrects on hydration.
		if (!deps.supported()) this.phase = 'unsupported';
		else if (deps.permission() === 'granted') this.phase = 'enabled';
		else if (deps.permission() === 'denied') this.phase = 'denied';
	}

	get view(): NotifyView {
		switch (this.phase) {
			case 'enabled':
				return { kind: 'on', text: this.message ?? 'Notifications on' };
			case 'unsupported':
				return { kind: 'status', text: 'Notifications not supported here' };
			case 'denied':
				return { kind: 'status', text: 'Notifications blocked — enable them in your browser settings' };
			default:
				return {
					kind: 'button',
					buttonLabel: this.busy ? 'Enabling…' : 'Notify me',
					busy: this.busy,
					error: this.phase === 'error' ? this.message : null
				};
		}
	}

	/** Register this browser with every dōjō in the inbox. On when at least one
	 *  took it; which did not is said. */
	async enable(tenantKeys: string[]): Promise<void> {
		if (this.busy) return;
		if (tenantKeys.length === 0) {
			this.phase = 'error';
			this.message = 'No dōjō to notify you about yet.';
			return;
		}
		this.busy = true;
		try {
			const failed: string[] = [];
			let prefsLost = false;
			let ok = 0;
			for (const t of tenantKeys) {
				const res = await this.#deps.subscribe(t);
				if (!res.ok) {
					// The browser's answer is the same for every dōjō: stop asking.
					if (res.reason === 'unsupported' || res.reason === 'denied') {
						this.phase = res.reason;
						this.message = res.message;
						return;
					}
					failed.push(`${t}: ${res.message}`);
					continue;
				}
				ok += 1;
				// A prefs failure does not undo a good subscription.
				await this.#deps.postPrefs(t).catch(() => (prefsLost = true));
			}
			if (ok === 0) {
				this.phase = 'error';
				this.message = failed.join('; ');
				return;
			}
			this.phase = 'enabled';
			this.message = failed.length
				? `Notifications on for ${ok} of ${tenantKeys.length} dōjōs — ${failed.join('; ')}`
				: prefsLost
					? 'Notifications on (preferences not saved)'
					: 'Notifications on';
		} finally {
			this.busy = false;
		}
	}
}
