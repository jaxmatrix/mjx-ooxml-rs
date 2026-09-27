/**
 * The states-matrix grid, and the token-dependency list — both **derived**, neither typed twice.
 *
 * Four story files show a states matrix, and a matrix that is written out by hand in each of them
 * is four lists that drift. So the cells come from `componentStateMatrix`, the captions and the
 * descriptions from `controlStateSpecs`, and the token-dependency list from the *paints* those
 * states actually use. A state added to the table appears in every matrix, in every story's
 * documentation, and in the blast-radius list, without anybody remembering to add it.
 *
 * ## The one string that is written twice, and why it cannot rot
 *
 * `data-state-cell` is spelled here and imported as `stateCellAttribute` by the gate, because
 * lit's `html` cannot interpolate an *attribute name*. If the two ever diverged the gate would find
 * no cells — so it asserts it found **exactly** `componentStateMatrix[archetype].length` of them,
 * and a divergence fails with the count rather than passing with an empty sweep. That is the
 * MJXOFF-181 lesson about a byte budget that measured 342 bytes of a 14 kB payload, applied to a
 * selector.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import {
  componentStateMatrix,
  controlStateSpecs,
  type ControlArchetype,
  type ControlState,
} from '../../src/controls/control-states.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/** One cell: the control in a state, and the state's name under it. */
export interface StateCell {
  readonly state: ControlState;
  readonly control: TemplateResult;
}

const gridStyle =
  'display:grid;grid-template-columns:repeat(auto-fit,minmax(9rem,1fr));' +
  'gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);' +
  'align-items:start;justify-items:center';

const captionStyle = 'color:var(--theme-text-secondary);margin:0;text-align:center';

/** The whole matrix, one cell per state, in the order the state table declares them. */
export function statesMatrix(cells: readonly StateCell[]): TemplateResult {
  return html`
    <div style=${gridStyle}>
      ${cells.map(
        (cell) => html`
          <div
            data-state-cell=${cell.state}
            title=${controlStateSpecs[cell.state].description}
            style="display:grid;gap:var(--mjx-density-step);justify-items:center"
          >
            ${cell.control}
            <p class=${typeRoleClass('dense')} style=${captionStyle}>${cell.state}</p>
          </div>
        `,
      )}
    </div>
  `;
}

/** A row of related specimens, for the stories that are not the matrix. */
export function specimenRow(children: readonly TemplateResult[]): TemplateResult {
  return html`
    <div
      style="display:flex;flex-wrap:wrap;align-items:flex-start;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${children}
    </div>
  `;
}

/** A labelled specimen, so an auditor can tell two shapes apart without reading the source. */
export function specimen(caption: string, control: TemplateResult): TemplateResult {
  return html`
    <div style="display:grid;gap:var(--mjx-density-step);justify-items:center">
      ${control}
      <p class=${typeRoleClass('dense')} style=${captionStyle}>${caption}</p>
    </div>
  `;
}

/** The states matrix as the story conventions want it: a name and a description per row. */
export function statesFor(
  archetype: ControlArchetype,
): readonly { name: string; description: string }[] {
  return componentStateMatrix[archetype].map((state) => ({
    name: state,
    description: controlStateSpecs[state].description,
  }));
}

/**
 * Everything else a control reads, beyond the paints its states name.
 *
 * The type role, the radius, the density step and gutter and the motion role are the same for all
 * four archetypes, so they are listed once here rather than four times.
 */
const sharedTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.outSoft',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'radius.control',
  'spacing',
  'text.sm',
];

/**
 * The blast radius of a token change, **computed from the state table**.
 *
 * A hand-written list is exactly the thing `storyConventions` warns about — *"a list nobody checks
 * is worse than no list, because a reader trusts it"* — so this walks the paints of the states the
 * archetype actually claims. The scheme is named `light` because a dependency is on the token
 * *pair*, and the light member is how the generated table spells the pair's name; the `dark` twin
 * moves with it by construction.
 */
export function tokenDependenciesFor(archetype: ControlArchetype): readonly TokenPath[] {
  const members = new Set<string>();
  for (const state of componentStateMatrix[archetype]) {
    const spec = controlStateSpecs[state];
    for (const paint of [spec.background, spec.borderColor, spec.text, spec.insetRing]) {
      if (paint !== undefined && paint !== 'transparent') members.add(paint);
    }
  }
  // Every control also carries the focus ring, whose colour is the pressed accent.
  members.add('accentPressed');
  const paths = [...members].map((member) => `theme.light.${member}` as TokenPath);
  return [...paths, ...sharedTokenDependencies].sort((left, right) => left.localeCompare(right));
}
