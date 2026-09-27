#!/usr/bin/env node
/**
 * Writes the *sample* plate and manifest under `public/plates/`.
 *
 * ## Why a sample and not the real thing
 *
 * R10 (MJXOFF-165) has landed and `cargo run -p mjx-render-oracle -- gallery <dir>` produces real
 * plates — but that command builds `mjx-paint`, which links `wgpu`, which links the platform's
 * graphics stack. Making the `ui/` check set depend on it would put a Vulkan toolchain between a
 * TypeScript contributor and a green run, and would make the catalogue unbuildable on any machine
 * without one. So the loader is exercised against a committed fixture that matches
 * `crates/mjx-render-oracle/src/plate.rs`'s manifest field for field, and the gallery story points
 * at a real directory when one is supplied.
 *
 * The fixture is deliberately **not** a picture of anything: it is a flat backdrop with a white
 * page on it, which is what the document surface actually looks like today, and its `covered` and
 * `drawCalls` figures are the true counts for that image. A fixture that pretended to be a
 * rendered slide would be the first thing to mislead a reader.
 *
 * Run: `node scripts/make-sample-plate.mjs`
 */

import { deflateSync } from 'node:zlib';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const width = 320;
const height = 200;

/** `#rrggbb` to a straight-alpha RGBA tuple. */
const rgba = (hex, alpha = 255) => [
  Number.parseInt(hex.slice(1, 3), 16),
  Number.parseInt(hex.slice(3, 5), 16),
  Number.parseInt(hex.slice(5, 7), 16),
  alpha,
];

const backdrop = rgba('#fdfcf9');
const page = rgba('#ffffff');
const border = rgba('#e7e0d2');

const pageBox = { x: 60, y: 30, w: 200, h: 140 };

let covered = 0;
const raw = Buffer.alloc(height * (1 + width * 4));
for (let y = 0; y < height; y += 1) {
  const row = y * (1 + width * 4);
  raw[row] = 0; // filter: none
  for (let x = 0; x < width; x += 1) {
    const insidePage =
      x >= pageBox.x && x < pageBox.x + pageBox.w && y >= pageBox.y && y < pageBox.y + pageBox.h;
    const onBorder =
      insidePage &&
      (x === pageBox.x ||
        x === pageBox.x + pageBox.w - 1 ||
        y === pageBox.y ||
        y === pageBox.y + pageBox.h - 1);
    const pixel = onBorder ? border : insidePage ? page : backdrop;
    const at = row + 1 + x * 4;
    raw[at] = pixel[0];
    raw[at + 1] = pixel[1];
    raw[at + 2] = pixel[2];
    raw[at + 3] = pixel[3];
    if (pixel[3] !== 0) covered += 1;
  }
}

/** One PNG chunk: length, type, payload, CRC-32. */
function chunk(type, payload) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(payload.length);
  const body = Buffer.concat([Buffer.from(type, 'latin1'), payload]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body) >>> 0);
  return Buffer.concat([length, body, crc]);
}

const crcTable = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n += 1) {
    let c = n;
    for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let c = -1;
  for (const byte of buffer) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  return c ^ -1;
}

const header = Buffer.alloc(13);
header.writeUInt32BE(width, 0);
header.writeUInt32BE(height, 4);
header[8] = 8; // bit depth
header[9] = 6; // colour type: RGBA
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', header),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

const directory = resolve(import.meta.dirname, '../public/plates');
mkdirSync(directory, { recursive: true });
const file = 'document-surface.png';
writeFileSync(resolve(directory, file), png);

const manifest = {
  version: 1,
  generator: 'ui/scripts/make-sample-plate.mjs (a fixture, not mjx-render-oracle)',
  premultiplied: false,
  provider: 'none',
  authoritative: false,
  parityClaimed: false,
  plates: [
    {
      name: 'document-surface',
      description:
        'The document surface as the tokens describe it: a true-white page on the light backdrop, ' +
        'with the warm page border. A fixture for the loader, not a render of any document.',
      file,
      width,
      height,
      sha256: createHash('sha256').update(png).digest('hex'),
      content: 'solid fills',
      provider: 'none',
      excluded:
        'This plate was not produced by mjx-render-oracle and no reference provider has seen it. ' +
        'It exercises the loader; it is not evidence about the renderer.',
      placeholders: 0,
      drawCalls: 3,
      covered,
      approver: 'fixture',
      reviewed: false,
      parity: false,
      referenceFile: null,
      diffFile: null,
      difference: null,
    },
  ],
  documents: [
    {
      fixture: '(none)',
      format: 'pptx',
      verdict: 'excluded',
      reason:
        'No format renders yet, and this fixture manifest was not produced by the oracle in any ' +
        'case. See crates/mjx-render-oracle/src/plate.rs::NO_FORMAT_RENDERS_YET.',
    },
  ],
};

writeFileSync(resolve(directory, 'plates.json'), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`wrote ${directory}/plates.json and ${file} (${String(covered)} covered pixels)`);
