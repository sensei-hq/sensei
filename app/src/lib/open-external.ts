// A link out of the app (#381). The Tauri webview ignores target=_blank, so a link opens
// in the system browser through the opener plugin. Outside Tauri (a plain browser in dev)
// the plugin refuses, and the link opens in a new tab instead.
import { openUrl } from '@tauri-apps/plugin-opener';

/** Open `href` in the system browser; a new tab where there is no Tauri. */
export async function openExternal(href: string): Promise<void> {
  try {
    await openUrl(href);
  } catch {
    window.open(href, '_blank', 'noopener');
  }
}

/** An anchor's click handler: the href stays on the anchor (hover, copy, keyboard), the
 *  open goes to the system browser. */
export function externalClick(event: MouseEvent): void {
  event.preventDefault();
  void openExternal((event.currentTarget as HTMLAnchorElement).href);
}
