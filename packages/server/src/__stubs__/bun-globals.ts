/**
 * Vitest setup file: polyfill Bun globals for Node.js test runs.
 *
 * Implements the subset of the Bun API used by serve.ts:
 *   Bun.serve({ port, fetch }) → { stop(), port }
 *
 * Uses Node's built-in `http` module to create a real HTTP server so the
 * spec's `fetch(...)` calls work against an actual listening socket.
 */
import * as http from "http";

interface BunServeOptions {
  port?: number;
  fetch: (req: Request) => Response | Promise<Response>;
}

interface BunServer {
  stop(): void;
  port: number;
}

function bunServe(opts: BunServeOptions): BunServer {
  const server = http.createServer(async (nodeReq, nodeRes) => {
    const url = `http://localhost${nodeReq.url}`;
    const method = nodeReq.method ?? "GET";
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries(nodeReq.headers)) {
      if (typeof v === "string") headers[k] = v;
      else if (Array.isArray(v)) headers[k] = v.join(", ");
    }

    let body: BodyInit | undefined;
    if (method !== "GET" && method !== "HEAD") {
      body = await new Promise<Buffer>((resolve, reject) => {
        const chunks: Buffer[] = [];
        nodeReq.on("data", (c: Buffer) => chunks.push(c));
        nodeReq.on("end", () => resolve(Buffer.concat(chunks)));
        nodeReq.on("error", reject);
      });
    }

    const webReq = new Request(url, { method, headers, body: body?.length ? body : undefined });
    let webRes: Response;
    try {
      webRes = await opts.fetch(webReq);
    } catch (err) {
      nodeRes.writeHead(500);
      nodeRes.end("Internal server error");
      return;
    }

    nodeRes.writeHead(webRes.status, Object.fromEntries(webRes.headers.entries()));
    const buf = await webRes.arrayBuffer();
    nodeRes.end(Buffer.from(buf));
  });

  const listenPort = opts.port ?? 0;
  server.listen(listenPort);

  // Obtain the actual bound port (in case 0 was passed)
  const address = server.address() as { port: number } | null;
  const actualPort = address?.port ?? listenPort;

  return {
    stop() {
      server.close();
    },
    port: actualPort,
  };
}

// Install Bun global if not already present (i.e. running under Node/Vitest)
if (typeof globalThis.Bun === "undefined") {
  (globalThis as unknown as Record<string, unknown>).Bun = {
    serve: bunServe,
  };
}
