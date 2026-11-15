// A request body read with a byte cap that holds whether or not the client
// declared a length: a declared length over the cap is refused before reading,
// and a streamed body is counted as it arrives and cut off — never read whole —
// once it passes the cap.

/** The body as text, or `null` when it is over `maxBytes`. */
export async function readCapped(request: Request, maxBytes: number): Promise<string | null> {
	const declared = Number(request.headers.get('content-length'));
	if (Number.isFinite(declared) && declared > maxBytes) return null;
	if (!request.body) return '';
	const reader = request.body.getReader();
	const chunks: Uint8Array[] = [];
	let size = 0;
	for (;;) {
		const { done, value } = await reader.read();
		if (done) break;
		size += value.byteLength;
		if (size > maxBytes) {
			await reader.cancel();
			return null;
		}
		chunks.push(value);
	}
	const bytes = new Uint8Array(size);
	let at = 0;
	for (const c of chunks) {
		bytes.set(c, at);
		at += c.byteLength;
	}
	return new TextDecoder().decode(bytes);
}
