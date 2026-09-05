//! Reading a `CT_Rst` — the one place `si`, `is`, their runs and their phonetic markup are decoded.
//!
//! # Two entry points, one reader
//!
//! An item reaches this module either with a byte range into the part it came from, or with no range
//! at all — a tree somebody edited before the table read it, or an item this store has just
//! authored. The second case is not a second reader: the element is serialized into the arena and
//! then read back through the first, so there is exactly one description of what a `CT_Rst` is.
//!
//! The `base` parameter is what makes that work. An element parsed from the part's own buffer
//! carries ranges that are already arena addresses, so `base` is zero; an element parsed from a
//! fragment the arena has just stored carries ranges relative to that fragment, so `base` is where
//! the fragment landed. Nothing else differs.
//!
//! # What is matched, and how
//!
//! Children are matched by **local name**, prefix ignored. A `CT_Rst` is in the SpreadsheetML
//! namespace by construction — it is a child of an element that is — and a producer may bind that
//! namespace to any prefix or to none. Matching the prefix would make this reader stricter than the
//! files it has to read, and a fragment lifted out of a part does not carry the declaration anyway.
//!
//! Nothing here refuses anything. An `rPh` with no `sb`, a `t` holding a comment, a child this
//! module does not know: all are read as far as they can be and preserved in full by the item's
//! extent, which is what actually gets written.

use std::sync::Arc;

use mjx_ooxml_core::{Interner, RawElement, RawNode};

use crate::arena::{layout_in_arena, span_present_between, ElementLayout, TextSpan};
use crate::error::SmlError;

use super::items::StringItems;
use super::record::{
    ItemExtras, ItemFlags, PackedPhoneticRun, PackedRun, PackedStringItem, RunFlags, NO_EXTRAS,
};

/// The XML namespace, the one namespace whose prefix is fixed by the specification.
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// Reads one `CT_Rst` element into a record, appending its runs and phonetic runs to `store`.
///
/// # Errors
///
/// [`SmlError::PackedStoreTooLarge`] if the arena cannot hold markup that had to be serialized into
/// it, or [`SmlError::Xml`] if such markup does not re-parse.
pub(super) fn read_item(
    store: &mut StringItems,
    element: &RawElement,
    interner: &Interner,
    source: Option<&Arc<[u8]>>,
    base: u32,
    leading: TextSpan,
) -> Result<PackedStringItem, SmlError> {
    let extent = extent_of(store, element, base);
    let Some(layout) = layout_of(store, element, interner, extent) else {
        // No usable range: serialize the element into the arena and read *those* bytes, which is the
        // same answer arrived at the slow way. Recursion terminates because a freshly parsed
        // fragment always has ranges.
        let mut markup = Vec::new();
        mjx_xml::fidelity::serialize_element(
            element,
            interner,
            source.map(Arc::as_ref),
            &mut markup,
        );
        let stored = store.arena.store(&markup)?;
        let document = mjx_xml::fidelity::parse(&markup)?;
        return read_item(
            store,
            &document.root,
            &document.interner,
            None,
            stored.start(),
            leading,
        );
    };

    let mut item = PackedStringItem {
        extent,
        leading,
        ..PackedStringItem::default()
    };
    let mut extras = ItemExtras {
        first_phonetic: store.phonetics.len() as u32,
        ..ItemExtras::default()
    };
    let first_run = store.runs.len() as u32;
    let mut run_count = 0u32;
    let mut phonetic_count = 0u32;
    let mut element_children = 0usize;
    let mut only_child_is_text = false;

    for child in element.children.iter() {
        let RawNode::Element(child) = child else {
            continue;
        };
        element_children += 1;
        match interner.resolve(child.name.local) {
            "t" if item.text_element.is_none() => {
                let text = read_text_element(store, child, interner, base);
                item.text_element = text.element;
                item.text = text.inner;
                if text.preserves_space {
                    item.flags |= ItemFlags::TEXT_PRESERVES_SPACE;
                }
                only_child_is_text = element_children == 1;
            }
            "r" => {
                let run = read_run(store, child, interner, base);
                store.runs.push(run);
                run_count += 1;
            }
            "rPh" => {
                let phonetic = read_phonetic_run(store, child, interner, base);
                store.phonetics.push(phonetic);
                phonetic_count += 1;
            }
            "phoneticPr" if extras.phonetic_properties.is_none() => {
                extras.phonetic_properties = extent_of(store, child, base);
            }
            _ => {}
        }
    }

    item.first_run = first_run;
    item.run_count = run_count;
    extras.phonetic_count = phonetic_count;
    if !extras.is_empty() {
        item.extras = store.extras.len() as u32;
        store.extras.push(extras);
    }

    // Internable means *interchangeable with the same text*: a bare `<si><t>…</t></si>` with
    // nothing else in it. An item carrying runs or phonetic guides displays the same characters and
    // is not the same value, so reusing it for a plain string would silently give a cell formatting
    // or ruby text nobody asked for.
    if only_child_is_text
        && element_children == 1
        && element.attributes.is_empty()
        && !layout.self_closing
        && run_count == 0
        && item.extras == NO_EXTRAS
    {
        item.flags |= ItemFlags::INTERNABLE;
    }

    Ok(item)
}

