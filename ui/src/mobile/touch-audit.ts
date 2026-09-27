/**
 * **The touch-target audit** — the catalogue's own enumeration of itself, and the rule every
 * target in it is measured against.
 *
 * MJXOFF-194 calls this *"the most valuable half of this child"*, and states the trap it is written
 * against:
 *
 * > The touch-target sweep is only meaningful if it covers **everything** — a sweep over the
 * > components this child happens to touch proves nothing, so it must enumerate the catalogue and
 * > fail on any component it cannot account for.
 *
 * And the programme's own recurring defect, which is the same shape:
 *
 * > A helper whose failure mode is *found nothing* makes every ceiling assertion pass.
 *
 * So the sweep is built to fail loudly in **four** independent ways, and each of them has been
 * watched failing:
 *
 * 1. **A component the registry does not know about.** `tests/mobile.test.ts` scans every file in
 *    `src/` for a `customElements.define` and compares what it finds against `catalogueComponents`
 *    in both directions. A new element is a failing unit test with its module path in the message,
 *    before any browser starts.
 * 2. **A component the registry knows about that no story renders.** The browser sweep records
 *    which tags it actually saw and requires every registered tag to have been seen.
 * 3. **A component declared to have no targets of its own that grew one.** `presentational`,
 *    `descriptor` and `container` entries are asserted to contribute **zero** targets, so a data
 *    element that acquires a button — or a menu that starts building its own — fails rather than
 *    quietly leaving the audit.
 * 4. **A target that is too small.** The measurement itself.
 *
 * ## Both axes, and the rule that makes that affordable
 *
 * MJXOFF-193 flagged that the catalogue's 24-pixel floor is asserted on the **block** axis only and
 * that the inline axis is unchecked. This child owns the sweep, so the decision is made here and
 * stated: **both axes are asserted.** A 200 x 12 target and a 12 x 200 one are equally unhittable,
 * and an audit that measured one of them was measuring the easy one — every control in this
 * catalogue is laid out in a row, so the block axis is the axis a stray padding fixes by accident.
 *
 * What makes that affordable rather than a wall of exemptions is that the rule asserted is
 * **WCAG 2.2 SC 2.5.8's actual rule and not a simplification of it**: a target passes if it is at
 * least 24 x 24, *or* if a 24-pixel-diameter circle centred on it touches no other target's circle.
 * The spacing exception is what legitimately passes a thin scrollbar thumb, a splitter and a slider
 * track — the three things a naive both-axes floor would have forced an exemption list for — and it
 * passes them for the right reason, which is that there is nothing next to them to hit by mistake.
 *
 * ## Node-importable
 *
 * Data and pure functions. The Playwright specs import this table and evaluate the geometry in the
 * page; the arithmetic is here so it is unit-tested against hand-worked cases as well.
 */

import { accessibleHitTargetMinimum } from '../foundations/density.ts';

/** How a component participates in the audit. */
export const auditKindNames = [
  'targets',
  'container',
  'presentational',
  'descriptor',
  'harness',
] as const;

/** One of the five. */
export type AuditKind = (typeof auditKindNames)[number];

/** One catalogue component, as the audit knows it. */
export interface CatalogueComponent {
  /** The registered tag name. The browser sweep checks this is genuinely defined. */
  readonly tag: string;
  /** Its module, relative to `src/`. The source scan compares this. */
  readonly module: string;
  /**
   * The **source text** of the first argument to `customElements.define`.
   *
   * Half this catalogue registers through a `tags` constant rather than a literal, so a scanner
   * that only understood literals would silently skip twenty-eight components — the *found
   * nothing* failure mode, in the enumeration itself. Comparing the expression text handles both
   * spellings without resolving anything.
   */
  readonly definedAs: string;
  readonly kind: AuditKind;
  /** Required for anything that is not `targets`: why it has none. */
  readonly reason?: string;
}

