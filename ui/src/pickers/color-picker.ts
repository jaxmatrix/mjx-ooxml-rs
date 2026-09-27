/**
 * `<mjx-color-picker>` — Office's *Font Colour*, *Shape Fill*, *Highlight* and *Border Colour*,
 * and the one control in this catalogue whose value is not a token.
 *
 * ```html
 * <mjx-color-picker label="Font colour" value="theme:accent1/lighter40" show-automatic>
 * </mjx-color-picker>
 * ```
 *
 * with the palettes handed in as **data**, because they belong to the document:
 *
 * ```ts
 * picker.themePalette = { accent1: '#2f6f4e', … };   // the document's <a:clrScheme>
 * picker.standardColors = ['#c00000', '#ff0000', … ];
 * picker.recentColors = ['#123457'];
 * ```
 *
 * ## Its job is to return the colour a person chose — including one that is not on the grid
 *
 * The field is a **text box** as well as a grid, so `#123457`, `rgb(18, 52, 87)`,
 * `hsl(210, 66%, 21%)` and `theme:accent1/lighter40` are all things a person may type, and every
 * one of them commits the same canonical value. That is `resolveTyped`, and it is why this
 * subclasses `<mjx-combo-box>` rather than `<mjx-dropdown>`: a picker whose only input was its own
 * grid could not return a colour that is not on its own grid, which is the requirement.
 *
 * `tests/pickers.test.ts` round-trips every representation, and
 * `tests/browser/pickers.spec.ts` types one that is on no swatch and asserts the value out equals
 * the value in.
 *
 * ## A theme choice reports as a slot, and never as a colour
 *
 * The standing rule of this project is that **the user's document wins over our defaults**. A
 * theme swatch commits `theme:accent1/lighter40`; the literal it currently paints as is *computed
 * from the palette the host supplied*, is used to fill one square, and is never stored, never
 * emitted and never round-tripped. Open the same document in a differently branded deck and the
 * run follows the new brand, which is what a theme colour is for.
 *
 * The gate for it is structural rather than pictorial: a parsed theme choice is asserted to carry
 * no `hex` property at all, so a future convenience that resolved it would fail rather than look
 * fine.
 *
 * ## What the two rings are, and why there are two
 *
 * The **cursor** ring is drawn outside the square, on the popup's own surface — a token against a
 * token, decided at build time. The **selection** ring is drawn inside the square, on the colour a
 * person chose, so it is decided at paint time by `chooseSwatchIndicatorAmong` against whichever
 * of `--theme-text-primary` and `--theme-surface` actually reads on that colour. One ring doing
 * both jobs would have had to be a fixed colour and would have been invisible on the swatch of
 * that colour — a defect whose visibility depends entirely on which colours happen to be in the
 * palette on the day somebody looks.
 *
 * ## The entries beneath the palette
 *
 * Office's colour grids carry **commands** below the swatches: *More Colours…* and *Eyedropper* on
 * nearly all of them, *Picture…*, *Gradient ▸* and *Texture ▸* under a fill, *Weight ▸*, *Sketched
 * ▸*, *Dashes ▸* and *Arrows ▸* under an outline. A host writes them as one real menu, slotted:
 *
 * ```html
 * <mjx-color-picker label="Shape Outline" show-no-fill no-fill-label="No Outline">
 *   <mjx-menu slot="entries" label="Shape Outline">
 *     <mjx-menu-item label="More Outline Colours…"></mjx-menu-item>
 *     <mjx-menu-item label="Weight">
 *       <mjx-menu slot="submenu" label="Weight">
 *         <mjx-menu-item kind="radio" label="1 pt" value="1" checked></mjx-menu-item>
 *       </mjx-menu>
 *     </mjx-menu-item>
 *   </mjx-menu>
 * </mjx-color-picker>
 * ```
 *
 * The menu is drawn beneath the palette, in the same popup, flush on it (the picker sets
 * `embedded` on it). Choosing any entry fires the menu's own `mjx-menu-activate`, which a host
 * listens for as it would on any menu, and closes the picker with focus back on the field.
 *
 * **The keyboard crosses between the two.** The swatches are a listbox with focus on the field and
 * `aria-activedescendant` naming the swatch; the entries are a menu with a roving tab stop and real
 * focus on the row. Two focus models in one popup, joined at exactly one seam, both halves of which
 * are pure functions in `picker-model.ts` and tested in Node:
 *
 * - `ArrowDown` on the palette's last drawn row moves focus onto the first entry, and the grid
 *   drops its cursor (`paletteMovesIntoEntries`);
 * - `ArrowUp` on the first entry moves focus back to the field, with the cursor on the swatch it
 *   left from (`entriesReturnToPalette`, `paletteReturnIndex`);
 * - `Escape` on an entry closes the picker, `Tab` leaves it, and inside a submenu both are the
 *   submenu's own. Pointing at a swatch while focus is on an entry brings focus back to the field,
 *   so the arrow keys always act on what the pointer last touched.
 *
 * Still one tab stop: a closed popup is not drawn, so no entry is reachable until it opens.
 *
 * ### Rejected, and why
 *
 * 1. **Loose `<mjx-menu-item slot="entries">` children, wrapped in a menu inside the shadow root.**
 *    A menu finds its rows by walking its own light-DOM children, and a slot is not one; a shadow
 *    menu over slotted rows would need its own item walk, key map, submenu handling and activation,
 *    which is a second menu. Wrapping them in a real `<mjx-menu>` costs the author one element.
 * 2. **Entries as swatch descriptors.** A command is not a colour: it has no value to commit, and a
 *    grid cell cannot hold a submenu, a check mark or an unavailable reason the way a menu row does.
 * 3. **A "More…" button that opens a separate floating menu.** Office draws the entries in the
 *    popup itself, under the grid, and a second popup is a second dismissal to get wrong.
 * 4. **Keeping `role="listbox"` on the popup and putting the menu inside it.** A menu is not an
 *    allowed child of a listbox. The popup now has no role; the listbox is the swatches inside it,
 *    and `aria-controls` names that (`PopupSurface.controlledElement`).
 * 5. **A data property of entry descriptors that the picker renders itself.** It would duplicate the
 *    menu's authoring API (kinds, sections, separators, submenus) in a second, weaker shape.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { themeVariable } from '../controls/control-states.ts';
import type { ThemeMember } from '../tokens/resolver.ts';
import { MjxComboBox } from '../inputs/combo-box.ts';
import { defineListFieldDependencies } from '../inputs/list-field.ts';
import { defineIcon } from '../icons/icon.ts';
import { defineMenus } from '../menus/index.ts';
import type { MjxMenu } from '../menus/menu.ts';
import type { MjxMenuItem } from '../menus/menu-item.ts';
import { menuEvents, menuTags } from '../menus/menu-model.ts';
import {
  displayTextFor,
  type ListCloseReason,
  type ListboxAction,
  type OptionDescriptor,
} from '../inputs/input-model.ts';
import { TokenResolver } from '../tokens/resolver.ts';
import type { PopupSurface } from '../inputs/list-surface.ts';
import {
  chooseSwatchHairlineAmong,
  chooseSwatchIndicatorAmong,
  colorPickerCss,
  colorPickerEntriesSlot,
  describeColorChoice,
  entriesReturnToPalette,
  formatColorChoice,
  galleryThemeSlots,
  nextSwatchIndex,
  noColorChosen,
  paletteMovesIntoEntries,
  paletteReturnIndex,
  parseColorChoice,
  resolveThemeColor,
  swatchCellBackground,
  swatchHairlineProperty,
  swatchIndicatorProperty,
  swatchKeyAction,
  swatchPaintProperty,
  swatchStatesCss,
  themeColorSlots,
  themeColorVariantNames,
  type ColorChoice,
  type SwatchIndicatorMember,
  type ThemeColorPalette,
} from './picker-model.ts';
import { SwatchSurface, type SwatchDescriptor } from './swatch-surface.ts';

/** The whole sheet a colour picker adopts, on top of the list field's. */
export const colorPickerSheet = [colorPickerCss, swatchStatesCss('.swatch')].join('\n');

