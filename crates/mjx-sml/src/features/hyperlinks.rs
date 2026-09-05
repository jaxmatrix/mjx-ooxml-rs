//! Hyperlinks: `x:hyperlinks` and the `x:hyperlink` entries in it, at rank **18** of
//! `CT_Worksheet`.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_Hyperlinks` | 2739 | `x:hyperlinks` (rank 18 of `CT_Worksheet`) |
//! | `CT_Hyperlink` | 2744 | `x:hyperlinks/hyperlink` |
//!
//! # A hyperlink covers a *range*, and the schema says so
//!
//! `@ref` is an `ST_Ref`, not an `ST_CellRef` — the same simple type `x:mergeCell` and `x:dimension`
//! carry, and MJXOFF-93's [`CellRange`]. One `x:hyperlink` can therefore link `B2:D4` in a single
//! element, which is what Excel writes when a caller selects several cells and inserts a link.
//! Nothing here splits such an entry into one per cell: that would change the file for no reason a
//! reader asked for, and it would lose the entry's identity as one link.
//!
//! # Two kinds, and the third shape that is neither a defect nor a third kind
//!
//! `CT_Hyperlink` declares `@r:id` and `@location` **both optional**, and the file decides which it
//! writes:
//!
//! * an **external** link names a relationship (`@r:id`) whose `.rels` entry is
//!   `TargetMode="External"` and whose `Target` is the URI;
//! * an **internal** link writes only `@location` — a cell reference such as `Sheet2!A1`, or a
//!   defined name — and names **no relationship at all**, because there is no part to reach.
//!
//! And then there is the one that catches a tidy-minded reader: **an entry carrying both an `@r:id`
//! and a `@location` is a real file Excel writes**, not a defect to clean up. An external target
//! with a fragment, and a link that once pointed outside and was repointed inside, both land there.
//! This crate reports what the element says and normalises none of it — it never drops the
//! relationship reference because a `@location` is present, never drops the `@location` because a
//! relationship is present, and never invents either.
//!
//! # This crate never resolves, rewrites or fetches a target
//!
//! [`Hyperlink`] holds the **raw relationship identifier** and nothing else, exactly as
//! [`TablePart`](crate::TablePart) does. Resolving one to a `.rels` entry is `mjx-xlsx`'s, in
//! `crates/mjx-xlsx/src/worksheet/hyperlinks.rs`; this crate has never heard of a package.
//!
//! When that resolution does happen, one rule governs it and it is stated in both places: **an
//! external target is an untrusted URI.** It is preserved exactly as the `.rels` wrote it — not
//! percent-normalised, not lower-cased, not resolved against a base, not made absolute, and above
//! all never fetched. A workbook may name `file:///etc/passwd` or a host that does not exist; both
//! are strings this library carries and neither is a thing it acts on.
//!
//! # A hyperlink and its relationship are one thing
//!
//! The invariant that runs the other way, and the reason the surface for this lives one tier up:
//! adding an external link adds a relationship, removing it removes that relationship, and a
//! relationship this library leaves behind is a defect
//! [`Workbook::validate`](https://docs.rs/mjx-xlsx) reports rather than something Excel is left to
//! repair. See `mjx_xlsx::SpreadsheetDefect::OrphanedHyperlinkRelationship`.

use mjx_ooxml_core::{
    Enumeration, Interner, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};

use crate::address::{CellRange, CellReference};
use crate::leaf::{attribute_bag, relationship_reference};
use crate::worksheet::{rebuild_element, WorksheetPart};

attribute_bag! {
    /// `x:hyperlink` (`CT_Hyperlink`, `sml.xsd:2744`) — one link over one range of cells.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Hyperlink`. Wire element: `hyperlink`.
    ///
    /// Five attributes, of which exactly one — `@ref` — is `use="required"`. `@r:id` is reached
    /// through [`relationship_id`](Self::relationship_id) rather than declared through the attribute
    /// grammar, for the same reason `x:tablePart`'s is: its *prefix* is the file's choice and not
    /// the schema's, so it cannot be declared through the derive's literal-prefix grammar without
    /// pinning `r`.
    ///
    /// `@display` is the text a consumer shows when the cell holds no value of its own; it is **not**
    /// kept in step with the cell's contents by anything here, because Excel does not keep them in
    /// step either and correcting one would be authoring a value nobody asked for.
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
    #[xml(attribute(local = "location", codec = Text, accessor = location))]
    #[xml(attribute(local = "tooltip", codec = Text, accessor = tooltip))]
    #[xml(attribute(local = "display", codec = Text, accessor = display))]
    Hyperlink, "hyperlink"
}

