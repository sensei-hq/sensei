import { describe, it, expect } from 'vitest';
import { deflateSync } from 'node:zlib';
import { existsSync, readFileSync } from 'node:fs';
import { isUniformPng, PAGE_BAND } from './png-blank';

// #284: a window capture without Screen Recording permission comes back as one
// flat colour. The harness must say so instead of saving it as evidence.

/** A minimal PNG: `w`×`h`, `depth`-bit RGBA, rows from `pixel(x, y)`. */
function png(w: number, h: number, pixel: (x: number, y: number) => number[], depth = 8): Buffer {
  const bpc = depth / 8;
  const raw = Buffer.alloc(h * (1 + w * 4 * bpc));
  let o = 0;
  for (let y = 0; y < h; y++) {
    raw[o++] = 0; // filter: none
    for (let x = 0; x < w; x++) for (const c of pixel(x, y)) { raw.writeUIntBE(c, o, bpc); o += bpc; }
  }
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(data.length);
    return Buffer.concat([len, Buffer.from(type), data, Buffer.alloc(4)]); // CRC unchecked
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = depth; ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0)),
  ]);
}

describe('isUniformPng', () => {
  it('is true for a capture of one flat colour', () => {
    expect(isUniformPng(png(8, 4, () => [255, 255, 255, 255]))).toBe(true);
  });
  it('is false when any pixel differs', () => {
    expect(isUniformPng(png(8, 4, (x, y) => (x === 7 && y === 3 ? [0, 0, 0, 255] : [255, 255, 255, 255])))).toBe(false);
  });
  it('reads 16-bit captures, as macOS writes them', () => {
    expect(isUniformPng(png(4, 4, () => [65535, 65535, 65535, 65535], 16))).toBe(true);
    expect(isUniformPng(png(4, 4, (x) => [x * 1000, 0, 0, 65535], 16))).toBe(false);
  });
  it('flags the real blank capture from the #284 probe', () => {
    const p = '/tmp/sensei-shots/probe/a-after-nav-plugin.png';
    if (!existsSync(p)) return;
    expect(isUniformPng(readFileSync(p))).toBe(true);
  });
});

// The second mode: the window frame and docked devtools paint, the page area
// does not. Checked over a band of rows — the page area — rather than the whole.
describe('isUniformPng over a band of rows', () => {
  const framed = png(8, 10, (x, y) => (y < 2 || y >= 8 ? [x * 30, 0, 0, 255] : [40, 40, 40, 255]));
  it('is true when only the band is flat', () => {
    expect(isUniformPng(framed)).toBe(false);
    expect(isUniformPng(framed, { top: 0.2, bottom: 0.8 })).toBe(true);
  });
  it('PAGE_BAND looks past a painted title bar to the page under it', () => {
    // 50 rows: a varied title bar in the top 6%, then the page area.
    const titled = (page: (x: number, y: number) => number[]) =>
      png(8, 50, (x, y) => (y < 3 ? [x * 30, 0, 0, 255] : page(x, y)));
    expect(isUniformPng(titled(() => [40, 40, 40, 255]), PAGE_BAND)).toBe(true);
    expect(isUniformPng(titled((x, y) => (y === 20 && x === 4 ? [200, 0, 0, 255] : [40, 40, 40, 255])), PAGE_BAND)).toBe(false);
  });
});
