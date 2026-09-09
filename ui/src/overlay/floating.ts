/**
 * **The minimal floating-layer primitive** — placement geometry, the clipping boundary, and the
 * one call that writes a placement onto an element.
 *
 * MJXOFF-184 asks for exactly this and says what to do with it:
 *
 * > The surface family lands in **U09**. **If a menu needs a generic floating-layer primitive,
 * > define it here minimally and hand it forward** — and say so in the report, so U09 extends it
 * > rather than writing a second one.
 *
 * So nothing here knows what a menu is. A dialog, a popover, a tooltip and a task pane are the
 * same three questions — *which side, shifted how far, and clipped by what* — and this answers
 * them for an anchor rectangle and a size. **U09 extends this file; it does not write a second
 * placement.**
 *
 * ## The boundary is the nearest declared viewport, not the window
 *
 * MJXOFF-183 left a gap and named it: its collapsed group's popup *"can be clipped by the harness
 * frame's `overflow: auto` at narrow widths, with no gate watching."* Closing it needs a rule about
 * what a floating box must stay inside, and there are two answers that both matter:
 *
 * * **What genuinely clips it.** A `position: fixed` box is clipped by an ancestor only when that
 *   ancestor is *both* a clipper and its containing block — a transform, a filter, or paint or
 *   layout containment. `.menu` scrolls its own list and is neither, which is why a submenu is not
 *   clamped inside its parent menu.
 * * **What declares itself a viewport.** A **container query container** says *the things inside me
 *   size themselves against me*, and this catalogue's whole responsive doctrine rests on it:
 *   `<mjx-resizable-container>`'s `.frame` is the simulated screen, and a menu that hung outside it
 *   would be reporting a responsive behaviour the container never had.
 *
 * `clippingBoundary()` therefore walks the **flat tree** — through slots and shadow hosts, which a
 * `parentNode` walk misses entirely, because the frame is in a shadow root and the story content is
 * slotted into it — and intersects the viewport with every ancestor that clips *and* declares one
 * of those two things. `.frame` satisfies both conditions and `.menu` satisfies neither, which is
 * the split that makes edge-flipping work and submenus still open outward.
 *
 * ⚠ **Measured, and worth writing down:** Chromium does **not** make a `container-type` element a
 * containing block for its fixed-position descendants, whatever CSS Containment's layout-containment
 * paragraph reads like. A sheet pinned with `inset: auto 0 0 0` inside the harness frame spans the
 * *window*, not the frame — which is why the sheet is pinned to the boundary in JavaScript rather
 * than by four zeroes in a stylesheet, and why `applyPlacement`'s correction below is a no-op here
 * and still required for a shell whose ancestor carries a transform.
 *
 * ## Coordinates are physical and viewport-relative; the *preference* is logical
 *
 * A caller says `inlineEnd`, and `physicalSide()` turns that into `right` or `left` from the
 * writing direction. Everything after that is arithmetic in viewport pixels, because a placement
 * that mixed logical and physical axes is a placement nobody can debug. RTL is therefore one
 * function call rather than a second code path, and `tests/menus.test.ts` asserts the mirror
 * property directly.
 *
 * ## The containing block is whatever the DOM gave us, and we correct for it
 *
 * `applyPlacement` writes viewport coordinates and then **measures what the browser actually
 * did** and corrects by the difference. That is not a hack around a bug: a fixed element's
 * containing block is the viewport *unless* an ancestor establishes one (a transform, a filter, or
 * layout containment — `container-type` again), and a component cannot know which. Asking for
 * viewport coordinates and correcting once by the observed offset is exact in both cases and costs
 * one forced reflow at open time.
 *
 * ## Node-importable
 *
 * No custom element is defined here, so the Playwright specs — which run in Node — can import the
 * geometry and compare it against what a browser did. That is the rule
 * `src/harness/presets.ts` established, applied to a module that also has DOM functions in it:
 * the DOM is touched inside function bodies, never at module scope.
 */

