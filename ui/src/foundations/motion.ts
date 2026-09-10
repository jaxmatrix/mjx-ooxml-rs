/**
 * The motion vocabulary — the easing tokens with their intended use **recorded and checked**.
 *
 * `DESIGN_TOKENS.md` §4:
 *
 * > **`--ease-spring` overshoots** (`cubic-bezier(.34, 1.56, .64, 1)`). Good for a panel or sheet
 * > entering; wrong for anything attached to a document object, where overshoot reads as
 * > imprecision. Use `--ease-ink` for selection, handles and canvas motion.
 *
 * That paragraph has been true and unenforced since MJXOFF-156. Here it becomes a test. Every role
 * below declares whether it is **attached to a document object**, and `tests/foundations.test.ts`
 * fails if any role that is uses the overshooting easing. A later child adding a selection-handle
 * animation with `--ease-spring` therefore fails a gate rather than shipping motion that reads as
 * imprecision — which is the only way a note in a document survives fifteen children.
 *
 * `tests/browser/foundations.spec.ts` then reads the *computed* `transition-timing-function` back
 * out of a browser and compares it to the generated token's own cubic-bezier, so a role that names
 * the right token in this file and resolves to the wrong curve in the cascade is caught as well.
 *
 * ## Node-importable
 *
 * Data and strings only.
 */

/** The roles, in the order the catalogue shows them. */
export const motionRoleNames = [
  'panelEnter',
  'sheetEnter',
  'surfaceSettle',
  'selection',
  'documentObject',
] as const;

/** One of the five roles. */
export type MotionRole = (typeof motionRoleNames)[number];

/** Everything a role fixes, plus the one fact the gate reads. */
export interface MotionRoleSpec {
  /** The easing's dotted token path, so a gate can look it up in the generated table. */
  readonly easingToken: 'ease.ink' | 'ease.outSoft' | 'ease.spring';
  /** The `var()` a stylesheet writes. */
  readonly easing: string;
  /** The duration's `var()`. One duration token exists; a role that needs another states a ratio. */
  readonly duration: string;
  /**
   * Whether the thing being animated is attached to an object in the user's document.
   *
   * **This is the field the gate reads.** True means overshoot is forbidden: a selection handle
   * that springs past its anchor and comes back has told the user their object moved.
   */
  readonly attachedToDocumentObject: boolean;
  readonly use: string;
}

export const motionRoles: Readonly<Record<MotionRole, MotionRoleSpec>> = {
  panelEnter: {
    easingToken: 'ease.spring',
    easing: 'var(--ease-spring)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'A task pane or an inspector docking. Overshoot reads as the panel arriving with weight.',
  },
  sheetEnter: {
    easingToken: 'ease.spring',
    easing: 'var(--ease-spring)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'A sheet or a dialog entering over the content.',
  },
  surfaceSettle: {
    easingToken: 'ease.outSoft',
    easing: 'var(--ease-out-soft)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'Hover, press and disclosure on chrome: a decelerating move with no overshoot at all.',
  },
  selection: {
    easingToken: 'ease.ink',
    easing: 'var(--ease-ink)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: true,
    use: 'A selection rectangle, a handle, an alignment guide. §4 names these exactly.',
  },
  documentObject: {
    easingToken: 'ease.ink',
    easing: 'var(--ease-ink)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: true,
    use: 'A shape, a row or a column moving on the canvas. Overshoot here reads as imprecision.',
  },
};

/** The easing token no document-attached role may use, named once so the gate cannot mistype it. */
export const overshootingEasingToken = 'ease.spring';

/** `panelEnter` → `.mjx-motion-panel-enter`. */
export function motionRoleClass(role: MotionRole): string {
  return `mjx-motion-${role.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`)}`;
}

export const motionCss = `
${motionRoleNames
  .map((role) => {
    const spec = motionRoles[role];
    return `:where(.${motionRoleClass(role)}) {
  transition-duration: ${spec.duration};
  transition-timing-function: ${spec.easing};
}`;
  })
  .join('\n')}

/* A person who has asked their operating system to stop moving things has asked this platform too.
 * Stated here, once, rather than in each of fifteen components. */
@media (prefers-reduced-motion: reduce) {
  :where(${motionRoleNames.map((role) => `.${motionRoleClass(role)}`).join(', ')}) {
    transition-duration: 0s;
    animation-duration: 0s;
  }
}
`;
