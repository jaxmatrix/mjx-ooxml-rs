/**
 * **The menus the Insert tab's commands open** — written once, rendered by both hosts.
 *
 * The ribbon programme's unit 3. Nearly every command on Office's Insert tab opens something — a
 * gallery, a menu, a picker — so this is the first tab where most of a group's face is a binding
 * rather than a generic button. Each binding lives in its host (`stories/ribbons/*.stories.ts` and
 * `stories/shell/*.stories.ts`), because `tests/shell.test.ts` counts what a shell composes in the
 * shell's own source. **What the binding opens does not**, and writing fifty-one menus twice would be
 * a hundred and two places for one of them to drift — the argument `dev/ribbons/census.ts` is
 * written under, one level up. So the menus are here, keyed by command id, and a host renders
 * `insertMenus(application, host)` once beside its ribbon.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan: *name what the tab shows; menus stay shallow.* Every entry below
 * is one Office shows under that command — *Austin* and *Banded* from Word's header gallery, *Edit
 * Header* and *Remove Header* under them, *Clustered Column* from Excel's column gallery — and there
 * are a handful of each rather than all of them. A gallery Office draws as a grid of pictures is a
 * menu of named entries here, because the art is the long tail and the command is the control that
 * opens it. Office's submenu arrows (Page Number → Top of Page, Quick Tables) are flattened for the
 * same reason.
 *
 * Two lists name things that are *this machine's* rather than Office's — a link menu's recent items
 * and a screenshot menu's available windows. The first uses invented file names, as
 * `ribbon-parts.ts`'s printers do; the second shows no windows at all, because a list of somebody's
 * open windows is a claim this catalogue cannot make.
 *
 * ## The id is derived, and the gate reads this file
 *
 * `commandMenu(host, '<command id>', …)` gives each menu `commandSurfaceId(host, commandId)`, and a
 * binding opens it with the same derivation written as a `data-opens` literal. `tests/ribbons.test.ts`
 * reads every `commandMenu(host, '…'` call in this directory and requires each menu to be bound by
 * both of its application's hosts, and every `data-opens` in a host to resolve — so a binding that
 * names a sibling's menu, or a menu nobody binds, fails in Node rather than in front of an auditor.
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, and no document
 * changes, because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string, shortcut?: string): TemplateResult {
  return html`<mjx-menu-item label=${label} shortcut=${shortcut ?? ''}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── what several applications open identically ──────────────────────────────

/** Pictures, in Word and PowerPoint: the three places a picture comes from. */
function picturesEntries(): TemplateResult[] {
  return [item('This Device…'), item('Stock Images…'), item('Online Pictures…')];
}

/** Shapes, in all three: a few of each of the gallery's first four categories. */
function shapesEntries(): TemplateResult[] {
  return [
    section('Lines', item('Line'), item('Line Arrow')),
    section('Rectangles', item('Rectangle'), item('Rectangle: Rounded Corners')),
    section('Basic Shapes', item('Text Box'), item('Oval'), item('Isosceles Triangle')),
    section('Block Arrows', item('Arrow: Right'), item('Arrow: Left')),
  ];
}

/** 3D Models, in all three. */
function threeDModelsEntries(): TemplateResult[] {
  return [item('This Device…'), item('Stock 3D Models…')];
}

/**
 * Screenshot, in all three: the clipping command, and no windows.
 *
 * Office lists the windows open on this machine above it. A catalogue that invented some would be
 * claiming to know what somebody has open; one that shows none is what Office shows on a machine with
 * nothing else running.
 */
function screenshotEntries(): TemplateResult[] {
  return [item('Screen Clipping')];
}

/** Link's arrow, in all three: recent items, then the dialog. The file names are invented. */
function linkEntries(): TemplateResult[] {
  return [
    section('Recent Items', item('Method notes.docx'), item('Findings.xlsx'), item('Where the time went.pptx')),
    separator(),
    item('Insert Link…', 'Ctrl+K'),
  ];
}

