/**
 * MJXOFF-193's model — **the margin packing problem, and the vocabulary four components share.**
 *
 * ## The one hard thing in this child
 *
 * Review is the only surface in the catalogue whose layout is a function of geometry the *canvas*
 * owns. A comment card wants to sit beside the text it is about; several comments can be about
 * three consecutive lines; and two cards may not overlap. That is a **one-dimensional packing
 * problem with preferred positions**, and the ticket says in as many words what makes it a trap:
 *
 * > The margin packing problem does not appear with well-spaced anchors, which is what a hand-made
 * > fixture naturally has — and a simple stacked list looks identical and correct there.
 *
 * So the naive answer is shipped beside the real one. [`stackMarginCards`] is the greedy downward
 * push everybody writes first: start at the anchor, and if a card would land on the one above it,
 * move it down. It never overlaps, it is obviously correct on a well-spaced fixture, and it is
 * wrong in two ways that only a tight cluster shows — it never moves a card **up**, so the whole
 * cluster droops below its anchors, and it cannot hold a selected card at its preferred position
 * because everything below the first collision is downstream of it.
 *
 * ## What the real one is, and why it is exactly this
 *
 * Sort the cards by anchor, subtract each card's cumulative stack offset, and the non-overlap
 * constraint *y_i + h_i + gap <= y_{i+1}* becomes plain **z_1 <= z_2 <= … <= z_n**. Minimising the
 * squared distance from the preferred positions under that constraint is textbook **isotonic
 * regression**, which [`packMarginCards`] solves exactly with pool-adjacent-violators in one pass.
 * There is no iteration, no relaxation and no tolerance: the answer is the optimum, and
 * `tests/annotation.test.ts` proves it against an independent reference that enumerates every block
 * structure and takes the best feasible one.
 *
 * Two properties fall out of the transform rather than being coded:
 *
 * * The **column bounds are a uniform box** in *z*, even though they are not in *y* — a lower bound
 *   on the first card is a lower bound on every *z*, and an upper bound on the last card's bottom is
 *   an upper bound on every *z*. So the bounded optimum is the unbounded one clamped, which is why
 *   there is no second algorithm for the bounded case.
 * * The **selected card's pin splits the problem in two.** Fixing *z_s* leaves the cards above it
 *   an isotonic problem with a ceiling and the cards below it one with a floor, and neither can
 *   affect the other. That is what makes *the selected card takes its preferred position and the
 *   others yield around it* a property of the decomposition rather than a special case.
 *
 * ## Node-importable
 *
 * Numbers, strings and one import of the catalogue's own virtualisation. No DOM, no CSS —
 * `src/foundations/splitter.ts`'s rule, for the reason it gives.
 */

import { ExtentTable } from '../foundations/extent-table.ts';
import { windowForOffset, type VirtualWindow } from '../foundations/virtual-list.ts';
import { phoneShellAtOrBelow } from '../harness/presets.ts';

// ── what an annotation is ──────────────────────────────────────────────────────────────────────

/**
 * The two comment models Word actually carries, and they are **not** two renderings of one thing.
 *
 * A *legacy* comment is a `w:comment` in `word/comments.xml` with an author, an initials string and
 * a date: one remark, no replies, resolved-ness nowhere in the model. A *threaded* comment is the
 * modern shape — `word/commentsExtended.xml` gives it a paragraph identity, a parent and a `done`
 * flag, so a reply is a comment whose parent is another comment and resolution is a property of the
 * thread. A card that rendered both alike would be telling a reader they can reply to something
 * that has nowhere to put a reply.
 */
export const commentModelNames = ['legacy', 'threaded'] as const;

/** One of the two. */
export type CommentModel = (typeof commentModelNames)[number];

/** What each model can actually do, read by the components and by the gate. */
export interface CommentModelSpec {
  /** Whether a reply has anywhere to go. */
  readonly repliable: boolean;
  /** Whether the model carries a resolved flag at all. */
  readonly resolvable: boolean;
  /** The part the model lives in, so a reader can go and look. */
  readonly part: string;
  /** What the card is announced as, so the two are distinguishable to a screen reader too. */
  readonly announcement: string;
  /** The attribute value the stylesheet keys the visual difference off. */
  readonly presentation: string;
}

