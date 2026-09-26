# The ribbon review checklist — sixty tabs, by hand

**Nothing in this file is ticked, and no agent may tick it.** The boxes are for the person doing the pass.

Every tab of all three applications is authored: **sixty tabs — Word 19, PowerPoint 25, Excel 16** — 349 groups
and 1,220 commands. A great deal of it was drawn from memory of Microsoft 365 rather than from a build anybody
had open: **373 lines of `ui/dev/ribbons/census.ts` carry a `GUESS:`, and 296 more in `ui/stories/ribbons/`.**
This file is the list of what to confirm or correct, in the order a reviewer should meet it.

Each tab's items are taken from that story's own *What to look at* block, which is ordered **least certain
first**. Where this file groups them by theme instead, the guesses are still called out under *Guesses here*.

---

## 1 · How to use it

Storybook is already running at **<http://localhost:6006/>**. Do not restart it.

**Reaching a tab.** In the sidebar open `Ribbons/Word`, `Ribbons/PowerPoint` or `Ribbons/Excel`, then click the
tab's story. Storybook spaces the export name, so the code's `TableDesign` reads **Table Design** in the
sidebar; this file uses the sidebar spelling throughout.

**Every story draws the whole ribbon** and selects one tab, so the strip is real and switching tabs works. What
you are looking at is a tab *in* its ribbon, not a tab extracted from one.

**Resizing.** Each story renders inside `<mjx-resizable-container>`. Use the container's own presets, not the
browser window and not the viewport addon:

| Preset | Width | What it is for |
|---|---|---|
| desktop | 1440 px | Every group `full`. The ordinary state. |
| tablet | 834 px | Groups `reduced`, some `collapsed`. Where the ladder starts to bite. |
| phone | 390 px | Everything `collapsed`, and **the tab strip becomes a picker** (at or below 600 px). |

You can also drag the container's edge to find the exact width at which a group gives way — which is what
*"drag narrow until X collapses"* means below.

**Shells.** `Shell/Word`, `Shell/PowerPoint` and `Shell/Excel` are the assembled applications. They draw **one**
contextual set each — Table Tools in Word and Excel, Picture Tools in PowerPoint — and no view tabs. Where a tab
says *also in `Shell/…`*, check it there too: the bindings are different code under different ids, and a
difference is a defect.

**What "right" means here.** Three things this catalogue deliberately does not do, so none of them is a finding:

- **Nothing dispatches a command.** Menus open, pickers open, galleries preview, toggles press — and no
  document changes. Command dispatch is loop 2.
- **No dialog is wired.** A command that opens a dialog in Office is a plain button here and does nothing.
- **Office greys many commands that are drawn available here**, on purpose, so their menus can be audited.

**Counts.** *N groups, M commands* below is what the tab **draws**. The census's own control count is normally
much larger, because it counts every entry inside every menu; where the two readings genuinely disagree, the
tab's *Guesses here* line says so.

**Two questions worth asking of everything**, and they are the ones a screenshot settles in seconds:

1. Is this list's **content and order** Office's?
2. Does this **glyph** say what the command does, with no label?

---

## 2 · If you only check twenty things

The twenty weakest calls in the catalogue, drawn from what each unit declared least certain. If the pass stops
early, stop here.

- [ ] **1. Transitions' Effect Options follows the gallery** — `Ribbons/PowerPoint → Transitions`. Pick Push:
  four edges. Pick Split: Vertical Out/In, Horizontal Out/In. Pick Flash: the button goes unavailable. Many of
  the per-transition entry lists are from memory.
- [ ] **2. Add Chart Element's eleven submenus and their starting ticks** — `Ribbons/Word → Chart Design`. Chart
  Title *Above Chart*, Legend *Bottom*, Gridlines *Primary Major Horizontal* ticked, Axes both ticked, Axis
  Titles neither, the rest on *None*. Every entry and start is a guess, and it is the same tab three times.
- [ ] **3. Other Theme Fills opens from inside the expanded gallery** — `Ribbons/PowerPoint → Shape Format`.
  Expand Theme Styles, press *Other Theme Fills* in its footer: a menu of Style 1–12 anchored to the footer,
  not swallowed by the gallery's popup.
- [ ] **4. The colour picker's palette-to-entries keyboard move** — `Ribbons/Word → Table Design`, open Shading.
  Arrow Down from the last row of recent colours must land on the first entry beneath the palette; Arrow Up must
  return. Arrow Right opens a submenu; choosing anything closes the picker.
- [ ] **5. Word Table Layout's eleven survivors** — `Ribbons/Word → Table Layout`. Which three of the four
  inserts and which three of the nine alignments survive is a guess, and it is the largest survivor set anywhere.
- [ ] **6. The nine cell alignments as a three-by-three grid** — `Ribbons/Word → Table Layout`. Icon-only, one
  set holding exactly one, declared down each column. Check they read as Office's grid with the top row across
  the top, and that a bare box-with-two-lines says *align* at all.
- [ ] **7. Gallery art — do two neighbours read as different styles?** — `Ribbons/Word → Table Design`, expand
  Table Styles. Judge Grid Table 4, Grid Table 5 Dark and Grid Table 6 Colorful side by side. The question is
  never the pixels.
- [ ] **8. Excel's Debug group** — `Ribbons/Excel → Review` and `→ View`. One button labelled *Debug*, no icon.
  The census names the group, counts one control and names none; all of it is a guess.
- [ ] **9. Excel's Night Mode group** — `Ribbons/Excel → View`. One large Switch Modes toggle, Word's reading,
  drawn last. Guessed from end to end.
- [ ] **10. PowerPoint's View Direction group** — `Ribbons/PowerPoint → View`. Left-to-Right (pressed) and
  Right-to-Left, small, drawn last. The census counts three controls and names none.
- [ ] **11. PowerPoint's Activity group** — `Ribbons/PowerPoint → Review`. One plain labelled button, *Show
  Changes*. The census counts two controls and names none.
- [ ] **12. Image Play** — `Ribbons/Word → Picture Format` (and PowerPoint's, and Excel's). One large toggle,
  *Play Animation*, starting pressed. The command, its label, its glyph and its start are all guessed, from a
  group id alone.
- [ ] **13. Starting measures, per application** — a chart is 15.24 × 8.89 cm in Word, 22.58 × 15.05 in
  PowerPoint, **12.7 × 7.62 in Excel** (`Ribbons/Excel → Chart Format`, recorded as the weakest call on that
  tab). A picture is 8.57 × 11.43 in Word, 25.4 × 19.05 in PowerPoint, 12.7 × 9.53 in Excel.
- [ ] **14. Remove Background's glyph** — `Ribbons/Word → Picture Format`. A subject before a hatched
  background; recorded as the weakest glyph on the tab, and it is on all three Picture Format tabs.
- [ ] **15. Reset to Match Style's glyph** — `Ribbons/Word → Chart Format`. Reset's loop, which says *reset*
  without *to the chart's style*. Recorded as the weakest on all three Chart Format tabs.
- [ ] **16. Cell Margins' `padding-left` glyph** — `Ribbons/Word → Table Layout`. An edge, a dashed guide and an
  arrow. Recorded as the weakest on the tab; judge whether it reads as margins at all.
- [ ] **17. Quick Layout's glyph** — `Ribbons/Word → Chart Design`. Four tiled regions, which reads *arrange
  windows* before *arrange a chart's title, plot and legend*.
- [ ] **18. The Design flyout footer** — `Ribbons/PowerPoint → Design`. Open the Variants flyout and press
  *Colours* at its foot. Nobody has watched a menu open from inside an open flyout.
- [ ] **19. Excel Table Design's Table Name combo box** — `Ribbons/Excel → Table Design`. Office draws a plain
  text box; this draws a combo box whose arrow opens a list of one. Recorded as the weakest part of the tab's
  shape — judge whether the list of one is acceptable.
- [ ] **20. Print Preview's Next and Previous Page glyphs** — `Ribbons/Word → Print Preview`, collapsed. A page
  with an arrow down and a page with an arrow up, standing alone beside the trigger. Recorded as a glyph that
  may read as *download* and *upload*.

---

## 3 · Word — 19 tabs

### File · `Ribbons/Word → File`
Seven backstage destinations as an ordinary tab: Info, Open, Save, Print, Share, Export, Help. 28 commands.

- [ ] Drag narrow: Help gives way first, then Print, then Info, Share and Export; **Open and Save last**.
- [ ] Collapse Save: **AutoSave stays beside the trigger** — the only survivor on the tab. Every other group
  collapses to its trigger alone.
- [ ] Expand again: every command draws in Office's order, survivors included.
- [ ] AutoSave is the only toggle on the tab and is **drawn pressed** (a cloud document).
- [ ] **Properties carries no icon**, deliberately. Check it reads as a command anyway.
- [ ] Printer reads a printer name and Copies reads 1; both are bound by the catalogue, not the census.

*Guesses here:* that Open and Save are the two `primary` groups; that six of the seven groups can keep no
survivor.

### Home · `Ribbons/Word → Home`
Six groups — Clipboard, Font, Paragraph, Styles, Editing, Editor. 38 commands.

- [ ] **Font draws two fields and eleven glyphs; Paragraph fourteen glyphs and nothing else.** Hover each: every
  icon-only command still has its name as a tooltip and an accessible name.
- [ ] Press each of the six character formats and all four alignments: each draws pressed.
- [ ] Collapse Font: **Bold and Italic** stay beside the trigger — and they draw **in the middle**, where Word
  draws them, not first. Underline draws pressed and does **not** survive (it is a split button in Office).
- [ ] Collapse Paragraph: **Left, Centre and Right** stay.
- [ ] Drag narrow: Editor and Editing give way first and keep nothing; **Font and Paragraph stand last**.
- [ ] Press Paste's arrow: the paste menu opens.
- [ ] Open the font picker, the size dropdown, the colour picker and the Styles gallery: each works.
- [ ] Judge the five replaced glyphs against Office: Format Painter, Bullets, Numbering, Borders, Select.

*Guesses here:* which three commands per group survive; the icon choices.

### Insert · `Ribbons/Word → Insert`
Nine groups — Pages, Tables, Illustrations, Media, Links, Comments, Header & Footer, Text, Symbols. 28 commands.

- [ ] Press Table, Shapes, Header and Text Box: each menu opens under it.
- [ ] Press the **arrow** on Link, Signature Line, Object and Equation: the split button's menu opens.
- [ ] Check a menu is a handful of real Office entries, not the whole gallery — Header lists the built-in
  headers by name, then Edit Header and Remove Header.
- [ ] Drag narrow: **nothing survives**. Every group collapses to its trigger alone.
- [ ] Sizes: Table, Pictures, Shapes, Icons, 3D Models, Online Videos, Comment, Text Box and Equation large;
  Pages, Links, Header & Footer and most of Text as columns of labelled commands.
- [ ] Six commands carry no icon — Cover Page, Cross-reference, Quick Parts, Drop Cap, Object, Symbol. Check
  each still reads.
- [ ] Tables and Illustrations stand last; Media and Comments give way first.

*Guesses here:* the sizes of Links and Header & Footer — Office has drawn them both ways.

### Draw · `Ribbons/Word → Draw`
Eleven groups — Drawing Tools, Pens, Write, Stencils, Editing, Drawing Canvas, Input Mode, Draw with Touch,
Replay, Help, Close. 17 commands. **Two generations of Office on one tab.**

- [ ] ⚠ Select Objects starts pressed. Press Pen: it fills and Select Objects releases. Press Pen again: it
  **stays** pressed.
- [ ] Press Eraser's **face**: it presses as one of the five, releasing Pen. Press its **arrow**: only the eraser
  sizes open.
- [ ] Collapse Write and press a tool inside the popup: Select Objects, beside the trigger, releases too.
- [ ] Collapse Write: **Select Objects** is the tab's only survivor.
- [ ] Press Add Pen, Pens, Colour, Thickness and Touch/Mouse Mode: each menu opens.
- [ ] Three commands carry no icon — Add Pen, Touch/Mouse Mode, Drawing Canvas.
- [ ] Pens and Write stand last.

*Guesses here:* the group **order** (no Office build draws this union); that pressing the tool that holds does
nothing, where Office's Pen opens its options; that `GroupEditingExcel` — an Excel id on Word's tab — is Ink
Editor.

### Design · `Ribbons/Word → Design`
Two groups — Style Set, Page Background. 10 commands.

- [ ] Arrow through the Style Set gallery, then open its flyout: two sections, *This Document* and *Built-In*,
  nine style sets from *This Document* to *Word 2013*.
- [ ] Judge each picture: a title, a heading and a line of body text in that set's weight.
- [ ] ⚠ Press Themes: a **menu of named themes**, not a grid. It carries no icon and is small where Office draws
  it large.
- [ ] Press Colours, Fonts, Paragraph Spacing, Effects and Watermark: each list opens with the current choice
  checked.
- [ ] Open Page Colour: a colour picker with *No Colour*.
- [ ] Nothing survives a collapse; no dialog launcher on either group; Style Set stands last.

*Guesses here:* Themes as a dropdown rather than a gallery; the group label *Style Set* where Office writes
*Document Formatting*.

### Layout · `Ribbons/Word → Layout`
Three groups — Page Setup, Paragraph, Arrange. 19 commands.

- [ ] Type `1.5 cm` into Indent Left and commit; type `abc` and the field must say it cannot read it **and keep
  your text**.
- [ ] Arrow Up in Indent Left steps by ¼ cm; in Spacing by 6 pt. Starts: 0 and 8 pt.
- [ ] Press Margins, Orientation, Size, Breaks, Line Numbers and Hyphenation: Office's lists, current choice
  checked — Normal, Portrait, A4, One.
- [ ] Sizes: Margins, Orientation and Columns large; **Size small between them** (Fluent draws no page size).
- [ ] Press Bring Forward's and Send Backward's arrows: Bring to Front and Bring in Front of Text.
- [ ] Selection Pane is a toggle with no icon, drawn pressed while the pane is open.
- [ ] Two dialog launchers, on Page Setup and Paragraph; Arrange has none. Nothing survives.

*Guesses here:* the starting measures and steps.

### References · `Ribbons/Word → References`
Seven groups — Table of Contents, Footnotes, Citations & Bibliography, Captions, Index, Table of Authorities,
Acronyms. 22 commands.

- [ ] ⚠ **Insert Footnote is a plain large button** with no arrow; **Next Footnote is the split button**. Press
  its arrow: Previous Footnote, Next Endnote, Previous Endnote. Press its face: nothing opens.
- [ ] Press Table of Contents (three built-in tables), Add Text (levels, *Do Not Show in Table of Contents*
  checked), Insert Citation (two ways), Bibliography (three built-in).
- [ ] Style is a dropdown field starting on **APA**; pick MLA and the field shows it.
- [ ] Update Table appears three times and Update Index beside them — **all four on one refresh glyph**, which is
  why none survives.
