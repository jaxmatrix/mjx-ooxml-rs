/**
 * The plate-manifest loader.
 *
 * R10 (MJXOFF-165) generates golden plates and a manifest beside them, and says who reads it:
 *
 * > R11's canvas harness and U01's Storybook gallery both load these plates, and neither of them
 * > is Rust. So the contract is a **file layout and a JSON manifest**, stated here, with a reader
 * > in `tests/the_plate_manifest_is_loadable.rs` that asks it every question a loader will ask.
 *
 * This is that loader on the TypeScript side. The field list below is transcribed from
 * `crates/mjx-render-oracle/src/plate.rs` — `PlateSet::manifest`, `plate_object` and
 * `document_object` — and the version constant from `MANIFEST_VERSION`.
 *
 * ## Why it refuses rather than guesses
 *
 * `plate.rs` on `MANIFEST_VERSION`: *"A loader that does not recognise it should refuse rather
 * than guess."* A gallery that silently rendered a version-2 manifest with version-1 assumptions
 * would show the auditor a picture with the wrong provenance beside it, which is worse than
 * showing nothing — the fields that matter here (`approver`, `reviewed`, `parity`, `excluded`) are
 * precisely the ones that say *how much this image is allowed to claim*.
 *
 * ## Why every field is validated and none is optional
 *
 * The three fields a reader is most likely to skip — `provider`, `excluded`, `parity` — are the
 * ones that stop a plate being read as evidence it is not. `parityClaimed` is `false` for every
 * manifest this project can currently generate, and a loader that let it default to `undefined`
 * would let a gallery quietly stop saying so.
 */

/** The only manifest version this loader understands. `plate.rs`'s `MANIFEST_VERSION`. */
export const supportedManifestVersion = 1;

/** `plate.rs`'s `MANIFEST_FILE`. */
export const manifestFileName = 'plates.json';

/** One rendered plate, and everything a loader has to know about it. */
export interface Plate {
  /** The specimen's stable name. */
  readonly name: string;
  /** What the page is for. */
  readonly description: string;
  /** The PNG's file name inside the manifest's directory. */
  readonly file: string;
  readonly width: number;
  readonly height: number;
  /** The PNG's SHA-256, so a loader can cache and a person can check. */
  readonly sha256: string;
  /** What kind of drawing it is. */
  readonly content: string;
  /** Where its reference came from. */
  readonly provider: string;
  /** Why the provider may not speak about this content, or `null` when it may. */
  readonly excluded: string | null;
  /** How many draws used stand-in geometry. Zero, asserted on the Rust side. */
  readonly placeholders: number;
  /** How many draw calls the page issued — beside `placeholders`, because zero placeholders is
   *  also true of a page that drew nothing. */
  readonly drawCalls: number;
  /** How many pixels are not fully transparent. */
  readonly covered: number;
  /** Who approved the baseline this plate was rendered against. */
  readonly approver: string;
  /** Whether a **person** approved it. */
  readonly reviewed: boolean;
  /** Whether this may be described as matching PowerPoint. `false` today, always. */
  readonly parity: boolean;
  /** The approved baseline's file name — present only when this plate differs from it. */
  readonly referenceFile: string | null;
  /** The amplified difference — present only when this plate differs from its baseline. */
  readonly diffFile: string | null;
  /** What the pixel comparison concluded, or `null` when it agreed. */
  readonly difference: string | null;
}

/** One committed document fixture, and what the oracle can say about it. */
export interface DocumentRow {
  readonly fixture: string;
  /** `pptx`, `docx` or `xlsx`. */
  readonly format: string;
  readonly verdict: string;
  readonly reason: string;
}

/** A whole manifest. */
export interface PlateManifest {
  readonly version: number;
  readonly generator: string;
  /** Whether the PNGs carry premultiplied alpha. `false`; stated because a decoder has to know. */
  readonly premultiplied: boolean;
  readonly provider: string;
  /** Whether the provider may be treated as authoritative. `false` for LibreOffice. */
  readonly authoritative: boolean;
  /** Whether any plate claims parity with Microsoft Office. `false` today. */
  readonly parityClaimed: boolean;
  readonly plates: readonly Plate[];
  readonly documents: readonly DocumentRow[];
}

/** Thrown when a manifest cannot be trusted. Named so a gallery can show the reason. */
export class PlateManifestError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'PlateManifestError';
  }
}

function object(value: unknown, where: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new PlateManifestError(`${where} is not an object.`);
  }
  return value as Record<string, unknown>;
}

function text(source: Record<string, unknown>, key: string, where: string): string {
  const value = source[key];
  if (typeof value !== 'string') {
    throw new PlateManifestError(`${where}.${key} is not a string.`);
  }
  return value;
}

function optionalText(source: Record<string, unknown>, key: string, where: string): string | null {
  const value = source[key];
  if (value === null) return null;
  if (typeof value !== 'string') {
    throw new PlateManifestError(`${where}.${key} is neither a string nor null.`);
  }
  return value;
}