/**
 * **The catalogue.** Every custom element `src/` registers, in module order.
 *
 * A new component is added here *and* nowhere else — the unit test finds it in the source and the
 * browser sweep finds it in a story. Forgetting either is a failing test with a name in it.
 */
export const catalogueComponents: readonly CatalogueComponent[] = [
  // ── annotation (MJXOFF-193) ────────────────────────────────────────────────
  {
    tag: 'mjx-comment-card',
    module: 'annotation/comment-card.ts',
    definedAs: 'annotationTags.commentCard',
    kind: 'targets',
  },
  {
    tag: 'mjx-comment-thread',
    module: 'annotation/comment-thread.ts',
    definedAs: 'annotationTags.commentThread',
    kind: 'targets',
  },
  {
    tag: 'mjx-review-pane',
    module: 'annotation/review-pane.ts',
    definedAs: 'annotationTags.reviewPane',
    kind: 'targets',
  },
  {
    tag: 'mjx-tracked-change-card',
    module: 'annotation/tracked-change-card.ts',
    definedAs: 'annotationTags.trackedChangeCard',
    kind: 'targets',
  },

  // ── controls (MJXOFF-182) ─────────────────────────────────────────────────
  { tag: 'mjx-button', module: 'controls/button.ts', definedAs: "'mjx-button'", kind: 'targets' },
  {
    tag: 'mjx-dialog-launcher',
    module: 'controls/dialog-launcher.ts',
    definedAs: "'mjx-dialog-launcher'",
    kind: 'targets',
  },
  {
    tag: 'mjx-split-button',
    module: 'controls/split-button.ts',
    definedAs: "'mjx-split-button'",
    kind: 'targets',
  },
  {
    tag: 'mjx-toggle-button',
    module: 'controls/toggle-button.ts',
    definedAs: "'mjx-toggle-button'",
    kind: 'targets',
  },

  // ── feedback (MJXOFF-189) ─────────────────────────────────────────────────
  {
    tag: 'mjx-empty-state',
    module: 'feedback/empty-state.ts',
    definedAs: 'feedbackTags.emptyState',
    kind: 'targets',
  },
  {
    tag: 'mjx-mini-toolbar',
    module: 'feedback/mini-toolbar.ts',
    definedAs: 'feedbackTags.miniToolbar',
    kind: 'targets',
  },
  {
    tag: 'mjx-progress',
    module: 'feedback/progress.ts',
    definedAs: 'feedbackTags.progress',
    kind: 'presentational',
    reason:
      'A progress bar is a reading, not a control. It has no interactive part at all, and a person cannot change it — the operation it reports is what changes it.',
  },
  {
    tag: 'mjx-screentip',
    module: 'feedback/screentip.ts',
    definedAs: 'feedbackTags.screentip',
    kind: 'presentational',
    reason:
      'A screentip is what a hover or a long press *reveals*; the target is the control it describes, which is audited on its own.',
  },
  {
    tag: 'mjx-toast',
    module: 'feedback/toast.ts',
    definedAs: 'feedbackTags.toast',
    kind: 'descriptor',
    reason:
      'A descriptor read for its attributes by the region that holds it and never rendered — `role="none"` and hidden, exactly as `<mjx-option>` is. The region builds the card a person presses.',
  },
  {
    tag: 'mjx-toast-region',
    module: 'feedback/toast.ts',
    definedAs: 'feedbackTags.toastRegion',
    kind: 'targets',
  },

  // ── formula (MJXOFF-192) ──────────────────────────────────────────────────
  {
    tag: 'mjx-formula-bar',
    module: 'formula/formula-bar.ts',
    definedAs: "'mjx-formula-bar'",
    kind: 'targets',
  },
  { tag: 'mjx-name-box', module: 'formula/name-box.ts', definedAs: "'mjx-name-box'", kind: 'targets' },

  // ── foundations (MJXOFF-181) ──────────────────────────────────────────────
  {
    tag: 'mjx-surface',
    module: 'foundations/surface.ts',
    definedAs: "'mjx-surface'",
    kind: 'presentational',
    reason: 'An elevation rung with a radius and a density. It paints a box and nothing else.',
  },

  // ── furniture (MJXOFF-190) ────────────────────────────────────────────────
  {
    tag: 'mjx-scrollbar',
    module: 'furniture/scrollbar.ts',
    definedAs: 'furnitureTags.scrollbar',
    kind: 'targets',
  },
  {
    tag: 'mjx-scroll-mark',
    module: 'furniture/scrollbar.ts',
    definedAs: 'furnitureTags.scrollMark',
    kind: 'descriptor',
    reason:
      'A declaration that something is at this position in the document — a search hit, a tracked change. The scrollbar draws it; it is not itself pressable.',
  },
  {
    tag: 'mjx-splitter',
    module: 'furniture/splitter.ts',
    definedAs: 'furnitureTags.splitter',
    kind: 'targets',
  },
  {
    tag: 'mjx-status-bar',
    module: 'furniture/status-bar.ts',
    definedAs: 'furnitureTags.statusBar',
    kind: 'targets',
  },
  {
    tag: 'mjx-status-segment',
    module: 'furniture/status-segment.ts',
    definedAs: 'furnitureTags.statusSegment',
    kind: 'presentational',
    reason:
      'A reading with a name — page 4 of 20, 3,182 words. It carries no role and no tab stop: the status bar is a live region a person watches, and the one control in it (the overflow disclosure) is the bar’s.',
  },
  {
    tag: 'mjx-zoom-control',
    module: 'furniture/zoom-control.ts',
    definedAs: 'furnitureTags.zoomControl',
    kind: 'targets',
  },

  // ── gallery (MJXOFF-185) ──────────────────────────────────────────────────
  { tag: 'mjx-gallery', module: 'gallery/gallery.ts', definedAs: 'galleryTags.gallery', kind: 'targets' },
  {
    tag: 'mjx-gallery-item',
    module: 'gallery/gallery-item.ts',
    definedAs: 'galleryTags.item',
    kind: 'descriptor',
    reason:
      'A descriptor, deliberately not a slot — MJXOFF-185 says so at length. The gallery reads it and draws the cell; the cell is the target and the gallery is where it is measured.',
  },

  // ── the harness (MJXOFF-180) ──────────────────────────────────────────────
  {
    tag: 'mjx-resizable-container',
    module: 'harness/resizable-container.ts',
    definedAs: "'mjx-resizable-container'",
    kind: 'harness',
    reason:
      'The audit frame itself. It is not shipped chrome and never reaches a phone; its own controls are the thing an auditor drives the catalogue with.',
  },

  // ── icons (MJXOFF-181) ────────────────────────────────────────────────────
  {
    tag: 'mjx-icon',
    module: 'icons/icon.ts',
    definedAs: "'mjx-icon'",
    kind: 'presentational',
    reason: 'A glyph. It is drawn inside targets and is never one.',
  },

  // ── inputs (MJXOFF-186) ───────────────────────────────────────────────────
  { tag: 'mjx-checkbox', module: 'inputs/checkbox.ts', definedAs: "'mjx-checkbox'", kind: 'targets' },
  { tag: 'mjx-combo-box', module: 'inputs/combo-box.ts', definedAs: "'mjx-combo-box'", kind: 'targets' },
  {
    tag: 'mjx-option',
    module: 'inputs/descriptors.ts',
    definedAs: "'mjx-option'",
    kind: 'descriptor',
    reason:
      'A value in a list, declared in light DOM. The dropdown and the combo box build the row a person presses, and those rows are audited there.',
  },
  {
    tag: 'mjx-segment',
    module: 'inputs/descriptors.ts',
    definedAs: "'mjx-segment'",
    kind: 'descriptor',
    reason: 'The same, for a segmented control.',
  },
  { tag: 'mjx-dropdown', module: 'inputs/dropdown.ts', definedAs: "'mjx-dropdown'", kind: 'targets' },
  {
    tag: 'mjx-label',
    module: 'inputs/label.ts',
    definedAs: "'mjx-label'",
    kind: 'presentational',
    reason:
      'A name for a field, with its required mark and its description. Pressing it moves focus to the field, which the platform does for a native label without the label being a target of its own.',
  },
  {
    tag: 'mjx-measure-input',
    module: 'inputs/measure-input.ts',
    definedAs: "'mjx-measure-input'",
    kind: 'targets',
  },
  {
    tag: 'mjx-segmented-control',
    module: 'inputs/segmented-control.ts',
    definedAs: "'mjx-segmented-control'",
    kind: 'targets',
  },
  { tag: 'mjx-slider', module: 'inputs/slider.ts', definedAs: "'mjx-slider'", kind: 'targets' },

  // ── menus (MJXOFF-184) ────────────────────────────────────────────────────
  {
    tag: 'mjx-context-menu',
    module: 'menus/context-menu.ts',
    definedAs: 'menuTags.contextMenu',
    kind: 'container',
    reason:
      'It holds `<mjx-menu-item>`s and builds none of their targets. Every item owns itself and is measured on its own; the menu’s own shadow root is a box, a scroller and a scrim.',
  },
  {
    tag: 'mjx-menu',
    module: 'menus/menu.ts',
    definedAs: "'mjx-menu'",
    kind: 'container',
    reason: 'The same: a box around items that own themselves.',
  },
  { tag: 'mjx-menu-item', module: 'menus/menu-item.ts', definedAs: "'mjx-menu-item'", kind: 'targets' },
  {
    tag: 'mjx-menu-section',
    module: 'menus/menu-structure.ts',
    definedAs: "'mjx-menu-section'",
    kind: 'presentational',
    reason: 'A titled grouping inside a menu. The items inside it are the targets.',
  },
  {
    tag: 'mjx-menu-separator',
    module: 'menus/menu-structure.ts',
    definedAs: "'mjx-menu-separator'",
    kind: 'presentational',
    reason: 'A rule between groups of items, with a presentation role and no behaviour.',
  },

  // ── the two mobile bars (MJXOFF-194) ──────────────────────────────────────
  {
    tag: 'mjx-command-bar',
    module: 'mobile/command-bar.ts',
    definedAs: 'mobileTags.commandBar',
    kind: 'targets',
  },
  {
    tag: 'mjx-contextual-action-bar',
    module: 'mobile/contextual-action-bar.ts',
    definedAs: 'mobileTags.contextualActionBar',
    kind: 'targets',
  },

  // ── navigators (MJXOFF-191) ───────────────────────────────────────────────
  {
    tag: 'mjx-sheet-tab-bar',
    module: 'navigators/sheet-tab-bar.ts',
    definedAs: 'navigatorTags.sheetTabBar',
    kind: 'targets',
  },
  {
    tag: 'mjx-thumbnail-rail',
    module: 'navigators/thumbnail-rail.ts',
    definedAs: 'navigatorTags.thumbnailRail',
    kind: 'targets',
  },
  { tag: 'mjx-tree', module: 'navigators/tree.ts', definedAs: 'navigatorTags.tree', kind: 'targets' },
  {
    tag: 'mjx-virtual-list',
    module: 'navigators/virtual-list.ts',
    definedAs: 'navigatorTags.virtualList',
    kind: 'targets',
  },

  // ── pickers (MJXOFF-187) ──────────────────────────────────────────────────
  {
    tag: 'mjx-color-picker',
    module: 'pickers/color-picker.ts',
    definedAs: "'mjx-color-picker'",
    kind: 'targets',
  },
  {
    tag: 'mjx-font-picker',
    module: 'pickers/font-picker.ts',
    definedAs: "'mjx-font-picker'",
    kind: 'targets',
  },

  // ── the R10 plate gallery (MJXOFF-180) ────────────────────────────────────
  {
    tag: 'mjx-plate-gallery',
    module: 'plates/gallery.ts',
    definedAs: "'mjx-plate-gallery'",
    kind: 'harness',
    reason:
      'The loader for the render oracle’s plate manifest. It is a developer surface for reviewing plates, not chrome that reaches a phone.',
  },

  // ── ribbon (MJXOFF-183) ───────────────────────────────────────────────────
  {
    tag: 'mjx-contextual-tab-set',
    module: 'ribbon/contextual-tab-set.ts',
    definedAs: "'mjx-contextual-tab-set'",
    kind: 'presentational',
    reason:
      'A coloured band naming a set of contextual tabs. The tab buttons live in the ribbon’s own shadow root and are audited there; the band is a picture and is hidden from the accessibility tree.',
  },
  {
    tag: 'mjx-ribbon-group',
    module: 'ribbon/ribbon-group.ts',
    definedAs: "'mjx-ribbon-group'",
    kind: 'targets',
  },
  {
    tag: 'mjx-ribbon-tab',
    module: 'ribbon/ribbon-tab.ts',
    definedAs: "'mjx-ribbon-tab'",
    kind: 'presentational',
    reason:
      'The panel half of a tab. Its button is drawn by the ribbon, for the reason MJXOFF-183 gives about IDREFs not crossing a shadow boundary.',
  },
  { tag: 'mjx-ribbon', module: 'ribbon/ribbon.ts', definedAs: "'mjx-ribbon'", kind: 'targets' },

  // ── surfaces (MJXOFF-188) ─────────────────────────────────────────────────
  { tag: 'mjx-dialog', module: 'surfaces/dialog.ts', definedAs: 'surfaceTags.dialog', kind: 'targets' },
  { tag: 'mjx-popover', module: 'surfaces/popover.ts', definedAs: 'surfaceTags.popover', kind: 'targets' },
  {
    tag: 'mjx-task-pane',
    module: 'surfaces/task-pane.ts',
    definedAs: 'surfaceTags.taskPane',
    kind: 'container',
    reason:
      'A dock around whatever the host slots into it, and the one surface MJXOFF-188 made NOT dismissible — so it has no close button of its own. Its splitter is a target, and at phone width the pane is not docked and the splitter is not drawn.',
  },
];

