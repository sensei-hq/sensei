// packages/engine/src/lib/infer-source-type.ts

export type InferredSource =
  | { source_type: 'llms.txt'; base_url: string }
  | { source_type: 'github';   base_url: string }
  | { source_type: 'http';     base_url: string }
  | { source_type: 'local';    base_url: string };

/**
 * Infer source_type and base_url from a user-provided URL or path.
 *
 * Detection order:
 *  1. URL ending in /llms.txt or .txt          → llms.txt
 *  2. github.com/{owner}/{repo}/tree/* URL    → github
 *  3. Any other http(s) URL                    → http
 *  4. Absolute filesystem path                 → local (normalized to file:// URL)
 *  5. file:// URL                              → local or llms.txt
 */
export function inferSourceType(input: string): InferredSource {
  // Already a file:// URL
  if (input.startsWith('file://')) {
    return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: input };
  }

  // HTTP/HTTPS URLs
  if (input.startsWith('http://') || input.startsWith('https://')) {
    try {
      const url = new URL(input);
      if (url.pathname.endsWith('/llms.txt') || url.pathname.endsWith('.txt')) {
        return { source_type: 'llms.txt', base_url: input };
      }
      if (url.hostname === 'github.com' && /^\/[^/]+\/[^/]+\/tree\//.test(url.pathname)) {
        return { source_type: 'github', base_url: input };
      }
    } catch {
      // Malformed URL — fall through to http type
    }
    return { source_type: 'http', base_url: input };
  }

  // Absolute filesystem path — normalize to file:// URL
  const fileUrl = input.startsWith('/') ? `file://${input}` : `file:///${input}`;
  return { source_type: input.endsWith('.txt') ? 'llms.txt' : 'local', base_url: fileUrl };
}
