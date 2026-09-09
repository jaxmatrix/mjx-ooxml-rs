/**
 * **The live-preview protocol** — a pure state machine, so *"leaving restores exactly what was
 * there"* is a thing that can be proved rather than a thing that can be observed once.
 *
 * MJXOFF-185 names the trap, and it is the sharpest one in the catalogue:
 *
 * > **Live preview is the whole point of a gallery and is exactly the part a static story cannot
 * > show.** A gallery with the events unwired looks identical in every screenshot and passes any
 * > appearance snapshot.
 *
 * The brief that commissioned this child sharpened it further, and that sentence is what this file
 * is shaped around:
 *
 * > The gate is not *"hovering changed something"* — it is **"leaving restored precisely the prior
 * > state"**. And: **a preview that never commits and a preview that always commits both look right
 * > in a still.**
 *
 * ## Why the protocol is a state machine and not three `dispatchEvent` calls
 *
 * Because *exactly* is an invariant over sequences, and a component that emitted the three events
 * from three handlers can only ever be tested one sequence at a time. Here the invariant is stated
 * once and proved over arbitrary input in a Node test:
 *
 * > **Every `preview` is followed by exactly one `cancel` or exactly one `commit`, never both and
 * > never neither, and there is at most one preview outstanding at any moment.**
 *
 * A listener that applies on `preview`, restores on `cancel` and keeps on `commit` is then *exactly*
 * correct by construction, for every sequence of hovers, focus moves, key presses and taps a person
 * can produce — including the ones nobody thought to write a test for.
 *
 * ## `restore` is in the protocol, not in the listener
 *
 * Every event carries `restore`: the value that was committed at the moment the preview began. A
 * protocol whose correctness depended on the listener keeping its own undo stack would be a
 * protocol that is right in this file and wrong in every application that consumed it — and the
 * failure mode is the specific one the brief warns about, *"restoring to the initial state rather
 * than to the committed one"*, which is invisible until somebody commits a style and then hovers
 * another and moves away.
 *
 * ## What this file deliberately does **not** do
 *
 * It does not know what a preview *is*, does not touch the DOM, and does not know the difference
 * between a pointer and an arrow key beyond passing `by` through. Coalescing — the timer that makes
 * a pointer crossing ten cells produce one request — belongs to the component, because it is a
 * question about a hand rather than about the protocol. What lives here is the part that has to be
 * true whatever the timing is, which is why the timing can be turned off in a gate without the
 * invariant going with it.
 *
 * ## Node-importable
 *
 * No DOM at all, by construction.
 */

import type { PreviewSource } from './gallery-model.ts';

/** The three things the protocol says. */
export const previewEventKinds = ['preview', 'cancel', 'commit'] as const;

/** One of the three. */
export type PreviewEventKind = (typeof previewEventKinds)[number];

/** One thing to emit. The component turns each of these into exactly one `CustomEvent`. */
export interface PreviewEvent {
  readonly kind: PreviewEventKind;
  /** The item this is about. For a `cancel`, the item whose preview is ending. */
  readonly value: string;
  /** The item's name, so a listener never has to look one up. */
  readonly label: string;
  /** What produced it. */
  readonly by: PreviewSource;
  /**
   * The value that was committed when the preview began, or `undefined` for none.
   *
   * **This is what makes restoration exact rather than approximate.** A `cancel` says *put back
   * this*, so a listener that has lost track of what it had still lands in the right place, and a
   * listener that restores to the *initial* value rather than the *committed* one is a listener
   * whose bug this field makes impossible.
   */
  readonly restore: string | undefined;
}

/** An item, as much of one as the protocol needs to know. */
export interface PreviewSubject {
  readonly value: string;
  readonly label: string;
}

/**
 * The protocol, as a machine.
 *
 * Every method **returns the events to emit, in order**, and mutates nothing else. That shape is
 * deliberate: it means the component cannot emit an event the machine did not authorise, and it
 * means a test can drive a thousand transitions without a DOM.
 */
export class PreviewSession {
  #committed: string | undefined;
  #previewing: PreviewSubject | undefined;

  constructor(committed?: string) {
    this.#committed = committed;
  }

