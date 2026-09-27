/**
 * `<mjx-plate-gallery>` — the plates from R10's generator, beside the live catalogue.
 *
 * A framework-free custom element with Shadow DOM, like every shipped component here. It does one
 * thing: load a manifest from a directory and show what it contains, **including the parts that
 * limit what the pictures may be taken to mean**.
 *
 * ## Why the provenance is rendered and not optional
 *
 * `plate.rs` is explicit that `provider`, `excluded` and `parity` exist so a reader cannot mistake
 * a plate for evidence:
 *
 * > A gallery that showed a gradient plate beside a LibreOffice reference without saying the
 * > comparison is not evidence would be inviting exactly the mistake the user's constraint is
 * > about.
 *
 * The user's standing constraint is that rendering parity is judged against Microsoft Office on
 * Windows and that LibreOffice is a change detector only. So the notice is part of the element
 * rather than part of a story: a later child that drops the notice would have to delete code, not
 * merely forget a prop.
 */

import {
  loadPlateManifest,
  plateUrl,
  provenanceNotice,
  type Plate,
  type PlateManifest,
} from './manifest.ts';

const styles = `
  :host { display: block; font-family: var(--font-sans); color: var(--theme-text-primary); }
  .notice {
    margin-block-end: calc(var(--spacing) * 4);
    padding: calc(var(--spacing) * 3);
    border: 1px solid var(--theme-secondary-accent);
    border-radius: var(--radius-control);
    background: var(--theme-secondary-surface);
    font-size: var(--text-sm);
    line-height: var(--leading-snug);
  }
  .error { border-color: var(--theme-border); background: var(--theme-surface); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: calc(var(--spacing) * 4); }
  figure {
    margin: 0;
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-card);
    background: var(--theme-surface);
    padding: calc(var(--spacing) * 3);
  }
  img { display: block; max-inline-size: 100%; height: auto; border-radius: var(--radius-chip); }
  figcaption { margin-block-start: calc(var(--spacing) * 2); font-size: var(--text-sm); line-height: var(--leading-snug); }
  dl { display: grid; grid-template-columns: max-content 1fr; gap: var(--spacing) calc(var(--spacing) * 3); margin: calc(var(--spacing) * 2) 0 0; font-size: var(--text-xs); }
  dt { color: var(--theme-text-secondary); }
  dd { margin: 0; }
`;

export class MjxPlateGallery extends HTMLElement {
  static readonly observedAttributes = ['src'];

  #root: ShadowRoot | undefined;
  #generation = 0;

  connectedCallback(): void {
    this.#root ??= this.attachShadow({ mode: 'open' });
    void this.reload();
  }

  attributeChangedCallback(): void {
    if (this.isConnected) void this.reload();
  }

  /** The directory the manifest and its PNGs live in. */
  get src(): string {
    return this.getAttribute('src') ?? '.';
  }

  /** Load and render. Resolves once the DOM reflects the manifest, so a test can await it. */
  async reload(): Promise<void> {
    const generation = ++this.#generation;
    const source = this.src;
    try {
      const manifest = await loadPlateManifest(source);
      if (generation !== this.#generation) return;
      this.#render(manifest, source);
    } catch (error) {
      if (generation !== this.#generation) return;
      this.#renderError(error instanceof Error ? error.message : String(error));
    }
  }

  #shell(): { root: ShadowRoot; body: HTMLElement } {
    const root = this.#root ?? this.attachShadow({ mode: 'open' });
    this.#root = root;
    root.replaceChildren();
    const sheet = this.ownerDocument.createElement('style');
    sheet.textContent = styles;
    const body = this.ownerDocument.createElement('div');
    root.append(sheet, body);
    return { root, body };
  }

  #renderError(message: string): void {
    const { body } = this.#shell();
    const notice = this.ownerDocument.createElement('p');
    notice.className = 'notice error';
    notice.dataset['role'] = 'error';
    notice.textContent = message;
    body.append(notice);
  }

  #render(manifest: PlateManifest, source: string): void {
    const { body } = this.#shell();
    body.dataset['plateCount'] = String(manifest.plates.length);

    const notice = provenanceNotice(manifest);
    if (notice !== undefined) {
      const element = this.ownerDocument.createElement('p');
      element.className = 'notice';
      element.dataset['role'] = 'provenance';
      element.textContent = notice;
      body.append(element);
    }

    const list = this.ownerDocument.createElement('ul');
    for (const plate of manifest.plates) list.append(this.#plate(plate, source));
    body.append(list);
  }

  #plate(plate: Plate, source: string): HTMLElement {
    const item = this.ownerDocument.createElement('li');
    const figure = this.ownerDocument.createElement('figure');
    const image = this.ownerDocument.createElement('img');
    image.src = plateUrl(source, plate);
    image.width = plate.width;
    image.height = plate.height;
    // The description, not the name: a screen reader gets what the picture is of, and the name is
    // already in the caption beside it.
    image.alt = plate.description;
    const caption = this.ownerDocument.createElement('figcaption');
    caption.textContent = `${plate.name} — ${plate.description}`;

    const facts = this.ownerDocument.createElement('dl');
    const rows: readonly (readonly [string, string])[] = [
      ['size', `${String(plate.width)} × ${String(plate.height)}`],
      ['draw calls', String(plate.drawCalls)],
      ['placeholders', String(plate.placeholders)],
      ['covered pixels', String(plate.covered)],
      ['approved by', plate.reviewed ? plate.approver : `${plate.approver} (not a human review)`],
      ['parity with Office', plate.parity ? 'claimed' : 'not claimed'],
      ...(plate.excluded === null ? [] : ([['excluded', plate.excluded]] as const)),
      ...(plate.difference === null ? [] : ([['differs from baseline', plate.difference]] as const)),
    ];
    for (const [term, definition] of rows) {
      const dt = this.ownerDocument.createElement('dt');
      dt.textContent = term;
      const dd = this.ownerDocument.createElement('dd');
      dd.textContent = definition;
      facts.append(dt, dd);
    }

    figure.append(image, caption, facts);
    item.append(figure);
    return item;
  }
}

/** Register the element. Idempotent. */
export function definePlateGallery(): void {
  if (customElements.get('mjx-plate-gallery') === undefined) {
    customElements.define('mjx-plate-gallery', MjxPlateGallery);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-plate-gallery': MjxPlateGallery;
  }
}
