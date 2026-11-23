import { describe, it, expect } from 'vitest';
import { createServer, type Server } from 'http';
import { captureSse } from './sse-capture.js';

// SSE capture for e2e: subscribe to a daemon stream from Node, collect
// frames, wait for a predicate. Proven against a canned server here so the
// helper itself never needs a live daemon to trust.
describe('captureSse', () => {
  let server: Server | null = null;
  const serve = (body: string) =>
    new Promise<string>((resolve) => {
      server = createServer((_req, res) => {
        res.writeHead(200, { 'Content-Type': 'text/event-stream' });
        res.end(body);
      }).listen(0, '127.0.0.1', () => {
        const addr = server!.address();
        resolve(`http://127.0.0.1:${typeof addr === 'object' && addr ? addr.port : 0}/events`);
      });
    });
  const stop = () => new Promise<void>((resolve) => (server ? server.close(() => resolve()) : resolve()));

  it('collects data frames and parses JSON payloads', async () => {
    const url = await serve('data: {"a":1}\n\ndata: {"b":2}\n\n');
    const cap = captureSse(url);
    try {
      const second = await cap.waitFor((e) => (e.data as { b?: number }).b === 2, 5000);
      expect(second.data).toEqual({ b: 2 });
      expect(cap.events).toHaveLength(2);
    } finally {
      cap.close();
      await stop();
    }
  });

  it('ignores comments and joins multi-line data', async () => {
    const url = await serve(': ping\n\ndata: {"a":\ndata: 1}\n\n');
    const cap = captureSse(url);
    try {
      const first = await cap.waitFor(() => true, 5000);
      expect(first.data).toEqual({ a: 1 });
    } finally {
      cap.close();
      await stop();
    }
  });

  it('sends caller headers (Last-Event-ID resume)', async () => {
    let seen: string | undefined;
    const srv = createServer((req, res) => {
      seen = req.headers['last-event-id'] as string | undefined;
      res.writeHead(200, { 'Content-Type': 'text/event-stream' });
      res.end('data: {"a":1}\n\n');
    });
    const url = await new Promise<string>((resolve) => {
      srv.listen(0, '127.0.0.1', () => {
        const addr = srv.address();
        resolve(`http://127.0.0.1:${typeof addr === 'object' && addr ? addr.port : 0}/e`);
      });
    });
    const cap = captureSse(url, { headers: { 'Last-Event-ID': '41' } });
    try {
      await cap.waitFor(() => true, 5000);
      expect(seen).toBe('41');
    } finally {
      cap.close();
      await new Promise<void>((resolve) => srv.close(() => resolve()));
      await stop();
    }
  });

  it('keeps undecodable payloads as raw text and times out cleanly', async () => {
    const url = await serve('data: not-json\n\n');
    const cap = captureSse(url);
    try {
      const first = await cap.waitFor(() => true, 5000);
      expect(first.data).toBe('not-json');
      await expect(cap.waitFor(() => false, 100)).rejects.toThrow(/timeout/);
    } finally {
      cap.close();
      await stop();
    }
  });
});
