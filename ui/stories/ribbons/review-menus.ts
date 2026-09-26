/**
 * **The menus the Review tab opens**, written once and rendered by both hosts.
 *
 * The ribbon programme's unit 8 wrote **Word's Review tab** alone, and PowerPoint's and Excel's followed in
 * their own units, one tab of one application each time. The pattern is `stories/ribbons/insert-menus.ts`'s, for its reasons. A binding
 * lives in its host. The menu it opens is written here, with its id from `commandSurfaceId(host,
 * commandId)` through `commandMenu`. A host renders `reviewMenus(application, host)` once beside its
 * ribbon. Every `commandMenu(host, '…'` call below spells its command id literally, so
 * `tests/ribbons.test.ts` can read it.
 *
 * ## Complete, not shallow
 *
 * Earlier units wrote *a handful of Office's own entries*; the user rejected a sampled Transitions
 * gallery, and since then a popup carries **every entry Office's popup has**, by Office's names and in
 * Office's order. Two things are still not Office's shape, and both are older decisions:
 *
 * - **A submenu is flattened into a labelled section** — Show Markup's *Balloons* and *Specific People*,
 *   Compare's *Show Source Documents* — as Insert flattened Page Number's.
 * - **Show Markup's *Specific People* lists All Reviewers alone.** Office lists the document's reviewers
 *   under it by name, and a reviewer is the document's data rather than Office's vocabulary: a name here
 *   would name somebody's colleague, which is `insert-menus.ts`'s argument about a real printer.
 *
 * The current choice is checked where a menu has one: For Everyone, Contextual, Simple Markup's four
 * markup kinds, Show Only Comments and Formatting in Balloons, All Reviewers, Show Both.
 *
 * ⚠ `GUESS:` **several entry lists are from memory of Microsoft 365 rather than from a build this project
 * can cite**, and each says so where it is written. None is invented to fill a count.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no document changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string, shortcut?: string): TemplateResult {
  return html`<mjx-menu-item label=${label} shortcut=${shortcut ?? ''}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** One entry that is a setting on its own, checked or not. */
