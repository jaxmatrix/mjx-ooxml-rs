/**
 * The surface specimens, and the four lists that are **derived rather than typed twice**.
 *
 * The states matrix comes from `surfaceKindNames`, its captions from `surfaceKinds`, and the
 * measured captions from the same functions the gates sweep — so a caption in the catalogue can
 * never say a different number from the one a test asserts, and a re-seed of the palette moves both
 * together. `<mjx-color-picker>`'s indicator caption established that shape and this follows it.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import {
  chooseDockEdge,
  chooseModalEdge,
  chooseSurfaceHandle,
  describeIndicator,
  modalEdgeCandidates,
  scrimBand,
  scrimColor,
  scrimOpacityPercent,
  surfaceKindNames,
  surfaceKinds,
  themeColor,
  worstAgainstBand,
  worstDockEdge,
  type SurfaceKind,
} from '../../src/surfaces/surface-model.ts';
import { indicatorSweepColors } from '../../src/pickers/picker-model.ts';
import { formatRatio } from '../../src/tokens/contrast.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';
import type { ColorScheme } from '../../tokens/tokens.ts';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:64ch"
    >
      ${text}
    </p>
  `;
}

/** A page-sized stage, so a centred dialog has something to be centred inside. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="min-block-size:32rem;display:flex;flex-direction:column;gap:var(--mjx-density-gutter);
             padding:calc(var(--mjx-density-gutter) * 2);background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/** A block of pretend document content, so a scrim has something to dim. */
export function documentBody(paragraphs = 6): TemplateResult {
  const lines = Array.from({ length: paragraphs }, (_unused, index) => index);
  return html`
    <div
      class=${typeRoleClass('body')}
      style="display:grid;gap:var(--mjx-density-gutter);color:var(--theme-text-primary)"
    >
      ${lines.map(
        (index) => html`
          <p style="margin:0">
            <a href="#stay-${String(index)}">Paragraph ${String(index + 1)}</a> — a run of the
            document behind the surface, carrying a link so that “nothing behind it is reachable”
            is a claim with something to be false about.
          </p>
        `,
      )}
    </div>
  `;
}

/** The states matrix, drawn from the model so a new row appears without anybody remembering. */
export function surfaceStatesFor(kinds: readonly SurfaceKind[] = surfaceKindNames): StoryState[] {
  return kinds.map((kind) => ({
    name: kind,
    description: `${surfaceKinds[kind].use} Focus: ${surfaceKinds[kind].focus}. Dismissed by ${
      surfaceKinds[kind].dismissals.length === 0
        ? 'nothing a person can do — the application closes it'
        : surfaceKinds[kind].dismissals.join(', ')
    }.`,
  }));
}

/** Every token the surfaces read. Hand-listed only where the component names one directly. */
export const surfaceTokenDependencies: readonly TokenPath[] = [
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.background',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.dark.surfaceRaised',
  'theme.dark.background',
  'theme.dark.border',
  'theme.dark.borderSubtle',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  'radius.chip',
  'radius.control',
  'radius.card',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'duration.transition',
  'ease.spring',
  'ease.outSoft',
  'text.sm',
  'font.sans',
];

const schemes: readonly ColorScheme[] = ['light', 'dark'];

/**
 * The scrim caption, measured at the moment the page renders.
 *
 * ⚠ **Every number below is computed, and none of them is written here.** MJXOFF-269 is the record
 * of what a caption that quoted a figure is worth once the palette moves: the figure stays and the
 * screen changes. Reading them out of the same functions `tests/surfaces.test.ts` sweeps means a
 * re-seed changes the caption and the gate in the same commit.
 */
export function scrimReport(): TemplateResult {
  const rows = schemes.map((scheme) => {
    const band = scrimBand(scheme);
    const edge = chooseModalEdge(scheme);
    const handle = chooseSurfaceHandle(scheme);
    return { scheme, band, edge, handle };
  });
  return html`
    <div
      class=${typeRoleClass('dense')}
      style="display:grid;gap:var(--mjx-density-step);padding:calc(var(--mjx-density-gutter) * 2);
             color:var(--theme-text-primary)"
    >
      <p style="margin:0">
        The scrim is the scheme’s darkest token at ${String(scrimOpacityPercent)} per cent, and
        which token that is differs by scheme — an ink scrim in the dark scheme would
        <em>lighten</em> the application rather than darken it.
      </p>
      ${rows.map(
        (row) => html`
          <p style="margin:0">
            <strong>${row.scheme}</strong> — scrim ${scrimColor(row.scheme)}; it composites
            anything at all into the band ${row.band.darkest} … ${row.band.lightest}. The modal’s
            edge is
            ${describeIndicator(row.edge.member, row.edge.ratio, 'that whole band')}, and the
            sheet’s handle is
            ${describeIndicator(row.handle.member, row.handle.ratio, 'the sheet’s own fill')}.
          </p>
        `,
      )}
      <p style="margin:0">
        Every fixed alternative fails: ${fixedEdgeFailures()}
      </p>
    </div>
  `;
}

