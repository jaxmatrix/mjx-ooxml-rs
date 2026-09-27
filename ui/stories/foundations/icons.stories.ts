import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { iconRequests, requestedIconIds, type IconSize } from '../../src/icons/manifest.ts';
import { subsetGlyphCount, subsetPathBytes } from '../../src/icons/generated.ts';
import { iconSizes } from '../../src/icons/manifest.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * The icon subset, entire.
 *
 * MJXOFF-181 chose Fluent UI System Icons because they are *Office's own icon language* and MIT
 * licensed. The package ships 20,679 files; this catalogue ships
 * {@link subsetGlyphCount} of them, and the number is asserted rather than described —
 * `tests/browser/icons.spec.ts` reads the built bundle and requires the set of icon ids in it to
 * **equal** the set `src/icons/manifest.ts` asks for.
 *
 * ## What these stories are for
 *
 * The trap the ticket names is that *"an icon subset is satisfied by any SVG"*. A story showing one
 * icon proves an `<svg>` renders. So these stories render **the whole subset**, at **every size it
 * was requested at**, in **both variants** — because a catalogue that shows a designer one icon at
 * one size is a catalogue nobody can choose from, and a story that renders one icon is a story that
 * would keep passing if the other sixty-eight stopped resolving.
 *
 * Nothing here names an icon in a string. Every attribute is built from `iconRequests`, so a row
 * removed from the manifest disappears from the catalogue and a row added appears in it, and the
 * stories cannot drift from the subset.
 */

const conventions = storyConventions({
  statesMatrix: [
    {
      name: 'resolved',
      description:
        'The subset carries the drawing. `data-icon-state="resolved"`, and the box is exactly the requested size.',
    },
    {
      name: 'unknown',
      description:
        'The subset does not carry it. Nothing renders, `data-icon-state="unknown"`, and the console says which id was missed. Deliberately not a fallback icon: a wrong icon is worse than a missing one, because a person acts on it.',
    },
    {
      name: 'decorative',
      description:
        'No `label`. `aria-hidden="true"`, no role — the ordinary case, because most icons sit inside something already named.',
    },
    {
      name: 'labelled',
      description: 'A `label`. `role="img"` and an `aria-label`, for an icon that carries meaning alone.',
    },
    {
      name: 'tinted',
      description: 'Fills with `currentColor`, so it takes the colour of the text it sits beside.',
    },
  ],
  tokenDependencies: [
    'theme.light.textPrimary',
    'theme.light.textSecondary',
    'theme.light.accent',
    'theme.light.accentPressed',
    'text.xs',
    'spacing',
    'radius.control',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does: 'Nothing. An icon is never focusable; the control around it is. That is a behaviour, and this row is how it is declared.',
    },
  ],
  screenReader:
    'A decorative icon announces nothing at all. A labelled one announces its label, as an image.',
});