- [ ] Table of Contents is small although Office draws it large (three tokens do not fit).
- [ ] Twelve commands carry no icon, from Add Text to Acronyms.
- [ ] Footnotes has the tab's one dialog launcher. Nothing survives; Table of Contents and Footnotes stand last.

*Guesses here:* that Acronyms is last; Office's Research group is out of scope and absent.

### Mailings · `Ribbons/Word → Mailings`
Five groups — Create, Start Mail Merge, Write & Insert Fields, Preview Results, Finish. 21 commands.

- [ ] **Preview Results starts pressed**, and the record navigator beside it is icon-only: First, Previous, the
  number, Next, Last.
- [ ] The record number is a combo box on 1: pick 3, then type 12.
- [ ] Collapse Preview Results: **Previous Record and Next Record** stay beside the trigger — the tab's only
  survivors.
- [ ] Press Start Mail Merge (document types, Normal Word Document checked), Select Recipients (three sources),
  Rules (Word's nine), Finish & Merge (three).
- [ ] **Insert Merge Field is a split button**: its arrow lists the fields, its face opens nothing.
- [ ] Press Highlight Merge Fields: it draws pressed.
- [ ] Judge Envelopes' envelope, Address Block's contact card and Greeting Line's waving hand, all large.
- [ ] No dialog launchers. Start Mail Merge and Write & Insert Fields stand last.

*Guesses here:* Preview Results pressed rather than off; `mail-multiple`, `people-list` and `people-edit`.

### Review · `Ribbons/Word → Review`
Nine groups — Proofing, Accessibility, Language, Comments, Tracking, Changes, Compare, Protect, Ink. 23 commands.

- [ ] ⚠ **Ink is drawn last, after Protect**; the census declares it fifth.
- [ ] Press Track Changes' arrow: For Everyone (checked), Just Mine, Lock Tracking.
- [ ] Press Show Markup: Comments, Ink, Insertions and Deletions, Formatting (all ticked), then *Balloons* (Show
  Only Comments and Formatting in Balloons checked) and *Specific People* (All Reviewers).
- [ ] Press Accept and Reject: five entries each. Press Compare: Compare, Combine, *Show Source Documents* (Show
  Both checked).
- [ ] Display for Review is a dropdown on **Simple Markup** over its four modes.
- [ ] **Track Changes, Show Comments and Hide Ink are split buttons whose face is a toggle.** Show Comments
  starts pressed; the other two unpressed. Press a face: it moves. Press its arrow: the menu opens and **the
  state does not move**.
- [ ] Press Restrict Editing: the plain toggle draws pressed.
- [ ] Collapse Comments: **Previous Comment and Next Comment** stay — the tab's only survivors.
- [ ] Five commands carry no icon: Show Markup, Compare, Previous Change, Next Change, Hide Ink.
- [ ] Spelling & Grammar and Check Accessibility are small where Office draws them large. Tracking has the one
  launcher.

*Guesses here:* Ink's position; Show Comments' Contextual/List arrow; Hide Ink's shape (the brief said *Show
Ink*). Speech is out of scope.

### View · `Ribbons/Word → View`
Seven groups — Document Views, Modes, Page Movement, Show, Zoom, Window, Night Mode. 25 commands.

- [ ] ⚠ Collapse Zoom: **100% (large, *1:1*), One Page and Page Width** stay beside the trigger; Zoom and
  Multiple Pages open from it. Check 100% keeps its large size in the survivor row.
- [ ] ⚠ Print Layout starts pressed. Press Web Layout: it fills and Print Layout releases. Press Web Layout
  again: it stays.
- [ ] Vertical (pressed) and Side to Side do the same, and **neither set touches the other**.
- [ ] Tab to a view and press **Space**: the same, by keyboard.
- [ ] Press Switch Windows: one window, *1 Method notes*, checked — the tab's only menu.
- [ ] Show is three checkboxes: Ruler and Gridlines unticked, **Navigation Pane ticked**. Tick one: it ticks.
- [ ] Judge the glyphs: Read Mode's open book, Print Layout's page, Web Layout's globe (large toggles, filled
  while pressed), Outline's stepped bars, Draft's lines with a pencil, Focus's four corners, Split's window cut
  across, View Side by Side's two panes, Switch Modes' half-dark circle.
- [ ] Seven commands carry no icon and are small: Vertical, Side to Side, Zoom, Multiple Pages, Arrange All,
  Synchronous Scrolling, Reset Window Position. No launchers.

*Guesses here:* that each zoom survivor's glyph reads with no label; the group labels *Document Views*, *Modes*
and *Night Mode* where Microsoft 365 says Views, Immersive and Dark Mode; Page Movement third; Night Mode last;
the window's name.

### Outlining · `Ribbons/Word → Outlining` *(view tab)*
Three groups — Outlining Tools, Master Document, Close. 21 commands.

- [ ] ⚠ Collapse Outlining Tools: **Promote and Demote** stay; the other ten open from the popup in order.
- [ ] ⚠ Press Collapse Subdocuments: it fills. Press again: it releases.
- [ ] ⚠ **Close Outline View is large with a three-word label** — it must wrap to two lines with no ellipsis,
  under a cross in a square.
- [ ] ⚠ Read the level row left to right: Promote to Heading 1, Promote, the **Outline Level field** on *Body
  Text*, Demote, Demote to Body Text, then Move Up, Move Down, Expand, Collapse.
- [ ] Open Outline Level: Level 1 to Level 9, then Body Text.
- [ ] Show Level reads *All Levels* and lists Level 1–9 then All Levels. Show Text Formatting is ticked; Show
  First Line Only is not.
- [ ] Master Document is drawn whole with **Show Document pressed**; releasing it hides nothing here. Lock
  Document fills while pressed.
- [ ] Open `Shell/Word`: **there is no Outlining tab** in the strip.

*Guesses here:* that a bare left arrow reads as *promote* rather than *back*; that Office relabels Collapse
Subdocuments rather than drawing it pressed, and draws it large; the group label *Outlining Tools* where Office
writes *Outline Tools*; the Outline Level list's order and the field's place between the arrows; all eight
Master Document glyphs.

### Print Preview · `Ribbons/Word → Print Preview` *(view tab)*
Four groups — Print, Page Setup, Zoom, Preview. 16 commands.

- [ ] ⚠ **Magnifier is a ticked checkbox, not a toggle button**, under Show Ruler (unticked), above Shrink One
  Page.
- [ ] ⚠ Collapse Preview: **Next Page and Previous Page** stay; the other four open from the popup.
- [ ] ⚠ Press Margins: Normal (checked), Narrow, Moderate, Wide, Mirrored, Office 2003 Default, Custom Margins…
  — **no Last Custom Setting**, because a new document has none.
- [ ] ⚠ Press Orientation (Portrait checked, Landscape) and Size (Letter … A4 checked … five envelopes, More
  Paper Sizes…). Size may need to scroll.
- [ ] Open the same three lists from the **Layout** tab: they must be identical.
- [ ] ⚠ Zoom and 100% are large; One Page, Two Pages and Page Width stack beside them. **Two Pages draws no
  glyph.** Collapse Zoom: 100%, One Page and Page Width stay, as on View.
- [ ] Close Print Preview is large with a three-word label — wrap to two lines, no ellipsis.
- [ ] Print and Options are large (a printer, a cog). Page Setup has the one launcher. **Size is small** between
  two large neighbours.
- [ ] Open `Shell/Word`: no Print Preview tab, and no Print Preview menu on the page.

*Guesses here:* Magnifier's shape and tick; that the page-arrow glyphs do not read as *download* and *upload*;
the Size list's order.

### Background Removal · `Ribbons/Word → Background Removal` *(view tab)*
Two groups — Refine, Close. 4 commands.

- [ ] ⚠ **Neither pencil starts pressed.** Press Mark Areas to Keep: it fills. Press Mark Areas to Remove: it
  fills and Keep releases. Press Remove again: it releases and **neither holds**.
- [ ] ⚠ Refine has **two** commands, not three — Delete Mark is absent.
- [ ] ⚠ Judge the four circles: a plus and a minus for the pencils, a cross and a tick for Discard All Changes
  and Keep Changes. Do the plus and the tick read as different *kinds* of command?
- [ ] All four are large: *Mark Areas to Remove* and *Discard All Changes* must wrap to two lines without an
  ellipsis.
- [ ] Collapse both groups: no survivors, each trigger alone. Press a pencil **inside the popup**: the set still
  holds at most one.
- [ ] Open `Shell/Word`: no Background Removal tab.

*Guesses here:* the release on a second press and the empty start (the Draw tab's tools, by contrast, keep one
pressed); every glyph; Delete Mark's absence.

### Table Design · `Ribbons/Word → Table Design` *(contextual — Table Tools)*
Three groups — Table Style Options, Table Styles, Borders. 14 commands.
**Items 1 and 2 hold on every contextual story.**

- [ ] **The band.** *Table Tools* is drawn over this tab and Layout, in the contextual tone, **after every core
  tab**. The tab's accessible name is *Table Design, Table Tools*.
- [ ] **The collapse order.** Table Style Options gives way first; Table Styles and Borders last.
- [ ] **Border Styles is Borders' first command**, large, left of Line Style. Press it: *Theme Borders*,
  twenty-one entries from *Single solid line, ½ pt, Text 1* to *… 1 ½ pt, Accent 6*, then Border Sampler.
- [ ] ⚠ Expand Table Styles: *Plain Tables* (7), *Grid Tables* (49), *List Tables* (49), one family to a row of
  seven, **Table Grid selected**. Footer: Modify Table Style…, Clear, New Table Style….
- [ ] ⚠ Check the gallery's six accent columns are **the six accents Shading's theme row shows**.
- [ ] ⚠ Open Shading (*No Colour*) and Pen Colour (*Automatic*): theme, standard and recent colours, with **More
  Colours…** beneath the palette on both.
- [ ] ⚠ In either picker, Arrow Down from the last row of recent colours moves onto More Colours…; Arrow Up
  returns.
- [ ] Six checkboxes, two columns of three: Header Row, Total Row, Banded Rows down the first; First Column, Last
  Column, Banded Columns down the second. **Header Row, Banded Rows and First Column ticked.**
- [ ] Line Style lists No Border and twenty-four styles, **Single** selected; Line Weight ¼ pt to 6 pt, **½ pt**
  selected. Office draws pictures of lines, not names.
- [ ] Press Borders' arrow: sixteen entries — the four edges; No, All, Outside and Inside Borders; the inside and
  diagonal lines; Horizontal Line; then Draw Table, **View Gridlines (ticked)** and Borders and Shading….
- [ ] Press Border Painter: the brush fills. Press again: it releases.
- [ ] The launcher at Borders' corner reads *Borders and Shading*. No other group has one.
- [ ] Drag narrow: **no survivor anywhere**, each group a trigger with nothing beside it.
- [ ] Open `Shell/Word` and select Table Design: every list, menu and starting state is the same.

*Guesses here:* every gallery picture and the footer's order; that Word's two pickers carry no Eyedropper,
Picture, Gradient or Texture (PowerPoint's do); the line-style names; the checkboxes' columns and start; View
Gridlines ticked; all three glyphs. The spelling *Pen Colour* is the census's; style names keep Word's *Colorful*.

### Table Layout · `Ribbons/Word → Table Layout` *(contextual — Table Tools)*
Seven groups — Table, Draw, Rows & Columns, Merge, Cell Size, Alignment, Data. 33 commands. Labelled **Layout**;
the band and the accessible name *Layout, Table Tools* tell it from the core Layout tab.

- [ ] ⚠ Judge **Cell Margins' `padding-left` glyph** — the weakest on the tab.
- [ ] ⚠ Align Top Left starts pressed. Press Align Centre: it fills and Top Left releases. Press Align Centre
  again: nothing changes.
- [ ] ⚠ Check the nine alignments draw as **Office's three-by-three grid, top row across the top** — they are
  declared down each column.
- [ ] ⚠ Draw Table and Eraser: neither starts pressed. Press Draw Table, then Eraser: Draw Table releases. Press
  Eraser again: it releases and neither holds. **Eraser is a plain toggle here**, with no sizes.
- [ ] Collapse Rows & Columns: **Insert Above, Insert Below, Insert Right** stay in that order; Delete and Insert
  Left in the popup. Insert Above keeps its large size.
- [ ] Collapse Merge: **Merge Cells and Split Table** stay; Split Cells in the popup.
- [ ] Collapse Cell Size: **Distribute Rows and Distribute Columns** stay; AutoFit, Height and Width in the popup.
- [ ] Collapse Alignment: **Align Top Left, Top Centre, Top Right** stay; the other six, Text Direction and Cell
  Margins in the popup.
- [ ] Collapse Data: **Repeat Header Rows** stays; Sort, Convert to Text and Formula in the popup. Table and Draw
  keep nothing.
- [ ] Collapse order: Draw first, then Table, then Merge, Cell Size and Data, and **Rows & Columns and Alignment
  last**.
- [ ] Press Select (Cell, Column, Row, Table), Delete (Cells…, Columns, Rows, Table), AutoFit (Contents, Window,
  Fixed Column Width — **none ticked**).
- [ ] Height 0.5 cm and Width 3.18 cm, stepping by 0.1.
- [ ] **View Gridlines starts pressed**; Repeat Header Rows starts unpressed — press it and it fills.
- [ ] Two launchers: *Insert Cells* at Rows & Columns' corner, *Table Properties* at Cell Size's.
- [ ] Hover an alignment: the name reads *Align Top Centre*, *Align Centre* — the census's spelling, where Office
  writes *Center*.
- [ ] Open `Shell/Word` and select Layout under the *Table Tools* band: every list, field, set and survivor is
  the same.

*Guesses here:* which three inserts and which three alignments survive; both sets' starts and the column order;
both measures; View Gridlines pressed; AutoFit ticking none; the two launchers' names; all twenty-four new
glyphs. Split Cells, Properties, Text Direction, Cell Margins, Sort, Convert to Text and Formula open dialogs in
Office and nothing here.

### Picture Format · `Ribbons/Word → Picture Format` *(contextual — Picture Tools)*
Six groups — Adjust, Picture Styles, Accessibility, Arrange, Size, Image Play. 25 commands.

- [ ] ⚠ **Image Play**: one large toggle, *Play Animation*, **starting pressed**.
- [ ] ⚠ Judge **Remove Background's glyph** — the weakest on the tab — then Artistic Effects' two lenses,
  Transparency's chequerboard, Compress Pictures' four inward arrows, Change Picture, Reset Picture, Picture
  Effects, Alt Text, Crop and Play Animation.
- [ ] ⚠ Press **Corrections**: *Sharpen/Soften* (five, Sharpen: 0% checked) and *Brightness/Contrast*
  (twenty-five, contrast down and brightness across, Brightness: 0% Contrast: 0% checked), then Picture
  Corrections Options….
- [ ] ⚠ Press **Colour**: *Colour Saturation* (seven, 100%), *Colour Tone* (seven, 6500 K), *Recolour*
  (twenty-one, No Recolour), then More Variations, Set Transparent Colour, Picture Colour Options….
- [ ] ⚠ Press **Artistic Effects** (twenty-three, None checked) and **Transparency** (seven, 0% checked), each
  ending in its Options….
- [ ] ⚠ Arrow through a section: **choosing a step moves the tick within its section alone.**
- [ ] ⚠ Expand Quick Styles: twenty-eight picture styles from Simple Frame, White to Metal Oval. **Nothing is
  selected and there is no footer.** Judge whether Metal Frame, Beveled Matte and the two Perspective styles read
  as different styles.
- [ ] Press Crop's face: it fills. Press again: it releases. Press its arrow: Crop, **Crop to Shape** (seven
  sections, 147 shapes), **Aspect Ratio** (Square, Portrait, Landscape), Fill, Fit.
