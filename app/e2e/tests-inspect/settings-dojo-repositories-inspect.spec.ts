/**
 * WP2 against the REAL daemon: Settings › Dōjō connection lists, under the
 * selected identity's dōjōs, every repository its last sync pass decided about —
 * one row each — and a skipped row reads its remedy from `sensei.reason_codes`
 * (in a refusal Notice), not a raw code. Run:
 * `bun run test:inspect -- settings-dojo-repositories`.
 */
import { expect, test } from '../fixtures';
import { DAEMON_URL, navigateTo } from '../helpers';

type Row = { entity: string; entity_key: string; state: string; remedy: string | null; persona: string | null };
type Persona = { label: string; sessionSlot: string | null; isDefault?: boolean };

test("the selected identity's repositories show each verdict with its remedy", async ({ tauriPage }) => {
  const body = await (await fetch(`${DAEMON_URL}/api/dojo/sync-state`)).json();
  const { personas }: { personas: Persona[] } = await (await fetch(`${DAEMON_URL}/api/auth/personas`)).json();
  const selected = personas.find((p) => p.isDefault) ?? personas[0];
  const rows: Row[] = (body.entities ?? []).filter((r: Row) => r.entity === 'repository' && r.persona === selected?.sessionSlot);
  test.skip(rows.length === 0, 'no repository verdicts for the default identity on this daemon yet');

  await navigateTo(tauriPage, '/settings/dojo');
  await expect
    .poll(() => tauriPage.evaluate(`document.querySelectorAll('[data-repository]').length`), { timeout: 30_000 })
    .toBe(new Set(rows.map((r) => r.entity_key)).size);

  const skipped = rows.filter((r) => r.state === 'skipped' && r.remedy);
  const sample = [...new Map(skipped.map((r) => [r.remedy, r])).values()];
  for (const r of sample) {
    const text = (await tauriPage.evaluate(
      `document.querySelector('[data-repository=${JSON.stringify(r.entity_key)}] [data-component="notice"]')?.textContent ?? ''`,
    )) as string;
    expect(text, r.entity_key).toContain(r.remedy!);
  }
  const summary = (await tauriPage.evaluate(`document.querySelector('[data-summary]')?.textContent?.trim() ?? ''`)) as string;
  console.log(`\n[inspect] dōjō repositories: ${rows.length} rows · ${summary} · remedies seen ${sample.map((r) => r.remedy).join(' | ')}`);
});