/** WordArt, in all three: the first styles of Office's gallery, under their own names. */
function wordArtEntries(): TemplateResult[] {
  return [
    item('Fill: Black, Text colour 1; Shadow'),
    item('Fill: Blue, Accent colour 1; Shadow'),
    item('Fill: White; Outline: Blue, Accent colour 1; Glow: Blue, Accent colour 1'),
    item('Gradient Fill: Blue, Accent colour 5; Reflection'),
  ];
}

/** Equation's arrow, in all three: the built-in equations, then the two ways to write a new one. */
function equationEntries(): TemplateResult[] {
  return [
    section(
      'Built-In',
      item('Area of Circle'),
      item('Binomial Theorem'),
      item('Fourier Series'),
      item('Pythagorean Theorem'),
      item('Quadratic Formula'),
    ),
    separator(),
    item('Insert New Equation', 'Alt+='),
    item('Ink Equation'),
  ];
}

/** Signature Line's arrow, in Word and Excel. */
function signatureLineEntries(): TemplateResult[] {
  return [item('Microsoft Office Signature Line…'), item('Add Signature Services…')];
}

// ── Word ─────────────────────────────────────────────────────────────────────

function wordInsertMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(
      host,
      'word.insert.pages.cover-page',
      'Cover Page',
      section('Built-In', item('Austin'), item('Banded'), item('Facet'), item('Grid'), item('Ion (Dark)'), item('Whisp')),
      separator(),
      item('Remove Current Cover Page'),
      item('Save Selection to Cover Page Gallery…'),
    )}
    ${commandMenu(
      host,
      'word.insert.tables.table',
      'Table',
      item('Insert Table…'),
      item('Draw Table'),
      item('Convert Text to Table…'),
      item('Excel Spreadsheet'),
      item('Quick Tables'),
    )}
    ${commandMenu(host, 'word.insert.illustrations.pictures', 'Pictures', ...picturesEntries())}
    ${commandMenu(
      host,
      'word.insert.illustrations.shapes',
      'Shapes',
      ...shapesEntries(),
      separator(),
      item('New Drawing Canvas'),
    )}
    ${commandMenu(host, 'word.insert.illustrations.3d-models', '3D Models', ...threeDModelsEntries())}
    ${commandMenu(host, 'word.insert.illustrations.screenshot', 'Screenshot', ...screenshotEntries())}
    ${commandMenu(host, 'word.insert.links.link', 'Link', ...linkEntries())}
    ${commandMenu(
      host,
      'word.insert.header-footer.header',
      'Header',
      section('Built-In', item('Blank'), item('Blank (Three Columns)'), item('Austin'), item('Banded'), item('Facet (Even Page)')),
      separator(),
      item('Edit Header'),
      item('Remove Header'),
      item('Save Selection to Header Gallery…'),
    )}
    ${commandMenu(
      host,
      'word.insert.header-footer.footer',
      'Footer',
      section('Built-In', item('Blank'), item('Blank (Three Columns)'), item('Austin'), item('Banded'), item('Facet (Even Page)')),
      separator(),
      item('Edit Footer'),
      item('Remove Footer'),
      item('Save Selection to Footer Gallery…'),
    )}
    ${commandMenu(
      host,
      'word.insert.header-footer.page-number',
      'Page Number',
      item('Top of Page'),
      item('Bottom of Page'),
      item('Page Margins'),
      item('Current Position'),
      separator(),
      item('Format Page Numbers…'),
      item('Remove Page Numbers'),
    )}
    ${commandMenu(
      host,
      'word.insert.text.text-box',
      'Text Box',
      section('Built-In', item('Simple Text Box'), item('Austin Quote'), item('Austin Sidebar'), item('Banded Quote')),
      separator(),
      item('Draw Text Box'),
      item('Save Selection to Text Box Gallery…'),
    )}
    ${commandMenu(
      host,
      'word.insert.text.quick-parts',
      'Quick Parts',
      item('AutoText'),
      item('Document Property'),
      item('Field…'),
      item('Building Blocks Organizer…'),
      separator(),
      item('Save Selection to Quick Part Gallery…'),
    )}
    ${commandMenu(host, 'word.insert.text.wordart', 'WordArt', ...wordArtEntries())}
    ${commandMenu(
      host,
      'word.insert.text.drop-cap',
      'Drop Cap',
      html`<mjx-menu-section label="Position">
        <mjx-menu-item kind="radio" label="None" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Dropped"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="In margin"></mjx-menu-item>
      </mjx-menu-section>`,
      separator(),
      item('Drop Cap Options…'),
    )}
    ${commandMenu(host, 'word.insert.text.signature-line', 'Signature Line', ...signatureLineEntries())}
    ${commandMenu(host, 'word.insert.text.object', 'Object', item('Object…'), item('Text from File…'))}
    ${commandMenu(host, 'word.insert.symbols.equation', 'Equation', ...equationEntries())}
    ${commandMenu(
      host,
      'word.insert.symbols.symbol',
      'Symbol',
      section('Recently Used', item('€'), item('£'), item('¥'), item('©'), item('®'), item('™'), item('±'), item('≠')),
      separator(),
      item('More Symbols…'),
    )}
  `;
}

// ── PowerPoint ───────────────────────────────────────────────────────────────

function powerpointInsertMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(
      host,
      'powerpoint.insert.slides.new-slide',
      'New Slide',
      section(
        'Office Theme',
        item('Title Slide'),
        item('Title and Content'),
        item('Section Header'),
        item('Two Content'),
        item('Comparison'),
        item('Title Only'),
        item('Blank'),
      ),
      separator(),
      item('Duplicate Selected Slides'),
      item('Slides from Outline…'),
      item('Reuse Slides…'),
    )}
    ${commandMenu(
      host,
      'powerpoint.insert.tables.table',
      'Table',
      item('Insert Table…'),
      item('Draw Table'),
      item('Excel Spreadsheet'),
    )}
    ${commandMenu(host, 'powerpoint.insert.images.pictures', 'Pictures', ...picturesEntries())}
    ${commandMenu(host, 'powerpoint.insert.images.screenshot', 'Screenshot', ...screenshotEntries())}
    ${commandMenu(
      host,
      'powerpoint.insert.images.photo-album',
      'Photo Album',
      item('New Photo Album…'),
      item('Edit Photo Album…'),
    )}
    ${commandMenu(host, 'powerpoint.insert.illustrations.shapes', 'Shapes', ...shapesEntries())}
    ${commandMenu(host, 'powerpoint.insert.illustrations.3d-models', '3D Models', ...threeDModelsEntries())}
    ${commandMenu(host, 'powerpoint.insert.camera.cameo', 'Cameo', item('This Slide'), item('All Slides'))}
    ${commandMenu(
      host,
      'powerpoint.insert.links.zoom',
      'Zoom',
      item('Summary Zoom'),
      item('Section Zoom'),
      item('Slide Zoom'),
    )}
    ${commandMenu(host, 'powerpoint.insert.links.link', 'Link', ...linkEntries())}
    ${commandMenu(host, 'powerpoint.insert.text.wordart', 'WordArt', ...wordArtEntries())}
    ${commandMenu(host, 'powerpoint.insert.symbols.equation', 'Equation', ...equationEntries())}
    ${commandMenu(
      host,
      'powerpoint.insert.media-clips.video',
      'Video',
      item('This Device…'),
      item('Stock Videos…'),
      item('Online Videos…'),
    )}
    ${commandMenu(
      host,
      'powerpoint.insert.media-clips.audio',
      'Audio',
      item('Audio on My PC…'),
      item('Record Audio…'),
    )}
  `;
}

