/**
 * **Exclusive sets**: toggles of which one holds at a time — exactly one, or at most one where the set says so.
 *
 * ```html
 * <mjx-toggle-button exclusive="word.view.document-views" label="Print Layout" pressed></mjx-toggle-button>
 * <mjx-toggle-button exclusive="word.view.document-views" label="Web Layout"></mjx-toggle-button>
 * <mjx-split-button toggle exclusive="word.draw.write.tools" label="Eraser"></mjx-split-button>
 * <mjx-toggle-button exclusive="word.background-removal.refine" exclusive-allows-none label="Mark Areas to Keep"></mjx-toggle-button>
 * ```
 *
 * Office holds exactly one of some sets of toggles: Word's five document views, its two page movements,
 * and the Draw tab's ink tools (Select Objects, Lasso Select, Pen, Highlighter, and Eraser, which is a
 * split button with a toggle face). A toggle that knows no siblings draws two of them pressed at once.
 *
 * ## The decision
 *
 * **A member declares its set by name, in an `exclusive` attribute, and the member that is activated
 * releases the others itself.** The set is looked up in the activated member's **scope**: its nearest
 * `<mjx-ribbon-tab>`, else its nearest `<mjx-ribbon>`, else its root node. Four rules follow:
 *
 * 1. **Activating an unpressed member presses it and releases every other member that holds**, whether
 *    that member holds `true` or `mixed`.
 * 2. **Activating the pressed member keeps it pressed.** Nothing about it moves and it reports nothing,
 *    so one member always stays pressed. A person cannot empty a set, as in Office, where pressing the
 *    view Word is already in keeps that view. **Unless the activated member carries
 *    `exclusive-allows-none`**: then the press releases it, as an ordinary toggle's does, and the set is
 *    left holding nothing. See *A set that may hold none* below.
 * 3. **Attributes move first, then every member that moved reports.** Each released member emits
 *    `mjx-change` with `detail.pressed` `false`, and then the activated member emits its own. That is
 *    the toggle's *move, then report* order, applied to the whole set: no listener can read a set in
 *    which two members hold.
 * 4. **A member is anything with a `pressed` position**: `<mjx-toggle-button>`, or
 *    `<mjx-split-button toggle>`. A plain split button, or any other element carrying `exclusive`, holds
 *    no position, so nothing releases it. `tests/ribbons.test.ts` refuses a host binding that makes one.
 *
 * The census declares the set on each command (`RibbonCommand.exclusive`), `renderCommand` carries it
 * onto the generic toggle, and a host binding (Eraser's split button) writes it by hand. A gate there
 * holds the two spellings together and requires each set to start with exactly one member pressed.
 *
 * ## A set that may hold none
 *
 * **Office's Background Removal tab holds at most one of Mark Areas to Keep and Mark Areas to Remove**, and
 * that is a different set from Word's views. Each arms a pencil. Pressing one while the other is armed
 * swaps the pencil, and pressing the armed one again puts the pencil down and gives back the ordinary
 * pointer, which is also where the tab starts. **The pointer is not a command on the tab**, where the Draw
 * tab's is (Select Objects), so a set of exactly one would leave a person no press that takes them back to
 * it. `GUESS:` the release on a second press, from Office's other arm-a-gesture commands (Format Painter,
 * Draw Table), which put the gesture down the same way.
 *
 * So a member may carry **`exclusive-allows-none`**, a boolean attribute. Rules 1, 3 and 4 are unchanged.
 * Rule 2 reads the attribute on the **activated** member: present, and the member that holds is released by
 * its own press, moving and reporting as any toggle does; absent, and rule 2 holds as written. Every member
 * of one set carries it or none does, and a set that carries it starts with **at most** one member pressed;
 * `tests/ribbons.test.ts` holds both. It changes nothing a member's component does, because
 * `planToggleActivation` reads it from the element it is handed, so a toggle button and a split button's
 * toggle face take part alike.
 *
 * Rejected:
 *
 * - **Two independent toggles.** Pressing Mark Areas to Remove with Mark Areas to Keep armed would draw
 *   both pressed, which is the defect this module exists to remove.
 * - **A set of exactly one with Mark Areas to Keep pressed at the start.** Office arms no pencil on entry,
 *   and a press could never put the pencil down again.
 * - **A third, invisible member standing for the pointer.** A command the ribbon does not draw is a
 *   member nobody can press, and the census would declare a command Office does not have.
 * - **Declaring it once on the set rather than on every member.** There is no element for the set; see the
 *   wrapper alternative below. Each member already carries the set's name the same way.
 *
 * ## ARIA: still toggle buttons
 *
 * Every member keeps `aria-pressed`, written for all three positions, and a released member's moves with
 * its attribute. Office exposes these commands as toggle buttons, not radios, and the ribbon's keyboard
 * model is the reason to follow it. A radio group is one tab stop with arrow keys moving *and selecting*
 * within it. A ribbon group already has its own focus order, and a radio group cannot span it: the Draw
 * tab's set is four toggles and a split button, and a collapsed Write group puts Select Objects in the
 * survivor row and the other four in the popup. Enter and Space reach every member through the native
 * `click` a `<button>` produces, so the keyboard needs nothing added.
 *
 * ## Alternatives rejected
 *
 * - **`role="radio"` in a `radiogroup`.** See the ARIA section. A radiogroup also needs one container
 *   around its members, and a ribbon group's commands are slotted one by one and moved between slots
 *   when the group collapses, so there is no element that could be that container.
 * - **A coordinator listening for `mjx-change` on the tab or the ribbon.** By the time a listener hears
 *   the event, the member has already moved and reported. Re-pressing the pressed member would therefore
 *   release it, announce *not pressed*, and have to be pressed again by the coordinator: a second write,
 *   a second event, and two announcements for a press that changed nothing. The rule is only right if
 *   the member knows it is in a set *before* it moves.
 * - **A wrapper element such as `<mjx-exclusive-set>`.** `<mjx-ribbon-group>` assigns its direct children
 *   to slots and reads their `slot` attribute to place survivors, so a wrapper would be one command to the
 *   group and would break the collapse. The Draw tab's set also spans two element types.
 * - **A module-level registry keyed by set name.** Storybook's docs page draws every story of a file on
 *   one page, so three Word ribbons share a document. A global registry would let a press in one release
 *   a member in another. Scoping by the DOM keeps each ribbon, and each tab, its own.
 * - **A binding per set in each host.** Two hosts times five sets, each wiring `@mjx-change`, for
 *   behaviour Office has on every surface. The generic toggle has no override to wire at all, and the
 *   census would state exclusivity only in prose.
 * - **Inferring exclusivity from the group.** Word's Show group is three independent checkboxes, and its
 *   Window group's View Side by Side and Synchronous Scrolling hold together. Exclusivity is declared.
 * - **Enforcing it on every write of `pressed`, as a radio input's `checked` does.** Attribute callbacks
 *   fire while a template is still being cloned, before the element is inside its tab, so the scope
 *   would be the template fragment and the result would depend on upgrade order. A write that silently
 *   rewrites other elements' attributes is also a second writer a host cannot see. A host that sets
 *   `pressed` directly writes one attribute, as with any other.
 *
 * `GUESS:` pressing the pressed member reports nothing. Office's Pen, pressed again, opens the pen's
 * options; that is a menu, and this project's menus are the host's.
 */

