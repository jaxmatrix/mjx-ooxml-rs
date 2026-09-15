import { describe, expect, it } from 'vitest';

import {
  pressedFromAttribute,
  splitButtonPressed,
  type PressedValue,
} from '../src/controls/control-states.ts';
import {
  activateToggle,
  exclusiveAttribute,
  exclusiveScopeOf,
  exclusiveScopeSelector,
  exclusiveSetFromAttribute,
  memberPressed,
  planToggleActivation,
  type ScopedElement,
} from '../src/controls/exclusive-set.ts';

/**
 * **Exclusive sets**, with no browser: the model (`planToggleActivation`), the scope it looks a set up in,
 * and the coordinator that writes and reports (`activateToggle`).
 *
 * The components cannot be constructed here, because each extends `HTMLElement` and this tier is Node. So
 * the members are fakes that read `pressed` through **the same two functions the components call**,
 * `pressedFromAttribute` and `splitButtonPressed`, and that record each event with a snapshot of every
 * member's position at the moment it was dispatched. That snapshot is what proves *move, then report* for
 * a whole set rather than for one control.
 */

type Kind = 'toggle' | 'split' | 'button';

interface Recorded {
  readonly source: string;
  readonly type: string;
  readonly detail: unknown;
  /** Every member's `pressed` attribute when the event was dispatched. */
  readonly positions: Readonly<Record<string, string | null>>;
}

class FakeScope {
  readonly members: FakeMember[] = [];
  readonly queries: string[] = [];

  querySelectorAll(selector: string): FakeMember[] {
    this.queries.push(selector);
    expect(selector).toBe(`[${exclusiveAttribute}]`);
    return this.members.filter((member) => member.getAttribute(exclusiveAttribute) !== null);
  }
}

class FakeMember implements ScopedElement {
  readonly #attributes = new Map<string, string>();

  constructor(
    readonly name: string,
    readonly kind: Kind,
    readonly scope: FakeScope,
    readonly log: Recorded[],
    attributes: Readonly<Record<string, string>> = {},
  ) {
    for (const [key, value] of Object.entries(attributes)) this.#attributes.set(key, value);
    scope.members.push(this);
  }

  getAttribute(name: string): string | null {
    return this.#attributes.get(name) ?? null;
  }

  setAttribute(name: string, value: string): void {
    this.#attributes.set(name, value);
  }

  hasAttribute(name: string): boolean {
    return this.#attributes.has(name);
  }

  /** What each component's `pressed` getter answers. */
  get pressed(): PressedValue | undefined {
    if (this.kind === 'button') return undefined;
    if (this.kind === 'split') return splitButtonPressed(this.hasAttribute('toggle'), this.getAttribute('pressed'));
    return pressedFromAttribute(this.getAttribute('pressed'));
  }

  closest(selectors: string): ParentNode | null {
    expect(selectors).toBe(exclusiveScopeSelector);
    return this.scope as unknown as ParentNode;
  }

  getRootNode(): Node {
    throw new Error('a member inside a tab never falls back to its root node');
  }

  dispatchEvent(event: Event): boolean {
    const positions: Record<string, string | null> = {};
    for (const member of this.scope.members) positions[member.name] = member.getAttribute('pressed');
    this.log.push({ source: this.name, type: event.type, detail: (event as CustomEvent).detail, positions });
    return true;
  }

  /** What a click on the control does: its component calls this with its own position. */
  press(): void {
    const pressed = this.pressed;
    if (pressed === undefined) throw new Error(`${this.name} holds no position and cannot be pressed as a toggle`);
    activateToggle(this as unknown as HTMLElement, pressed);
  }
}

/** The Draw tab's five tools, and Ruler beside them in no set. */
interface DrawToolMembers {
  readonly select: FakeMember;
  readonly lasso: FakeMember;
  readonly pen: FakeMember;
  readonly highlighter: FakeMember;
  readonly eraser: FakeMember;
  readonly ruler: FakeMember;
}