- [ ] Press Picture Effects: seven submenus — Preset, Shadow, Reflection, Glow, Soft Edges, Bevel, 3-D Rotation.
  Press Picture Layout: thirty-one SmartArt picture layouts.
- [ ] Open Picture Border: *No Outline*, starting on none, with More Outline Colours…, Weight ▸, Sketched ▸,
  Dashes ▸ — **no Eyedropper**.
- [ ] Adjust's small column: Compress Pictures (plain), Change Picture (From a File…, From Stock Images…, From
  Online Sources…, From Icons…, From Clipboard), **Reset Picture** (split: Reset Picture, Reset Picture & Size).
- [ ] Arrange is Layout's eight, opening the same lists under this tab's ids. **Position stays small with no
  glyph.**
- [ ] Height 8.57 cm and Width 11.43 cm, stepping by 0.01.
- [ ] Alt Text is a large toggle, unpressed; pressed it fills, and no pane opens.
- [ ] Two launchers: *Format Picture* at Picture Styles', *Layout* at Size's.
- [ ] Drag narrow: Image Play first, Accessibility next, then Arrange and Size, Adjust and Picture Styles last.
  **No survivor anywhere.**
- [ ] Open `Shell/Word`: **no Picture Tools band** (the shell draws Table Tools).

*Guesses here:* the whole of Image Play, from a group id; every glyph; every preset list's contents and counts;
every gallery look; that Crop draws pressed; every Crop to Shape name; no Eyedropper; both measures.

### Shape Format · `Ribbons/Word → Shape Format` *(contextual — Drawing Tools)*
Seven groups — Insert Shapes, Shape Styles, WordArt Styles, Text, Accessibility, Arrange, Size. 25 commands.
*(Theme Styles, Other Theme Fills, the Shapes list and Edit Shape are covered on
`Ribbons/PowerPoint → Shape Format`; Arrange on Picture Format.)*

- [ ] ⚠ **The Text group is Word's alone.** Press Text Direction: *Horizontal* (checked), *Rotate all text 90°*,
  *Rotate all text 270°*, *Text Direction Options…* — **no Stacked**, which PowerPoint's has.
- [ ] ⚠ Press Align Text: *Top* (checked), *Middle*, *Bottom*, one set — choosing one moves the tick.
- [ ] ⚠ **Create Link is a plain small button**; only the unlinked state is drawn (Office reads *Break Link* on a
  linked box).
- [ ] ⚠ Judge **Create Link's chain** — the weakest glyph on the tab; it says *hyperlink* before *flow this text
  into the next box*. Compare with Text Direction's rotated letters and Align Text's centred bar.
- [ ] ⚠ **Draw Text Box is a split button.** Press its face: nothing opens (it arms a gesture). Press its arrow:
  *Draw Text Box*, *Draw Vertical Text Box*. **No Merge Shapes** beside it.
- [ ] ⚠ Open Text Fill: starts on **Background 1**, with *No Fill*, More Fill Colours… and Gradient ▸. Open Text
  Outline: starts on none, with *No Outline*, More Outline Colours…, Weight ▸, Dashes ▸. Both carry **fewer
  entries than PowerPoint's**.
- [ ] Open Shape Fill (**Accent 1**): More Fill Colours…, Picture…, Gradient ▸, Texture ▸ — **no Eyedropper**.
  Open Shape Outline (**Accent 1, Darker 50%**): More Outline Colours…, Weight ▸, Sketched ▸, Dashes ▸, Arrows ▸.
- [ ] Press Shapes: the whole gallery with **no Action Buttons** and **New Drawing Canvas** under it — the same
  list `Insert → Shapes` opens. Edit Shape's Change Shape has no Action Buttons either.
- [ ] Arrange is Picture Format's eight, with **Align to Margin ticked**.
- [ ] Height and Width start on 2.54 cm, stepping by 0.01.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, and **Layout** at Size's (PowerPoint's says *Size
  and Position*).
- [ ] Alt Text is a large toggle, unpressed.
- [ ] Drag narrow: Accessibility first, then Insert Shapes, WordArt Styles, Text, Arrange and Size, Shape Styles
  last. **No survivor anywhere.**
- [ ] Open `Shell/Word`: **no Drawing Tools band**, and none of these menus on the page.

*Guesses here:* every Text-group label and both starts; Draw Text Box's label, shape and both entries; all four
picker starts and entry lists; both measures; all three launcher names.

### Chart Design · `Ribbons/Word → Chart Design` *(contextual — Chart Tools)*
Four groups — Chart Layouts, Chart Styles, Data, Type. 9 commands, **every one large**.

- [ ] ⚠ Press **Change Chart Type**: eight submenus, Excel's Insert → Charts families, each holding exactly the
  list `Ribbons/Excel → Insert` opens for it and ending on *More … Charts…*. **No Map family.** Office opens a
  dialog here.
- [ ] ⚠ Press **Add Chart Element**: eleven submenus — Chart Title (*Above Chart* checked), Legend (*Bottom*),
  Gridlines (*Primary Major Horizontal* ticked, a checkbox each), Axes (both ticked), Axis Titles (neither), Data
  Labels, Data Table, Error Bars, Lines, Up/Down Bars (each on *None*), Trendline (plain entries, a verb per
  series). Each ends on its *More … Options…* under a separator.
- [ ] ⚠ **Lines and Up/Down Bars open here**, where Office greys both on a column chart — so their entries can be
  judged.
- [ ] ⚠ Choose a radio entry: the tick moves **within its submenu**.
- [ ] ⚠ Expand the Chart Styles gallery: *Style 1* to *Style 16*, starting on **Style 1** — three columns in
  Accent 1, 2 and 3 on a paper, tinted or dark ground; solid, outlined, hatched, shaded or pale; with or without
  gridlines. Do two neighbours read as different looks?
- [ ] ⚠ Judge **Quick Layout's glyph** — four tiled regions, the weakest on the tab. Its menu is *Layout 1* to
  *Layout 11*.
- [ ] Press Change Colours: *Colourful* (Palettes 1–4) and *Monochromatic* (Palettes 1–13), **one set across
  both**, on *Colourful Palette 1*.
- [ ] Press Edit Data's arrow: *Edit Data*, *Edit Data in Excel*. Its face opens the data sheet.
- [ ] Switch Row/Column, Select Data and Refresh Data are plain large buttons, **drawn available**. Judge their
  three glyphs together.
- [ ] Check the catalogue's spelling: *Change Colours*, *Colourful*, *Centred Overlay*, *Centre*.
- [ ] No dialog launcher. Drag narrow: Chart Styles and Type first, then Data, Chart Layouts last. **No
  survivor.**
- [ ] Open `Shell/Word`: no Chart Tools band.

*Guesses here:* Change Chart Type as a menu, and the missing Map family; every Add Chart Element entry and start;
sixteen styles and every look; both Change Colours counts.

### Chart Format · `Ribbons/Word → Chart Format` *(contextual — Chart Tools)*
Seven groups — Current Selection, Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size.
24 commands. Labelled **Format**; the accessible name *Format, Chart Tools* tells it from Shape Format.

- [ ] ⚠ **Chart Elements** starts on *Chart Area* and lists ten parts **alphabetically**: Chart Area, Chart
  Title, Horizontal (Category) Axis, Legend, Plot Area, Series "Series 1"–"Series 3", Vertical (Value) Axis,
  Vertical (Value) Axis Major Gridlines. **The series come before the vertical axis.**
- [ ] ⚠ Check the field is wide enough for *Vertical (Value) Axis Major Gridlines*.
- [ ] ⚠ Judge **Reset to Match Style's glyph** — the weakest on the tab — beside **Format Selection's** column
  chart with a pencil. Both plain small buttons, drawn available.
- [ ] ⚠ Open Shape Outline: starts on *Text 1, Lighter 80%*, and has **no Sketched ▸ and no Arrows ▸**, where
  Shape Format's has both. Open Shape Fill: *Background 1*, with More Fill Colours…, Picture…, Gradient ▸,
  Texture ▸.
- [ ] ⚠ Open Text Fill (*Text 1, Lighter 40%*, with More Fill Colours… and Gradient ▸) and Text Outline (none,
  with More Outline Colours…, Weight ▸, Dashes ▸). **Text Effects keeps Transform**, which Office may grey.
- [ ] Press Shapes: the gallery with **no New Drawing Canvas and no Action Buttons**. **Change Shape** is small,
  drawn available, wearing Edit Shape's glyph. No Draw Text Box, Edit Points or Merge Shapes.
- [ ] Height 8.89 cm and Width 15.24 cm, stepping by 0.01.
- [ ] Arrange is Shape Format's eight, with **Align to Margin ticked**.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, *Layout*. **None on Current Selection.**
- [ ] Alt Text is a large toggle, unpressed.
- [ ] Drag narrow: Insert Shapes first, then Accessibility, then Current Selection, WordArt Styles, Arrange and
  Size, Shape Styles last. **No survivor.**
- [ ] Open `Shell/Word`: no Chart Tools band.

*Guesses here:* the Chart Elements order and every label; both picker starts and both omissions; the whole Text
Fill / Text Outline reading; both measures; all three launcher names.

---

## 4 · PowerPoint — 25 tabs

### File · `Ribbons/PowerPoint → File`
Seven groups, as Word's. 32 commands.

- [ ] **Export** carries *Create a Video*, *Package Presentation for CD* and *Create Handouts*.
- [ ] **Share** carries *Publish Slides*.
- [ ] Package Presentation for CD and Create Handouts carry **no icon** — check both still read.
- [ ] The collapse order is Word's: Help first, Open and Save last.

*Guesses here:* none specific to this tab beyond Word's File readings.

### Home · `Ribbons/PowerPoint → Home`
Six groups — Clipboard, Slides, Font, Paragraph, Drawing, Editing. 43 commands.

- [ ] **Slides**: New Slide (the tab's one large command), Layout, Reset, Section.
- [ ] **Drawing draws six commands** — Shapes, Arrange, the style gallery and the three shape formats — and
  **keeps no survivor**: all six open a gallery or a menu.
- [ ] Check **Arrange draws `layer`**, not the Layout command's glyph one group to its left. The two must not
  read as one command.
- [ ] **Text Shadow carries no icon** — the one labelled toggle in a row of glyphs. It draws pressed, and can
  never be a survivor.
- [ ] Collapse Font and Paragraph: the declared three each stay. Slides, Drawing and Editing keep none.

*Guesses here:* which three per group survive; Text Shadow's labelled treatment.

### Insert · `Ribbons/PowerPoint → Insert`
Eleven groups — Slides, Tables, Images, Illustrations, Camera, Links, Comments, Text, Symbols, Media Clips,
Content. 28 commands.

- [ ] **Images** (Pictures, Screenshot, Photo Album) and **Illustrations** (Shapes, Icons, 3D Models, SmartArt,
  Chart) are all large — Word draws three of its seven small.
- [ ] Camera (Cameo) and Media (Video, Audio, Screen Recording) exist in no other application.
- [ ] Press New Slide's arrow: the layout list, as on Home. Press Zoom: Summary, Section and Slide Zoom.
- [ ] **Text Box opens nothing** — it arms a drawing gesture, so it is a plain button and not a survivor.
- [ ] **Symbol opens nothing either**, although Word's does.
- [ ] Two group labels are the census's, not Office's: **Media Clips** (Office writes *Media*) and **Content**,
  one control the census names without describing — it carries the group's own label and no icon.
- [ ] Five commands carry no icon: Reuse Slides, **Zoom**, Object, Symbol, Content. Zoom is the one to judge —
  Fluent's magnifier is the status bar's view zoom.

*Guesses here:* Camera's and Content's positions — each is drawn where the declaration puts it.

### Draw · `Ribbons/PowerPoint → Draw`
Nine groups. 15 commands. See `Ribbons/Word → Draw` for the tab's shape.

- [ ] **Word's tab without two groups**: no Editing (Ink Editor) and no Drawing Canvas — the strip goes straight
  from Stencils to Input Mode.
- [ ] Eraser is the same **split button** whose face is a toggle that draws pressed.
- [ ] The same six commands open something; Select Objects is the only survivor.

*Guesses here:* the census counts seven controls in Pens where Word counts six, and the same three commands are
drawn — nothing is padded to the count.

### Design · `Ribbons/PowerPoint → Design`
Three groups — Themes, Variants, Customise. 4 commands.

- [ ] Expand Themes: nine themes by name, each a letter over four accent colours. Its footer holds **Browse for
  Themes** and **Save Current Theme**.
- [ ] Expand Variants: the current theme's four.
- [ ] ⚠ **Open the Variants flyout and press Colours at its foot.** Check the menu appears **beside the button**
  and the flyout behaves. Repeat for Fonts, Effects and Background Styles.
- [ ] Press Slide Size: Standard, Widescreen, Custom Slide Size. Format Background is a plain button.
- [ ] Check **Format Background fits its large button** — *Background* is ten letters and nobody has measured it.
- [ ] Nothing survives; Themes and Variants stand last.

*Guesses here:* the footer-button treatment of Colours, Fonts, Effects and Background Styles. Office's Designer
group is out of scope.

### Transitions · `Ribbons/PowerPoint → Transitions`
Three groups — Preview, Transition Styles, Timing. 10 commands.

- [ ] Expand the gallery: None, then **thirteen Subtle, twenty-nine Exciting, seven Dynamic Content**, in
  Office's order, starting on **Fade**. Open the flyout to see the headings.
- [ ] Judge the pictograms: an empty frame for None, a half-covered frame for Push and Wipe, bars for Cut.
- [ ] ⚠ **Effect Options follows the transition.** On Fade: two entries, Smoothly checked and Through Black. Pick
  Push: four edges. Pick Split: Vertical Out, Vertical In, Horizontal Out, Horizontal In. Pick **Flash**: the
  button goes **unavailable**.
- [ ] Effect Options carries no icon, so it is small where Office draws it large.
- [ ] Timing: Sound on *[No Sound]* over Office's whole list down to *Other Sound…*; Duration a combo box on
  **00.70** — pick 01.00, then type 01.25.
- [ ] Then Apply To All, the *Advance Slide* heading, **On Mouse Click ticked**, After unticked, the advance time
  on 00:00.00.
- [ ] ⚠ Office stacks Timing in **two columns**; this draws one row at full width. Judge whether that is
  acceptable — the order is Office's, the columns are not.
- [ ] Preview is large with a slide-transition glyph; Apply To All is a plain labelled button. No launchers;
  Preview gives way first.

*Guesses here:* many transitions' entry lists, from memory; Strips' place last in Subtle; Duration as a combo box
rather than a measure input; the group labels (*Transition Styles* is Office's *Transition to This Slide*), and
Timing drawing six commands where the census counts two.

### Animations · `Ribbons/PowerPoint → Animations`
Four groups — Preview, Animations, Custom Animation, Timing. 12 commands.

- [ ] Expand the gallery: None, **thirteen Entrance, nineteen Emphasis, thirteen Exit, six Motion Paths**,
  starting on **Fly In**. The flyout's headings are Office's.
- [ ] The footer carries More Entrance Effects, More Emphasis Effects, More Exit Effects, More Motion Paths and a
  **greyed OLE Action Verbs**.
- [ ] Judge the pictures: Entrance stars solid, Emphasis stars on a soft ground, Exit stars outlined, a motion
  path a dashed line.
- [ ] Press Effect Options: Fly In's eight directions (From Bottom checked) and three sequences (As One Object
  checked).
- [ ] ⚠ **Unlike Transitions', Effect Options does not follow the gallery**: pick Spin and it still offers Fly
  In's entries. Judge whether that is acceptable.
- [ ] Preview is a **split button** (Preview, and AutoPreview checked). Add Animation is large and opens the
  gallery's effects under the same headings. Trigger opens *On Click of* the slide's shapes.
- [ ] Animation Pane is a toggle; Animation Painter a plain button.
- [ ] Timing: Start a dropdown on **On Click**, Duration a combo box on **00.50**, Delay a combo box on **00.00**;
  Move Earlier and Move Later carry arrows and labels.
- [ ] Nothing survives; the one launcher is on the Animations group.

*Guesses here:* the gallery pictures; two group labels are the census's (Office writes *Animation* and *Advanced
Animation*).