/**
 * ⚠ **Both halves of "distinguishable" live here.** A visual difference nobody can hear is a
 * difference only some readers get, so each model carries a presentation *and* a word.
 */
export const commentModels: Readonly<Record<CommentModel, CommentModelSpec>> = {
  legacy: {
    repliable: false,
    resolvable: false,
    part: 'word/comments.xml',
    announcement: 'note',
    presentation: 'legacy',
  },
  threaded: {
    repliable: true,
    resolvable: true,
    part: 'word/commentsExtended.xml',
    announcement: 'conversation',
    presentation: 'threaded',
  },
};

/** The four revisions Word tracks, and the four this card renders. */
export const trackedChangeKindNames = ['insertion', 'deletion', 'formatting', 'move'] as const;

/** One of the four. */
export type TrackedChangeKind = (typeof trackedChangeKindNames)[number];

/** Everything a tracked change's card needs that is not in the document's own text. */
export interface TrackedChangeSpec {
  /** The icon the card carries. */
  readonly icon: string;
  /** The verb, in the words the card and the screen reader both use. */
  readonly verb: string;
  /** The `w:` element the revision is carried as, so a reader can go and look. */
  readonly element: string;
  /** The attribute value the stylesheet keys off. */
  readonly presentation: TrackedChangeKind;
}

export const trackedChangeKinds: Readonly<Record<TrackedChangeKind, TrackedChangeSpec>> = {
  insertion: { icon: 'add', verb: 'Inserted', element: 'w:ins', presentation: 'insertion' },
  deletion: { icon: 'delete', verb: 'Deleted', element: 'w:del', presentation: 'deletion' },
  formatting: {
    icon: 'text-bold',
    verb: 'Formatted',
    element: 'w:rPrChange',
    presentation: 'formatting',
  },
  move: { icon: 'arrow-down-right', verb: 'Moved', element: 'w:moveTo', presentation: 'move' },
};

/**
 * The size every icon on a card is drawn at.
 *
 * ⚠ **16 and not 20, and it constrains which glyphs a kind may name.** Fluent draws each size
 * separately, so an icon the subset carries only at 20 renders *nothing* at 16 — a blank space in a
 * card, which is the failure `icon.ts` says nobody notices. `tests/annotation.test.ts` asserts every
 * icon named above exists in the subset at this size, which is the only thing that keeps the two
 * tables honest with each other.
 */
export const annotationIconSize = 16;

/** What a margin card is, at the level the pane cares about. */
export const annotationKindNames = ['comment', 'trackedChange'] as const;

/** One of the two. */
export type AnnotationKind = (typeof annotationKindNames)[number];

// ── the margin packing problem ─────────────────────────────────────────────────────────────────

/** A card asking to be placed. */
export interface MarginCard {
  /** Its identity, which is what a connector line and a feed position are keyed on. */
  readonly id: string;
  /**
   * Where its anchor sits, in the column's own coordinates — the block offset of the text the
   * annotation is about. This is the number the canvas owns and the chrome is handed.
   */
  readonly anchorTop: number;
  /** How tall the card is. Measured, never assumed: a two-line comment and a thread differ. */
  readonly extent: number;
}

/** Where a card ended up. */
export interface PackedCard {
  readonly id: string;
  /** The placed block offset. */
  readonly top: number;
  readonly extent: number;
  readonly anchorTop: number;
  /**
   * `top - anchorTop`. Negative means the card was pulled **up** — which is the whole difference
   * between this and a stack, and is why it is reported rather than derivable from a story.
   */
  readonly displacement: number;
}

/** Everything the packer needs beyond the cards. */
export interface PackingOptions {
  /** The clear space between two cards. Never zero: two touching cards read as one card. */
  readonly gap: number;
  /** The first block offset a card may occupy. */
  readonly columnTop: number;
  /**
   * The last one, or `undefined` when the column scrolls — which is the review pane's own case, and
   * the reason this is optional rather than a large number.
   */
  readonly columnBottom?: number;
  /** The card that must hold its preferred position, if one is selected. */
  readonly selected?: string;
}

