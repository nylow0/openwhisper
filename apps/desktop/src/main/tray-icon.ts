import { nativeImage, type NativeImage } from 'electron';
import { deflateSync } from 'node:zlib';

// ── Minimal PNG encoder ────────────────────────────────────────────────────
// Lets us synthesize the app icon at runtime with no asset files on disk.

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) {
      c = (c & 1) !== 0 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(data: Buffer): number {
  let c = 0xffffffff;
  for (let i = 0; i < data.length; i += 1) {
    c = CRC_TABLE[(c ^ data[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function pngChunk(type: string, data: Buffer): Buffer {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([length, body, crc]);
}

function encodePng(size: number, rgba: Buffer): Buffer {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr.writeUInt8(8, 8); // 8-bit depth
  ihdr.writeUInt8(6, 9); // RGBA colour type

  const stride = size * 4;
  const raw = Buffer.alloc(size * (stride + 1));
  for (let y = 0; y < size; y += 1) {
    rgba.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride);
  }

  return Buffer.concat([
    signature,
    pngChunk('IHDR', ihdr),
    pngChunk('IDAT', deflateSync(raw)),
    pngChunk('IEND', Buffer.alloc(0)),
  ]);
}

/**
 * Builds the OpenWhisper icon at the requested size: an indigo rounded square
 * with a white "record" dot. Rendered at 4x and box-downscaled with
 * premultiplied alpha for smooth edges.
 */
export function createAppIcon(size: number): NativeImage {
  const ss = 4;
  const big = size * ss;
  const shape = new Uint8Array(big * big * 4);

  const margin = big * 0.05;
  const corner = big * 0.24;
  const center = big / 2;
  const half = (big - 2 * margin) / 2;
  const dotRadius = big * 0.2;

  for (let y = 0; y < big; y += 1) {
    for (let x = 0; x < big; x += 1) {
      const px = x + 0.5;
      const py = y + 0.5;
      const qx = Math.max(Math.abs(px - center) - (half - corner), 0);
      const qy = Math.max(Math.abs(py - center) - (half - corner), 0);
      const insideSquare = Math.hypot(qx, qy) <= corner;
      const insideDot = Math.hypot(px - center, py - center) <= dotRadius;
      const i = (y * big + x) * 4;
      if (insideDot) {
        shape[i] = 255;
        shape[i + 1] = 255;
        shape[i + 2] = 255;
        shape[i + 3] = 255;
      } else if (insideSquare) {
        shape[i] = 99;
        shape[i + 1] = 102;
        shape[i + 2] = 241;
        shape[i + 3] = 255;
      }
    }
  }

  const out = Buffer.alloc(size * size * 4);
  const samples = ss * ss;
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let sa = 0;
      let sr = 0;
      let sg = 0;
      let sb = 0;
      for (let dy = 0; dy < ss; dy += 1) {
        for (let dx = 0; dx < ss; dx += 1) {
          const i = ((y * ss + dy) * big + (x * ss + dx)) * 4;
          const a = shape[i + 3];
          sa += a;
          sr += shape[i] * a;
          sg += shape[i + 1] * a;
          sb += shape[i + 2] * a;
        }
      }
      const o = (y * size + x) * 4;
      if (sa > 0) {
        out[o] = Math.round(sr / sa);
        out[o + 1] = Math.round(sg / sa);
        out[o + 2] = Math.round(sb / sa);
      }
      out[o + 3] = Math.round(sa / samples);
    }
  }

  return nativeImage.createFromBuffer(encodePng(size, out));
}
