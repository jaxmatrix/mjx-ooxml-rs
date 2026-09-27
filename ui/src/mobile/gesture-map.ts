/**
 * **The gesture map** — which gesture belongs to whom, where, and what happens when that is
 * ambiguous.
 *
 * MJXOFF-194's constraint, and it is a hard one:
 *
 * > No gesture may conflict with the canvas's pan, pinch or selection gestures. **The canvas wins
 * > any ambiguity**, because the document is the product.
 *
 * ## Why this is a table and not a set of event handlers
 *
 * A gesture conflict is not a bug that appears in one component; it appears in the *seam* between
 * the chrome and the canvas, and neither side can see it alone. A sheet that takes a two-finger
 * drag is a sheet that has stolen the document's zoom, and nothing in the sheet's own tests can
 * possibly notice. So the claims are declared centrally, the resolution rule is one function, and
 * the browser gate compares the declared claims against what the chrome's CSS actually does.
 *
 * ## The CSS half, which is the half that actually enforces it
 *
 * A claim is not made by adding a listener; it is made by writing `touch-action`. A chrome element
 * that leaves `touch-action` alone lets every gesture through to the document beneath, and one that
 * writes `touch-action: none` has taken *all* of them — pinch included. So each region declares the
 * `touch-action` it writes, `gesturesSuppressedBy` derives which gestures that removes from the
 * canvas, and `tests/mobile.test.ts` requires the derivation to equal the region's declared claims
 * exactly. **A region that claims one gesture and writes a `touch-action` taking three fails.**
 *
 * ## Node-importable
 *
 * Data and pure functions.
 */

/** The gestures this platform has an opinion about. */
export const gestureNames = [
  'tap',
  'doubleTap',
  'longPress',
  'panBlock',
  'panInline',
  'twoFingerPan',
  'pinch',
] as const;

/** One of the seven. */
export type Gesture = (typeof gestureNames)[number];

/**
 * The gestures the canvas keeps **everywhere**, on every surface, without exception.
 *
 * Both are multi-touch, and both are the document's own: pinch is zoom and a two-finger pan is a
 * scroll of the page. A chrome surface that took either would make the document unusable while the
 * surface was on screen, and there is no chrome affordance worth that. `tests/mobile.test.ts`
 * asserts no region claims one, which is the assertion that fails if somebody writes
 * `touch-action: none` on a bar to stop it scrolling.
 */
export const canvasReservedGestures: readonly Gesture[] = ['pinch', 'twoFingerPan'];

/** The surfaces a gesture can land on. */
export const gestureRegionNames = [
  'canvas',
  'commandBar',
  'commandBarButton',
  'sheetHandle',
  'sheetHeader',
  'sheetScroller',
  'overflowPanel',
] as const;

/** One of the seven. */
export type GestureRegion = (typeof gestureRegionNames)[number];

/** Who a region belongs to. */
export type GestureOwner = 'canvas' | 'chrome';

/**
 * What `touch-action` a region writes, and therefore what it takes.
 *
 * `auto` takes nothing: every gesture reaches the document. `pan-y` takes the inline axis, because
 * the browser will only scroll the block axis and treats an inline drag as the element's own.
 * `pan-x` is the mirror. `none` takes everything, which is why only the grab handle has it.
 */
export const touchActionValues = ['auto', 'pan-x', 'pan-y', 'none'] as const;

/** One of the four. */
export type TouchActionValue = (typeof touchActionValues)[number];

/**
 * Which gestures a `touch-action` value takes away from the document.
 *
 * This is the browser's own behaviour written down, and it is the derivation the conflict gate
 * runs. `pan-x` means *the browser may pan this element horizontally*, so a horizontal drag is the
 * element's and a vertical one is still the page's — hence `panInline` for `pan-x`.
 */
export function gesturesSuppressedBy(value: TouchActionValue): readonly Gesture[] {
  switch (value) {
    case 'auto':
      return [];
    case 'pan-x':
      return ['panInline'];
    case 'pan-y':
      return ['panBlock'];
    case 'none':
      return ['panBlock', 'panInline', 'twoFingerPan', 'pinch'];
  }
}

/** One region's whole declaration. */
export interface GestureRegionSpec {
  readonly owner: GestureOwner;
  /** What it writes. The browser gate reads the computed value and compares. */
  readonly touchAction: TouchActionValue;
  /**
   * The gestures the chrome takes here. **Must equal `gesturesSuppressedBy(touchAction)`** for a
   * chrome region, and must be empty for the canvas.
   */
  readonly claims: readonly Gesture[];
  /** What each claim does, in the words the audit reads. */
  readonly does: string;
  /** What is deliberately left to the canvas here, and why. */
  readonly leaves: string;
}