/** The names of the sections, in the order the popup draws them. */
export const colorSectionNames = {
  reset: '',
  theme: 'Theme colours',
  standard: 'Standard colours',
  recent: 'Recent colours',
} as const;

export class MjxColorPicker extends MjxComboBox {
  static override readonly observedAttributes: readonly string[] = [
    ...MjxComboBox.observedAttributes,
    'automatic',
    'show-automatic',
    'show-no-fill',
    'no-fill-label',
  ];

  #themePalette: ThemeColorPalette = {};
  #standardColors: readonly string[] = [];
  #recentColors: readonly string[] = [];
  #swatches: readonly SwatchDescriptor[] = [];
  #palette: SwatchSurface | undefined;
  #preview: HTMLElement | undefined;
  #resolver: TokenResolver | undefined;
  /** Whether the cursor is where a person's arrow keys put it. See `handleAction`. */
  #cursorMovedByKeyboard = false;
  /** The region beneath the swatches the entries are slotted into. Built once. */
  #entriesRegion: HTMLElement | undefined;
  #entriesSlot: HTMLSlotElement | undefined;
  /** The swatch the keyboard left the palette from, so `ArrowUp` off the first entry returns there. */
  #entriesLeftFrom: number | undefined;

  // ── what the host supplies, because it belongs to the document ─────────────

