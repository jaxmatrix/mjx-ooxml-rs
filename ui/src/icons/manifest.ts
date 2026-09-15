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
    why: "Paragraph alignment. One of a radio group, whose chosen member is drawn filled. **Since PowerPoint's Table Layout unit, also Align Left, Align Centre and Align Right** (with `text-align-center` and `text-align-right`) in its Alignment group, icon-only toggles in one exclusive set and survivors: the same act on a cell's text. `GUESS:`.",
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
    why: "File verb. ⚠ **24 was added by the ribbon programme's unit 1**, which made Save the headline command of the File tab's Save group and therefore a `size=\"large\"` button. A large button draws at 24 and this row carried 20 alone, so it would have been the blank square `tests/ribbons.test.ts` exists to refuse — and did refuse, before a pixel was rendered. **Since Excel's View unit, also Keep**, small, in Excel's View Sheet View group, which saves a temporary sheet view. `GUESS:`.",
  },
  { name: 'folder-open', sizes: [20, 24], variants: ['regular'], why: 'File verb.' },
  {
    name: 'add',
    sizes: [16, 20],
    variants: ['regular'],
    why: "Insertion, in a toolbar and inline in a list. **Since Excel's View unit, also New**, small, in Excel's View Sheet View group: a new temporary sheet view. `GUESS:`.",
  },
  {
    name: 'delete',
    sizes: [16, 20, 24],
    variants: ['regular'],
    why: "Removal. **Since PowerPoint's Recording unit, also Clear Recording**, the large dropdown in Recording's Edit group, which is the bin the record window's own Delete button draws, so 24 as well. `GUESS:`. **Since PowerPoint's Slide Master unit, also Slide Master's Delete**, small, which deletes the selected master or layout.",
  },
  {
    name: 'search',
    sizes: [16, 20],
    variants: ['regular'],
    why: 'Find, in the chrome and inline in a filter field.',
  },
  {
    name: 'settings',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Options. **Since Word's Print Preview unit, also Print Preview's Options**, large, which opens Word Options: a cog. Hence 24 as well as 20. `GUESS:`. **Since PowerPoint's Print Preview unit, also PowerPoint's Options**, large, which opens a menu of printing options: still a set of options. **Since Excel's Print Preview unit, also Excel's Page Setup**, large, in the same slot of the same group, which opens the Page Setup dialog: the settings of the printed page. Fluent draws no page with a cog, and every page glyph here is already another command. `GUESS:`.",
  },
  {
    name: 'table',
    sizes: [20, 24],
    variants: ['regular'],
    why: "The shared table command, which Word and Excel both need. ⚠ **24 was added by the ribbon programme's unit 3**: Table is the headline of the Insert tab's Tables group in all three applications, and a `size=\"large\"` button draws at 24. `tests/ribbons.test.ts` had used exactly this row as its example of a large button with no 24-pixel drawing, and now uses `table-checker`, which the vendor ships at 20 alone and therefore can never stop being an example.",
  },
  {
    name: 'slide-layout',
    sizes: [20, 24],
    variants: ['regular'],
    why: "PowerPoint's layout command. **Since PowerPoint's Slide Master unit, also Insert Layout**, large, in Edit Master: a slide with a layout inside it, the layout the command adds. Hence 24 as well as 20. Fluent draws no layout with a plus. `GUESS:`. **Since PowerPoint's Slide Master Home unit, also Insert Layout there**, large, in Master Slides: the same command. Layout beside it draws `layout-row-two-split-top` instead, so the one group does not carry one glyph twice.",
  },
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
    why: "Protect Document / Presentation / Workbook — the Info group's first command in all three applications. A document with a padlock rather than a bare `shield`: a shield says *this is defended*, and what the command actually does is put a lock on this one file. ⚠ **20 alone, and that is a rendering finding rather than a preference.** It was `size=\"large\"` and asked for 24 until the tab was looked at: a large button is bounded by `largeControlWidthUnits` so its label wraps to two lines inside about eighty pixels, and *Protect Document* came out as `Protect Docume…`. A truncated command is a command nobody can read, so the three Protect verbs are small — and the 24-pixel drawing nothing would have used went with them. **Since unit 8, the 24 and the filled drawing are back, for Restrict Editing** on Word's Review tab: a large toggle whose two-word label fits, drawn pressed while its pane is open. Restrict Editing is what Protect Document's menu opens, so it is the same padlock on the same file. **Since Excel's Review, also Protect Workbook** there, a large toggle drawn pressed while the workbook's structure is protected, which is what Info's Protect Workbook menu opens as *Protect Workbook Structure*.",
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
    why: "Print — the headline of the Print group in Word and PowerPoint, `size=\"large\"`, hence 24 as well as 20. Excel has no Print group here; see `dev/ribbons/census.ts` on the census's missing `TabPrint` row. **Since Word's Print Preview unit, also Print Preview's Print**, large, which opens the Print dialog. `GUESS:`. **Since PowerPoint's Print Preview unit, also PowerPoint's**, the same command. **Since Excel's Print Preview unit, also Excel's**, the same command, on the Print Preview tab rather than in File.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Present Online, on Word's and PowerPoint's Share group. A figure beside a screen, which is what the command does and what Office draws for it. **Since PowerPoint's Slide Show unit, also Present Online** in Start Slide Show, the same command, large, so 24 as well.",
  },
  {
    name: 'slide-multiple',
    sizes: [20],
    variants: ['regular'],
    why: "Publish Slides — PowerPoint's own Share command, which uploads individual slides to a library rather than the deck as a file. Slides in the plural is the distinction the glyph carries. **Since PowerPoint's Slide Master Home unit, also Section there**, small, in Master Slides: Home's command, now a dropdown.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "**Since PowerPoint's Recording unit, also Export**, the large dropdown in Recording's Export group, an arrow leaving a box, so 24 as well (`GUESS:`). Export Workbook Data, beside it. The arrow leaving a box, against `arrow-upload`'s arrow entering one — the two commands on that page send different things in different directions, and drawing them alike would hide that. **Since Excel's Table Design unit, also its Export**, the large dropdown in External Table Data, which exports the table to a SharePoint list or a Visio diagram: an arrow leaving a box. `GUESS:`.",
  },
  {
    name: 'data-histogram',
    sizes: [20],
    variants: ['regular'],
    why: "Workbook Statistics — Excel's own addition to the Info group. A histogram is the plainest thing Fluent draws for *statistics*; `document-data` was the alternative and says *a document with numbers in it*, which is every workbook rather than this command. **Since Excel's Review, also Workbook Statistics** in its Proofing group, the same dialog from a second door.",
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
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Format Painter, on the Clipboard group of all three applications — and the row that replaces a `settings` cog that had been standing in for it since the shells were written. A brush is what Office draws and what the command does: it picks formatting up and puts it down somewhere else. **Since Word's Table Design unit, also Border Painter**, the large toggle in the Borders group, so 24 and the filled drawing too: Office's own glyph for it is a brush over a border, and the command paints the pen's border onto the edges it is dragged across. Sharing Format Painter's brush is why Border Painter is no survivor (demotion rule 2). `GUESS:`.",
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
    why: "Text Effects and Typography, on Word's Font group. Deliberately **not** `text-effects-sparkle`, which Fluent also ships: a sparkle now reads as *generated by a model* across the whole industry, and a person who pressed it expecting Copilot would have got a WordArt gallery. **Unit 3 gives it a second command, WordArt**, on the Insert tab's Text group in all three applications — the same idea twice rather than a collision, since both put an outlined, shadowed letter on the page — and 24, because PowerPoint and Excel draw WordArt large. **Since PowerPoint's Table Design unit, also WordArt Styles' Text Effects**, small, the census's shared group that Shape Format and Chart Format will call too: the outlined, shadowed letter is what the menu's Shadow, Glow and Bevel make. `GUESS:`.",
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
    why: "Sort on Word's Paragraph group and Sort & Filter on Excel's Editing group. The plain two-way arrow rather than `arrow-sort-down`: both commands open a surface where the direction is chosen, so a glyph that had already chosen one would be wrong half the time. **Since unit 7, also Sort** on Excel's Data tab, large, which opens exactly that surface — so 24 as well. **Since Word's Table Layout unit, also Sort** in the Data group, large: Home's Sort, applied to the table. `GUESS:`.",
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
    why: "Shape Effects, on PowerPoint's Drawing group. A shape with a drop shadow on it — which is one of the five effects the menu offers and the only one that can be drawn at twenty pixels. `wand` and `sparkle` were the alternatives and both now read as *the computer will decide*, which this command is not. **Since unit 5, also Effects** on Word's Design and Excel's Page Layout: a theme's effects are the shape effects it hands to every shape. Small in both, so 20 alone still. **Since PowerPoint's Slide Master unit, also Edit Theme's Effects**, small, which opens the theme effect sets Design's Effects opens: the drop shadow is one of them. **Since PowerPoint's Handout Master unit, also Handout Master's Effects**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Effects**, the same shared group. **Since PowerPoint's Table Design unit, also Table Design's Effects**, small, which opens Cell Bevel, Shadow and Reflection for the table's cells: the shadow is one of the three, the same idea applied to cells. **Since PowerPoint's Shape Format unit, also Shape Format's Shape Effects**, small, which is Home's Shape Effects on the contextual tab and opens the same seven lists. `GUESS:`.",
  },
  {
    name: 'border-all',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Borders, on Word's Paragraph group and Excel's Font group — replacing a bare `table`, which said *insert a table* rather than *draw lines around this*. `border-all` is the member Office shows resting, and the other fifteen border glyphs Fluent ships belong to the menu underneath it, which stays shallow per decision 3 of the approved plan. **Since Word's Table Design unit, also Table Design's Borders**, the large split button in the Borders group: the same command and the same resting face, so 24 as well. `GUESS:`. **Since PowerPoint's Table Design unit, also its Borders**, the small split button in the Table Styles group: the same command and resting face. `GUESS:`.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "**Since PowerPoint's Recording unit, also Reset to Cameo**, the large dropdown in Recording's Edit group, the same verb, so 24 as well (`GUESS:`). Reset, on PowerPoint's Slides group — the command that puts a slide's placeholders back where its layout says they go. Deliberately not `arrow-undo`, which this subset carries for the quick-access bar: undo takes back the last thing anybody did, and Reset discards every override at once. **Since PowerPoint's Slide Master Home unit, also Reset there**, small, in Master Slides: Home's command.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Text Direction on PowerPoint's Paragraph group and Orientation on Excel's Alignment group — the same command in two vocabularies, and Fluent's rotated glyph is what both applications draw. **Since Word's Table Layout unit, also Text Direction** in Alignment, large, which turns the selected cells' text a quarter further with each press: the same command, so the same glyph, and 24 too. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Text Direction**, a large dropdown over Horizontal, Rotate all text 90°, Rotate all text 270° and Stacked. `GUESS:`.",
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
    why: "Top Align, on Excel's Alignment group — the first of the three vertical alignments Excel puts above the three horizontal ones. Both variants: all three are toggles, which unit 2 could not draw because the group's essential slots were spent on Left, Centre and Right. **Since PowerPoint's Table Layout unit, also Align Top, Centre Vertically and Align Bottom** (with `align-center-vertical` and `align-bottom`), icon-only toggles in one exclusive set in its Alignment group, Align Top pressed: the same act on a table cell. `GUESS:`.",
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
    why: "Convert to SmartArt, on PowerPoint's Paragraph group — replacing a bare `table`. `organization` was the near neighbour and was refused: SmartArt is lists, processes and cycles as well as hierarchies, and an org chart would have named one layout out of eight. The same glyph for **SmartArt** on the Insert tab in all three applications, which is the same gallery; 24 since unit 3, because PowerPoint draws it large. **Since Word's Picture Format unit, also Picture Layout**, small, in Picture Styles, which turns the selected pictures into a SmartArt picture layout: the same act as Convert to SmartArt, from a picture rather than from text. `GUESS:`.",
  },
  {
    name: 'shapes',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Shapes, the first command of PowerPoint's Drawing group. Three overlapping outlines, which is what the gallery underneath it contains. 24 since unit 3, where Shapes is a `size=\"large\"` command of the Insert tab's Illustrations group in all three applications. **Since PowerPoint's Shape Format unit, also Insert Shapes' Shapes**, large, standing where Office draws its in-ribbon shape gallery, and opening the same whole gallery Insert's opens.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: "Merge & Centre, on Excel's Alignment group — replacing a bare `table`. Two cells becoming one is the command's whole content, and it is what separates it from every other table glyph in this subset. **Since Word's Table Layout unit, also Merge Cells** in the Merge group, small and a survivor: the same act in Word, and rule 2's standard refuses only a glyph that is a *different* command's. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Merge Cells**, large and a survivor, beside Split Cells: the same act on a slide, so 24 too. `GUESS:` large.",
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
    sizes: [20, 24],
    variants: ['regular'],
    why: 'Delete, beside it, and refused the same two specific alternatives for the same reason. A table with a dismiss mark is as broad as the command is. **Since Word\'s Table Layout unit, also Delete** in Rows & Columns, a large dropdown whose menu deletes cells, columns, rows or the table: as broad as Excel\'s, so the same glyph, and 24 too. `GUESS:`. **Since PowerPoint\'s Table Layout unit, also its Delete**, the same large dropdown, over columns, rows or the table. `GUESS:`.',
  },
  {
    name: 'table-settings',
    sizes: [20],
    variants: ['regular'],
    why: "Format, the third of Excel's Cells group — row height, column width, hide and unhide, protection. A cog on a table, which is the only honest thing to draw for a menu that broad. **Since Word's Table Layout unit, also Properties** in the Table group, small, which opens Table Properties: a table's settings, the same picture. `GUESS:`. **Since Excel's Table Design unit, also its Properties**, small, in External Table Data, which opens External Data Properties: how the table refreshes and keeps its layout, a table's settings. Data's Properties, the same dialog, carries no glyph, and that unit is left as it is. `GUESS:`.",
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
    why: "Clear, on Excel's Editing group — the command that removes contents, formats, comments or all three. Distinct from `clear-formatting` one group away, which removes formatting alone; see that row on why the two are not drawn alike. **Since unit 4, also Eraser** on the Draw tab's Write group in all three applications: a large toggle, so 24, and `filled` for its pressed state — in Word and PowerPoint the face of a `<mjx-split-button toggle>`, in Excel a plain toggle. Two commands with one eraser, and no collision: neither is a survivor, so rule 2's glyph standard never asks which one a bare eraser means. **Since Word's Table Layout unit, also Eraser** in Table Layout's Draw group, a large plain toggle that erases cell borders, in one set with Draw Table that may hold none; no survivor either, so the same holds. `GUESS:`. **Since PowerPoint's Table Design unit, also its Eraser**, the large toggle in Draw Borders, beside Draw Table. `GUESS:`.",
  },
  {
    name: 'arrow-down',
    sizes: [20],
    variants: ['regular'],
    why: "Fill, on Excel's Editing group. A judgement: the menu's default and overwhelmingly commonest entry is Fill Down, and a plain down arrow is what Office puts on the button. `arrow-download` was refused because a tray under the arrow says *save this to my machine*, and `drop` because a droplet reads as colour. **Since unit 7, also Move Earlier's partner, Move Later**, on PowerPoint's Animations tab, labelled: the animation moves down the list. **Since Word's Outlining unit, also Move Down**, icon-only, in Outlining Tools, the same verb; that this is already Fill's glyph is why Move Down does not survive a collapse. `GUESS:`.",
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
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Page Break, on Word's Pages group. Fluent draws the break itself — two page edges and the gap between them — which is Office's own picture of the command. **Since unit 5, also Breaks** on Word's Layout and Excel's Page Layout, whose first entry is that page break. Small in both. **Since Excel's View unit, also Page Break Preview**, the large toggle in Excel's View Workbook Views group, so 24 and the filled drawing: the breaks the view paints over the sheet. `GUESS:`.",
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
    why: "Text Box, the large headline of the Insert tab's Text group in all three applications. Fluent spells it as one word and draws a box with lines of text in it; `text-box-settings`, the only other candidate, is a cog on a box. **Since PowerPoint's Shape Format unit, also Insert Shapes' Text Box**, small, the same command beside the shape gallery.",
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
    why: "Equation, the large first command of the Symbols group in all three applications. Fluent draws a formula with an operator in it. Deliberately not `math-symbols`, which is a calculator's four operators and would read as *calculate*. **Since unit 6, also Insert Function** on Excel's Formulas tab, large: the drawing is *fx*, which is Office's own mark for that command. **Since Word's Table Layout unit, also Formula** in the Data group, small, which opens the Formula dialog: *fx* again, the command that writes a field computing a cell. `GUESS:`.",
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
    why: "Slicer, on Excel's Filters group, large. A slicer is a set of filter buttons, and the funnel is the filter mark across the whole of Office. **Since unit 7, also Filter** on Excel's Data tab: a large toggle, so the filled drawing too. Sharing the funnel with Slicer is why Filter does not survive a collapse. **Since Excel's Table Design unit, also Insert Slicer**, large, in its Tools group: the same Slicer command, reached from the table. `GUESS:`.",
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
    variants: ['regular', 'filled'],
    why: "Colours, the theme colour sets, on Word's Design (large) and Excel's Page Layout (small): a painter's palette, which is what a set of theme colours is. Not `color-fill` or `color-line`, which set one colour on one thing. **Since PowerPoint's View unit, also Colour**, the small toggle in the Colour/Greyscale group, so the filled drawing too. `GUESS:`. **Since PowerPoint's Slide Master unit, also Edit Theme's Colours**, small, over the same list. **Since PowerPoint's Handout Master unit, also Handout Master's Colours**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Colours**, the same shared group. **Since Word's Picture Format unit, also Colour**, the large dropdown in Adjust, which opens Colour Saturation, Colour Tone and Recolour: a painter's palette, a picture's colours. Design's Colours draws it too, and neither is a survivor, so rule 2 never asks which one a bare palette means. `GUESS:`.",
  },
  {
    name: 'text-font',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Fonts, the theme font pairs, on Word's Design (large) and Excel's Page Layout (small): two letters of two sizes, a heading font over a body font. Not `text-color` or `font-increase`, which act on the selected text. **Since PowerPoint's Slide Master unit, also Edit Theme's Fonts**, small, over the same list. **Since PowerPoint's Handout Master unit, also Handout Master's Fonts**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Fonts**, the same shared group.",
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
    why: "Margins, on Word's Layout and Excel's Page Layout, large: a page with its margins dashed in, which is Office's picture of the command. **Since Word's Print Preview unit, also Print Preview's Margins**, the same command and Layout's list.",
  },
  {
    name: 'orientation',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Orientation, on Word's Layout and Excel's Page Layout, large: a portrait page turning to landscape. Not `document-landscape`, which shows one answer rather than the choice. **Since Word's Print Preview unit, also Print Preview's Orientation**, the same command and Layout's list. **Since PowerPoint's Print Preview unit, also PowerPoint's**, small, the same list. **Since PowerPoint's Handout Master unit, also Handout Orientation**, large, in Page Setup: the same choice, over Layout's list. **Since PowerPoint's Notes Master unit, also Notes Page Orientation**, large, in Page Setup: the same choice, over Layout's list.",
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
    why: "Bring Forward, the split button on Arrange in Word (small) and Excel (large): the hatched shape moving in front of the plain one. **Since PowerPoint's Table Layout unit, also its Arrange Bring Forward**, small, through the census's shared `arrangeCommands`; with it Send Backward's `position-backward` and Align's `align-left`. `GUESS:`.",
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
    why: "Slide Size, on PowerPoint's Customise group, large: a frame with an arrow to its corner, the slide being resized. **Since PowerPoint's Slide Master unit, also Slide Master's Size**, large, the same command and list. **Since PowerPoint's Handout Master unit, also Handout Master's Slide Size**, large, in Page Setup: the same command and list. **Since PowerPoint's Notes Master unit, also Notes Master's Slide Size**, large, in Page Setup: the same command and list.",
  },
  {
    name: 'color-background',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Format Background, on PowerPoint's Customise group, large: a paint bucket over a frame, the fill behind a whole slide rather than on a shape (`color-fill`). **Since PowerPoint's Slide Master unit, also Background Styles** on the Background group, small, the glyph the Variants footer already draws for it. **Since PowerPoint's Handout Master unit, also Handout Master's Background Styles**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Background Styles**, the same shared group.",
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
    why: "Preview Results, on Word's Mailings tab: a large toggle, so 24 and the filled drawing. An eye is *show*; that is also why it does not survive a collapse unlabelled. **Since Excel's View unit, also Unhide**, small, in Excel's View Window group: a hidden window brought back into sight. `GUESS:`.",
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
    why: "Move Earlier, on PowerPoint's Animations tab, labelled: the animation moves up the list. **Since Word's Outlining unit, also Move Up**, icon-only, in Outlining Tools: the paragraph moves up the outline, the same verb. `GUESS:`.",
  },
  {
    name: 'globe',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "From Web, on Excel's Data tab: the web, as everywhere in Office. **Since Word's View unit, also Web Layout**, a large toggle on Word's View tab, so 24 and the filled drawing: Office's own Web Layout glyph is a page with a globe on it. `GUESS:`.",
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
    why: "Refresh All, on Excel's Data tab, large: the refresh arrow. Not `arrow-sync`, which is AutoSave. **Since Excel's Table Design unit, also its Refresh**, the large split button in External Table Data, whose face refreshes the table from its source: the same arrow. `GUESS:`.",
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
    why: "Show Detail, on Excel's Outline group: the plus in a box the outline draws in the margin. **Since Word's Outlining unit, also Expand**, icon-only, in Outlining Tools: the same verb on Word's outline. `GUESS:`.",
  },
  {
    name: 'subtract-square',
    sizes: [20],
    variants: ['regular'],
    why: "Hide Detail, beside Show Detail: the minus in a box. **Since Word's Outlining unit, also Collapse**, icon-only, in Outlining Tools, beside Expand. `GUESS:`.",
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
    why: "Track Changes, the large split button on Word's Review tab: a page with a pencil, edits recorded against the page. Its face turns tracking on and off (`<mjx-split-button toggle>`), so it draws pressed and needs the filled drawing. **Since Excel's Review, also Track Changes** there, small: Office 2016's legacy dropdown of Highlight Changes and Accept/Reject Changes, the same idea in a workbook.",
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
  // ── Review (Excel's Review) ─────────────────────────────────────────────────
  //
  // Excel's Review tab, authored after PowerPoint's. Every other glyph it draws is a row above; three of
  // those gained a line of `why` (Workbook Statistics, Protect Workbook, Track Changes). Every command drawn
  // without an icon is listed, with its reason, in the *Excel's Review* part of `dev/ribbons/census.ts`.
  {
    name: 'top-speed',
    sizes: [20],
    variants: ['regular'],
    why: "Check Performance, in Excel's Review Performance group, small: a speedometer, Office's own gauge for the Workbook Performance pane. No other command in this subset draws a gauge. `GUESS:`.",
  },
  {
    name: 'note',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Notes, the large dropdown in Excel's Review tab: a sticky note, what Microsoft 365 renamed the legacy cell comment to. Not `comment`, which is a threaded comment everywhere in this subset.",
  },
  {
    name: 'table-lock',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Protect Sheet, large, in Excel's Review Protect group: a grid with a padlock, one sheet locked. Not `document-lock`, which is the whole workbook beside it. Office relabels the command Unprotect Sheet rather than drawing it pressed, so regular alone. `GUESS:`.",
  },
  // ── View (Word's View) ────────────────────────────────────────────────────
  //
  // Word's View tab, one application under the one-tab-one-application rule. **20 for a small command,
  // 20 and 24 for a large one, and `filled` for every toggle**, which draws filled while it holds. `globe`
  // above gained a command rather than a row here. Every command the tab draws without an icon is listed,
  // with its reason, in the *commands View shows* section of `dev/ribbons/census.ts`.
  {
    name: 'book-open',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Read Mode, the large toggle on Word's View tab: an open book, Office's own picture for the reading view. `GUESS:`. **Since PowerPoint's View unit, also Reading View**, the large toggle in Presentation Views: the same picture for the same idea.",
  },
  {
    name: 'document-one-page',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Print Layout, the large toggle on Word's View tab, pressed in a new document: one printed page. `GUESS:`. **Since Excel's View unit, also Page Layout**, the large toggle in Excel's Workbook Views group: the sheet as printed pages, the same idea.",
  },
  {
    name: 'list-bar-tree-offset',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Outline, the small toggle in Word's View Document Views group: bars stepping one level deeper each, the heading hierarchy the view shows. Not `text-bullet-list-tree`, which is Multilevel List. `GUESS:`. **Since PowerPoint's View unit, also Outline View**, small there too, because Fluent draws no 24.",
  },
  {
    name: 'drafts',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Draft, the small toggle in Word's View Document Views group: lines of text with a pencil and no page, the text edited with the layout taken away. Not plain lines, which are every alignment glyph, or `document-one-page`, which is Print Layout. `GUESS:`.",
  },
  {
    name: 'full-screen-maximize',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Focus, the large toggle in Word's View Modes group: four corners pushed outward, the page filling the screen. `GUESS:`.",
  },
  {
    name: 'immersive-reader',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Immersive Reader, the large button in Word's View Modes group: Fluent's own mark for the product.",
  },
  {
    name: 'ratio-one-to-one',
    sizes: [20, 24],
    variants: ['regular'],
    why: "100%, the large button in Word's View Zoom group, and a survivor: *1:1*, actual size. Not `zoom-in`, which is a different command. `GUESS:` that it reads unlabelled. **Since Excel's View unit, also Excel's 100%**, large and a survivor there too. **Since Word's Print Preview unit, also Print Preview's 100%**, large and a survivor, as on View.",
  },
  {
    name: 'document-fit',
    sizes: [20],
    variants: ['regular'],
    why: "One Page, in Word's View Zoom group, and a survivor: a page inside four fit corners, one whole page fitted to the window. `GUESS:`. **Since Word's Print Preview unit, also Print Preview's One Page**, a survivor, as on View.",
  },
  {
    name: 'auto-fit-width',
    sizes: [20],
    variants: ['regular'],
    why: "Page Width, in Word's View Zoom group, and a survivor: a width between two stops, the page's width fitted to the window. `GUESS:`. **Since Word's Print Preview unit, also Print Preview's Page Width**, a survivor, as on View.",
  },
  {
    name: 'window-new',
    sizes: [20, 24],
    variants: ['regular'],
    why: "New Window, the large button in Word's View Window group: a window with an arrow leaving it, a second window on the same document. **Since PowerPoint's View unit, also PowerPoint's New Window**, large, the same command. **Since Excel's View unit, also Excel's New Window**.",
  },
  {
    name: 'split-horizontal',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Split, the large button in Word's View Window group: a window cut across, where Word puts the split bar. A plain button, because Office relabels it Remove Split rather than drawing it pressed. `GUESS:`. **Since Excel's View unit, also Excel's Split**, a small toggle Excel draws pressed while the window is split, so the filled drawing too.",
  },
  {
    name: 'column-double-compare',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "View Side by Side, the small toggle in Word's View Window group: two panes set beside each other to be compared. Fluent draws it at 20 alone, which is the size the command is. `GUESS:`. **Since Excel's View unit, also Excel's View Side by Side**, the same toggle.",
  },
  {
    name: 'window-multiple',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Switch Windows, the large dropdown in Word's View Window group: windows overlapping, one of which to bring forward. `GUESS:`. **Since PowerPoint's View unit, also PowerPoint's Switch Windows**, large, the same dropdown. **Since Excel's View unit, also Excel's Switch Windows**.",
  },
  {
    name: 'dark-theme',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Switch Modes, the large toggle in Word's View Night Mode group: a circle half dark, the page's light and dark modes. `GUESS:`. **Since Excel's View unit, also Excel's Switch Modes**, in the Night Mode group read as Word's. **Since PowerPoint's Black and White unit, also Colour Mode's Inverse Greyscale**, a large toggle: light and dark exchanged. Never on one ribbon with Word's or Excel's, and not `circle-half-fill`, which View's Black and White draws on PowerPoint's. `GUESS:`.",
  },
  {
    name: 'brightness-high',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Light Greyscale, the large toggle in PowerPoint's Black and White Colour Mode group (and Greyscale's, the same shared group): a sun with long rays, greyscale lightened. Not `weather-sunny`, which reads as weather, nor `color-off`, which Greyscale beside it draws. `GUESS:`. **Since Word's Picture Format unit, also Corrections**, the large dropdown in Adjust, which opens Sharpen/Soften and Brightness/Contrast: a sun, brightness, the half of the gallery a person reaches for. Not a survivor, so the shared sun collides with nothing. `GUESS:`.",
  },
  // ── View (PowerPoint's View) ──────────────────────────────────────────────
  //
  // PowerPoint's View tab, one application under the one-tab-one-application rule. The same sizing as Word's
  // View: **20 for a small command, 20 and 24 for a large one, and `filled` for every toggle.** `book-open`,
  // `list-bar-tree-offset`, `window-new`, `window-multiple` and `color` above gained a command rather than a
  // row here. The reasoning for every glyph is the *PowerPoint's View* part of `dev/ribbons/census.ts`.
  {
    name: 'panel-left',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Normal, the large toggle in PowerPoint's View Presentation Views group, pressed in a new deck: a window with a narrow pane at its left, the thumbnails beside the slide. Not `panel-left-text`, Word's Reviewing Pane. `GUESS:`.",
  },
  {
    name: 'slide-grid',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Slide Sorter, the large toggle in PowerPoint's View Presentation Views group: slides in a grid. `GUESS:`.",
  },
  {
    name: 'notepad',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Notes Page, the large toggle in PowerPoint's View Presentation Views group: a page of lines. Not `note`, Excel's Notes. `GUESS:`.",
  },
  {
    name: 'slide-text-edit',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Slide Master, the large button in PowerPoint's View Master Views group: a slide with a pencil, the slide's layout edited. It opens a tab rather than holding a state, so regular alone. `GUESS:`.",
  },
  {
    name: 'document-one-page-multiple',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Handout Master, the large button in PowerPoint's View Master Views group: pages stacked, copies to hand out. Word refused it for Multiple Pages for that reading. `GUESS:`.",
  },
  {
    name: 'notepad-edit',
    sizes: [20],
    variants: ['regular'],
    why: "Notes Master, in PowerPoint's View Master Views group: Notes Page's page with a pencil. Fluent draws it at 20 alone, so the command is small where Office draws it large. `GUESS:`.",
  },
  {
    name: 'panel-bottom',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Notes, the toggle in PowerPoint's View Show group: a window with a pane at its foot, where the notes pane opens. Fluent draws it at 20 alone, so the command is small. `GUESS:`.",
  },
  {
    name: 'zoom-in',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Zoom, the large button in PowerPoint's View Zoom group, which opens the Zoom dialog: a magnifier. No Zoom In command is in this subset. `GUESS:`. **Since Excel's View unit, also Excel's Zoom**, the same dialog. **Since Word's Print Preview unit, also Print Preview's Zoom**, large, the same dialog; Word's View draws none, and the census records the disagreement. **Since PowerPoint's Print Preview unit, also PowerPoint's Print Preview Zoom**, large, View's command under that tab's id. **Since Excel's Print Preview unit, also Excel's Print Preview Zoom**, large, which opens no dialog and switches the preview between the whole page and a magnified page: a magnifier is what the press does. `GUESS:`.",
  },
  {
    name: 'page-fit',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Fit to Window, the large button in PowerPoint's View Zoom group, and a survivor: a landscape frame inside fit corners, the slide fitted to the window. `GUESS:` that it reads unlabelled. **Since PowerPoint's Print Preview unit, also that tab's Fit to Window**, large and a survivor, as on View.",
  },
  {
    name: 'color-off',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Greyscale, the small toggle in PowerPoint's View Colour/Greyscale group: the Colour palette struck through, the colour taken out. `GUESS:`. **Since PowerPoint's Black and White unit, also Colour Mode's Greyscale**, a large toggle in the colour-mode tabs' exclusive set, so the 24 too: the same palette with its colour taken out.",
  },
  {
    name: 'circle-half-fill',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Black and White, the small toggle in PowerPoint's View Colour/Greyscale group: a circle half black and half white. Not `dark-theme`, Word's Switch Modes. `GUESS:`.",
  },
  {
    name: 'layout-column-two',
    sizes: [20],
    variants: ['regular'],
    why: "Arrange All, in PowerPoint's View Window group: two windows side by side, as PowerPoint tiles them. `GUESS:`. **Since Excel's View unit, also Reset Window Position**, small, in Excel's View Window group: two equal columns, the compared windows sharing the screen equally again. Excel's Arrange All draws `layout-cell-four` instead.",
  },
  {
    name: 'stack',
    sizes: [20],
    variants: ['regular'],
    why: "Cascade, in PowerPoint's View Window group: squares offset down a diagonal, windows cascaded. `GUESS:`.",
  },
  {
    name: 'arrow-move',
    sizes: [20],
    variants: ['regular'],
    why: "Move Split, in PowerPoint's View Window group: four arrows, the keys that move the split bars between panes. `GUESS:`.",
  },
  {
    name: 'text-direction-horizontal-ltr',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Left-to-Right, the small toggle in PowerPoint's View Direction group, pressed: a letter and an arrow pointing right. `GUESS:`, as the whole group is.",
  },
  {
    name: 'text-direction-horizontal-rtl',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Right-to-Left, the small toggle in PowerPoint's View Direction group: a letter and an arrow pointing left. `GUESS:`, as the whole group is.",
  },
  // ── View (Excel's View) ───────────────────────────────────────────────────
  //
  // Excel's View tab, one application under the one-tab-one-application rule. The same sizing as Word's and
  // PowerPoint's View: **20 for a small command, 20 and 24 for a large one, and `filled` for every toggle.**
  // `save`, `add`, `document-page-break`, `document-one-page`, `zoom-in`, `ratio-one-to-one`, `window-new`,
  // `split-horizontal`, `eye`, `column-double-compare`, `layout-column-two`, `window-multiple` and
  // `dark-theme` above gained a command rather than a row here; `settings` already says *Options*. The
  // reasoning for every glyph is the *Excel's View* part of `dev/ribbons/census.ts`.
  {
    name: 'arrow-exit',
    sizes: [20],
    variants: ['regular'],
    why: "Exit, small, in Excel's View Sheet View group: an arrow leaving a frame, out of the temporary sheet view and back to Default. `GUESS:`.",
  },
  {
    name: 'grid',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Normal, the large toggle in Excel's View Workbook Views group, pressed in a new workbook: cells in rows and columns, the sheet with nothing laid over it. Not `table`, Insert's Table. `GUESS:`.",
  },
  {
    name: 'window-bullet-list',
    sizes: [20],
    variants: ['regular'],
    why: "Custom Views, in Excel's View Workbook Views group: a window holding a list, the named views the dialog keeps. Fluent draws it at 20 alone, so the command is small where Office draws it large. `GUESS:`.",
  },
  {
    name: 'zoom-fit',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Zoom to Selection, the large button in Excel's View Zoom group, and a survivor: a magnifier inside fit corners, the selected cells fitted to the window. Not `zoom-in`, Zoom's. `GUESS:` that it reads unlabelled.",
  },
  {
    name: 'layout-cell-four',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Arrange All, the large button in Excel's View Window group: four windows tiled, the first arrangement in the Arrange Windows dialog it opens. `GUESS:`. **Since PowerPoint's Handout Master unit, also Slides Per Page**, large, in Handout Master's Page Setup: a page divided into frames, slides laid out on one handout. Not `slide-grid`, Slide Sorter, nor `document-one-page-multiple`, Handout Master itself on View. The two uses are never on one ribbon. `GUESS:`.",
  },
  {
    name: 'table-freeze-column-and-row',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Freeze Panes, the large dropdown in Excel's View Window group, and the first entry of its menu: a grid with its top row and first column held, Fluent's own drawing of the command.",
  },
  {
    name: 'table-freeze-row',
    sizes: [20],
    variants: ['regular'],
    why: "Freeze Top Row, the second entry of Excel's Freeze Panes menu: a grid with its top row held.",
  },
  {
    name: 'table-freeze-column',
    sizes: [20],
    variants: ['regular'],
    why: "Freeze First Column, the third entry of Excel's Freeze Panes menu: a grid with its first column held.",
  },
  {
    name: 'eye-off',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Hide, small, in Excel's View Window group: an eye struck through, the window put out of sight. Unhide draws `eye`. `GUESS:`. **Since PowerPoint's Black and White unit, also Colour Mode's Don't Show**, a large toggle in an exclusive set, so the 24 and the filled drawing too: the object not drawn in this colour mode.",
  },
  {
    name: 'arrow-bidirectional-up-down',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Synchronous Scrolling, the small toggle in Excel's View Window group: an arrow up and down, scrolling. Not `arrow-sync`, AutoSave's, which is why Word's Synchronous Scrolling carries none. `GUESS:`.",
  },
  // ── Slide Show (PowerPoint's) ─────────────────────────────────────────────
  //
  // PowerPoint's Slide Show tab, the only application with one. The View tabs' sizing: **20 and 24 for a large
  // command, and `filled` for the toggle.** Every command here is large. `presenter` above gained a command and a
  // 24 drawing rather than a row here. The reasoning for every glyph is the *commands Slide Show shows* section
  // of `dev/ribbons/census.ts`.
  {
    name: 'slide-multiple-arrow-right',
    sizes: [20, 24],
    variants: ['regular'],
    why: "From Beginning, the large button in PowerPoint's Slide Show Start Slide Show group: stacked slides with an arrow forward, the whole deck played through. Not `previous`, Mailings' First Record. `GUESS:`.",
  },
  {
    name: 'slide-play',
    sizes: [20, 24],
    variants: ['regular'],
    why: "From Current Slide, the large button in PowerPoint's Slide Show Start Slide Show group: one slide with a play mark, this slide played. `GUESS:`.",
  },
  {
    name: 'slide-text-multiple',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Custom Slide Show, the large dropdown in PowerPoint's Slide Show Start Slide Show group: slides stacked, a chosen part of the deck. Not `slide-multiple`, File's Publish Slides. `GUESS:`.",
  },
  {
    name: 'person-voice',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Rehearse with Coach, the large button in PowerPoint's Slide Show Rehearse group: a figure speaking, which is what Speaker Coach listens to. `GUESS:`, as the command is.",
  },
  {
    name: 'slide-settings',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Set Up Slide Show, the large button in PowerPoint's Slide Show Set Up group, which opens its dialog: a slide with a cog. `GUESS:`.",
  },
  {
    name: 'slide-hide',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Hide Slide, the large toggle in PowerPoint's Slide Show Set Up group, and a survivor: a slide in a dashed outline, the picture Office's thumbnail rail draws for a hidden slide. `GUESS:` that it reads unlabelled.",
  },
  {
    name: 'timer',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Rehearse Timings, the large button in PowerPoint's Slide Show Set Up group: a stopwatch, the timer the rehearsal runs. `GUESS:`.",
  },
  {
    name: 'slide-record',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Record, the large split button in PowerPoint's Slide Show Set Up group: a slide with the record mark. Not `record`, Insert's Screen Recording, which records the screen rather than the slides. `GUESS:`.",
  },
  // ── Recording (PowerPoint's) ──────────────────────────────────────────────
  //
  // PowerPoint's Recording tab, the only application with one. Every command here is large, so **20 and 24**,
  // and nothing is a toggle, so no `filled`. Eight commands reuse a glyph because they are the same command as
  // one drawn elsewhere (`slide-multiple-arrow-right`, `slide-play`, `slide-record`, `screenshot`, `record`,
  // `camera`, `video`, `speaker-2`, `question-circle`), and `delete`, `arrow-reset` and `arrow-export` above
  // gained a 24 drawing rather than a row here. The reasoning for every glyph is the *commands Recording shows*
  // section of `dev/ribbons/census.ts`.
  {
    name: 'save-arrow-right',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Save as Show, the large button in PowerPoint's Recording Save group: a save carried onward, a copy that opens straight into the show. Not `save-edit`, File's Save As. `GUESS:`.",
  },
  {
    name: 'video-clip',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Export to Video, the large button in PowerPoint's Recording Save group: the clip the export writes. Not `video`, which is Video in Auto-play Media on the same tab and inserts one. `GUESS:`.",
  },
  {
    name: 'play-circle',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Preview, the large button in PowerPoint's Recording Preview group, which plays the current slide's recording: a play mark. Not `slide-transition`, Transitions' Preview, which previews a transition. `GUESS:`.",
  },
  // ── Outlining (Word's) ────────────────────────────────────────────────────
  //
  // Word's Outlining tab, the only application with one, and the first view tab authored. **20 for a small or
  // icon-only command, 20 and 24 for a large one, and `filled` for every toggle.** `arrow-up`, `arrow-down`,
  // `add-square` and `subtract-square` above gained a command rather than a row here. The reasoning for every
  // glyph is the *commands Outlining shows* section of `dev/ribbons/census.ts`.
  {
    name: 'arrow-previous',
    sizes: [20],
    variants: ['regular'],
    why: "Promote to Heading 1, icon-only, in Word's Outlining Tools: an arrow out, stopped at a wall, all the way out to the top level. Not `chevron-double-left`, a panel's collapse chevron. `GUESS:`.",
  },
  {
    name: 'arrow-left',
    sizes: [20],
    variants: ['regular'],
    why: "Promote, icon-only, in Word's Outlining Tools, and a survivor: an arrow out, one level. `GUESS:` that it reads unlabelled.",
  },
  {
    name: 'arrow-right',
    sizes: [20],
    variants: ['regular'],
    why: "Demote, icon-only, in Word's Outlining Tools, and a survivor: an arrow in, one level. `GUESS:` that it reads unlabelled.",
  },
  {
    name: 'arrow-next',
    sizes: [20],
    variants: ['regular'],
    why: "Demote to Body Text, icon-only, in Word's Outlining Tools: an arrow in, stopped at a wall, all the way in to body text. `GUESS:`.",
  },
  {
    name: 'document-multiple',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Show Document, the large toggle in Word's Outlining Master Document group, pressed: pages stacked, a master document and its subdocuments. `GUESS:`.",
  },
  {
    name: 'arrow-collapse-all',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Collapse Subdocuments, the small toggle in Word's Outlining Master Document group: lines drawn in towards each other, every subdocument folded to its link. `GUESS:`.",
  },
  {
    name: 'document-add',
    sizes: [20],
    variants: ['regular'],
    why: "Create, small, in Word's Outlining Master Document group: a new subdocument made from the selected heading. `GUESS:`.",
  },
  {
    name: 'document-arrow-left',
    sizes: [20],
    variants: ['regular'],
    why: "Insert, small, in Word's Outlining Master Document group: a document with an arrow, an existing file brought in as a subdocument. `GUESS:`.",
  },
  {
    name: 'link-dismiss',
    sizes: [20],
    variants: ['regular'],
    why: "Unlink, small, in Word's Outlining Master Document group: a link struck off, the subdocument's text copied in and its file let go. `GUESS:`. **Since Excel's Table Design unit, also its Unlink**, small, in External Table Data: the table's tie to its SharePoint list let go, the same struck link. `GUESS:`.",
  },
  {
    name: 'merge',
    sizes: [20],
    variants: ['regular'],
    why: "Merge, small, in Word's Outlining Master Document group: two paths joining, the selected subdocuments made one. `GUESS:`.",
  },
  {
    name: 'arrow-split',
    sizes: [20],
    variants: ['regular'],
    why: "Split, small, in Word's Outlining Master Document group: one path dividing, a subdocument split at the selection. Not `split-horizontal`, View's Split, which splits a window. `GUESS:`.",
  },
  {
    name: 'lock-closed',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Lock Document, the small toggle in Word's Outlining Master Document group: a padlock, filled while the subdocument is locked. Not `document-lock`, Protect Document and Restrict Editing. `GUESS:`.",
  },
  {
    name: 'dismiss-square',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Close Outline View, the large button in Word's Outlining Close group: a cross in a square, Office's own picture of the command. Not the plain `dismiss`, which closes a panel, a chip or a dialog. `GUESS:`. **Since Word's Print Preview unit, also Close Print Preview**, large: the same verb, leaving a view. **Since PowerPoint's Print Preview unit, also PowerPoint's Close Print Preview**, the same command. **Since Excel's Print Preview unit, also Excel's**, the same command. **Since PowerPoint's Slide Master unit, also Close Master View**, large: the same verb, leaving a view. **Since PowerPoint's Handout Master unit, also Handout Master's Close Master View**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Close Master View**, the same shared group. **Since PowerPoint's Black and White unit, also Back To Colour View**, large, in the colour-mode tabs' shared Close group: the same verb, leaving a view. Not `color`, which View's Colour draws.",
  },
  // ── Print Preview (Word's) ────────────────────────────────────────────────
  //
  // Word's Print Preview tab, the second view tab authored. **20 for a small command.** `print`, `settings`,
  // `document-margins`, `orientation`, `zoom-in`, `ratio-one-to-one`, `document-fit`, `auto-fit-width` and
  // `dismiss-square` above gained a command rather than a row here. The reasoning for every glyph is the
  // *commands Print Preview shows* section of `dev/ribbons/census.ts`.
  {
    name: 'arrow-minimize-vertical',
    sizes: [20],
    variants: ['regular'],
    why: "Shrink One Page, small, in Word's Print Preview Preview group: two arrows pressed together from above and below, the document squeezed until its last page is gone. `GUESS:`.",
  },
  {
    name: 'document-arrow-down',
    sizes: [20],
    variants: ['regular'],
    why: "Next Page, small, in Word's Print Preview Preview group, and a survivor: a page, and the direction Word's pages run. Not `caret-down`, a dropdown's arrow. `GUESS:` that it reads as a page rather than *download*. **Since PowerPoint's Print Preview unit, also PowerPoint's Next Page**, a survivor there too. **Since Excel's Print Preview unit, also Excel's Next Page**, a survivor there too.",
  },
  {
    name: 'document-arrow-up',
    sizes: [20],
    variants: ['regular'],
    why: "Previous Page, small, in Word's Print Preview Preview group, and a survivor, beside Next Page. `GUESS:` that it reads as a page rather than *upload*. **Since PowerPoint's Print Preview unit, also PowerPoint's Previous Page**, a survivor there too. **Since Excel's Print Preview unit, also Excel's Previous Page**, a survivor there too.",
  },
  // ── Background Removal (Word's, and written for all three) ─────────────────
  //
  // Word's Background Removal tab, the third view tab authored. All four commands are large, so **20 and 24**,
  // and the two pencils are toggles, so **`filled` as well** for the pressed state. PowerPoint's and Excel's
  // units reuse these rows. The reasoning for every glyph is the *commands Background Removal shows* section of
  // `dev/ribbons/census.ts`.
  {
    name: 'add-circle',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Mark Areas to Keep, a large toggle in Background Removal's Refine group: Office's pencil with a plus, keeping the plus because Fluent draws no pencil with one. Not `add-square`, which is Expand and Show Detail. `GUESS:`.",
  },
  {
    name: 'subtract-circle',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "Mark Areas to Remove, a large toggle beside Mark Areas to Keep: Office's pencil with a minus, keeping the minus. Not `subtract-square`, which is Collapse and Hide Detail. `GUESS:`.",
  },
  {
    name: 'dismiss-circle',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Discard All Changes, large, in Background Removal's Close group: a cross, every mark thrown away. Not `dismiss-square`, which leaves a view with nothing discarded, nor `arrow-undo`, one step. `GUESS:`.",
  },
  {
    name: 'checkmark-circle',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Keep Changes, large, beside Discard All Changes: a tick, the background removed. Not `document-checkmark`, which is Review's Accept. `GUESS:`.",
  },
  // ── Slide Master (PowerPoint's) ───────────────────────────────────────────
  //
  // Six new glyphs; Delete, Insert Layout, Colours, Fonts, Effects, Background Styles, Slide Size and Close Master
  // View reuse rows above, each of which says so. `dev/ribbons/census.ts` gives the reasoning for every choice.
  {
    name: 'slide-text-title-add',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Insert Slide Master, large, in Slide Master's Edit Master group: a slide with a title bar and a plus, the slide that carries the title placeholder, added. Not `slide-add`, which is New Slide. `GUESS:`. **Since PowerPoint's Slide Master Home unit, also Insert Slide Master there**, large, in Master Slides: the same command under that tab's id.",
  },
  {
    name: 'rename',
    sizes: [20],
    variants: ['regular'],
    why: "Rename, small, in Slide Master's Edit Master group, which opens the Rename Layout dialog: a text cursor in a field. `GUESS:`.",
  },
  {
    name: 'pin',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Preserve, the small toggle in Slide Master's Edit Master group: a pushpin, which is Office's own picture of the command, filled while the master is preserved. `GUESS:`.",
  },
  {
    name: 'slide-text-title-checkmark',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Master Layout, large, in Slide Master's Master Layout group, which opens a dialog of ticks for the placeholders a master carries: a slide with a title and a tick. Not `slide-settings`, Set Up Slide Show. `GUESS:`.",
  },
  {
    name: 'slide-content',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Insert Placeholder, the large split button in Slide Master's Master Layout group: a slide holding a picture and lines, the Content placeholder its face inserts. `GUESS:`.",
  },
  {
    name: 'style-guide',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Themes, large, in Slide Master's Edit Theme group: a fanned swatch book, colours, fonts and effects chosen as one set. Not `color`, Colours beside it, nor `design-ideas`, Designer. Word's and Excel's Themes carry none. `GUESS:`. **Since PowerPoint's Handout Master unit, also Handout Master's Themes**, the same shared group. **Since PowerPoint's Notes Master unit, also Notes Master's Themes**, the same shared group.",
  },
  // ── Slide Master Home (PowerPoint's) ──────────────────────────────────────
  //
  // One new glyph. Clipboard, Font, Paragraph, Drawing and Editing are Home's commands and draw Home's rows unchanged;
  // Master Slides reuses `slide-text-title-add`, `slide-layout`, `arrow-reset` and `slide-multiple`, each of which
  // says so. `dev/ribbons/census.ts` gives the reasoning.
  {
    name: 'layout-row-two-split-top',
    sizes: [20],
    variants: ['regular'],
    why: "Layout, the small dropdown in Slide Master Home's Master Slides group, which applies one of the master's layouts: a frame divided into a title row and two panes, the arrangement of placeholders a layout is. Not `slide-layout`, which Insert Layout draws in the same group. `GUESS:`.",
  },
  // ── Table Design (Word's) ─────────────────────────────────────────────────
  //
  // One new glyph. Borders reuses `border-all` and Border Painter `paint-brush`, each of which says so. Every other
  // Table Design command is a checkbox, the gallery, a colour picker or a field, and carries none.
  // `dev/ribbons/census.ts` gives the reasoning.
  {
    name: 'line-style',
    sizes: [24],
    variants: ['regular'],
    why: "Border Styles, the large dropdown in Word's Table Design, which loads the pen with one of the theme's borders: three lines in three dashes, a border's style chosen from a set. Not `border-all`, which Borders draws beside it, nor `line-thickness`, which is one line's weight. Large alone, so 24 alone. `GUESS:`.",
  },
  // ── Table Layout (Word's) ─────────────────────────────────────────────────
  //
  // Twenty-four new glyphs. Delete reuses `table-dismiss` and Text Direction `text-direction-rotate-90-right`, each now
  // at 24; Properties reuses `table-settings`, Eraser `eraser`, Merge Cells `table-cells-merge`, Sort `arrow-sort` and
  // Formula `math-formula`, each of which says so. Height and Width are fields and carry none.
  // `dev/ribbons/census.ts` gives the reasoning.
  {
    name: 'table-cursor',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Select, small, in Word's Table Layout Table group, which opens Select Cell, Column, Row and Table: a table with a pointer, Office's own picture. Not `select-all-on`, Home's Select. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Select**, large, which opens Select Table, Column and Row: the same command, so 24 too. `GUESS:`.",
  },
  {
    name: 'border-inside',
    sizes: [20, 24],
    variants: ['regular', 'filled'],
    why: "View Gridlines, the small toggle in Word's Table Layout Table group, pressed: a dashed box with its inside lines, the lines that show where a borderless table's cells are. Filled while pressed. Unlabelled it reads as Inside Borders, which is why it is no survivor. `GUESS:`. **Since PowerPoint's Table Layout unit, also its View Gridlines**, a large toggle, pressed: the same command and the same reason for no survivor, so 24 in both variants. `GUESS:`.",
  },
  {
    name: 'table-edit',
    sizes: [24],
    variants: ['regular', 'filled'],
    why: "Draw Table, the large toggle in Word's Table Layout Draw group, which arms a pencil that draws cell borders: a table with a pencil. In one set with Eraser that may hold none, so filled while armed. `GUESS:`. **Since PowerPoint's Table Design unit, also its Draw Table**, the large toggle in Draw Borders: the same pencil drawing the same borders, in one exclusive set with Eraser. `GUESS:`.",
  },
  {
    name: 'table-stack-above',
    sizes: [24],
    variants: ['regular'],
    why: "Insert Above, the large button in Word's Table Layout Rows & Columns group, and a survivor: a table with a new line above it. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Insert Above**, large and a survivor, and with it **Insert Below, Left and Right draw the three rows below** on that tab too, at the same sizes. `GUESS:`.",
  },
  {
    name: 'table-stack-below',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Below, small, in Word's Table Layout Rows & Columns group, and a survivor: a table with a new line below it. `GUESS:`.",
  },
  {
    name: 'table-stack-left',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Left, small, in Word's Table Layout Rows & Columns group: a table with a new line to its left. The one insert that gives way to the survivor ceiling. `GUESS:`.",
  },
  {
    name: 'table-stack-right',
    sizes: [20],
    variants: ['regular'],
    why: "Insert Right, small, in Word's Table Layout Rows & Columns group, and a survivor: a table with a new line to its right. `GUESS:`.",
  },
  {
    name: 'table-cells-split',
    sizes: [20, 24],
    variants: ['regular'],
    why: "Split Cells, small, in Word's Table Layout Merge group, which opens Split Cells: one cell divided, the reverse of `table-cells-merge` beside it. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Split Cells**, large beside a large Merge Cells, which opens a dialog asking for rows and columns: the same command, so 24 too. `GUESS:` large.",
  },
  {
    name: 'table-split',
    sizes: [20],
    variants: ['regular'],
    why: "Split Table, small, in Word's Table Layout Merge group, and a survivor: a table cut in two along a row. `GUESS:`.",
  },
  {
    name: 'arrow-autofit-content',
    sizes: [24],
    variants: ['regular'],
    why: "AutoFit, the large dropdown in Word's Table Layout Cell Size group, which opens AutoFit Contents, AutoFit Window and Fixed Column Width: content with arrows out to its edges. Not `auto-fit-width`, View's Page Width. 24 alone. `GUESS:`.",
  },
  {
    name: 'align-space-evenly-vertical',
    sizes: [20],
    variants: ['regular'],
    why: "Distribute Rows, small, in Word's Table Layout Cell Size group, and a survivor: three equal bars stacked, rows given one height. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Distribute Rows**, with `align-space-evenly-horizontal` for Distribute Columns, both small survivors in Cell Size. `GUESS:`.",
  },
  {
    name: 'align-space-evenly-horizontal',
    sizes: [20],
    variants: ['regular'],
    why: "Distribute Columns, small, in Word's Table Layout Cell Size group, and a survivor: three equal bars side by side, columns given one width. `GUESS:`.",
  },
  {
    name: 'textbox-align-top-left',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Top Left, first of the nine icon-only cell-alignment toggles in Word's Table Layout Alignment group, one exclusive set, pressed in a new table, and a survivor: a box with its two lines at the top left. Filled while pressed. `GUESS:` for all nine.",
  },
  {
    name: 'textbox-align-middle-left',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Centre Left, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the middle left. `GUESS:`.",
  },
  {
    name: 'textbox-align-bottom-left',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Bottom Left, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the bottom left. `GUESS:`.",
  },
  {
    name: 'textbox-align-top-center',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Top Centre, a cell-alignment toggle in Word's Table Layout Alignment group, and a survivor: two lines at the top centre. `GUESS:`.",
  },
  {
    name: 'textbox-align-center',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Centre, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the centre of the box. Fluent's name for the middle of the grid; `textbox-align-middle` is the same box with lines spanning its width. `GUESS:`.",
  },
  {
    name: 'textbox-align-bottom-center',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Bottom Centre, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the bottom centre. `GUESS:`.",
  },
  {
    name: 'textbox-align-top-right',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Top Right, a cell-alignment toggle in Word's Table Layout Alignment group, and a survivor: two lines at the top right. `GUESS:`.",
  },
  {
    name: 'textbox-align-middle-right',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Centre Right, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the middle right. `GUESS:`.",
  },
  {
    name: 'textbox-align-bottom-right',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Align Bottom Right, a cell-alignment toggle in Word's Table Layout Alignment group: two lines at the bottom right. `GUESS:`.",
  },
  {
    name: 'padding-left',
    sizes: [24],
    variants: ['regular'],
    why: "Cell Margins, the large button in Word's Table Layout Alignment group, which opens Table Options: an edge, a dashed inner guide and the space between them, the room between a cell's border and its text. Not `document-margins`, which is a page's. The weakest glyph on the tab. `GUESS:`. **Since PowerPoint's Table Layout unit, also its Cell Margins**, a large dropdown over Normal, None, Narrow, Wide and Custom Margins…, and the weakest glyph on that tab too. `GUESS:`.",
  },
  {
    name: 'table-arrow-repeat-all',
    sizes: [20],
    variants: ['regular', 'filled'],
    why: "Repeat Header Rows, the small toggle in Word's Table Layout Data group, and a survivor: a table with a loop, its header row repeated on every page. Not `table-freeze-row`, Excel's Freeze Top Row, which holds a row still. Filled while pressed. `GUESS:`.",
  },
  {
    name: 'table-switch',
    sizes: [20],
    variants: ['regular'],
    why: "Convert to Text, small, in Word's Table Layout Data group, which opens Convert Table to Text: a table with a turn arrow. Not `convert-range`, which Excel's Table Design will want for Convert to Range. `GUESS:`.",
  },
  {
    name: 'resize-table',
    sizes: [20],
    variants: ['regular'],
    why: "Resize Table, small, in Excel's Table Design Properties group, which opens Resize Table to choose the table's new range: a table inside the corner marks of a selection. `GUESS:`.",
  },
  {
    name: 'pivot',
    sizes: [20],
    variants: ['regular'],
    why: "Summarize with PivotTable, small, in Excel's Table Design Tools group, which opens Create PivotTable over the table: blocks turned by a bent arrow, the nearest Fluent comes to pivoting a table. ⚠ Insert's PivotTable found no honest glyph and carries none; that unit is left as it is, and this is the weakest glyph on the tab. `GUESS:`.",
  },
  {
    name: 'convert-range',
    sizes: [20],
    variants: ['regular'],
    why: "Convert to Range, small, in Excel's Table Design Tools group, which turns the table back into ordinary cells: Fluent's own picture for the command, a table's rows turning into plain lines. `table-switch` is Word's Convert to Text. `GUESS:`.",
  },
  {
    name: 'globe-arrow-forward',
    sizes: [20],
    variants: ['regular'],
    why: "Open in Browser, small, in Excel's Table Design External Table Data group, which opens the table's SharePoint list in a web browser: the web with an arrow going to it. Not `globe`, which is From Web, data coming in. `GUESS:`.",
  },
  // ── Picture Format (Word's, and written for all three) ──────────────────────
  //
  // Word's Picture Format tab, the sixth contextual tab authored. PowerPoint's and Excel's units reuse these rows. The
  // reasoning for every glyph is the *commands Picture Format shows* section of `dev/ribbons/census.ts`.
  {
    name: 'video-background-effect',
    sizes: [24],
    variants: ['regular'],
    why: "Remove Background, the large button that opens Adjust and the Background Removal tab: a subject standing in front of a hatched background, the part the command takes away. Fluent draws no picture with its background struck out, and `image-off` says *no picture at all*. ⚠ The weakest glyph on Word's Picture Format. `GUESS:`.",
  },
  {
    name: 'photo-filter',
    sizes: [24],
    variants: ['regular'],
    why: "Artistic Effects, the large dropdown in Picture Format's Adjust group, which opens twenty-three filters (Pencil Sketch, Watercolour Sponge, Glass): Fluent's own photo-filter mark, two overlapping lenses. Not `paint-brush`, which is Format Painter, nor `image-sparkle`, which now reads as *generated*. `GUESS:`.",
  },
  {
    name: 'transparency-square',
    sizes: [24],
    variants: ['regular'],
    why: "Transparency, the large dropdown in Picture Format's Adjust group: the chequerboard every image editor draws behind what is see-through. `GUESS:`.",
  },
  {
    name: 'arrow-minimize',
    sizes: [20],
    variants: ['regular'],
    why: "Compress Pictures, small, in Picture Format's Adjust group, which opens the Compress Pictures dialog: four arrows pointing in, Office's own picture of a picture made smaller. It also means *leave full screen* elsewhere; it is not a survivor, so the collision is never asked about. `GUESS:`.",
  },
  {
    name: 'image-arrow-forward',
    sizes: [20],
    variants: ['regular'],
    why: "Change Picture, small, in Picture Format's Adjust group, which replaces the picture and keeps its size and formatting: a picture with an arrow going on to the next. Not `image-edit`, which says edit this one. `GUESS:`.",
  },
  {
    name: 'image-arrow-counterclockwise',
    sizes: [20],
    variants: ['regular'],
    why: "Reset Picture, the small split button in Picture Format's Adjust group, which discards the formatting (and, from its arrow, the size) a picture was given: a picture with a turn back. Not `arrow-reset` alone, which is Recording's Reset to Cameo and says nothing about a picture. `GUESS:`.",
  },
  {
    name: 'image-shadow',
    sizes: [20],
    variants: ['regular'],
    why: "Picture Effects, the small dropdown in Picture Format's Picture Styles group, which opens Preset, Shadow, Reflection, Glow, Soft Edges, Bevel and 3-D Rotation: a picture with a shadow behind it. Not `square-shadow`, Shape Effects, because the menu is about the picture. `GUESS:`.",
  },
  {
    name: 'image-alt-text',
    sizes: [24],
    variants: ['regular', 'filled'],
    why: "Alt Text, the large toggle in Picture Format's Accessibility group, which opens the Alt Text pane and draws pressed while it is open: Fluent's own picture with a text label. Filled for the pressed state. **Since PowerPoint's Shape Format unit, also Shape Format's Alt Text**, the same large toggle for a shape: Fluent draws no shape with a label, and the pane it opens is the same one. `GUESS:`.",
  },
  {
    name: 'crop',
    sizes: [24],
    variants: ['regular', 'filled'],
    why: "Crop, the large split button in Picture Format's Size group, whose face puts the picture into crop mode and draws pressed while the handles are out, and whose arrow opens Crop to Shape, Aspect Ratio, Fill and Fit: the crop marks every image editor draws. Filled for the pressed state. `GUESS:`.",
  },
  {
    name: 'play',
    sizes: [24],
    variants: ['regular', 'filled'],
    why: "Play Animation, the large toggle in Picture Format's Image Play group, which plays or pauses a moving picture: the play mark, filled while it plays. Not `play-circle`, Recording's Preview, which plays a slide. `GUESS:` the command, its label and its glyph; see the census.",
  },
  // ── Shape Format (PowerPoint's, the first Drawing Tools tab authored) ─────────
  // Two new glyphs. Every other glyph on the tab is reused: `shapes` (Shapes, large), `textbox` (Text Box, small),
  // `square-shadow` (Shape Effects), `text-effects` (Text Effects), `image-alt-text` (Alt Text) and `arrangeCommands`'
  // five. The census's *commands Shape Format shows* section says why each command carries what it carries.
  {
    name: 'bezier-curve-square',
    sizes: [20],
    variants: ['regular'],
    why: "Edit Shape, the small dropdown in Shape Format's Insert Shapes group, which opens Change Shape, Edit Points and Reroute Connectors: a square whose corners carry Bézier handles, which is what Edit Points puts on a shape. Not `draw-shape`, a pencil drawing a new shape, which is Insert's Shapes. Fluent draws it at 12 and 20 only, and the command is small. `GUESS:`.",
  },
  {
    name: 'shape-union',
    sizes: [20],
    variants: ['regular'],
    why: "Merge Shapes, the small dropdown in PowerPoint's Shape Format Insert Shapes group, which opens Union, Combine, Fragment, Intersect and Subtract: two overlapping shapes drawn as one outline, which is Union, the first of the five and the operation a person means by merging. `shape-subtract`, `shape-intersect` and `shape-exclude` are the other entries, and a menu row carries no glyph here. `GUESS:`.",
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
