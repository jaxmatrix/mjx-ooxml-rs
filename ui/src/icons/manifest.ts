/**
 * The icon subset request — **the only list that decides what reaches a bundle.**
 *
 * MJXOFF-181 chose [Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons):
 * MIT licensed, no attribution required, authored by Microsoft, and therefore *Office's own icon
 * language*, so command recognition is as high as it can be. The package ships **20,679** SVG
 * files. The ticket's own words about the trap that matters here:
 *
 * > *"Icons render and typography is applied"* is satisfied by a build that ships all 19,757
 * > icons […] **So the gates are a bundle-size assertion and a literal-value lint.**
 *
 * ## How the subset is kept a subset
 *
 * Four things hold it, and each catches a different way of losing it:
 *
 * | Failure | Caught by |
 * |---|---|
 * | Somebody imports `@fluentui/svg-icons` from a shipped module | ESLint (`no-restricted-imports`, `eslint.config.js`) |
 * | The committed `generated.ts` stops agreeing with this list | `npm run icons:check` |
 * | An icon reaches the built bundle by any other route | `tests/browser/icons.spec.ts` — the id set in `storybook-static/` must **equal** this list |
 * | The gate itself stops being able to reject | `tests/icon-subset.test.ts`, which measures the **real, whole** icon set and watches the budget refuse it |
 *
 * The third is the bundle-size assertion the ticket asks for, and it asserts *equality* rather
 * than a ceiling: a subset that silently shrank is as much a defect as one that grew, because it
 * means a component is referencing an icon that is not there.
 *
 * ## Node-importable, deliberately
 *
 * No DOM class is defined in this file, for the reason `src/harness/presets.ts` states: the
 * browser tier's specs run in Node and cannot import a module that evaluates
 * `class extends HTMLElement` at module scope. The generator, the drift gate and the bundle gate
 * all read this list from Node.
 *
 * ## Scope
 *
 * MJXOFF-181 §*Not in scope*: **icon *choice* per command belongs to the child that builds that
 * command's control.** What is below is the starting vocabulary a catalogue needs before any
 * control exists — the formatting commands, the history pair, the file verbs, the disclosure
 * chevrons, the status marks, and one icon carried at every size so the size ladder has something
 * to render. A later child adds a row here and regenerates; it does not reach into
 * `generated.ts`.
 */

/** The sizes `<mjx-icon>` renders at. Fluent draws each size separately — this is not scaling. */
export const iconSizes = [16, 20, 24, 32, 48] as const;

/** One of the five sizes. */
export type IconSize = (typeof iconSizes)[number];

/**
 * The two variants a chrome needs.
 *
 * `regular` is the resting drawing; `filled` is Fluent's own convention for a *selected* or
 * *active* state, which is why a toggle command requests both and a one-shot command requests only
 * `regular`.
 */
export const iconVariants = ['regular', 'filled'] as const;

/** One of the two variants. */
export type IconVariant = (typeof iconVariants)[number];

/** One row of the request: an icon, and exactly the sizes and variants that are wanted. */
export interface IconRequest {
  /** The Fluent icon's name, hyphenated. `text_bold_20_regular.svg` is `text-bold`. */
  readonly name: string;
  /** Every size this icon is wanted at. Asking for a size nothing uses is asking for dead bytes. */
  readonly sizes: readonly IconSize[];
  /** Every variant this icon is wanted in. */
  readonly variants: readonly IconVariant[];
  /** Why the catalogue needs it. A row with no reason is a row nobody can delete later. */
  readonly why: string;
}