/** Word's Draw tools, in one tab: four toggles and a split Eraser, Select Objects pressed. */
function drawTools(): { scope: FakeScope; log: Recorded[]; members: DrawToolMembers } {
  const scope = new FakeScope();
  const log: Recorded[] = [];
  const set = { exclusive: 'word.draw.write.tools' };
  const members = {
    select: new FakeMember('select', 'toggle', scope, log, { ...set, pressed: '' }),
    lasso: new FakeMember('lasso', 'toggle', scope, log, set),
    pen: new FakeMember('pen', 'toggle', scope, log, set),
    highlighter: new FakeMember('highlighter', 'toggle', scope, log, set),
    eraser: new FakeMember('eraser', 'split', scope, log, { ...set, toggle: '' }),
    // Beside them on the tab, and in no set.
    ruler: new FakeMember('ruler', 'toggle', scope, log, { pressed: 'true' }),
  };
  return { scope, log, members };
}

const positionsOf = (members: DrawToolMembers): Record<string, PressedValue | undefined> =>
  Object.fromEntries(
    (Object.entries(members) as [string, FakeMember][]).map(([name, member]) => [name, member.pressed]),
  );

// ── the model ────────────────────────────────────────────────────────────────

describe('what an exclusive attribute names', () => {
  it('names a set, trimmed', () => {
    expect(exclusiveSetFromAttribute('word.view.document-views')).toBe('word.view.document-views');
    expect(exclusiveSetFromAttribute('  word.view.page-movement ')).toBe('word.view.page-movement');
  });

  it('names nothing when absent or blank, so a blank attribute joins no set', () => {
    expect(exclusiveSetFromAttribute(null)).toBeUndefined();
    expect(exclusiveSetFromAttribute('')).toBeUndefined();
    expect(exclusiveSetFromAttribute('   ')).toBeUndefined();
  });
});

describe('what counts as a member', () => {
  it('reads a position from the pressed property', () => {
    expect(memberPressed({ pressed: 'true' })).toBe('true');
    expect(memberPressed({ pressed: 'mixed' })).toBe('mixed');
    expect(memberPressed({ pressed: 'false' })).toBe('false');
  });

  it('finds none on a plain split button, a button, or a value that is not a position', () => {
    expect(memberPressed({ pressed: undefined })).toBeUndefined();
    expect(memberPressed({})).toBeUndefined();
    expect(memberPressed({ pressed: 'on' })).toBeUndefined();
    expect(memberPressed({ pressed: true })).toBeUndefined();
  });
});

describe('the model: one activation', () => {
  it('outside a set, is the toggle rule and releases nothing', () => {
    const { members } = drawTools();
    const candidates = Object.values(members);
    expect(planToggleActivation(members.ruler, 'true', candidates)).toEqual({ next: 'false', moves: true, releases: [] });
    expect(planToggleActivation(members.ruler, 'false', candidates)).toEqual({ next: 'true', moves: true, releases: [] });
    expect(planToggleActivation(members.ruler, 'mixed', candidates)).toEqual({ next: 'true', moves: true, releases: [] });
  });

  it('presses an unpressed member and releases the member that holds', () => {
    const { members } = drawTools();
    const plan = planToggleActivation(members.pen, 'false', Object.values(members));
    expect(plan.next).toBe('true');
    expect(plan.moves).toBe(true);
    expect(plan.releases.map((member) => member.name)).toEqual(['select']);
  });

  it('keeps the pressed member pressed, and releases nothing', () => {
    const { members } = drawTools();
    expect(planToggleActivation(members.select, 'true', Object.values(members))).toEqual({
      next: 'true',
      moves: false,
      releases: [],
    });
  });

  it('releases a mixed member, and takes a mixed activated member to pressed', () => {
    const { members } = drawTools();
    members.lasso.setAttribute('pressed', 'mixed');
    const plan = planToggleActivation(members.highlighter, 'false', Object.values(members));
    expect(plan.releases.map((member) => member.name)).toEqual(['select', 'lasso']);
    expect(planToggleActivation(members.lasso, 'mixed', Object.values(members)).moves).toBe(true);
  });

  it('never releases a member of another set, a toggle in no set, or a split without toggle', () => {
    const { members, scope, log } = drawTools();
    const other = new FakeMember('vertical', 'toggle', scope, log, { exclusive: 'word.view.page-movement', pressed: 'true' });
    const plainSplit = new FakeMember('plain-split', 'split', scope, log, { exclusive: 'word.draw.write.tools', pressed: 'true' });
    const plan = planToggleActivation(members.pen, 'false', scope.members);
    expect(plan.releases.map((member) => member.name)).toEqual(['select']);
    expect(plan.releases).not.toContain(other);
    expect(plan.releases).not.toContain(plainSplit);
    expect(plan.releases).not.toContain(members.ruler);
  });

  it('repairs a set a host left with two members pressed, on the next press of either', () => {
    const { members } = drawTools();
    members.pen.setAttribute('pressed', 'true');
    const plan = planToggleActivation(members.select, 'true', Object.values(members));
    expect(plan.moves).toBe(false);
    expect(plan.releases.map((member) => member.name)).toEqual(['pen']);
  });
});