### Slide Show · `Ribbons/PowerPoint → Slide Show`
Four groups — Start Slide Show, Rehearse, Set Up, Monitors. 14 commands.

- [ ] ⚠ **Rehearse is guessed end to end**: one large Rehearse with Coach (a figure speaking), drawn **second**
  where Microsoft 365 draws it though the census declares it after Set Up. It opens nothing.
- [ ] ⚠ Press Present Online: Office Presentation Service, Skype for Business.
- [ ] ⚠ Press Custom Slide Show: **Custom Shows… alone**, because the deck has saved none.
- [ ] ⚠ Press Record's **arrow**: From Current Slide…, From Beginning…, then a *Clear* section of four. Press its
  **face**: nothing opens.
- [ ] ⚠ Collapse Set Up: **Hide Slide** stays beside the trigger (large, a dashed slide) — the tab's only
  survivor. Press it: it draws pressed with a filled glyph.
- [ ] Monitors: Monitor reads *Automatic* and lists Automatic and Primary Monitor; **Use Presenter View is
  ticked**.
- [ ] Set Up's three checkboxes are **ticked**: Play Narrations, Use Timings, Show Media Controls. Untick one.
- [ ] Judge the glyphs: From Beginning's stacked slides with an arrow against From Current Slide's one slide with
  a play mark; Present Online's presenter; Custom Slide Show's stacked slides; Set Up Slide Show's slide with a
  cog; Rehearse Timings' stopwatch; Record's slide with a record mark.
- [ ] No launchers, no exclusive set. Captions & Subtitles is out of scope and absent.

*Guesses here:* the whole Rehearse group; all three menus, from memory; that the dashed slide reads with no
label; every glyph.

### Recording · `Ribbons/PowerPoint → Recording`
Ten groups — Record, Recording, Content, Camera, Auto-play Media, Edit, Save, Export, Preview, Help. 15 commands.

- [ ] ⚠ **Recording, Edit, Export, Preview and Help are guessed entirely** — the census names each group, counts
  its controls and names none. Recording is one large Record split button; Edit is Clear Recording (a bin) and
  Reset to Cameo (a reset loop); Export one dropdown; Preview one play mark; Help one question mark.
- [ ] ⚠ **Record is drawn twice in effect**: the Record group's From Beginning and From Current Slide, then the
  Recording group's Record. Office never draws both. Judge whether that reads as a defect.
- [ ] ⚠ Press each arrow: Record (From Current Slide…, From Beginning…), Clear Recording (on Current Slide, on
  All Slides), Reset to Cameo (the same pair), Export (Export Video, Customize Export). Record's face opens
  nothing.
- [ ] Press Screenshot, Cameo, Video and Audio and compare with `Ribbons/PowerPoint → Insert`: Screen Clipping;
  This Slide and All Slides; This Device…, Stock Videos…, Online Videos…; Audio on My PC…, Record Audio….
  **They should be identical.**
- [ ] Judge the glyphs: Save as Show's save-with-an-arrow; **Export to Video's film clip, which must read
  differently from Video's camera two groups before it**; Export's arrow leaving a box.
- [ ] Drag narrow: **nothing survives**; each popup holds every command in declared order.
- [ ] The group labels *Recording* and *Auto-play Media* are the census's (Office writes Record and Auto-Play
  Media). No launchers, no toggles.