relationship_reference!(Hyperlink);

/// `x:hyperlinks` (`CT_Hyperlinks`, `sml.xsd:2739`) — every hyperlink on the sheet, in document
/// order.
///
/// **`ST_`/`CT_` symbol:** `CT_Hyperlinks`. Wire element: `hyperlinks`, rank **18** of
/// `CT_Worksheet` — between `dataValidations` (17) and `printOptions` (19).
///
/// The schema declares `hyperlink` `minOccurs="1"`, so a sheet that writes this element at all
/// writes at least one entry. That is why [`WorksheetPart::remove_hyperlink`](crate::WorksheetPart::remove_hyperlink)
/// takes the whole element out with the last entry rather than leaving an empty `<hyperlinks/>` the
/// schema gate would reject — the rule [`MergedCells`](crate::MergedCells) already follows.
///
/// `CT_Hyperlinks` declares **no `@count`**, unlike most of its neighbours, so there is no cache
/// here to refresh or to leave stale.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct Hyperlinks {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "hyperlink", variant = Link, ty = Hyperlink))]
    content: Vec<HyperlinksContent>,
}

/// One child of [`Hyperlinks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperlinksContent {
    /// `x:hyperlink` — one link.
    Link(Hyperlink),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl Hyperlinks {
    /// Builds an empty `x:hyperlinks`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `hyperlink` `minOccurs="1"`, so an element with no entry is invalid
    /// markup; it is still constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "hyperlinks"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[HyperlinksContent] {
        &self.content
    }

    /// Every `x:hyperlink`, in document order.
    pub fn links(&self) -> impl Iterator<Item = &Hyperlink> + '_ {
        self.content.iter().filter_map(|item| match item {
            HyperlinksContent::Link(link) => Some(link),
            HyperlinksContent::Raw(_) => None,
        })
    }

    /// How many links the sheet lists.
    #[must_use]
    pub fn len(&self) -> usize {
        self.links().count()
    }

    /// Whether the element lists no link at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:hyperlink`, mutably.
    pub fn link_mut(&mut self, index: usize) -> Option<&mut Hyperlink> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                HyperlinksContent::Link(link) => Some(link),
                HyperlinksContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a link after the ones already present.
    ///
    /// `CT_Hyperlinks` declares exactly one child slot, so "after the ones already present" and "at
    /// its rank in the sequence" are the same position — the shape [`TableParts`](crate::TableParts)
    /// has for the same reason.
    ///
    /// **Nothing checks that the new entry's `@ref` is disjoint from the ones already there.**
    /// Overlapping hyperlinks are a shape Excel writes and resolves by its own precedence; refusing
    /// or merging them here would be repairing markup on a guess.
    pub fn push(&mut self, link: Hyperlink) {
        self.content.push(HyperlinksContent::Link(link));
        self.empty = false;
    }

    /// Removes the `index`-th `x:hyperlink` and returns it, or `None` when the element holds fewer.
    ///
    /// Markup between the entries is left where it is: only the entry element itself is taken out.
    /// **The relationship an external entry named is not touched** — this crate cannot see one. The
    /// caller that can is `mjx_xlsx::Workbook::remove_cell_hyperlink`, and it is where the
    /// two-halves rule is enforced.
    pub fn remove(&mut self, index: usize) -> Option<Hyperlink> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, HyperlinksContent::Link(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            HyperlinksContent::Link(link) => Some(link),
            HyperlinksContent::Raw(_) => unreachable!("the position was filtered on `Link`"),
        }
    }

    /// The position of the first entry whose `@ref` contains `reference`, or `None`.
    ///
    /// **Document order, not precedence order.** Where two entries cover the same cell — which the
    /// schema permits and Excel writes — this answers the first the file lists and makes no claim
    /// about which one a consumer would follow.
    ///
    /// An entry whose `@ref` is absent or will not parse is stepped over rather than reported: it is
    /// the file's defect, and this is a lookup.
    #[must_use]
    pub fn position_covering(
        &self,
        interner: &Interner,
        reference: crate::address::CellReference,
    ) -> Option<usize> {
        self.links().position(|link| {
            link.range(interner)
                .is_ok_and(|range| range.contains(reference))
        })
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                HyperlinksContent::Link(link) => RawNode::Element(link.as_raw_element()),
                HyperlinksContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for Hyperlinks {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// The curated surface on the worksheet frame
// -----------------------------------------------------------------------------------------------

impl WorksheetPart {
    /// The first `x:hyperlink` whose `@ref` contains `reference`, or `None`.
    ///
    /// **Document order, not precedence order.** `CT_Hyperlinks` permits two entries to cover one
    /// cell and Excel writes such files; this answers the first the sheet lists and makes no claim
    /// about which one a consumer would follow. Reading does not mark the part edited.
    #[must_use]
    pub fn hyperlink_covering(&self, reference: CellReference) -> Option<&Hyperlink> {
        let links = self.hyperlinks()?;
        let index = links.position_covering(self.interner(), reference)?;
        links.links().nth(index)
    }

    /// The position of the first `x:hyperlink` covering `reference`, for a caller that is about to
    /// remove or replace it.
    ///
    /// Separate from [`hyperlink_covering`](Self::hyperlink_covering) because a borrow of the entry
    /// and a call to [`remove_hyperlink`](Self::remove_hyperlink) cannot be held at once, and
    /// because `mjx-xlsx` needs the index to keep the relationship in step.
    #[must_use]
    pub fn hyperlink_position_covering(&self, reference: CellReference) -> Option<usize> {
        self.hyperlinks()?
            .position_covering(self.interner(), reference)
    }

    /// Appends `link` to `x:hyperlinks`, creating the element at rank 18 of `CT_Worksheet`'s
    /// sequence if the sheet has none.
    ///
    /// **This adds no relationship**, because this crate has never heard of one. A caller adding an
    /// *external* link must add the relationship too, and the call that does both together is
    /// `mjx_xlsx::Workbook::set_cell_hyperlink`. Adding an entry here whose `@r:id` names nothing
    /// leaves a dangling reference `mjx_opc::Package::validate` reports.
    pub fn add_hyperlink(&mut self, link: Hyperlink) {
        if self.hyperlinks().is_none() {
            let prefix = self.element_prefix().map(str::to_owned);
            let element = Hyperlinks::new(self.interner_mut(), prefix.as_deref());
            self.set_hyperlinks(Some(element));
        }
        self.hyperlinks_mut()
            .expect("the hyperlinks element was just ensured")
            .push(link);
    }

    /// Removes the `index`-th `x:hyperlink` and returns it, or `None` when the sheet holds fewer.
    ///
    /// When the last entry goes, the whole `x:hyperlinks` element goes with it: the schema declares
    /// `hyperlink` `minOccurs="1"`, so an empty one is markup no validator accepts.
    ///
    /// **The relationship an external entry named is not removed**, for the same reason
    /// [`add_hyperlink`](Self::add_hyperlink) adds none. The entry is returned rather than dropped
    /// precisely so the caller one tier up can read its `@r:id` off it and remove the matching
    /// relationship — which `mjx_xlsx::Workbook::remove_cell_hyperlink` does, and which
    /// `mjx_xlsx::SpreadsheetDefect::OrphanedHyperlinkRelationship` catches when something does not.
    pub fn remove_hyperlink(&mut self, index: usize) -> Option<Hyperlink> {
        let links = self.hyperlinks_mut()?;
        let removed = links.remove(index)?;
        if links.is_empty() {
            self.set_hyperlinks(None);
        }
        Some(removed)
    }
}