// ── the scope ────────────────────────────────────────────────────────────────

describe('where a set is looked up', () => {
  const tab = { tab: true } as unknown as ParentNode;
  const root = { root: true } as unknown as Node;

  it('is the nearest ribbon tab or ribbon, asked for in one selector so the nearer one wins', () => {
    const asked: string[] = [];
    const element: ScopedElement = {
      closest: (selectors) => {
        asked.push(selectors);
        return tab;
      },
      getRootNode: () => root,
    };
    expect(exclusiveScopeOf(element)).toBe(tab);
    expect(asked).toEqual(['mjx-ribbon-tab, mjx-ribbon']);
  });

  it('is the root node when the member is in no ribbon', () => {
    const element: ScopedElement = { closest: () => null, getRootNode: () => root };
    expect(exclusiveScopeOf(element)).toBe(root);
  });

  it('keeps two ribbons on one page apart, although their sets share a name', () => {
    const first = drawTools();
    const second = drawTools();
    first.members.pen.press();
    expect(first.members.select.pressed).toBe('false');
    expect(second.members.select.pressed, 'a press in one ribbon released a tool in another').toBe('true');
    expect(second.log).toEqual([]);
  });
});

// ── the coordinator ──────────────────────────────────────────────────────────

describe('the coordinator: pressing a member of a set', () => {
  it('presses it, releases the one that held, and leaves the rest of the tab alone', () => {
    const { members } = drawTools();
    members.pen.press();
    expect(positionsOf(members)).toEqual({
      select: 'false',
      lasso: 'false',
      pen: 'true',
      highlighter: 'false',
      eraser: 'false',
      ruler: 'true',
    });
  });

  it('moves every attribute first, then reports the released member and then the pressed one', () => {
    const { members, log } = drawTools();
    members.pen.press();
    expect(log.map(({ source, type, detail }) => ({ source, type, detail }))).toEqual([
      { source: 'select', type: 'mjx-change', detail: { pressed: 'false' } },
      { source: 'pen', type: 'mjx-change', detail: { pressed: 'true' } },
    ]);
    for (const event of log) {
      expect(event.positions['select'], `${event.source} reported before Select Objects moved`).toBe('false');
      expect(event.positions['pen'], `${event.source} reported before Pen moved`).toBe('true');
    }
  });

  it('does nothing, and reports nothing, when the member that holds is pressed again', () => {
    const { members, log } = drawTools();
    members.pen.press();
    log.length = 0;
    members.pen.press();
    expect(members.pen.pressed).toBe('true');
    expect(log).toEqual([]);
    // One member always holds, however the set is pressed.
    for (const name of ['lasso', 'highlighter', 'select', 'select', 'eraser', 'eraser', 'pen'] as const) {
      members[name].press();
      const holding = Object.entries(positionsOf(members)).filter(
        ([member, pressed]) => member !== 'ruler' && pressed === 'true',
      );
      expect(holding.map(([member]) => member)).toEqual([name]);
    }
  });

  it('treats a split button with a toggle face as a member in both directions', () => {
    const { members, log } = drawTools();
    members.eraser.press();
    expect(members.eraser.pressed).toBe('true');
    expect(members.select.pressed).toBe('false');
    log.length = 0;
    members.highlighter.press();
    expect(members.eraser.pressed).toBe('false');
    expect(log.map(({ source, detail }) => ({ source, detail }))).toEqual([
      { source: 'eraser', detail: { pressed: 'false' } },
      { source: 'highlighter', detail: { pressed: 'true' } },
    ]);
  });

  it('outside a set, flips and reports once, without looking for siblings', () => {
    const { members, log, scope } = drawTools();
    members.ruler.press();
    expect(members.ruler.pressed).toBe('false');
    expect(members.select.pressed).toBe('true');
    expect(log.map(({ source, detail }) => ({ source, detail }))).toEqual([{ source: 'ruler', detail: { pressed: 'false' } }]);
    expect(scope.queries).toEqual([]);
  });
});