import { spacingMultiple } from '../foundations/density.ts';

/** A rectangle in viewport coordinates. */
export interface Rect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** The writing direction a placement is resolved in. */
export type Direction = 'ltr' | 'rtl';

/** The side a caller asks for, in logical terms. */
export type LogicalSide = 'blockEnd' | 'blockStart' | 'inlineEnd' | 'inlineStart';

/** The side a placement resolves to, in physical terms. */
export type PhysicalSide = 'bottom' | 'top' | 'right' | 'left';

/** How the floating box lines up with the anchor on the cross axis. */
export type Align = 'start' | 'end' | 'center';

/** The four logical sides, in the order the catalogue lists them. */
export const logicalSides: readonly LogicalSide[] = [
  'blockEnd',
  'blockStart',
  'inlineEnd',
  'inlineStart',
];

/** The four physical sides. */
export const physicalSides: readonly PhysicalSide[] = ['bottom', 'top', 'right', 'left'];

/** The three alignments. */
export const alignments: readonly Align[] = ['start', 'end', 'center'];

/** Whether a value names a logical side. */
export function isLogicalSide(value: unknown): value is LogicalSide {
  return (logicalSides as readonly string[]).includes(String(value));
}

/** Whether a value names an alignment. */
export function isAlign(value: unknown): value is Align {
  return (alignments as readonly string[]).includes(String(value));
}

/**
 * A logical side resolved against a writing direction.
 *
 * The block axis does not mirror — a menu below its button is below it in Arabic too — so only the
 * inline pair depends on `direction`. That asymmetry is the whole of this project's RTL story, and
 * having it in one three-line function is what stops it being re-derived, slightly differently, in
 * every component that floats.
 */
export function physicalSide(side: LogicalSide, direction: Direction): PhysicalSide {
  switch (side) {
    case 'blockEnd':
      return 'bottom';
    case 'blockStart':
      return 'top';
    case 'inlineEnd':
      return direction === 'rtl' ? 'left' : 'right';
    case 'inlineStart':
      return direction === 'rtl' ? 'right' : 'left';
  }
}

/** `bottom` → `top`, `right` → `left`. The side a flip lands on. */
export function oppositeSide(side: PhysicalSide): PhysicalSide {
  switch (side) {
    case 'bottom':
      return 'top';
    case 'top':
      return 'bottom';
    case 'right':
      return 'left';
    case 'left':
      return 'right';
  }
}

/** Whether a physical side puts the floating box above or below the anchor. */
export function isBlockSide(side: PhysicalSide): boolean {
  return side === 'top' || side === 'bottom';
}

/** Everything `placeFloating` needs, and nothing about what is being placed. */
export interface PlacementRequest {
  /** The thing being placed against, in viewport coordinates. A point is a zero-sized rectangle. */
  readonly anchor: Rect;
  /** The floating box's natural size. */
  readonly floating: { readonly width: number; readonly height: number };
  /** What it must stay inside — `clippingBoundary()` in a browser, a literal rect in a test. */
  readonly boundary: Rect;
  /** The side the caller prefers, logically. */
  readonly side: LogicalSide;
  /** How it lines up on the cross axis. */
  readonly align: Align;
  /** The writing direction. */
  readonly direction: Direction;
  /** The space between the anchor and the floating box. */
  readonly gap: number;
}

/** Where the floating box goes, and what had to be done to it to get it there. */
export interface Placement {
  /** Viewport coordinates for the floating box's top-left corner. */
  readonly x: number;
  readonly y: number;
  /** The side it actually ended up on. */
  readonly side: PhysicalSide;
  /** The alignment it was asked for; recorded so a gate can say what it compared against. */
  readonly align: Align;
  /** True when the preferred side did not fit and the opposite one was taken. */
  readonly flipped: boolean;
  /** True when the cross axis had to move to keep the box inside the boundary. */
  readonly shifted: boolean;
  /**
   * True when the box is larger than the room on its main axis even after flipping.
   *
   * Not a failure: it is what a menu with forty items does on a short screen, and the answer is to
   * scroll rather than to be cut off. `maxBlockSize` is the room it has.
   */
  readonly constrained: boolean;
  /** The room available on the chosen side, so the caller can cap and scroll instead of clipping. */
  readonly maxBlockSize: number;
  readonly maxInlineSize: number;
}

