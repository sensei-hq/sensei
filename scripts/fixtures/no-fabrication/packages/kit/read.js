// Fixture: a shared package that fetches.
export async function read(url) {
	const r = await fetch(url).catch(() => undefined); // expect: T1
	return r;
}
