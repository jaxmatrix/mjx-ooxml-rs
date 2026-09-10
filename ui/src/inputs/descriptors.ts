/**
 * `<mjx-option>` and `<mjx-segment>` — **data written as markup, never content.**
 *
 * ```html
 * <mjx-dropdown label="Font">
 *   <mjx-option value="cambria" label="Cambria" description="Serif"></mjx-option>
 *   <mjx-option value="wingdings" label="Wingdings" unavailable
 *               explanation="The printer does not have it."></mjx-option>
 * </mjx-dropdown>
 * ```
 *
 * ## Read for attributes, never cloned and never slotted
 *
 * U06 found its first defect here in another shape: *item art cloned into a shadow root never
 * received a document class, so swatches rendered colourless while every counting gate stayed
 * green.* Nothing is cloned. A descriptor is read for its attributes and the row is built fresh by
 * the surface that owns it, so there is no element that could arrive in a tree with the wrong
 * ancestors.
 *
 * And nothing is matched by `instanceof`. U06's second defect: **custom elements upgrade in tree
 * order**, so a field's `connectedCallback` can run before its options have upgraded, and an
 * `instanceof` that ran too early would report a field with no options — which reads exactly like
 * an empty list. `optionDescriptorsIn` matches on `localName`, which is true from parse time.
 *
 * ## They are `display: none`, in two places
 *
 * On the element itself, and — through `inputDocumentCss` — on the document, so a descriptor does
 * not flash its attributes into the page in the moment between parsing and upgrading. That is the
 * same arrangement `galleryDocumentCss` makes and for the same reason.
 */

import type { OptionDescriptor } from './input-model.ts';

/**
 * What a descriptor fires when one of its attributes changes.
 *
 * Bubbling and composed, so the field listens once on itself. Without it a story that toggled an
 * option's `unavailable` would change nothing, because `slotchange` fires for added and removed
 * children and not for a child that changed.
 */
export const optionsChangedEvent = 'mjx-options-changed';

/** The attributes a descriptor carries. */
const descriptorAttributes = [
  'value',
  'label',
  'description',
  'unavailable',
  'explanation',
  'category',
  'icon',
] as const;

/** Read one element's attributes as a descriptor. */
function descriptorOf(element: Element): OptionDescriptor {
  const attribute = (name: string): string | undefined => {
    const found = element.getAttribute(name);
    return found === null || found === '' ? undefined : found;
  };
  const value = attribute('value');
  const label = attribute('label');
  return {
    // A descriptor with only a label is a descriptor whose value *is* its label, which is what an
    // author means by `<mjx-option label="Left">` and saves a duplicated string in every story.
    value: value ?? label ?? '',
    label: label ?? value ?? '',
    ...(attribute('description') === undefined ? {} : { description: attribute('description') }),
    ...(element.hasAttribute('unavailable') ? { unavailable: true } : {}),
    ...(attribute('explanation') === undefined ? {} : { explanation: attribute('explanation') }),
    ...(attribute('category') === undefined ? {} : { category: attribute('category') }),
  } as OptionDescriptor;
}

/**
 * Every descriptor of a given tag directly inside a host, in document order.
 *
 * Matched by `localName`, for the upgrade-order reason in the module note.
 */
export function optionDescriptorsIn(host: Element, tag: string): OptionDescriptor[] {
  const found: OptionDescriptor[] = [];
  for (const child of host.children) {
    if (child.localName === tag) found.push(descriptorOf(child));
  }
  return found;
}

/** The base both descriptors share: invisible, inert, and noisy when it changes. */
class MjxDescriptor extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [...descriptorAttributes];

  connectedCallback(): void {
    // `role="none"` as well as `display: none`, because a descriptor that a stylesheet failed to
    // reach must still not be a node in the accessibility tree.
    this.setAttribute('role', 'none');
    this.#announce();
  }

  attributeChangedCallback(): void {
    this.#announce();
  }

  #announce(): void {
    this.dispatchEvent(new CustomEvent(optionsChangedEvent, { bubbles: true, composed: true }));
  }
}

/** One choice in a dropdown or a combo box. */
export class MjxOption extends MjxDescriptor {}

/** One choice in a segmented control. */
export class MjxSegment extends MjxDescriptor {}

/** Register both. Idempotent. */
export function defineDescriptors(): void {
  if (customElements.get('mjx-option') === undefined) {
    customElements.define('mjx-option', MjxOption);
  }
  if (customElements.get('mjx-segment') === undefined) {
    customElements.define('mjx-segment', MjxSegment);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-option': MjxOption;
    'mjx-segment': MjxSegment;
  }
}
