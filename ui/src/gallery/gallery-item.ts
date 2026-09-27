/**
 * `<mjx-gallery-item>` — **a descriptor, not a rendered element.**
 *
 * ```html
 * <mjx-gallery-item value="heading-1" label="Heading 1" category="Built-In">
 *   <p style="font-size:1.4em;font-weight:800">AaBbCc</p>
 * </mjx-gallery-item>
 * ```
 *
 * MJXOFF-185: *"Item rendering is arbitrary content — a style gallery item is a miniature of
 * formatted text, a shape gallery item is a shape, a colour gallery item is a swatch. **The item is
 * a slot, not an icon.**"*
 *
 * ## It is a slot, and it is deliberately not a `<slot>`
 *
 * The obvious implementation is a shadow root with a `<slot>` in it, and it is wrong here in two
 * ways that a gallery — and only a gallery — feels:
 *
 * * **A slotted node can be in exactly one place.** The in-ribbon strip and the expanded flyout are
 *   two geometries of *one* item set and they are on screen at the same time. Slotted items would
 *   have to be authored twice, and two copies of an item set is precisely how the two surfaces come
 *   to disagree about which one is selected — the failure the brief calls out by name.
 * * **A slotted node is rendered.** *"A theme gallery with hundreds of entries must not build
 *   hundreds of nodes"* is not satisfiable while every item's markup is live in the light DOM: the
 *   nodes are already built, and hiding them is not virtualising them.
 *
 * So this element **captures** its children into an inert `DocumentFragment` on connect and leaves
 * itself empty. A fragment has no layout boxes, upgrades no custom elements and paints nothing, and
 * the gallery clones it into whichever cells it decides to build. That is the same trick a
 * `<template>` plays, done for the author so that the markup they write is ordinary markup they can
 * see in devtools rather than something wrapped in a tag they have to remember.
 *
 * The capture is re-run when the children change, so a framework re-rendering the item's content —
 * which is what a story does on every scheme switch — is picked up rather than lost.
 *
 * ## ⚠ The art is cloned into a **shadow root**, so it carries its own styling
 *
 * The gallery's cells live in the gallery's shadow tree, and a document stylesheet does not reach
 * inside one. An item whose art relies on a class defined in the page therefore renders as an empty
 * box — measured, on the swatch specimens, which came out as eight captions and no colour while
 * every gate that counted cells or read a caption stayed green. **Custom properties do cross the
 * boundary**, so the answer is inline style with `var(--theme-…)` values rather than a literal, and
 * `tests/browser/gallery.spec.ts` asserts that a swatch actually paints something.
 *
 * That is the price of the item being reusable in two surfaces at once, and it is the same price a
 * `<template>` charges. It is stated here, in `stories/gallery/specimens.ts`, and in `ui/README.md`,
 * because it is the one thing about this component that is not discoverable from its API.
 *
 * ## What it announces
 *
 * Nothing, itself. It is `display: none` and carries no role: the *cell* the gallery builds is what
 * a screen reader meets, and it takes its accessible name from `label` rather than from the item's
 * content, because the content of a swatch is a colour and the content of a style miniature is the
 * word *"AaBbCc"*. Neither is the name of anything.
 */

import {
  galleryTags,
  galleryEvents,
  uncategorisedSectionLabel,
} from './gallery-model.ts';

/** Everything the gallery needs to build a cell. */
export interface GalleryItemDescriptor {
  /** The value the events carry. Falls back to the label. */
  readonly value: string;
  /** The item's name — the cell's caption and its accessible name. */
  readonly label: string;
  /** The section it belongs to in the expanded surface. */
  readonly category: string;
  /** Whether it may be chosen. An unavailable item stays reachable and never previews. */
  readonly unavailable: boolean;
  /** Why not, when it is unavailable. */
  readonly explanation: string;
  /** The element the descriptor came from, so the gallery can report a selection by identity. */
  readonly source: MjxGalleryItem;
}

