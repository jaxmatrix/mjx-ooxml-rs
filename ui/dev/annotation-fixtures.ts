/**
 * The review fixtures — and **the tight cluster is the whole point of the file.**
 *
 * MJXOFF-193's trap, in its own words:
 *
 * > The margin packing problem does not appear with well-spaced anchors, which is what a hand-made
 * > fixture naturally has — and a simple stacked list looks identical and correct there.
 *
 * So there are two fixtures and the gate uses both. [`wellSpacedAnchors`] is the fixture a person
 * would have written: six annotations, one every couple of hundred pixels, on which the packer and
 * a naive stack agree **exactly** — `tests/annotation.test.ts` asserts that they agree, because a
 * fixture that could not have caught the defect is worth knowing about explicitly rather than
 * trusting. [`tightlyClusteredAnchors`] is the one that matters: seven annotations whose anchors
 * span about four lines of text, so that no arrangement can put every card beside its own anchor and
 * the packer has to decide who yields.
 *
 * They live in `dev/` for `src/harness/presets.ts`'s reason, restated once per child: **a module
 * that defines a custom element cannot be imported from Node**, and both the unit tier and the
 * story files need these.
 */

import type { ReviewAnnotation } from '../src/annotation/review-pane.ts';

/** The people who wrote in the fixture document. Eight, which is exactly the palette's ring. */
export const reviewAuthors: readonly string[] = [
  'Ada Lovelace',
  'Charles Babbage',
  'Grace Hopper',
  'Alan Turing',
  'Karen Spärck Jones',
  'Barbara Liskov',
  'Edsger Dijkstra',
  'Margaret Hamilton',
];

/** A line of body text, in CSS pixels, for a fixture that talks about *a few lines apart*. */
export const fixtureLineHeight = 22;

/**
 * **The fixture the gate exists for.** Seven annotations inside about four lines of text.
 *
 * A comfortable card is around ninety pixels tall and there are seven of them, so the cluster wants
 * roughly seven hundred pixels of column for eighty-eight pixels of document. Nothing can put every
 * card beside its anchor; what a packer can do is centre the crowd on it, which is what pulling
 * cards **up** buys and is exactly what a stack cannot do.
 */
export const tightlyClusteredAnchors: readonly ReviewAnnotation[] = [
  {
    id: 'c1',
    kind: 'comment',
    model: 'threaded',
    author: 'Ada Lovelace',
    time: '10:15',
    text: 'Is this the 1843 figure or the 1842 one? The caption says otherwise.',
    anchorTop: 300,
    replies: [
      { id: 'c1r1', author: 'Charles Babbage', time: '10:22', text: 'The 1843 one.' },
      { id: 'c1r2', author: 'Grace Hopper', time: '10:31', text: 'Caption fixed.' },
    ],
  },
  {
    id: 'c2',
    kind: 'comment',
    model: 'legacy',
    author: 'Charles Babbage',
    time: '10:18',
    text: 'A legacy note: no replies and no resolved flag exist in this model.',
    anchorTop: 312,
  },
  {
    id: 'r1',
    kind: 'trackedChange',
    changeKind: 'deletion',
    author: 'Grace Hopper',
    time: '10:20',
    text: 'Removed a repeated clause.',
    excerpt: 'the Analytical Engine',
    anchorTop: 318,
  },
  {
    id: 'c3',
    kind: 'comment',
    model: 'threaded',
    author: 'Alan Turing',
    time: '10:24',
    text: 'This sentence is doing two jobs.',
    anchorTop: 330,
    replies: [{ id: 'c3r1', author: 'Ada Lovelace', time: '10:40', text: 'Split it.' }],
  },
  {
    id: 'r2',
    kind: 'trackedChange',
    changeKind: 'insertion',
    author: 'Karen Spärck Jones',
    time: '10:26',
    text: 'Added a qualifying clause.',
    excerpt: ', which had not yet been built,',
    anchorTop: 334,
  },
  {
    id: 'c4',
    kind: 'comment',
    model: 'threaded',
    author: 'Barbara Liskov',
    time: '10:29',
    text: 'Resolved, but kept for the record.',
    anchorTop: 344,
    resolved: true,
    replies: [{ id: 'c4r1', author: 'Edsger Dijkstra', time: '10:33', text: 'Agreed.' }],
  },
  {
    id: 'r3',
    kind: 'trackedChange',
    changeKind: 'formatting',
    author: 'Margaret Hamilton',
    time: '10:35',
    text: 'Made the heading a Heading 2.',
    excerpt: 'Notes on the Engine',
    anchorTop: 388,
  },
];