/**
 * What each fixed choice would have bottomed out at, so the rule's necessity is on the page.
 *
 * The worst of the two schemes, per candidate. Every one of them fails somewhere, which is the
 * whole argument for choosing rather than fixing — and it is on the page rather than only in a
 * test, because an auditor looking at a dialog cannot otherwise tell that the hairline around it
 * was decided by a measurement.
 */
function fixedEdgeFailures(): string {
  return modalEdgeCandidates
    .map((member) => {
      const worst = Math.min(
        ...schemes.map((scheme) =>
          worstAgainstBand(themeColor(scheme, member), scrimBand(scheme)),
        ),
      );
      return `${member} ${formatRatio(worst)}`;
    })
    .join('; ');
}

/**
 * The page colour a task pane's edge is **worst** against, over both schemes.
 *
 * ⚠ **Derived, and MJXOFF-279 is why.** *Against A Coloured Page* used to write `#808080` — a
 * hand-picked mid-grey, chosen because a mid-grey is far from both ends of the palette. It was a
 * reasonable guess and it was not the worst case: the rule actually bottoms out on a **teal**,
 * because `chooseIndicator` takes the *quietest sufficient* candidate rather than the loudest, so
 * the colour that pins it is the one that sits exactly on `textPrimary`'s threshold rather than the
 * one furthest from everything.
 *
 * Computing it here rather than writing it means the specimen an auditor looks at is the same
 * colour the caption beneath it reports, and a re-seed of the palette moves both together — which
 * is the shape every other measured caption in this file already has. It also leaves no literal in
 * the story, which is what let a colour drift out of step with the gate in the first place.
 */
export const worstPageColor: string = (() => {
  let worst = '';
  let lowest = Number.POSITIVE_INFINITY;
  for (const colour of indicatorSweepColors()) {
    let here = Number.POSITIVE_INFINITY;
    for (const scheme of schemes) here = Math.min(here, chooseDockEdge(colour, scheme).ratio);
    if (here < lowest) {
      lowest = here;
      worst = colour;
    }
  }
  return worst;
})();

/** The dock-edge caption: the worst case over the whole sweep, in both schemes. */
export function dockEdgeReport(): TemplateResult {
  const sweep = indicatorSweepColors();
  const rows = schemes.map((scheme) => ({ scheme, worst: worstDockEdge(sweep, scheme) }));
  return html`
    <div
      class=${typeRoleClass('dense')}
      style="display:grid;gap:var(--mjx-density-step);padding:calc(var(--mjx-density-gutter) * 2);
             color:var(--theme-text-primary)"
    >
      <p style="margin:0">
        A task pane’s edge sits against the document’s page, and a page may be any colour a person
        chooses. It is therefore chosen per page rather than fixed — the same rule the colour
        picker’s selection ring uses, asked about a different surface.
      </p>
      ${rows.map(
        (row) => html`
          <p style="margin:0">
            <strong>${row.scheme}</strong> — over ${String(sweep.length)} page colours the weakest
            edge the rule produces is
            ${row.worst === undefined
              ? 'nothing at all, which is itself the finding'
              : describeIndicator(
                  row.worst.indicator.member,
                  row.worst.indicator.ratio,
                  row.worst.against,
                )}.
          </p>
        `,
      )}
    </div>
  `;
}

/** The keyboard rows, shared by all three story files. */
export const surfaceKeyboard = [
  { keys: 'Tab', does: 'Moves within a modal, a sheet and a flyout; leaves a popover, which closes it; leaves a modeless dialog and a task pane into the page.' },
  { keys: 'Shift + Tab', does: 'The same, backwards. In a trapping surface it wraps from the first stop to the last.' },
  { keys: 'Escape', does: 'Closes the innermost open surface and returns focus to what opened it. A task pane ignores it.' },
  { keys: 'Enter / Space', does: 'Activates the control under the keyboard, including the close button.' },
  { keys: 'Arrow Left / Arrow Right', does: 'On a task pane’s splitter, resizes the pane by one step. Mirrored under right-to-left.' },
  { keys: 'Home / End', does: 'On a task pane’s splitter, takes the pane to its narrowest or widest.' },
];

/** What a screen reader announces. */
export const surfaceScreenReader =
  'A modal announces “dialog” with its title, and everything behind it is removed from the ' +
  'accessibility tree as well as from the tab order. A modeless dialog and a popover announce the ' +
  'same role without the modal flag, so the document behind them is still read. A task pane ' +
  'announces “complementary” with its title — never “dialog”, because the document beside it is ' +
  'never unavailable — and its splitter announces “separator” with its percentage of the screen.';
