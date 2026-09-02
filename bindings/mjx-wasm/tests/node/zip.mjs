// The eight lines of ZIP reading the walkthrough comparison needs, and no dependency for it.
//
// Node ships `zlib` but no archive reader, and the comparison has to be on *decompressed part
// payloads* — the round-trip contract this repository states is per-part payload identity plus
// structural container identity, not identical archive bytes. Comparing the two archives directly
// would fail on compression-level differences that mean nothing.

import { inflateRawSync } from "node:zlib";

const END_OF_CENTRAL_DIRECTORY = 0x06054b50;
const CENTRAL_FILE_HEADER = 0x02014b50;

/**
 * Every part of an OPC package, by name, decompressed.
 *
 * @param {Buffer} archive the package's bytes
 * @returns {Record<string, Buffer>} each part's decompressed payload
 */
export function partPayloads(archive) {
  const buffer = Buffer.from(archive);
  const end = findEndOfCentralDirectory(buffer);
  const entryCount = buffer.readUInt16LE(end + 10);
  let offset = buffer.readUInt32LE(end + 16);

  const parts = {};
  for (let entry = 0; entry < entryCount; entry += 1) {
    if (buffer.readUInt32LE(offset) !== CENTRAL_FILE_HEADER) {
      throw new Error(`central directory entry ${entry} has the wrong signature`);
    }
    const method = buffer.readUInt16LE(offset + 10);
    const compressedSize = buffer.readUInt32LE(offset + 20);
    const nameLength = buffer.readUInt16LE(offset + 28);
    const extraLength = buffer.readUInt16LE(offset + 30);
    const commentLength = buffer.readUInt16LE(offset + 32);
    const localOffset = buffer.readUInt32LE(offset + 42);
    const name = buffer.toString("utf8", offset + 46, offset + 46 + nameLength);

    const localNameLength = buffer.readUInt16LE(localOffset + 26);
    const localExtraLength = buffer.readUInt16LE(localOffset + 28);
    const dataStart = localOffset + 30 + localNameLength + localExtraLength;
    const compressed = buffer.subarray(dataStart, dataStart + compressedSize);
    parts[name] = method === 0 ? Buffer.from(compressed) : inflateRawSync(compressed);

    offset += 46 + nameLength + extraLength + commentLength;
  }
  return parts;
}

/** Where the end-of-central-directory record starts, scanning back from the archive's end. */
function findEndOfCentralDirectory(buffer) {
  for (let offset = buffer.length - 22; offset >= 0; offset -= 1) {
    if (buffer.readUInt32LE(offset) === END_OF_CENTRAL_DIRECTORY) {
      return offset;
    }
  }
  throw new Error("not a ZIP archive: no end-of-central-directory record");
}
