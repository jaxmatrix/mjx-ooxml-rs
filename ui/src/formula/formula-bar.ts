/**
 * `<mjx-formula-bar>` — **the most-used control in Excel, and it appears in no ribbon census**,
 * because it is not a ribbon control.
 *
 * ```html
 * <mjx-formula-bar label="Formula" value="=SUM(A1:A9)">
 *   <mjx-name-box slot="name-box" address="B7"></mjx-name-box>
 * </mjx-formula-bar>
 * ```
 *
 * ## Two layers, and why the text you can see is not the text you are typing in
 *
 * The editor is a `<textarea>` whose own glyphs are **transparent**, laid over a `<div>` that draws
 * the same characters in colour. The alternative — a `contenteditable` with coloured spans — was
 * rejected, and not on taste:
 *
 * * the caret, the selection, undo, redo, IME composition and every platform text gesture are the
 *   `<textarea>`'s for free, and are each a re-implementation in the other design;
 * * `selectionStart` is a number, where a `contenteditable`'s caret is a `Range` inside a shadow
 *   root and **`ShadowRoot.getSelection()` is not a standard** — Chromium has it, other engines do
 *   not, and every caret-relative behaviour in this child would have rested on it;
 * * the browser cannot paste markup into a `<textarea>`.
 *
 * What the arrangement costs is an alignment invariant: the two boxes must lay text out
 * identically or the caret sits beside its own glyph. That is not left to care —
 * `alignedTextProperties` names the properties, both rules are written from that one list, and
 * `tests/browser/formula.spec.ts` compares the two through `getComputedStyle`.
 *
 * ⚠ **One honest consequence, found rather than assumed.** axe reports the contrast of both layers
 * as `incomplete` rather than as a pass — a transparent foreground over a positioned sibling is a
 * case its algorithm declines to judge — so the a11y sweep does **not** measure the reference
 * colours. They are measured in `tests/formula.test.ts` instead, against both schemes' surfaces,
 * which is a stronger check than the sweep would have made: the sweep only ever runs in light.
 *
 * ## What is ARIA-legal here, checked rather than assumed
 *
 * The autocomplete is *not* announced as a combobox, and that is a finding rather than a shortcut:
 * `<textarea role="combobox">` fails axe's `aria-allowed-role`, and `aria-expanded` on a textbox
 * fails `aria-allowed-attr` — both confirmed against axe-core before this component was written.
 * What a textarea *may* carry is `aria-controls` and **`aria-activedescendant`**, which it does, so
 * the active completion is announced as the arrow keys move through it. That the list opened at all
 * is said by the live region, in words, because the attribute that would normally say it is illegal
 * here.
 *
 * ## The four modes are drawn twice
 *
 * `Ready`, `Enter`, `Edit` and `Point` differ in a fill **and** in the shape of the chip's mark — a
 * ring, a dot, a bar, a diamond — because a state told only in colour is a state some readers
 * cannot read. They are also announced, in a sentence rather than a word, because the difference a
 * person needs is *what the next arrow key will do*.
 *
 * ## No evaluation, anywhere
 *
 * There is no calculation engine in this loop. This component tokenises, colours, matches brackets
 * and says which argument the caret is in; it has no way to produce a value and nowhere to put one.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { defineIcon } from '../icons/icon.ts';
import { ListSurface, listAlign, listSide } from '../inputs/list-surface.ts';
import type { OptionDescriptor } from '../inputs/input-model.ts';
import {
  applyPlacement,
  clearPlacement,
  clippingBoundary,
  floatingCss,
  floatingProperties,
  installFloatingProperties,
  placeFloating,
  rectOf,
  resolveLength,
  type Direction,
} from '../overlay/floating.ts';
import {
  activeArgument,
  applyCompletion,
  applyModeTrigger,
  bracketAt,
  bracketReport,
  clampFormulaRows,
  completionPrefix,
  completionsFor,
  emphasisedParameter,
  formulaEvents,
  formulaFunctions,
  formulaModes,
  formulaRowBounds,
  isFormula,
  pointReady,
  readyModeState,
  referenceColourSlots,
  referenceHighlights,
  rowsAfterDrag,
  rowsAfterKey,
  signatureFor,
  tokeniseFormula,
  type ActiveArgument,
  type BracketPair,
  type FormulaMode,
  type FormulaModeState,
  type FunctionSignature,
  type ReferenceHighlight,
} from './formula-model.ts';
import { formulaBarCss, formulaMotionClass, formulaTypeRoles } from './formula-sheets.ts';

/** The three affordances between the name box and the editor, and what each is called. */
export const affordanceLabels = {
  cancel: 'Cancel',
  confirm: 'Enter',
  insert: 'Insert function',
} as const;

