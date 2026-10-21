// Naming a team (#307 A3) — one draft for both "New team" and "Rename". A blank
// name is not sent; whether the write worked is the save's to say (it returns
// false and the screen shows why), and a failed save keeps the draft open.

/** Save a name: `teamId` null creates, otherwise renames. Resolves whether it saved. */
export type SaveTeam = (teamId: string | null, name: string) => Promise<boolean>;

export class TeamDraft {
	open = $state(false);
	/** The team being renamed; null while creating. */
	teamId = $state<string | null>(null);
	name = $state('');

	start(team?: { id: string; name: string }): void {
		this.teamId = team?.id ?? null;
		this.name = team?.name ?? '';
		this.open = true;
	}

	cancel(): void {
		this.open = false;
	}

	async submit(save: SaveTeam): Promise<void> {
		const name = this.name.trim();
		if (!name) return;
		if (await save(this.teamId, name)) this.open = false;
	}
}