const meta: Meta = {
  title: 'Foundations/Icons',
  parameters: {
    docs: {
      description: {
        component:
          'The Fluent subset, entire. Every attribute on this page is built from ' +
          'src/icons/manifest.ts, so what you see is exactly what the bundle carries.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const grid = (children: unknown) => html`
  <div
    style="display:grid;grid-template-columns:repeat(auto-fill,minmax(9rem,1fr));gap:var(--mjx-density-gutter);padding:var(--mjx-density-gutter)"
  >
    ${children}
  </div>
`;

const cell = (caption: string, body: unknown) => html`
  <div
    style="display:flex;flex-direction:column;align-items:center;gap:var(--mjx-density-step);padding:var(--mjx-density-step);border-radius:var(--radius-control);background:var(--theme-surface)"
  >
    ${body}
    <span class=${typeRoleClass('dense')} style="color:var(--theme-text-secondary);text-align:center"
      >${caption}</span
    >
  </div>
`;

/**
 * Every glyph in the subset, at every size and in every variant the manifest asked for.
 *
 * This is the story that would notice sixty-eight icons quietly failing to resolve, and the reason
 * it is first.
 */
export const TheWholeSubset: Story = {
  render: () =>
    grid(
      iconRequests.flatMap((request) =>
        request.sizes.flatMap((size) =>
          request.variants.map((variant) =>
            cell(
              `${request.name} · ${String(size)} · ${variant}`,
              html`<mjx-icon
                name=${request.name}
                size=${String(size)}
                variant=${variant}
                label=${`${request.name}, ${variant}, ${String(size)} pixels`}
              ></mjx-icon>`,
            ),
          ),
        ),
      ),
    ),
};

/**
 * The size ladder, on the one icon carried at all five sizes.
 *
 * Fluent draws each size separately — its own stroke weights, its own optical corrections — so
 * these are five drawings and not one drawing scaled. Putting them side by side is the only way to
 * see that, and it is why the manifest spends five rows on a single icon.
 */
export const TheSizeLadder: Story = {
  render: () => {
    const ladder = iconRequests.find(
      (request) => request.sizes.length === iconSizes.length,
    );
    if (ladder === undefined) return html`<p>No icon in the manifest is carried at every size.</p>`;
    return html`
      <div
        style="display:flex;align-items:flex-end;gap:var(--mjx-density-gutter);padding:var(--mjx-density-gutter)"
      >
        ${iconSizes.map(
          (size: IconSize) => html`
            <div style="display:flex;flex-direction:column;align-items:center;gap:var(--mjx-density-step)">
              <mjx-icon
                name=${ladder.name}
                size=${String(size)}
                label=${`${ladder.name} at ${String(size)} pixels`}
              ></mjx-icon>
              <span class=${typeRoleClass('dense')} style="color:var(--theme-text-secondary)"
                >${String(size)}</span
              >
            </div>
          `,
        )}
      </div>
    `;
  },
};

/**
 * Regular against filled, on every icon that asked for both.
 *
 * Fluent's convention, adopted here: `filled` is a *state*, not a style. A toggle that is on and a
 * radio member that is chosen are filled; a resting command is not. An icon that asked for only one
 * variant is a command that is never in a state.
 */
export const RegularAgainstFilled: Story = {
  render: () =>
    grid(
      iconRequests
        .filter((request) => request.variants.length > 1)
        .map((request) => {
          const size = request.sizes.at(-1) ?? 20;
          return cell(
            request.name,
            html`
              <div style="display:flex;gap:var(--mjx-density-gutter);align-items:center">
                <mjx-icon
                  name=${request.name}
                  size=${String(size)}
                  variant="regular"
                  label=${`${request.name}, resting`}
                ></mjx-icon>
                <mjx-icon
                  name=${request.name}
                  size=${String(size)}
                  variant="filled"
                  label=${`${request.name}, selected`}
                  style="color:var(--theme-accent-pressed)"
                ></mjx-icon>
              </div>
            `,
          );
        }),
    ),
};

/**
 * Tinting: the paths are filled with `currentColor` and nothing else.
 *
 * That one decision is what lets an icon inside a pressed button turn the pressed colour with no
 * icon-specific rule anywhere in the system. Each row below sets only `color`.
 */
export const TintedFromTokens: Story = {
  render: () => {
    const tints = [
      { token: '--theme-text-primary', label: 'text-primary — the resting chrome icon' },
      { token: '--theme-text-secondary', label: 'text-secondary — a de-emphasised row' },
      { token: '--theme-accent-pressed', label: 'accent-pressed — a chosen command' },
      { token: '--theme-secondary-accent', label: 'secondary-accent — a warning mark' },
    ];
    const sample = iconRequests[0];
    if (sample === undefined) return html``;
    return html`
      <div
        style="display:flex;flex-direction:column;gap:var(--mjx-density-step);padding:var(--mjx-density-gutter)"
      >
        ${tints.map(
          (tint) => html`
            <div
              class=${typeRoleClass('control')}
              style="display:flex;align-items:center;gap:var(--mjx-density-step);color:var(--theme-text-primary)"
            >
              <!-- Only the icon takes the tint. The caption stays text-primary on purpose: two of
                   these tokens are tagged fill-only in DESIGN_TOKENS.md §2.2 and painting a label
                   in one would make this story fail the contrast gate — correctly, and for a
                   reason that has nothing to do with icons. -->
              <span style=${`display:inline-flex;color:var(${tint.token})`}>
                <mjx-icon name=${sample.name} size="24"></mjx-icon>
              </span>
              <span>${tint.label}</span>
            </div>
          `,
        )}
      </div>
    `;
  },
};

/**
 * An icon the subset does not carry.
 *
 * The ticket asks for a subset gate that is *"about the subset being the right one and staying
 * it — assert the set, and that an icon not in the subset cannot be referenced."* This story is
 * the visible half of that: nothing renders, and `data-icon-state` says `unknown`.
 *
 * It is deliberately **not** an accessibility violation and not a thrown error. A chrome that fails
 * to open because one toolbar button named an icon nobody added would be a worse outcome than a
 * gap; the thing that turns this into a build failure is `tests/browser/icons.spec.ts`, which is
 * where a build failure belongs.
 */
export const AnIconOutsideTheSubset: Story = {
  render: () => html`
    <div
      class=${typeRoleClass('control')}
      style="display:flex;flex-direction:column;gap:var(--mjx-density-step);padding:var(--mjx-density-gutter)"
    >
      <div style="display:flex;align-items:center;gap:var(--mjx-density-step)">
        <mjx-icon
          data-probe="unknown-name"
          name="not-an-icon-in-the-subset"
          size="24"
        ></mjx-icon>
        <span>A name no manifest row asks for. Nothing is drawn.</span>
      </div>
      <div style="display:flex;align-items:center;gap:var(--mjx-density-step)">
        <mjx-icon
          data-probe="unknown-size"
          name=${iconRequests[1]?.name ?? ''}
          size="48"
        ></mjx-icon>
        <span>A real name at a size the manifest did not ask for. Also nothing.</span>
      </div>
    </div>
  `,
};

/** What the subset costs, stated from the generated numbers rather than from memory. */
export const TheSubsetLedger: Story = {
  render: () => html`
    <div
      class=${typeRoleClass('body')}
      style="padding:var(--mjx-density-gutter);display:flex;flex-direction:column;gap:var(--mjx-density-step)"
    >
      <p style="margin:0">
        <strong>${String(subsetGlyphCount)}</strong> glyphs from
        <strong>${String(iconRequests.length)}</strong> icons, carrying
        <strong>${String(subsetPathBytes)}</strong> bytes of path data — subset from the 20,679
        files <code>@fluentui/svg-icons</code> ships.
      </p>
      <p style="margin:0">
        <code>tests/browser/icons.spec.ts</code> requires the ids in the built bundle to equal
        these ${String(requestedIconIds().length)} exactly, and holds the bundle's total path data
        under a budget derived from that number.
      </p>
      <p style="margin:0">
        Fluent UI System Icons are MIT licensed, © Microsoft Corporation — the licence is at
        <code>ui/src/icons/LICENSE-fluent.txt</code>.
      </p>
    </div>
  `,
};