function clamp(value: number, low: number, high: number): number {
  if (high < low) return low;
  return Math.min(Math.max(value, low), high);
}

/**
 * Place a box against an anchor, inside a boundary. **Pure arithmetic, no DOM.**
 *
 * The order is the one every floating implementation converges on and it is worth stating, because
 * doing it in the other order produces a box that flips when it only needed to shift:
 *
 * 1. **Flip** — if the preferred side has less room than the box needs *and* the opposite side has
 *    more room than the preferred one, take the opposite side. The second condition matters: on a
 *    screen too short for the menu either way, flipping achieves nothing and moves the menu away
 *    from the thing it belongs to.
 * 2. **Shift** — move along the cross axis until the box is inside the boundary.
 * 3. **Constrain** — report the room on the main axis so the caller can scroll rather than overflow.
 */
export function placeFloating(request: PlacementRequest): Placement {
  const { anchor, floating, boundary, align, direction, gap } = request;

  const boundaryRight = boundary.x + boundary.width;
  const boundaryBottom = boundary.y + boundary.height;
  const anchorRight = anchor.x + anchor.width;
  const anchorBottom = anchor.y + anchor.height;

  const room: Readonly<Record<PhysicalSide, number>> = {
    top: anchor.y - boundary.y - gap,
    bottom: boundaryBottom - anchorBottom - gap,
    left: anchor.x - boundary.x - gap,
    right: boundaryRight - anchorRight - gap,
  };

  const preferred = physicalSide(request.side, direction);
  const opposite = oppositeSide(preferred);
  const needed = isBlockSide(preferred) ? floating.height : floating.width;
  const flipped = room[preferred] < needed && room[opposite] > room[preferred];
  const side = flipped ? opposite : preferred;
  const available = room[side];

  let x: number;
  let y: number;
  let rawCross: number;
  let low: number;
  let high: number;

  if (isBlockSide(side)) {
    y = side === 'bottom' ? anchorBottom + gap : anchor.y - gap - floating.height;
    // The cross axis is inline, and it is the only axis the writing direction touches: `start`
    // means the anchor's leading edge, which is its right edge in RTL.
    const leading = direction === 'rtl' ? anchorRight - floating.width : anchor.x;
    const trailing = direction === 'rtl' ? anchor.x : anchorRight - floating.width;
    rawCross =
      align === 'center'
        ? anchor.x + (anchor.width - floating.width) / 2
        : align === 'start'
          ? leading
          : trailing;
    low = boundary.x;
    high = boundaryRight - floating.width;
    x = clamp(rawCross, low, high);
    y = clamp(y, boundary.y, Math.max(boundary.y, boundaryBottom - floating.height));
  } else {
    x = side === 'right' ? anchorRight + gap : anchor.x - gap - floating.width;
    rawCross =
      align === 'center'
        ? anchor.y + (anchor.height - floating.height) / 2
        : align === 'start'
          ? anchor.y
          : anchorBottom - floating.height;
    low = boundary.y;
    high = boundaryBottom - floating.height;
    y = clamp(rawCross, low, high);
    x = clamp(x, boundary.x, Math.max(boundary.x, boundaryRight - floating.width));
  }

  const cross = isBlockSide(side) ? x : y;
  return {
    x,
    y,
    side,
    align,
    flipped,
    shifted: cross !== rawCross,
    constrained: available < needed,
    maxBlockSize: isBlockSide(side) ? Math.max(available, 0) : boundary.height,
    maxInlineSize: isBlockSide(side) ? boundary.width : Math.max(available, 0),
  };
}

