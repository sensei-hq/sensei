// packages/engine/src/lib/infer-source-type.ts

export type InferredSource =
  | { source_type: 'llms.txt'; base_url: string; local_path?: undefined }
  | { source_type: 'github';   base_url: string; local_path?: undefined }
  | { source_type: 'http';     base_url: string; local_path?: undefined }
  | { source_type: 'local';    local_path: string; base_url?: undefined };

/**
 * Infer source_type, base_url, and local_path from a user-provided URL or path.
 *
 * Detection order:
 *  1. URL ending in /llms.txt          → llms.txt
 *  2. github.com/{owner}/{repo}/tree/* → github
 *  3. Any other http(s) URL            → http
 *  4. Anything else                    → local
 */
export function inferSourceType(input: string): InferredSource {
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
  return { source_type: 'local', local_path: input };
}
