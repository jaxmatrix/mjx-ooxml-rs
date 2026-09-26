/**
 * **The lists Word's Outlining tab offers**, written once for every host that draws the tab.
 *
 * **Word's Outlining tab**, authored under the one-tab-one-application rule, and the only application with the
 * tab: PowerPoint's outline is a pane of Normal view and Excel has none. The pattern is
 * `stories/ribbons/view-menus.ts`'s `excelSheetViews`, for its reason: a field's options are data a host renders
 * inside its own `<mjx-dropdown>`, so the list is written here and the binding lives in the host.
 *
 * ## No menus, because the tab opens none
 *
 * Every command on the tab is a button, a toggle, a checkbox or a field, and Office's Outlining tab has no
 * dropdown button, split button or gallery. **So this file declares no `commandMenu` and exports no
 * `outliningMenus` renderer**: a renderer every host called and that drew nothing would be a menu set found to
 * be empty, which is a claim, rather than the absence of one. What the tab *does* open is its two fields' own
 * lists, and those are below.
 *
 * ## Who renders these
 *
 * Outlining is `appearance: 'view'`: Office shows it only inside Outline view, so `tabsFor` leaves it out of a
 * strip unless `includeViewTabs` is asked for, and only `Ribbons/Word` asks. **`stories/ribbons/word.stories.ts`
 * is the one host that binds these lists.** `Shell/Word` never draws the tab, and binds nothing for it.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**, by
 * Office's names and in Office's order.
 *
 * - **Outline Level**: Level 1 to Level 9, then Body Text. ⚠ `GUESS:` the order. Word's Paragraph dialog lists
 *   Body Text first; the ribbon's box is remembered with it last. A host starts it on Body Text, because a new
 *   document's paragraph is Normal text.
 * - **Show Level**: Level 1 to Level 9, then All Levels. A host starts it on All Levels, which is where Outline
 *   view starts.
 *
 * **Nothing here dispatches a command.** A level can be chosen and the field shows it; no paragraph moves,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

/** One option of a field: the value a host starts it on, and what the list draws. */
interface FieldOption {
  readonly value: string;
  readonly label: string;
}

/** Level 1 to Level 9, which both fields list first. */
const levels: readonly FieldOption[] = Array.from({ length: 9 }, (_, index) => ({
  value: `level-${String(index + 1)}`,
  label: `Level ${String(index + 1)}`,
}));

/** The Outline Level field's list: the nine levels, then Body Text. A host starts it on `body-text`. */
export const outlineLevels: readonly FieldOption[] = [...levels, { value: 'body-text', label: 'Body Text' }];

/** The Show Level field's list: the nine levels, then All Levels. A host starts it on `all-levels`. */
export const showLevels: readonly FieldOption[] = [...levels, { value: 'all-levels', label: 'All Levels' }];
