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

  // ── Home (Word, PowerPoint, Excel) ──────────────────────────────────────────
  //
  // The ribbon programme's unit 2, and the first tab whose face is mostly **icon-only**. Word's
  // Font group draws four commands with labels and eleven without; Excel's Alignment group draws
  // eleven and labels none of them. That is Office's own layout, and it is why this block is
  // twice the size of unit 1's: a `size="icon"` command *is* its icon, so a missing glyph is not a
  // blemish but the whole control.
  //
  // **20 alone, for every row but one.** A command icon is drawn at 20; only `size="large"` asks
  // for 24, and the single large command this unit adds is PowerPoint's New Slide. Nothing here is
  // a toggle, which is the other half of the rule: `text-bold`, `text-italic`, `text-underline`
  // and the three alignment marks already carry both variants from the rows above, and the
  // essential ceiling of three per group means no group can declare a fourth toggle — so
  // Strikethrough, Justify, Show/Hide and Excel's three vertical alignments are drawn as icon
  // buttons and need the resting drawing only. `dev/ribbons/census.ts` records that at length.
  //
  // **Seven commands on this tab carry no icon**, and the rule is unit 1's: a near-miss is worse
  // than nothing, because a person acts on a glyph. They are PowerPoint's *Text Shadow*, Excel's
  // *Comma Style*, *Increase Decimal*, *Decrease Decimal* and *Power Options*, plus the two
  // galleries and every picker, which are bound by a host rather than drawn as buttons.
  {
    name: 'paint-brush',
    sizes: [20],
    variants: ['regular'],
    why: "Format Painter, on the Clipboard group of all three applications — and the row that replaces a `settings` cog that had been standing in for it since the shells were written. A brush is what Office draws and what the command does: it picks formatting up and puts it down somewhere else.",
  },
  {
    name: 'font-increase',
    sizes: [20],
    variants: ['regular'],
    why: "Increase Font Size, in Word, PowerPoint and Excel. Fluent draws the pair as a large A beside a small one with the direction marked, which is Office's own device — and it is the reason this is not `add`: a plus sign beside a font-size box would read as *add a size to the list*.",
  },
  {
    name: 'font-decrease',
    sizes: [20],
    variants: ['regular'],
    why: 'Decrease Font Size, the other half of the pair. Drawn as a set with `font-increase`, because a reader tells the two apart by comparing them rather than by reading either.',
  },
  {
    name: 'text-change-case',
    sizes: [20],
    variants: ['regular'],
    why: "Change Case, on Word's and PowerPoint's Font group. Aa — the same two letters Office puts on the button, and the one glyph in this block that is literally the command's own name.",
  },
  {
    name: 'clear-formatting',
    sizes: [20],
    variants: ['regular'],
    why: "Clear All Formatting, on Word's and PowerPoint's Font group. Deliberately **not** `eraser`, which this subset also carries for Excel's *Clear*: Excel's command removes the cells' contents as well, and drawing the two alike would say they did the same thing in two applications when they do not.",
  },
  {
    name: 'text-strikethrough',
    sizes: [20],
    variants: ['regular'],
    why: "Strikethrough, in Word and PowerPoint. ⚠ **Regular only, and that is a consequence rather than an oversight.** Strikethrough is a state in Office and would be a `<mjx-toggle-button>` here, but `essentialCommandLimit` is 3 and Bold, Italic and Underline have already taken the group's three essential slots — so it is drawn as an icon button and never renders filled. `dev/ribbons/census.ts` records the finding where a reader will meet it.",
  },
  {
    name: 'text-subscript',
    sizes: [20],
    variants: ['regular'],
    why: "Subscript, on Word's Font group. Regular only, for the reason `text-strikethrough` gives.",
  },
  {
    name: 'text-superscript',
    sizes: [20],
    variants: ['regular'],
    why: 'Superscript, beside it. The pair is drawn as a pair for the same reason the font-size pair is: neither is legible except against the other.',
  },
  {
    name: 'text-effects',
    sizes: [20],
    variants: ['regular'],
    why: "Text Effects and Typography, on Word's Font group. Deliberately **not** `text-effects-sparkle`, which Fluent also ships: a sparkle now reads as *generated by a model* across the whole industry, and a person who pressed it expecting Copilot would have got a WordArt gallery.",
  },
  {
    name: 'highlight',
    sizes: [20],
    variants: ['regular'],
    why: "Text Highlight Colour, on Word's Font group. A marker pen, which is Office's own drawing — and what distinguishes it from Font Colour, which is bound as a colour picker rather than a button in both hosts.",
  },
  {
    name: 'text-color',
    sizes: [20],
    variants: ['regular'],
    why: "Font Colour, on Excel's Font group — and the one place in this unit where an application's colour command is a *button* rather than a picker. Excel's Font group already carries a wide `<mjx-color-picker>` for Fill Colour; a second one beside it would be eighteen rems of field in a group that also holds a font picker and a size box, so Font Colour is the icon Office draws and the palette belongs to a later loop.",
  },
  {
    name: 'text-bullet-list-ltr',
    sizes: [20],
    variants: ['regular'],
    why: "Bullets, on Word's and PowerPoint's Paragraph group — replacing an `add` that had been standing in for it. The `ltr` member rather than the bare `text-bullet-list`: Fluent's plain one is a checklist square, and this one is the dot-and-line Office uses.",
  },
  {
    name: 'text-number-list-ltr',
    sizes: [20],
    variants: ['regular'],
    why: 'Numbering, beside it — replacing a `subtract`, which was the same stand-in problem read the other way round: a minus sign beside a bullet list says *remove one*.',
  },
  {
    name: 'text-bullet-list-tree',
    sizes: [20],
    variants: ['regular'],
    why: "Multilevel List, on Word's Paragraph group. The indented tree is exactly what the command produces, and it is what keeps the three list buttons from being one picture drawn three times.",
  },
  {
    name: 'text-indent-decrease',
    sizes: [20],
    variants: ['regular'],
    why: 'Decrease Indent in Word and Excel, and Decrease List Level in PowerPoint — one glyph for two commands Office names differently and draws identically, because in a deck the indent *is* the outline level.',
  },
  {
    name: 'text-indent-increase',
    sizes: [20],
    variants: ['regular'],
    why: 'Increase Indent / Increase List Level, the other half of that pair.',
  },
  {
    name: 'arrow-sort',
    sizes: [20],
    variants: ['regular'],
    why: "Sort on Word's Paragraph group and Sort & Filter on Excel's Editing group. The plain two-way arrow rather than `arrow-sort-down`: both commands open a surface where the direction is chosen, so a glyph that had already chosen one would be wrong half the time.",
  },
  {
    name: 'text-paragraph',
    sizes: [20],
    variants: ['regular'],
    why: "Show/Hide ¶, on Word's Paragraph group. The pilcrow, which is the command, the glyph and the thing it reveals all at once.",
  },
  {
    name: 'text-align-justify',
    sizes: [20],
    variants: ['regular'],
    why: "Justify, in Word and PowerPoint — the fourth member of an alignment radio group whose other three are toggles. Regular only: see `text-strikethrough` on why a group cannot declare a fourth essential command, and `dev/ribbons/census.ts` on what that costs here.",
  },
  {
    name: 'text-line-spacing',
    sizes: [20],
    variants: ['regular'],
    why: "Line and Paragraph Spacing in Word, Line Spacing in PowerPoint. Arrows between ruled lines, which is Office's drawing too.",
  },
  {
    name: 'color-fill',
    sizes: [20],
    variants: ['regular'],
    why: "Shading on Word's Paragraph group and Shape Fill on PowerPoint's Drawing group — two commands, one idea: *put a colour behind this*. The paint bucket is what both applications draw.",
  },
  {
    name: 'color-line',
    sizes: [20],
    variants: ['regular'],
    why: "Shape Outline, on PowerPoint's Drawing group. Fluent's own counterpart to `color-fill`, and the pairing is what stops fill and outline being two buckets.",
  },
  {
    name: 'square-shadow',
    sizes: [20],
    variants: ['regular'],
    why: "Shape Effects, on PowerPoint's Drawing group. A shape with a drop shadow on it — which is one of the five effects the menu offers and the only one that can be drawn at twenty pixels. `wand` and `sparkle` were the alternatives and both now read as *the computer will decide*, which this command is not.",
  },
  {
    name: 'border-all',
    sizes: [20],
    variants: ['regular'],
    why: "Borders, on Word's Paragraph group and Excel's Font group — replacing a bare `table`, which said *insert a table* rather than *draw lines around this*. `border-all` is the member Office shows resting, and the other fifteen border glyphs Fluent ships belong to the menu underneath it, which stays shallow per decision 3 of the approved plan.",
  },
  {
    name: 'text-proofing-tools',
    sizes: [20],
    variants: ['regular'],
    why: "Editor — the single command of Word's `GroupEditor`, which the census has carried since unit 0 and which nothing rendered until now. A pen over text is Office's own mark for the proofing pane; `text-grammar-checkmark` was the alternative and says *this text has been checked*, which is the result rather than the command.",
  },
  {
    name: 'select-all-on',
    sizes: [20],
    variants: ['regular'],
    why: "Select, on the Editing group of Word and PowerPoint — replacing a `checkmark`, which said *done* rather than *choose things*. A marquee around content is what the command does and what Office draws.",
  },
  {
    name: 'slide-add',
    sizes: [20, 24],
    variants: ['regular'],
    why: "New Slide — the headline of PowerPoint's Slides group and **the only `size=\"large\"` command this unit adds**, hence 24 as well as 20. Two words fit inside `largeControlWidthUnits` comfortably, which is the test unit 1 learned to apply before promoting anything to large.",
  },
  {
    name: 'arrow-reset',
    sizes: [20],
    variants: ['regular'],
    why: "Reset, on PowerPoint's Slides group — the command that puts a slide's placeholders back where its layout says they go. Deliberately not `arrow-undo`, which this subset carries for the quick-access bar: undo takes back the last thing anybody did, and Reset discards every override at once.",
  },
  {
    name: 'font-space-tracking-out',
    sizes: [20],
    variants: ['regular'],
    why: "Character Spacing, on PowerPoint's Font group. Fluent draws letter tracking directly, which is rare enough to be worth saying: most of the judgement calls in this block are about finding a near neighbour, and this one is the command itself.",
  },
  {
    name: 'text-column-two',
    sizes: [20],
    variants: ['regular'],
    why: "Columns, on PowerPoint's Paragraph group — the command that splits a placeholder's text into columns. Two rather than three, because two is what the menu's first entry does.",
  },
  {
    name: 'text-direction-rotate-90-right',
    sizes: [20],
    variants: ['regular'],
    why: "Text Direction on PowerPoint's Paragraph group and Orientation on Excel's Alignment group — the same command in two vocabularies, and Fluent's rotated glyph is what both applications draw.",
  },
  {
    name: 'align-center-vertical',
    sizes: [20],
    variants: ['regular'],
    why: "Align Text on PowerPoint's Paragraph group and Middle Align on Excel's Alignment group. Vertical alignment, which every application in this unit has and none of them names the same way.",
  },
  {
    name: 'align-top',
    sizes: [20],
    variants: ['regular'],
    why: "Top Align, on Excel's Alignment group — the first of the three vertical alignments Excel puts above the three horizontal ones. Icon buttons rather than toggles: the group's three essential slots are already spent on Left, Centre and Right.",
  },
  {
    name: 'align-bottom',
    sizes: [20],
    variants: ['regular'],
    why: 'Bottom Align, the third of them.',
  },
  {
    name: 'diagram',
    sizes: [20],
    variants: ['regular'],
    why: "Convert to SmartArt, on PowerPoint's Paragraph group — replacing a bare `table`. `organization` was the near neighbour and was refused: SmartArt is lists, processes and cycles as well as hierarchies, and an org chart would have named one layout out of eight.",
  },
  {
    name: 'shapes',
    sizes: [20],
    variants: ['regular'],
    why: "Shapes, the first command of PowerPoint's Drawing group and the one that survives its collapse. Three overlapping outlines, which is what the gallery underneath it contains.",
  },
  {
    name: 'layer',
    sizes: [20],
    variants: ['regular'],
    why: "Arrange, on PowerPoint's Drawing group — replacing the `slide-layout` the shell had bound, which is the icon of the *Layout* command one group to the left. Stacked layers say *which of these is in front*, which is what Arrange decides.",
  },
  {
    name: 'text-wrap',
    sizes: [20],
    variants: ['regular'],
    why: "Wrap Text, on Excel's Alignment group — replacing an `arrow-down-right`, which is a return arrow and reads as *indent* or *go to the next cell*. Text turning a corner inside a box is the command.",
  },
  {
    name: 'table-cells-merge',
    sizes: [20],
    variants: ['regular'],
    why: "Merge & Centre, on Excel's Alignment group — replacing a bare `table`. Two cells becoming one is the command's whole content, and it is what separates it from every other table glyph in this subset.",
  },
  {
    name: 'text-percent',
    sizes: [20],
    variants: ['regular'],
    why: "Percent Style, on Excel's Number group. The per-cent sign, which is the button Office draws.",
  },
  {
    name: 'currency-dollar-euro',
    sizes: [20],
    variants: ['regular'],
    why: "Accounting Number Format, on Excel's Number group. ⚠ **Two currency marks rather than one, deliberately.** Office's own button carries whichever symbol the locale uses, and a lone dollar in a product that has not chosen a locale would be a claim about who it is for. `money` was the alternative and draws a banknote, which is the wrong noun: the command formats a number, it does not represent cash.",
  },
  {
    name: 'table-lightning',
    sizes: [20],
    variants: ['regular'],
    why: "Conditional Formatting, on Excel's Styles group. A judgement, and stated as one: Fluent draws no colour scale or data bar at twenty pixels, and in its vocabulary a lightning bolt on an object means *this happens by itself*, which is exactly what a conditional rule is. The near neighbours that were refused are `color-fill`, which is already Shading and Shape Fill, and `data-bar-horizontal`, which is a chart.",
  },
  {
    name: 'table-checker',
    sizes: [20],
    variants: ['regular'],
    why: "Format as Table, on Excel's Styles group. Banded rows, which is what every table style in the gallery does and the one property that makes a formatted range look like a table at a glance. ⚠ **20 only, and no 24 exists in the vendor set** — which costs nothing, because the command is small; a later unit that wants it large has to choose a different glyph.",
  },
  {
    name: 'table-add',
    sizes: [20],
    variants: ['regular'],
    why: "Insert, the first command of Excel's Cells group. `table-insert-row` and `table-insert-column` were both refused: the button is a split whose menu inserts cells, rows, columns *or* a sheet, and naming one of the four on its face would send somebody to the wrong menu entry.",
  },
  {
    name: 'table-dismiss',
    sizes: [20],
    variants: ['regular'],
    why: 'Delete, beside it, and refused the same two specific alternatives for the same reason. A table with a dismiss mark is as broad as the command is.',
  },
  {
    name: 'table-settings',
    sizes: [20],
    variants: ['regular'],
    why: "Format, the third of Excel's Cells group — row height, column width, hide and unhide, protection. A cog on a table, which is the only honest thing to draw for a menu that broad.",
  },
  {
    name: 'autosum',
    sizes: [20],
    variants: ['regular'],
    why: "AutoSum, on Excel's Editing group — replacing an `add`, which drew a plus sign for a command Office has always drawn as a sigma. Fluent ships the sigma under this exact name, which is as close to a command's own icon as this subset gets.",
  },
  {
    name: 'eraser',
    sizes: [20],
    variants: ['regular'],
    why: "Clear, on Excel's Editing group — the command that removes contents, formats, comments or all three. Distinct from `clear-formatting` one group away, which removes formatting alone; see that row on why the two are not drawn alike.",
  },
  {
    name: 'arrow-down',
    sizes: [20],
    variants: ['regular'],
    why: "Fill, on Excel's Editing group. A judgement: the menu's default and overwhelmingly commonest entry is Fill Down, and a plain down arrow is what Office puts on the button. `arrow-download` was refused because a tray under the arrow says *save this to my machine*, and `drop` because a droplet reads as colour.",
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
