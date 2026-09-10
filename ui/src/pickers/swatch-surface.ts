/**
 * The colour grid a `<mjx-color-picker>` pops open — **a `listbox` of swatches, and still not a
 * menu.**
 *
 * U07 settled the distinction for a list of values and it holds here for a grid of them: a swatch
 * is a *value* one of which is chosen, so the pattern is `role="listbox"` / `role="option"` with
 * `aria-activedescendant` on the field, focus never reaches a cell, and the whole control is one
 * tab stop. That is what makes `Tab` free to mean *leave* and the popup a disclosure rather than a
 * trap — and it is why this satisfies `PopupSurface` rather than being a second kind of thing.
 *
 * ## Not a custom element, for U06's reason
 *
 * A plain class the picker constructs when the picker is ready. A popup that was its own element
 * would sometimes upgrade before the component that had to fill it, which is U06's third defect.
 *
 * ## Not virtualised, and that is a decision rather than an omission
 *
 * A theme block is sixty cells, a standard row is ten, and the recent row is single figures. The
 * whole purpose of a colour grid is that a person sees it at once; windowing it would be paying
 * virtualisation's complexity to hide the thing the control exists to show. The font picker next
 * door *is* virtualised, through U06's planner unchanged, and that is where the four-hundred-row
 * problem actually lives.
 *
 * What is still borrowed is the arithmetic that matters: `nextSwatchIndex` is U06's
 * `nextGalleryIndex` per section — including its ragged-tail clamp and its refusal to wrap — and
 * the placement is `overlay/floating.ts`, the same one a menu, a gallery flyout and a listbox use.
 */

import {
  applyPlacement,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingProperties,
  placeFloating,
  rectOf,
  resolveLength,
  syncTopLayer,
  type Direction,
  type Placement,
  type Rect,
} from '../overlay/floating.ts';
import { listAlign, listSide, type PopupSurface } from '../inputs/list-surface.ts';
import { inputBoxProperties, type OptionDescriptor } from '../inputs/input-model.ts';
import {
  pickerBoxProperties,
  pickerMotionClass,
  pickerTypeClass,
  swatchIndicatorProperty,
  swatchPaintProperty,
  swatchRowPlan,
  swatchSections,
  swatchSelectedGlyph,
  type SwatchSection,
} from './picker-model.ts';

/**
 * One swatch, as data.
 *
 * A superset of `OptionDescriptor`, so the whole field machinery keeps working unchanged: the
 * `value` is the **serialised choice** (`theme:accent1/lighter40`, `#123457`, `automatic`) and the
 * `label` is what a person is told it is. `paint` and `indicator` are computed by the picker from
 * the document's palette and are for drawing only — nothing downstream ever reads them.
 */
export interface SwatchDescriptor extends OptionDescriptor {
  /** The colour to fill the square with, or `undefined` for a choice that paints nothing. */
  readonly paint: string | undefined;
  /** The token member the ring and the check mark are drawn in, chosen against `paint`. */
  readonly indicator: string;
  /** A word drawn on the chip, for the entries that are not colours. */
  readonly chipLabel?: string;
}

/** Whether an option carries the extra two fields a swatch needs. */
export function isSwatchDescriptor(option: OptionDescriptor): option is SwatchDescriptor {
  return 'indicator' in option;
}

/** What the surface tells its owner. */
export interface SwatchSurfaceHost {
  chose(option: OptionDescriptor, index: number): void;
  movedActive(index: number): void;
}

interface BuiltCell {
  readonly index: number;
  readonly element: HTMLElement;
}

export class SwatchSurface implements PopupSurface {
  readonly #palette: HTMLElement;
  readonly #host: SwatchSurfaceHost;
  readonly #idPrefix: string;

  #options: readonly OptionDescriptor[] = [];
  #sections: SwatchSection[] = [];
  #selected: string | undefined;
  #active = -1;
  #built: BuiltCell[] = [];
  #open = false;
  #placement: Placement | undefined;
  #anchor: Rect | undefined;
  #natural: { width: number; height: number } | undefined;
  #observer: ResizeObserver | undefined;