/** The default clear space, in density units, so the value itself stays in the stylesheet. */
export const marginCardGapUnits = 2;

/**
 * Sort into anchor order, and **stably**: two annotations on the same line keep the order the
 * document gave them, which for a comment thread is the order they were written in.
 */
function inAnchorOrder(cards: readonly MarginCard[]): readonly MarginCard[] {
  return cards
    .map((card, index) => ({ card, index }))
    .sort((a, b) => a.card.anchorTop - b.card.anchorTop || a.index - b.index)
    .map((entry) => entry.card);
}

/**
 * Pool adjacent violators: the exact solution to *minimise sum (z_i - q_i)^2 subject to z
 * non-decreasing*, in one pass.
 *
 * Each pooled block carries its sum and its size rather than its mean, so a long run costs no
 * precision and the merge is an addition rather than a weighted average of averages.
 */
function isotonic(values: readonly number[]): number[] {
  const sums: number[] = [];
  const sizes: number[] = [];
  for (const value of values) {
    sums.push(value);
    sizes.push(1);
    // A block whose mean is below its predecessor's is not a solution: pool them and look again,
    // because pooling can push the merged block below the one before it in turn.
    while (sums.length > 1) {
      const last = sums.length - 1;
      const a = sums[last - 1] ?? 0;
      const na = sizes[last - 1] ?? 1;
      const b = sums[last] ?? 0;
      const nb = sizes[last] ?? 1;
      if (a / na <= b / nb) break;
      sums.splice(last - 1, 2, a + b);
      sizes.splice(last - 1, 2, na + nb);
    }
  }
  const out: number[] = [];
  for (const [block, sum] of sums.entries()) {
    const size = sizes[block] ?? 1;
    for (let i = 0; i < size; i += 1) out.push(sum / size);
  }
  return out;
}

/** Clamp a whole isotonic solution into a **uniform** box. See this file's note for why that works. */
function clampAll(values: readonly number[], low: number, high: number): number[] {
  return values.map((value) => Math.min(high, Math.max(low, value)));
}

/**
 * Place the cards: **no overlap, and every card as near its anchor as that allows.**
 *
 * Returns them in anchor order, which is the order a feed announces them in and the order the
 * connector contract is emitted in. A caller that needs input order can key on `id`.
 */
export function packMarginCards(
  cards: readonly MarginCard[],
  options: PackingOptions,
): readonly PackedCard[] {
  const ordered = inAnchorOrder(cards);
  const count = ordered.length;
  if (count === 0) return [];
  const gap = Math.max(0, options.gap);

  // The cumulative stack offset, which is what turns "must not overlap" into "must not decrease".
  const stackOffset: number[] = [];
  let running = 0;
  for (const [index, card] of ordered.entries()) {
    stackOffset.push(running);
    running += Math.max(0, card.extent) + (index === count - 1 ? 0 : gap);
  }
  const stackExtent = running;

  const preferred = ordered.map((card, index) => card.anchorTop - (stackOffset[index] ?? 0));

  const low = options.columnTop;
  const high =
    options.columnBottom === undefined
      ? Number.POSITIVE_INFINITY
      : Math.max(low, options.columnBottom - stackExtent);

  const pinnedAt =
    options.selected === undefined
      ? -1
      : ordered.findIndex((card) => card.id === options.selected);

  let placed: number[];
  if (pinnedAt < 0) {
    placed = clampAll(isotonic(preferred), low, high);
  } else {
    // The pin splits the problem: above it, an isotonic problem with a ceiling; below it, one with
    // a floor. Neither half can reach across, which is exactly what "the others yield around it"
    // means when it is a property rather than a promise.
    const pin = Math.min(high, Math.max(low, preferred[pinnedAt] ?? low));
    const above = clampAll(isotonic(preferred.slice(0, pinnedAt)), low, Math.min(high, pin));
    const below = clampAll(isotonic(preferred.slice(pinnedAt + 1)), Math.max(low, pin), high);
    placed = [...above, pin, ...below];
  }

  return ordered.map((card, index) => {
    const top = (placed[index] ?? 0) + (stackOffset[index] ?? 0);
    return {
      id: card.id,
      top,
      extent: Math.max(0, card.extent),
      anchorTop: card.anchorTop,
      displacement: top - card.anchorTop,
    };
  });
}

