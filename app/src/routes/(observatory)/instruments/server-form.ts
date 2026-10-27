// Observatory · Instruments — the "add a server" form (#343), read into the
// body the daemon registers. Pure.

export type ServerForm = { name: string; command: string; args: string; env: string };

export type ServerBody = { name: string; command: string; args: string[]; env: Record<string, string> };

/** One argument per line (an argument may contain spaces); one `KEY=VALUE`
 *  per environment line, the value taken whole after the first `=`. */
export function parseServerForm(form: ServerForm): { ok: true; server: ServerBody } | { ok: false; error: string } {
  const name = form.name.trim();
  const command = form.command.trim();
  if (!name) return { ok: false, error: 'A name is required' };
  if (!command) return { ok: false, error: 'A command is required' };
  const lines = (text: string) => text.split('\n').map((l) => l.trim()).filter((l) => l !== '');
  const env: Record<string, string> = {};
  for (const line of lines(form.env)) {
    const at = line.indexOf('=');
    if (at <= 0) return { ok: false, error: `Environment line "${line}" is not KEY=VALUE` };
    env[line.slice(0, at)] = line.slice(at + 1);
  }
  return { ok: true, server: { name, command, args: lines(form.args), env } };
}