/** The registry as a map, for a sweep that has a tag and wants the entry. */
export const catalogueByTag: ReadonlyMap<string, CatalogueComponent> = new Map(
  catalogueComponents.map((component) => [component.tag, component]),
);

/** The tags whose targets are measured. */
export const auditedTags: readonly string[] = catalogueComponents
  .filter((component) => component.kind === 'targets')
  .map((component) => component.tag);

/**
 * The tags asserted to contribute no targets **of their own**.
 *
 * ⚠ **`container` is a weaker claim than `targets` and the difference is worth stating.** A menu is
 * full of things to press and builds none of them: every one is an `<mjx-menu-item>`, which owns
 * itself and is measured on its own. So a container is asserted to add nothing, which is the same
 * anti-rot property `presentational` has — a menu that grew a button of its own fails — and it is
 * **not** asserted to contain anything, because what it contains is audited elsewhere or belongs to
 * the host application.
 */
export const targetlessTags: readonly string[] = catalogueComponents
  .filter(
    (component) =>
      component.kind === 'presentational' ||
      component.kind === 'descriptor' ||
      component.kind === 'container',
  )
  .map((component) => component.tag);

// ── what counts as a target ──────────────────────────────────────────────────

/**
 * The selector the sweep walks with, applied inside every shadow root as well as the light DOM.
 *
 * It is deliberately role-first rather than tag-first: this catalogue's controls are `<button>`
 * elements inside shadow roots, but its list rows, tree items and gallery cells are `<div>`s
 * carrying a role, and a tag-based selector would have found the easy half.
 */