*Guesses here:* five whole groups; the group order (the census's declaration); the menus, from Microsoft's
support wording.

### Review · `Ribbons/PowerPoint → Review`
Seven groups — Proofing, Accessibility, Language, Comments, Compare, Activity, Ink. 19 commands.

- [ ] ⚠ **Activity is one plain labelled button, *Show Changes*, guessed entirely.** The census counts two
  controls and names nothing.
- [ ] ⚠ **Ink is drawn last, after Activity**; the census declares it fifth, before Compare.
- [ ] Show Comments and Hide Ink are split buttons whose face is a toggle, **both starting unpressed**. Press
  Show Comments' face: it fills. Press its arrow: Comments Pane and Show Markup (ticked) open and **the face does
  not move**.
- [ ] Press Hide Ink's arrow: Hide Ink, Delete All Ink in Presentation.
- [ ] **Compare is PowerPoint's own group, seven commands**: Compare, then Accept and Reject as large split
  buttons (Accept Change, Accept All Changes to This Slide, Accept All Changes to the Presentation, and Reject's
  three), then Previous Change, Next Change, Reviewing Pane (a plain toggle), End Review.
- [ ] ⚠ Office greys all but Compare until a comparison is under way; **they are drawn available here**.
- [ ] Collapse Comments: **Previous Comment and Next Comment** stay — small, where Office draws them large.
- [ ] Press Check Accessibility (Check Accessibility, Alt Text, Reading Order Pane, Options: Accessibility),
  Language (Set Proofing Language, Language Preferences), Delete (Delete, then all comments and ink on this
  slide, and in this presentation).
- [ ] Six commands carry no icon: Compare, Previous Change, Next Change, End Review, Show Changes, Hide Ink.
  **Spelling is large** (one word fits, unlike Word's Spelling & Grammar). Translate is a plain large button with
  no arrow. No launchers.

*Guesses here:* the whole Activity group; both Ink positions; both split-button arrows' entries and starting
positions; Reviewing Pane's glyph. Insights and Chinese Translation are out of scope.

### View · `Ribbons/PowerPoint → View`
Seven groups — Presentation Views, Master Views, Show, Zoom, Colour/Greyscale, Window, View Direction. 24 commands.

- [ ] ⚠ **View Direction is guessed end to end**: Left-to-Right (pressed) and Right-to-Left, small, drawn last.
  Press Right-to-Left: Left-to-Right releases.
- [ ] ⚠ **Three exclusive sets, each independent.** Normal starts pressed: press Slide Sorter, Normal releases;
  press it again, it stays.
- [ ] ⚠ Colour (pressed), Greyscale and Black and White do the same — and **pressing Greyscale leaves the
  presentation view alone**.
- [ ] ⚠ Tab to a view and press **Space**: the same, by keyboard.
- [ ] ⚠ Collapse Zoom: **Fit to Window** stays (large, a landscape frame in fit corners) — the tab's only
  survivor; Zoom opens from the popup.
- [ ] Show: **Ruler ticked**, Gridlines and Guides unticked; Notes draws pressed when pressed. The launcher at
  Show's corner reads *Grid Settings*.
- [ ] Press Switch Windows: one window, *1 Where the time went*, checked — the tab's only menu.
- [ ] Three commands are small where Office draws them large: Outline View, Notes Master, Notes.
- [ ] Judge the sixteen glyphs, filled while pressed: Normal's window with a left pane, Outline View's stepped
  bars, Slide Sorter's grid, Notes Page's notepad, Reading View's open book, Slide Master's slide with a pencil,
  Handout Master's stacked pages, Notes Master's notepad with a pencil, Notes' bottom pane, Zoom's magnifier,
  Colour's palette, Greyscale's struck palette, Black and White's half circle, Arrange All's two columns,
  Cascade's stack, Move Split's four arrows.

*Guesses here:* the whole View Direction group; that Fit to Window's glyph reads with no label; every glyph; the
launcher's name. *Colour/Greyscale* is spelt as the census spells it.

### Slide Master · `Ribbons/PowerPoint → Slide Master` *(view tab)*
Six groups — Edit Master, Master Layout, Edit Theme, Background, Size, Close. 17 commands.

- [ ] ⚠ **Colours, Fonts and Effects are in Edit Theme**, small in a column beside a large Themes. Judge whether
  Microsoft 365 draws them in **Background** instead.
- [ ] ⚠ Press Themes: *This Presentation* (Office Theme, checked), then *Office* with **thirty themes** from
  Facet to Wood Type, then Browse for Themes… and Save Current Theme….
- [ ] ⚠ Press Colours (**twenty-four sets** and Customise Colours…), Fonts (**twenty pairs** and Customise
  Fonts…), Effects (**fifteen**), Background Styles (Style 1 checked to Style 12, Format Background…, Reset Slide
  Background).
- [ ] ⚠ Compare those four lists with `Ribbons/PowerPoint → Design`'s Variants footer, `Ribbons/Word → Design`
  and `Ribbons/Excel → Page Layout`: **all four now draw the same lists.**
- [ ] ⚠ Press Insert Placeholder's **arrow**: Content, Content (Vertical), Text, Text (Vertical), Picture, Chart,
  Table, SmartArt, Media, Online Image — **none with a glyph**. Its face does nothing.
- [ ] Press Preserve: the pin fills. Press again: it releases.
- [ ] Three checkboxes: **Title and Footers ticked**, stacked after Insert Placeholder; **Hide Background
  Graphics unticked**, under Background Styles.
- [ ] The launcher at Background's corner reads *Format Background*. No other group has one.
- [ ] Judge the glyphs: Insert Slide Master's slide with a title and a plus, Insert Layout's layout, the bin,
  Rename's cursor in a field, Preserve's pin, Master Layout's slide with a title and a tick, Insert Placeholder's
  slide with a picture and lines, Themes' swatch book, then Design's five and the cross in a square.
- [ ] Drag narrow: **no survivor anywhere**.
- [ ] Open `Shell/PowerPoint`: no Slide Master tab and no Slide Master menu on the page.

*Guesses here:* the placement of Colours/Fonts/Effects; every theme list; Preserve's unpressed start; every glyph.

### Home (Slide Master view) · `Ribbons/PowerPoint → Slide Master Home` *(view tab)*
Six groups — Clipboard, Master Slides, Font, Paragraph, Drawing, Editing. 44 commands.
**The second tab called Home**, the one Slide Master view shows beside Slide Master.

- [ ] ⚠ **Master Slides** is the one group of its own, second, where Home has Slides: Insert Slide Master and
  Insert Layout large, then Layout, Reset and Section small in a column. **The census counts 9; five are drawn.**
- [ ] ⚠ **Layout's glyph is new** — a frame split into a title row and two panes, not Home's Layout glyph,
  because Insert Layout beside it wears that one. **Compare the two: they must not read as one command twice.**
- [ ] ⚠ Press Layout: an *Office Theme* section with **eleven layouts**, Title Slide to Vertical Title and Text,
  none ticked. Press Section: Add Section, Rename Section, Remove Section, Remove All Sections, a separator,
  Collapse All, Expand All. **Office greys Section and Reset in master view; here they are available.**
- [ ] Switch between this story and `Home`: Clipboard, Font, Paragraph, Drawing and Editing must match **command
  for command, glyph for glyph, pressed state for pressed state** (Bold and Align Left pressed), with the same
  four launchers.
- [ ] Inspect a command: its id carries `slide-master-home`.
- [ ] Change the font size here, then open `Home`: **Home's field has not changed.**
- [ ] Collapse: Font keeps Bold, Italic and Underline; Paragraph keeps Align Left, Centre and Align Right; Master
  Slides, Clipboard, Drawing and Editing keep nothing.
- [ ] Open `Shell/PowerPoint`: **one Home tab**, the ordinary one, and no Layout or Section menu of this tab.

*Guesses here:* the reading of Master Slides' count of 9; Layout's new glyph; every Layout and Section entry, and
the greying.

### Handout Master · `Ribbons/PowerPoint → Handout Master` *(view tab)*
Five groups — Page Setup, Placeholders, Edit Theme, Background, Close. 14 commands.

- [ ] ⚠ **Page Setup draws three large dropdowns** — Handout Orientation, Slide Size, Slides Per Page — where
  the census counts 11.
- [ ] ⚠ Judge **Slides Per Page's glyph**: a page divided into four frames, the glyph Excel's Arrange All draws
  for four tiled windows. Does it read as slides on a handout? Press it: 1, 2, 3, 4, **6 (checked)**, 9 Slides,
  Outline.
- [ ] Press Handout Orientation (Portrait checked, Landscape) and Slide Size (Standard 4:3, **Widescreen 16:9
  checked**, Custom Slide Size…).
- [ ] **Four checkboxes**, Header and Date over Footer and Page Number, **all ticked**. Untick one.
- [ ] Switch between this story and `Slide Master`: Edit Theme, Background and Close must match **command for
  command, glyph for glyph and list for list**, with the one *Format Background* launcher.
- [ ] Inspect a command: its id carries `handout-master`.
- [ ] Tick Hide Background Graphics here, then open `Slide Master`: **that tab's checkbox has not changed.**
- [ ] Office greys Themes here; it is drawn available.
- [ ] Drag narrow: no survivor anywhere.
- [ ] Open `Shell/PowerPoint`: no Handout Master tab.

*Guesses here:* that the census's 11 counts the menus' eleven choices; Slides Per Page's glyph, labels and start;
that all four checkboxes start ticked.

### Notes Master · `Ribbons/PowerPoint → Notes Master` *(view tab)*
Five groups — Page Setup, Placeholders, Edit Theme, Background, Close. 15 commands.

- [ ] ⚠ **Page Setup draws two large dropdowns** — Notes Page Orientation and Slide Size — where the census
  counts 3. **Handout Master's reading of its own Page Setup would give 4 here, so the two counts disagree with
  each other.** Judge which is right.
- [ ] ⚠ **The collapse order is the reverse of Handout Master's.** Drag narrow: Close first, then Page Setup,
  Placeholders and Edit Theme, **Background last**. Do the same on `Handout Master`: there **Background goes
  before Page Setup**. Judge whether that difference is right for Office.
- [ ] **Six checkboxes, all ticked**: Header, Slide Image, Footer, Date, Body, Page Number. Office draws two
  columns of three, Header/Slide Image/Footer down the first — **check the order they flow in here**, and untick
  one: only it unticks.
- [ ] Press Notes Page Orientation and Slide Size: they must match `Handout Master`'s **entry for entry**.
- [ ] Edit Theme, Background and Close must match `Slide Master`'s, with the one *Format Background* launcher.
  Inspect a command: its id carries `notes-master`.
- [ ] Tick Hide Background Graphics here, then open `Handout Master`: that tab's checkbox has not changed.
- [ ] Drag narrow: no survivor. Open `Shell/PowerPoint`: no Notes Master tab.

*Guesses here:* that the census counts Slide Size's face and arrow as two; the checkbox order and that all six
start ticked.

### Black and White · `Ribbons/PowerPoint → Black and White` *(view tab)*
Two groups — Colour Mode, Close. 11 commands.

- [ ] ⚠ **Colour Mode is ten toggles in one exclusive set, not a gallery.** Automatic starts pressed. Press
  Black: it fills and Automatic releases. Press Black again: it stays.
- [ ] ⚠ Tab to Inverse Greyscale and press **Space**: the same, by keyboard.
- [ ] ⚠ **Judge whether a row of toggles reads as *choose one setting for this object*.** Office greys the ten
  with nothing selected and highlights none for a mixed selection; here one always holds.
- [ ] ⚠ **Five settings carry no glyph and are small**: Automatic, Grey with White Fill, Black with Greyscale
  Fill, Black with White Fill, Black and White. Check the small labels are readable in their columns.
- [ ] ⚠ Check Office's order survives: **Automatic, three large, five small, Don't Show**.
- [ ] ⚠ **Four large labels wrap unmeasured**: Light Greyscale, Inverse Greyscale, Don't Show, Back To Colour
  View (*Back To* over *Colour View*). **Look for a clipped third line.**
- [ ] Judge the glyphs: Greyscale's struck palette, Light Greyscale's sun, Inverse Greyscale's half-dark circle,
  Don't Show's struck eye, Back To Colour View's cross in a square.
- [ ] Press White here, then open `View`: its Colour/Greyscale set **has not changed**. Open `Greyscale`: its set
  still holds Automatic.
- [ ] The group is labelled **Colour Mode**, the census's, where Office writes *Change Selected Object*. Office's
  *Grayscale*, *Gray* and *Color* are spelt Greyscale, Grey and Colour.
- [ ] Drag narrow: Close collapses first, then Colour Mode; no survivor.
- [ ] Open `Shell/PowerPoint`: no Black and White tab.

*Guesses here:* Automatic as the start; that each large label fits; every glyph; the toggle-row treatment where
Office draws coloured swatches.

### Greyscale · `Ribbons/PowerPoint → Greyscale` *(view tab)*
Two groups — Colour Mode, Close. 11 commands. **The same two functions as Black and White**, so that tab's items
hold here unchanged.

- [ ] ⚠ **This tab's set is its own.** Press Black here: it fills and Automatic releases. Open `Black and White`:
  its set **still holds Automatic**. Come back: Black still holds.
- [ ] ⚠ Automatic starts pressed here too.
- [ ] **Compare it with `Black and White`**: the two tabs should look identical apart from the tab's name — same
  labels, sizes, glyphs and order, with Back To Colour View last. **Any difference is a defect.**
- [ ] Check the spelling: Greyscale, Grey, Back To Colour View; the group labelled Colour Mode.
- [ ] Drag narrow: Close first, then Colour Mode; no survivor. Open `Shell/PowerPoint`: no Greyscale tab.

*Guesses here:* that Office keeps the two tabs' sets apart — if it keeps one setting per shape, a
document-backed host would press the same member on both.

### Print Preview · `Ribbons/PowerPoint → Print Preview` *(view tab)*
Four groups — Print, Page Setup, Zoom, Preview. 10 commands.

- [ ] ⚠ **Colour/Greyscale is a field in Page Setup**, not a submenu of Options. It sits under Print What with
  Orientation between them: Colour (selected), Greyscale, Pure Black and White.
- [ ] ⚠ **Options opens a menu, not a dialog**, unlike Word's. Press it: Header and Footer…, then three unticked
  checkboxes (Scale to Fit Paper, Frame Slides, Print Comments and Ink Markup), a *Print Order* section with
  Horizontal checked and Vertical, and Print Hidden Slides unticked. **No Colour/Greyscale entry.**
- [ ] ⚠ Press Print What: **nine shapes** — Slides (selected), Handouts at 1, 2, 3, 4, 6 and 9 slides per page,
  Notes Pages, Outline View. **The longest label must fit the field or ellipsise inside it, not grow it.**
- [ ] ⚠ Orientation is **small** under Print What and opens Portrait (checked), Landscape. Office greys it while
  Print What is Slides; here it is available.
- [ ] Collapse Zoom: **Fit to Window** stays. Collapse Preview: **Next Page and Previous Page** stay, and Close
  Print Preview opens from the popup.
- [ ] Every glyph is one the subset already carries for the same command: a printer, a cog, the turning page, a
  magnifier, the landscape frame in fit corners, page-with-arrow down and up, a cross in a square.
- [ ] *Close Print Preview* must wrap without an ellipsis. **No launcher on any group.**
- [ ] Open `Shell/PowerPoint`: no Print Preview tab.

*Guesses here:* Colour/Greyscale's placement; every Options entry and tick; Orientation's size.

### Background Removal · `Ribbons/PowerPoint → Background Removal` *(view tab)*
Two groups — Refine, Close. 4 commands. **Word's tab under PowerPoint's ids.**

- [ ] ⚠ Neither pencil starts pressed; press Keep, then Remove (Keep releases), then Remove again (it releases
  and neither holds).
- [ ] **Inspect a pencil**: its `exclusive` attribute is `powerpoint.background-removal.refine`, **not Word's**.
- [ ] ⚠ Refine has two commands, not three — Delete Mark is absent.
- [ ] ⚠ Judge the four circles. All four commands are large; the long labels must wrap to two lines without an
  ellipsis.
- [ ] Collapse both: no survivors; the set still holds at most one inside the popup.
- [ ] Open `Shell/PowerPoint`: no Background Removal tab.

*Guesses here:* as Word's — the release on a second press, the empty start (View's three sets keep one pressed),
every glyph, Delete Mark's absence.

### Table Design · `Ribbons/PowerPoint → Table Design` *(contextual — Table Tools)*
Four groups — Table Style Options, Table Styles, WordArt Styles, Draw Borders. 19 commands.
*(See `Ribbons/Word → Table Design` for the band and collapse order, which hold on every contextual story.)*

- [ ] ⚠ Open **Shading**: theme, standard and recent colours with *No Fill*, then More Fill Colours…,
  **Eyedropper**, Picture…, then **Gradient, Texture and Table Background**, each a submenu. *(Table Background's
  own colour grid is not drawn, only its four commands.)*
- [ ] ⚠ Open **Text Fill**: the same without Table Background, on *No Fill*. Open **Text Outline**: *No Outline*,
  More Outline Colours…, Eyedropper, then Weight, Sketched and Dashes. Open **Pen Colour**: More Colours… and
  Eyedropper, starting on **Text 1**.
- [ ] ⚠ In any of the four: Arrow Down from the last row of recent colours moves onto the **first entry**; Arrow
  Up returns; **Arrow Right opens a submenu**; choosing anything closes the picker.
- [ ] ⚠ Expand **Quick Styles**: twenty letters *A*, four rows of five, with Clear WordArt under them. **Nothing
  is selected.** Judge whether the shadow, glow, bevel, reflection, gradient and pattern styles read as different
  styles.
- [ ] ⚠ Expand **Table Styles**: *Best Match for Document* (14), *Light* (21), *Medium* (28), *Dark* (11), one
  family to a row of seven, **Medium Style 2 - Accent 1 selected**. **No style appears twice.** Footer: Clear
  Table.
- [ ] ⚠ Draw Table and Eraser: neither starts pressed; press Draw Table then Eraser (Draw Table releases); press
  Eraser again (it releases and neither holds).
- [ ] Press **Effects**: Cell Bevel (No Bevel and twelve), Shadow (No Shadow, Outer nine, Inner nine, Perspective
  five, Shadow Options…), Reflection (No Reflection, nine, Reflection Options…).
- [ ] Press **Text Effects**: Shadow and Reflection again, then Glow (twenty-four, More Glow Colours, Glow
  Options…), Bevel (ending in 3-D Options…), 3-D Rotation (Parallel, Perspective, Oblique), Transform (Follow
  Path four, Warp thirty-two). **Hover an entry with a submenu to open it.**
- [ ] Press Borders' arrow (small split button): **twelve entries**, No Border and All Borders first, then
  Outside, Inside, the four edges, the two inside lines, the two diagonals.
- [ ] Six checkboxes, two columns of three. **Header Row and Banded Rows ticked — unlike Word's, which also ticks
  First Column.**
- [ ] Pen Style lists No Border and **eight dashes**, Solid selected; Pen Weight ¼ pt to 6 pt, **1 pt** selected.
- [ ] Two launchers: *Format Text Effects* at WordArt Styles', *Format Shape* at Draw Borders'.
- [ ] Drag narrow: Table Style Options, WordArt Styles and Draw Borders first, **Table Styles last**. No survivor.
- [ ] Open `Shell/PowerPoint`: **its strip draws Picture Tools**, so there is no Table Design tab and none of
  these menus on the page.

*Guesses here:* every picker entry and label; every Quick Styles name and picture; the Best Match reading and
every table-style picture; the checkbox start; both launchers, and that the first two groups have none; every
glyph (all reused).

### Layout · `Ribbons/PowerPoint → Table Layout` *(contextual — Table Tools)*
Seven groups — Table, Rows & Columns, Merge, Cell Size, Alignment, Table Size, Arrange. 28 commands.
**No Draw or Data group; it does have Table Size and Arrange.**

- [ ] ⚠ Press **Cell Margins**: Normal (ticked), None, Narrow, Wide, **each with its four measures as a second
  line**, then Custom Margins…. Judge the `padding-left` glyph again.
- [ ] ⚠ **Two alignment sets, each holding one.** Align Left and Align Top start pressed. Press Align Centre: it
  fills, Align Left releases, **Align Top stays pressed**. Press Align Bottom: Align Top releases and Align
  Centre stays.
- [ ] ⚠ Check the six read as **Office's two rows of three**: Left, Centre, Right over Top, Centre Vertically,
  Bottom.
- [ ] ⚠ **Arrange has four commands, not six** — no Group and no Rotate. Press Bring Forward's arrow (Bring
  Forward, Bring to Front) and Send Backward's (Send Backward, Send to Back); their faces do nothing.
- [ ] ⚠ Press Align: the six alignments, Distribute Horizontally and Vertically, then **Align to Slide ticked**
  and Align Selected Objects.
- [ ] Collapse Rows & Columns: **Insert Above, Insert Below, Insert Right** stay. Collapse Merge: **Merge Cells**
  alone. Collapse Cell Size: **Distribute Rows and Distribute Columns**. Collapse Alignment: **Align Left, Align
  Centre, Align Right**. Table, Table Size and Arrange keep nothing.
- [ ] Collapse order: Table and Merge first, then Cell Size, Table Size and Arrange, **Rows & Columns and
  Alignment last**.
- [ ] Press Select: **Select Table, Select Column, Select Row — no cell.** Press Delete: **Delete Columns, Rows,
  Table — no cell.** PowerPoint cannot select or delete one.