function setting(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="checkbox" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── Word's Review ────────────────────────────────────────────────────────────

/**
 * Check Accessibility's arrow: the checker, then the three doors Office puts beside it. `GUESS:` the
 * list, from Microsoft 365's Word, and *Options: Accessibility* in particular.
 */
function checkAccessibilityEntries(): TemplateResult[] {
  return [
    item('Check Accessibility'),
    item('Alt Text'),
    item('Navigation Pane'),
    separator(),
    item('Options: Accessibility'),
  ];
}

/** Translate: Microsoft 365's two, which replaced Word 2016's Mini Translator and Translate Document. */
function translateEntries(): TemplateResult[] {
  return [item('Translate Selection'), item('Translate Document')];
}

/** Language: Office's two dialogs. **Shared with PowerPoint**, whose Language arrow is the same two. */
function languageEntries(): TemplateResult[] {
  return [item('Set Proofing Language…'), item('Language Preferences…')];
}

/** Delete's arrow: the comment, the comments a filter shows, and every comment. */
function deleteCommentEntries(): TemplateResult[] {
  return [item('Delete'), item('Delete All Comments Shown'), item('Delete All Comments in Document')];
}

/**
 * Show Comments' arrow: Microsoft 365's two comment views, Contextual checked. `GUESS:` the whole
 * shape; see `dev/ribbons/census.ts`.
 */
function showCommentsEntries(): TemplateResult[] {
  return [choice('Contextual', true), choice('List')];
}

/** Track Changes' arrow: whose edits are tracked, then the password lock. */
function trackChangesEntries(): TemplateResult[] {
  return [choice('For Everyone', true), choice('Just Mine'), separator(), item('Lock Tracking')];
}

/**
 * Show Markup: the four kinds of markup, each checked as in a new document, then Office's two submenus
 * flattened. Balloons' three placements with Word's default checked; Specific People's All Reviewers.
 */
function showMarkupEntries(): TemplateResult[] {
  return [
    setting('Comments', true),
    setting('Ink', true),
    setting('Insertions and Deletions', true),
    setting('Formatting', true),
    section(
      'Balloons',
      choice('Show Revisions in Balloons'),
      choice('Show All Revisions Inline'),
      choice('Show Only Comments and Formatting in Balloons', true),
    ),
    section('Specific People', setting('All Reviewers', true)),
  ];
}

/** Reviewing Pane's arrow: the pane's two orientations. The face opens the vertical one. */
function reviewingPaneEntries(): TemplateResult[] {
  return [item('Reviewing Pane Vertical…'), item('Reviewing Pane Horizontal…')];
}

/** Accept's arrow: Office's five, in Office's order. */
function acceptEntries(): TemplateResult[] {
  return [
    item('Accept and Move to Next'),
    item('Accept This Change'),
    item('Accept All Changes Shown'),
    item('Accept All Changes'),
    item('Accept All Changes and Stop Tracking'),
  ];
}

/** Reject's arrow: Office's five, in Office's order. */
function rejectEntries(): TemplateResult[] {
  return [
    item('Reject and Move to Next'),
    item('Reject Change'),
    item('Reject All Changes Shown'),
    item('Reject All Changes'),
    item('Reject All Changes and Stop Tracking'),
  ];
}

/** Compare: the two dialogs, then the Show Source Documents submenu flattened, Show Both checked. */
function compareEntries(): TemplateResult[] {
  return [
    item('Compare…'),
    item('Combine…'),
    section(
      'Show Source Documents',
      choice('Hide Source Documents'),
      choice('Show Original'),
      choice('Show Revised'),
      choice('Show Both', true),
    ),
  ];
}

/** Block Authors' arrow. `GUESS:` the pair, as Word 2010 drew it. */
function blockAuthorsEntries(): TemplateResult[] {
  return [item('Block Authors'), item('Release All of My Blocked Areas')];
}

/** Hide Ink's arrow. `GUESS:` the pair, from Microsoft 365. */
function hideInkEntries(): TemplateResult[] {
  return [setting('Hide Ink'), item('Delete All Ink in Document')];
}

/** Thirteen menus. Display for Review is a field the hosts bind. */
function wordReviewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.review.accessibility.check-accessibility', 'Check Accessibility', ...checkAccessibilityEntries())}
    ${commandMenu(host, 'word.review.language.translate', 'Translate', ...translateEntries())}
    ${commandMenu(host, 'word.review.language.language', 'Language', ...languageEntries())}
    ${commandMenu(host, 'word.review.comments.delete', 'Delete', ...deleteCommentEntries())}
    ${commandMenu(host, 'word.review.comments.show-comments', 'Show Comments', ...showCommentsEntries())}
    ${commandMenu(host, 'word.review.tracking.track-changes', 'Track Changes', ...trackChangesEntries())}
    ${commandMenu(host, 'word.review.tracking.show-markup', 'Show Markup', ...showMarkupEntries())}
    ${commandMenu(host, 'word.review.tracking.reviewing-pane', 'Reviewing Pane', ...reviewingPaneEntries())}
    ${commandMenu(host, 'word.review.changes.accept', 'Accept', ...acceptEntries())}
    ${commandMenu(host, 'word.review.changes.reject', 'Reject', ...rejectEntries())}
    ${commandMenu(host, 'word.review.compare.compare', 'Compare', ...compareEntries())}
    ${commandMenu(host, 'word.review.protect.block-authors', 'Block Authors', ...blockAuthorsEntries())}
    ${commandMenu(host, 'word.review.ink.hide-ink', 'Hide Ink', ...hideInkEntries())}
  `;
}

// ── PowerPoint's Review ──────────────────────────────────────────────────────
//
// Seven menus. Only Language's list is shared with Word, because it is the only one that is the same list:
// every other arrow either names a slide or a presentation where Word's names a document, or offers
// something Word's does not. The five entry helpers above are shared.

/**
 * Check Accessibility's arrow in PowerPoint: Word's list with **Reading Order Pane** where Word has
 * Navigation Pane, because a slide has a reading order and no headings to navigate. `GUESS:` the list,
 * from Microsoft 365's PowerPoint, and *Options: Accessibility* in particular.
 */
function powerpointCheckAccessibilityEntries(): TemplateResult[] {
  return [
    item('Check Accessibility'),
    item('Alt Text'),
    item('Reading Order Pane'),
    separator(),
    item('Options: Accessibility'),
  ];
}

/**
 * Delete's arrow in PowerPoint: the comment, then every comment and ink stroke on the slide, then in the
 * whole deck. `GUESS:` the wording in Microsoft 365, which is PowerPoint 2013's.
 */
function powerpointDeleteCommentEntries(): TemplateResult[] {
  return [
    item('Delete'),
    item('Delete All Comments and Ink on This Slide'),
    item('Delete All Comments and Ink in This Presentation'),
  ];
}

/**
 * Show Comments' arrow in PowerPoint: the Comments pane, which the face also opens and which starts
 * closed, and whether comment markers are drawn on the slide, which starts on. `GUESS:` the pair in
 * Microsoft 365, which is PowerPoint 2013's.
 */
function powerpointShowCommentsEntries(): TemplateResult[] {
  return [setting('Comments Pane'), setting('Show Markup', true)];
}

/** Accept's arrow in PowerPoint: one change, the slide's changes, the deck's changes. */
function powerpointAcceptEntries(): TemplateResult[] {
  return [
    item('Accept Change'),
    item('Accept All Changes to This Slide'),
    item('Accept All Changes to the Presentation'),
  ];
}

/** Reject's arrow in PowerPoint: the same three, for rejecting. */
function powerpointRejectEntries(): TemplateResult[] {
  return [
    item('Reject Change'),
    item('Reject All Changes to This Slide'),
    item('Reject All Changes to the Presentation'),
  ];
}

/** Hide Ink's arrow in PowerPoint. `GUESS:` the pair, and *in Presentation* in particular. */
function powerpointHideInkEntries(): TemplateResult[] {
  return [setting('Hide Ink'), item('Delete All Ink in Presentation')];
}

/**
 * Seven menus. Spelling, Thesaurus, Translate, New Comment, the four navigators, Compare, Reviewing Pane,
 * End Review and Show Changes open a pane, a dialog or a file picker, or act at once, so they have none.
 */
function powerpointReviewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.review.accessibility.check-accessibility', 'Check Accessibility', ...powerpointCheckAccessibilityEntries())}
    ${commandMenu(host, 'powerpoint.review.language.language', 'Language', ...languageEntries())}
    ${commandMenu(host, 'powerpoint.review.comments.delete', 'Delete', ...powerpointDeleteCommentEntries())}
    ${commandMenu(host, 'powerpoint.review.comments.show-comments', 'Show Comments', ...powerpointShowCommentsEntries())}
    ${commandMenu(host, 'powerpoint.review.compare.accept', 'Accept', ...powerpointAcceptEntries())}
    ${commandMenu(host, 'powerpoint.review.compare.reject', 'Reject', ...powerpointRejectEntries())}
    ${commandMenu(host, 'powerpoint.review.ink.hide-ink', 'Hide Ink', ...powerpointHideInkEntries())}
  `;
}