/**
 * **The map.** Seven regions, and the only two that take anything are the two grab areas and the
 * two scrollers.
 *
 * ⚠ **`longPress`, `tap` and `doubleTap` are claimed by nobody, and that is deliberate.** They are
 * not pan gestures and `touch-action` cannot express them, so a claim on one would be a claim this
 * table could not enforce — and, just as importantly, **nothing in this catalogue implements one.**
 * A convention described in a table with no code behind it is the shape CLAUDE.md calls worse than
 * none, so the rows say what the components actually do. A press that lands on chrome never reaches
 * the canvas because the chrome is opaque; where it could genuinely be either, `resolveGesture`
 * says canvas.
 */
export const gestureRegions: Readonly<Record<GestureRegion, GestureRegionSpec>> = {
  canvas: {
    owner: 'canvas',
    touchAction: 'auto',
    claims: [],
    does: 'Pan, pinch to zoom, tap to place the caret, double-tap to select a word, long-press for the context menu, two-finger pan to scroll. ⚠ There is no canvas in this catalogue: this row is the CONTRACT the chrome is written against, and what honours it is every other row claiming nothing it does not need.',
    leaves: 'Everything. This is the document, and it is the product.',
  },
  commandBar: {
    owner: 'chrome',
    touchAction: 'pan-x',
    claims: ['panInline'],
    does: 'A horizontal drag scrolls the command rail, which is what makes an overflowing row usable without opening anything.',
    leaves:
      'The block axis, so a vertical flick that starts on the bar still scrolls the document — and pinch, so a two-finger zoom that happens to begin over the bar still zooms.',
  },
  commandBarButton: {
    owner: 'chrome',
    touchAction: 'auto',
    claims: [],
    does: 'Tap activates, and nothing else is taken. ⚠ A LONG PRESS IS LEFT TO THE PLATFORM and is deliberately not claimed: the touch equivalent of a screentip needs a command bar wired to a document, which is loop 2, and a convention described here with no code behind it would be worse than none.',
    leaves: 'Every pan, both multi-touch gestures, and the long press.',
  },
  sheetHandle: {
    owner: 'chrome',
    touchAction: 'none',
    claims: ['panBlock', 'panInline', 'twoFingerPan', 'pinch'],
    does: 'Drags the sheet between its detents, and dismisses it below the dismissal threshold.',
    leaves:
      'Nothing — and it is the only region in this table of which that is true. A grab handle is a few pixels tall and a person who put two fingers on it did not mean to zoom.',
  },
  sheetHeader: {
    owner: 'chrome',
    touchAction: 'pan-y',
    claims: ['panBlock'],
    does: 'A second grab area, because a handle alone is a small target and the title row above the content is the obvious place to pull from.',
    leaves:
      'The inline axis and both multi-touch gestures, so a pinch over the header still reaches the document.',
  },
  sheetScroller: {
    owner: 'chrome',
    touchAction: 'pan-y',
    claims: ['panBlock'],
    does: 'Scrolls the sheet’s own content. `sheetDragClaim` decides whether a block-axis drag scrolls this or moves the sheet, and the sheet only ever wins at the top of the scroller.',
    leaves: 'The inline axis and both multi-touch gestures.',
  },
  overflowPanel: {
    owner: 'chrome',
    touchAction: 'pan-y',
    claims: ['panBlock'],
    does: 'Scrolls the overflow panel’s grid of commands.',
    leaves: 'The inline axis and both multi-touch gestures.',
  },
};

/**
 * **Who gets this gesture here** — and the ambiguity rule, which is the whole point.
 *
 * A region owns a gesture only if it says so explicitly. Anything a region has not claimed goes to
 * the canvas, including gestures no region could claim (`tap`, `doubleTap`, `longPress`) and
 * including regions that do not exist in this table. *Ambiguity resolves to the canvas* is
 * therefore the function's **default branch** rather than a special case bolted onto it, which is
 * the only shape in which the rule cannot be forgotten.
 */
export function resolveGesture(gesture: Gesture, region: GestureRegion): GestureOwner {
  const spec = gestureRegions[region];
  if (spec === undefined) return 'canvas';
  return spec.claims.includes(gesture) ? 'chrome' : 'canvas';
}