/// One `<t>` as this reader sees it.
struct TextElement {
    /// The whole `<t …>…</t>`.
    element: TextSpan,
    /// Its still-escaped content. Present and empty for `<t/>` and `<t></t>`.
    inner: TextSpan,
    /// Whether it carried `xml:space="preserve"`.
    preserves_space: bool,
}

/// Reads a `t` child.
fn read_text_element(
    store: &mut StringItems,
    element: &RawElement,
    interner: &Interner,
    base: u32,
) -> TextElement {
    let extent = extent_of(store, element, base);
    let inner = match layout_of(store, element, interner, extent) {
        Some(layout) => span_present_between(layout.inner_start, layout.inner_end),
        None => TextSpan::NONE,
    };
    TextElement {
        element: extent,
        inner,
        preserves_space: preserves_space(element, interner),
    }
}

/// Whether `element` carries `xml:space="preserve"`.
///
/// Accepts the attribute either by its resolved namespace or by the literal `xml:` prefix, because a
/// fragment lifted out of a part has no declaration to resolve against and the `xml` prefix is bound
/// by the specification itself rather than by a declaration.
fn preserves_space(element: &RawElement, interner: &Interner) -> bool {
    element.attributes.iter().any(|attribute| {
        if interner.resolve(attribute.name.local) != "space" {
            return false;
        }
        let by_namespace = attribute
            .name
            .namespace
            .is_some_and(|namespace| interner.resolve(namespace) == XML_NAMESPACE);
        let by_prefix = attribute
            .name
            .prefix
            .is_some_and(|prefix| interner.resolve(prefix) == "xml");
        (by_namespace || by_prefix) && &*attribute.value == b"preserve"
    })
}

/// Reads an `r` child — `CT_RElt`.
fn read_run(
    store: &mut StringItems,
    element: &RawElement,
    interner: &Interner,
    base: u32,
) -> PackedRun {
    let mut run = PackedRun {
        extent: extent_of(store, element, base),
        ..PackedRun::default()
    };
    for child in element.children.iter() {
        let RawNode::Element(child) = child else {
            continue;
        };
        match interner.resolve(child.name.local) {
            "rPr" if run.properties.is_none() => {
                run.properties = extent_of(store, child, base);
            }
            "t" if run.text_element.is_none() => {
                let text = read_text_element(store, child, interner, base);
                run.text_element = text.element;
                run.text = text.inner;
                if text.preserves_space {
                    run.flags |= RunFlags::TEXT_PRESERVES_SPACE;
                }
            }
            _ => {}
        }
    }
    run
}

/// Reads an `rPh` child — `CT_PhoneticRun`.
///
/// `sb` and `eb` are `use="required"`, and a file that omits one is read as zero rather than
/// refused: the element's bytes are preserved either way, and an unreadable ruby offset is not a
/// reason to fail opening a workbook.
fn read_phonetic_run(
    store: &mut StringItems,
    element: &RawElement,
    interner: &Interner,
    base: u32,
) -> PackedPhoneticRun {
    let extent = extent_of(store, element, base);
    let mut phonetic = PackedPhoneticRun {
        extent,
        ..PackedPhoneticRun::default()
    };
    for attribute in element.attributes.iter() {
        let Ok(text) = core::str::from_utf8(&attribute.value) else {
            continue;
        };
        match interner.resolve(attribute.name.local) {
            "sb" => phonetic.start_base = text.trim().parse().unwrap_or(0),
            "eb" => phonetic.end_base = text.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    for child in element.children.iter() {
        let RawNode::Element(child) = child else {
            continue;
        };
        if interner.resolve(child.name.local) == "t" && phonetic.text.is_none() {
            phonetic.text = read_text_element(store, child, interner, base).inner;
        }
    }
    phonetic
}

/// The element's own byte range as an arena span, or [`TextSpan::NONE`].
fn extent_of(store: &StringItems, element: &RawElement, base: u32) -> TextSpan {
    match element.source_span() {
        Some(span) => store.arena.span_over(
            base.saturating_add(span.start),
            base.saturating_add(span.end),
        ),
        None => TextSpan::NONE,
    }
}

/// The element's start-tag / content split, in arena addresses.
fn layout_of(
    store: &StringItems,
    element: &RawElement,
    interner: &Interner,
    extent: TextSpan,
) -> Option<ElementLayout> {
    let mut qname = Vec::new();
    if let Some(prefix) = element.name.prefix {
        qname.extend_from_slice(interner.resolve(prefix).as_bytes());
        qname.push(b':');
    }
    qname.extend_from_slice(interner.resolve(element.name.local).as_bytes());
    layout_in_arena(store.arena.bytes(extent), &qname, extent)
}