// ── the DOM half ─────────────────────────────────────────────────────────────

/** A `DOMRect` as this module's plain shape, so a placement can be compared in Node. */
export function rectOf(element: Element): Rect {
  const box = element.getBoundingClientRect();
  return { x: box.left, y: box.top, width: box.width, height: box.height };
}

/** The visual viewport, as a rectangle. */
export function viewportRect(): Rect {
  return {
    x: 0,
    y: 0,
    width: document.documentElement.clientWidth,
    height: document.documentElement.clientHeight,
  };
}

/** The overlapping part of two rectangles; zero-sized when they do not overlap. */
export function intersectRects(first: Rect, second: Rect): Rect {
  const x = Math.max(first.x, second.x);
  const y = Math.max(first.y, second.y);
  const right = Math.min(first.x + first.width, second.x + second.width);
  const bottom = Math.min(first.y + first.height, second.y + second.height);
  return { x, y, width: Math.max(right - x, 0), height: Math.max(bottom - y, 0) };
}

/** A rectangle pulled in on every side. A negative result is clamped to zero size. */
export function insetRect(rect: Rect, inset: number): Rect {
  return {
    x: rect.x + inset,
    y: rect.y + inset,
    width: Math.max(rect.width - inset * 2, 0),
    height: Math.max(rect.height - inset * 2, 0),
  };
}

/**
 * The flat-tree parent: through an assigned slot, and out of a shadow root by its host.
 *
 * A `parentNode` walk is the obvious thing to write and it is wrong here in a way that is silent:
 * the story content is *slotted* into `<mjx-resizable-container>`'s `.frame`, so the element that
 * actually clips it is never on the `parentNode` chain at all, and a boundary walk that used one
 * would return the viewport and place every menu half outside its frame.
 */
function flatTreeParent(node: Node): Node | null {
  if (node instanceof Element) {
    const slot = node.assignedSlot;
    if (slot !== null) return slot;
  }
  const parent: Node | null = node.parentNode;
  if (parent instanceof ShadowRoot) return parent.host;
  return parent;
}

/** An element's padding box — the rectangle its `overflow` actually clips to. */
function paddingBox(element: HTMLElement): Rect {
  const box = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  const top = Number.parseFloat(style.borderTopWidth) || 0;
  const right = Number.parseFloat(style.borderRightWidth) || 0;
  const bottom = Number.parseFloat(style.borderBottomWidth) || 0;
  const left = Number.parseFloat(style.borderLeftWidth) || 0;
  return {
    x: box.left + left,
    y: box.top + top,
    width: Math.max(box.width - left - right, 0),
    height: Math.max(box.height - top - bottom, 0),
  };
}

/** Whether an element clips what overflows it. */
function clips(style: CSSStyleDeclaration): boolean {
  if (style.contain.includes('paint') || style.contain.includes('strict')) return true;
  return style.overflowX !== 'visible' || style.overflowY !== 'visible';
}

/**
 * Whether an element is a containing block for its `position: fixed` descendants.
 *
 * **This is the condition that decides whether a clipping ancestor matters at all**, and getting
 * it wrong in either direction is a visible defect:
 *
 * * A fixed box is *not* clipped by an ancestor's `overflow` unless that ancestor is also its
 *   containing block. A menu's own `.menu` scrolls, so a boundary walk that intersected with every
 *   scroller would clamp every **submenu** inside its parent menu's box — a submenu one item tall.
 * * A fixed box *is* clipped when the ancestor is both. `<mjx-resizable-container>`'s `.frame`
 *   carries `container-type: inline-size`, which applies layout containment and therefore makes it
 *   a containing block, **and** `overflow: auto`. That is MJXOFF-183's unwatched clipping, and it
 *   is why this function names `containerType` rather than only `transform`.
 */