  /**
   * The document's own colour scheme, slot by slot.
   *
   * ⚠ **There is no default and there must not be one.** A picker handed no palette draws no theme
   * section at all, rather than falling back to this platform's accent — which would put our green
   * into a customer's branded deck and would look completely correct until somebody opened the
   * file somewhere else. *Author a default only where nothing exists* is the project's rule, and a
   * document always has a theme; if we have not been told it, we have not been told it.
   */
  get themePalette(): ThemeColorPalette {
    return this.#themePalette;
  }

  set themePalette(palette: ThemeColorPalette) {
    this.#themePalette = { ...palette };
    this.render();
  }

  /** The fixed row of standard colours. Supplied, not shipped — see the module note. */
  get standardColors(): readonly string[] {
    return this.#standardColors;
  }

  set standardColors(colors: readonly string[]) {
    this.#standardColors = [...colors];
    this.render();
  }

  /** The colours this person has used lately. */
  get recentColors(): readonly string[] {
    return this.#recentColors;
  }

  set recentColors(colors: readonly string[]) {
    this.#recentColors = [...colors];
    this.render();
  }

  /** What *Automatic* currently resolves to, for painting the chip. Absent draws the empty rule. */
  get automatic(): string | undefined {
    return this.getAttribute('automatic') ?? undefined;
  }

  /** Whether the popup offers *Automatic*. */
  get showsAutomatic(): boolean {
    return this.hasAttribute('show-automatic');
  }

  /** Whether the popup offers *No fill*. */
  get showsNoFill(): boolean {
    return this.hasAttribute('show-no-fill');
  }

  /**
   * What the *No fill* chip is called: `no-fill-label`, or *No fill* when a host sets none.
   *
   * Office names the one value three ways (*No Fill* under a fill, *No Outline* under an outline,
   * *No Colour* under Word's Shading), and the name is also what the field shows and what a person
   * may type. The value stays `none` whatever it is called.
   */
  get noFillLabel(): string {
    const declared = (this.getAttribute('no-fill-label') ?? '').trim();
    return declared === '' ? describeColorChoice({ kind: 'none' }) : declared;
  }

  /**
   * ⚠ **Always true, and it is not an attribute.**
   *
   * A colour picker whose value had to be on its own grid could not return `#123457`, and
   * returning the colour a person chose is the entire job. The base class's `allow-custom` opt-in
   * is for a combo box whose author may reasonably want the list to be exhaustive; here it never
   * is.
   */
  override get allowCustom(): boolean {
    return true;
  }

  // ── the choice ─────────────────────────────────────────────────────────────

  /** What the picker reports, parsed. `undefined` when nothing has been chosen. */
  get choice(): ColorChoice | undefined {
    return this.value === noColorChosen ? undefined : parseColorChoice(this.value);
  }

  set choice(next: ColorChoice | undefined) {
    this.value = next === undefined ? noColorChosen : formatColorChoice(next);
  }

  /** The swatches the popup is showing. Computed at every render from the host's palettes. */
  override get visibleOptions(): readonly OptionDescriptor[] {
    return this.#swatches;
  }

  /** The grid, so a gate can read the sections the arrows are computed over. */
  get paletteSurface(): SwatchSurface | undefined {
    return this.#palette;
  }

