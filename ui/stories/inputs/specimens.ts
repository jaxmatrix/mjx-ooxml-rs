/**
 * The input specimens, and the lists that are **derived rather than typed twice**.
 *
 * `matrix.ts` and `stories/menus/specimens.ts` establish the rule and this follows it: the cells
 * come from the state tables, the captions from their descriptions, and the token-dependency lists
 * from the paints those states actually use. A state added to a table appears in the catalogue, in
 * the documentation and in the blast-radius list without anybody remembering.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  boxStateNames,
  boxStates,
  fieldStateNames,
  fieldStates,
  optionStateNames,
  optionStates,
  paintSpecOf,
  sliderPaints,
  type OptionDescriptor,
} from '../../src/inputs/index.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

const captionStyle = 'color:var(--theme-text-secondary);margin:0';

const gridStyle =
  'display:grid;grid-template-columns:repeat(auto-fit,minmax(16rem,1fr));' +
  'gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);' +
  'align-items:start';

const stackStyle =
  'display:grid;gap:calc(var(--mjx-density-gutter) * 2);' +
  'padding:calc(var(--mjx-density-gutter) * 2);max-inline-size:32rem';

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:60ch"
    >
      ${text}
    </p>
  `;
}

/** A stage a popup can be placed inside and clipped by. */
export function stage(height: string): string {
  return (
    `position:relative;block-size:${height};padding:calc(var(--mjx-density-gutter) * 2);` +
    'border:1px solid var(--theme-border-subtle);border-radius:var(--radius-card);' +
    'margin:calc(var(--mjx-density-gutter) * 2);overflow:auto'
  );
}

/** A column of fields, which is what a task pane actually is. */
export function stack(...children: TemplateResult[]): TemplateResult {
  return html`<div style=${stackStyle}>${children}</div>`;
}

// ── the states matrices ──────────────────────────────────────────────────────

/**
 * The field states, one specimen each.
 *
 * `focus` is never forced: `:focus-visible` is the browser's own judgement about how focus
 * arrived, and a story that asserted it into existence would be auditing a picture of a ring.
 * `unavailable` and `disabled` are real attributes rather than forced states, for the same reason.
 */
export function fieldStatesMatrix(): TemplateResult {
  const specimen = (state: string): TemplateResult => {
    switch (state) {
      case 'rest':
        return html`<mjx-measure-input label="Left indent" value="12"></mjx-measure-input>`;
      case 'hover':
        return html`<mjx-measure-input
          label="Left indent"
          value="12"
          force-field-state="hover"
        ></mjx-measure-input>`;
      case 'editing':
        return html`<mjx-measure-input
          label="Left indent"
          value="12"
          force-field-state="editing"
        ></mjx-measure-input>`;
      case 'invalid':
        return html`<mjx-measure-input
          label="Left indent"
          value="12"
          force-field-state="invalid"
        ></mjx-measure-input>`;
      case 'unavailable':
        return html`<mjx-measure-input
          label="Left indent"
          value="12"
          unavailable
          explanation="The paragraph inherits its indent from its style."
        ></mjx-measure-input>`;
      case 'disabled':
        return html`<mjx-measure-input label="Left indent" value="12" disabled></mjx-measure-input>`;
      default:
        return html`<mjx-measure-input label="Left indent" value="12"></mjx-measure-input>`;
    }
  };

  return html`
    <div style=${gridStyle}>
      ${fieldStateNames.map(
        (state) => html`
          <div
            data-field-state-cell=${state}
            title=${fieldStates[state].description}
            style="display:grid;gap:var(--mjx-density-step)"
          >
            ${specimen(state)}
            <p class=${typeRoleClass('dense')} style=${captionStyle}>${state}</p>
          </div>
        `,
      )}
    </div>
  `;
}

/** The four positions of the checkbox's square, plus the row states around them. */
export function boxStatesMatrix(): TemplateResult {
  const specimen = (state: string): TemplateResult => {
    switch (state) {
      case 'rest':
        return html`<mjx-checkbox label="Ruler"></mjx-checkbox>`;
      case 'hover':
        return html`<mjx-checkbox label="Ruler" force-state="hover"></mjx-checkbox>`;
      case 'checked':
        return html`<mjx-checkbox label="Ruler" checked="true"></mjx-checkbox>`;
      case 'mixed':
        return html`<mjx-checkbox label="Bold" checked="mixed"></mjx-checkbox>`;
      default:
        return html`<mjx-checkbox label="Ruler"></mjx-checkbox>`;
    }
  };

  return html`
    <div style=${gridStyle}>
      ${boxStateNames.map(
        (state) => html`
          <div
            data-box-state-cell=${state}
            title=${boxStates[state].description}
            style="display:grid;gap:var(--mjx-density-step)"
          >
            ${specimen(state)}
            <p class=${typeRoleClass('dense')} style=${captionStyle}>${state}</p>
          </div>
        `,
      )}
    </div>
  `;
}