function establishesFixedContainingBlock(style: CSSStyleDeclaration): boolean {
  if (style.transform !== 'none' || style.perspective !== 'none') return true;
  if (style.filter !== 'none') return true;
  if (style.backdropFilter !== undefined && style.backdropFilter !== 'none') return true;
  if (/\b(?:layout|paint|strict|content)\b/.test(style.contain)) return true;
  if (style.containerType !== '' && style.containerType !== 'normal') return true;
  return /\b(?:transform|perspective|filter|contain)\b/.test(style.willChange);
}

/**
 * The rectangle a **fixed-position** floating box must stay inside: the viewport, intersected with
 * every flat-tree ancestor that both clips and is a containing block for it.
 *
 * `inset` is pulled off every edge, so a menu never sits flush against the boundary it was
 * flipped to avoid.
 */
/**
 * The nearest ancestor that would clip a fixed-position box, or `undefined` for the viewport.
 *
 * A floating box has to be re-placed when the thing that clips it changes size, and this is the
 * element to observe. In this catalogue it is `<mjx-resizable-container>`'s `.frame`, which is
 * exactly right: driving the harness's width must move a menu, and it must do so **without a
 * `resize` listener on the window**, because the window is not what changed.
 */
export function clippingAncestor(element: Element): HTMLElement | undefined {
  let node: Node | null = flatTreeParent(element);
  while (node !== null) {
    if (node instanceof HTMLElement) {
      const style = getComputedStyle(node);
      if (clips(style) && establishesFixedContainingBlock(style)) return node;
    }
    node = flatTreeParent(node);
  }
  return undefined;
}

export function clippingBoundary(element: Element, inset = 0): Rect {
  let rect = viewportRect();
  let node: Node | null = flatTreeParent(element);
  while (node !== null) {
    if (node instanceof HTMLElement) {
      const style = getComputedStyle(node);
      if (clips(style) && establishesFixedContainingBlock(style)) {
        rect = intersectRects(rect, paddingBox(node));
      }
    }
    node = flatTreeParent(node);
  }
  return inset === 0 ? rect : insetRect(rect, inset);
}

/**
 * The custom properties a floating element reads.
 *
 * The two lengths are declared with `@property` and a `<length>` syntax, which is what makes
 * `getComputedStyle().getPropertyValue()` return **pixels** rather than the un-substituted token
 * text. Without the registration, `--mjx-floating-gap: var(--spacing)` computes to the string
 * `0.25rem` and a component that parsed it would place every menu a quarter of a pixel away from
 * its button. `resolveLength` below returns `0` when the registration is missing rather than a
 * nonsense number, and `tests/browser/menus.spec.ts` asserts the gap is greater than zero — so a
 * browser without `@property` fails the gate instead of quietly placing menus flush.
 */
export const floatingProperties = {
  /** The distance between the anchor and the floating box. */
  gap: '--mjx-floating-gap',
  /** How far a floating box is kept off the boundary's edges. */
  boundaryInset: '--mjx-floating-boundary-inset',
  /** Written by `applyPlacement`. */
  x: '--mjx-floating-x',
  y: '--mjx-floating-y',
  /** The room on the chosen side, so a caller scrolls rather than overflows. */
  maxBlockSize: '--mjx-floating-max-block-size',
  maxInlineSize: '--mjx-floating-max-inline-size',
  /** Written only by `pinFloating`: a pinned box is as wide as the boundary it is pinned to. */
  inlineSize: '--mjx-floating-inline-size',
} as const;

/** The gap and the inset, in spacing units, so a re-seed of `--spacing` moves both. */
export const floatingMetricUnits = { gap: 1, boundaryInset: 2 } as const;

/**
 * The registrations and the defaults.
 *
 * `@property` is document-scoped wherever the sheet carrying it is applied, so a component that
 * floats installs this on **its own shadow root and on the document**, exactly as
 * `ui/README.md` prescribes for a component that also puts foundation classes on its host.
 */
