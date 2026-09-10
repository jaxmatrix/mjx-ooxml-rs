import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  typeRoleClass,
  typeRoleNames,
  typeRoles,
  type TypeRole,
} from '../../src/foundations/typography.ts';

/**
 * The type scale.
 *
 * MJXOFF-181's trap, in its own words: **"Typography is the identity-value trap. If every story
 * renders one size at one weight, the type scale has never been exercised. Render the scale."**
 * So every story on this page renders **all six roles**, and `tests/browser/foundations.spec.ts`
 * reads the computed `font-size`, `line-height`, `font-weight` and `font-family` of each one back
 * out of the browser and asserts they are distinct and derived from the tokens.
 *
 * Two rules from `DESIGN_TOKENS.md` §4 are visible here and are enforced by tests rather than by
 * this paragraph:
 *
 * > **Nunito Sans is a rounded humanist face** and reads slightly wider than a typical UI font.
 * > Dense surfaces […] should use `--text-xs` with `--leading-tight`, not the site's relaxed 1.7.
 *
 * > **Young Serif is display-only.** It belongs in empty states and onboarding, never in the
 * > chrome or in document content.
 *
 * ## No font is fetched
 *
 * U01's decision, unchanged: `--font-sans` names *Nunito Sans* and falls back to `system-ui` until
 * a self-hosted face is added. **The fallback is what you are looking at**, and it is visible in
 * the catalogue rather than hidden behind a CDN link — which also means the *proportions* on this
 * page are right and the *shapes* are not yet.
 */

const conventions = storyConventions({
  statesMatrix: typeRoleNames.map((role) => ({
    name: role,
    description: typeRoles[role].use,
  })),
  tokenDependencies: [
    'font.sans',
    'font.serif',
    'text.xs',
    'text.sm',
    'leading.tight',
    'leading.snug',
    'leading.relaxed',
    'leading.body',
    'tracking.tight',
    'fontWeight.medium',
    'fontWeight.semibold',
    'fontWeight.bold',
    'fontWeight.extrabold',
    'theme.light.textPrimary',
    'theme.light.textSecondary',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does: 'Nothing. Typography is a class on text; a heading is not focusable and must not be made so.',
    },
  ],
  screenReader:
    'Nothing of its own. A role is a visual weight, not a semantic one — a pane title styled ' +
    'with `paneTitle` still has to be an `<h2>` for a screen reader to announce it as a heading, ' +
    'which is why these stories put the class on real elements rather than on `<div>`s.',
});

const meta: Meta = {
  title: 'Foundations/Typography',
  parameters: {
    docs: {
      description: {
        component:
          'Six roles, every one of them a var() over a generated token. Switch the Theme toolbar ' +
          'to check both schemes; switch the Container toolbar to check the scale holds at 390px.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const specimen = 'Insert a table into the document';

/**
 * The scale, one role per row, with what each is for.
 *
 * The single most useful page in this child for an auditor: six sizes, four weights and two
 * families in one view, at whatever container width the toolbar is set to.
 */
export const TheScale: Story = {
  render: () => html`
    <div
      style="display:flex;flex-direction:column;gap:var(--mjx-density-gutter);padding:var(--mjx-density-gutter)"
    >
      ${typeRoleNames.map((role: TypeRole) => {
        const spec = typeRoles[role];
        return html`
          <div
            data-type-role=${role}
            style="display:flex;flex-direction:column;gap:var(--mjx-density-step);padding:var(--mjx-density-gutter);background:var(--theme-surface);border-radius:var(--radius-card)"
          >
            <span data-specimen=${role} class=${typeRoleClass(role)}>${specimen}</span>
            <span
              class=${typeRoleClass('dense')}
              style="color:var(--theme-text-secondary);font-family:var(--font-mono)"
              >${role} — ${spec.size} / ${spec.leading} / ${spec.weight}</span
            >
            <span class=${typeRoleClass('dense')} style="color:var(--theme-text-secondary)"
              >${spec.use}</span
            >
          </div>
        `;
      })}
    </div>
  `,
};

/**
 * The three roles a dense surface uses, at the density a dense surface uses them.
 *
 * This is §4's correction made visible: `dense` is `--text-xs` on `--leading-tight`, and it is what
 * a formula bar, a cell and an inspector row are set in. Reading it beside `body` — the same face
 * at the site's relaxed 1.7 — is the whole argument for the correction.
 */
export const DenseAgainstRelaxed: Story = {
  render: () => html`
    <div
      style="display:grid;grid-template-columns:repeat(auto-fit,minmax(16rem,1fr));gap:var(--mjx-density-gutter);padding:var(--mjx-density-gutter)"
    >
      ${(['dense', 'body'] as const).map(
        (role) => html`
          <div
            data-comparison=${role}
            style="padding:var(--mjx-density-gutter);background:var(--theme-surface);border-radius:var(--radius-card)"
          >
            <p class=${typeRoleClass('label')} style="margin:0 0 var(--mjx-density-step)">${role}</p>
            <p class=${typeRoleClass(role)} style="margin:0">
              A properties inspector shows eleven rows in the space a relaxed measure gives it for
              six. That is the whole of the argument, and it is why DESIGN_TOKENS.md §4 makes the
              correction rather than leaving the source site's body measure in place.
            </p>
          </div>
        `,
      )}
    </div>
  `,
};

/**
 * Young Serif, in the one place it is allowed.
 *
 * §4 is unambiguous — *display-only, never in the chrome or in document content* — and
 * `tests/foundations.test.ts` enforces it by asserting that `display` is the only role naming the
 * serif. This story is what the rule is *for*: an empty state, where a display face does the work
 * a paragraph of apology otherwise would.
 */
export const DisplayIsForEmptyStates: Story = {
  render: () => html`
    <div
      style="display:flex;flex-direction:column;align-items:center;gap:var(--mjx-density-gutter);padding:calc(var(--mjx-density-gutter) * 4);text-align:center;background:var(--theme-surface);border-radius:var(--radius-panel)"
    >
      <mjx-icon name="document" size="48" label="Document"></mjx-icon>
      <h2 data-specimen="display-empty-state" class=${typeRoleClass('display')} style="margin:0">
        Nothing open yet
      </h2>
      <p class=${typeRoleClass('body')} style="margin:0;max-inline-size:36ch">
        Open a document, a workbook or a deck to begin. This is the only kind of surface Young
        Serif belongs on.
      </p>
    </div>
  `,
};