export class MjxGalleryItem extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'value',
    'label',
    'category',
    'unavailable',
    'explanation',
  ];

  #content: DocumentFragment | undefined;
  #observer: MutationObserver | undefined;

  connectedCallback(): void {
    // No shadow root and no role: this element is data. `galleryDocumentCss` and the gallery's own
    // `::slotted` rule both hide it, so it never occupies a pixel even before it upgrades.
    this.hidden = true;
    this.#capture();
    if (this.#observer === undefined && typeof MutationObserver !== 'undefined') {
      this.#observer = new MutationObserver(() => {
        this.#capture();
      });
    }
    this.#observer?.observe(this, { childList: true });
  }

  disconnectedCallback(): void {
    this.#observer?.disconnect();
  }

  attributeChangedCallback(): void {
    // The gallery re-reads descriptors on this, rather than the item pushing a re-render: a cell is
    // the gallery's node and only the gallery knows whether it is currently built.
    this.dispatchEvent(new CustomEvent(itemChangedEvent, { bubbles: true, composed: true }));
  }

  /** The value the protocol's events carry. */
  get value(): string {
    return this.getAttribute('value') ?? this.label;
  }

  /** The item's name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The section it joins in the expanded surface. */
  get category(): string {
    const declared = (this.getAttribute('category') ?? '').trim();
    return declared === '' ? uncategorisedSectionLabel : declared;
  }

  /** Whether it may be chosen. */
  get unavailable(): boolean {
    return this.hasAttribute('unavailable');
  }

  /** Why not. */
  get explanation(): string {
    return this.getAttribute('explanation') ?? '';
  }

  /** Everything the gallery needs, in one object. */
  get descriptor(): GalleryItemDescriptor {
    return {
      value: this.value,
      label: this.label,
      category: this.category,
      unavailable: this.unavailable,
      explanation: this.explanation,
      source: this,
    };
  }

  /**
   * A fresh copy of the item's art, for one cell.
   *
   * A clone per cell rather than a moved node, because the same item may be built in the strip and
   * in the flyout **at the same time** — which is the whole reason the content lives in a fragment.
   */
  cloneContent(): DocumentFragment {
    const content = this.#content;
    if (content === undefined) return document.createDocumentFragment();
    return content.cloneNode(true) as DocumentFragment;
  }

  /** Whether the item has any art at all. Exposed so a gate can say *nothing was captured*. */
  get hasContent(): boolean {
    return (this.#content?.childNodes.length ?? 0) > 0;
  }

  /**
   * Move the light DOM into a fragment.
   *
   * Whitespace-only text is dropped, because a template's indentation is not art and a cell that
   * kept it would have a stray newline changing its measured height — and the row pitch every
   * virtual window is computed from is a measured height.
   */
  #capture(): void {
    if (this.childNodes.length === 0) return;
    const fragment = document.createDocumentFragment();
    while (this.firstChild !== null) {
      const node = this.firstChild;
      this.removeChild(node);
      if (node.nodeType === Node.TEXT_NODE && (node.textContent ?? '').trim() === '') continue;
      fragment.append(node);
    }
    if (fragment.childNodes.length === 0) return;
    this.#content = fragment;
    this.dispatchEvent(new CustomEvent(itemChangedEvent, { bubbles: true, composed: true }));
  }
}

/**
 * The private event an item fires when anything about it changed.
 *
 * Named here rather than in `gallery-model.ts` because it is **not** part of the published
 * protocol: `galleryEvents` is what a shell listens to, and this is how two elements of one
 * component talk to each other. A shell that listened to it would be depending on an internal.
 */
export const itemChangedEvent = 'mjx-gallery-item-changed';

/** Register the element. Idempotent. */
export function defineGalleryItem(): void {
  if (customElements.get(galleryTags.item) === undefined) {
    customElements.define(galleryTags.item, MjxGalleryItem);
  }
}

/** Re-exported so a story imports one module. */
export { galleryEvents };

declare global {
  interface HTMLElementTagNameMap {
    'mjx-gallery-item': MjxGalleryItem;
  }
}