export const floatingCss = `
@property ${floatingProperties.gap} {
  syntax: '<length>';
  inherits: true;
  initial-value: 0px;
}
@property ${floatingProperties.boundaryInset} {
  syntax: '<length>';
  inherits: true;
  initial-value: 0px;
}
@property ${floatingProperties.x} {
  syntax: '<length>';
  inherits: false;
  initial-value: 0px;
}
@property ${floatingProperties.y} {
  syntax: '<length>';
  inherits: false;
  initial-value: 0px;
}

:where(:root, .mjx-foundations, .mjx-floating) {
  ${floatingProperties.gap}: ${spacingMultiple(floatingMetricUnits.gap)};
  ${floatingProperties.boundaryInset}: ${spacingMultiple(floatingMetricUnits.boundaryInset)};
}
`;

/**
 * One registered length, in pixels, or `0` when it did not resolve to one.
 *
 * `Number.parseFloat` on an unregistered custom property returns the leading number of whatever
 * text is there — `0.25` for `0.25rem` — which is why the unit is checked rather than assumed.
 */
export function resolveLength(element: Element, property: string): number {
  const raw = getComputedStyle(element).getPropertyValue(property).trim();
  if (!raw.endsWith('px')) return 0;
  const value = Number.parseFloat(raw);
  return Number.isFinite(value) ? value : 0;
}

/**
 * Write a placement onto an element, and correct for whatever containing block it was given.
 *
 * The correction is one measure and one write. See the module note: a fixed element's containing
 * block is the viewport unless an ancestor establishes one, `container-type: inline-size` does
 * establish one, and this catalogue's harness frame carries exactly that — so the uncorrected
 * coordinates would be wrong by the frame's offset in every story.
 */
export function applyPlacement(element: HTMLElement, placement: Placement): void {
  const write = (x: number, y: number): void => {
    element.style.setProperty(floatingProperties.x, `${String(x)}px`);
    element.style.setProperty(floatingProperties.y, `${String(y)}px`);
  };
  element.style.setProperty(
    floatingProperties.maxBlockSize,
    `${String(Math.round(placement.maxBlockSize))}px`,
  );
  element.style.setProperty(
    floatingProperties.maxInlineSize,
    `${String(Math.round(placement.maxInlineSize))}px`,
  );
  write(placement.x, placement.y);

  const actual = element.getBoundingClientRect();
  const dx = placement.x - actual.left;
  const dy = placement.y - actual.top;
  if (dx !== 0 || dy !== 0) write(placement.x + dx, placement.y + dy);
}

/** Remove every coordinate this module wrote, so the stylesheet is back in charge. */
export function clearPlacement(element: HTMLElement): void {
  for (const property of [
    floatingProperties.x,
    floatingProperties.y,
    floatingProperties.maxBlockSize,
    floatingProperties.maxInlineSize,
    floatingProperties.inlineSize,
  ]) {
    element.style.removeProperty(property);
  }
}

/**
 * Pin a box to one edge of its boundary, at the boundary's full width — what a sheet is.
 *
 * **Not `inset: auto 0 0 0`, and the difference is measured rather than assumed.** A fixed box's
 * percentages and zeroes resolve against its containing block, and Chromium does not make a
 * `container-type` element one — so four zeroes inside the harness frame produce a sheet the width
 * of the *window*. Pinning against the same rectangle `placeFloating` flips against keeps the two
 * presentations answering to one boundary, which is what makes a container resize across the
 * threshold move the menu from one to the other and not from one to something else.
 *
 * `blockFraction` is how much of the boundary a sheet may cover before it scrolls.
 */