/**
 * **The positive control**, and it is shipped rather than written in a test on purpose.
 *
 * This is the greedy downward push: start each card at its anchor, and move it down if it would
 * land on the one above. It produces no overlap, it is identical to [`packMarginCards`] on a
 * well-spaced fixture, and on a tight cluster it is worse in two ways at once — it never pulls a
 * card up, and a selected card cannot hold its position because everything below the first
 * collision has been pushed. `tests/annotation.test.ts` computes both and asserts they differ.
 *
 * A gate with no naive answer beside the real one is a gate nobody has watched reject anything —
 * U11's rule, U12's, and MJXOFF-192's `naiveActiveArgument`.
 */
export function stackMarginCards(
  cards: readonly MarginCard[],
  options: PackingOptions,
): readonly PackedCard[] {
  const ordered = inAnchorOrder(cards);
  const gap = Math.max(0, options.gap);
  let cursor = options.columnTop;
  return ordered.map((card) => {
    const top = Math.max(cursor, card.anchorTop);
    cursor = top + Math.max(0, card.extent) + gap;
    return {
      id: card.id,
      top,
      extent: Math.max(0, card.extent),
      anchorTop: card.anchorTop,
      displacement: top - card.anchorTop,
    };
  });
}

/**
 * How far the placement is from the placement everyone wanted: the sum of squared displacements.
 *
 * The number two layouts are compared on, and the number *"as near its anchor as packing allows"*
 * is measured in. A sum of absolute displacements would be a different objective with a different
 * optimum, so it is named here rather than recomputed per test.
 */
export function packingCost(placed: readonly PackedCard[]): number {
  return placed.reduce((total, card) => total + card.displacement * card.displacement, 0);
}

/** The first pair that overlaps, or `undefined`. The gate's own words for what it found. */
export function firstOverlap(
  placed: readonly PackedCard[],
  gap: number,
): { readonly above: string; readonly below: string; readonly by: number } | undefined {
  const ordered = [...placed].sort((a, b) => a.top - b.top);
  for (let index = 1; index < ordered.length; index += 1) {
    const above = ordered[index - 1];
    const below = ordered[index];
    if (above === undefined || below === undefined) continue;
    const wanted = above.top + above.extent + gap;
    if (below.top < wanted - 1e-9) {
      return { above: above.id, below: below.id, by: wanted - below.top };
    }
  }
  return undefined;
}

// ── virtualisation: the same arithmetic, not a second one ──────────────────────────────────────

/**
 * The window of packed cards a pane should build, for a document with hundreds of annotations.
 *
 * ⚠ **This is `windowForOffset`, and deliberately not a second route to it.** U13's finding was
 * that the right route can be transitive; this child's is the mirror image of it — the route
 * through `<mjx-virtual-list>`'s [`VirtualScroller`] is the *wrong* one, because that class owns a
 * DOM contract (two spacers and a run of contiguous rows) that packed cards cannot honour: a packed
 * card sits at an absolute offset with a gap of its own choosing above it.
 *
 * What a packed layout **is**, though, is exactly what an [`ExtentTable`] describes — rows of
 * unequal height at known offsets — once each card's extent is taken to include the space beneath
 * it. So the gaps are absorbed into the rows, the table's binary search answers *which card is at
 * this offset*, and `windowForOffset` does the rest. The arithmetic has one home and this is a
 * caller of it.
 */