- [ ] Press Text Direction: Horizontal (ticked), Rotate all text 90°, Rotate all text 270°, **Stacked**, then
  More Options…. *(Word's has no Stacked.)*
- [ ] Cell Size's Height and Width start on **1.02 cm and 5.84 cm**; Table Size's on **2.04 cm and 29.21 cm**,
  stepping by 0.01. **The two pairs do not follow each other.** Lock Aspect Ratio starts unticked.
- [ ] **Large where Word's are small**: Select and View Gridlines, Merge Cells and Split Cells. View Gridlines
  starts pressed.
- [ ] Hover an alignment: *Align Centre*, *Centre Vertically* — the census's spelling.
- [ ] No dialog launcher on any group. Split Cells, More Options…, Custom Margins… and Selection Pane open a
  dialog or pane in Office and nothing here.
- [ ] Open `Shell/PowerPoint`: no Table Layout tab.

*Guesses here:* every cell-margin measure; both alignment starts and the grid; PowerPoint's Align list; which
three inserts and which three alignments survive; every measure; the four sizes and View Gridlines' start; that
no group has a launcher; every glyph.

### Picture Format · `Ribbons/PowerPoint → Picture Format` *(contextual — Picture Tools)*
Six groups — Adjust, Picture Styles, Accessibility, Arrange, Size, Image Play. 23 commands.
**Word's tab through Word's functions** — see `Ribbons/Word → Picture Format` for every list.

- [ ] ⚠ **Convert to SmartArt, where Word says Picture Layout.** Same place (third in Picture Styles' column),
  same glyph, same thirty-one layouts. Judge the label.
- [ ] ⚠ Image Play: Play Animation, a large toggle, **starting pressed**.
- [ ] ⚠ **Arrange has six commands, not Word's eight** — no Position and no Wrap Text. Bring Forward's and Send
  Backward's arrows list **without Word's text layers**.
- [ ] ⚠ Press Align: six alignments, two distributions, then **Align to Slide ticked** and Align Selected
  Objects. Group opens Group, Regroup, Ungroup; Rotate its four turns and flips and More Rotation Options….
  **All six are small.**
- [ ] ⚠ Open **Picture Border**: *No Outline*, starting on none, with More Outline Colours…, **Eyedropper**,
  Weight ▸, Sketched ▸, Dashes ▸. **Word's has no Eyedropper.**
- [ ] ⚠ Height and Width start on **19.05 cm and 25.4 cm**, stepping by 0.01. They do not follow each other.
- [ ] Two launchers: *Format Picture*, and **Size and Position** at Size's (Word's says Layout).
- [ ] Press Corrections, Colour, Artistic Effects and Transparency: **each is Word's whole list** with the
  unchanged state checked.
- [ ] Expand Quick Styles: Word's twenty-eight, in this deck's palette. Nothing is selected.
- [ ] Press Crop's face (fills, then releases) and its arrow (Crop, Crop to Shape 147, Aspect Ratio, Fill, Fit).
- [ ] Press Picture Effects: Preset, Shadow, Reflection, Glow, Soft Edges, Bevel, 3-D Rotation.
- [ ] Alt Text is a large toggle, unpressed. No pane opens.
- [ ] Drag narrow: Image Play first, Accessibility next, then Arrange and Size, Adjust and Picture Styles last.
  No survivor.
- [ ] **Also in `Shell/PowerPoint`**, which draws Picture Tools: select Picture Format under the band and check
  every list, field, gallery and picker is the same.

*Guesses here:* the Convert to SmartArt label; the whole of Image Play; both measures; both launcher labels;
every gallery look.

### Shape Format · `Ribbons/PowerPoint → Shape Format` *(contextual — Drawing Tools)*
Six groups — Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size. 21 commands.
**This is the story that covers Theme Styles, Other Theme Fills, the Shapes list and Edit Shape for all three
applications.**

- [ ] ⚠ Expand the Theme Styles gallery: *Theme Styles*, **six rows of seven *Abc* boxes** (Coloured Outline,
  Coloured Fill, Light 1 Outline Coloured Fill, Subtle Effect, Moderate Effect, Intense Effect — each for Dark 1
  and the six accents), then *Presets*, **seven Transparent, Coloured Outline boxes over a chequerboard**.
- [ ] ⚠ **Do the six rows read as six families?** And is Transparent told from Coloured Outline? Nothing is
  selected; hover one to read its name.
- [ ] ⚠ **Press *Other Theme Fills* in the gallery's footer**: a menu of *Style 1* to *Style 12* anchored to the
  footer. **Check it opens from inside the expanded gallery and that the gallery's popup does not swallow it.**
- [ ] ⚠ Press **Shapes** (a large button, where Office draws an in-ribbon gallery of outlines): Lines (12),
  Rectangles (9), Basic Shapes (43, starting with Text Box and Vertical Text Box), Block Arrows (27), Equation
  Shapes (6), Flowchart (28), Stars and Banners (20), Callouts (16), **Action Buttons (12)** — every shape by
  name. **The same list opens from `Insert → Shapes` in all three applications.**
- [ ] ⚠ Open **Shape Fill** (Accent 1): *No Fill*, More Fill Colours…, Eyedropper, Picture…, Gradient ▸,
  Texture ▸. Open **Shape Outline** (Accent 1, Darker 50%): *No Outline*, More Outline Colours…, Eyedropper,
  Weight ▸, Sketched ▸, Dashes ▸, **Arrows ▸** (eleven line ends, then More Arrows…).
- [ ] ⚠ Press **Edit Shape**: Change Shape ▸ (the Shapes list without Lines or the text boxes, **with** Action
  Buttons), Edit Points, and **Reroute Connectors unavailable with its explanation**. Arrow onto it: it stays in
  the sequence and does nothing.
- [ ] Press Merge Shapes: Union, Combine, Fragment, Intersect, Subtract. Judge its `shape-union` glyph and Edit
  Shape's `bezier-curve-square` — the two new ones.
- [ ] Press Shape Effects: Preset, Shadow, Reflection, Glow, Soft Edges, Bevel, 3-D Rotation.
- [ ] WordArt Styles is Table Design's: Quick Styles' twenty letters and Clear WordArt, **Text Fill starting on
  Background 1**, Text Outline, Text Effects' six submenus.
- [ ] Arrange is Picture Format's six, all small, with **Align to Slide ticked**.
- [ ] Height and Width start on 2.54 cm, stepping by 0.01; they do not follow each other.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, *Size and Position*.
- [ ] Text Box is a small plain button; **Alt Text is a large toggle, unpressed — its glyph is the weakest on the
  tab.**
- [ ] Drag narrow: Accessibility first, then Insert Shapes, WordArt Styles, Arrange and Size, Shape Styles last.
  No survivor.
- [ ] Open `Shell/PowerPoint`: no Shape Format tab.

*Guesses here:* every Theme Styles name and look, and that Presets is one row; the twelve Other Theme Fills and
that nothing else is in the submenu; every shape name; both picker starts and every entry; both measures; all
three launchers.

### Chart Design · `Ribbons/PowerPoint → Chart Design` *(contextual — Chart Tools)*
Four groups — Chart Layouts, Chart Styles, Data, Type. 9 commands, every one large.
**It is `Ribbons/Word`'s Chart Design under PowerPoint's ids — any difference between the two is a defect.**

- [ ] **Judge the two side by side with `Ribbons/Word → Chart Design`.** Office's two tabs do not differ.
- [ ] ⚠ Change Chart Type: eight submenus, each ending on *More … Charts…*, **no Map family** though PowerPoint's
  dialog lists one.
- [ ] ⚠ Add Chart Element: Word's eleven submenus and Word's starts. **Lines and Up/Down Bars open**, where
  Office greys both.
- [ ] ⚠ Chart Styles: *Style 1* to *Style 16* on Style 1, in the catalogue's one specimen theme, **so the
  pictures match Word's exactly**.
- [ ] ⚠ Quick Layout's four-tiled-regions glyph — the weakest on the tab. Its menu is *Layout 1* to *Layout 11*.
- [ ] Press Edit Data's arrow: *Edit Data*, *Edit Data in Excel*.
- [ ] **Data is Word's four** (the census's count of 6 is Word's own; Excel's is 2). Switch Row/Column, Select
  Data and Refresh Data are plain large buttons, drawn available.
- [ ] Press Change Colours: *Colourful* (1–4) and *Monochromatic* (1–13), one set, on *Colourful Palette 1*.
- [ ] Spelling: *Change Colours*, *Colourful*, *Centred Overlay*, *Centre*. No launcher.
- [ ] Drag narrow: Chart Styles and Type first, then Data, Chart Layouts last. No survivor.
- [ ] Open `Shell/PowerPoint`: no Chart Tools band.

*Guesses here:* as Word's, plus that PowerPoint's inserted chart starts as Word's, and both Edit Data labels.

### Chart Format · `Ribbons/PowerPoint → Chart Format` *(contextual — Chart Tools)*
Seven groups — Current Selection, Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size.
22 commands. **Word's Chart Format less Position and Wrap Text — every other difference is a defect.**

- [ ] ⚠ Open Shape Fill and Shape Outline: **both start on *No Fill* and *No Outline***, where Word's start on
  white and a light grey — a chart PowerPoint inserts is transparent on the slide. **Check the *none* chip is the
  one marked.**
- [ ] ⚠ **Every colour picker carries an Eyedropper, second under the palette**: Shape Fill (More Fill Colours…,
  Eyedropper, Picture…, Gradient ▸, Texture ▸), Shape Outline (More Outline Colours…, Eyedropper, Weight ▸,
  Dashes ▸ — **still no Sketched or Arrows**), Text Fill (Shape Fill's five, on *Text 1, Lighter 40%*), Text
  Outline (More Outline Colours…, Eyedropper, Weight ▸, **Sketched ▸**, Dashes ▸, on none).
- [ ] ⚠ Height **15.05 cm** and Width **22.58 cm**, stepping by 0.01 — the chart PowerPoint inserts on a slide
  with no content placeholder.
- [ ] ⚠ Chart Elements starts on *Chart Area* and lists Word's ten parts alphabetically, **the three series
  before the vertical axis**. Check the field fits *Vertical (Value) Axis Major Gridlines*.
- [ ] ⚠ Judge **Reset to Match Style's glyph** beside Format Selection's column chart with a pencil.
- [ ] Arrange is Shape Format's six, all small, with **Align to Slide ticked**. No Position, no Wrap Text.
- [ ] Press Shapes: the gallery with **no Action Buttons**. Change Shape is small, drawn available, wearing Edit
  Shape's glyph. No Edit Points, Text Box or Merge Shapes.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, **Size and Position** (Word's says *Layout*). None
  on Current Selection.
- [ ] Text Effects keeps Transform; Alt Text is a large toggle, unpressed.
- [ ] Drag narrow: Insert Shapes first, then Accessibility, then Current Selection, WordArt Styles, Arrange and
  Size, Shape Styles last. No survivor.
- [ ] Open `Shell/PowerPoint`: no Chart Tools band.

*Guesses here:* both picker starts; every picker entry; both measures; the Chart Elements order and labels; all
three launchers.

---

## 5 · Excel — 16 tabs

### File · `Ribbons/Excel → File`
Seven groups — Info, Open, Save, Share, Export, Publish, Help. 27 commands.

- [ ] **No Print group at all.** The strip goes Info, Open, Save, Share, Export, Publish, Help. *(Excel obviously
  has a File → Print page; the census dump carries no row, and inventing one was refused.)*
- [ ] **Publish is Excel's Power BI page**, three controls — the one File group where Office and the census agree
  exactly. Its headline carries **no icon**.
- [ ] **Info carries a fifth command**, Workbook Statistics, which the other two have no equivalent of.
- [ ] The collapse order is Word's.

*Guesses here:* the missing Print group is a **recorded census gap**, not a design decision — worth confirming
it is acceptable to ship this way.

### Home · `Ribbons/Excel → Home`
Eight groups — Clipboard, Font, Alignment, Number, Styles, Cells, Editing, Power Options. 43 commands.

- [ ] **Cells**: Insert, Delete, Format.
- [ ] ⚠ **Power Options is the one group in this catalogue whose content is honestly unknown.** The census gives
  an id, a count of one and an in-scope flag, and names no control. The command takes **the group's own label and
  no icon**. Judge whether shipping it is right.
- [ ] **Alignment is eleven commands and seven draw pressed**: Top, Middle and Bottom Align, Wrap Text, and Left,
  Centre and Right are toggles — **Bottom and Centre pressed**, because an unformatted cell is bottom-aligned.
- [ ] Collapse Alignment: **Left, Centre and Right** stay, and they draw **on the second row** where Office draws
  them, not ahead of Top Align.
- [ ] Clipboard, Number, Styles, Cells and Editing keep **no survivor** — every candidate opens a menu, AutoSum
  included (a split button in Office).
- [ ] Check **Underline** is present in Font.
- [ ] Three commands in Number carry no icon: Comma Style, Increase Decimal, Decrease Decimal. *(They were drawn
  with a plus and a minus, which mean insert and delete everywhere else on this tab.)*

*Guesses here:* Power Options entirely; which three per group survive.

### Insert · `Ribbons/Excel → Insert`
Ten groups — Tables, Illustrations, Charts, Sparklines, Slicers, Links, Comments, Text, Symbols, Cell Controls.
35 commands — **the largest tab of the three.**

- [ ] **Charts is eleven commands and eight are glyphs alone** — the one place on any Insert tab where Office
  draws no names. Hover each: the accessible name is Office's tooltip (*Insert Column or Bar Chart* and the rest).
- [ ] Press each chart family: a menu of real chart types under **Office's own section headings**.
- [ ] **Insert Combo Chart is the one labelled command in the row** — Fluent draws columns and lines, never both.
- [ ] Charts carries **the only dialog launcher on any application's Insert tab**.
- [ ] **PivotTable, the headline of the tab, carries no icon** — a labelled split button beside Recommended
  PivotTables (no glyph either) and a large Table.
- [ ] **The group Office calls Filters is labelled Slicers** (the census's `GroupSlicerInsert`), holding Slicer
  and Timeline.
- [ ] **Cell Controls is drawn last**, holding Checkbox — the one command on the tab that passes the first two
  demotion rules, and it still keeps nothing (a survivor would leave its collapsed popup empty).
- [ ] Nineteen commands open a menu; seven carry no icon (PivotTable, Recommended PivotTables, Insert Combo
  Chart, PivotChart, Win/Loss, Object, Symbol).
- [ ] Tables and Charts stand last.

*Guesses here:* Cell Controls' position.

### Draw · `Ribbons/Excel → Draw`
Eight groups. 14 commands.

- [ ] **No Stencils group** — Excel's census declares none. There is no Ruler, and the strip goes from Write to
  Input Mode.
- [ ] **Eraser is a plain toggle, not a split button.** Press it: it draws pressed, releasing the tool that held;
  **nothing opens.** Compare `Ribbons/PowerPoint → Draw`, where the same command has an arrow.
- [ ] Five commands open something: Add Pen, Pens, Colour, Thickness, Touch/Mouse Mode.
- [ ] Select Objects is the only survivor.

*Guesses here:* the group order, as on Word's and PowerPoint's Draw.

### Page Layout · `Ribbons/Excel → Page Layout`
Five groups — Themes, Page Setup, Scale to Fit, Sheet Options, Arrange. 24 commands.

- [ ] Scale to Fit: Width and Height are dropdowns (Automatic, 1 page …); **Scale is a combo box** — pick 75%,
  then type 80%. *(A percentage is not a length, so it is not a measure input.)*
- [ ] Sheet Options is four checkboxes: **View Gridlines and View Headings ticked**, Print Gridlines and Print
  Headings not. *(Office draws them under *Gridlines* and *Headings* headings; here each carries the full name.)*
- [ ] Arrange is Word's Layout Arrange **without Position and Wrap Text**. **Bring Forward and Send Backward are
  large split buttons at its head**, as Excel draws them. Selection Pane is a toggle with no icon.
- [ ] Press Themes, Colours, Fonts and Effects: **the same menus as Word's Design tab** — compare them. Themes
  carries no icon, so it is small.
- [ ] In Page Setup only Margins and Orientation are large; Size, Print Area, Background and Print Titles have no
  glyph, and Breaks sits in their column.
- [ ] **Three dialog launchers** — Page Setup, Scale to Fit, Sheet Options — all opening Page Setup. Nothing
  survives; Page Setup stands last.

*Guesses here:* the checkbox labelling where Office uses headings.

### Formulas · `Ribbons/Excel → Formulas`
Four groups — Function Library, Named Cells, Formula Auditing, Calculation. 24 commands.

- [ ] Judge the category glyphs at 20 px: Recently Used, Financial, Logical, Text, Date & Time, Lookup &
  Reference and Math & Trig each draw **a book with their mark on it**; More Functions the plain book.
- [ ] Date & Time, Lookup & Reference and Math & Trig are small — three tokens do not fit a large button.
- [ ] Press AutoSum's arrow: Sum, Average, Count Numbers, Max, Min.
- [ ] Press each category: **ten of its functions**, then *Insert Function… Shift+F3*. Press More Functions:
  Office's six further categories. Insert Function is the plain *fx* button.
- [ ] **The second group is labelled Named Cells**; Office calls it Defined Names. Define Name is a split button.
- [ ] Press Use in Formula: the names the name box shows (Revenue, CostOfSales, Headcount, Q1, PrintArea), then
  Paste Names.
- [ ] Press Remove Arrows' and Error Checking's arrows: their menus open. Press Show Formulas: it draws pressed.
- [ ] Watch Window carries no icon, so it is small where Office draws it large.
- [ ] Press Calculation Options: **Automatic checked** and the two other modes. No launchers; nothing survives;
  Function Library stands last.

*Guesses here:* the group label *Named Cells*. Office's Python groups are out of scope.

### Data · `Ribbons/Excel → Data`
Nine groups — Get External Data, Queries & Connections, Workbook Links, Connections, Data Types, Sort & Filter,
Data Tools, Forecast, Outline. 33 commands.

- [ ] ⚠ **There is no Get Data.** Get & Transform Data (Power Query) is out of scope; the in-scope **Get External
  Data** is Office 2016's legacy group: From Access, From Web, From Text, From Other Sources (a dropdown of the
  legacy wizards), Existing Connections.