  /** The value that is actually applied — what the document would keep if everything stopped now. */
  get committed(): string | undefined {
    return this.#committed;
  }

  /** The value being previewed, or `undefined`. */
  get previewing(): string | undefined {
    return this.#previewing?.value;
  }

  /**
   * What a listener obeying the protocol is currently showing.
   *
   * The unit gate simulates a listener and asserts that its state equals this after **every** event
   * of **every** sequence. That is a correspondence assertion in MJXOFF-182's sense: it proves the
   * emitted stream *means* what the machine says, rather than merely that the machine is
   * self-consistent.
   */
  get effective(): string | undefined {
    return this.#previewing?.value ?? this.#committed;
  }

  /** How many previews are outstanding. **The invariant is that this is never more than one.** */
  get outstanding(): number {
    return this.#previewing === undefined ? 0 : 1;
  }

  /**
   * Set the committed value without emitting anything — what a host does when the document changed
   * underneath the gallery.
   *
   * Refused while a preview is in flight, because a committed value that moved under a live preview
   * is a `restore` that would put back something that was never there. The component only calls it
   * from its `value` attribute, and cancels first.
   */
  adopt(committed: string | undefined): void {
    if (this.#previewing !== undefined) return;
    this.#committed = committed;
  }

  /**
   * Ask for a preview.
   *
   * Re-requesting the item already being previewed emits **nothing** — a pointer that wobbles
   * inside one cell, or a focus that is restored to the cell it was already on, is not a new
   * request, and a protocol that emitted one would flood a renderer with work it has already done.
   */
  request(subject: PreviewSubject, by: PreviewSource): PreviewEvent[] {
    if (this.#previewing?.value === subject.value) return [];
    const events: PreviewEvent[] = [];
    const previous = this.#previewing;
    if (previous !== undefined) {
      events.push({
        kind: 'cancel',
        value: previous.value,
        label: previous.label,
        by,
        restore: this.#committed,
      });
    }
    this.#previewing = subject;
    events.push({
      kind: 'preview',
      value: subject.value,
      label: subject.label,
      by,
      restore: this.#committed,
    });
    return events;
  }

  /** Take the preview back. Nothing to say when there is none. */
  cancel(by: PreviewSource): PreviewEvent[] {
    const previewing = this.#previewing;
    if (previewing === undefined) return [];
    this.#previewing = undefined;
    return [
      {
        kind: 'cancel',
        value: previewing.value,
        label: previewing.label,
        by,
        restore: this.#committed,
      },
    ];
  }

  /**
   * Make it real.
   *
   * The live preview of the item being committed is **not** cancelled first, and that is the one
   * asymmetry in the machine. A cancel-then-commit pair would put the old style back for one frame
   * and then apply the new one — a flash on the single interaction a gallery exists for. A preview
   * of a *different* item is cancelled, because it genuinely is being taken back.
   */
  commit(subject: PreviewSubject, by: PreviewSource): PreviewEvent[] {
    const events: PreviewEvent[] = [];
    const previewing = this.#previewing;
    if (previewing !== undefined && previewing.value !== subject.value) {
      events.push({
        kind: 'cancel',
        value: previewing.value,
        label: previewing.label,
        by,
        restore: this.#committed,
      });
    }
    const restore = this.#committed;
    this.#previewing = undefined;
    this.#committed = subject.value;
    events.push({ kind: 'commit', value: subject.value, label: subject.label, by, restore });
    return events;
  }
}

/**
 * A listener that obeys the protocol, written once so both tiers can use the same one.
 *
 * The unit gate drives it through random sequences and compares it against
 * `PreviewSession.effective`; the story wires the real events to the same three lines. **Neither
 * of them is allowed to be a second opinion about what the protocol means**, which is the rule
 * `mjx-paint` states as *refusing to compare a painter with itself* — a listener written twice,
 * slightly differently, would make a green browser gate evidence about the story rather than about
 * the component.
 */
export function applyPreviewEvent(
  state: { applied: string | undefined },
  event: PreviewEvent,
): void {
  switch (event.kind) {
    case 'preview':
      state.applied = event.value;
      break;
    case 'cancel':
      state.applied = event.restore;
      break;
    case 'commit':
      state.applied = event.value;
      break;
  }
}