// ── Excel's Review ───────────────────────────────────────────────────────────
//
// Four menus, and none of them shares a list with Word or PowerPoint: Check Accessibility's arrow has no
// pane in it, Hide Ink's names a sheet, and Notes and Track Changes are Excel's alone. The five entry
// helpers at the top of this file are shared.

/**
 * Check Accessibility's arrow in Excel: the checker, Alt Text, then the options. Word's Navigation Pane and
 * PowerPoint's Reading Order Pane have no Excel counterpart here. `GUESS:` the list, from Microsoft 365's
 * Excel, and *Options: Accessibility* in particular.
 */
function excelCheckAccessibilityEntries(): TemplateResult[] {
  return [item('Check Accessibility'), item('Alt Text'), separator(), item('Options: Accessibility')];
}

/**
 * Notes: Microsoft 365's six, in Office's order. New Note opens a note on the selected cell (Shift+F2);
 * the two navigators move between notes; Show/Hide Note and Show All Notes are settings, off in a new
 * workbook; Convert to Comments turns every note into a threaded comment, after a confirmation. `GUESS:`
 * the separator before Convert to Comments.
 */
function excelNotesEntries(): TemplateResult[] {
  return [
    item('New Note', 'Shift+F2'),
    item('Previous Note'),
    item('Next Note'),
    setting('Show/Hide Note'),
    setting('Show All Notes'),
    separator(),
    item('Convert to Comments'),
  ];
}

/** Track Changes: Office 2016's two dialogs, which Microsoft 365 keeps as *Track Changes (Legacy)*. */
function excelTrackChangesEntries(): TemplateResult[] {
  return [item('Highlight Changes…'), item('Accept/Reject Changes')];
}

/** Hide Ink's arrow in Excel. `GUESS:` the pair, and *on Sheet* in particular. */
function excelHideInkEntries(): TemplateResult[] {
  return [setting('Hide Ink'), item('Delete All Ink on Sheet')];
}

/**
 * Four menus. Spelling, Thesaurus, Workbook Statistics, Check Performance, Translate, the comment commands,
 * the Protect group, Share Workbook, Protect and Share Workbook and Debug open a pane or a dialog, act at
 * once, or are toggles, so they have none.
 */
function excelReviewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.review.accessibility.check-accessibility', 'Check Accessibility', ...excelCheckAccessibilityEntries())}
    ${commandMenu(host, 'excel.review.notes.notes', 'Notes', ...excelNotesEntries())}
    ${commandMenu(host, 'excel.review.changes.track-changes', 'Track Changes', ...excelTrackChangesEntries())}
    ${commandMenu(host, 'excel.review.ink.hide-ink', 'Hide Ink', ...excelHideInkEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** All three applications, each written in its own unit. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordReviewMenus,
  powerpoint: powerpointReviewMenus,
  excel: excelReviewMenus,
};

/**
 * Every menu one application's Review tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `insertMenus` is.
 */
export function reviewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