/** The subset. Adding a row and running `npm run icons:subset` is the whole workflow. */
export const iconRequests: readonly IconRequest[] = [
  // ── the size ladder ────────────────────────────────────────────────────────
  {
    name: 'document',
    sizes: [16, 20, 24, 32, 48],
    variants: ['regular', 'filled'],
    why: 'The size-ladder exemplar: the one icon carried at every size, so Foundations/Icons can render the whole ladder rather than one size at one weight.',
  },

  // ── character formatting: toggles, so both variants ────────────────────────
  {
    name: 'text-bold',
    sizes: [16, 20, 24],
    variants: ['regular', 'filled'],
    why: 'Character formatting. A toggle, so it needs the filled drawing for its selected state.',
  },
  {
    name: 'text-italic',
    sizes: [16, 20, 24],
    variants: ['regular', 'filled'],
    why: 'Character formatting. A toggle.',
  },
  {
    name: 'text-underline',
    sizes: [16, 20, 24],
    variants: ['regular', 'filled'],
    why: 'Character formatting. A toggle.',
  },

  // ── paragraph alignment: a radio group, so both variants ───────────────────
  {
    name: 'text-align-left',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: 'Paragraph alignment. One of a radio group, whose chosen member is drawn filled.',
  },
  {
    name: 'text-align-center',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: 'Paragraph alignment.',
  },
  {
    name: 'text-align-right',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: 'Paragraph alignment.',
  },

  // ── one-shot commands: regular only ────────────────────────────────────────
  {
    name: 'arrow-undo',
    sizes: [20, 24],
    variants: ['regular'],
    why: 'History. A one-shot command is never in a selected state, so it needs no filled drawing.',
  },
  { name: 'arrow-redo', sizes: [20, 24], variants: ['regular'], why: 'History.' },
  { name: 'save', sizes: [20], variants: ['regular'], why: 'File verb.' },
  { name: 'folder-open', sizes: [20], variants: ['regular'], why: 'File verb.' },
  {
    name: 'add',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Insertion, in a toolbar and inline in a list.',
  },
  { name: 'delete', sizes: [16, 20], variants: ['regular'], why: 'Removal.' },
  {
    name: 'search',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Find, in the chrome and inline in a filter field.',
  },
  { name: 'settings', sizes: [20], variants: ['regular'], why: 'Options.' },
  {
    name: 'table',
    sizes: [20],
    variants: ['regular'],
    why: 'The shared table command, which Word and Excel both need.',
  },
  { name: 'slide-layout', sizes: [20], variants: ['regular'], why: "PowerPoint's layout command." },
  {
    name: 'comment',
    sizes: [16, 20],
    variants: ['regular', 'filled'],
    why: 'Review. Filled marks a thread with unread replies.',
  },

  // ── disclosure and dismissal: dense, so 16 as well as 20 ───────────────────
  {
    name: 'chevron-down',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Disclosure, on a menu button and in a dense inspector row.',
  },
  {
    name: 'chevron-right',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Disclosure, in a tree and a submenu.',
  },
  {
    name: 'dismiss',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Closing a panel, a chip or a dialog.',
  },
  {
    name: 'checkmark',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Confirmation, and the chosen member of a menu radio group.',
  },

  // ── status: filled too, because a status mark reads at a glance when solid ─
  {
    name: 'warning',
    sizes: [16, 20],
    variants: ['regular', 'filled'],
    why: 'Status. Filled is the drawing a warning actually wants.',
  },
  { name: 'info', sizes: [16, 20], variants: ['regular', 'filled'], why: 'Status.' },
];

/**
 * The sentinel every generated icon id carries.
 *
 * It is what makes the bundle gate *precise*: a minified bundle is grepped for this prefix, and
 * the set of ids found must equal the set this manifest asks for. A prefix distinctive enough not
 * to occur by accident is the whole requirement, and `mjx-` plus the vendor's name is that.
 */
export const iconIdPrefix = 'mjx-fluent:';

/** `text-bold` at 20, filled → `mjx-fluent:text-bold-20-filled`. */
export function iconId(name: string, size: IconSize, variant: IconVariant): string {
  return `${iconIdPrefix}${name}-${String(size)}-${variant}`;
}

/** `text-bold` at 20, filled → `text_bold_20_filled.svg`, the file the vendor ships. */
export function fluentFileName(name: string, size: IconSize, variant: IconVariant): string {
  return `${name.replaceAll('-', '_')}_${String(size)}_${variant}.svg`;
}

/** Every id the manifest asks for, in a stable order. */
export function requestedIconIds(): readonly string[] {
  const ids: string[] = [];
  for (const request of iconRequests) {
    for (const size of request.sizes) {
      for (const variant of request.variants) ids.push(iconId(request.name, size, variant));
    }
  }
  return ids;
}
