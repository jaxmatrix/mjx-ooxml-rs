import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

import {
  PlateManifestError,
  parsePlateManifest,
  plateUrl,
  provenanceNotice,
  supportedManifestVersion,
} from '../src/plates/manifest.ts';

/**
 * The manifest shape is transcribed from `crates/mjx-render-oracle/src/plate.rs`. Written out here
 * in full rather than imported from the fixture, so that the fixture and the contract are two
 * statements and a drift between them is visible.
 */
const manifestFromR10 = {
  version: 1,
  generator: 'mjx-render-oracle 0.0.0',
  premultiplied: false,
  provider: 'LibreOffice',
  authoritative: false,
  parityClaimed: false,
  plates: [
    {
      name: 'gradients',
      description: 'A linear gradient across a rounded rectangle.',
      file: 'gradients.png',
      width: 640,
      height: 480,
      sha256: 'a'.repeat(64),
      content: 'gradient',
      provider: 'LibreOffice',
      excluded: 'LibreOffice does not reproduce this gradient interpolation.',
      placeholders: 0,
      drawCalls: 12,
      covered: 300_000,
      approver: 'generator',
      reviewed: false,
      parity: false,
      referenceFile: null,
      diffFile: null,
      difference: null,
    },
  ],
  documents: [
    {
      fixture: 'minimal.pptx',
      format: 'pptx',
      verdict: 'excluded',
      reason: 'no format renders yet.',
    },
  ],
};

describe('the plate manifest', () => {
  it('parses the shape crates/mjx-render-oracle/src/plate.rs emits', () => {
    const manifest = parsePlateManifest(manifestFromR10);
    expect(manifest.version).toBe(supportedManifestVersion);
    expect(manifest.premultiplied).toBe(false);
    expect(manifest.authoritative).toBe(false);
    expect(manifest.parityClaimed).toBe(false);
    expect(manifest.plates).toHaveLength(1);
    expect(manifest.plates[0]?.placeholders).toBe(0);
    expect(manifest.plates[0]?.reviewed).toBe(false);
    expect(manifest.documents[0]?.verdict).toBe('excluded');
  });

  it('refuses a version it does not understand rather than guessing', () => {
    // plate.rs on MANIFEST_VERSION: "A loader that does not recognise it should refuse rather than
    // guess." The fields a version bump would move are the ones that say how much a plate may
    // claim, so a lenient loader would show a picture with the wrong provenance beside it.
    expect(() => parsePlateManifest({ ...manifestFromR10, version: 2 })).toThrow(PlateManifestError);
    expect(() => parsePlateManifest({ ...manifestFromR10, version: 2 })).toThrow(/version 1/);
  });

  it.each([
    ['a missing plate field', { plates: [{ ...manifestFromR10.plates[0], sha256: undefined }] }],
    ['a plate count that is not a number', { plates: [{ ...manifestFromR10.plates[0], covered: 'lots' }] }],
    ['a boolean written as a string', { parityClaimed: 'false' }],
    ['plates that are not an array', { plates: {} }],
  ])('refuses %s', (_name, patch) => {
    expect(() => parsePlateManifest({ ...manifestFromR10, ...patch })).toThrow(PlateManifestError);
  });

  it('names the provider in the notice whenever it is not authoritative', () => {
    const manifest = parsePlateManifest(manifestFromR10);
    const notice = provenanceNotice(manifest);
    expect(notice).toContain('LibreOffice');
    expect(notice).toContain('not evidence of parity');
    // The user's standing constraint: parity is judged against Microsoft Office on Windows, and
    // LibreOffice is a change detector. The notice disappears only when a provider is authoritative.
    expect(provenanceNotice({ ...manifest, authoritative: true })).toBeUndefined();
  });

  it('resolves a plate URL against the directory it was loaded from, with or without a slash', () => {
    const plate = parsePlateManifest(manifestFromR10).plates[0];
    expect(plate).toBeDefined();
    if (plate === undefined) return;
    expect(plateUrl('/plates', plate)).toBe('/plates/gradients.png');
    expect(plateUrl('/plates/', plate)).toBe('/plates/gradients.png');
  });

  it('parses the committed fixture the catalogue actually serves', () => {
    const path = resolve(import.meta.dirname, '../public/plates/plates.json');
    const manifest = parsePlateManifest(JSON.parse(readFileSync(path, 'utf8')));
    expect(manifest.plates).toHaveLength(1);
    // The fixture must never claim to be evidence. It was not produced by the oracle and no
    // reference provider has seen it.
    expect(manifest.parityClaimed).toBe(false);
    expect(manifest.authoritative).toBe(false);
    expect(manifest.plates[0]?.reviewed).toBe(false);
    expect(manifest.plates[0]?.excluded).not.toBeNull();
  });
});