/** The two icons. `fx` is drawn as text, because Excel's is a letterform rather than a glyph. */
export const affordanceIcons = {
  cancel: { name: 'dismiss', size: 16 },
  confirm: { name: 'checkmark', size: 16 },
} as const;

/** What the drag handle is called, and how it announces its value. */
export const handleLabel = 'Formula bar height, in rows';

/** How many completions are offered at once before the list scrolls. U07's own cap, unchanged. */
export { listboxVisibleRows as completionVisibleRows } from '../inputs/input-model.ts';

let nextBarSerial = 0;

export class MjxFormulaBar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'value', 'rows', 'disabled'];

  #root: ShadowRoot | undefined;
  #bar: HTMLElement | undefined;
  #entry: HTMLTextAreaElement | undefined;
  #backdrop: HTMLElement | undefined;
  #handle: HTMLElement | undefined;
  #modeChip: HTMLElement | undefined;
  #modeText: HTMLElement | undefined;
  #tooltip: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #buttons = new Map<keyof typeof affordanceLabels, HTMLButtonElement>();
  #surface: ListSurface | undefined;

  #catalogue: readonly FunctionSignature[] = formulaFunctions;
  #modeState: FormulaModeState = readyModeState;
  #caret = 0;
  /** The text as it was when the edit began, which is what a cancel restores. */
  #textOnEdit = '';
  #completions: readonly FunctionSignature[] = [];
  #prefix: { readonly text: string; readonly from: number } | undefined;
  #dragFrom: { readonly rows: number; readonly y: number } | undefined;
  #instance = `mjx-formula-${String((nextBarSerial += 1))}`;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.ownerDocument.addEventListener('pointerdown', this.#onDocumentDown, true);
    this.render();
  }

  disconnectedCallback(): void {
    this.ownerDocument.removeEventListener('pointerdown', this.#onDocumentDown, true);
    this.#surface?.dispose();
    this.#surface = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root === undefined) return;
    this.render();
  }

  // ── the surface ────────────────────────────────────────────────────────────

  /** What the editor is called. */
  get label(): string {
    return this.getAttribute('label') ?? 'Formula';
  }

  /** The text in the editor — a formula or a literal value. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** How tall the editor is, in rows. */
  get rows(): number {
    return clampFormulaRows(Number(this.getAttribute('rows') ?? formulaRowBounds.min));
  }

  set rows(next: number) {
    this.setAttribute('rows', String(clampFormulaRows(next)));
  }

  /** Where the caret is, as an offset into `value`. */
  get caret(): number {
    return this.#caret;
  }

  /** Which of the four modes the bar is in. */
  get mode(): FormulaMode {
    return this.#modeState.mode;
  }

  /** The function catalogue the autocomplete offers. Data — a host replaces it wholesale. */
  get functions(): readonly FunctionSignature[] {
    return this.#catalogue;
  }

  set functions(next: readonly FunctionSignature[]) {
    this.#catalogue = [...next];
    this.render();
  }

  /**
   * **The reference-colouring contract**, for whatever is in the editor now.
   *
   * The same value the `mjx-formula-references` event carries. Exposed as a property as well
   * because a grid that mounts after the bar has to be able to ask, rather than wait for the next
   * keystroke.
   */
  get references(): readonly ReferenceHighlight[] {
    return referenceHighlights(this.value);
  }

  /** Which argument of which call the caret is in, or `undefined`. */
  get argument(): ActiveArgument | undefined {
    return activeArgument(this.value, this.#caret);
  }

  /** The bracket pair the caret is on, or `undefined`. */
  get brackets(): BracketPair | undefined {
    return bracketAt(this.value, this.#caret);
  }

  /** Whether the autocomplete is showing. */
  get completionsOpen(): boolean {
    return this.#surface?.open === true;
  }

  /** The completions currently offered. */
  get completions(): readonly FunctionSignature[] {
    return this.#completions;
  }

  /** The completion the keyboard is on, or `undefined`. */
  get activeCompletion(): FunctionSignature | undefined {
    const index = this.#surface?.activeIndex ?? -1;
    return index < 0 ? undefined : this.#completions[index];
  }

  override focus(options?: FocusOptions): void {
    if (this.#entry === undefined) super.focus(options);
    else this.#entry.focus(options);
  }

  /** Begin an entry — a person typed into an empty cell. */
  beginEntry(): void {
    this.#toMode('beginEntry');
  }

  /** Begin an edit — F2, a double-click, or a press in the bar. */
  beginEdit(): void {
    this.#toMode('beginEdit');
  }

  /** Accept what is in the editor. */
  commit(): void {
    if (this.mode === 'ready') return;
    this.#closeCompletions();
    this.#toMode('commit');
    this.dispatchEvent(
      new CustomEvent(formulaEvents.commit, { bubbles: true, composed: true, detail: { text: this.value } }),
    );
  }

  /** Throw the edit away and put back the text the edit started from. */
  cancel(): void {
    if (this.mode === 'ready') return;
    const abandoned = this.value;
    this.#closeCompletions();
    this.value = this.#textOnEdit;
    this.#toMode('cancel');
    this.dispatchEvent(
      new CustomEvent(formulaEvents.cancel, { bubbles: true, composed: true, detail: { text: abandoned } }),
    );
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, [floatingCss, formulaBarCss].join('\n'));

    const bar = document.createElement('div');
    bar.className = 'bar';
    bar.setAttribute('part', 'bar');

    const nameSlot = document.createElement('div');
    nameSlot.className = 'name-slot';
    const slot = document.createElement('slot');
    slot.name = 'name-box';
    nameSlot.append(slot);

    const divider = document.createElement('div');
    divider.className = 'divider';
    divider.setAttribute('aria-hidden', 'true');

    const affordances = document.createElement('div');
    affordances.className = 'affordances';
    affordances.setAttribute('role', 'group');
    affordances.setAttribute('aria-label', 'Formula actions');
    for (const key of ['cancel', 'confirm', 'insert'] as const) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = `affordance ${key} mjx-hit-target`;
      button.setAttribute('part', `affordance ${key}`);
      button.setAttribute('aria-label', affordanceLabels[key]);
      if (key === 'insert') {
        button.textContent = 'fx';
      } else {
        const icon = document.createElement('mjx-icon');
        icon.setAttribute('name', affordanceIcons[key].name);
        icon.setAttribute('size', String(affordanceIcons[key].size));
        icon.setAttribute('aria-hidden', 'true');
        button.append(icon);
      }
      button.addEventListener('click', () => {
        this.#pressed(key);
      });
      this.#buttons.set(key, button);
      affordances.append(button);
    }

    const editor = document.createElement('div');
    editor.className = 'editor';

    const backdrop = document.createElement('div');
    backdrop.className = 'backdrop';
    backdrop.setAttribute('aria-hidden', 'true');

    const entry = document.createElement('textarea');
    entry.className = `entry ${formulaTypeRoles.editor}`;
    entry.setAttribute('part', 'entry');
    entry.id = `${this.#instance}-entry`;
    entry.autocomplete = 'off';
    entry.spellcheck = false;
    entry.setAttribute('autocapitalize', 'off');
    entry.setAttribute('autocorrect', 'off');
    entry.rows = this.rows;
    entry.addEventListener('input', this.#onInput);
    entry.addEventListener('keydown', this.#onKeyDown);
    entry.addEventListener('keyup', this.#syncCaret);
    entry.addEventListener('pointerup', this.#syncCaret);
    entry.addEventListener('select', this.#syncCaret);
    entry.addEventListener('focus', this.#onFocus);
    entry.addEventListener('blur', this.#onBlur);
    entry.addEventListener('scroll', this.#onScroll);
    editor.append(backdrop, entry);

    const handle = document.createElement('div');
    handle.className = 'handle mjx-hit-target';
    handle.setAttribute('part', 'handle');
    handle.setAttribute('role', 'separator');
    handle.setAttribute('aria-orientation', 'horizontal');
    handle.setAttribute('aria-label', handleLabel);
    handle.tabIndex = 0;
    const grip = document.createElement('span');
    grip.className = 'grip';
    grip.setAttribute('aria-hidden', 'true');
    handle.append(grip);
    handle.addEventListener('keydown', this.#onHandleKey);
    handle.addEventListener('pointerdown', this.#onHandleDown);
    handle.addEventListener('pointermove', this.#onHandleMove);
    handle.addEventListener('pointerup', this.#onHandleUp);
    handle.addEventListener('pointercancel', this.#onHandleUp);

    bar.append(nameSlot, divider, affordances, editor, handle);

    const chip = document.createElement('div');
    chip.className = `mode ${formulaTypeRoles.mode}`;
    chip.setAttribute('part', 'mode');
    const dot = document.createElement('span');
    dot.className = 'mode-dot';
    dot.setAttribute('aria-hidden', 'true');
    const modeText = document.createElement('span');
    chip.append(dot, modeText);

    const tooltip = document.createElement('div');
    tooltip.className = `tooltip ${formulaTypeRoles.tooltip} ${formulaMotionClass}`;
    tooltip.setAttribute('part', 'tooltip');
    tooltip.setAttribute('role', 'tooltip');
    tooltip.id = `${this.#instance}-tooltip`;
    tooltip.hidden = true;
    entry.setAttribute('aria-describedby', tooltip.id);

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('role', 'status');
    live.setAttribute('aria-live', 'polite');

    const surface = new ListSurface(
      {
        chose: (option) => {
          this.#complete(option.value);
        },
        movedActive: () => {
          this.#syncActiveDescendant();
        },
      },
      `${this.#instance}-completions`,
    );
    surface.element.id = `${this.#instance}-completions-list`;
    surface.element.setAttribute('aria-label', 'Functions');
    entry.setAttribute('aria-controls', surface.element.id);
    this.#surface = surface;

    root.append(bar, chip, tooltip, live, surface.element);

    this.#bar = bar;
    this.#entry = entry;
    this.#backdrop = backdrop;
    this.#handle = handle;
    this.#modeChip = chip;
    this.#modeText = modeText;
    this.#tooltip = tooltip;
    this.#live = live;
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-draw from the attributes and the caret. */
  render(): void {
    const entry = this.#entry;
    const chip = this.#modeChip;
    const handle = this.#handle;
    if (entry === undefined || chip === undefined || handle === undefined) return;

    entry.setAttribute('aria-label', this.label);
    if (entry.value !== this.value) entry.value = this.value;
    entry.rows = this.rows;
    entry.disabled = this.hasAttribute('disabled');
    if (this.rows > 1) entry.setAttribute('aria-multiline', 'true');
    else entry.removeAttribute('aria-multiline');

    handle.setAttribute('aria-valuenow', String(this.rows));
    handle.setAttribute('aria-valuemin', String(formulaRowBounds.min));
    handle.setAttribute('aria-valuemax', String(formulaRowBounds.max));
    handle.setAttribute('aria-valuetext', `${String(this.rows)} of ${String(formulaRowBounds.max)} rows`);

    chip.dataset['mode'] = this.mode;
    if (this.#modeText !== undefined) this.#modeText.textContent = formulaModes[this.mode].label;

    const editing = this.mode !== 'ready';
    this.#buttons.get('cancel')?.toggleAttribute('disabled', !editing);
    this.#buttons.get('confirm')?.toggleAttribute('disabled', !editing);

    this.#paintBackdrop();
    this.#paintTooltip();
  }

  /**
   * The coloured layer.
   *
   * Rebuilt from the tokens on every keystroke, which is affordable because a formula is a few
   * dozen characters and is the only way the layer cannot drift from the text: there is one source
   * for both, and it is `value`.
   */
  #paintBackdrop(): void {
    const backdrop = this.#backdrop;
    if (backdrop === undefined) return;
    const source = this.value;
    const slots = new Map<number, number>();
    for (const highlight of this.references) slots.set(highlight.from, highlight.slot);
    const pair = this.brackets;
    const unmatched = new Set(bracketReport(source).unmatched);

    const pieces: Node[] = [];
    for (const token of tokeniseFormula(source)) {
      const span = document.createElement('span');
      span.className = `token-${token.kind}`;
      const slot = slots.get(token.from);
      if (slot !== undefined) span.dataset['slot'] = String(slot);
      if (token.kind === 'openParen' || token.kind === 'closeParen') {
        if (pair !== undefined && (token.from === pair.open || token.from === pair.close)) {
          span.classList.add('bracket-match');
        }
        if (unmatched.has(token.from)) span.classList.add('bracket-unmatched');
      }
      span.textContent = token.text;
      pieces.push(span);
    }
    // ⚠ A trailing newline is invisible in a `<div>` and visible in a `<textarea>`: without this the
    // last line of a multi-line formula scrolls one line out of step with its own colouring.
    pieces.push(document.createTextNode('\n'));
    backdrop.replaceChildren(...pieces);
  }

  // ── the caret, and everything that depends on it ───────────────────────────

  #syncCaret = (): void => {
    const entry = this.#entry;
    if (entry === undefined) return;
    const caret = entry.selectionStart ?? 0;
    const moved = caret !== this.#caret;
    this.#caret = caret;
    if (this.mode !== 'ready') {
      // GUESS: Excel enters Point mode when an arrow key or a click actually names a range. This bar
      // has no grid to point at, so it reports the state the person needs to know — *what the next
      // arrow key would do* — which is exactly `pointReady`.
      this.#toMode(pointReady(this.value, caret) ? 'enterPoint' : 'leavePoint');
    }
    if (moved || this.mode !== 'ready') this.#paintBackdrop();
    this.#paintTooltip();
    this.dispatchEvent(
      new CustomEvent(formulaEvents.caret, {
        bubbles: true,
        composed: true,
        detail: { caret, argument: this.argument, brackets: this.brackets },
      }),
    );
  };

  #onInput = (): void => {
    const entry = this.#entry;
    if (entry === undefined) return;
    if (this.mode === 'ready') this.#toMode('beginEdit');
    this.setAttribute('value', entry.value);
    this.#caret = entry.selectionStart ?? entry.value.length;
    this.#emitReferences();
    this.#refreshCompletions();
    this.render();
    this.#syncCaret();
  };

  #onScroll = (): void => {
    const backdrop = this.#backdrop;
    const entry = this.#entry;
    if (backdrop === undefined || entry === undefined) return;
    backdrop.scrollTop = entry.scrollTop;
    backdrop.scrollLeft = entry.scrollLeft;
  };

  #onFocus = (): void => {
    if (this.mode === 'ready') this.#toMode('beginEdit');
    this.#textOnEdit = this.value;
    this.#syncCaret();
  };

  #onBlur = (): void => {
    this.#closeCompletions();
  };

  #emitReferences(): void {
    this.dispatchEvent(
      new CustomEvent(formulaEvents.references, {
        bubbles: true,
        composed: true,
        detail: { references: this.references, slots: referenceColourSlots },
      }),
    );
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const surface = this.#surface;

    if (this.completionsOpen && surface !== undefined) {
      switch (event.key) {
        case 'ArrowDown':
          this.#moveCompletion(1);
          event.preventDefault();
          return;
        case 'ArrowUp':
          this.#moveCompletion(-1);
          event.preventDefault();
          return;
        case 'Enter':
        case 'Tab': {
          const active = this.activeCompletion;
          if (active !== undefined) {
            this.#complete(active.name);
            event.preventDefault();
            // ⚠ Tab is stopped ONLY when it completed something. A list that swallowed Tab
            // unconditionally would be a text box a person could not leave.
            return;
          }
          break;
        }
        case 'Escape':
          this.#closeCompletions();
          // Stopped, so one Escape dismisses the list and a second one cancels the edit. Excel's
          // order, and the one that never throws away an edit a person did not mean to lose.
          event.preventDefault();
          event.stopPropagation();
          return;
        default:
          break;
      }
    }

    switch (event.key) {
      case 'Enter':
        if (event.shiftKey) return;
        this.commit();
        event.preventDefault();
        return;
      case 'Escape':
        this.cancel();
        event.preventDefault();
        return;
      default:
        return;
    }
  };

  // ── the autocomplete ───────────────────────────────────────────────────────

  #refreshCompletions(): void {
    const prefix = completionPrefix(this.value, this.#caret);
    if (prefix === undefined) {
      this.#closeCompletions();
      return;
    }
    const matches = completionsFor(prefix.text, this.#catalogue);
    if (matches.length === 0) {
      this.#closeCompletions();
      return;
    }
    this.#prefix = prefix;
    this.#openCompletions(matches);
  }

  /** Open the list, or refresh the one that is open. */
  #openCompletions(matches: readonly FunctionSignature[]): void {
    const surface = this.#surface;
    const entry = this.#entry;
    if (surface === undefined || entry === undefined) return;
    const wasOpen = surface.open;
    this.#completions = matches;
    surface.setOptions(matches.map(completionOption));
    surface.setActive(0);
    if (!wasOpen) surface.show(entry, this.#direction());
    else surface.place(entry, this.#direction());
    this.#syncActiveDescendant();
    if (!wasOpen) {
      // The attribute that would normally say a list opened is illegal on a textarea — see the
      // module note — so this is said in words instead.
      this.#announce(
        `${String(matches.length)} function${matches.length === 1 ? '' : 's'}. ` +
          `${matches[0]?.name ?? ''}. Use the arrow keys, then Tab or Enter.`,
      );
    }
  }

  #closeCompletions(): void {
    const surface = this.#surface;
    if (surface === undefined || !surface.open) return;
    surface.hide();
    this.#completions = [];
    this.#prefix = undefined;
    this.#entry?.removeAttribute('aria-activedescendant');
  }

  #moveCompletion(delta: number): void {
    const surface = this.#surface;
    if (surface === undefined) return;
    const count = this.#completions.length;
    if (count === 0) return;
    const next = (surface.activeIndex + delta + count) % count;
    surface.setActive(next);
    this.#syncActiveDescendant();
  }

  #syncActiveDescendant(): void {
    const entry = this.#entry;
    const active = this.#surface?.activeDescendantId;
    if (entry === undefined) return;
    if (active === undefined) entry.removeAttribute('aria-activedescendant');
    else entry.setAttribute('aria-activedescendant', active);
  }

  #complete(name: string): void {
    const entry = this.#entry;
    const signature = signatureFor(name, this.#catalogue);
    const prefix = this.#prefix ?? { text: '', from: this.#caret };
    if (entry === undefined || signature === undefined) return;
    const applied = applyCompletion(this.value, prefix, signature);
    this.#closeCompletions();
    this.setAttribute('value', applied.text);
    entry.value = applied.text;
    entry.setSelectionRange(applied.caret, applied.caret);
    this.#caret = applied.caret;
    this.#emitReferences();
    this.render();
    entry.focus();
    this.#syncCaret();
  }

  // ── the argument tooltip ───────────────────────────────────────────────────

  /**
   * The signature, with the argument the caret is in emphasised.
   *
   * *"The piece that makes Excel's formula editing usable and the piece most often omitted"*, in
   * the ticket's words. It is drawn from `activeArgument` and `emphasisedParameter` and computes
   * nothing itself, which is what makes the caret table a gate on the tooltip as well as on the
   * parser.
   */
  #paintTooltip(): void {
    const tooltip = this.#tooltip;
    const entry = this.#entry;
    if (tooltip === undefined || entry === undefined) return;
    const argument = this.mode === 'ready' ? undefined : this.argument;
    const signature = argument === undefined ? undefined : signatureFor(argument.functionName, this.#catalogue);
    if (argument === undefined || signature === undefined || this.completionsOpen) {
      if (!tooltip.hidden) {
        tooltip.hidden = true;
        clearPlacement(tooltip);
      }
      return;
    }

    const emphasis = emphasisedParameter(signature, argument.argumentIndex);
    const pieces: Node[] = [document.createTextNode(`${signature.name}(`)];
    for (const [index, parameter] of signature.parameters.entries()) {
      if (index > 0) pieces.push(document.createTextNode(', '));
      const span = document.createElement('span');
      const name = parameter.optional === true ? `[${parameter.name}]` : parameter.name;
      span.textContent = parameter.repeating === true ? `${name}, …` : name;
      if (index === emphasis) span.className = 'emphasis';
      pieces.push(span);
    }
    pieces.push(document.createTextNode(')'));
    const summary = document.createElement('span');
    summary.className = 'summary';
    summary.textContent = signature.summary;
    pieces.push(summary);
    tooltip.replaceChildren(...pieces);
    tooltip.hidden = false;
    this.#placeTooltip();
  }

  #placeTooltip(): void {
    const tooltip = this.#tooltip;
    const entry = this.#entry;
    if (tooltip === undefined || entry === undefined || tooltip.hidden) return;
    clearPlacement(tooltip);
    const natural = tooltip.getBoundingClientRect();
    const inset = resolveLength(tooltip, floatingProperties.boundaryInset);
    const gap = resolveLength(tooltip, floatingProperties.gap);
    applyPlacement(
      tooltip,
      placeFloating({
        anchor: rectOf(entry),
        floating: { width: natural.width, height: natural.height },
        boundary: clippingBoundary(tooltip, inset),
        side: listSide,
        align: listAlign,
        direction: this.#direction(),
        gap,
      }),
    );
  }

  // ── the affordances ────────────────────────────────────────────────────────

  #pressed(key: keyof typeof affordanceLabels): void {
    switch (key) {
      case 'cancel':
        this.cancel();
        this.focus();
        return;
      case 'confirm':
        this.commit();
        return;
      case 'insert': {
        this.focus();
        if (this.mode === 'ready') this.#toMode('beginEdit');
        this.#prefix = { text: '', from: this.#caret };
        this.#openCompletions(this.#catalogue);
        this.dispatchEvent(
          new CustomEvent(formulaEvents.insertFunction, { bubbles: true, composed: true, detail: {} }),
        );
        return;
      }
    }
  }

  // ── the drag handle ────────────────────────────────────────────────────────

  #onHandleKey = (event: KeyboardEvent): void => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const next = rowsAfterKey(event.key, this.rows);
    if (next === undefined) return;
    event.preventDefault();
    this.#setRows(next);
  };

  #onHandleDown = (event: PointerEvent): void => {
    const handle = this.#handle;
    if (handle === undefined) return;
    this.#dragFrom = { rows: this.rows, y: event.clientY };
    handle.setPointerCapture(event.pointerId);
    event.preventDefault();
  };

  #onHandleMove = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    const entry = this.#entry;
    if (from === undefined || entry === undefined) return;
    const rowHeight = entry.getBoundingClientRect().height / Math.max(1, this.rows);
    this.#setRows(rowsAfterDrag(from.rows, event.clientY - from.y, rowHeight));
  };

  #onHandleUp = (event: PointerEvent): void => {
    const handle = this.#handle;
    if (this.#dragFrom === undefined || handle === undefined) return;
    this.#dragFrom = undefined;
    if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
  };

  #setRows(next: number): void {
    const clamped = clampFormulaRows(next);
    if (clamped === this.rows) return;
    this.rows = clamped;
    this.render();
    this.#announce(`${String(clamped)} row${clamped === 1 ? '' : 's'}`);
    this.dispatchEvent(
      new CustomEvent(formulaEvents.rows, { bubbles: true, composed: true, detail: { rows: clamped } }),
    );
  }

  // ── modes ──────────────────────────────────────────────────────────────────

  #toMode(trigger: Parameters<typeof applyModeTrigger>[1]): void {
    const previous = this.#modeState.mode;
    const next = applyModeTrigger(this.#modeState, trigger);
    this.#modeState = next;
    if (next.mode === previous) return;
    if (previous === 'ready') this.#textOnEdit = this.value;
    this.render();
    const announcement = formulaModes[next.mode].announcement;
    this.#announce(announcement);
    this.dispatchEvent(
      new CustomEvent(formulaEvents.mode, {
        bubbles: true,
        composed: true,
        detail: { mode: next.mode, previous, announcement },
      }),
    );
  }

  #announce(text: string): void {
    const live = this.#live;
    if (live === undefined) return;
    live.textContent = text;
  }

  // ── the rest ───────────────────────────────────────────────────────────────

  #direction(): Direction {
    const bar = this.#bar;
    if (bar === undefined) return 'ltr';
    return getComputedStyle(bar).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  #onDocumentDown = (event: PointerEvent): void => {
    if (!this.completionsOpen) return;
    if (event.composedPath().includes(this)) return;
    this.#closeCompletions();
  };
}

/** A signature as a row in U07's listbox: the name, and the signature and summary beside it. */
function completionOption(signature: FunctionSignature): OptionDescriptor {
  return {
    value: signature.name,
    label: signature.name,
    description: `${signature.parameters.map((parameter) => parameter.name).join(', ')} — ${signature.summary}`,
    category: signature.category,
  };
}

/** Register the element. Idempotent. */
export function defineFormulaBar(): void {
  defineIcon();
  if (customElements.get('mjx-formula-bar') === undefined) {
    customElements.define('mjx-formula-bar', MjxFormulaBar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-formula-bar': MjxFormulaBar;
  }
}

export { isFormula };
