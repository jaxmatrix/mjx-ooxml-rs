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
  {
    name: 'save',
    sizes: [20, 24],
    variants: ['regular'],
    why: "File verb. ⚠ **24 was added by the ribbon programme's unit 1**, which made Save the headline command of the File tab's Save group and therefore a `size=\"large\"` button. A large button draws at 24 and this row carried 20 alone, so it would have been the blank square `tests/ribbons.test.ts` exists to refuse — and did refuse, before a pixel was rendered.",
  },
  { name: 'folder-open', sizes: [20, 24], variants: ['regular'], why: 'File verb.' },
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
    name: 'chevron-up',
    sizes: [16],
    variants: ['regular'],
    why: "A gallery strip's scroll-back affordance (MJXOFF-185). 16 alone, and the pair with chevron-down: the two are one control drawn twice, and a strip that could only scroll forward would be a strip that loses the item a person just passed.",
  },
  {
    name: 'chevron-double-down',
    sizes: [16],
    variants: ['regular'],
    why: "A gallery strip's *expand* affordance (MJXOFF-185), and deliberately not a second chevron-down. Office draws the More button as a chevron with a bar over it precisely because scroll-one-row and show-everything sit next to each other and must not be the same picture; two identical glyphs stacked in one rail is a control nobody can aim at.",
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
    why: 'Confirmation, and the on state of a menuitemcheckbox.',
  },
  {
    name: 'subtract',
    sizes: [16, 20],
    variants: ['regular'],
    why: "The indeterminate position of a tri-state checkbox (MJXOFF-186). A dash rather than a faint or half-drawn tick, and that is the whole reason it is a separate drawing: `mixed` must not read as a weaker `checked`, which is the same argument `controlStateSpecs.mixed` makes about the fill. ⚠ **20 was added by the ribbon programme's unit 0, as a defect fix rather than a new command.** Word's and PowerPoint's Numbering and Excel's Decrease decimal have asked for `subtract` at the size a small ribbon button draws since the shells were written, and 16 was the only drawing here — so all three rendered an empty square, silently, exactly as `<mjx-icon>`'s own documentation warns. `tests/ribbons.test.ts` is the gate that found it and the gate that keeps it found.",
  },
  {
    name: 'radio-button',
    sizes: [16],
    variants: ['filled'],
    why: "The chosen member of a menuitemradio group (MJXOFF-184). A check mark would have done the job and would have said the wrong thing: a check says *this is on*, a bullet says *this one, of these*, and a menu that drew both states with one glyph would hide the difference between a checkbox and a radio from everyone who cannot read the role. 16 alone, and filled alone: it is a mark rather than a command's icon, and an unchecked radio draws nothing at all.",
  },

  // ── status: filled too, because a status mark reads at a glance when solid ─
  {
    name: 'warning',
    sizes: [16, 20],
    variants: ['regular', 'filled'],
    why: 'Status. Filled is the drawing a warning actually wants.',
  },
  { name: 'info', sizes: [16, 20], variants: ['regular', 'filled'], why: 'Status.' },

  // ── the ribbon's own marks (MJXOFF-182) ───────────────────────────
  {
    name: 'cut',
    sizes: [20, 24],
    variants: ['regular'],
    why: "The clipboard verb (MJXOFF-194). A contextual action bar's shared four are cut, copy, paste and delete, and the first three had no drawing in this subset at all — Office's own three most-used commands, reachable on a phone only by name.",
  },
  { name: 'copy', sizes: [20, 24], variants: ['regular'], why: 'Clipboard verb.' },
  {
    name: 'clipboard-paste',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Clipboard verb. Fluent's own name for paste, and deliberately not the bare clipboard glyph: a clipboard says *the clipboard*, and the arrow into it says *put this there*.",
  },
  {
    name: 'more-horizontal',
    sizes: [20, 24],
    variants: ['regular'],
    why: "The mobile bars' overflow control (MJXOFF-194). Horizontal rather than vertical because the rail it overflows is horizontal, and a vertical ellipsis beside a horizontal row reads as a menu for the row rather than as more of it.",
  },
  {
    name: 'arrow-down-right',
    sizes: [16, 20],
    variants: ['regular'],
    why: "The dialog launcher's corner mark — Office's own glyph for *this group has a full dialog*, drawn at 16 because it is a mark rather than a command's icon. ⚠ **20 was added by the ribbon programme's unit 0, for the same reason `subtract` gained one**: Excel's Wrap Text and Sort & Filter are small ribbon buttons naming this icon, and a small button draws at 20, so both were blank squares.",
  },

  // ── File (Word, PowerPoint, Excel) ──────────────────────────────────────────
  //
  // The ribbon programme's unit 1: the File tab of all three applications, which decision 1 of the
  // approved plan makes an ordinary ribbon tab whose groups are the backstage destinations — Info,
  // Open, Save, Print, Share, Export, Help.
  //
  // Two rules shaped every row below and both are worth stating once here rather than twenty-five
  // times underneath. **A size nothing draws is dead bytes**, so a command's icon asks for 20 and
  // only a `size="large"` command additionally asks for 24; the seven that do are the headline of
  // their group. And **a near-miss is worse than nothing**, which is `<mjx-icon>`'s own argument —
  // a person acts on a glyph — so four commands on the File tab carry no icon at all: Properties,
  // Package Presentation for CD, Create Handouts and Publish to Power BI. Fluent has no honest
  // drawing for any of them, and the alternative was a picture that would send somebody to the
  // wrong page.
  {
    name: 'document-lock',
    sizes: [20],
    variants: ['regular'],
    why: "Protect Document / Presentation / Workbook — the Info group's first command in all three applications. A document with a padlock rather than a bare `shield`: a shield says *this is defended*, and what the command actually does is put a lock on this one file. ⚠ **20 alone, and that is a rendering finding rather than a preference.** It was `size=\"large\"` and asked for 24 until the tab was looked at: a large button is bounded by `largeControlWidthUnits` so its label wraps to two lines inside about eighty pixels, and *Protect Document* came out as `Protect Docume…`. A truncated command is a command nobody can read, so the three Protect verbs are small — and the 24-pixel drawing nothing would have used went with them.",
  },
  {
    name: 'document-search',
    sizes: [20],
    variants: ['regular'],
    why: "Check for Issues — Office's Inspect Document, which is a magnifier over a document in Office's own icon language too. Distinct from the bare `search` this subset already carries, and deliberately: `search` is *find text in this document*, which is a different command on a different tab.",
  },
  {
    name: 'history',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Two commands, and they are the same idea twice rather than a collision. Open's **Recent** is `size=\"large\"` and needs 24; Info's **Manage Document** — Office's recover-unsaved-versions command — is small and needs 20. Both are a clock in Office as well, because both mean *what this file was before now*.",
  },
  {
    name: 'people',
    sizes: [20],
    variants: ['regular'],
    why: 'Shared with Me, on the Open group. Two figures rather than one: the place is defined by there being somebody else in it.',
  },
  {
    name: 'cloud',
    sizes: [20],
    variants: ['regular'],
    why: "OneDrive, on the Open group. The plain cloud and not `cloud-arrow-up`: this is a *place to open from*, and an upload arrow would say the button sends something.",
  },
  {
    name: 'desktop',
    sizes: [20],
    variants: ['regular'],
    why: 'This PC, on the Open group — the other half of the pair with `cloud`, and the pair is the whole point of both rows: a person reads *here* against *there*.',
  },
  {
    name: 'save-edit',
    sizes: [20],
    variants: ['regular'],
    why: "Save As. The save glyph with a pencil on it, which is Fluent's own way of saying *save under a name you choose* — and the reason Save, Save As and Save a Copy can sit in one group without becoming three identical squares.",
  },
  {
    name: 'save-copy',
    sizes: [20],
    variants: ['regular'],
    why: 'Save a Copy. Fluent draws it as the save glyph doubled, which is exactly what the command means and exactly how it differs from Save As.',
  },
  {
    name: 'arrow-sync',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "AutoSave — the File tab's one toggle, so both variants: a toggle draws filled when it is pressed, and a regular-only request is a control that goes blank the moment somebody turns it on. The circular arrows are Office's own AutoSave mark.",
  },
  {
    name: 'print',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Print — the headline of the Print group in Word and PowerPoint, `size=\"large\"`, hence 24 as well as 20. Excel has no Print group here; see `dev/ribbons/census.ts` on the census's missing `TabPrint` row.",
  },
  {
    name: 'share',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Share — the headline of the Share group in all three applications, `size=\"large\"`.",
  },
  {
    name: 'mail',
    sizes: [20],
    variants: ['regular'],
    why: 'Email, on the Share group. The envelope, which is the one glyph in this block nobody has to learn.',
  },
  {
    name: 'link',
    sizes: [20],
    variants: ['regular'],
    why: 'Get a Link, on the Share group — the command that produces a URL rather than sending a file, and the chain is what distinguishes the two.',
  },
  {
    name: 'presenter',
    sizes: [20],
    variants: ['regular'],
    why: "Present Online, on Word's and PowerPoint's Share group. A figure beside a screen, which is what the command does and what Office draws for it.",
  },
  {
    name: 'slide-multiple',
    sizes: [20],
    variants: ['regular'],
    why: "Publish Slides — PowerPoint's own Share command, which uploads individual slides to a library rather than the deck as a file. Slides in the plural is the distinction the glyph carries.",
  },
  {
    name: 'document-pdf',
    sizes: [20],
    variants: ['regular'],
    why: "Create PDF/XPS Document — the Export group's first command in all three applications. Fluent draws the format's own name on the page, which is the one case where a glyph can say a file format without ambiguity. 20 alone for the reason `document-lock` gives at length: a four-word label cannot be drawn inside a large button's width, so the command is small and nothing draws it at 24.",
  },
  {
    name: 'arrow-swap',
    sizes: [20],
    variants: ['regular'],
    why: "Change File Type, on the Export group. Two arrows exchanging: the command does not move the document anywhere, it exchanges one format for another, and `document-arrow-right` would have said *send this somewhere*.",
  },
  {
    name: 'video',
    sizes: [20],
    variants: ['regular'],
    why: "Create a Video — PowerPoint's Export group, and one of the two places PowerPoint's File tab genuinely differs from Word's.",
  },
  {
    name: 'question-circle',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Help — the headline of the Help group in all three applications, `size=\"large\"`. ⚠ **The 24-pixel drawing does not render anywhere in the catalogue, and it is still not dead bytes.** The Help group is `ancillary`, and `groupPriorities.ancillary.reduceAtOrBelow` is 1600 — the harness's own widest container — so inside Storybook that group is reduced at every width and draws its icons at 20. A shell on a real 2560-pixel monitor is not inside the harness, the group is full there, and a 20-only request would be a blank square nobody could reproduce in the catalogue. This row is what stops that, and this sentence is why a reader who greps for the 24 and finds it unused should leave it alone.",
  },
  {
    name: 'person-support',
    sizes: [20],
    variants: ['regular'],
    why: 'Contact Support, on the Help group. A person with a headset — the command reaches a human, which is exactly what separates it from Help.',
  },
  {
    name: 'person-feedback',
    sizes: [20],
    variants: ['regular'],
    why: "Feedback, on the Help group. Fluent's own pairing with `person-support`, and the pair is what keeps *ask for help* and *tell us something* from being one picture.",
  },
  {
    name: 'megaphone',
    sizes: [20],
    variants: ['regular'],
    why: "What’s New, on the Help group. An announcement, which is what the page is. Deliberately **not** `sparkle` or `star-emphasis`: Fluent's sparkle has become the industry's mark for *this is generated by a model*, and a person who pressed it expecting Copilot would have been sent to a release-notes page.",
  },
  {
    name: 'arrow-upload',
    sizes: [20],
    variants: ['regular'],
    why: "Upload Workbook — Excel's Publish group, which exists because the census gives Excel a `Publish2Tab` the other two applications do not have.",
  },
  {
    name: 'arrow-export',
    sizes: [20],
    variants: ['regular'],
    why: "Export Workbook Data, beside it. The arrow leaving a box, against `arrow-upload`'s arrow entering one — the two commands on that page send different things in different directions, and drawing them alike would hide that.",
  },
  {
    name: 'data-histogram',
    sizes: [20],
    variants: ['regular'],
    why: "Workbook Statistics — Excel's own addition to the Info group. A histogram is the plainest thing Fluent draws for *statistics*; `document-data` was the alternative and says *a document with numbers in it*, which is every workbook rather than this command.",
  },
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