// ── Excel ────────────────────────────────────────────────────────────────────

function excelInsertMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(
      host,
      'excel.insert.tables.pivottable',
      'PivotTable',
      item('From Table/Range'),
      item('From External Data Source'),
      item('From Data Model'),
      item('From Power BI'),
    )}
    ${commandMenu(
      host,
      'excel.insert.illustrations.pictures',
      'Pictures',
      section('Place in Cell', ...picturesEntries()),
      section('Place over Cells', ...picturesEntries()),
    )}
    ${commandMenu(host, 'excel.insert.illustrations.shapes', 'Shapes', ...shapesEntries())}
    ${commandMenu(host, 'excel.insert.illustrations.3d-models', '3D Models', ...threeDModelsEntries())}
    ${commandMenu(host, 'excel.insert.illustrations.screenshot', 'Screenshot', ...screenshotEntries())}
    ${commandMenu(
      host,
      'excel.insert.charts.column-bar',
      'Insert Column or Bar Chart',
      section('2-D Column', item('Clustered Column'), item('Stacked Column'), item('100% Stacked Column')),
      section('2-D Bar', item('Clustered Bar'), item('Stacked Bar'), item('100% Stacked Bar')),
      separator(),
      item('More Column Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.hierarchy',
      'Insert Hierarchy Chart',
      section('Treemap', item('Treemap')),
      section('Sunburst', item('Sunburst')),
      separator(),
      item('More Hierarchy Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.waterfall',
      'Insert Waterfall, Funnel, Stock, Surface or Radar Chart',
      section('Waterfall', item('Waterfall')),
      section('Funnel', item('Funnel')),
      section('Stock', item('High-Low-Close')),
      section('Surface', item('3-D Surface')),
      section('Radar', item('Radar')),
      separator(),
      item('More Stock Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.line-area',
      'Insert Line or Area Chart',
      section('2-D Line', item('Line'), item('Stacked Line'), item('Line with Markers')),
      section('2-D Area', item('Area'), item('Stacked Area')),
      separator(),
      item('More Line Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.statistic',
      'Insert Statistic Chart',
      section('Histogram', item('Histogram'), item('Pareto')),
      section('Box and Whisker', item('Box and Whisker')),
      separator(),
      item('More Statistical Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.combo',
      'Insert Combo Chart',
      item('Clustered Column – Line'),
      item('Clustered Column – Line on Secondary Axis'),
      item('Stacked Area – Clustered Column'),
      separator(),
      item('Create Custom Combo Chart…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.pie-doughnut',
      'Insert Pie or Doughnut Chart',
      section('2-D Pie', item('Pie'), item('Pie of Pie'), item('Bar of Pie')),
      section('Doughnut', item('Doughnut')),
      separator(),
      item('More Pie Charts…'),
    )}
    ${commandMenu(
      host,
      'excel.insert.charts.scatter-bubble',
      'Insert Scatter (X, Y) or Bubble Chart',
      section('Scatter', item('Scatter'), item('Scatter with Smooth Lines')),
      section('Bubble', item('Bubble')),
      separator(),
      item('More Scatter Charts…'),
    )}
    ${commandMenu(host, 'excel.insert.charts.maps', 'Maps', item('Filled Map'))}
    ${commandMenu(
      host,
      'excel.insert.charts.pivotchart',
      'PivotChart',
      item('PivotChart'),
      item('PivotChart & PivotTable'),
    )}
    ${commandMenu(host, 'excel.insert.links.link', 'Link', ...linkEntries())}
    ${commandMenu(host, 'excel.insert.text.wordart', 'WordArt', ...wordArtEntries())}
    ${commandMenu(host, 'excel.insert.text.signature-line', 'Signature Line', ...signatureLineEntries())}
    ${commandMenu(host, 'excel.insert.symbols.equation', 'Equation', ...equationEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

const menusByApplication: Readonly<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordInsertMenus,
  powerpoint: powerpointInsertMenus,
  excel: excelInsertMenus,
};

/**
 * Every menu one application's Insert tab opens, with ids for one host's page.
 *
 * A host renders this once, beside its `<mjx-ribbon>` — floating and closed, so it occupies no layout
 * — and binds each command's control with `data-opens` set to the same `commandSurfaceId`.
 */
export function insertMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  return menusByApplication[application](host);
}
