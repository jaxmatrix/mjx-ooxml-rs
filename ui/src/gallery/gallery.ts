/**
 * `<mjx-gallery>` — the in-ribbon strip, the expanded flyout, and live preview.
 *
 * ```html
 * <mjx-gallery label="Styles" value="normal">
 *   <mjx-gallery-item value="normal" label="Normal" category="Built-In">…</mjx-gallery-item>
 *   <mjx-gallery-item value="title"  label="Title"  category="Built-In">…</mjx-gallery-item>
 *   <mjx-button slot="footer" label="Save Selection as a New Style"></mjx-button>
 * </mjx-gallery>
 * ```
 *
 * `gallery` is **1,773 of Office's published controls** — the second-largest archetype after
 * `button` — and the parts of this file that matter are the parts a picture cannot show. What a
 * screenshot proves about a gallery is nothing at all: a gallery with the preview protocol unwired
 * looks identical in every frame.
 *
 * ## One item set, two geometries, one selection
 *
 * The strip and the expanded surface are **drawings of the same list**, built by the same code from
 * the same descriptors, with **one** `#activeIndex` and **one** committed value between them. That
 * is structural rather than remembered, and it is the answer to the failure the brief names:
 *
 * > The strip's scroll-by-row and the flyout's grid are different geometries of one item set. A
 * > gate that tests them separately can pass while they disagree about which item is selected.
 *
 * There is nothing to keep in step, because there is only one of it. What the gate then asserts is
 * the *observable* consequence — `aria-selected`, the roving tab stop and the scroll position — on
 * both surfaces, in both directions across the transition.
 *
 * ## Virtualisation is not optional and it is not cosmetic
 *
 * `<mjx-gallery-item>` holds its art in an inert fragment, so an item that is not on screen has
 * **no rendered nodes at all**. Each surface plans its rows (`galleryRowPlan`), takes a window
 * (`galleryWindow`), and builds only that. The sizer is as tall as *every* row, built or not, so
 * the scrollbar tells the truth — a virtualised list whose scroller is only as tall as what it
 * built is a list that claims to be three rows long.
 *
 * The window is then **extended to contain the active row**, which is worth a sentence because it
 * is what keeps the roving tab stop reachable. Without it, scrolling the active cell out of view
 * leaves a listbox with no `tabindex="0"` in it — a surface a keyboard cannot enter — and the fix
 * of putting the tab stop on the scroller instead is two focus behaviours where one will do.
 *
 * ## Two measurements, and only one of them is a layout decision
 *
 * MJXOFF-183 ruled out *measuring in a resize handler* to decide a presentation, and that rule is
 * kept: **how many columns there are is CSS's decision** — `repeat(auto-fill, minmax(…, 1fr))` —
 * and which presentation the surface is in is read back out of `--mjx-gallery-presentation` exactly
 * as `<mjx-ribbon-group>` and `<mjx-menu>` do. What the component measures is *what CSS decided*,
 * by reading the used `grid-template-columns` off a zero-height probe row, because two-dimensional
 * keyboard navigation cannot be done without knowing the column count and no amount of CSS will
 * tell `ArrowDown` what it means. **Those are different questions**, and the second one is not a
 * layout decision at all.
 *
 * ## The preview protocol
 *
 * `preview-session.ts` owns it and this file owns the *timing*, which is the only part that depends
 * on what a hand is doing:
 *
 * * **Pointer — coalesced.** Entering a cell schedules a request; entering another before it fires
 *   replaces it. Ten cells crossed quickly produce one request, and `preview-delay="0"` turns that
 *   off so the gate has a run where the assertion demonstrably fails.
 * * **Keyboard — immediate.** `<mjx-menu>`'s rule: *"the keyboard never waits for any of it."* An
 *   arrow key is a deliberate act per item, and the outstanding-preview invariant is what protects
 *   the renderer instead of a delay.
 * * **Touch — press and hold previews, a tap commits.** The affordance is chosen and argued in
 *   `touchPreviewAffordance`, and marked `GUESS:` there, because Office's own mobile applications
 *   have no live preview to be parity with.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { isForcibleState } from '../controls/control-states.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  applyPlacement,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingCss,
  floatingProperties,
  installFloatingProperties,
  pinFloating,
  placeFloating,
  rectOf,
  resolveLength,
  type Placement,
  type Rect,
} from '../overlay/floating.ts';
import {
  cellsInWindow,
  galleryBoxProperties,
  galleryEvents,
  galleryFlyoutAlign,
  galleryFlyoutSide,
  galleryForcedStateAttribute,
  galleryGroupPresentationAttribute,
  galleryKeyAction,
  galleryMotionClass,
  galleryOpenAttribute,
  galleryOverscanRows,
  galleryPresentationOrder,
  galleryPresentationProperty,
  galleryPresentations,
  galleryRowPlan,
  gallerySheet,
  gallerySheetBoundaryFraction,
  gallerySurfaceAttribute,
  galleryTags,
  galleryTypeClass,
  galleryWindow,
  groupPresentationProperty,
  isGalleryMovement,
  nextGalleryIndex,
  pointerPreviewSettleDelay,
  previewDelayAttribute,
  galleryTouchPreviewDelay,
  type GalleryPresentation,
  type GalleryRow,
  type GallerySurface,
  type GalleryWindow,
  type PreviewSource,
} from './gallery-model.ts';
import { groupPresentationOrder, type GroupPresentation } from '../ribbon/ribbon-model.ts';
import { longPressMoveTolerance } from '../menus/menu-model.ts';
import { itemChangedEvent } from './gallery-item.ts';
import type { GalleryItemDescriptor, MjxGalleryItem } from './gallery-item.ts';
import { PreviewSession, type PreviewEvent } from './preview-session.ts';

type Timer = ReturnType<typeof setTimeout>;

/** Everything one surface owns. Two of these; one component. */
interface Surface {
  readonly name: GallerySurface;
  readonly root: HTMLElement;
  readonly viewport: HTMLElement;
  readonly sizer: HTMLElement;
  readonly layer: HTMLElement;
  readonly probe: HTMLElement;
  /** Whether it groups items into their categories. The strip is linear; the flyout is not. */
  readonly sectioned: boolean;
  plan: readonly GalleryRow[];
  /** The block offset of every row, plus one past the end. Length is `plan.length + 1`. */
  offsets: readonly number[];
  columns: number;
  rowPitch: number;
  headingPitch: number;
  built: GalleryWindow;
  /** Everything `#buildRows` reads, so a rebuild that would change nothing can be skipped. */
  signature: string;
  /** The cells currently in the DOM, by item index. */
  cells: Map<number, HTMLElement>;
}

