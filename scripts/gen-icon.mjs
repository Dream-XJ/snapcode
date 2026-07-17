// 生成 SnapCode 应用图标：圆角方形蓝底 + 白色闪电，输出 PNG 多尺寸与 icon.ico。
// 仅依赖 Node 内置模块：node scripts/gen-icon.mjs
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

const OUT = path.resolve("src-tauri/icons");
mkdirSync(OUT, { recursive: true });

const BG = [37, 99, 235]; // blue-600
const FG = [255, 255, 255];

// 闪电多边形（1024 画布坐标）
const BOLT = [
  [560, 90],
  [300, 580],
  [470, 580],
  [420, 930],
  [720, 440],
  [540, 440],
];

function pointInPolygon(x, y, poly) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i];
    const [xj, yj] = poly[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

function render(size) {
  const scale = size / 1024;
  const r = 190 * scale;
  const buf = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      // 圆角矩形
      let inRound;
      if ((x >= r && x < size - r) || (y >= r && y < size - r)) {
        inRound = true;
      } else {
        const cx = x < r ? r : size - r - 1;
        const cy = y < r ? r : size - r - 1;
        inRound = (x - cx) ** 2 + (y - cy) ** 2 <= r * r;
      }
      let px = [0, 0, 0, 0];
      if (inRound) {
        px = [...BG, 255];
        const bx = (x + 0.5) / scale;
        const by = (y + 0.5) / scale;
        if (pointInPolygon(bx, by, BOLT)) px = [...FG, 255];
      }
      const o = (y * size + x) * 4;
      buf[o] = px[0];
      buf[o + 1] = px[1];
      buf[o + 2] = px[2];
      buf[o + 3] = px[3];
    }
  }
  return buf;
}

// ---- PNG 编码 ----
const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
function encodePng(size, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---- ICO（内嵌 PNG）----
function encodeIco(png256) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2); // icon type
  header.writeUInt16LE(1, 4); // count
  const entry = Buffer.alloc(16);
  entry[0] = 0; // 256
  entry[1] = 0;
  entry[2] = 0;
  entry[3] = 0;
  entry.writeUInt16LE(1, 4); // planes
  entry.writeUInt16LE(32, 6); // bpp
  entry.writeUInt32LE(png256.length, 8);
  entry.writeUInt32LE(22, 12);
  return Buffer.concat([header, entry, png256]);
}

const png256 = encodePng(256, render(256));
writeFileSync(path.join(OUT, "icon.ico"), encodeIco(png256));
writeFileSync(path.join(OUT, "32x32.png"), encodePng(32, render(32)));
writeFileSync(path.join(OUT, "128x128.png"), encodePng(128, render(128)));
writeFileSync(path.join(OUT, "128x128@2x.png"), png256);
writeFileSync(path.join(OUT, "icon.png"), encodePng(1024, render(1024)));
console.log("icons written to", OUT);