function count(source: Record<string, unknown>, key: string, where: string): number {
  const value = source[key];
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) {
    throw new PlateManifestError(`${where}.${key} is not a count.`);
  }
  return value;
}

function flag(source: Record<string, unknown>, key: string, where: string): boolean {
  const value = source[key];
  if (typeof value !== 'boolean') {
    throw new PlateManifestError(`${where}.${key} is not a boolean.`);
  }
  return value;
}

function array(source: Record<string, unknown>, key: string, where: string): unknown[] {
  const value = source[key];
  if (!Array.isArray(value)) throw new PlateManifestError(`${where}.${key} is not an array.`);
  return value;
}

function parsePlate(value: unknown, index: number): Plate {
  const where = `plates[${String(index)}]`;
  const source = object(value, where);
  return {
    name: text(source, 'name', where),
    description: text(source, 'description', where),
    file: text(source, 'file', where),
    width: count(source, 'width', where),
    height: count(source, 'height', where),
    sha256: text(source, 'sha256', where),
    content: text(source, 'content', where),
    provider: text(source, 'provider', where),
    excluded: optionalText(source, 'excluded', where),
    placeholders: count(source, 'placeholders', where),
    drawCalls: count(source, 'drawCalls', where),
    covered: count(source, 'covered', where),
    approver: text(source, 'approver', where),
    reviewed: flag(source, 'reviewed', where),
    parity: flag(source, 'parity', where),
    referenceFile: optionalText(source, 'referenceFile', where),
    diffFile: optionalText(source, 'diffFile', where),
    difference: optionalText(source, 'difference', where),
  };
}

function parseDocumentRow(value: unknown, index: number): DocumentRow {
  const where = `documents[${String(index)}]`;
  const source = object(value, where);
  return {
    fixture: text(source, 'fixture', where),
    format: text(source, 'format', where),
    verdict: text(source, 'verdict', where),
    reason: text(source, 'reason', where),
  };
}

/**
 * Parse a manifest, refusing anything this loader cannot vouch for.
 *
 * @throws {PlateManifestError} on an unrecognised version or a malformed field.
 */
export function parsePlateManifest(value: unknown): PlateManifest {
  const source = object(value, 'the manifest');
  const version = source['version'];
  if (version !== supportedManifestVersion) {
    throw new PlateManifestError(
      `this loader understands plate-manifest version ${String(supportedManifestVersion)}, ` +
        `and this manifest declares ${JSON.stringify(version)}. Refusing rather than guessing: ` +
        'the fields that say how much a plate may claim are exactly the ones a version bump moves.',
    );
  }
  return {
    version,
    generator: text(source, 'generator', 'the manifest'),
    premultiplied: flag(source, 'premultiplied', 'the manifest'),
    provider: text(source, 'provider', 'the manifest'),
    authoritative: flag(source, 'authoritative', 'the manifest'),
    parityClaimed: flag(source, 'parityClaimed', 'the manifest'),
    plates: array(source, 'plates', 'the manifest').map(parsePlate),
    documents: array(source, 'documents', 'the manifest').map(parseDocumentRow),
  };
}

/**
 * Fetch and parse a manifest from a directory URL.
 *
 * The plates themselves are resolved relative to that same directory, which is the layout
 * `plate.rs` documents: the manifest and every PNG sit side by side.
 */
export async function loadPlateManifest(
  directoryUrl: string,
  fetcher: typeof fetch = fetch,
): Promise<PlateManifest> {
  const base = directoryUrl.endsWith('/') ? directoryUrl : `${directoryUrl}/`;
  const response = await fetcher(`${base}${manifestFileName}`);
  if (!response.ok) {
    throw new PlateManifestError(
      `${base}${manifestFileName} answered ${String(response.status)}. Run ` +
        '`cargo run -p mjx-render-oracle -- gallery <directory>` to produce one.',
    );
  }
  return parsePlateManifest(await response.json());
}

/** A plate's PNG URL, relative to the directory its manifest was loaded from. */
export function plateUrl(directoryUrl: string, plate: Plate): string {
  const base = directoryUrl.endsWith('/') ? directoryUrl : `${directoryUrl}/`;
  return `${base}${plate.file}`;
}

/**
 * The one sentence a gallery must show beside any plate whose provider is not authoritative.
 *
 * Written here rather than in a story so that fifteen further children cannot each invent their
 * own wording for it — and so that the day a provider *is* authoritative, one edit removes it
 * everywhere.
 */
export function provenanceNotice(manifest: PlateManifest): string | undefined {
  if (manifest.authoritative) return undefined;
  return (
    `Reference provider: ${manifest.provider}, which is not authoritative. These plates are a ` +
    'change detector, not evidence of parity with Microsoft Office.'
  );
}