export function pinFloating(
  element: HTMLElement,
  boundary: Rect,
  blockFraction: number,
): Placement {
  const maxBlockSize = Math.round(boundary.height * blockFraction);
  element.style.setProperty(floatingProperties.inlineSize, `${String(Math.round(boundary.width))}px`);
  element.style.setProperty(floatingProperties.maxBlockSize, `${String(maxBlockSize)}px`);
  const height = Math.min(element.getBoundingClientRect().height, maxBlockSize);
  const placement: Placement = {
    x: boundary.x,
    y: boundary.y + boundary.height - height,
    side: 'bottom',
    align: 'start',
    flipped: false,
    shifted: false,
    constrained: height >= maxBlockSize,
    maxBlockSize,
    maxInlineSize: boundary.width,
  };
  applyPlacement(element, placement);
  // `applyPlacement` rewrites the caps from the placement; the width is this function's own.
  element.style.setProperty(floatingProperties.inlineSize, `${String(Math.round(boundary.width))}px`);
  return placement;
}

/**
 * Whether a pointer is travelling **toward** a rectangle rather than merely leaving where it was.
 *
 * This is the *safe triangle*, and MJXOFF-184 names its absence as the classic menu defect:
 *
 * > **with the diagonal-travel tolerance** that lets a pointer move toward a submenu without
 * > passing over a sibling and closing it. Its absence is felt immediately.
 *
 * The triangle's apex is where the pointer *was*, and its base is the near edge of the target,
 * grown by `grace` at both ends so a pointer aimed at the submenu's first or last item is still
 * inside it. A pointer inside that triangle is on its way; a pointer outside it has gone somewhere
 * else, and the sibling it is now over is the one it means.
 *
 * It is pure, and it is here rather than in the menu, because it is a property of two rectangles
 * and a pointer — U09's task pane will want it for exactly the same reason.
 */
export function travellingToward(
  pointer: { readonly x: number; readonly y: number },
  previous: { readonly x: number; readonly y: number },
  target: Rect,
  side: PhysicalSide,
  grace: number,
): boolean {
  if (target.width <= 0 || target.height <= 0) return false;
  if (pointer.x === previous.x && pointer.y === previous.y) return false;

  const near =
    side === 'right'
      ? { start: { x: target.x, y: target.y - grace }, end: { x: target.x, y: target.y + target.height + grace } }
      : side === 'left'
        ? {
            start: { x: target.x + target.width, y: target.y - grace },
            end: { x: target.x + target.width, y: target.y + target.height + grace },
          }
        : side === 'bottom'
          ? {
              start: { x: target.x - grace, y: target.y },
              end: { x: target.x + target.width + grace, y: target.y },
            }
          : {
              start: { x: target.x - grace, y: target.y + target.height },
              end: { x: target.x + target.width + grace, y: target.y + target.height },
            };

  // Moving away from the target's near edge is never travel toward it, whatever the triangle says.
  const towards =
    side === 'right'
      ? pointer.x >= previous.x
      : side === 'left'
        ? pointer.x <= previous.x
        : side === 'bottom'
          ? pointer.y >= previous.y
          : pointer.y <= previous.y;
  if (!towards) return false;

  return pointInTriangle(pointer, previous, near.start, near.end);
}

function sign(
  point: { readonly x: number; readonly y: number },
  first: { readonly x: number; readonly y: number },
  second: { readonly x: number; readonly y: number },
): number {
  return (point.x - second.x) * (first.y - second.y) - (first.x - second.x) * (point.y - second.y);
}

/** Barycentric sign test. Exported because a geometry helper nobody can test is a guess. */
export function pointInTriangle(
  point: { readonly x: number; readonly y: number },
  a: { readonly x: number; readonly y: number },
  b: { readonly x: number; readonly y: number },
  c: { readonly x: number; readonly y: number },
): boolean {
  const first = sign(point, a, b);
  const second = sign(point, b, c);
  const third = sign(point, c, a);
  const negative = first < 0 || second < 0 || third < 0;
  const positive = first > 0 || second > 0 || third > 0;
  return !(negative && positive);
}
