/**
 * Is a PNG one flat colour? (#284)
 *
 * The inspect harness captures the app window natively. Without macOS Screen
 * Recording permission for the app — which a rebuilt, re-signed bundle loses —
 * the capture comes back as a single colour, and was saved as if it were
 * evidence. This reads the pixels to tell.
 *
 * A minimal decoder: non-interlaced PNG, any colour type, 8- or 16-bit, with the
 * five scanline filters undone (real captures use them, so comparing filtered
 * bytes would misread a flat image).
 */
import { inflateSync } from 'node:zlib';

const CHANNELS: Record<number, number> = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };

/** The rows that hold the page in an inspect capture: below the title bar,
 *  above where the debug build docks its devtools. */
export const PAGE_BAND = { top: 0.06, bottom: 0.6 } as const;

/** Whether the PNG — or, given `band`, the rows between those fractions of its
 *  height — is one colour. */
export function isUniformPng(png: Buffer, band?: { top: number; bottom: number }): boolean {
  let width = 0;
  let height = 0;
  let depth = 8;
  let channels = 4;
  const idat: Buffer[] = [];
  for (let o = 8; o + 8 <= png.length; ) {
    const len = png.readUInt32BE(o);
    const type = png.toString('ascii', o + 4, o + 8);
    const data = png.subarray(o + 8, o + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      depth = data[8];
      channels = CHANNELS[data[9]] ?? 4;
    } else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    o += 12 + len;
  }
  const bpp = Math.max(1, (channels * depth) / 8);
  const stride = Math.ceil((width * channels * depth) / 8);
  const raw = inflateSync(Buffer.concat(idat));
  // Rows above the band still decode (each row's filter reads the one before);
  // only rows inside it are compared.
  const from = band ? Math.floor(band.top * height) : 0;
  const to = band ? Math.min(height, Math.ceil(band.bottom * height)) : height;
  let prev = Buffer.alloc(stride);
  let first: Buffer | null = null;
  for (let y = 0; y < to; y++) {
    const start = y * (stride + 1);
    const filter = raw[start];
    const line = Buffer.from(raw.subarray(start + 1, start + 1 + stride));
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? line[i - bpp] : 0;
      const b = prev[i];
      const c = i >= bpp ? prev[i - bpp] : 0;
      const p = a + b - c;
      const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
      const predictor =
        filter === 1 ? a :
        filter === 2 ? b :
        filter === 3 ? (a + b) >> 1 :
        filter === 4 ? (pa <= pb && pa <= pc ? a : pb <= pc ? b : c) : 0;
      line[i] = (line[i] + predictor) & 0xff;
    }
    if (y >= from) {
      // Compare every pixel with the first one in the band.
      first ??= Buffer.from(line.subarray(0, bpp));
      for (let i = 0; i < stride; i += bpp) {
        if (line.compare(first, 0, bpp, i, i + bpp) !== 0) return false;
      }
    }
    prev = line;
  }
  return true;
}