- [ ] ⚠ **Three groups are one Office group in three generations**, each command drawn once: Queries &
  Connections (Refresh All as a large split button, the Queries & Connections toggle, Properties); Workbook Links
  (its toggle); Connections (Connections, Edit Links). Judge whether the three reading as three groups is
  acceptable.
- [ ] Collapse Sort & Filter: **Sort A to Z and Sort Z to A** stay. **Filter is a large toggle** — press it and
  it draws pressed; it does **not** survive, because its funnel is also Insert's Slicer.
- [ ] ⚠ **Data Types is an in-ribbon gallery** of Stocks, Currencies and Geography, each drawn with an icon
  (`building-bank`, `money`, `map`), nothing selected. **A blank cell is the finding** — check an icon renders
  inside a gallery cell at all.
- [ ] Data Validation, Group and Ungroup are split buttons; What-If Analysis is a dropdown (Scenario Manager,
  Goal Seek, Data Table).
- [ ] Outline has the tab's one dialog launcher. Text to Columns, Remove Duplicates, Consolidate, Manage Data
  Model, Ungroup and Subtotal carry no icon.

*Guesses here:* the three-generations reading; that an icon renders inside a gallery cell.

### Review · `Ribbons/Excel → Review`
Eleven groups — Proofing, Performance, Accessibility, Language, Threaded Comments, Comments, Notes, Protect,
Changes, Ink, Debug. 23 commands.

- [ ] ⚠ **Debug is one plain button labelled *Debug*, guessed entirely.** The census names the group, counts one
  control, names none. It opens nothing.
- [ ] ⚠ **Threaded Comments, Comments and Notes are one Office group in three generations.** Threaded Comments
  is Microsoft 365's five; **Comments is Office 2016's, drawn as two toggles on its face** — Show/Hide Comment
  and Show All Comments; press either and it draws pressed. **Notes is one large dropdown**: New Note
  (Shift+F2), Previous Note, Next Note, Show/Hide Note and Show All Notes (**both unticked**), then Convert to
  Comments.
- [ ] ⚠ **Changes is Office 2016's legacy group**: Share Workbook, Protect and Share Workbook (labels alone), and
  Track Changes (a small dropdown of Highlight Changes… and Accept/Reject Changes). *(Microsoft 365 hides all
  three unless the ribbon is customised.)*
- [ ] ⚠ **Group order**: Performance is **second**, after Proofing, and Ink is **after Changes** — the census
  declares Performance tenth and Ink seventh.
- [ ] Protect: Protect Sheet (large, a grid with a padlock), Protect Workbook (a large toggle, pressed once
  pressed), then Allow Edit Ranges and Unshare Workbook, both labels alone. *(Office greys Unshare Workbook;
  available here.)*
- [ ] Collapse Threaded Comments: **Previous Comment and Next Comment** stay. Show Comments is a large plain
  toggle with no arrow.
- [ ] Press Check Accessibility's arrow (Check Accessibility, Alt Text, Options: Accessibility) and Hide Ink's
  (Hide Ink, Delete All Ink on Sheet — its face is a toggle).
- [ ] Check Performance is a small speedometer. Spelling and Translate are large plain buttons. No launchers.

*Guesses here:* the whole Debug group; the three-generations reading; the legacy Changes group; both group
positions. Insights and Lineage are out of scope.

### View · `Ribbons/Excel → View`
Seven groups — Sheet View, Workbook Views, Show, Zoom, Window, Night Mode, Debug. 28 commands.

- [ ] ⚠ **Page Break Preview is large with a three-word label** — it must wrap to *Page Break* over *Preview*
  without an ellipsis, beside Normal (pressed, a grid) and Page Layout (a printed page).
- [ ] ⚠ **Night Mode and Debug are guessed end to end.** Night Mode is Word's reading — one large Switch Modes
  toggle; Debug is one small labelled button with no icon, as on Review. Both drawn last.
- [ ] ⚠ **Sheet View is drawn first**, where Microsoft 365 draws it; the census declares it fifth. Its dropdown
  reads *Default* and lists Default alone. Keep, Exit, New and Options are small buttons (a disk, an exit arrow,
  a plus, a cog), **all available though Office greys most of them here**.
- [ ] ⚠ Collapse Zoom: **100% and Zoom to Selection** stay beside the trigger; Zoom opens from the popup.
- [ ] **Workbook Views is one exclusive set.** Press Page Layout: Normal releases. Press it again: it stays.
  Custom Views is a small button, **not in the set**.
- [ ] Press Freeze Panes: Freeze Panes, Freeze Top Row, Freeze First Column, **each with its glyph and a one-line
  description**. Press Switch Windows: one window, *1 Findings*, checked.
- [ ] Split, View Side by Side and Synchronous Scrolling fill while pressed. Hide (eye struck through), Unhide
  (eye) and Reset Window Position (two columns) are plain buttons.
- [ ] Show is **four checkboxes, all ticked**: Ruler, Gridlines, Formula Bar, Headings. No launchers.

*Guesses here:* that Page Break Preview's label fits; both last groups entirely; Sheet View's position; that
*1:1* and the magnifier-in-fit-corners read with no label.

### Print Preview · `Ribbons/Excel → Print Preview` *(view tab)*
Three groups — Print, Preview, Zoom. 7 commands.

- [ ] ⚠ **The groups read Print, Zoom, Preview** — Office's order; the census declares Zoom last. Drag narrow and
  **Zoom still collapses first**, because it is `ancillary`. Judge whether drawn order and collapse order
  disagreeing is acceptable.
- [ ] ⚠ **Page Setup draws a cog**, large beside Print, where Word's and PowerPoint's tabs draw Options. Does it
  read as the page's settings?
- [ ] ⚠ **Show Margins is an unticked checkbox, not a toggle button**, under Next Page and Previous Page, in one
  column. Tick it: it ticks and nothing else changes.
- [ ] ⚠ **Zoom is a large magnifier alone in its group, a plain button**: pressing it does **not** stay pressed
  and opens nothing.
- [ ] Collapse Preview: **Next Page and Previous Page** stay; Show Margins and Close Print Preview open from the
  popup. Collapse Zoom: its trigger stands alone.
- [ ] Close Print Preview is large with a three-word label — wrap without an ellipsis.
- [ ] **No dialog launcher on any group, and no button opens a menu.**
- [ ] Open `Shell/Excel`: no Print Preview tab.

*Guesses here:* the group order; Page Setup's cog; Show Margins' shape and start (the brief listed a toggle);
that Office does not draw Zoom pressed while magnified.

### Background Removal · `Ribbons/Excel → Background Removal` *(view tab)*
Two groups — Refine, Close. 4 commands. **Word's tab under Excel's ids.**

- [ ] ⚠ Neither pencil starts pressed; press Keep, then Remove (Keep releases), then Remove again (neither holds).
- [ ] **Inspect a pencil**: its `exclusive` attribute is `excel.background-removal.refine`.
- [ ] ⚠ Refine has two commands, not three. ⚠ Judge the four circles.
- [ ] All four are large; the long labels must wrap to two lines without an ellipsis.
- [ ] Collapse both: no survivors; the set still holds at most one inside the popup.
- [ ] Open `Shell/Excel`: no Background Removal tab.

*Guesses here:* as Word's. *(View's Workbook Views set, by contrast, keeps one pressed.)*

### Table Design · `Ribbons/Excel → Table Design` *(contextual — Table Tools)*
Five groups — Properties, Tools, External Table Data, Table Style Options, Table Styles. 19 commands.
⚠ **Excel's Table Tools is its own census set, with no Layout tab** — a worksheet table's rows and columns are
the sheet's.

- [ ] ⚠ **Table Name is a combo box**, where Office draws a plain text box. It reads *Table1*; type a new name
  and press Enter — it keeps the text. **Its arrow opens a list of one, Table1.** Judge whether that is
  acceptable; it is the weakest part of the tab's shape.
- [ ] ⚠ Expand Table Styles: *Light* (22), *Medium* (28), *Dark* (11), one family to a row of seven, **Table
  Style Medium 2 selected**. **Light opens with None**, so its rows start one cell later than Medium's — the
  brief listed 60 and this draws 61.
- [ ] ⚠ Check Dark ends with Dark 8 to 11, the three accent pairs drawn in their first accent. Footer: New Table
  Style… and Clear.
- [ ] ⚠ Hover a cell: the name reads *Table Style Light 9* — **the name the file carries**, without the colour
  word Microsoft 365 adds.
- [ ] ⚠ Judge the new glyphs: Resize Table's table in corner marks, **Summarize with PivotTable's turned blocks
  (`pivot`, the weakest — Insert's PivotTable carries no glyph)**, Convert to Range's table turning to lines,
  Open in Browser's globe with an arrow. Reused: Insert Slicer's funnel, Export's arrow leaving a box, Refresh
  Data's refresh arrow, Properties' table with a cog, Unlink's struck link. **Remove Duplicates has none.**
- [ ] **Seven checkboxes in three columns**: Header Row, Total Row, Banded Rows; First Column, Last Column,
  Banded Columns; **Filter Button alone**. **Header Row, Banded Rows and Filter Button ticked.**
- [ ] Press Export (a large dropdown): Export Table to SharePoint List…, Export Table to Visio Pivot Diagram….
- [ ] Press Refresh's arrow (a large split button): Refresh (Alt+F5), Refresh All (Ctrl+Alt+F5), Refresh Status,
  Cancel Refresh, then Connection Properties…. Its face does nothing.
- [ ] **Properties, Open in Browser and Unlink are available**, small in a column — Office greys them, and
  Refresh, for a table with no external source.
- [ ] Tools: Summarize with PivotTable, Remove Duplicates and Convert to Range small in a column, then Insert
  Slicer large. Properties: Table Name over Resize Table.
- [ ] Drag narrow: External Table Data first, then Properties and Tools, **Table Style Options and Table Styles
  last**. **No launcher and no survivor anywhere.**
- [ ] **Also in `Shell/Excel`**, which draws Table Tools: select Table Design there and check every list, menu
  and starting state is the same.

*Guesses here:* the combo box; None's place and every gallery picture; every new glyph; the checkbox columns and
start.

### Picture Format · `Ribbons/Excel → Picture Format` *(contextual — Picture Tools)*
Six groups — Adjust, Picture Styles, Accessibility, Arrange, Size, Image Play. 23 commands.
**Word's tab through Word's functions** — see `Ribbons/Word → Picture Format` for every list.

- [ ] ⚠ Image Play: Play Animation, a large toggle, **starting pressed**.
- [ ] ⚠ Open **Picture Border**: More Outline Colours…, Weight ▸, Sketched ▸, Dashes ▸ — **no Eyedropper**,
  exactly Word's. *(PowerPoint's has one.)* *No Outline* is the chip, starting on none.
- [ ] ⚠ **Arrange has six commands, and Bring Forward and Send Backward are large at the head**, as on Page
  Layout, where Word and PowerPoint draw them small. Their arrows list **without Word's text layers**.
- [ ] ⚠ Press Align: the six alignments, two distributions, then **Snap to Grid, Snap to Shape and View
  Gridlines — the last ticked**. *(Word's ends on Align to Margin, PowerPoint's on Align to Slide.)*
- [ ] ⚠ Height and Width start on **9.53 cm and 12.7 cm**, stepping by 0.01. They do not follow each other.
- [ ] Two launchers: *Format Picture*, and **Size and Properties** at Size's (Word's says Layout, PowerPoint's
  Size and Position).
- [ ] **Picture Layout, as Word says** — third in Picture Styles' column, with the diagram glyph, opening the
  thirty-one layouts. *(PowerPoint calls the same button Convert to SmartArt.)*
- [ ] Press Corrections, Colour, Artistic Effects and Transparency: each is Word's whole list with the unchanged
  state checked.
- [ ] Expand Quick Styles: Word's twenty-eight in this workbook's palette. Nothing selected.
- [ ] Crop's face fills then releases; its arrow opens Crop, Crop to Shape (147), Aspect Ratio, Fill, Fit.
- [ ] Picture Effects opens the seven submenus. Alt Text is a large toggle, unpressed.
- [ ] **Remove Background's glyph is still the weakest** on this tab too.
- [ ] Drag narrow: Image Play first, Accessibility next, then Arrange and Size, Adjust and Picture Styles last.
  No survivor.
- [ ] Open `Shell/Excel`: **no Picture Tools band** (the shell draws Table Tools).

*Guesses here:* the whole of Image Play; that Excel's Picture Border has no Eyedropper; both measures; both
launcher labels; every gallery look.

### Shape Format · `Ribbons/Excel → Shape Format` *(contextual — Drawing Tools)*
Six groups — Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size. 20 commands.
*(Theme Styles, Other Theme Fills, the Shapes list and Edit Shape are covered on
`Ribbons/PowerPoint → Shape Format`.)*

- [ ] ⚠ **Text Fill and Text Outline are PowerPoint's less the Eyedropper**, not Word's shorter pair. Text Fill
  starts on Background 1 with *No Fill*, More Fill Colours…, **Picture…, Gradient ▸, Texture ▸**; Text Outline on
  none with *No Outline*, More Outline Colours…, Weight ▸, **Sketched ▸**, Dashes ▸.
- [ ] ⚠ **No Eyedropper anywhere on the tab.** Shape Fill starts on Accent 1 (More Fill Colours…, Picture…,
  Gradient ▸, Texture ▸); Shape Outline on Accent 1, Darker 50% (More Outline Colours…, Weight ▸, Sketched ▸,
  Dashes ▸, Arrows ▸).
- [ ] ⚠ **Arrange is Picture Format's six, not Word's eight**: Bring Forward and Send Backward **large** at the
  head; Selection Pane small with no glyph; **Align ending on Snap to Grid, Snap to Shape and View Gridlines,
  the last ticked**; Group and Rotate. No Position or Wrap Text.
- [ ] **Text Box is a small split button** beside Edit Shape. Press its face: nothing opens. Press its arrow:
  *Draw Horizontal Text Box*, *Vertical Text Box* — the two Excel's Insert tab offers. **No Merge Shapes.**
- [ ] Press Shapes: the whole gallery with **no Action Buttons and no New Drawing Canvas**. Edit Shape's Change
  Shape has no Action Buttons either, and **Reroute Connectors is unavailable**.
- [ ] Height and Width start on 2.54 cm, stepping by 0.01; they do not follow each other.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, **Size and Properties**.
- [ ] Shape Effects opens Picture Effects' seven submenus; Text Effects WordArt's six.
- [ ] **Alt Text is a large toggle, unpressed — its glyph is the weakest on the tab.**
- [ ] Eleven glyphs, every one reused. The two galleries, four pickers, Height, Width and Selection Pane carry
  none.