// ── the option lists ─────────────────────────────────────────────────────────

/** A short list of real values, with one unavailable member and one second line. */
export const lineSpacingOptions: readonly OptionDescriptor[] = [
  { value: '1.0', label: 'Single' },
  { value: '1.15', label: '1.15', description: 'Word’s own default since 2007' },
  { value: '1.5', label: '1.5 lines' },
  { value: '2.0', label: 'Double' },
  {
    value: 'exactly',
    label: 'Exactly…',
    unavailable: true,
    explanation: 'The paragraph inherits its spacing from its style.',
  },
];

/** The alignment set, which is what a segmented control is for. */
export const alignmentSegments: readonly OptionDescriptor[] = [
  { value: 'left', label: 'Left' },
  { value: 'center', label: 'Centre' },
  { value: 'right', label: 'Right' },
  { value: 'justify', label: 'Justify' },
];

const families = [
  'Aptos',
  'Arial',
  'Bahnschrift',
  'Bookman Old Style',
  'Cambria',
  'Candara',
  'Cascadia Mono',
  'Century Gothic',
  'Comic Sans MS',
  'Consolas',
  'Constantia',
  'Corbel',
  'Courier New',
  'Ebrima',
  'Franklin Gothic',
  'Gabriola',
  'Garamond',
  'Georgia',
  'Impact',
  'Ink Free',
  'Javanese Text',
  'Leelawadee UI',
  'Lucida Console',
  'Malgun Gothic',
  'Marlett',
  'Microsoft Sans Serif',
  'Nirmala UI',
  'Palatino Linotype',
  'Rockwell',
  'Segoe UI',
  'Sitka Text',
  'Sylfaen',
  'Tahoma',
  'Times New Roman',
  'Trebuchet MS',
  'Verdana',
  'Wingdings',
];

/**
 * A font list long enough that virtualisation is the difference between a list and a wall.
 *
 * Deliberately larger than the eight rows a list shows and larger than the window plus its
 * overscan, so `builtRowCount` is meaningfully smaller than `options.length` — an assertion that a
 * twelve-item list would satisfy by accident.
 */
export const fontOptions: readonly OptionDescriptor[] = families.map((family, index) => ({
  value: family.toLowerCase().replaceAll(' ', '-'),
  label: family,
  category: index < 3 ? 'Theme fonts' : 'All fonts',
  ...(family === 'Wingdings'
    ? { unavailable: true, explanation: 'The printer does not have it.' }
    : {}),
}));

/** How many options the long list carries, so a gate names a number rather than a length. */
export const fontOptionCount = fontOptions.length;

// ── the token dependencies ───────────────────────────────────────────────────

const sharedTokenDependencies: readonly TokenPath[] = [
  'duration.transition',
  'ease.outSoft',
  'ease.spring',
  'fontWeight.bold',
  'fontWeight.medium',
  'leading.snug',
  'leading.tight',
  'radius.chip',
  'radius.control',
  'radius.panel',
  'shadow.lift',
  'spacing',
  'text.sm',
  'text.xs',
];

function pathsFor(members: Iterable<string>): TokenPath[] {
  return [...new Set(members)].map((member) => `theme.light.${member}` as TokenPath);
}

function membersOfTable(table: Readonly<Record<string, { readonly paint: unknown }>>): string[] {
  const members: string[] = [];
  for (const entry of Object.values(table)) {
    const spec = paintSpecOf(entry as Parameters<typeof paintSpecOf>[0]);
    for (const paint of [spec.background, spec.borderColor, spec.text, spec.insetRing]) {
      if (paint !== undefined && paint !== 'transparent') members.push(paint);
    }
  }
  return members;
}

/** The blast radius of a token change, **computed from the tables** rather than listed by hand. */
export function fieldTokenDependencies(): readonly TokenPath[] {
  const members: string[] = membersOfTable(fieldStates);
  members.push('surface', 'surfaceRaised', 'textSecondary', 'accentPressed', 'secondaryAccent');
  return [...pathsFor(members), ...sharedTokenDependencies].sort((a, b) => a.localeCompare(b));
}

