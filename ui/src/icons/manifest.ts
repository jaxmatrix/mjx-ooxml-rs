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
    sizes: [20, 24],
    variants: ['regular'],
    why: "The shared table command, which Word and Excel both need. ⚠ **24 was added by the ribbon programme's unit 3**: Table is the headline of the Insert tab's Tables group in all three applications, and a `size=\"large\"` button draws at 24. `tests/ribbons.test.ts` had used exactly this row as its example of a large button with no 24-pixel drawing, and now uses `table-checker`, which the vendor ships at 20 alone and therefore can never stop being an example.",
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
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Protect Document / Presentation / Workbook — the Info group's first command in all three applications. A document with a padlock rather than a bare `shield`: a shield says *this is defended*, and what the command actually does is put a lock on this one file. ⚠ **20 alone, and that is a rendering finding rather than a preference.** It was `size=\"large\"` and asked for 24 until the tab was looked at: a large button is bounded by `largeControlWidthUnits` so its label wraps to two lines inside about eighty pixels, and *Protect Document* came out as `Protect Docume…`. A truncated command is a command nobody can read, so the three Protect verbs are small — and the 24-pixel drawing nothing would have used went with them. **Since unit 8, the 24 and the filled drawing are back, for Restrict Editing** on Word's Review tab: a large toggle whose two-word label fits, drawn pressed while its pane is open. Restrict Editing is what Protect Document's menu opens, so it is the same padlock on the same file.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: 'Email, on the Share group. The envelope, which is the one glyph in this block nobody has to learn. **Since unit 7, also Envelopes** on Word\'s Mailings tab, where Office draws it large, so 24 as well.',
  },
  {
    name: 'link',
    sizes: [20, 24],
    variants: ['regular'],
    why: 'Get a Link, on the Share group — the command that produces a URL rather than sending a file, and the chain is what distinguishes the two. 24 since unit 3: **Link** on the Insert tab Links group is large in PowerPoint and Excel, and small in Word.',
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Create a Video — PowerPoint's Export group, and one of the two places PowerPoint's File tab genuinely differs from Word's. 24 since unit 3: Word's **Online Videos** and PowerPoint's **Video** are the large headline of their Insert tab's Media group, and both are a film camera in Office too.",
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
  // and the three alignment marks already carry both variants from the rows above. Unit 2 drew
  // Strikethrough, Subscript, Superscript, Justify, Show/Hide and Excel's three vertical alignments
  // as icon buttons, because a group could not declare a fourth toggle; **unit 2b made each of them
  // a toggle** — a survivor is now declared rather than inferred from being a state — and so each
  // row below that names one of them requests `filled` too, together with Excel's Wrap Text, which
  // Office also draws pressed. `dev/ribbons/census.ts` records which of them survive a collapse.
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
    variants: ['regular', 'filled'],
    why: "Strikethrough, in Word and PowerPoint. **Both variants**, because it is a toggle: Office draws it pressed on struck-through text. Until unit 2b it was an icon button drawing `regular` alone, because a group could not declare a fourth toggle after Bold, Italic and Underline; a survivor is declared now, and a state command is simply a toggle.",
  },
  {
    name: 'text-subscript',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Subscript, on Word's Font group. Both variants, for the reason `text-strikethrough` gives.",
  },
  {
    name: 'text-superscript',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: 'Superscript, beside it. The pair is drawn as a pair for the same reason the font-size pair is: neither is legible except against the other. Both variants, as a toggle.',
  },
  {
    name: 'text-effects',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Text Effects and Typography, on Word's Font group. Deliberately **not** `text-effects-sparkle`, which Fluent also ships: a sparkle now reads as *generated by a model* across the whole industry, and a person who pressed it expecting Copilot would have got a WordArt gallery. **Unit 3 gives it a second command, WordArt**, on the Insert tab's Text group in all three applications — the same idea twice rather than a collision, since both put an outlined, shadowed letter on the page — and 24, because PowerPoint and Excel draw WordArt large.",
  },
  {
    name: 'highlight',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Text Highlight Colour, on Word's Font group. A marker pen, which is Office's own drawing — and what distinguishes it from Font Colour, which is bound as a colour picker rather than a button in both hosts. **Since unit 4, also Highlighter** on the Draw tab's Write group in all three applications — the same marker making the same stroke, in ink rather than on text — which is a large toggle, so 24 and `filled`.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Sort on Word's Paragraph group and Sort & Filter on Excel's Editing group. The plain two-way arrow rather than `arrow-sort-down`: both commands open a surface where the direction is chosen, so a glyph that had already chosen one would be wrong half the time. **Since unit 7, also Sort** on Excel's Data tab, large, which opens exactly that surface — so 24 as well.",
  },
  {
    name: 'text-paragraph',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Show/Hide ¶, on Word's Paragraph group. The pilcrow, which is the command, the glyph and the thing it reveals all at once. Both variants: Office draws it pressed while formatting marks are showing.",
  },
  {
    name: 'text-align-justify',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Justify, in Word and PowerPoint — the fourth member of an alignment radio group. **Both variants**: it is a toggle like the other three, so a justified paragraph reads as justified. It is the one alignment a collapsed group does not keep; see `dev/ribbons/census.ts`.",
  },
  {
    name: 'text-line-spacing',
    sizes: [20],
    variants: ['regular'],
    why: "Line and Paragraph Spacing in Word, Line Spacing in PowerPoint. Arrows between ruled lines, which is Office's drawing too. **Since unit 5, also Paragraph Spacing** on Word's Design tab: the same spacing, set for the whole document. Small, so 20 alone still.",
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
    why: "Shape Outline, on PowerPoint's Drawing group. Fluent's own counterpart to `color-fill`, and the pairing is what stops fill and outline being two buckets. **Since unit 4, also Colour** on the Draw tab's Pens group: a pen over a stroke of colour is the ink colour as much as it is an outline's, and both set the colour of a drawn line. Small in both, so 20 alone still.",
  },
  {
    name: 'square-shadow',
    sizes: [20],
    variants: ['regular'],
    why: "Shape Effects, on PowerPoint's Drawing group. A shape with a drop shadow on it — which is one of the five effects the menu offers and the only one that can be drawn at twenty pixels. `wand` and `sparkle` were the alternatives and both now read as *the computer will decide*, which this command is not. **Since unit 5, also Effects** on Word's Design and Excel's Page Layout: a theme's effects are the shape effects it hands to every shape. Small in both, so 20 alone still.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Columns, on PowerPoint's Paragraph group — the command that splits a placeholder's text into columns. Two rather than three, because two is what the menu's first entry does. **Since unit 5, also Columns** on Word's Layout tab, where Office draws it large, so 24 as well.",
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
    variants: ['regular', 'filled'],
    why: "Align Text on PowerPoint's Paragraph group and Middle Align on Excel's Alignment group. Vertical alignment, which every application in this unit has and none of them names the same way. `filled` is for Middle Align, which is a toggle; Align Text opens a menu and draws `regular` alone.",
  },
  {
    name: 'align-top',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Top Align, on Excel's Alignment group — the first of the three vertical alignments Excel puts above the three horizontal ones. Both variants: all three are toggles, which unit 2 could not draw because the group's essential slots were spent on Left, Centre and Right.",
  },
  {
    name: 'align-bottom',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: 'Bottom Align, the third of them, and the one drawn pressed at rest: an unformatted Excel cell is bottom-aligned.',
  },
  {
    name: 'diagram',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Convert to SmartArt, on PowerPoint's Paragraph group — replacing a bare `table`. `organization` was the near neighbour and was refused: SmartArt is lists, processes and cycles as well as hierarchies, and an org chart would have named one layout out of eight. The same glyph for **SmartArt** on the Insert tab in all three applications, which is the same gallery; 24 since unit 3, because PowerPoint draws it large.",
  },
  {
    name: 'shapes',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Shapes, the first command of PowerPoint's Drawing group. Three overlapping outlines, which is what the gallery underneath it contains. 24 since unit 3, where Shapes is a `size=\"large\"` command of the Insert tab's Illustrations group in all three applications.",
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
    variants: ['regular', 'filled'],
    why: "Wrap Text, on Excel's Alignment group — replacing an `arrow-down-right`, which is a return arrow and reads as *indent* or *go to the next cell*. Text turning a corner inside a box is the command. Both variants: Office draws Wrap Text pressed on a wrapping cell, so it is a toggle.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "AutoSum, on Excel's Editing group — replacing an `add`, which drew a plus sign for a command Office has always drawn as a sigma. Fluent ships the sigma under this exact name, which is as close to a command's own icon as this subset gets. **Since unit 6, also AutoSum** on Excel's Formulas tab, where Office draws it large, so 24 as well.",
  },
  {
    name: 'eraser',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Clear, on Excel's Editing group — the command that removes contents, formats, comments or all three. Distinct from `clear-formatting` one group away, which removes formatting alone; see that row on why the two are not drawn alike. **Since unit 4, also Eraser** on the Draw tab's Write group in all three applications: a large toggle, so 24, and `filled` for its pressed state — in Word and PowerPoint the face of a `<mjx-split-button toggle>`, in Excel a plain toggle. Two commands with one eraser, and no collision: neither is a survivor, so rule 2's glyph standard never asks which one a bare eraser means.",
  },
  {
    name: 'arrow-down',
    sizes: [20],
    variants: ['regular'],
    why: "Fill, on Excel's Editing group. A judgement: the menu's default and overwhelmingly commonest entry is Fill Down, and a plain down arrow is what Office puts on the button. `arrow-download` was refused because a tray under the arrow says *save this to my machine*, and `drop` because a droplet reads as colour. **Since unit 7, also Move Earlier's partner, Move Later**, on PowerPoint's Animations tab, labelled: the animation moves down the list.",
  },
  // ── Insert (Word, PowerPoint, Excel) ────────────────────────────────────────
  //
  // The ribbon programme's unit 3. Insert is the tab of **nouns**: which table, which shape, which
  // chart. So nearly every glyph below names a *thing that will be on the page* rather than an action
  // on the selection, and the judgement each row records is whether Fluent draws that thing or merely
  // something near it.
  //
  // **20 for every command, and 24 only for a `size="large"` one** — Office's Insert tab is mostly
  // large buttons, so this block asks for 24 more often than Home's did. **Nothing here is a toggle**,
  // so no row asks for `filled`: the Insert tab has no state a command draws pressed.
  //
  // **Six rows above gained 24 for this unit rather than a row here** — `table`, `shapes`, `video`,
  // `link`, `diagram` and `text-effects` — and each says so in its own `why`. `data-histogram` and
  // `slide-add` are reused at the sizes they already carry.
  //
  // **Commands on this tab that carry no icon do so by unit 1's rule**: Cover Page, Cross-reference,
  // Quick Parts, Drop Cap, Reuse Slides, Zoom, Content, PivotTable, Recommended PivotTables, Insert
  // Combo Chart, PivotChart, Win/Loss, every Object and every Symbol. `dev/ribbons/census.ts` gives
  // the reason for each group; the short version is that Fluent draws no cover page, no Ω, no pivot
  // and no combo chart, and a near neighbour would send somebody to a different command.
  {
    name: 'document-one-page-add',
    sizes: [20],
    variants: ['regular'],
    why: "Blank Page, on Word's Pages group. A single page with a plus, rather than `document-add`: that one has the folded corner of *a file*, and Blank Page adds a page inside this document rather than a new document. The two are still close enough that Blank Page is refused as a survivor — see `dev/ribbons/census.ts`.",
  },
  {
    name: 'document-page-break',
    sizes: [20],
    variants: ['regular'],
    why: "Page Break, on Word's Pages group. Fluent draws the break itself — two page edges and the gap between them — which is Office's own picture of the command. **Since unit 5, also Breaks** on Word's Layout and Excel's Page Layout, whose first entry is that page break. Small in both.",
  },
  {
    name: 'image',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Pictures, on Word's and Excel's Illustrations group and PowerPoint's Images group — large in all three, hence 24. The plain landscape rather than `image-add`: the plus would say *add* on a tab where every command adds something, and the noun is what tells Pictures from Shapes and Icons beside it.",
  },
  {
    name: 'icons',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Icons, on the Illustrations group of all three applications, large. Fluent's own mark for *a set of icons*, which is exactly the library the command opens.",
  },
  {
    name: 'cube',
    sizes: [20, 24],
    variants: ['regular'],
    why: "3D Models, on the Illustrations group of all three applications — large in Word and PowerPoint, small in Excel. A cube is what Office draws and the plainest three-dimensional object there is. Not `cube-add`, which Fluent draws at 20 alone and which would say *add* for the reason `image` gives.",
  },
  {
    name: 'data-bar-vertical',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Three commands, and one idea: **Chart** on Word's and PowerPoint's Illustrations group (Office draws a column chart for *any* chart), **Insert Column or Bar Chart** on Excel's Charts group, and **Column** on Excel's Sparklines group (a column chart the size of a cell). 24 because PowerPoint's Chart and Excel's Column sparkline are large.",
  },
  {
    name: 'screenshot',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Screenshot, on Word's and Excel's Illustrations group (small) and PowerPoint's Images group (large). Fluent draws a frame with capture corners, which is the clipping the command's menu offers.",
  },
  {
    name: 'bookmark',
    sizes: [20],
    variants: ['regular'],
    why: "Bookmark, on Word's Links group. The ribbon marker Office draws, and the one glyph on the Links group nobody has to learn.",
  },
  {
    name: 'comment-add',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Comment, the one command of the Insert tab's Comments group in all three applications, large. The speech bubble with a plus rather than the bare `comment` this subset carries for Review: that one marks a thread that exists, and this command starts one. **Since unit 8, also New Comment** on Word's Review tab, large: the same command by its Review name.",
  },
  {
    name: 'document-header',
    sizes: [20],
    variants: ['regular'],
    why: "Header, on Word's Header & Footer group. A page with its top band drawn, which is Office's picture too — and one of a set with `document-footer` and `document-page-number`, so the three read as three positions on one page.",
  },
  {
    name: 'document-footer',
    sizes: [20],
    variants: ['regular'],
    why: 'Footer, beside it: the same page with the band at the bottom.',
  },
  {
    name: 'document-page-number',
    sizes: [20],
    variants: ['regular'],
    why: "Page Number, the third of Word's Header & Footer commands. A page with a number in its corner.",
  },
  {
    name: 'textbox',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Text Box, the large headline of the Insert tab's Text group in all three applications. Fluent spells it as one word and draws a box with lines of text in it; `text-box-settings`, the only other candidate, is a cog on a box.",
  },
  {
    name: 'signature',
    sizes: [20],
    variants: ['regular'],
    why: "Signature Line, on Word's and Excel's Text group. A pen on a line, which is the thing the command inserts.",
  },
  {
    name: 'calendar-clock',
    sizes: [20],
    variants: ['regular'],
    why: "Date & Time, on Word's and PowerPoint's Text group. A calendar with a clock, because the command is both, and a bare `calendar` would have said *date* alone.",
  },
  {
    name: 'math-formula',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Equation, the large first command of the Symbols group in all three applications. Fluent draws a formula with an operator in it. Deliberately not `math-symbols`, which is a calculator's four operators and would read as *calculate*. **Since unit 6, also Insert Function** on Excel's Formulas tab, large: the drawing is *fx*, which is Office's own mark for that command.",
  },
  {
    name: 'image-multiple',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Photo Album, on PowerPoint's Images group, large. Pictures stacked, which is an album, and which keeps it from being the single picture `image` draws one command to its left.",
  },
  {
    name: 'camera',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Cameo, PowerPoint's Camera group, large — a live camera feed on a slide. The camera body, not `video`, which this subset already carries for Video two groups along and which means *a recording*.",
  },
  {
    name: 'cursor-click',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Action, on PowerPoint's Links group, large. The command sets what happens when a shape is clicked, and a pointer mid-click is that sentence as a picture; Office draws a starburst for the same reason.",
  },
  {
    name: 'document-header-footer',
    sizes: [20],
    variants: ['regular'],
    why: "Header & Footer, on PowerPoint's and Excel's Text group. A page with both bands drawn — the command opens one surface for the two, which is exactly the difference between it and Word's separate Header and Footer.",
  },
  {
    name: 'number-symbol-square',
    sizes: [20],
    variants: ['regular'],
    why: "Slide Number, on PowerPoint's Text group. A number sign in a frame. `document-page-number` was refused: a slide is not a portrait page, and one command drawn in each application's own vocabulary is clearer than one glyph that is wrong in one of them.",
  },
  {
    name: 'speaker-2',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Audio, on PowerPoint's Media group, large. A speaker with sound coming out of it; `music-note-2` was refused, because most audio on a slide is narration rather than music.",
  },
  {
    name: 'record',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Screen Recording, on PowerPoint's Media group, large. The record mark — a filled circle in a ring — which is the one convention every recorder shares. `desktop` was the other half of the command and is already This PC on the File tab.",
  },
  {
    name: 'chart-multiple',
    sizes: [20],
    variants: ['regular'],
    why: "Recommended Charts, the first command of Excel's Charts group. Several charts together, because the command offers a choice of them; each single chart glyph beside it already names one family. ⚠ **20 alone, and that is a measurement.** It was `size=\"large\"` with a 24 requested, and the built catalogue showed its label clipped: *Recommended* does not fit a large button's width on one line. So the command is small, as `document-lock` records for Protect Document, and the 24 nothing would draw went with it.",
  },
  {
    name: 'data-treemap',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Hierarchy Chart, on Excel's Charts group — icon-only, as Office draws it. The treemap is the family's first member and its picture in Office.",
  },
  {
    name: 'data-waterfall',
    sizes: [20],
    variants: ['regular'],
    why: 'Insert Waterfall, Funnel, Stock, Surface or Radar Chart — icon-only. Office draws the waterfall for the whole family, and so does this.',
  },
  {
    name: 'data-line',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Insert Line or Area Chart (icon-only) and the Line sparkline (large) on Excel's Insert tab: a line chart, at the size of a chart and at the size of a cell.",
  },
  {
    name: 'data-pie',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Pie or Doughnut Chart, on Excel's Charts group — icon-only.",
  },
  {
    name: 'data-scatter',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Scatter (X, Y) or Bubble Chart, on Excel's Charts group — icon-only.",
  },
  {
    name: 'map',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Maps, on Excel's Charts group, large — the filled map chart. A folded map rather than `globe`, which is *the web* everywhere else in Office. **Since unit 7, also Geography** in Excel's Data Types gallery, at 24: Office draws a map there too.",
  },
  {
    name: 'filter',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Slicer, on Excel's Filters group, large. A slicer is a set of filter buttons, and the funnel is the filter mark across the whole of Office. **Since unit 7, also Filter** on Excel's Data tab: a large toggle, so the filled drawing too. Sharing the funnel with Slicer is why Filter does not survive a collapse.",
  },
  {
    name: 'timeline',
    sizes: [20, 24],
    variants: ['regular'],
    why: 'Timeline, beside it: a date filter drawn as a timeline, which is what Fluent draws.',
  },
  {
    name: 'checkbox-checked',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Checkbox, the one command of Excel's Cell Controls group, large. The ticked box is the control the command turns a cell into. A verb here, not a toggle — pressing it converts the selection, it does not draw pressed — so `regular` alone.",
  },
  // ── Draw (Word, PowerPoint, Excel) ──────────────────────────────────────────
  //
  // The ribbon programme's unit 4. Draw is the first tab that is mostly **state**: the tool in hand,
  // the ruler on the page, whether a finger inks. So unlike Insert, most rows below ask for `filled`,
  // because a toggle draws filled while it is pressed and a regular-only request goes blank the moment
  // somebody presses it. **20 for a small command and 20 and 24 for a large one**, because a large
  // command lays out sideways at 20 in a reduced group (see `question-circle`).
  //
  // **Three rows above gained Draw commands rather than a row here**: `eraser` and `highlight` (each
  // now 24 and `filled`) and `color-line`. `question-circle` is reused at the sizes it already has.
  //
  // **Three commands carry no icon**: Add Pen, Touch/Mouse Mode and Drawing Canvas. `dev/ribbons/census.ts`
  // gives the reasons in the *commands Draw shows* section.
  {
    name: 'cursor',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Select Objects, on the Draw tab's Write group in all three applications: the plain arrow pointer, which is Office's own picture of putting the pen down to select. A small toggle, and **the tab's one survivor**, so it has to be recognisable with no label: a pointer is. Not `select-object`, a dashed marquee with handles, which is what a selection looks like rather than the tool that makes one.",
  },
  {
    name: 'lasso',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Lasso Select, beside Select Objects: a dashed loop with its rope, which is the gesture. A small toggle.",
  },
  {
    name: 'pen',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Pen, on the Draw tab's Write group, a large toggle. Fluent's plain pen: the Draw tab's pens are ballpoint ink, not a nib or a brush.",
  },
  {
    name: 'ruler',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Ruler, the one command of Word's and PowerPoint's Stencils group, a large toggle. Office draws a ruler too.",
  },
  {
    name: 'hand-draw',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Draw with Touch, the one command of its group in all three applications: a finger tracing a stroke, which is the sentence the command toggles. A small toggle, because three tokens do not wrap inside a large button. Not `tap-single`, which is a tap and names a press rather than ink.",
  },
  {
    name: 'replay',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Ink Replay, large: a circular arrow around a play mark. The play mark is what separates it from `arrow-reset` and `arrow-undo`, which would say *take it back* rather than *play it back*.",
  },
  {
    name: 'pen-dismiss',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Stop Inking, the Close group's one command, large: a pen with a cross on it, which is Office's own picture of putting the pen away. `dismiss` alone would say *close this panel*.",
  },
  {
    name: 'inking-tool',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Pens, the pen-style gallery on the Draw tab's Pens group, large. Fluent's own mark for an inking tool, a nib. Distinct from `pen` for the Pen *tool* one group away: the gallery chooses what the pens draw with, and the tool picks one up. `GUESS:` that a reader will tell the two apart.",
  },
  {
    name: 'line-thickness',
    sizes: [20],
    variants: ['regular'],
    why: "Thickness, on the Draw tab's Pens group: three rules of increasing weight, which is the menu under it. Small.",
  },
  {
    name: 'text-edit-style',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Ink Editor, on Word's Draw tab (the census's `GroupEditingExcel`), a large toggle: a letter beside a pen, because the command edits text by pen gesture. `GUESS:` the command itself — see `dev/ribbons/census.ts`.",
  },
  // ── Design and Layout (Word's Design and Layout, PowerPoint's Design, Excel's Page Layout) ──
  //
  // The ribbon programme's unit 5. Nearly every command here opens a menu, a gallery or a dialog, so
  // every row below is `regular` alone: nothing draws pressed. The one toggle on these tabs that
  // could, Selection Pane, carries no icon. **20 for a small command, and 20 and 24 for a large one**,
  // because a large command lays out sideways at 20 in a reduced group.
  //
  // **Four rows above gained a command rather than a row here**: `text-line-spacing` (Paragraph
  // Spacing), `square-shadow` (Effects), `text-column-two` (Columns, now 24 too) and
  // `document-page-break` (Breaks). Every command these tabs draw without an icon is listed, with its
  // reason, in the *commands Design and Layout show* section of `dev/ribbons/census.ts`.
  {
    name: 'color',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Colours, the theme colour sets, on Word's Design (large) and Excel's Page Layout (small): a painter's palette, which is what a set of theme colours is. Not `color-fill` or `color-line`, which set one colour on one thing.",
  },
  {
    name: 'text-font',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Fonts, the theme font pairs, on Word's Design (large) and Excel's Page Layout (small): two letters of two sizes, a heading font over a body font. Not `text-color` or `font-increase`, which act on the selected text.",
  },
  {
    name: 'document-border',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Page Borders, on Word's Design tab, large: a page with a border drawn inside its edge. Not `border-all`, Home's Borders, which is a grid of cell or paragraph edges.",
  },
  {
    name: 'document-margins',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Margins, on Word's Layout and Excel's Page Layout, large: a page with its margins dashed in, which is Office's picture of the command.",
  },
  {
    name: 'orientation',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Orientation, on Word's Layout and Excel's Page Layout, large: a portrait page turning to landscape. Not `document-landscape`, which shows one answer rather than the choice.",
  },
  {
    name: 'text-position-square',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Wrap Text, on Word's Layout Arrange group, large: text flowing round a square, the first wrap Office's menu offers after In Line. Not `text-wrap`, which is Excel's Home Wrap Text, text wrapping inside a cell.",
  },
  {
    name: 'position-forward',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Bring Forward, the split button on Arrange in Word (small) and Excel (large): the hatched shape moving in front of the plain one.",
  },
  {
    name: 'position-backward',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Send Backward, beside Bring Forward and its mirror: the hatched shape moving behind the plain one.",
  },
  {
    name: 'align-left',
    sizes: [20],
    variants: ['regular'],
    why: "Align, the dropdown on Arrange in Word and Excel: two shapes lined up against one edge, which is what the menu does to a selection of shapes. Not `text-align-left`, whose ragged lines are paragraph alignment.",
  },
  {
    name: 'group',
    sizes: [20],
    variants: ['regular'],
    why: "Group, the dropdown on Arrange in Word and Excel: shapes inside one set of selection handles.",
  },
  {
    name: 'rotate-right',
    sizes: [20],
    variants: ['regular'],
    why: "Rotate, the dropdown on Arrange in Word and Excel: a shape with a turning arrow, as Office draws it. Not `arrow-rotate-clockwise`, a bare arrow that reads as *refresh*.",
  },
  {
    name: 'slide-size',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Slide Size, on PowerPoint's Customise group, large: a frame with an arrow to its corner, the slide being resized.",
  },
  {
    name: 'color-background',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Format Background, on PowerPoint's Customise group, large: a paint bucket over a frame, the fill behind a whole slide rather than on a shape (`color-fill`).",
  },  // ── References, Transitions and Formulas (Word's References, PowerPoint's Transitions, Excel's Formulas) ──
  //
  // The ribbon programme's unit 6. Nothing here draws pressed: the one toggle with a state Office draws,
  // Show Formulas, carries no icon, so every row is `regular` alone. **20 for a small command, and 20
  // and 24 for a large one.** Two rows above gained a command rather than a row here: `autosum` (now 24
  // too) and `math-formula` (Insert Function). Every command these tabs draw without an icon is listed,
  // with its reason, in the *commands References, Transitions and Formulas show* section of
  // `dev/ribbons/census.ts`.
  {
    name: 'document-bullet-list',
    sizes: [20],
    variants: ['regular'],
    why: "Table of Contents, on Word's References tab, small: a page holding a list, which is what a table of contents is. Small because three tokens do not fit a large button.",
  },
  {
    name: 'document-sync',
    sizes: [20],
    variants: ['regular'],
    why: "Update Table (three times) and Update Index, on Word's References tab: a page with a refresh on it, the generated table brought up to date. Not `arrow-sync`, which is AutoSave, and the four sharing one glyph is why none of them survives a collapse.",
  },
  {
    name: 'text-footnote',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Insert Footnote, on Word's References tab, large: *Ab* with a superscript 1, Office's own picture of the command.",
  },
  {
    name: 'document-endnote',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Endnote, beside Insert Footnote: a page with a mark at its foot.",
  },
  {
    name: 'text-quote',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Insert Citation, on Word's References tab, large: a pair of quotation marks. `GUESS:` a citation marks words taken from a source; Office draws a page Fluent does not.",
  },
  {
    name: 'library',
    sizes: [20],
    variants: ['regular'],
    why: "Manage Sources, on Word's References tab: books on a shelf, the document's collected sources. Not `book-open`, which is Read Mode.",
  },
  {
    name: 'slide-transition',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Preview, on PowerPoint's Transitions tab, large: a slide sliding off its neighbours, the thing the command plays. Not `play`, a video's, or `slide-play`, From Current Slide.",
  },
  {
    name: 'book-star',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Recently Used, on Excel's Formulas tab, large: a book with a star. Fluent ships Excel's own function library books, and this is the first.",
  },
  {
    name: 'book-coins',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Financial, on Excel's Formulas tab, large: Excel's book with coins.",
  },
  {
    name: 'book-question-mark',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Logical, on Excel's Formulas tab, large: Excel's book with a question mark.",
  },
  {
    name: 'book-letter',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Text, on Excel's Formulas tab, large: Excel's book with a letter.",
  },
  {
    name: 'book-clock',
    sizes: [20],
    variants: ['regular'],
    why: "Date & Time, on Excel's Formulas tab, small (three tokens): Excel's book with a clock.",
  },
  {
    name: 'book-search',
    sizes: [20],
    variants: ['regular'],
    why: "Lookup & Reference, on Excel's Formulas tab, small (three tokens): Excel's book with a magnifier. **Since unit 8, also Thesaurus** on Word's Review tab, small: a book being looked in for a word. `GUESS:`.",
  },
  {
    name: 'book-theta',
    sizes: [20],
    variants: ['regular'],
    why: "Math & Trig, on Excel's Formulas tab, small (three tokens): Excel's book with a theta.",
  },
  {
    name: 'book',
    sizes: [20, 24],
    variants: ['regular'],
    why: "More Functions, on Excel's Formulas tab, large: the plain book the other seven categories are drawn on, for the categories that have no book of their own.",
  },
  {
    name: 'tag-multiple',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Name Manager, on Excel's Formulas tab, large: a stack of name tags, every defined name at once. `GUESS:` Office draws a tag over a table.",
  },
  {
    name: 'tag-add',
    sizes: [20],
    variants: ['regular'],
    why: "Define Name, the split button beside Name Manager: a name tag with a plus, a new name.",
  },
  {
    name: 'checkmark-circle-warning',
    sizes: [20],
    variants: ['regular'],
    why: "Error Checking, the split button on Excel's Formula Auditing group: a check that found a warning. Not `warning`, the status mark. **Since unit 7, also Check for Errors** on Word's Mailings tab, the same check before a merge.",
  },
  {
    name: 'calculator',
    sizes: [20],
    variants: ['regular'],
    why: "Calculation Options, on Excel's Formulas tab: a calculator. Small, because *Calculation* is as long as the *Recommended* unit 3 measured clipping.",
  },
  {
    name: 'calculator-arrow-clockwise',
    sizes: [20],
    variants: ['regular'],
    why: "Calculate Now, on Excel's Calculation group: a calculator running again.",
  },
  {
    name: 'table-calculator',
    sizes: [20],
    variants: ['regular'],
    why: "Calculate Sheet, beside Calculate Now: a calculator over a sheet, one sheet recalculated. Fluent draws it at 20 alone, and the command is small.",
  },
  // ── Mailings, Animations and Data (Word's Mailings, PowerPoint's Animations, Excel's Data) ──
  //
  // The ribbon programme's unit 7. One row here draws pressed and carries `filled` (`eye`, Preview
  // Results), and one row above gained it (`filter`, Data's Filter). **20 for a small or icon-only command, and 20 and 24 for a large one**; the
  // two data type pictures are 24 alone. Six rows above gained a command rather than a row here. Every
  // command these tabs draw without an icon is listed, with its reason, in the *commands Mailings,
  // Animations and Data show* section of `dev/ribbons/census.ts`.
  {
    name: 'mail-multiple',
    sizes: [20],
    variants: ['regular'],
    why: "Start Mail Merge, on Word's Mailings tab, small: a stack of envelopes, many letters from one document. `GUESS:` Office draws a page with envelopes Fluent does not.",
  },
  {
    name: 'people-list',
    sizes: [20],
    variants: ['regular'],
    why: "Select Recipients, on Word's Mailings tab: people beside a list, the recipient list being chosen. Small, because *Recipients* is the length of the *Recommended* unit 3 measured clipping. `GUESS:`.",
  },
  {
    name: 'people-edit',
    sizes: [20],
    variants: ['regular'],
    why: "Edit Recipient List, beside Select Recipients: the same people with a pencil. `GUESS:`.",
  },
  {
    name: 'contact-card',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Address Block, on Word's Mailings tab, large: a card with a name and lines, an address. `GUESS:` Office draws a page with an address block.",
  },
  {
    name: 'hand-wave',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Greeting Line, on Word's Mailings tab, large: a waving hand, a greeting.",
  },
  {
    name: 'eye',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Preview Results, on Word's Mailings tab: a large toggle, so 24 and the filled drawing. An eye is *show*; that is also why it does not survive a collapse unlabelled.",
  },
  {
    name: 'previous',
    sizes: [20],
    variants: ['regular'],
    why: "First Record, on Word's Mailings tab, icon-only: the skip-to-start mark Office's navigator draws.",
  },
  {
    name: 'caret-left',
    sizes: [20],
    variants: ['regular'],
    why: "Previous Record, on Word's Mailings tab, icon-only, and a survivor: one step back through the records.",
  },
  {
    name: 'caret-right',
    sizes: [20],
    variants: ['regular'],
    why: "Next Record, on Word's Mailings tab, icon-only, and a survivor: one step forward through the records.",
  },
  {
    name: 'next',
    sizes: [20],
    variants: ['regular'],
    why: "Last Record, on Word's Mailings tab, icon-only: the skip-to-end mark.",
  },
  {
    name: 'person-search',
    sizes: [20],
    variants: ['regular'],
    why: "Find Recipient, on Word's Mailings tab: a person under a magnifier.",
  },
  {
    name: 'star-add',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Add Animation, on PowerPoint's Animations tab, large: a star with a plus, Office's own picture of the command.",
  },
  {
    name: 'paint-brush-sparkle',
    sizes: [20],
    variants: ['regular'],
    why: "Animation Painter, on PowerPoint's Animations tab: a brush with sparkles, the effect copied. Not `paint-brush`, which is Format Painter.",
  },
  {
    name: 'flash',
    sizes: [20],
    variants: ['regular'],
    why: "Trigger, on PowerPoint's Animations tab, and Flash Fill, on Excel's Data tab: the lightning bolt Office draws for both. `GUESS:` sharing it, which neither survivor rule is asked to judge.",
  },
  {
    name: 'arrow-up',
    sizes: [20],
    variants: ['regular'],
    why: "Move Earlier, on PowerPoint's Animations tab, labelled: the animation moves up the list.",
  },
  {
    name: 'globe',
    sizes: [20],
    variants: ['regular'],
    why: "From Web, on Excel's Data tab: the web, as everywhere in Office.",
  },
  {
    name: 'document-text',
    sizes: [20],
    variants: ['regular'],
    why: "From Text, on Excel's Data tab: a page of text.",
  },
  {
    name: 'database',
    sizes: [20],
    variants: ['regular'],
    why: "From Other Sources, on Excel's Data tab: a database, the sources the wizards connect to.",
  },
  {
    name: 'plug-connected',
    sizes: [20],
    variants: ['regular'],
    why: "Existing Connections, on Excel's Data tab: a plug in its socket. `GUESS:`.",
  },
  {
    name: 'arrow-clockwise',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Refresh All, on Excel's Data tab, large: the refresh arrow. Not `arrow-sync`, which is AutoSave.",
  },
  {
    name: 'building-bank',
    sizes: [24],
    variants: ['regular'],
    why: "Stocks, in Excel's Data Types gallery: the bank Office draws for the Stocks data type. 24, the size of a gallery cell's picture.",
  },
  {
    name: 'money',
    sizes: [24],
    variants: ['regular'],
    why: "Currencies, in Excel's Data Types gallery: notes and coins. `GUESS:`.",
  },
  {
    name: 'text-sort-ascending',
    sizes: [20],
    variants: ['regular'],
    why: "Sort A to Z, on Excel's Data tab, icon-only, and a survivor: A over Z with an arrow, Office's own sort mark.",
  },
  {
    name: 'text-sort-descending',
    sizes: [20],
    variants: ['regular'],
    why: "Sort Z to A, beside Sort A to Z, icon-only, and a survivor.",
  },
  {
    name: 'filter-dismiss',
    sizes: [20],
    variants: ['regular'],
    why: "Clear, on Excel's Sort & Filter group: the funnel struck out.",
  },
  {
    name: 'filter-sync',
    sizes: [20],
    variants: ['regular'],
    why: "Reapply, beside Clear: the funnel run again.",
  },
  {
    name: 'table-simple-checkmark',
    sizes: [20],
    variants: ['regular'],
    why: "Data Validation, the split button on Excel's Data Tools group: a table with a tick, the cells checked.",
  },
  {
    name: 'table-link',
    sizes: [20],
    variants: ['regular'],
    why: "Relationships, on Excel's Data Tools group: tables joined. `GUESS:`.",
  },
  {
    name: 'data-trending',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Forecast Sheet, on Excel's Data tab, large: a line going up. `GUESS:`. Not `data-line`, which is Insert's Line chart.",
  },
  {
    name: 'group-list',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Group, the large split button on Excel's Outline group: rows under one bracket. Not `group`, which is Arrange's shapes. `GUESS:`.",
  },
  {
    name: 'add-square',
    sizes: [20],
    variants: ['regular'],
    why: "Show Detail, on Excel's Outline group: the plus in a box the outline draws in the margin.",
  },
  {
    name: 'subtract-square',
    sizes: [20],
    variants: ['regular'],
    why: "Hide Detail, beside Show Detail: the minus in a box.",
  },
  // ── Review (Word's Review) ──────────────────────────────────────────────────
  //
  // The ribbon programme's unit 8, narrowed to Word alone. **20 for a small command, and 20 and 24 for a
  // large one.** No row here is a toggle's, so none carries `filled`; `document-lock` above gained it for
  // Restrict Editing. Three rows above gained a command rather than a row here. Every command the tab draws
  // without an icon is listed, with its reason, in the *commands Review shows* section of
  // `dev/ribbons/census.ts`.
  {
    name: 'text-grammar-checkmark',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Spelling & Grammar, on Word's Review tab, small (three tokens): text with a tick, Office's *ABC✓*. Not `text-proofing-tools`, which is Home's Editor. **Since PowerPoint's Review, also Spelling** there, large, because one word fits: hence the 24.",
  },
  {
    name: 'text-word-count',
    sizes: [20],
    variants: ['regular'],
    why: "Word Count, on Word's Review tab: Fluent's own drawing of the command.",
  },
  {
    name: 'accessibility-checkmark',
    sizes: [20],
    variants: ['regular'],
    why: "Check Accessibility, the split button on Word's Review tab, small (*Accessibility* is thirteen letters): the accessibility figure with a tick, the checker.",
  },
  {
    name: 'translate',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Translate, the large dropdown on Word's Review tab: Fluent's own translation mark.",
  },
  {
    name: 'local-language',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Language, the large dropdown on Word's Review tab: Fluent's own language mark.",
  },
  {
    name: 'comment-dismiss',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Delete, the large split button on Word's Review Comments group: a speech bubble struck out, a comment deleted. Not `delete`, the bin, which deletes whatever is selected.",
  },
  {
    name: 'comment-arrow-left',
    sizes: [20],
    variants: ['regular'],
    why: "Previous Comment, on Word's Review tab, and a survivor: a speech bubble with an arrow back. `GUESS:` that it reads unlabelled.",
  },
  {
    name: 'comment-arrow-right',
    sizes: [20],
    variants: ['regular'],
    why: "Next Comment, beside Previous Comment, and a survivor.",
  },
  {
    name: 'comment-multiple',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Show Comments, the split button on Word's Review tab: many speech bubbles, every comment shown. Its face is a toggle (`<mjx-split-button toggle>`) that starts pressed, so the filled drawing too. **Since PowerPoint's Review, also Show Comments** there, large, where the face opens the Comments pane: hence the 24, in both variants.",
  },
  {
    name: 'document-edit',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Track Changes, the large split button on Word's Review tab: a page with a pencil, edits recorded against the page. Its face turns tracking on and off (`<mjx-split-button toggle>`), so it draws pressed and needs the filled drawing.",
  },
  {
    name: 'panel-left-text',
    sizes: [20],
    variants: ['regular'],
    why: "Reviewing Pane, the split button on Word's Review tab: a pane of text at the left, where Word opens it. `GUESS:`.",
  },
  {
    name: 'document-checkmark',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Accept, the large split button on Word's Review Changes group: a page with a tick, the change kept.",
  },
  {
    name: 'document-dismiss',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Reject, beside Accept: a page struck out, the change refused. **Since PowerPoint's Review, also Reject** in its Compare group, large where Word's is small: hence the 24.",
  },
  {
    name: 'person-lock',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Block Authors, on Word's Review Protect group, large: a person with a padlock, other authors kept out of a region. `GUESS:`.",
  },
  // ── Review (PowerPoint's Review) ────────────────────────────────────────────
  //
  // PowerPoint's Review tab, authored after Word's. Every other glyph it draws is one of Word's rows above;
  // three of those gained a 24 (Spelling, Show Comments and Reject are large here). Every command drawn
  // without an icon is listed, with its reason, in the *PowerPoint's Review* part of `dev/ribbons/census.ts`.
  {
    name: 'panel-right',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Reviewing Pane, the toggle in PowerPoint's Review Compare group: a window with a pane at the right, where PowerPoint docks its Revisions pane. Word's Reviewing Pane draws `panel-left-text` because Word opens it at the left, and Fluent has no right-hand pane with text in it. A toggle, so filled too. `GUESS:`.",
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