/** The fixture a person would have written, on which a stack looks perfect. Six, well apart. */
export const wellSpacedAnchors: readonly ReviewAnnotation[] = [
  {
    id: 'w1',
    kind: 'comment',
    model: 'threaded',
    author: 'Ada Lovelace',
    time: '09:00',
    text: 'First remark.',
    anchorTop: 0,
  },
  {
    id: 'w2',
    kind: 'comment',
    model: 'threaded',
    author: 'Charles Babbage',
    time: '09:20',
    text: 'Second remark.',
    anchorTop: 260,
  },
  {
    id: 'w3',
    kind: 'trackedChange',
    changeKind: 'insertion',
    author: 'Grace Hopper',
    time: '09:40',
    text: 'Third remark.',
    excerpt: 'a phrase',
    anchorTop: 520,
  },
  {
    id: 'w4',
    kind: 'comment',
    model: 'legacy',
    author: 'Alan Turing',
    time: '10:00',
    text: 'Fourth remark.',
    anchorTop: 780,
  },
  {
    id: 'w5',
    kind: 'comment',
    model: 'threaded',
    author: 'Karen Spärck Jones',
    time: '10:20',
    text: 'Fifth remark.',
    anchorTop: 1040,
  },
  {
    id: 'w6',
    kind: 'trackedChange',
    changeKind: 'move',
    author: 'Barbara Liskov',
    time: '10:40',
    text: 'Sixth remark.',
    excerpt: 'a paragraph',
    anchorTop: 1300,
  },
];

/**
 * A document with as many annotations as the node-count gate needs, spread the way a real one is:
 * mostly apart, in occasional knots.
 *
 * Deterministic — a fixture whose contents depend on `Math.random` is a fixture whose failures
 * cannot be reproduced.
 */
export function manyAnnotations(count: number): readonly ReviewAnnotation[] {
  const kinds = ['insertion', 'deletion', 'formatting', 'move'] as const;
  const out: ReviewAnnotation[] = [];
  let anchor = 0;
  for (let index = 0; index < count; index += 1) {
    // Every fifth annotation lands within a line of the one before it, so the packing problem is
    // present in the large fixture too rather than only in the small one.
    anchor += index % 5 === 0 ? fixtureLineHeight : 180;
    const author = reviewAuthors[index % reviewAuthors.length] ?? 'Ada Lovelace';
    if (index % 3 === 0) {
      out.push({
        id: `m${String(index)}`,
        kind: 'trackedChange',
        changeKind: kinds[index % kinds.length] ?? 'insertion',
        author,
        time: '11:00',
        text: `Revision ${String(index)}.`,
        excerpt: 'a phrase in the body',
        anchorTop: anchor,
      });
      continue;
    }
    out.push({
      id: `m${String(index)}`,
      kind: 'comment',
      model: index % 7 === 0 ? 'legacy' : 'threaded',
      author,
      time: '11:00',
      text: `Remark ${String(index)} on this paragraph.`,
      anchorTop: anchor,
      ...(index % 11 === 0 ? { resolved: true } : {}),
    });
  }
  return out;
}

/** A roster of plausible author names, for measuring what a hashed assignment would do. */
export const sampleAuthorNames: readonly string[] = [
  'Ada Lovelace', 'Charles Babbage', 'Grace Hopper', 'Alan Turing',
  'Karen Spärck Jones', 'Barbara Liskov', 'Edsger Dijkstra', 'Margaret Hamilton',
  'Katherine Johnson', 'Donald Knuth', 'Frances Allen', 'John McCarthy',
  'Radia Perlman', 'Leslie Lamport', 'Shafi Goldwasser', 'Tony Hoare',
  'Jean Bartik', 'Peter Naur', 'Adele Goldberg', 'Ken Thompson',
  'Sophie Wilson', 'Dennis Ritchie', 'Anita Borg', 'Niklaus Wirth',
  'Evelyn Boyd Granville', 'Vint Cerf', 'Mary Allen Wilkes', 'Robert Kahn',
  'Erna Hoover', 'Butler Lampson', 'Lynn Conway', 'Alan Kay',
];