/** The affordances in the strip's rail. */
const affordanceIcons = {
  scrollBack: { name: 'chevron-up', size: 16 },
  scrollForward: { name: 'chevron-down', size: 16 },
  expand: { name: 'chevron-double-down', size: 16 },
} as const;

/**
 * The names those three affordances announce.
 *
 * `GUESS:` Office labels its More button with the gallery's own name and nothing else; these say
 * what they do, because an icon-only control in a rail of three identical-sized buttons is exactly
 * where *"Styles"* three times over is useless.
 */
function affordanceLabels(label: string): Readonly<Record<keyof typeof affordanceIcons, string>> {
  const name = label.trim() === '' ? 'gallery' : label.trim();
  return {
    scrollBack: `Scroll ${name} back one row`,
    scrollForward: `Scroll ${name} forward one row`,
    expand: `More ${name}`,
  };
}

export class MjxGallery extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'expanded',
    previewDelayAttribute,
    galleryForcedStateAttribute,
  ];

  #root: ShadowRoot | undefined;
  #strip: Surface | undefined;
  #expanded: Surface | undefined;
  #scrollBack: HTMLButtonElement | undefined;
  #scrollForward: HTMLButtonElement | undefined;
  #expandButton: HTMLButtonElement | undefined;
  #descriptors: GalleryItemDescriptor[] = [];
  #activeIndex = 0;
  #session = new PreviewSession();
  #pointerTimer: Timer | undefined;
  #pointerPending: number | undefined;
  #holdTimer: Timer | undefined;
  #holdIndex: number | undefined;
  #holdOrigin: { x: number; y: number } | undefined;
  #holdPreviewed = false;
  #pointerDownRecently = false;
  #placement: Placement | undefined;
  #anchor: Rect | undefined;
  #watching = false;
  #observer: ResizeObserver | undefined;
  #dismissing = false;
  #groupPresentation: GroupPresentation = 'full';
  /** Bumped whenever the descriptors are re-read, so a rebuild cannot be skipped over new data. */
  #revision = 0;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    // The cells live in this shadow root and wear the foundations' focus ring; the `@property`
    // registrations are document-scoped. `ui/README.md` states the rule and this component needs
    // both halves of it, exactly as `<mjx-menu>` does.
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.#session.adopt(this.getAttribute('value') ?? undefined);
    this.#collect();
    this.render();
    this.#startObserving();
  }

  disconnectedCallback(): void {
    this.#stopObserving();
    this.#stopWatching();
    this.#clearPointerTimer();
    this.#clearHold();
  }

  attributeChangedCallback(name: string, previous: string | null, next: string | null): void {
    if (this.#root === undefined || previous === next) return;
    if (name === 'value') {
      // A committed value that moved under a live preview would be a `restore` that put back
      // something that was never there, so the preview is taken back first.
      this.#emit(this.#session.cancel('pointer'));
      this.#session.adopt(next ?? undefined);
    }
    if (name === 'expanded') {
      if (next === null) this.#teardownExpanded();
      else this.#showExpanded();
      return;
    }
    this.render();
  }

  // ── the declared surface ───────────────────────────────────────────────────

  /** The gallery's accessible name. Both listboxes announce it. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The committed value — the item that is actually applied. */
  get value(): string | undefined {
    return this.#session.committed;
  }

  set value(next: string | undefined) {
    if (next === undefined) this.removeAttribute('value');
    else this.setAttribute('value', next);
  }

  /** Whether the expanded surface is showing. */
  get expanded(): boolean {
    return this.hasAttribute('expanded');
  }

  set expanded(next: boolean) {
    if (next) this.setAttribute('expanded', '');
    else this.removeAttribute('expanded');
  }

  /** Every item, in document order. Matched by tag name for the reason `<mjx-menu>` gives. */
  get items(): MjxGalleryItem[] {
    const found: MjxGalleryItem[] = [];
    for (const child of this.children) {
      if (child.localName === galleryTags.item) found.push(child as MjxGalleryItem);
    }
    return found;
  }

  /** The descriptors the surfaces are built from. */
  get descriptors(): readonly GalleryItemDescriptor[] {
    return this.#descriptors;
  }

  /** Which cell holds the roving tab stop, in both surfaces at once. */
  get activeIndex(): number {
    return this.#activeIndex;
  }

  /** The index of the committed item, or `-1`. */
  get selectedIndex(): number {
    const value = this.#session.committed;
    if (value === undefined) return -1;
    return this.#descriptors.findIndex((item) => item.value === value);
  }

  /** The value being previewed right now, or `undefined`. **Exposed so a gate can name it.** */
  get previewing(): string | undefined {
    return this.#session.previewing;
  }

  /** How many previews are outstanding. The protocol's invariant is that this is never above one. */
  get outstandingPreviews(): number {
    return this.#session.outstanding;
  }

  /**
   * **Which presentation CSS put this gallery in** — read back, never computed here.
   *
   * `galleryPresentationAt()` says what it *should* be and the browser gate compares the two. A
   * component that decided its own presentation and then reported it would be grading its own
   * homework.
   */
  get presentation(): GalleryPresentation {
    const surface = this.expanded ? this.#expanded : this.#strip;
    if (surface === undefined) return 'strip';
    const value = getComputedStyle(surface.root).getPropertyValue(galleryPresentationProperty).trim();
    return (galleryPresentationOrder as readonly string[]).includes(value)
      ? (value as GalleryPresentation)
      : 'strip';
  }

  /** The ribbon group's presentation, as this gallery read it. `full` when there is no group. */
  get groupPresentation(): GroupPresentation {
    return this.#groupPresentation;
  }

  /** How many columns CSS laid out, per surface. What `ArrowDown` moves by. */
  columnsIn(name: GallerySurface): number {
    return this.#surface(name)?.columns ?? 1;
  }

  /** How many cells are actually in the DOM, per surface. **The node-count gate reads this.** */
  builtCellsIn(name: GallerySurface): number {
    return this.#surface(name)?.cells.size ?? 0;
  }

  /** The window a surface built, so the gate can compare it against `galleryWindow()`. */
  windowIn(name: GallerySurface): GalleryWindow {
    return this.#surface(name)?.built ?? { firstRow: 0, lastRow: 0 };
  }

  /** The row plan a surface is working from, so the gate can compare it against `galleryRowPlan()`. */
  planIn(name: GallerySurface): readonly GalleryRow[] {
    return this.#surface(name)?.plan ?? [];
  }

  /** How many cells the plan says the built window holds. Derived, never counted from the DOM. */
  expectedCellsIn(name: GallerySurface): number {
    const surface = this.#surface(name);
    if (surface === undefined) return 0;
    return cellsInWindow(surface.plan, surface.built);
  }

  /** The placement the last expansion produced, for a gate to compare against `placeFloating`. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /** The anchor it was placed against. */
  get anchorRect(): Rect | undefined {
    return this.#anchor;
  }

  /** The rectangle it had to stay inside. */
  get boundaryRect(): Rect | undefined {
    const surface = this.#expanded;
    if (surface === undefined) return undefined;
    return clippingBoundary(
      surface.root,
      resolveLength(surface.root, floatingProperties.boundaryInset),
    );
  }

  /** How long the pointer path waits before requesting a preview. */
  get previewDelay(): number {
    const declared = this.getAttribute(previewDelayAttribute);
    if (declared === null) return pointerPreviewSettleDelay;
    const value = Number.parseInt(declared, 10);
    return Number.isFinite(value) && value >= 0 ? value : pointerPreviewSettleDelay;
  }

  // ── construction ───────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, [floatingCss, gallerySheet].join('\n'));

    this.#strip = this.#buildSurface('strip', false);
    this.#expanded = this.#buildSurface('expanded', true);

    const rail = document.createElement('div');
    rail.className = 'rail';
    rail.setAttribute('part', 'rail');

    this.#scrollBack = this.#buildAffordance('scrollBack', 'scroll-back');
    this.#scrollForward = this.#buildAffordance('scrollForward', 'scroll-forward');
    this.#expandButton = this.#buildAffordance('expand', 'expand');
    this.#expandButton.setAttribute('aria-haspopup', 'listbox');
    rail.append(this.#scrollBack, this.#scrollForward, this.#expandButton);
    this.#strip.root.append(rail);

    const footer = document.createElement('div');
    footer.className = 'footer';
    footer.setAttribute('part', 'footer');
    const footerSlot = document.createElement('slot');
    footerSlot.name = 'footer';
    footer.append(footerSlot);
    this.#expanded.root.append(footer);

    const itemSlot = document.createElement('slot');
    itemSlot.addEventListener('slotchange', this.#onSlotChange);

    root.append(this.#strip.root, this.#expanded.root, itemSlot);

    this.addEventListener(itemChangedEvent, this.#onItemChanged);
    this.addEventListener('keydown', this.#onKeyDown);
  }

  #buildSurface(name: GallerySurface, sectioned: boolean): Surface {
    const root = document.createElement('div');
    root.className = `surface ${galleryMotionClass(sectioned ? 'flyout' : 'strip')}`;
    root.setAttribute(gallerySurfaceAttribute, name);
    root.setAttribute('part', sectioned ? 'flyout' : 'strip');

    const viewport = document.createElement('div');
    viewport.className = 'viewport';
    viewport.setAttribute('part', 'viewport');
    // A gallery is a **listbox**, not a menu, and MJXOFF-185 says the distinction matters to a
    // screen reader: a listbox is a set of choices one of which is currently in force, which is
    // exactly what a gallery is, and a menu is a set of commands, which it is not.
    viewport.setAttribute('role', 'listbox');
    viewport.addEventListener('scroll', this.#onScroll, { passive: true });
    viewport.addEventListener('pointerover', this.#onPointerOver);
    viewport.addEventListener('pointerleave', this.#onPointerLeave);
    viewport.addEventListener('pointerdown', this.#onPointerDown);
    viewport.addEventListener('pointerup', this.#onPointerUp);
    viewport.addEventListener('pointercancel', this.#onPointerUp);
    viewport.addEventListener('pointermove', this.#onPointerMove);
    viewport.addEventListener('click', this.#onClick);
    viewport.addEventListener('focusin', this.#onFocusIn);
    viewport.addEventListener('focusout', this.#onFocusOut);

    const sizer = document.createElement('div');
    sizer.className = 'sizer';
    // ⚠ Presentational on purpose. `role="listbox"` owns `option` children, and the sizer and the
    // layer sit between them because a virtual list needs a spacer that is as tall as every row and
    // a layer that is pushed down inside it. Saying so explicitly is what stops an accessibility
    // checker walking the listbox's owned children and finding a `<div>` where an `option` belongs.
    sizer.setAttribute('role', 'presentation');

    const layer = document.createElement('div');
    layer.className = 'layer';
    layer.setAttribute('role', 'presentation');

    // The probe is how the component learns **what CSS decided**, rather than deciding it: the used
    // value of `grid-template-columns` is a list of tracks, and counting them is exact, costs no
    // arithmetic over tokens, and is right at every width and every density without knowing what
    // `--spacing` is. It is zero-height and hidden, so it is not a cell and is not announced.
    const probe = document.createElement('div');
    probe.className = 'row';
    probe.dataset['kind'] = 'probe';
    probe.setAttribute('role', 'presentation');
    probe.setAttribute('aria-hidden', 'true');
    probe.style.blockSize = '0';
    probe.style.visibility = 'hidden';

    sizer.append(layer);
    viewport.append(probe, sizer);
    root.append(viewport);

    return {
      name,
      root,
      viewport,
      sizer,
      layer,
      probe,
      sectioned,
      plan: [],
      offsets: [0],
      columns: 1,
      rowPitch: 0,
      headingPitch: 0,
      built: { firstRow: 0, lastRow: 0 },
      signature: '',
      cells: new Map(),
    };
  }

  #buildAffordance(kind: keyof typeof affordanceIcons, part: string): HTMLButtonElement {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'affordance mjx-hit-target mjx-motion-surface-settle';
    button.setAttribute('part', part);
    button.dataset['affordance'] = kind;
    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', affordanceIcons[kind].name);
    glyph.setAttribute('size', String(affordanceIcons[kind].size));
    glyph.setAttribute('variant', 'regular');
    button.append(glyph);
    button.addEventListener('click', this.#onAffordance);
    return button;
  }

  #surface(name: GallerySurface): Surface | undefined {
    return name === 'strip' ? this.#strip : this.#expanded;
  }

  #surfaces(): Surface[] {
    const found: Surface[] = [];
    if (this.#strip !== undefined) found.push(this.#strip);
    if (this.#expanded !== undefined) found.push(this.#expanded);
    return found;
  }

  // ── the item set ───────────────────────────────────────────────────────────

  #onSlotChange = (): void => {
    this.#collect();
    this.render();
  };

  #onItemChanged = (): void => {
    this.#collect();
    this.render();
  };

  /**
   * Re-read every item's descriptor.
   *
   * ⚠ **`customElements.upgrade` is load-bearing, and this is the measured reason.** Custom elements
   * are upgraded in tree order, so a gallery's own `connectedCallback` runs while its
   * `<mjx-gallery-item>` children are still plain `HTMLElement`s — no `descriptor`, and no captured
   * art. The first build therefore produced cells with captions and **empty art boxes**, and the
   * later `slotchange` did not fix them because the rebuild was skipped: the window, the column
   * count and the item count were all unchanged, so the signature matched. The strip stayed empty
   * while the flyout — built later, from real descriptors — was perfect, which is exactly the shape
   * of defect that survives a screenshot of the thing that works.
   *
   * Upgrading the children here removes the race rather than waiting it out, and `#revision` makes
   * *the data changed* part of the rebuild signature so a future one cannot hide the same way.
   */
  #collect(): void {
    for (const child of this.children) {
      if (child.localName === galleryTags.item) customElements.upgrade(child);
    }
    this.#descriptors = this.items
      .filter((item): item is MjxGalleryItem => typeof item.descriptor === 'object')
      .map((item) => item.descriptor);
    this.#revision += 1;
    if (this.#activeIndex >= this.#descriptors.length) {
      this.#activeIndex = Math.max(0, this.#descriptors.length - 1);
    }
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render both surfaces from the attributes and the item set. */
  render(): void {
    if (this.#root === undefined) return;
    this.#readGroupPresentation();

    const labels = affordanceLabels(this.label);
    this.#scrollBack?.setAttribute('aria-label', labels.scrollBack);
    this.#scrollForward?.setAttribute('aria-label', labels.scrollForward);
    this.#expandButton?.setAttribute('aria-label', labels.expand);
    this.#expandButton?.setAttribute('aria-expanded', this.expanded ? 'true' : 'false');

    const expanded = this.#expanded;
    if (expanded !== undefined) {
      expanded.root.setAttribute(galleryOpenAttribute, this.expanded ? 'true' : 'false');
      this.#syncTopLayer(expanded.root);
    }
    // While the flyout is showing, the strip is a second listbox holding the same options. `inert`
    // takes it out of the tab order **and** out of the accessibility tree, which is what stops a
    // screen reader offering the same forty styles twice.
    const strip = this.#strip;
    if (strip !== undefined) strip.root.inert = this.expanded;

    for (const surface of this.#surfaces()) this.#renderSurface(surface);
    // ⚠ After the surfaces, and unconditionally. A rebuild that was skipped because nothing about
    // the *window* changed still has to be told the selection moved — a host setting `value`
    // programmatically goes through here and through no other path, and without this the cell that
    // used to be selected kept its mark in both surfaces while `value` said something else.
    this.#syncSelection();
    this.#syncAffordances();
  }

  /**
   * Mirror the ribbon group's presentation onto both surfaces.
   *
   * `--mjx-group-presentation` is published by `<mjx-ribbon-group>` on the box that contains its
   * slot, and custom properties inherit through the flat tree — so a slotted gallery reads the
   * group's own decision without measuring anything and without the group knowing what a gallery
   * is. The value is mirrored to an attribute because a decision a gate must read has to be
   * somewhere `getComputedStyle` can see it, and because CSS cannot match on a custom property's
   * value without a style container query this catalogue does not otherwise depend on.
   */
  #readGroupPresentation(): void {
    const declared = getComputedStyle(this).getPropertyValue(groupPresentationProperty).trim();
    const presentation = (groupPresentationOrder as readonly string[]).includes(declared)
      ? (declared as GroupPresentation)
      : 'full';
    this.#groupPresentation = presentation;
    for (const surface of this.#surfaces()) {
      surface.root.setAttribute(galleryGroupPresentationAttribute, presentation);
    }
  }

  #renderSurface(surface: Surface): void {
    surface.viewport.setAttribute('aria-label', this.label);
    this.#measure(surface);

    const categories = this.#descriptors.map((item) => item.category);
    surface.plan = galleryRowPlan(categories, surface.columns, surface.sectioned);
    surface.offsets = rowOffsets(surface);

    const total = surface.offsets[surface.offsets.length - 1] ?? 0;
    surface.sizer.style.setProperty(
      galleryBoxProperties.scrollBlockSize,
      `${String(Math.round(total))}px`,
    );
    surface.root.style.setProperty(
      galleryBoxProperties.rowPitch,
      `${String(Math.round(surface.rowPitch + this.#gap(surface)))}px`,
    );

    surface.built = this.#windowFor(surface);
    // A rebuild that would produce the same cells is a rebuild that costs a focus and a paint for
    // nothing. The signature is every input `#buildRows` reads; the selection is deliberately not
    // in it, because `#syncSelection` moves that without touching a node.
    const signature = [
      surface.columns,
      surface.plan.length,
      surface.built.firstRow,
      surface.built.lastRow,
      this.#descriptors.length,
      this.#revision,
      this.getAttribute(galleryForcedStateAttribute) ?? '',
    ].join(':');
    if (signature === surface.signature) return;
    surface.signature = signature;
    this.#buildRows(surface);
  }

  /**
   * The window this surface builds: what is visible, plus the overscan, **plus the active row.**
   *
   * The last clause is what keeps the roving tab stop reachable. A listbox whose only
   * `tabindex="0"` has been scrolled out of the built window is a listbox `Tab` cannot enter, and
   * the alternative — moving the tab stop onto the scroller when that happens — is two focus
   * behaviours where one will do.
   */
  #windowFor(surface: Surface): GalleryWindow {
    const rowCount = surface.plan.length;
    if (rowCount === 0) return { firstRow: 0, lastRow: 0 };
    const scrollTop = surface.viewport.scrollTop;
    const height = surface.viewport.clientHeight;
    const firstVisible = rowIndexAt(surface.offsets, scrollTop);
    const lastVisible = rowIndexAt(surface.offsets, scrollTop + height);
    const visible = Math.max(1, lastVisible - firstVisible + 1);
    const scrolled = galleryWindow(rowCount, firstVisible, visible, galleryOverscanRows);
    const activeRow = this.#rowOf(surface, this.#activeIndex);
    if (activeRow < 0) return scrolled;
    return {
      firstRow: Math.min(scrolled.firstRow, activeRow),
      lastRow: Math.max(scrolled.lastRow, activeRow + 1),
    };
  }

  /** Which row of a surface's plan holds an item, or `-1`. */
  #rowOf(surface: Surface, index: number): number {
    for (const [row, entry] of surface.plan.entries()) {
      if (entry.kind === 'cells' && index >= entry.start && index < entry.end) return row;
    }
    return -1;
  }

  #buildRows(surface: Surface): void {
    // `document.activeElement` is the **host** while focus is inside a shadow root, so a
    // `contains` against it is false for every cell this component owns. Following the focus down
    // is the difference between a rebuild that keeps the keyboard where it was and one that drops
    // it on the floor on every scroll.
    const hadFocus = surface.viewport.contains(deepActiveElement(this.ownerDocument));
    surface.layer.replaceChildren();
    surface.cells.clear();

    const offset = surface.offsets[surface.built.firstRow] ?? 0;
    surface.layer.style.setProperty(
      galleryBoxProperties.layerOffset,
      `${String(Math.round(offset))}px`,
    );

    let section: HTMLElement | undefined;
    for (let index = surface.built.firstRow; index < surface.built.lastRow; index += 1) {
      const row = surface.plan[index];
      if (row === undefined) continue;
      if (row.kind === 'heading') {
        section = document.createElement('div');
        section.className = 'section';
        section.setAttribute('part', 'section');
        // A section is a `group` inside the listbox — the ARIA structure a categorised list of
        // choices actually has. The heading is `aria-hidden` because the group already carries the
        // same words, and a screen reader that read both would announce every category twice.
        section.setAttribute('role', 'group');
        section.setAttribute('aria-label', row.category);
        const heading = document.createElement('p');
        heading.className = `heading ${galleryTypeClass('heading')}`;
        heading.setAttribute('part', 'heading');
        heading.setAttribute('aria-hidden', 'true');
        heading.textContent = row.category;
        const headingRow = document.createElement('div');
        headingRow.className = 'row';
        headingRow.dataset['kind'] = 'heading';
        headingRow.setAttribute('role', 'presentation');
        headingRow.append(heading);
        section.append(headingRow);
        surface.layer.append(section);
        continue;
      }
      const container = surface.sectioned && section !== undefined ? section : surface.layer;
      const element = document.createElement('div');
      element.className = 'row';
      element.dataset['kind'] = 'cells';
      element.setAttribute('part', 'grid');
      element.setAttribute('role', 'presentation');
      for (let item = row.start; item < row.end; item += 1) {
        const cell = this.#buildCell(surface, item);
        if (cell === undefined) continue;
        element.append(cell);
        surface.cells.set(item, cell);
      }
      container.append(element);
    }

    this.#syncTabStops(surface);
    if (hadFocus) surface.cells.get(this.#activeIndex)?.focus({ preventScroll: true });
  }

  #buildCell(surface: Surface, index: number): HTMLElement | undefined {
    const item = this.#descriptors[index];
    if (item === undefined) return undefined;

    const cell = document.createElement('div');
    // ⚠ The type role goes on **the element the state table paints**, never on the caption inside
    // it. See `galleryTypeRoles`: a role class on `.caption` declares its own `font-weight` and
    // would beat the weight a selected cell inherits from its state.
    cell.className = `cell ${galleryTypeClass('cell')} mjx-hit-target mjx-motion-surface-settle`;
    cell.setAttribute('part', 'cell');
    cell.setAttribute('role', 'option');
    cell.id = `${surface.name}-cell-${String(index)}`;
    cell.dataset['index'] = String(index);
    cell.dataset['value'] = item.value;
    const selected = item.value === this.#session.committed;
    cell.setAttribute('aria-selected', selected ? 'true' : 'false');
    // The presentation mirror. `aria-selected` is the announcement and `data-selected` is what the
    // state table matches on, exactly as `<mjx-menu-item>` mirrors its ARIA onto `.item` — one way,
    // in one place, so the two cannot drift.
    cell.dataset['selected'] = selected ? 'true' : 'false';
    const forced = this.getAttribute(galleryForcedStateAttribute);
    if (isForcibleState(forced)) cell.dataset['state'] = forced;
    if (item.unavailable) {
      cell.setAttribute('aria-disabled', 'true');
      cell.dataset['unavailable'] = '';
      if (item.explanation.trim() !== '') cell.title = item.explanation;
    }
    // The name comes from `label`, never from the art: the content of a swatch is a colour and the
    // content of a style miniature is the word "AaBbCc". Neither is the name of anything.
    cell.setAttribute(
      'aria-label',
      item.unavailable && item.explanation.trim() !== ''
        ? `${item.label}, ${item.explanation}`
        : item.label,
    );

    const art = document.createElement('div');
    art.className = 'art';
    art.setAttribute('part', 'art');
    art.setAttribute('aria-hidden', 'true');
    art.append(item.source.cloneContent());

    const caption = document.createElement('span');
    caption.className = 'caption';
    caption.setAttribute('part', 'caption');
    caption.setAttribute('aria-hidden', 'true');
    caption.textContent = item.label;

    cell.append(art, caption);
    return cell;
  }

  #syncTabStops(surface: Surface): void {
    for (const [index, cell] of surface.cells) {
      cell.tabIndex = index === this.#activeIndex ? 0 : -1;
    }
  }

  #syncAffordances(): void {
    const surface = this.#strip;
    if (surface === undefined) return;
    const atStart = surface.viewport.scrollTop <= 1;
    const atEnd =
      surface.viewport.scrollTop + surface.viewport.clientHeight >=
      surface.viewport.scrollHeight - 1;
    if (this.#scrollBack !== undefined) this.#scrollBack.disabled = atStart;
    if (this.#scrollForward !== undefined) this.#scrollForward.disabled = atEnd;
  }

  // ── measurement ────────────────────────────────────────────────────────────

  #gap(surface: Surface): number {
    const value = Number.parseFloat(getComputedStyle(surface.layer).rowGap);
    return Number.isFinite(value) ? value : 0;
  }

  /**
   * Read back what CSS decided: how many columns, how tall a row is, how tall a heading is.
   *
   * Every one of these is a *used value*, measured after the browser laid the surface out — not a
   * number this component chose. The column count comes from the probe row's
   * `grid-template-columns`, so it is right at any width, any density and any `--spacing`.
   *
   * The two pitches are read off whatever is currently built and remembered, because the first
   * render has nothing built to measure — the same *"measured twice, one frame apart"* shape
   * `<mjx-menu>`'s placement has, and for the same reason: anything that settles late changes the
   * size the plan was computed for.
   */
  #measure(surface: Surface): void {
    const tracks = getComputedStyle(surface.probe).gridTemplateColumns.trim();
    const columns =
      tracks === '' || tracks === 'none' ? 1 : tracks.split(/\s+/).filter((part) => part !== '').length;
    surface.columns = Math.max(1, columns);

    const row = surface.layer.querySelector('.row[data-kind="cells"]');
    if (row instanceof HTMLElement && row.offsetHeight > 0) surface.rowPitch = row.offsetHeight;
    const heading = surface.layer.querySelector('.row[data-kind="heading"]');
    if (heading instanceof HTMLElement && heading.offsetHeight > 0) {
      surface.headingPitch = heading.offsetHeight;
    }
    if (surface.rowPitch === 0) {
      // The bootstrap. `--mjx-gallery-cell-art-block` plus the cell's own padding is close enough
      // to plan the first window with; the pass after the first paint replaces it with the truth.
      const probe = Number.parseFloat(
        getComputedStyle(surface.root).getPropertyValue(galleryBoxProperties.cellArtBlock),
      );
      surface.rowPitch = Number.isFinite(probe) && probe > 0 ? probe : 1;
    }
    if (surface.headingPitch === 0) surface.headingPitch = surface.rowPitch;
  }

  // ── the pointer ────────────────────────────────────────────────────────────

  #cellFrom(event: Event): { cell: HTMLElement; index: number } | undefined {
    for (const node of event.composedPath()) {
      if (node === this) break;
      if (node instanceof HTMLElement && node.classList.contains('cell')) {
        const index = Number.parseInt(node.dataset['index'] ?? '', 10);
        if (Number.isFinite(index)) return { cell: node, index };
      }
    }
    return undefined;
  }

  #onPointerOver = (event: PointerEvent): void => {
    if (event.pointerType === 'touch') return;
    const found = this.#cellFrom(event);
    if (found === undefined) return;
    this.#schedulePreview(found.index);
  };

  #onPointerLeave = (event: PointerEvent): void => {
    if (event.pointerType === 'touch') return;
    this.#clearPointerTimer();
    this.#emit(this.#session.cancel('pointer'));
  };

  /**
   * The coalescing, and the whole of it.
   *
   * Entering a cell **replaces** whatever was scheduled, so a pointer crossing ten cells on its way
   * to the eleventh schedules eleven times and fires once. A pointer that stops has expressed an
   * intent; a pointer that is still moving has not, and the difference is the only thing standing
   * between a live preview and a renderer asked to lay out a document ten times a second.
   *
   * At `preview-delay="0"` the timer still exists but fires on the next task, which is what makes
   * the gate's *"and here is the same traversal with the coalescing taken away"* run produce ten
   * requests rather than one — the anti-vacuity check MJXOFF-184 established.
   */
  #schedulePreview(index: number): void {
    if (this.#pointerPending === index) return;
    this.#pointerPending = index;
    this.#clearPointerTimer();
    this.#pointerTimer = setTimeout(() => {
      this.#pointerTimer = undefined;
      this.#pointerPending = undefined;
      this.#requestPreview(index, 'pointer');
    }, this.previewDelay);
  }

  #clearPointerTimer(): void {
    if (this.#pointerTimer !== undefined) clearTimeout(this.#pointerTimer);
    this.#pointerTimer = undefined;
    this.#pointerPending = undefined;
  }

  #onPointerDown = (event: PointerEvent): void => {
    const found = this.#cellFrom(event);
    if (found === undefined) return;
    // A click focuses the cell, and `focusin` is a preview path. Recording that the focus came from
    // a pointer keeps the two from being two requests: the session refuses a re-request of the item
    // it is already previewing, and marking the source keeps the event's `by` honest.
    this.#pointerDownRecently = true;
    queueMicrotask(() => {
      this.#pointerDownRecently = false;
    });
    if (event.pointerType !== 'touch') return;

    // ── the touch affordance: press and hold previews, a tap commits ──
    this.#holdIndex = found.index;
    this.#holdOrigin = { x: event.clientX, y: event.clientY };
    this.#holdPreviewed = false;
    this.#holdTimer = setTimeout(() => {
      this.#holdTimer = undefined;
      this.#holdPreviewed = true;
      this.#requestPreview(found.index, 'touch');
    }, galleryTouchPreviewDelay);
  };

  #onPointerMove = (event: PointerEvent): void => {
    if (event.pointerType !== 'touch' || this.#holdOrigin === undefined) return;
    const dx = event.clientX - this.#holdOrigin.x;
    const dy = event.clientY - this.#holdOrigin.y;
    // Past the tolerance it is a scroll, not a press. A gallery on a phone is a scrolling surface
    // first, and a hold that survived a flick would preview whatever the thumb started on.
    if (Math.hypot(dx, dy) > longPressMoveTolerance) this.#clearHold();
  };

  #onPointerUp = (event: PointerEvent): void => {
    if (event.pointerType !== 'touch') return;
    const index = this.#holdIndex;
    const previewed = this.#holdPreviewed;
    const pending = this.#holdTimer !== undefined;
    this.#clearHold();
    if (index === undefined) return;
    if (previewed) {
      // Held long enough to see it, released without choosing. Exactly undone.
      this.#emit(this.#session.cancel('touch'));
      return;
    }
    if (pending) this.#commit(index, 'touch');
  };

  #clearHold(): void {
    if (this.#holdTimer !== undefined) clearTimeout(this.#holdTimer);
    this.#holdTimer = undefined;
    this.#holdIndex = undefined;
    this.#holdOrigin = undefined;
    this.#holdPreviewed = false;
  }

  #onClick = (event: MouseEvent): void => {
    const found = this.#cellFrom(event);
    if (found === undefined) return;
    // A synthesised click from a touch tap has already been committed by `pointerup`; committing
    // twice would emit two `commit` events for one tap.
    if (this.#holdIndex !== undefined) return;
    event.preventDefault();
    this.#commit(found.index, 'pointer');
  };

  // ── focus ──────────────────────────────────────────────────────────────────

  #onFocusIn = (event: FocusEvent): void => {
    const found = this.#cellFrom(event);
    if (found === undefined) return;
    this.#setActive(found.index, { focus: false, reveal: false });
    // **Keyboard focus emits a preview-request, and this is the line that makes live preview more
    // than a mouse feature.** Immediately, with no settle: an arrow key is a deliberate act per
    // item, and what protects the renderer is the outstanding-preview invariant rather than a delay.
    this.#requestPreview(found.index, this.#pointerDownRecently ? 'pointer' : 'keyboard');
  };

  #onFocusOut = (event: FocusEvent): void => {
    const next = event.relatedTarget;
    if (next instanceof Node && this.#root?.contains(next) === true) return;
    // Deferred and re-checked, for the reason `<mjx-menu>` records: `relatedTarget` is null for the
    // commonest movement inside a component that rebuilds its own DOM, and acting on it
    // synchronously cancels a preview the keyboard is still inside.
    queueMicrotask(() => {
      const active = deepActiveElement(this.ownerDocument);
      if (active !== null && this.#root?.contains(active) === true) return;
      if (active !== null && this.contains(active)) return;
      // ⚠ **A null active element is the component rebuilding under itself, not a person
      // leaving.** A virtualised surface replaces its own cells, and the frame in which the
      // focused cell has been removed and its replacement not yet focused is a frame in which
      // `document.activeElement` is the body. Treating that as *focus left the gallery* closed
      // the flyout in the same task it opened in — measured, and it is `<mjx-menu>`'s
      // deferred-focusout note arriving by a route a menu does not have, because a menu does not
      // rebuild its own rows.
      if (active === null) return;
      this.#emit(this.#session.cancel('keyboard'));
      if (this.expanded && !this.#dismissing) this.#collapse('blur');
    });
  };

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    const found = this.#cellFrom(event);
    const onAffordance = event.target === this.#expandButton;
    if (found === undefined && !onAffordance) return;

    const action = galleryKeyAction(event.key, {
      direction: getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr',
      expanded: this.expanded,
      altKey: event.altKey,
    });
    if (action === undefined) return;

    if (action === 'expand') {
      this.#expand('keyboard');
      event.preventDefault();
      return;
    }
    if (found === undefined) return;
    const surface = this.expanded ? this.#expanded : this.#strip;
    if (surface === undefined) return;

    if (isGalleryMovement(action)) {
      const next = nextGalleryIndex(action, this.#activeIndex, {
        count: this.#descriptors.length,
        columns: surface.columns,
        rowsPerPage: Math.max(1, Math.round(surface.viewport.clientHeight / Math.max(1, surface.rowPitch))),
      });
      this.#setActive(next, { focus: true, reveal: true });
    } else if (action === 'commit') {
      this.#commit(this.#activeIndex, 'keyboard');
    } else if (action === 'cancel') {
      // ⚠ **Escape reverts, and it reverts whether or not anything is open.** A gallery whose
      // Escape only closed a flyout would leave a preview applied to the document with nothing on
      // screen to say so.
      this.#emit(this.#session.cancel('keyboard'));
      if (this.expanded) this.#collapse('escape');
    }
    event.preventDefault();
    event.stopPropagation();
  };

  /** Move the roving tab stop, in **both** surfaces, and reveal it in whichever is showing. */
  #setActive(index: number, options: { focus: boolean; reveal: boolean }): void {
    if (index < 0 || index >= this.#descriptors.length) return;
    const changed = index !== this.#activeIndex;
    this.#activeIndex = index;
    for (const surface of this.#surfaces()) {
      if (changed) {
        const window_ = this.#windowFor(surface);
        if (window_.firstRow !== surface.built.firstRow || window_.lastRow !== surface.built.lastRow) {
          surface.built = window_;
          surface.signature = '';
          this.#buildRows(surface);
        }
      }
      this.#syncTabStops(surface);
      if (options.reveal) this.#reveal(surface, index);
    }
    if (options.focus) {
      const surface = this.expanded ? this.#expanded : this.#strip;
      surface?.cells.get(index)?.focus({ preventScroll: true });
    }
  }

  /** Scroll a surface so the item's row is inside its viewport. */
  #reveal(surface: Surface, index: number): void {
    const row = this.#rowOf(surface, index);
    if (row < 0) return;
    const top = surface.offsets[row] ?? 0;
    const bottom = surface.offsets[row + 1] ?? top;
    const view = surface.viewport;
    if (top < view.scrollTop) view.scrollTop = top;
    else if (bottom > view.scrollTop + view.clientHeight) {
      view.scrollTop = bottom - view.clientHeight;
    }
  }

  #onScroll = (event: Event): void => {
    const surface = this.#surfaces().find((candidate) => candidate.viewport === event.target);
    if (surface === undefined) return;
    const next = this.#windowFor(surface);
    if (next.firstRow !== surface.built.firstRow || next.lastRow !== surface.built.lastRow) {
      surface.built = next;
      surface.signature = '';
      this.#buildRows(surface);
    }
    if (surface.name === 'strip') this.#syncAffordances();
  };

  // ── the protocol ───────────────────────────────────────────────────────────

  #requestPreview(index: number, by: PreviewSource): void {
    const item = this.#descriptors[index];
    if (item === undefined) return;
    // An unavailable item never previews. Showing a person what a style they cannot apply would
    // look like is a promise the commit is going to break.
    if (item.unavailable) {
      this.#emit(this.#session.cancel(by));
      return;
    }
    this.#emit(this.#session.request({ value: item.value, label: item.label }, by));
  }

  #commit(index: number, by: PreviewSource): void {
    const item = this.#descriptors[index];
    if (item === undefined || item.unavailable) return;
    this.#clearPointerTimer();
    this.#emit(this.#session.commit({ value: item.value, label: item.label }, by));
    // The attribute follows the session rather than driving it, and `adopt` is a no-op on a value
    // that is already committed — so the reflection cannot emit a second event.
    if (this.getAttribute('value') !== item.value) this.setAttribute('value', item.value);
    else this.render();
    if (this.expanded) this.#collapse('activate');
  }

  /** Turn the machine's events into DOM events. One each, in order, and nothing invented here. */
  #emit(events: readonly PreviewEvent[]): void {
    for (const event of events) {
      const type =
        event.kind === 'preview'
          ? galleryEvents.preview
          : event.kind === 'cancel'
            ? galleryEvents.previewCancel
            : galleryEvents.commit;
      this.dispatchEvent(
        new CustomEvent(type, {
          bubbles: true,
          composed: true,
          detail: {
            value: event.value,
            label: event.label,
            by: event.by,
            restore: event.restore ?? null,
          },
        }),
      );
    }
    if (events.length > 0) this.#syncSelection();
  }

  /** Re-mark the selected cells without rebuilding them. */
  #syncSelection(): void {
    const committed = this.#session.committed;
    for (const surface of this.#surfaces()) {
      for (const [index, cell] of surface.cells) {
        const item = this.#descriptors[index];
        const selected = item !== undefined && item.value === committed;
        cell.setAttribute('aria-selected', selected ? 'true' : 'false');
        cell.dataset['selected'] = selected ? 'true' : 'false';
      }
    }
  }

  // ── expanding ──────────────────────────────────────────────────────────────

  #onAffordance = (event: Event): void => {
    const button = event.currentTarget;
    if (!(button instanceof HTMLElement)) return;
    const kind = button.dataset['affordance'];
    const surface = this.#strip;
    if (kind === 'expand') {
      if (this.expanded) this.#collapse('activate');
      else this.#expand('pointer');
      return;
    }
    if (surface === undefined) return;
    const step = surface.rowPitch + this.#gap(surface);
    surface.viewport.scrollTop += kind === 'scrollBack' ? -step : step;
  };

  /** Open the expanded surface. */
  #expand(by: PreviewSource): void {
    if (this.expanded) return;
    this.setAttribute('expanded', '');
    this.dispatchEvent(
      new CustomEvent(galleryEvents.expand, {
        bubbles: true,
        composed: true,
        detail: { expanded: true, by },
      }),
    );
  }

  /** Close it, and put focus back on the button that opened it. */
  #collapse(reason: 'escape' | 'activate' | 'outside' | 'blur'): void {
    if (!this.expanded) return;
    this.removeAttribute('expanded');
    this.dispatchEvent(
      new CustomEvent(galleryEvents.expand, {
        bubbles: true,
        composed: true,
        detail: { expanded: false, by: reason },
      }),
    );
    // `blur` deliberately does not restore: focus has already gone somewhere a person chose, and
    // yanking it back is worse than not restoring it at all. The same three reasons `<mjx-menu>`
    // restores for are the three that restore here.
    if (reason !== 'blur') this.#expandButton?.focus();
  }

  #showExpanded(): void {
    const surface = this.#expanded;
    if (surface === undefined) return;
    this.render();
    this.#place();

    const presentation = this.presentation;
    for (const name of galleryPresentationOrder) {
      surface.root.classList.remove(galleryMotionClass(name));
    }
    surface.root.classList.add(galleryMotionClass(presentation));
    surface.root.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete surface.root.dataset['entering'];
      // The second pass. The first placement measures a box the browser has only just been asked to
      // lay out, and the row pitch the plan was built from was a bootstrap estimate until now — so
      // both the plan and the placement are recomputed once the surface has a real size.
      this.#renderSurface(surface);
      this.#place();
      this.#reveal(surface, this.#activeIndex);
      surface.cells.get(this.#activeIndex)?.focus({ preventScroll: true });
    });

    this.#startWatching();
  }

  #teardownExpanded(): void {
    const surface = this.#expanded;
    this.#emit(this.#session.cancel('keyboard'));
    this.#stopWatching();
    if (surface !== undefined) {
      clearPlacement(surface.root);
      delete surface.root.dataset['entering'];
    }
    this.#placement = undefined;
    this.render();
  }

  /**
   * Put the expanded box in the top layer, and take it out again.
   *
   * `manual` rather than `auto`: dismissal and focus are this component's own, and a gallery's
   * flyout very often opens from inside a *collapsed ribbon group*, which is itself an overlay — an
   * `auto` popover would close its host the moment it opened.
   */
  #syncTopLayer(box: HTMLElement): void {
    if (typeof box.showPopover !== 'function') return;
    if (box.getAttribute('popover') !== 'manual') box.setAttribute('popover', 'manual');
    if (!this.isConnected) return;
    const showing = box.matches(':popover-open');
    if (this.expanded && !showing) box.showPopover();
    else if (!this.expanded && showing) box.hidePopover();
  }

  #place(): void {
    const surface = this.#expanded;
    const invoker = this.#expandButton;
    if (surface === undefined || invoker === undefined) return;

    const presentation = this.presentation;
    clearPlacement(surface.root);
    if (!galleryPresentations[presentation].anchored) {
      const box = clippingBoundary(surface.root, 0);
      this.#anchor = rectOf(invoker);
      this.#placement = pinFloating(surface.root, box, gallerySheetBoundaryFraction);
      return;
    }
    const anchor = rectOf(invoker);
    this.#anchor = anchor;
    const natural = surface.root.getBoundingClientRect();
    const inset = resolveLength(surface.root, floatingProperties.boundaryInset);
    const gap = resolveLength(surface.root, floatingProperties.gap);
    const placement = placeFloating({
      anchor,
      floating: { width: natural.width, height: natural.height },
      boundary: clippingBoundary(surface.root, inset),
      side: galleryFlyoutSide,
      align: galleryFlyoutAlign,
      direction: getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr',
      gap,
    });
    applyPlacement(surface.root, placement);
    this.#placement = placement;
  }

  /** Re-read the group, re-plan both surfaces and re-place the flyout. What a resize asks for. */
  reposition(): void {
    this.#readGroupPresentation();
    for (const surface of this.#surfaces()) this.#renderSurface(surface);
    if (this.expanded) this.#place();
    this.#syncAffordances();
  }

  /**
   * Watch what can change the plan — **always, not only while the flyout is open.**
   *
   * Three boxes, and each answers a different question the component cannot answer without being
   * told: **this element**, because its width is what CSS turns into a column count; **its parent**,
   * because a ribbon group changing presentation resizes its panel and republishes
   * `--mjx-group-presentation`, and a gallery that only re-read that on open would keep two rows in
   * a reduced group forever; and **the clipping ancestor**, because a flyout has to be re-placed
   * when the thing that clips it moves.
   *
   * Measured: the first version started this on expand, and the degradation story reported `full` at
   * every width — correctly, because nothing had asked it again.
   */
  #startObserving(): void {
    if (this.#observer !== undefined || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(() => {
      this.reposition();
    });
    observer.observe(this);
    const parent = this.parentElement;
    if (parent !== null) observer.observe(parent);
    const clipper = clippingAncestor(this);
    if (clipper !== undefined) observer.observe(clipper);
    this.#observer = observer;
  }

  #stopObserving(): void {
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  /** The dismissal listeners, which only mean anything while the flyout is showing. */
  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    this.ownerDocument.addEventListener('pointerdown', this.#onDocumentPointerDown, true);
  }

  #stopWatching(): void {
    if (!this.#watching) return;
    this.#watching = false;
    this.ownerDocument.removeEventListener('pointerdown', this.#onDocumentPointerDown, true);
  }

  #onDocumentPointerDown = (event: Event): void => {
    const path = event.composedPath();
    if (path.includes(this)) return;
    if (this.#expanded !== undefined && path.includes(this.#expanded.root)) return;
    this.#dismissing = true;
    this.#collapse('outside');
    // Queued behind the browser's own focus handling, for the reason `<mjx-menu>` records:
    // `pointerdown` runs before it, so focusing the invoker here would be undone a moment later.
    setTimeout(() => {
      this.#dismissing = false;
      this.#expandButton?.focus();
    }, 0);
  };
}

