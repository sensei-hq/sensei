// A spec file: excluded whatever it contains.
const rows = await fetch('/x').catch(() => []);
const items = rows ?? [];