  constructor(host: SwatchSurfaceHost, idPrefix: string) {
    this.#host = host;
    this.#idPrefix = idPrefix;

    const palette = document.createElement('div');
    palette.className = `palette ${pickerMotionClass}`;
    palette.setAttribute('part', 'palette');
    palette.setAttribute('role', 'listbox');
    palette.dataset['open'] = 'false';
    palette.addEventListener('pointerdown', this.#onPointerDown);
    palette.addEventListener('pointerover', this.#onPointerOver);
    this.#palette = palette;
  }

  get element(): HTMLElement {
    return this.#palette;
  }

  get open(): boolean {
    return this.#open;
  }

  get options(): readonly OptionDescriptor[] {
    return this.#options;
  }

  get activeIndex(): number {
    return this.#active;
  }

  get activeOption(): OptionDescriptor | undefined {
    return this.#active < 0 ? undefined : this.#options[this.#active];
  }

  get activeDescendantId(): string | undefined {
    if (this.#active < 0 || this.#active >= this.#options.length) return undefined;
    return this.#cellId(this.#active);
  }

  /** The sections the grid is currently made of — what `nextSwatchIndex` is asked about. */
  get sections(): readonly SwatchSection[] {
    return this.#sections;
  }

  /** How many cells exist right now. Equal to the option count: nothing here is windowed. */
  get builtCellCount(): number {
    return this.#built.length;
  }

  /** The placement the last open produced, for a gate to re-run `placeFloating` against. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /** The anchor the last placement was computed against. */
  get anchorRect(): Rect | undefined {
    return this.#anchor;
  }

  /** The natural size the last placement was computed for. */
  get naturalSize(): { readonly width: number; readonly height: number } | undefined {
    return this.#natural;
  }

  /** The boundary the last placement had to stay inside. */
  get boundaryRect(): Rect {
    return clippingBoundary(this.#palette, resolveLength(this.#palette, floatingProperties.boundaryInset));
  }

  #cellId(index: number): string {
    return `${this.#idPrefix}-swatch-${String(index)}`;
  }

  setOptions(options: readonly OptionDescriptor[]): void {
    this.#options = options;
    this.#sections = swatchSections(options);
    if (this.#active >= options.length) this.#active = options.length - 1;
    this.#render();
  }

  setSelected(value: string | undefined): void {
    this.#selected = value;
    this.#syncState();
  }

  setActive(index: number): void {
    const clamped = index < 0 ? -1 : Math.min(index, this.#options.length - 1);
    this.#active = clamped;
    this.#syncState();
    if (clamped >= 0) this.#scrollCellIntoView(clamped);
  }

  /**
   * ⚠ **Moving the cursor updates the cells that exist; it does not rebuild them.**
   *
   * This looks like an optimisation and is a correctness fix. Rebuilding put a *new* element under
   * a stationary mouse pointer, the browser fired `pointerover` on it, `#onPointerOver` did its
   * job — and the cursor snapped straight back to whichever cell the mouse happened to be resting
   * on. Every arrow key appeared to do nothing.
   *
   * It was found by the browser gate and it is exactly the kind of defect that is invisible in a
   * story: with the pointer anywhere else on the page the arrows work perfectly. `ListSurface` does
   * rebuild, and must, because its rows are a *window* that changes as the cursor moves; a grid's
   * cells are all of its cells, so there is nothing to re-window and no reason to touch the DOM.
   */
  #syncState(): void {
    for (const cell of this.#built) {
      const option = this.#options[cell.index];
      if (option === undefined) continue;
      cell.element.setAttribute('aria-selected', String(option.value === this.#selected));
      if (cell.index === this.#active) cell.element.dataset['active'] = '';
      else delete cell.element.dataset['active'];
    }
  }

  /**
   * Bring a cell into view.
   *
   * ⚠ The scroll happens **after** the render here and before it in `ListSurface`, and the
   * difference is the reason the two are not one function: a virtualised list has to scroll first
   * because its window is a function of the scroll offset, and a grid that builds every cell has
   * to render first because the element it means to scroll to may not exist yet. Getting either
   * one the other way round produces the same silent symptom — `aria-activedescendant` naming a
   * node that is not in the tree — so both say which they are.
   */
  #scrollCellIntoView(index: number): void {
    const cell = this.#built.find((entry) => entry.index === index)?.element;
    if (cell === undefined) return;
    const view = this.#palette.clientHeight;
    if (view <= 0) return;
    const top = cell.offsetTop;
    const height = cell.offsetHeight;
    if (top < this.#palette.scrollTop) this.#palette.scrollTop = top;
    else if (top + height > this.#palette.scrollTop + view) {
      this.#palette.scrollTop = top + height - view;
    }
  }

  #onPointerOver = (event: PointerEvent): void => {
    const cell = this.#cellIn(event.composedPath());
    if (cell === undefined || cell.index === this.#active) return;
    this.#active = cell.index;
    this.#render();
    this.#host.movedActive(cell.index);
  };

  #onPointerDown = (event: PointerEvent): void => {
    const cell = this.#cellIn(event.composedPath());
    if (cell === undefined) return;
    // `pointerdown` with `preventDefault`, for the reason `ListSurface` gives: a click would move
    // focus off the field first, the field's blur handler would close the popup, and the click
    // would land on nothing.
    event.preventDefault();
    const option = this.#options[cell.index];
    if (option === undefined || option.unavailable === true) return;
    this.#host.chose(option, cell.index);
  };

  #cellIn(path: readonly EventTarget[]): BuiltCell | undefined {
    for (const node of path) {
      if (node === this.#palette) return undefined;
      const found = this.#built.find((cell) => cell.element === node);
      if (found !== undefined) return found;
    }
    return undefined;
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  #render(): void {
    const palette = this.#palette;
    palette.replaceChildren();
    this.#built = [];

    const rows = swatchRowPlan(this.#sections);
    const built: BuiltCell[] = [];
    let grid: HTMLElement | undefined;
    let gridSection: SwatchSection | undefined;

    for (const row of rows) {
      if (row.kind === 'heading') {
        palette.append(this.#buildHeading(row.category));
        grid = undefined;
        continue;
      }
      const section = this.#sections.find(
        (candidate) => row.start >= candidate.start && row.start < candidate.end,
      );
      if (section === undefined) continue;
      if (grid === undefined || gridSection !== section) {
        grid = document.createElement('div');
        grid.className = 'section-grid';
        grid.style.setProperty(pickerBoxProperties.swatchColumns, String(section.columns));
        gridSection = section;
        palette.append(grid);
      }
      for (let index = row.start; index < row.end; index += 1) {
        const option = this.#options[index];
        if (option === undefined) continue;
        const cell = this.#buildCell(option, index);
        built.push({ index, element: cell });
        grid.append(cell);
      }
    }

    this.#built = built;
  }

  #buildHeading(category: string): HTMLElement {
    const heading = document.createElement('div');
    heading.className = `section-heading ${pickerTypeClass('swatchHeading')}`;
    // `presentation`, not `heading`: the sections are visual grouping inside one listbox, and a
    // heading role inside a listbox is a node a screen reader has to walk past on every arrow.
    heading.setAttribute('role', 'presentation');
    heading.textContent = category;
    return heading;
  }

  #buildCell(option: OptionDescriptor, index: number): HTMLElement {
    const cell = document.createElement('button');
    cell.type = 'button';
    cell.className = 'swatch';
    cell.setAttribute('part', 'swatch');
    cell.setAttribute('role', 'option');
    cell.id = this.#cellId(index);
    // ⚠ Not focusable. Focus stays on the field and `aria-activedescendant` names this row, which
    // is what gives the whole control one tab stop. A `<button>` is focusable by default, so the
    // negative index is load-bearing rather than defensive.
    cell.tabIndex = -1;
    cell.setAttribute('aria-selected', String(option.value === this.#selected));
    cell.setAttribute('aria-posinset', String(index + 1));
    cell.setAttribute('aria-setsize', String(this.#options.length));
    // The whole announcement, on the row: a swatch has no text of its own, so a name assembled by
    // a screen reader from its contents would be silence.
    cell.setAttribute('aria-label', option.label);
    cell.title = option.label;
    if (index === this.#active) cell.dataset['active'] = '';
    if (option.unavailable === true) {
      cell.setAttribute('aria-disabled', 'true');
      if (option.explanation !== undefined && option.explanation !== '') {
        cell.title = `${option.label} — ${option.explanation}`;
        cell.setAttribute('aria-label', `${option.label}, ${option.explanation}`);
      }
    }

    const paint = document.createElement('span');
    paint.className = 'swatch-paint';
    paint.setAttribute('aria-hidden', 'true');

    // The check mark goes **inside** the coloured square rather than over the whole cell, so one
    // rule centres it in a square-only swatch and in a labelled chip alike — and so it is never
    // drawn over the chip's word.
    const mark = document.createElement('mjx-icon');
    mark.className = 'swatch-mark';
    mark.setAttribute('name', swatchSelectedGlyph.name);
    mark.setAttribute('size', String(swatchSelectedGlyph.size));
    mark.setAttribute('aria-hidden', 'true');
    paint.append(mark);
    cell.append(paint);

    if (isSwatchDescriptor(option)) {
      cell.style.setProperty(swatchIndicatorProperty, option.indicator);
      if (option.paint === undefined) paint.dataset['empty'] = '';
      else paint.style.setProperty(swatchPaintProperty, option.paint);
      if (option.chipLabel !== undefined && option.chipLabel !== '') {
        const label = document.createElement('span');
        label.className = `swatch-label ${pickerTypeClass('fieldValue')}`;
        // Not aria-hidden: it is the chip's visible name and duplicates the row's own label, which
        // a screen reader reads from aria-label. Hiding it would have been the tidier-looking
        // choice and would have made the row's text invisible to a rule that checks visible text.
        label.textContent = option.chipLabel;
        cell.dataset['chip'] = '';
        cell.append(label);
      }
    } else {
      paint.dataset['empty'] = '';
    }

    return cell;
  }

  // ── opening, placing and closing ───────────────────────────────────────────

  show(anchor: HTMLElement, direction: Direction): void {
    if (this.#open) {
      this.place(anchor, direction);
      return;
    }
    this.#open = true;
    this.#palette.dataset['open'] = 'true';
    this.#syncTopLayer();
    this.#render();
    this.place(anchor, direction);
    // The second pass a frame later, for the reason `<mjx-menu>` gives: the first placement
    // measures a box the browser has only just been asked to lay out.
    requestAnimationFrame(() => {
      if (this.#open) this.place(anchor, direction);
    });
    if (typeof ResizeObserver !== 'undefined') {
      this.#observer = new ResizeObserver(() => {
        if (this.#open) this.place(anchor, direction);
      });
      this.#observer.observe(clippingAncestor(this.#palette) ?? this.#palette.ownerDocument.documentElement);
    }
  }

  place(anchor: HTMLElement, direction: Direction): void {
    const palette = this.#palette;
    clearPlacement(palette);
    palette.style.removeProperty(inputBoxProperties.listMaxBlockSize);

    const anchorRect = rectOf(anchor);
    this.#anchor = anchorRect;

    const measured = palette.getBoundingClientRect();
    this.#natural = { width: measured.width, height: measured.height };

    const inset = resolveLength(palette, floatingProperties.boundaryInset);
    const gap = resolveLength(palette, floatingProperties.gap);
    const placement = placeFloating({
      anchor: anchorRect,
      floating: { width: measured.width, height: measured.height },
      boundary: clippingBoundary(palette, inset),
      side: listSide,
      align: listAlign,
      direction,
      gap,
    });
    applyPlacement(palette, placement);
    this.#placement = placement;
  }

  hide(): void {
    if (!this.#open) return;
    this.#open = false;
    this.#palette.dataset['open'] = 'false';
    this.#syncTopLayer();
    clearPlacement(this.#palette);
    this.#placement = undefined;
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  dispose(): void {
    this.hide();
  }

  /**
   * The **top layer** while it is open.
   *
   * A field inside a scrolling task pane clips its own popup — MJXOFF-184 measured the failure: a
   * correct rectangle, no pixels and no pointer. `manual` rather than `auto`, because dismissal
   * and focus belong to the owner and an auto popover would light-dismiss out from under it.
   */
  #syncTopLayer(): void {
    // ⚠ The dance moved into `overlay/floating.ts` in MJXOFF-188. Conditions unchanged.
    syncTopLayer(this.#palette, {
      inTopLayer: true,
      open: this.#open,
      connected: this.#palette.isConnected,
    });
  }
}