/** The same, for the checkbox's square and the row around it. */
export function boxTokenDependencies(): readonly TokenPath[] {
  const members: string[] = membersOfTable(boxStates);
  members.push('borderSubtle', 'border', 'textPrimary', 'accentPressed');
  return [...pathsFor(members), ...sharedTokenDependencies].sort((a, b) => a.localeCompare(b));
}

/** The same, for a list field: its own field plus every option paint. */
export function listTokenDependencies(): readonly TokenPath[] {
  const members: string[] = [...membersOfTable(fieldStates), ...membersOfTable(optionStates)];
  members.push('surfaceRaised', 'border', 'borderSubtle', 'textSecondary', 'accentPressed');
  return [...pathsFor(members), ...sharedTokenDependencies].sort((a, b) => a.localeCompare(b));
}

/** The same, for the slider, from the paints it actually draws. */
export function sliderTokenDependencies(): readonly TokenPath[] {
  const members: string[] = [...Object.values(sliderPaints)];
  members.push('textPrimary', 'accentPressed');
  return [...pathsFor(members), ...sharedTokenDependencies].sort((a, b) => a.localeCompare(b));
}

/** The same, for a label: the only control here that is text and nothing else. */
export function labelTokenDependencies(): readonly TokenPath[] {
  return [
    ...pathsFor(['textPrimary', 'textSecondary', 'secondaryAccent']),
    'fontWeight.bold',
    'leading.tight',
    'spacing',
    'text.xs',
    'tracking.tight',
  ].sort((a, b) => a.localeCompare(b)) as TokenPath[];
}

// ── the declarations ─────────────────────────────────────────────────────────

/** The field states matrix as the story conventions want it. */
export function fieldStatesFor(): readonly { name: string; description: string }[] {
  return fieldStateNames.map((state) => ({
    name: state,
    description: fieldStates[state].description,
  }));
}

/** The box states matrix as the story conventions want it. */
export function boxStatesFor(): readonly { name: string; description: string }[] {
  return boxStateNames.map((state) => ({
    name: state,
    description: boxStates[state].description,
  }));
}

/** The option states matrix as the story conventions want it. */
export function optionStatesFor(): readonly { name: string; description: string }[] {
  return optionStateNames.map((state) => ({
    name: state,
    description: optionStates[state].description,
  }));
}

/** A field's states plus the list's, which is what a list field actually shows an auditor. */
export function listStatesFor(): readonly { name: string; description: string }[] {
  return [
    ...fieldStatesFor().map((row) => ({ ...row, name: `field · ${row.name}` })),
    ...optionStatesFor().map((row) => ({ ...row, name: `option · ${row.name}` })),
  ];
}

/** The keyboard model of a listbox-shaped field, from the vocabulary the component obeys. */
export const listKeyboard = [
  { keys: 'Tab', does: 'Enters the field — the whole control is one tab stop — and from inside, leaves it and closes the list.' },
  { keys: 'Arrow Down / Arrow Up', does: 'Opens the list, and then moves the keyboard cursor through it, wrapping at both ends.' },
  { keys: 'Page Down / Page Up', does: 'Moves ten options at a time, stopping at the ends rather than wrapping.' },
  { keys: 'Enter', does: 'Commits the option the cursor is on and closes the list.' },
  { keys: 'Escape', does: 'Closes the list without committing. Pressed again, with the list closed, it puts the field back to the value it reports.' },
];

/** The dropdown's own two rows, which are the ones the combo box inverts. */
export const dropdownKeyboard = [
  ...listKeyboard,
  { keys: 'Home / End', does: 'The first and last option. This field is not a text box, so they are not caret keys.' },
  { keys: 'Space', does: 'Opens the list, and commits from inside it.' },
  { keys: 'A printable character', does: 'Opens the list and starts a type-ahead; a repeated character cycles. It does not change the value — a native select fires a change per letter, and each one would be an undo entry.' },
];

/** The combo box's, with the two rows the other way round. */
export const comboBoxKeyboard = [
  ...listKeyboard,
  { keys: 'Home / End', does: 'The start and end of the text. **The list does not take them**: this field is a text box, and a combo box that jumped to the last font when a person pressed End would be unusable.' },
  { keys: 'Any printable character', does: 'Types into the field, filters the list, opens it if it was closed, and puts the cursor on the first match.' },
];