export const interactiveSelector = [
  'a[href]',
  'button',
  'input:not([type="hidden"])',
  'select',
  'textarea',
  'summary',
  '[tabindex]:not([tabindex="-1"])',
  '[role="button"]',
  '[role="checkbox"]',
  '[role="combobox"]',
  '[role="link"]',
  '[role="menuitem"]',
  '[role="menuitemcheckbox"]',
  '[role="menuitemradio"]',
  '[role="option"]',
  '[role="radio"]',
  '[role="separator"][tabindex]',
  '[role="slider"]',
  '[role="spinbutton"]',
  '[role="switch"]',
  '[role="tab"]',
  '[role="treeitem"]',
].join(',');

// ── the rule ─────────────────────────────────────────────────────────────────

/** A measured box, in CSS pixels. */
export interface TargetBox {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** Why a target passed, or how it failed. */
export type TargetVerdict =
  | { readonly ok: true; readonly rule: 'size' | 'spacing' }
  | { readonly ok: false; readonly rule: 'crowded'; readonly nearest: number };

/** The centre of a box. */
function centre(box: TargetBox): { x: number; y: number } {
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

/**
 * **WCAG 2.2 SC 2.5.8, Target Size (Minimum)** — the rule itself, not a simplification.
 *
 * A target passes if it is at least `accessibleHitTargetMinimum` on **both** axes, or if a circle
 * of that diameter centred on it does not intersect the equivalent circle of any other target.
 * Two circles of diameter *d* intersect exactly when their centres are closer than *d*, which is
 * why the spacing test is one distance comparison rather than a geometry library.
 *
 * The undersized-but-spaced branch is what legitimately passes a scrollbar thumb, a splitter and a
 * slider track. It is not a loophole: a thin control with nothing within 24 pixels of it is a
 * control nobody misses by aiming at its neighbour, which is the whole thing the criterion is
 * about.
 */
export function meetsTargetSize(
  target: TargetBox,
  neighbours: readonly TargetBox[],
  minimum: number = accessibleHitTargetMinimum,
): TargetVerdict {
  if (target.width >= minimum && target.height >= minimum) return { ok: true, rule: 'size' };

  const here = centre(target);
  let nearest = Number.POSITIVE_INFINITY;
  for (const other of neighbours) {
    const there = centre(other);
    const distance = Math.hypot(here.x - there.x, here.y - there.y);
    if (distance < nearest) nearest = distance;
  }
  if (nearest >= minimum) return { ok: true, rule: 'spacing' };
  return { ok: false, rule: 'crowded', nearest };
}

/**
 * The floor the two mobile bars are held to, which is **higher** than the WCAG minimum.
 *
 * 24 pixels is a legal floor and a poor phone experience; the density system's comfortable target
 * is 40 and its compact one 32, and a bar that exists only on a phone has no excuse for the floor.
 * This is asserted on the bars alone rather than on the catalogue, because a compact desktop
 * inspector legitimately sits at the lower number.
 */
export const mobileBarTargetMinimum = 40;

/** How the audit reports a failure, in a message a person can act on. */
export function describeTarget(
  where: string,
  box: TargetBox,
  verdict: TargetVerdict,
  minimum: number = accessibleHitTargetMinimum,
): string {
  const size = `${box.width.toFixed(1)} x ${box.height.toFixed(1)}`;
  if (verdict.ok) return `${where} is ${size} and passes by ${verdict.rule}.`;
  return (
    `${where} is ${size}, which is under the ${String(minimum)} px floor on at least one axis, ` +
    `and its nearest neighbouring target is ${verdict.nearest.toFixed(1)} px away — under the ` +
    `${String(minimum)} px the spacing exception needs. Either make the target bigger or move ` +
    'whatever is beside it.'
  );
}