  /**
   * The menu slotted beneath the palette, or `undefined` when the host slotted none.
   *
   * The first `<mjx-menu slot="entries">` child. Matched by tag name rather than `instanceof`, for
   * the reason `<mjx-menu>`'s own `items` gives: a child may not have upgraded yet.
   */
  get entriesMenu(): MjxMenu | undefined {
    for (const child of this.children) {
      if (child.localName === menuTags.menu && child.getAttribute('slot') === colorPickerEntriesSlot) {
        return child as MjxMenu;
      }
    }
    return undefined;
  }

  /** The region the entries are drawn in, so a gate can measure where it is. */
  get entriesRegion(): HTMLElement | undefined {
    return this.#entriesRegion;
  }

  // ── lifecycle ──────────────────────────────────────────────────────────────

  override connectedCallback(): void {
    super.connectedCallback();
    const root = this.shadowRoot;
    // Adopted *after* the field's own sheet, which is the order `installControlStyles` exists to
    // state once: a component wears the foundations, then the archetype, then its own.
    if (root !== null) installControlStyles(root, colorPickerSheet);
    if (this.#resolver === undefined) this.#resolver = new TokenResolver(this);
    if (this.#entriesRegion === undefined) this.#buildEntries();
    this.render();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this.#resolver?.dispose();
    this.#resolver = undefined;
  }

  /**
   * ⚠ **The entry, plus one listener that clears the keyboard cursor while a person types.**
   *
   * This is the fix for a defect the browser gate found, and it was silent in exactly the way this
   * control cannot afford: type `#123457`, press Enter, and the picker committed
   * `theme:accent3/darker25` instead.
   *
   * The base class opens its list on input and puts the cursor on the first *filtered* option,
   * which is right for a combo box — what you typed and what the cursor is on are the same thing.
   * A colour palette is **not filtered by what you type**, because it is a grid whose geometry is
   * its meaning, so the cursor has nothing whatever to do with the typed text and `Enter` was
   * committing an unrelated swatch.
   *
   * With no cursor, the base class's own `commit` path already does the right thing —
   * `commitActive()` finds nothing and the typed text is resolved instead — so the rule this adds
   * is one sentence: **typing clears the cursor; moving the cursor is how you get it back.** Enter
   * then commits the swatch when a person put the cursor on one, and the text when they did not.
   * The listener is registered after the base's, so it runs second and wins.
   */
  protected override createEntry(): HTMLInputElement {
    const entry = super.createEntry();
    entry.addEventListener('input', () => {
      this.#cursorMovedByKeyboard = false;
      this.#palette?.setActive(-1);
    });
    return entry;
  }

  /**
   * ⚠ **Typing beats pointing, and this is the line that says so.**
   *
   * The browser gate found the defect: type `#123457`, press Enter, and the picker committed
   * `theme:accent1/lighter80` — a colour the person had never seen, let alone chosen. Two things
   * conspire. The base class opens its list on input and puts the cursor on the first option,
   * which is right for a combo box (typing *filters*, so the cursor and the text mean the same
   * thing) and wrong here (the palette is a grid whose geometry is its meaning and is not
   * filtered). And a palette that opens under a stationary mouse pointer receives a `pointerover`
   * from the browser, so the cursor lands on whatever the pointer happens to be resting on.
   *
   * So `Enter` commits the swatch **only when a person put the cursor there** — with the arrow
   * keys, or by moving the mouse onto it while not typing. Otherwise the cursor is cleared and the
   * base class's own path resolves the text, which is the thing the person actually typed. The
   * four cases each read as a sentence:
   *
   * | What happened | What Enter commits |
   * |---|---|
   * | typed a colour | the colour typed |
   * | typed, then arrowed onto a swatch | that swatch |
   * | did not type, hovered a swatch | that swatch |
   * | did not type, arrowed onto a swatch | that swatch |
   */
  protected override handleAction(action: ListboxAction, event: KeyboardEvent): 'handled' | 'passed' {
    if (action === 'commit' && this.typing && !this.#cursorMovedByKeyboard) {
      this.#palette?.setActive(-1);
    }
    return super.handleAction(action, event);
  }

  protected override closed(reason: ListCloseReason): void {
    super.closed(reason);
    this.#cursorMovedByKeyboard = false;
    this.#entriesLeftFrom = undefined;
    this.#closeEntrySubmenus();
  }

  protected override createSurface(idPrefix: string): PopupSurface {
    const palette = new SwatchSurface(this, idPrefix);
    this.#palette = palette;
    return palette;
  }

  /**
   * A swatch the pointer moved onto, or the keyboard did.
   *
   * ⚠ **When focus is on an entry, the pointer brings it back to the field.** A menu row takes
   * real focus when the pointer passes over it, so a person who brushed the entries and then
   * pointed at a swatch would otherwise have arrow keys still driving the menu while the grid
   * showed a cursor: two things looking active and the keyboard on the one not under the pointer.
   */
  override movedActive(index: number): void {
    if (index >= 0 && this.#focusIsOnAnEntry()) this.entryElement?.focus({ preventScroll: true });
    super.movedActive(index);
  }

  /**
   * Rebuild the swatches, then render.
   *
   * Before `super.render()` rather than after, because the base reads `visibleOptions` while it
   * renders — a picker that rebuilt afterwards would draw one palette and report another, which is
   * the shape of every defect this catalogue's list controls are written to avoid.
   */
  override render(): void {
    this.#swatches = this.#buildSwatches();
    super.render();
  }

  protected override rendered(field: HTMLElement, entry: HTMLElement): void {
    super.rendered(field, entry);
    let preview = this.#preview;
    if (preview === undefined) {
      preview = document.createElement('span');
      preview.className = 'preview';
      preview.setAttribute('aria-hidden', 'true');
      field.prepend(preview);
      this.#preview = preview;
    }
    const choice = this.choice;
    const paint = choice === undefined ? undefined : this.#paintOf(choice);
    if (paint === undefined) {
      preview.dataset['empty'] = '';
      preview.style.removeProperty(swatchPaintProperty);
    } else {
      delete preview.dataset['empty'];
      preview.style.setProperty(swatchPaintProperty, paint);
    }
    preview.style.setProperty(swatchIndicatorProperty, this.#indicatorFor(paint));
    // The preview's edge sits on the *field*, and the palette's hairlines sit on the *popup*. Two
    // backgrounds, so two answers, written where each is read.
    preview.style.setProperty(swatchHairlineProperty, this.#hairlineOn('surface'));
    this.#palette?.element.style.setProperty(
      swatchHairlineProperty,
      this.#hairlineOn(swatchCellBackground),
    );
  }

  // ── the grammar ────────────────────────────────────────────────────────────

  /** The swatch whose label a person typed — *"Accent 1, Lighter 40%"* is a thing to type. */
  protected override matchByLabel(text: string): string | undefined {
    const wanted = text.trim().toLowerCase();
    const match = this.#swatches.find(
      (swatch) => swatch.label.toLowerCase() === wanted && swatch.unavailable !== true,
    );
    return match?.value;
  }

  /**
   * The grammar this field accepts, in place of the base class's *"whatever you typed"*.
   *
   * An emptied field is *no choice*, which is a value a colour picker may hold and is different
   * from *No fill* — one means the run says nothing about colour, the other means it says
   * explicitly that there is none.
   */
  protected override resolveTyped(text: string): string | undefined {
    if (text.trim() === '') return noColorChosen;
    const byLabel = this.matchByLabel(text);
    if (byLabel !== undefined) return byLabel;
    const choice = parseColorChoice(text);
    return choice === undefined ? undefined : formatColorChoice(choice);
  }

  /** The value's own label, read from the swatches rather than from `<mjx-option>` children. */
  override displayText(): string {
    return displayTextFor(this.#swatches, this.value, this.allowCustom);
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  /**
   * The grid's own keys, before the listbox map sees them.
   *
   * Two rules decide everything here, and both are about the field being a text box as well as a
   * grid:
   *
   * * the popup must be **open**, so a shut picker's arrows and `Home`/`End` are the caret's;
   * * the person must not be **typing**, so `Left`, `Right`, `Home` and `End` stay the caret's
   *   while there is a half-written value in the field. `ArrowDown` and `ArrowUp` are taken in
   *   either case, because a one-line text box has no use for them.
   *
   * A third, for the entries: `ArrowDown` off the palette's last row moves onto them. See the
   * module note.
   */
  protected override interceptKey(event: KeyboardEvent): 'handled' | 'passed' {
    if (!this.open) return 'passed';
    const surface = this.#palette;
    if (surface === undefined) return 'passed';
    const action = swatchKeyAction(event.key, { open: true, direction: this.direction });
    if (action === undefined) return 'passed';

    const caretKey = ['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key);
    if (this.typing && caretKey) return 'passed';

    switch (action) {
      case 'inlineNext':
      case 'inlinePrevious':
      case 'rowNext':
      case 'rowPrevious':
      case 'first':
      case 'last':
      case 'sectionNext':
      case 'sectionPrevious': {
        if (surface.options.length === 0) {
          if (action === 'rowNext') this.#enterEntries(-1);
          return 'handled';
        }
        // Only from a cursor a person can see: with none (a person is typing) Arrow Down puts the
        // cursor on the grid first, as it always has.
        if (surface.activeIndex >= 0 && paletteMovesIntoEntries(action, surface.activeIndex, surface.sections)) {
          if (this.#enterEntries(surface.activeIndex)) return 'handled';
        }
        const from = surface.activeIndex < 0 ? 0 : surface.activeIndex;
        const next = nextSwatchIndex(action, from, surface.sections);
        this.#cursorMovedByKeyboard = true;
        surface.setActive(next);
        this.movedActive(next);
        return 'handled';
      }
      default:
        // `open`, `close`, `commit`, `revert` and `leave` mean exactly what they mean for every
        // other list field, and are deliberately left to the map that already says so.
        return 'passed';
    }
  }

  // ── the entries ────────────────────────────────────────────────────────────

  /**
   * The region under the swatches, and the four listeners that join the menu to the picker.
   *
   * The listeners are on the host rather than on the menu, so a menu that is swapped for another
   * needs no rewiring: each one asks `entriesMenu` at the moment it runs.
   */
  #buildEntries(): void {
    const surface = this.#palette;
    if (surface === undefined) return;
    const region = document.createElement('div');
    region.className = 'entries';
    region.setAttribute('part', 'entries');
    region.hidden = true;
    const slot = document.createElement('slot');
    slot.name = colorPickerEntriesSlot;
    slot.addEventListener('slotchange', this.#onEntriesChange);
    region.append(slot);
    surface.element.append(region);
    this.#entriesRegion = region;
    this.#entriesSlot = slot;

    // Capture, so the picker sees Arrow Up on the first entry before the menu wraps it round to
    // the last.
    this.addEventListener('keydown', this.#onEntriesKeyDown, true);
    this.addEventListener('focusin', this.#onEntriesFocusIn);
    this.addEventListener('focusout', this.#onEntriesFocusOut);
    this.addEventListener(menuEvents.activate, this.#onEntryActivated);
    this.addEventListener(menuEvents.submenuToggle, this.#onEntrySubmenuToggle);
    this.#onEntriesChange();
  }

  #onEntriesChange = (): void => {
    const region = this.#entriesRegion;
    const slot = this.#entriesSlot;
    if (region === undefined || slot === undefined) return;
    region.hidden = slot.assignedElements().length === 0;
    const menu = this.entriesMenu;
    if (menu !== undefined && !menu.hasAttribute('embedded')) menu.setAttribute('embedded', '');
    // The popup changed size under an open placement.
    const field = this.fieldElement;
    if (this.open && field !== undefined) this.#palette?.place(field, this.direction);
  };

  /** Move the keyboard from the palette onto the first entry. `false` when there is none to move to. */
  #enterEntries(leftFrom: number): boolean {
    const surface = this.#palette;
    const menu = this.entriesMenu;
    if (surface === undefined || menu === undefined || menu.items.length === 0) return false;
    this.#entriesLeftFrom = leftFrom >= 0 ? leftFrom : undefined;
    this.#cursorMovedByKeyboard = false;
    surface.setActive(-1);
    this.movedActive(-1);
    menu.focusItem(0);
    return true;
  }

  /** Move the keyboard from the entries back onto the swatch it left, or the row above the entries. */
  #returnToPalette(): void {
    const surface = this.#palette;
    if (surface === undefined) return;
    const index = paletteReturnIndex(surface.sections, this.#entriesLeftFrom);
    this.#entriesLeftFrom = undefined;
    this.entryElement?.focus({ preventScroll: true });
    this.#cursorMovedByKeyboard = index >= 0;
    surface.setActive(index);
    this.movedActive(index);
  }

  /** The index of an event target among the entries menu's own rows, or `-1`. A submenu's row is not one. */
  #entryIndexOf(target: EventTarget | null): number {
    const menu = this.entriesMenu;
    if (menu === undefined || !(target instanceof HTMLElement) || target.localName !== menuTags.item) return -1;
    return menu.items.indexOf(target as MjxMenuItem);
  }

  #focusIsOnAnEntry(): boolean {
    const menu = this.entriesMenu;
    const active = deepActiveElement(this.ownerDocument);
    return menu !== undefined && active !== null && menu.contains(active);
  }

  #onEntriesKeyDown = (event: KeyboardEvent): void => {
    if (!this.open || event.ctrlKey || event.metaKey || event.altKey) return;
    const index = this.#entryIndexOf(event.target);
    // A submenu's row, or the field: their own key maps decide.
    if (index < 0) return;
    if (entriesReturnToPalette(event.key, index)) {
      event.preventDefault();
      event.stopPropagation();
      this.#returnToPalette();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      this.closeList('escape');
      return;
    }
    // Tab leaves, as it does from the field. No preventDefault: the browser's own sequential
    // navigation continues, and the popup is already hidden so it does not stop on another entry.
    if (event.key === 'Tab') this.closeList('tab', { restoreFocus: false });
  };

  /**
   * An entry took focus, by the keyboard or because the pointer passed over it (a menu row takes
   * focus on hover). The grid drops its cursor, so only one thing looks active, and the focused
   * entry is kept inside the popup's scrollport when the palette is capped and scrolls.
   */
  #onEntriesFocusIn = (event: FocusEvent): void => {
    if (this.#entryIndexOf(event.target) < 0) return;
    const surface = this.#palette;
    if (surface !== undefined && surface.activeIndex >= 0) {
      this.#cursorMovedByKeyboard = false;
      surface.setActive(-1);
      this.movedActive(-1);
    }
    const palette = surface?.element;
    if (palette === undefined || palette.scrollHeight <= palette.clientHeight) return;
    const row = (event.target as HTMLElement).getBoundingClientRect();
    const box = palette.getBoundingClientRect();
    if (row.top < box.top) palette.scrollTop -= box.top - row.top;
    else if (row.bottom > box.bottom) palette.scrollTop += row.bottom - box.bottom;
  };

  /**
   * The disclosure half of the focus model, for focus that is on an entry.
   *
   * The field's own `focusout` watches the field. Focus on an entry is outside it, so leaving the
   * picker from an entry would otherwise leave the popup open. Deferred a microtask and re-read
   * through every shadow root, for the reason `MjxListField` gives: a row rebuilding under focus
   * reports `null` for a moment on its way back.
   */
  #onEntriesFocusOut = (): void => {
    if (!this.open) return;
    queueMicrotask(() => {
      if (!this.open) return;
      const active = deepActiveElement(this.ownerDocument);
      if (active === null) return;
      if (this.contains(active) || this.shadowRoot?.contains(active) === true) return;
      this.closeList('blur', { restoreFocus: false });
    });
  };

  /** An entry was chosen: close the picker, with focus back on the field. The event still reaches the host. */
  #onEntryActivated = (event: Event): void => {
    const menu = this.entriesMenu;
    if (menu === undefined || !event.composedPath().includes(menu)) return;
    this.closeList('commit');
  };

  /** A submenu opened on a hover timer after the picker closed: close it, rather than strand it in the top layer. */
  #onEntrySubmenuToggle = (): void => {
    if (!this.open) this.#closeEntrySubmenus();
  };

  /** Close every submenu the entries have open, deepest first through each submenu's own close. */
  #closeEntrySubmenus(): void {
    const menu = this.entriesMenu;
    if (menu === undefined) return;
    for (const item of menu.items) {
      const submenu = item.submenu;
      if (!item.submenuOpen || submenu === undefined || submenu.localName !== menuTags.menu) continue;
      (submenu as MjxMenu).close('outside', { restoreFocus: false });
    }
  }

  // ── the swatches ───────────────────────────────────────────────────────────

  #buildSwatches(): SwatchDescriptor[] {
    const swatches: SwatchDescriptor[] = [];

    if (this.showsAutomatic) {
      swatches.push(
        this.#swatchFor({ kind: 'automatic' }, colorSectionNames.reset, describeColorChoice({ kind: 'automatic' })),
      );
    }
    if (this.showsNoFill) {
      const label = this.noFillLabel;
      swatches.push(this.#swatchFor({ kind: 'none' }, colorSectionNames.reset, label, label));
    }

    // The theme block: variant-major, so each **column** is one slot and each row is one step of
    // the luminance ladder — which is the arrangement a person reads a theme gallery in, and the
    // one `ArrowDown` therefore has to mean "the same slot, one step lighter or darker".
    const hasTheme = galleryThemeSlots.some((slot) => this.#themePalette[slot] !== undefined);
    if (hasTheme) {
      for (const variant of themeColorVariantNames) {
        for (const slot of galleryThemeSlots) {
          const choice: ColorChoice = { kind: 'theme', slot, variant };
          const paint = resolveThemeColor(slot, variant, this.#themePalette);
          const swatch = this.#swatchFor(choice, colorSectionNames.theme);
          swatches.push(
            paint === undefined
              ? {
                  ...swatch,
                  unavailable: true,
                  explanation: `This document does not define ${themeColorSlots[slot].label}.`,
                }
              : swatch,
          );
        }
      }
    }

    for (const hex of this.#standardColors) {
      const choice = parseColorChoice(hex);
      if (choice === undefined) continue;
      swatches.push(this.#swatchFor(choice, colorSectionNames.standard));
    }

    for (const hex of this.#recentColors) {
      const choice = parseColorChoice(hex);
      if (choice === undefined) continue;
      swatches.push(this.#swatchFor(choice, colorSectionNames.recent));
    }

    return swatches;
  }

  #swatchFor(choice: ColorChoice, category: string, chipLabel?: string, label?: string): SwatchDescriptor {
    const paint = this.#paintOf(choice);
    return {
      value: formatColorChoice(choice),
      label: label ?? describeColorChoice(choice),
      category,
      paint,
      indicator: this.#indicatorFor(paint),
      ...(chipLabel === undefined ? {} : { chipLabel }),
    };
  }

  /** What a choice paints as. **Only ever used to fill a square.** */
  #paintOf(choice: ColorChoice): string | undefined {
    switch (choice.kind) {
      case 'automatic':
        return this.automatic;
      case 'none':
        return undefined;
      case 'theme':
        return resolveThemeColor(choice.slot, choice.variant, this.#themePalette);
      case 'literal':
        return choice.hex;
    }
  }

  /**
   * The token member the ring and the check mark are drawn in, for one swatch.
   *
   * Measured against the two candidates **as this host resolves them**, so a host that has
   * re-themed the platform gets the right answer rather than the answer for our own palette.
   */
  #indicatorFor(paint: string | undefined): string {
    const member: SwatchIndicatorMember =
      paint === undefined
        ? 'textPrimary'
        : chooseSwatchIndicatorAmong(paint, {
            textPrimary: this.#resolver?.theme('textPrimary') ?? themeVariable('textPrimary'),
            surface: this.#resolver?.theme('surface') ?? themeVariable('surface'),
          }).member;
    return themeVariable(member);
  }

  /**
   * The token member a swatch's **hairline** is drawn in, on one background.
   *
   * A different question from `#indicatorFor` and therefore a different call: the indicator sits on
   * the colour a person chose, the hairline sits against whatever the square is drawn on. Written
   * once per surface — onto the palette, which every cell inherits from, and onto the preview,
   * whose background is the field rather than the popup — because nothing about it varies per
   * swatch. See `swatchHairlineProperty` for the defect that came of conflating them.
   */
  #hairlineOn(backgroundMember: ThemeMember): string {
    const resolver = this.#resolver;
    const background = resolver?.theme(backgroundMember) ?? themeVariable(backgroundMember);
    return themeVariable(
      chooseSwatchHairlineAmong(background, {
        textPrimary: resolver?.theme('textPrimary') ?? themeVariable('textPrimary'),
        surface: resolver?.theme('surface') ?? themeVariable('surface'),
      }).member,
    );
  }
}

/** The focused element, followed through every shadow root it is hiding in. */
function deepActiveElement(document_: Document): Element | null {
  let element: Element | null = document_.activeElement;
  while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
  if (element === document_.body || element === document_.documentElement) return null;
  return element;
}

/** Register the element, and the menu elements its entries are written in. Idempotent. */
export function defineColorPicker(): void {
  defineIcon();
  defineListFieldDependencies();
  defineMenus();
  if (customElements.get('mjx-color-picker') === undefined) {
    customElements.define('mjx-color-picker', MjxColorPicker);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-color-picker': MjxColorPicker;
  }
}