/** The cumulative block offset of every row, plus one past the last. */
function rowOffsets(surface: Surface): number[] {
  const gap = Number.parseFloat(getComputedStyle(surface.layer).rowGap);
  const spacing = Number.isFinite(gap) ? gap : 0;
  const offsets: number[] = [0];
  let total = 0;
  for (const row of surface.plan) {
    total += (row.kind === 'heading' ? surface.headingPitch : surface.rowPitch) + spacing;
    offsets.push(total);
  }
  return offsets;
}

/** The last row whose offset is at or before a scroll position. */
function rowIndexAt(offsets: readonly number[], position: number): number {
  if (offsets.length <= 1) return 0;
  let low = 0;
  let high = offsets.length - 2;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if ((offsets[middle] ?? 0) <= position) low = middle;
    else high = middle - 1;
  }
  return low;
}

/** The focused element, followed through every shadow root it is hiding in. */
function deepActiveElement(document_: Document): Element | null {
  let element: Element | null = document_.activeElement;
  while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
  if (element === document_.body || element === document_.documentElement) return null;
  return element;
}

/** Register the element. Idempotent. */
export function defineGallery(): void {
  defineIcon();
  if (customElements.get(galleryTags.gallery) === undefined) {
    customElements.define(galleryTags.gallery, MjxGallery);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-gallery': MjxGallery;
  }
}