import { controlEvents, emitControlEvent } from './control-element.ts';
import { isPressedValue, nextPressed, type PressedValue } from './control-states.ts';

/** The attribute a member declares its set in. */
export const exclusiveAttribute = 'exclusive';

/**
 * Where a set is looked up: the nearest ribbon tab, else the nearest ribbon.
 *
 * A tab first, because no set spans two tabs, and a ribbon draws every tab at once with only one shown.
 */
export const exclusiveScopeSelector = 'mjx-ribbon-tab, mjx-ribbon';

/**
 * The boolean attribute that lets a set hold none: the member that holds is released by its own press. See
 * *A set that may hold none* in the module note.
 */
export const exclusiveAllowsNoneAttribute = 'exclusive-allows-none';

/** What an `exclusive` attribute names: a set, or nothing when it is absent or blank. */
export function exclusiveSetFromAttribute(declared: string | null): string | undefined {
  const trimmed = declared?.trim() ?? '';
  return trimmed === '' ? undefined : trimmed;
}

/** What the coordinator reads and writes on a member. `HTMLElement` satisfies it. */
export interface ExclusiveMember {
  getAttribute(name: string): string | null;
  setAttribute(name: string, value: string): void;
}

/** The set a member declares, or `undefined`. */
export function exclusiveSetOf(member: ExclusiveMember): string | undefined {
  return exclusiveSetFromAttribute(member.getAttribute(exclusiveAttribute));
}