- [ ] Drag narrow: Accessibility first, then Insert Shapes, WordArt Styles, Arrange and Size, Shape Styles last.
  No survivor.
- [ ] Open `Shell/Excel`: no Drawing Tools band.

*Guesses here:* the whole Text Fill / Text Outline reading, **Sketched most**; both starts, and that recent
builds have not brought the Eyedropper to Excel; that Excel still lacks Merge Shapes; both measures; all three
launchers.

### Chart Design · `Ribbons/Excel → Chart Design` *(contextual — Chart Tools)*
Five groups — Chart Layouts, Chart Styles, Data, Type, **Location**. 8 commands, every one large.

- [ ] **Judge it beside `Ribbons/Word → Chart Design`**: Chart Layouts, Chart Styles and Type must match Word's
  and PowerPoint's exactly. **Data and Location are the only places Excel's may differ.**
- [ ] ⚠ **Move Chart's glyph is the weakest on the tab**: four arrows, which say *move* and not *chart* or
  *sheet*. Office draws a chart with an arrow leaving it. Pressing it opens nothing. It is **Location**, Excel's
  own group, holding nothing else.
- [ ] ⚠ **Data is two commands, not four**: Switch Row/Column and Select Data — **no Edit Data or Refresh Data**,
  because a workbook's chart reads its own cells. **There is no split button on this tab.** Judge the two glyphs
  together.
- [ ] ⚠ Press Change Chart Type: eight submenus, **exactly the lists this ribbon's own Insert tab opens** for
  each family, each ending on *More … Charts…*. **Compare them with `Insert`. No Map family**, though Excel's
  dialog lists one.
- [ ] ⚠ Press Add Chart Element: **Word's starts**, read as the Clustered Column Excel inserts. Lines and Up/Down
  Bars open, where Office greys both.
- [ ] ⚠ Expand Chart Styles: *Style 1* to *Style 16* on Style 1, in the catalogue's one specimen theme, **so the
  pictures match Word's and PowerPoint's exactly.**
- [ ] Quick Layout: four tiled regions; its menu is *Layout 1* to *Layout 11*.
- [ ] Press Change Colours: *Colourful* (1–4) and *Monochromatic* (1–13), one set, on *Colourful Palette 1*.
- [ ] ⚠ **The census's priorities differ from Word's, and the collapse shows it.** Drag narrow: **Location**
  first, then Data and Type (where Word's Data is `standard`), and **Chart Layouts and Chart Styles last** (where
  Word's Chart Styles is `secondary`). Judge whether that difference is right.
- [ ] Spelling: *Change Colours*, *Colourful*, *Centred Overlay*, *Centre*. No launcher; no survivor.
- [ ] Open `Shell/Excel`: no Chart Tools band.

*Guesses here:* Move Chart's glyph; the two-command Data group; Change Chart Type as a menu and the missing Map
family; that Excel's inserted chart starts as Word's; sixteen styles and every look; both Change Colours counts.

### Chart Format · `Ribbons/Excel → Chart Format` *(contextual — Chart Tools)*
Seven groups — Current Selection, Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size.
22 commands. **`Ribbons/Word`'s Chart Format less Position and Wrap Text — judge the two side by side; every
difference other than those below is a defect.**

- [ ] ⚠ **Height 7.62 cm and Width 12.7 cm**, stepping by 0.01 — neither Word's 15.24 × 8.89 nor PowerPoint's
  22.58 × 15.05. **This is the weakest call on the tab**: a chart dropped onto a sheet is the easiest thing in
  Office to have been resized before anybody looked, so both numbers are a default rather than an observation.
- [ ] ⚠ **No Eyedropper under any of the four pickers**, where PowerPoint's Chart Format carries one under each.
  Shape Fill: More Fill Colours…, Picture…, Gradient ▸, Texture ▸. Shape Outline: More Outline Colours…,
  Weight ▸, Dashes ▸ — **still no Sketched or Arrows**, which Excel's *Shape* Format's outline has. Text Fill:
  Shape Fill's four. Text Outline: More Outline Colours…, Weight ▸, **Sketched ▸**, Dashes ▸.
- [ ] ⚠ **The pickers start where Word's do, not where PowerPoint's do**: Shape Fill on *Background 1*, Shape
  Outline on *Text 1, Lighter 80%* — a chart Excel inserts is opaque. Text Fill on *Text 1, Lighter 40%*, Text
  Outline on none.
- [ ] ⚠ Chart Elements starts on *Chart Area* and lists Word's ten parts alphabetically, **the three series
  before the vertical axis**. Check the field fits *Vertical (Value) Axis Major Gridlines*.
- [ ] ⚠ Judge **Reset to Match Style's glyph** beside Format Selection's column chart with a pencil; both plain
  small buttons, drawn available.
- [ ] Arrange is Excel's six: Bring Forward and Send Backward **large** at the head; Selection Pane small with no
  glyph; **Align ending on Snap to Grid, Snap to Shape and View Gridlines, the last ticked**; Group and Rotate.
  No Position and no Wrap Text.
- [ ] Press Shapes: the gallery with **no Action Buttons and no New Drawing Canvas**. Change Shape is small,
  drawn available, wearing Edit Shape's glyph. No Edit Points, Text Box or Merge Shapes.
- [ ] Three launchers: *Format Shape*, *Format Text Effects*, **Size and Properties**. None on Current Selection.
- [ ] Text Effects keeps Transform; Alt Text is a large toggle, unpressed.
- [ ] Drag narrow: Insert Shapes first, then Accessibility, then Current Selection, WordArt Styles, Arrange and
  Size, Shape Styles last. No survivor.
- [ ] Open `Shell/Excel`: no Chart Tools band.

*Guesses here:* both measures (**the weakest call on the tab**); every picker entry; all four starts and the
rounding; the Chart Elements order and labels; all three launchers.

---

## 6 · Cross-cutting behaviours

These are not one tab's, and a defect in any of them is a defect in dozens. Check each once, on the tab named,
then spot-check elsewhere.

### Survivors when a group collapses

- [ ] **A survivor draws where it is declared, not first and not last.** `Ribbons/Word → Home`: Bold and Italic
  must draw **in the middle** of Font at every width, and beside the trigger once collapsed.
- [ ] **A survivor keeps its declared size.** `Ribbons/Word → Table Layout`: Insert Above is large beside the
  trigger.
- [ ] **Collapse, then widen back to desktop**: every command returns in order, the popup closes and its focus
  trap is gone.
- [ ] **A collapsed group's survivor and its popup are one exclusive set.** `Ribbons/Word → Draw`: collapse
  Write, press a tool inside the popup, and Select Objects beside the trigger releases.
- [ ] **A group with a survivor never leaves its popup empty.** Excel Insert's Checkbox is the case that was
  refused on that ground.

### Exclusive toggle sets

- [ ] **Pressing the member that holds keeps it** — everywhere except Background Removal.
- [ ] **Background Removal's two pencils may hold none**, in all three applications: press the one that holds and
  it releases.
- [ ] **Two sets on one tab do not touch each other.** `Ribbons/Word → View` (views and page movement),
  `Ribbons/PowerPoint → View` (three sets), `Ribbons/PowerPoint → Table Layout` (horizontal and vertical
  alignment).
- [ ] **A set is scoped to its tab.** Press a member on `Ribbons/PowerPoint → Greyscale`, then open
  `Black and White`: its set is untouched. Same for Slide Master Home's font size against Home's.
- [ ] **Keyboard**: Tab to a member and press **Space** — the set moves exactly as a click does.
- [ ] Members stay **toggle buttons with `aria-pressed`**, not radios: each is its own tab stop.

### Split buttons with a toggle face

- [ ] **Pressing the face moves the state; pressing the arrow opens the menu and the state does not move.**
  `Ribbons/Word → Review` (Track Changes, Show Comments, Hide Ink), `Ribbons/PowerPoint → Review`,
  `Ribbons/Excel → Review` (Hide Ink), `Ribbons/Word → Draw` (Eraser).
- [ ] **A split button whose face arms a gesture opens nothing from the face**: Word's Draw Text Box, Excel's
  Text Box, PowerPoint's Record.
- [ ] Compare `Ribbons/PowerPoint → Draw`'s split Eraser with `Ribbons/Excel → Draw`'s **plain toggle** Eraser —
  the same command, two shapes, from two census counts.

### Colour pickers

- [ ] **Arrow Down from the last row of recent colours moves onto the first entry beneath the palette; Arrow Up
  returns.** Check on `Ribbons/Word → Table Design` (Shading) and `Ribbons/PowerPoint → Table Design` (all four).
- [ ] **Arrow Right opens a submenu** (Gradient, Texture, Weight, Sketched, Dashes, Arrows); choosing anything
  closes the picker.
- [ ] **The chip and the entries are per application and per tab.** The Eyedropper is the fastest tell: present
  on PowerPoint's pickers, absent on Word's and Excel's — with the exception of PowerPoint's Picture Border.
- [ ] **A *none* chip is the one marked where it should be**: PowerPoint's Chart Format starts on *No Fill* and
  *No Outline*; Word's starts on white and light grey.

### Menus opening from inside a popup or gallery

- [ ] `Ribbons/PowerPoint → Design`: open the **Variants flyout** and press Colours, Fonts, Effects and
  Background Styles at its foot. The menu must appear beside the button and the flyout must behave.
- [ ] `Ribbons/PowerPoint → Shape Format`: expand Theme Styles and press **Other Theme Fills** in the footer.
  The gallery's popup must not swallow it.
- [ ] `Ribbons/PowerPoint → Table Design`: **hover an entry with a submenu** inside Effects and Text Effects —
  three levels deep (menu ▸ submenu ▸ entry).
- [ ] **Any collapsed group**: a gallery inside a collapsed popup still expands (PowerPoint Home's Drawing is the
  case that was refused a survivor on this ground).
- [ ] A menu opened from inside a popup closes with **Escape**, returning focus to the button that opened it.

### The phone presentation

- [ ] At the **phone** preset (390 px) **the tab strip becomes a picker**: one button naming the selected tab,
  and the list a popup. Check on all three applications.
- [ ] In the picker, **Arrow Down / Arrow Up** move and select; Home and End reach the first and last tab.
- [ ] Select a tab from the picker: **every group arrives collapsed with exactly its declared survivors.**
- [ ] A **contextual band** at phone width still says which set the tab belongs to.
- [ ] Nothing overflows horizontally, and no large command's label clips. **The long-label tabs are the ones to
  look at**: Background Removal (all three), Outlining, both Print Previews, Black and White.

### Keyboard and focus order

- [ ] **Tab** enters the tab strip at the selected tab and leaves it on the next press — **exactly one tab is a
  tab stop**.
- [ ] **Arrow Right / Arrow Left** move along the tabs **and select as they go** (automatic activation).
- [ ] **Home / End** reach the first and last tab.
- [ ] **Enter or Space on a collapsed group** opens its popup and **focus does not move**.
- [ ] **Arrow Down on a collapsed group** opens it **and puts focus on the first command**.
- [ ] **Tab inside an open group cycles within it** — focus cannot leave an open popup.
- [ ] **Escape** closes the popup and returns focus to the trigger (or the tab picker).
- [ ] An icon-only command is still reachable and named: its label is the accessible name, drawn off-screen and
  never dropped. Check on `Ribbons/Word → Home`'s Paragraph (fourteen glyphs) and Table Layout's nine alignments.
- [ ] With a screen reader: the strip announces *"Ribbon, tab list"*, a tab as *"Home, tab, selected, 1 of 10"*,
  **a contextual tab with its set in its own name** (*"Design, Table Tools, tab"*), a collapsed group as *"Font,
  collapsed, button"* and its popup as *"Font, group"*. A set appearing announces politely; closing announces too.

### The states an auditor must be able to see

- [ ] expanded · collapsed to tabs · hidden · **simplified** (the single-row form, every group at most reduced)
- [ ] group · full · reduced · collapsed · collapsed and open
- [ ] tab strip · strip · picker
- [ ] tab · selected (accent tint and a bold label) · contextual (a titled honey band)

---

## 7 · What this pass does not cover, and where findings go

**Knowingly not covered here:**

- **MJXOFF-339 — the browser tier is red.** *"The `ui` branch's browser tier is red: 18 Playwright tests fail at
  e72dd5e, most likely since the hand-audit pass (8d96c23)."* Those failures are being tracked separately; a
  visual problem you find here is **not** assumed to be one of them, and is worth recording even if it looks
  related.
- **MJXOFF-346 — the ribbon visual design pass, after every tab is authored.** This checklist is about
  *content and behaviour* — is this the right list, in the right order, with the right glyph and the right
  starting state. **Spacing, weight, density and the overall look are that ticket's**, not this pass's. If
  something is merely ugly rather than wrong, note it against 346.
- **MJXOFF-341 — `<mjx-ribbon-group>` needs imperative slot assignment, and nothing detects or tests an engine
  without it.** So **the survivor and collapse behaviour above is only checked in this browser.** Do not judge
  it in Safari/WebKit and report a defect; that gap is the ticket.
- **The narrow-width presentation is judged with a mouse.** Nothing here has been used with a thumb. The canvas
  harness has a phone path (`serve --host 0.0.0.0`); the ribbon catalogue does not.
- **Nothing dispatches, no dialog is wired, and many commands Office greys are drawn available.** All three are
  deliberate and named per tab above.
- **The visual baselines were generated by the code under test**, and no person has approved them. They lock the
  current appearance against accidental change; they do not assert it is right. This pass is what makes them
  mean something.

**Where to record findings.**

Work on this project is tracked in the self-hosted Plane project **MJX Office (`MJXOFF`)**. For each finding:

1. **Tick the box** in this file only when the item is *right*. Leave it unticked when it is wrong — the
   unticked boxes are the worklist.
2. **Write the correction beside the item**, in one line, naming what Office actually does. *"Excel's Picture
   Border does have an Eyedropper"* is a complete finding; *"wrong"* is not.
3. **A correction to content or a starting state** — a list's entries, an order, a tick, a measure, a label —
   goes to the tab's own ticket as a comment, or to a new `MJXOFF` item naming the story path and the command.
4. **A correction to a glyph** goes in one place per glyph, not per tab: `arrow-reset`, `pivot` and
   `padding-left` each appear on several tabs and each wants **one** decision.
5. **Anything merely ugly** goes to **MJXOFF-346**, not here.
6. **Anything that is a component defect rather than a ribbon-data defect** — a popup that mis-anchors, focus
  escaping a trap, a label clipping where it should wrap — is its own `MJXOFF` item against the component, with
  the story path and the container preset it reproduces at.

**When a correction lands**, the fix is usually **two** files: the declaration in `ui/dev/ribbons/census.ts`
(and its `GUESS:` comment removed, because it is no longer a guess) and the menu or picture in
`ui/stories/ribbons/`. The story's *What to look at* block is the third, and it is what this checklist is
regenerated from.