export const checkboxKeyboard = [
  { keys: 'Tab', does: 'Reaches the control. One tab stop.' },
  { keys: 'Space', does: 'Moves to the next position. From indeterminate that is *checked*, never unchecked.' },
  { keys: 'Enter', does: 'The same, because the platform’s button activates on both.' },
];

export const sliderKeyboard = [
  { keys: 'Tab', does: 'Reaches the track, which is the focusable element.' },
  { keys: 'Arrow Right / Arrow Up', does: 'One step up. Under right-to-left, Arrow **Left** is the one that increases — the inline axis mirrors and the block axis never does.' },
  { keys: 'Arrow Left / Arrow Down', does: 'One step down, mirrored the same way.' },
  { keys: 'Page Up / Page Down', does: 'A tenth of the range, rounded onto a step boundary and never smaller than one step.' },
  { keys: 'Home / End', does: 'The minimum and the maximum, **exactly** — including when the maximum is not on a step boundary.' },
];

export const segmentedKeyboard = [
  { keys: 'Tab', does: 'Enters the group at whichever segment is chosen, and leaves it. One tab stop for the whole group.' },
  { keys: 'Arrow keys', does: 'Move **and choose**, wrapping at both ends — which is what a radio group does and is the difference from a listbox. Both axes are bound.' },
  { keys: 'Home / End', does: 'The first and last segment.' },
];

export const measureKeyboard = [
  { keys: 'Tab', does: 'Reaches the field, and commits what is in it on the way out.' },
  { keys: 'Arrow Up / Arrow Down', does: 'One step, **in the unit the field is showing** — a centimetres field moves by a centimetre-sized step, never by 1/28th of one.' },
  { keys: 'Enter', does: 'Commits what was typed, or refuses it and says why.' },
  { keys: 'Escape', does: 'Back to the value the field reports, and out of the invalid state. It is the only way out of it.' },
];

export const labelKeyboard = [
  { keys: 'Tab', does: 'Never stops here. A label is not focusable, which is itself the audit finding.' },
  { keys: 'A pointer press', does: 'Focuses the control it names, which is what a native <label> would have given.' },
];

// ── what a screen reader hears ───────────────────────────────────────────────

export const dropdownScreenReader =
  'The field announces its name, that it is a combo box, and whether it is collapsed or expanded. ' +
  'Opening it announces the list and how many options it holds; each arrow key announces the ' +
  'option’s label and its position — “Cambria, 5 of 18” — because focus never leaves the field and ' +
  'aria-activedescendant is what points at the row. The chosen option announces “selected”. An ' +
  'unavailable option announces “dimmed” and reads its reason, and is still reached by the arrows.';

export const comboBoxScreenReader =
  'The same as the dropdown, with aria-autocomplete="list" added, so a screen reader says that ' +
  'typing will narrow the list. Filtering re-announces how many options remain. Committing a ' +
  'string the list does not carry announces nothing new — the field simply reports what it ' +
  'reports, which is why the refusal is also an event a host can speak.';

export const checkboxScreenReader =
  'Announced as a check box with its name and one of three positions: checked, unchecked or ' +
  '**partially checked**, which is what aria-checked="mixed" is read as. An unavailable one ' +
  'announces “dimmed” and reads its reason.';

export const sliderScreenReader =
  'Announced as a slider with its name, its current value and the ends of its range. ' +
  'aria-valuetext is what turns 100 into “100 percent”; without it the number is announced with ' +
  'no sense of what it measures. Every key press re-announces the new value.';

export const segmentedScreenReader =
  'Announced as a radio group with its name, then the chosen member and its position in the set — ' +
  '“Centre, radio button, checked, 2 of 4”. Moving with the arrows both moves and chooses, so ' +
  'every announcement is of a selection that has been made.';

export const measureScreenReader =
  'Announced as an edit field with its name and its contents, unit and all. A field holding text ' +
  'it could not read announces “invalid entry” and then reads the message beneath it, which is ' +
  'associated through aria-errormessage and aria-describedby together — the second because it is ' +
  'the one every screen reader announces today.';

export const labelScreenReader =
  'A label is not announced as an object of its own. Its text becomes the accessible **name** of ' +
  'the control it points at, so a screen reader reads “Font size, edit, 12 pt” rather than reading ' +
  'a caption and then an unnamed field. A required field has the word “required” after its name, ' +
  'off-screen beside the asterisk, because an asterisk on its own is read as “star”.';