/**
 * Whether a member's set may hold none, read from its `exclusive-allows-none` attribute. Presence is the
 * answer, as for any boolean attribute, so `exclusive-allows-none="false"` still allows none.
 */
export function exclusiveAllowsNone(member: ExclusiveMember): boolean {
  return member.getAttribute(exclusiveAllowsNoneAttribute) !== null;
}

/**
 * The position a member holds, read from its `pressed` property, or `undefined` when it holds none.
 *
 * The property rather than the attribute, because each component already owns the reading. A split button
 * without `toggle` answers `undefined`, and so does an element that is not a toggle at all.
 */
export function memberPressed(member: object): PressedValue | undefined {
  const declared = (member as { readonly pressed?: unknown }).pressed;
  // `isPressedValue` compares `String(value)`, so a boolean `true` would pass it. A component's `pressed`
  // is always one of the three strings, and anything else on an element is not a toggle's position.
  return typeof declared === 'string' && isPressedValue(declared) ? declared : undefined;
}

/** What one activation of a toggle does. */
export interface ToggleActivation<Member> {
  /** The activated toggle's position afterwards. */
  readonly next: PressedValue;
  /** Whether the activated toggle's position changes, and so whether it reports. */
  readonly moves: boolean;
  /** The other members of its set that hold, and are released. Empty outside a set. */
  readonly releases: readonly Member[];
}

/**
 * **The model**: what activating `activated`, at `current`, does to it and to `candidates`.
 *
 * Outside a set, the toggle's own rule: `nextPressed`, and it always moves. Inside a set, the activated
 * member ends pressed, and it moves only if it was not already. Every other candidate that declares the
 * same set and holds `true` or `mixed` is released. `releases` is computed even when the activated member
 * was already pressed, so a set a host has left with two members pressed is repaired by the next press.
 *
 * **Inside a set that allows none**, the one difference: an activated member already at `true` ends at
 * `false` and moves. Others that hold are still released, so the repair still happens and the set ends
 * empty. A member at `mixed` still goes to `true`, as `nextPressed` takes it.
 */
export function planToggleActivation<Member extends ExclusiveMember>(
  activated: Member,
  current: PressedValue,
  candidates: Iterable<Member>,
): ToggleActivation<Member> {
  const set = exclusiveSetOf(activated);
  if (set === undefined) return { next: nextPressed(current), moves: true, releases: [] };
  const releasesItself = current === 'true' && exclusiveAllowsNone(activated);
  const releases: Member[] = [];
  for (const candidate of candidates) {
    if (candidate === activated || exclusiveSetOf(candidate) !== set) continue;
    const pressed = memberPressed(candidate);
    if (pressed === 'true' || pressed === 'mixed') releases.push(candidate);
  }
  if (releasesItself) return { next: 'false', moves: true, releases };
  return { next: 'true', moves: current !== 'true', releases };
}

/** What `exclusiveScopeOf` needs of the activated element. `Element` satisfies it. */
export interface ScopedElement {
  closest(selectors: string): ParentNode | null;
  getRootNode(): Node;
}

/**
 * The node a member's set is looked up in: its nearest ribbon tab or ribbon, else its root node.
 *
 * The root node is a document or a shadow root when the element is connected, and the topmost ancestor
 * when it is not. Any of them can be queried, so a toggle outside a ribbon still finds its set.
 */
export function exclusiveScopeOf(element: ScopedElement): ParentNode {
  return element.closest(exclusiveScopeSelector) ?? (element.getRootNode() as unknown as ParentNode);
}

/**
 * **The coordinator**: activate a toggle-shaped control, releasing its set's other members.
 *
 * Both `<mjx-toggle-button>` and `<mjx-split-button toggle>` call this from their activation, so *move,
 * then report* is written once for a toggle inside a set and outside one. See the module note for the
 * order and for why nothing reports when the pressed member is pressed again.
 */
export function activateToggle(host: HTMLElement, current: PressedValue): void {
  const candidates =
    exclusiveSetOf(host) === undefined
      ? []
      : Array.from(exclusiveScopeOf(host).querySelectorAll<HTMLElement>(`[${exclusiveAttribute}]`));
  const plan = planToggleActivation(host, current, candidates);
  if (plan.moves) host.setAttribute('pressed', plan.next);
  for (const member of plan.releases) member.setAttribute('pressed', 'false');
  for (const member of plan.releases) {
    emitControlEvent(member, controlEvents.change, { pressed: 'false' });
  }
  if (plan.moves) emitControlEvent(host, controlEvents.change, { pressed: plan.next });
}