/** A region whose declared claims do not match the `touch-action` it writes. */
export interface GestureInconsistency {
  readonly region: GestureRegion;
  readonly declared: readonly Gesture[];
  readonly derived: readonly Gesture[];
}

/**
 * Every region where the claims and the `touch-action` disagree.
 *
 * Empty is the only acceptable answer and `tests/mobile.test.ts` says so. The point of computing it
 * as a list rather than as a boolean is that the failure message names the region and both sets,
 * which is the difference between a gate that says *something is wrong* and one that says what.
 */
export function gestureInconsistencies(): readonly GestureInconsistency[] {
  const found: GestureInconsistency[] = [];
  for (const region of gestureRegionNames) {
    const spec = gestureRegions[region];
    const derived = [...gesturesSuppressedBy(spec.touchAction)].sort();
    const declared = [...spec.claims].sort();
    if (declared.join(',') !== derived.join(',')) found.push({ region, declared, derived });
  }
  return found;
}

/** Every region that has taken a gesture the canvas reserves. There must be exactly one. */
export function regionsTakingReservedGestures(): readonly GestureRegion[] {
  return gestureRegionNames.filter((region) =>
    gestureRegions[region].claims.some((gesture) => canvasReservedGestures.includes(gesture)),
  );
}

/**
 * The one region **in the table above** allowed to take a reserved gesture, and the argument for it.
 *
 * A grab handle is a strip a few pixels tall. `touch-action: none` on it is what makes a drag from
 * the handle move the sheet rather than scroll the page behind, and there is no narrower value that
 * does: `pan-y` would hand the block axis to the browser, which is the axis the drag *is*. The cost
 * is that a pinch beginning exactly on the handle does not reach the document, and that is
 * acceptable because nobody pinches a grab handle.
 *
 * It is named here rather than merely allowed, so that a second region acquiring `touch-action:
 * none` is a failing test with a name in it rather than a silent widening.
 */
export const reservedGestureException: GestureRegion = 'sheetHandle';

/** One place in the catalogue where a component writes `touch-action: none`. */
export interface ReservedGestureSite {
  /** The module, relative to `src/`. */
  readonly module: string;
  readonly what: string;
  readonly because: string;
}

/**
 * **Every place in the whole catalogue that takes the document's gestures.**
 *
 * ⚠ **This list exists because the region table above was not enough**, and finding that out is one
 * of the things MJXOFF-194's sweep did. The table describes the mobile shell; a browser gate over
 * the mobile shell's own stories therefore checks the mobile shell — which is a sweep over what
 * this child happened to touch, and is precisely the trap the ticket names one level up. There were
 * already **five** `touch-action: none` declarations in this catalogue when it started, in four
 * crates this child does not own, and not one of them was written down anywhere.
 *
 * All five turn out to be the same thing and it is the same argument the sheet handle makes: **a
 * drag affordance takes everything, because a drag on a grab strip is a drag and not a pan.** None
 * of them is a scrolling surface and none of them is large enough to pinch on purpose. So the list
 * is a set of justifications rather than an exemption list, and `tests/mobile.test.ts` scans `src/`
 * for the declaration and requires the set of modules to match this one **exactly** — a sixth site
 * is a failing test with a path in it, and a site that went away is too.
 */
export const reservedGestureSites: readonly ReservedGestureSite[] = [
  {
    module: 'formula/formula-sheets.ts',
    what: "the formula bar's expand handle",
    because:
      'A resize grip. A drag on it lengthens the bar, and handing the block axis to the browser would scroll the sheet behind it instead.',
  },
  {
    module: 'furniture/furniture-model.ts',
    what: "the scrollbar's track and the splitter's grip",
    because:
      'Both are drags. A scrollbar that let the browser pan it would scroll twice, and a splitter that did would move the pane and the document together.',
  },
  {
    module: 'inputs/input-model.ts',
    what: "the slider's track",
    because:
      'A slider is a drag along its own axis, and a browser pan on the same axis would move the page under the thumb.',
  },
  {
    module: 'surfaces/surface-model.ts',
    what: "the task pane's splitter",
    because: 'The same drag as the furniture splitter, on the surface that owns its own edge.',
  },
  {
    module: 'mobile/mobile-sheets.ts',
    what: "the bottom sheet's grab handle",
    because:
      'The region named by `reservedGestureException`. A few pixels tall, and the one place a drag means *move the sheet*.',
  },
];