export function reviewWindow(
  placed: readonly PackedCard[],
  offset: number,
  viewportExtent: number,
): VirtualWindow {
  if (placed.length === 0) return { firstRow: 0, lastRow: 0 };
  const first = placed[0];
  if (first === undefined) return { firstRow: 0, lastRow: 0 };
  const table = new ExtentTable(placed.length, Math.max(1, first.extent));
  for (const [index, card] of placed.entries()) {
    const next = placed[index + 1];
    // The row's extent is its own height plus whatever the packer left beneath it, so the table's
    // offsets are the packed offsets and nothing has to be corrected afterwards.
    const extent = next === undefined ? card.extent : next.top - card.top;
    table.recordMeasured(index, Math.max(1, extent));
  }
  return windowForOffset(table, offset - first.top, viewportExtent);
}

// ── the connector contract ─────────────────────────────────────────────────────────────────────

/**
 * **What R11 binds to.** One entry per card, saying where its anchor is and where its card ended up.
 *
 * Inventory entry 54 — *"comment anchor and its connector line to the margin card"* — is **canvas,
 * not chrome**, and it is not in this child's scope. What *is* in scope is making that binding
 * trivial, which means the chrome must say, for every card it placed: which annotation, where the
 * anchor sits, where the card's leading edge ended up, which side of the column the document is on,
 * and which author colour the line should be drawn in. A canvas that had to measure the DOM to find
 * that out would be reading a layout it does not own, and the first defect would be the two
 * disagreeing about a scroll offset.
 *
 * This is MJXOFF-192's `ReferenceHighlight` applied to a second surface, and deliberately the same
 * shape: a pure description of what was drawn, emitted whenever it changes.
 */
export interface AnnotationAnchorReport {
  readonly id: string;
  readonly kind: AnnotationKind;
  /** For a comment, which of the two models it is. Absent for a tracked change. */
  readonly model?: CommentModel;
  /** Where the anchor sits, in the column's coordinates. The number the canvas gave us back. */
  readonly anchorTop: number;
  /** Where the card was placed. */
  readonly cardTop: number;
  readonly cardExtent: number;
  /**
   * Where the line should meet the card — not `cardTop`, because a line landing on a corner reads
   * as an arrow rather than as an attachment. It is inset by [`connectorInsetUnits`] density steps.
   */
  readonly connectorTop: number;
  /** Which side of the column the document is on, so the line leaves the correct edge. */
  readonly side: ConnectorSide;
  /** Which author-colour slot the line is drawn in. */
  readonly authorSlot: number;
  /** Whether this is the card the reader is on, which the canvas draws differently. */
  readonly selected: boolean;
}

/** Which edge of the card the connector leaves from. */
export const connectorSideNames = ['inlineStart', 'inlineEnd'] as const;

/** One of the two. */
export type ConnectorSide = (typeof connectorSideNames)[number];

/**
 * How far down the card's leading edge the connector lands, in density steps.
 *
 * A number rather than a length so it survives the palette re-seed, and a *step* rather than a
 * fraction of the card so a tall thread and a one-line note attach at the same visual height.
 */
export const connectorInsetUnits = 3;

// ── the two presentations ──────────────────────────────────────────────────────────────────────

/**
 * The review pane's two presentations. **A margin column has no room on a phone**, so below the
 * shell threshold it becomes a sheet listing the annotations — and packing stops being meaningful,
 * because there are no anchors beside a full-width list.
 */
export const reviewPresentationNames = ['margin', 'sheet'] as const;

/** One of the two. */
export type ReviewPresentation = (typeof reviewPresentationNames)[number];

/**
 * The width at or below which the margin column becomes a sheet.
 *
 * ⚠ **An alias of `phoneShellAtOrBelow`, never a literal.** `presets.ts` states the rule this is
 * the fourth instance of: *two numbers that must agree and are written twice are two numbers that
 * will not agree*, and `tests/annotation.test.ts` asserts the alias still is one.
 */
export const reviewSheetAtOrBelow = phoneShellAtOrBelow;

/** Which presentation a container width gets. Read by the stylesheet's generator and by the gate. */
export function reviewPresentationAt(containerWidth: number): ReviewPresentation {
  return containerWidth <= reviewSheetAtOrBelow ? 'sheet' : 'margin';
}

