import { describe, expect, it } from 'vitest';
import { deflateSync } from 'node:zlib';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { shot } from './shot';

// #284: WKWebView paints the page area some time after the DOM settles, and a
// capture taken at once can come back flat — wholly (blank) or in the page
// band while the frame and docked devtools paint (unpainted). shot() must let
// the paint land before the first capture and recapture while it is flat,
// instead of reporting the first flat frame.

/** A minimal PNG: `w`×`h`, 8-bit RGBA, rows from `pixel(x, y)`. */
function png(w: number, h: number, pixel: (x: number, y: number) => number[]): Buffer {
  const raw = Buffer.alloc(h * (1 + w * 4));
  let o = 0;
  for (let y = 0; y < h; y++) {
    raw[o++] = 0; // filter: none
    for (let x = 0; x < w; x++) for (const c of pixel(x, y)) raw[o++] = c;
  }
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    return Buffer.concat([len, Buffer.from(type), data, Buffer.alloc(4)]); // CRC unchecked
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

// 50 rows: a varied title bar in the top 6%, then the page area (PAGE_BAND).
const GOOD = png(8, 50, (x, y) =>
  y < 3 ? [x * 30, 0, 0, 255] : y === 20 && x === 4 ? [200, 0, 0, 255] : [40, 40, 40, 255],
);
const UNPAINTED = png(8, 50, (x, y) => (y < 3 ? [x * 30, 0, 0, 255] : [40, 40, 40, 255]));
const BLANK = png(8, 50, () => [255, 255, 255, 255]);

/** A tauriPage whose captures play back `frames` in order (last repeats). */
function mockPage(frames: Buffer[]): {
  page: { evaluate: (s: string) => Promise<unknown>; screenshot: (o: { path: string }) => Promise<unknown> };
  events: string[];
} {
  const events: string[] = [];
  let calls = 0;
  return {
    events,
    page: {
      evaluate: async () => {
        events.push('eval');
        return undefined;
      },
      screenshot: async ({ path }: { path: string }) => {
        events.push('shot');
        writeFileSync(path, frames[Math.min(calls++, frames.length - 1)]);
      },
    },
  };
}

const FAST = { settleMs: 5, retryGapMs: 1 };

describe('shot', () => {
  it('keeps a good first capture', async () => {
    const { page, events } = mockPage([GOOD]);
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(page, path, FAST)).resolves.toBe('ok');
    expect(events.filter((e) => e === 'shot')).toHaveLength(1);
    expect(readFileSync(path).equals(GOOD)).toBe(true);
  });

  it('settles for paint before the first capture', async () => {
    const { page, events } = mockPage([GOOD]);
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    const t0 = Date.now();
    await shot(page, path, { settleMs: 60, retryGapMs: 1 });
    expect(Date.now() - t0).toBeGreaterThanOrEqual(60);
    expect(events[0]).toBe('eval');
    expect(events.filter((e) => e === 'shot')).toHaveLength(1);
  });

  it('recaptures while the page area is flat, and keeps the painted frame', async () => {
    const { page, events } = mockPage([UNPAINTED, UNPAINTED, GOOD]);
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(page, path, FAST)).resolves.toBe('ok');
    expect(events.filter((e) => e === 'shot')).toHaveLength(3);
    expect(readFileSync(path).equals(GOOD)).toBe(true);
  });

  it('gives up after its tries and removes the file', async () => {
    const { page, events } = mockPage([UNPAINTED]);
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(page, path, { settleMs: 1, tries: 3, retryGapMs: 1 })).resolves.toBe('unpainted');
    expect(events.filter((e) => e === 'shot')).toHaveLength(3);
    expect(existsSync(path)).toBe(false);
  });

  it('captures anyway when the settle frames throw', async () => {
    const { page } = mockPage([GOOD]);
    const broken = { ...page, evaluate: async () => Promise.reject(new Error('dead webview')) };
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(broken, path, FAST)).resolves.toBe('ok');
    expect(readFileSync(path).equals(GOOD)).toBe(true);
  });

  // #425: no window on screen (a locked or sleeping display) makes the capture
  // itself throw. The picture is for a person; the DOM assertions are the
  // evidence — so it is reported, never a failed test.
  it('reports a capture that throws as unavailable, without failing', async () => {
    const { page } = mockPage([GOOD]);
    const gone = { ...page, screenshot: async () => Promise.reject(new Error('no window')) };
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(gone, path, FAST)).resolves.toBe('unavailable');
    expect(existsSync(path)).toBe(false);
  });

  it('retries a wholly flat capture the same way', async () => {
    const { page, events } = mockPage([BLANK, GOOD]);
    const path = join(mkdtempSync(join(tmpdir(), 'shot-')), 'a.png');
    await expect(shot(page, path, FAST)).resolves.toBe('ok');
    expect(events.filter((e) => e === 'shot')).toHaveLength(2);
    expect(readFileSync(path).equals(GOOD)).toBe(true);
  });
});