/** The custom property a presentation writes, so a gate reads a decision CSS made. */
export const reviewPresentationProperty = '--mjx-review-presentation';

// ── ARIA ───────────────────────────────────────────────────────────────────────────────────────

/**
 * **A conversation, not a flat list of paragraphs** — the ticket's constraint, and the reason this
 * is `feed`/`article` rather than `list`/`listitem`.
 *
 * A margin column of annotations is a stream of authored items, most of which are not built at any
 * one moment because the pane is virtualised, and each of which is a small document with its own
 * heading and its own controls. That is the WAI-ARIA **feed** pattern exactly: `role="feed"` on the
 * column, `role="article"` per card with `aria-posinset` and `aria-setsize` so a reader is told
 * *comment 4 of 213* even though only nine exist in the DOM, and focus moving article-to-article
 * rather than through every control in every card.
 *
 * A `list` would have announced *"list, 9 items"* on a document with 213 comments — the
 * virtualisation lying to the one reader who cannot see the scrollbar — and a reply would have been
 * a paragraph inside a list item rather than an authored thing with its own author and time.
 */
export const reviewAriaPattern = {
  /** The margin column. */
  container: 'feed',
  /** A card, and a reply inside a thread: a feed's articles may nest. */
  item: 'article',
  /** Set while the pane is rebuilding its window, so a reader is not walked through half a list. */
  busy: 'aria-busy',
} as const;

/** The keyboard the feed pattern specifies, plus the two aliases Office readers expect. */
export const reviewKeyboard = [
  { keys: 'Page Down', does: 'Moves to the next annotation.' },
  { keys: 'Page Up', does: 'Moves to the previous one.' },
  { keys: 'Arrow Down / Arrow Up', does: 'The same, for readers who arrived from Office.' },
  { keys: 'Control + Home', does: 'Moves to the first annotation.' },
  { keys: 'Control + End', does: 'Moves to the last one.' },
  { keys: 'Tab', does: "Moves into the focused card's own controls." },
  { keys: 'Enter', does: 'Expands or collapses a thread.' },
] as const;

/** How many replies a collapsed thread shows before it says how many more there are. */
export const collapsedReplyCount = 1;

// ── events, tags, titles ───────────────────────────────────────────────────────────────────────

/** Every event this child emits, in one table. */
export const annotationEvents = {
  /** **The connector contract.** `detail: { anchors: AnnotationAnchorReport[] }`. */
  anchors: 'mjx-annotation-anchors',
  /** One card reporting its own anchor. `detail: AnnotationAnchorReport`. */
  anchor: 'mjx-annotation-anchor',
  /** The reader moved to a different card. `detail: { id, index, count }`. */
  select: 'mjx-annotation-select',
  /** A thread was expanded or collapsed. `detail: { id, expanded }`. */
  expand: 'mjx-annotation-expand',
  /** A reply was written. `detail: { id, text }`. */
  reply: 'mjx-annotation-reply',
  /** A thread was resolved or reopened. `detail: { id, resolved }`. */
  resolve: 'mjx-annotation-resolve',
  /** A comment was deleted. `detail: { id }`. */
  delete: 'mjx-annotation-delete',
  /** A tracked change was accepted or rejected. `detail: { id, verdict }`. */
  verdict: 'mjx-annotation-verdict',
} as const;

/** What a tracked change's card asks for. */
export const trackedChangeVerdictNames = ['accept', 'reject'] as const;

/** One of the two. */
export type TrackedChangeVerdict = (typeof trackedChangeVerdictNames)[number];

/** The four tags this child registers. */
export const annotationTags = {
  commentCard: 'mjx-comment-card',
  commentThread: 'mjx-comment-thread',
  trackedChangeCard: 'mjx-tracked-change-card',
  reviewPane: 'mjx-review-pane',
} as const;

/** The story titles, so a browser gate names a story once. */
export const annotationStoryTitles = {
  commentCard: 'Annotation/Comment Card',
  commentThread: 'Annotation/Comment Thread',
  trackedChangeCard: 'Annotation/Tracked Change Card',
  reviewPane: 'Annotation/Review Pane',
} as const;
