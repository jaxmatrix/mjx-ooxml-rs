//! Authoring a complete PresentationML package from nothing.
//!
//! [`Presentation::open`](crate::Presentation::open) needs a `.pptx` to exist already. This module is
//! the other half: it writes the five parts a deck cannot do without — `presentation.xml`, a theme, a
//! slide master, one slide layout, and the relationships tying them together — on top of
//! [`mjx_opc::Package::empty`], which supplies `[Content_Types].xml` and the package-root `.rels`.
//!
//! # Why the markup is written out rather than shipped as a template
//!
//! A committed `.pptx` template would be the shortest route and the wrong one: it is markup nothing
//! in this repository can explain, it cannot follow the caller's slide size, and it is invisible to
//! the schema gate that exists precisely to keep us honest about what we emit
//! (`tests/schema_validity.rs`). Every element below is here because something needs it, and every
//! one of them is validated against the ECMA-376 XSDs by the same suite that validates a deck the
//! library edits.
//!
//! # What "minimal" means here
//!
//! Only elements the schema requires, plus the few a deck is useless without. Concretely:
//!
//! - `p:notesSz` is **required** (`CT_Presentation`, `minOccurs="1"`), which is easy to miss because
//!   nothing renders it until a notes master exists.
//! - `p:clrMap` is **required** on a slide master (`EG_TopLevelSlide`).
//! - `a:clrScheme` / `a:fontScheme` / `a:fmtScheme` are all **required** in `a:themeElements`, and
//!   the three style lists inside `a:fmtScheme` each demand **at least three entries**
//!   (`minOccurs="3"`), which is why the fill, line and effect lists come in threes.
//! - The master's two placeholders and the layout's matching pair are not required by the schema —
//!   they are what makes the result *usable*: they give
//!   [`add_slide_from_layout`](crate::Presentation::add_slide_from_layout) slots to clone and give
//!   the title and body text somewhere to inherit size, font and colour from.
//!
//! Deliberately absent: document properties (`docProps/*`, optional, and markup this project only
//! preserves elsewhere), `p:defaultTextStyle`, a notes master, and any Office extension list.

use mjx_opc::{Package, PartName, Relationship, TargetMode};

use crate::constants;
use crate::error::PptxError;
use crate::geometry::SlideSize;

/// The main presentation part of a deck this module builds.
pub(crate) const PRESENTATION_PART: &str = "/ppt/presentation.xml";
/// The single slide master.
pub(crate) const SLIDE_MASTER_PART: &str = "/ppt/slideMasters/slideMaster1.xml";
/// The single slide layout, owned by [`SLIDE_MASTER_PART`].
pub(crate) const SLIDE_LAYOUT_PART: &str = "/ppt/slideLayouts/slideLayout1.xml";
/// The theme, shared by the presentation and the master.
pub(crate) const THEME_PART: &str = "/ppt/theme/theme1.xml";

/// The smallest slide extent `p:sldSz` can express (ECMA-376 `ST_SlideSizeCoordinate`), in EMU.
pub(crate) const MIN_SLIDE_EXTENT_EMU: i64 = 914_400;
/// The largest slide extent `p:sldSz` can express (ECMA-376 `ST_SlideSizeCoordinate`), in EMU.
pub(crate) const MAX_SLIDE_EXTENT_EMU: i64 = 51_206_400;

/// The slide extent the placeholder geometry below was measured against: PowerPoint's 16:9 default,
/// 13⅓ × 7½ inches. Every placeholder offset and size is scaled from it to the caller's slide size,
/// so a 4:3 or A4 deck gets the same proportions rather than a title hanging off the edge.
const REFERENCE_WIDTH_EMU: i64 = 12_192_000;
/// The height half of [`REFERENCE_WIDTH_EMU`]'s reference extent.
const REFERENCE_HEIGHT_EMU: i64 = 6_858_000;

/// Builds a complete, valid PresentationML package with one master, one layout, a theme and **no
/// slides**.
///
/// The part graph is wired exactly as an Office-written deck wires it:
///
/// ```text
/// /_rels/.rels                     rId1 officeDocument -> ppt/presentation.xml
/// /ppt/_rels/presentation.xml.rels rId1 slideMaster    -> slideMasters/slideMaster1.xml
///                                  rId2 theme          -> theme/theme1.xml
/// /ppt/slideMasters/_rels/…        rId1 slideLayout    -> ../slideLayouts/slideLayout1.xml
///                                  rId2 theme          -> ../theme/theme1.xml
/// /ppt/slideLayouts/_rels/…        rId1 slideMaster    -> ../slideMasters/slideMaster1.xml
/// ```
///
/// The relationship ids are not arbitrary: `presentation.xml` names `rId1` from its
/// `p:sldMasterId`, and `slideMaster1.xml` names `rId1` from its `p:sldLayoutId`, so those two must
/// be added first within their part.
///
/// # Errors
/// Returns [`PptxError::InvalidSlideSize`] if either extent is outside the range `p:sldSz` can
/// express, or another [`PptxError`] if a package edit fails.
pub(crate) fn package(size: SlideSize) -> Result<Package, PptxError> {
    validate(size)?;

    let presentation = PartName::new(PRESENTATION_PART)?;
    let master = PartName::new(SLIDE_MASTER_PART)?;
    let layout = PartName::new(SLIDE_LAYOUT_PART)?;
    let theme = PartName::new(THEME_PART)?;

    let mut package = Package::empty();

    package.insert_part(
        &presentation,
        constants::CONTENT_TYPE_PRESENTATION,
        presentation_bytes(size),
    )?;
    package.insert_part(
        &master,
        constants::CONTENT_TYPE_SLIDE_MASTER,
        slide_master_bytes(size),
    )?;
    package.insert_part(
        &layout,
        constants::CONTENT_TYPE_SLIDE_LAYOUT,
        slide_layout_bytes(),
    )?;
    package.insert_part(&theme, constants::CONTENT_TYPE_THEME, theme_bytes())?;

    add_rel(
        &mut package,
        None,
        "rId1",
        constants::REL_OFFICE_DOCUMENT,
        "ppt/presentation.xml",
    )?;

    add_rel(
        &mut package,
        Some(&presentation),
        "rId1",
        constants::REL_SLIDE_MASTER,
        "slideMasters/slideMaster1.xml",
    )?;
    add_rel(
        &mut package,
        Some(&presentation),
        "rId2",
        constants::REL_THEME,
        "theme/theme1.xml",
    )?;

    add_rel(
        &mut package,
        Some(&master),
        "rId1",
        constants::REL_SLIDE_LAYOUT,
        "../slideLayouts/slideLayout1.xml",
    )?;
    add_rel(
        &mut package,
        Some(&master),
        "rId2",
        constants::REL_THEME,
        "../theme/theme1.xml",
    )?;

    add_rel(
        &mut package,
        Some(&layout),
        "rId1",
        constants::REL_SLIDE_MASTER,
        "../slideMasters/slideMaster1.xml",
    )?;

    Ok(package)
}

/// Rejects a slide extent `p:sldSz` cannot express, rather than emitting markup no conforming
/// consumer accepts. The bound is `ST_SlideSizeCoordinate`, not a house rule.
fn validate(size: SlideSize) -> Result<(), PptxError> {
    let in_range = |extent: i64| (MIN_SLIDE_EXTENT_EMU..=MAX_SLIDE_EXTENT_EMU).contains(&extent);
    if in_range(size.width_emu) && in_range(size.height_emu) {
        return Ok(());
    }
    Err(PptxError::InvalidSlideSize {
        width_emu: size.width_emu,
        height_emu: size.height_emu,
        min: MIN_SLIDE_EXTENT_EMU,
        max: MAX_SLIDE_EXTENT_EMU,
    })
}

/// Adds one relationship, keeping the call sites above readable.
fn add_rel(
    package: &mut Package,
    source: Option<&PartName>,
    id: &str,
    rel_type: &str,
    target: &str,
) -> Result<(), PptxError> {
    package.add_relationship(
        source,
        Relationship {
            id: id.to_owned(),
            rel_type: rel_type.to_owned(),
            target: target.to_owned(),
            mode: TargetMode::Internal,
        },
    )?;
    Ok(())
}

/// Rescales a reference measurement to the caller's slide extent: `value * extent / reference`.
///
/// The intermediate product overflows `i64` for large values (`51206400 * 51206400` is ~2.6e15,
/// which fits, but the general form does not), so it is computed in `i128` and narrowed back. A
/// zero reference or an unrepresentable result falls back to the unscaled value rather than
/// panicking — neither can happen with the constants here, and a library path never panics.
fn scaled(value: i64, extent: i64, reference: i64) -> i64 {
    if reference == 0 {
        return value;
    }
    let product = i128::from(value) * i128::from(extent) / i128::from(reference);
    i64::try_from(product).unwrap_or(value)
}

/// The XML declaration every part this module writes begins with, matching what Office emits and
/// what the rest of this crate's templates use.
const XML_DECLARATION: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n"
);

/// The three namespace declarations a PresentationML part root carries.
const PML_NAMESPACES: &str = concat!(
    r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#,
    r#" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#,
);

/// The bytes of `presentation.xml`: one slide master, an empty slide list, the caller's slide size,
/// and the notes-page size the schema requires alongside it.
///
/// `p:sldIdLst` is present and empty. It has to be present —
/// [`Presentation::open`](crate::Presentation::open) treats a missing one as a malformed deck, since
/// a deck that lists no slides at all is different from a deck whose list is empty — and it has to be
/// empty, because a blank document has no slides yet.
///
/// `p:notesSz` is the slide extent turned on its side, which is the portrait notes page every
/// PowerPoint deck records. Nothing renders it until a notes master exists, but `CT_Presentation`
/// requires the element.
fn presentation_bytes(size: SlideSize) -> Vec<u8> {
    format!(
        concat!(
            "{declaration}",
            "<p:presentation{namespaces}>",
            r#"<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>"#,
            "<p:sldIdLst/>",
            r#"<p:sldSz cx="{width}" cy="{height}" type="{kind}"/>"#,
            r#"<p:notesSz cx="{height}" cy="{width}"/>"#,
            "</p:presentation>",
        ),
        declaration = XML_DECLARATION,
        namespaces = PML_NAMESPACES,
        width = size.width_emu,
        height = size.height_emu,
        kind = size.kind.to_wire(),
    )
    .into_bytes()
}

/// The bytes of `theme1.xml`.
///
/// The colour scheme is the Office 2013 palette, so a deck built here looks like a deck built in
/// PowerPoint rather than like a debugging artefact. `dk1`/`lt1` are plain `a:srgbClr` rather than
/// `a:sysClr`: the value is then the same everywhere, which is what the effective-colour readers
/// resolve against.
///
/// The three fill styles are the same colour at three strengths (`phClr` is the placeholder the
/// shape's `a:fillRef` substitutes), the three line styles are three widths, and the three effect
/// styles are empty — `a:effectStyle` requires an effect group, and an empty `a:effectLst` is the
/// honest way to say "no effect" rather than inventing a shadow nothing asked for.
fn theme_bytes() -> Vec<u8> {
    concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        "\n",
        r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
        r#" name="Office Theme">"#,
        "<a:themeElements>",
        // --- colours -------------------------------------------------------------------------
        r#"<a:clrScheme name="Office">"#,
        r#"<a:dk1><a:srgbClr val="000000"/></a:dk1>"#,
        r#"<a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>"#,
        r#"<a:dk2><a:srgbClr val="44546A"/></a:dk2>"#,
        r#"<a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>"#,
        r#"<a:accent1><a:srgbClr val="4472C4"/></a:accent1>"#,
        r#"<a:accent2><a:srgbClr val="ED7D31"/></a:accent2>"#,
        r#"<a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>"#,
        r#"<a:accent4><a:srgbClr val="FFC000"/></a:accent4>"#,
        r#"<a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>"#,
        r#"<a:accent6><a:srgbClr val="70AD47"/></a:accent6>"#,
        r#"<a:hlink><a:srgbClr val="0563C1"/></a:hlink>"#,
        r#"<a:folHlink><a:srgbClr val="954F72"/></a:folHlink>"#,
        "</a:clrScheme>",
        // --- fonts ---------------------------------------------------------------------------
        r#"<a:fontScheme name="Office">"#,
        r#"<a:majorFont><a:latin typeface="Calibri Light"/><a:ea typeface=""/>"#,
        r#"<a:cs typeface=""/></a:majorFont>"#,
        r#"<a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/>"#,
        r#"<a:cs typeface=""/></a:minorFont>"#,
        "</a:fontScheme>",
        // --- the style matrix ----------------------------------------------------------------
        r#"<a:fmtScheme name="Office">"#,
        "<a:fillStyleLst>",
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#,
        r#"<a:solidFill><a:schemeClr val="phClr"><a:tint val="60000"/></a:schemeClr></a:solidFill>"#,
        r#"<a:solidFill><a:schemeClr val="phClr"><a:shade val="80000"/></a:schemeClr></a:solidFill>"#,
        "</a:fillStyleLst>",
        "<a:lnStyleLst>",
        r#"<a:ln w="6350" cap="flat" cmpd="sng" algn="ctr">"#,
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
        r#"<a:ln w="12700" cap="flat" cmpd="sng" algn="ctr">"#,
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
        r#"<a:ln w="19050" cap="flat" cmpd="sng" algn="ctr">"#,
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
        "</a:lnStyleLst>",
        "<a:effectStyleLst>",
        "<a:effectStyle><a:effectLst/></a:effectStyle>",
        "<a:effectStyle><a:effectLst/></a:effectStyle>",
        "<a:effectStyle><a:effectLst/></a:effectStyle>",
        "</a:effectStyleLst>",
        "<a:bgFillStyleLst>",
        r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#,
        r#"<a:solidFill><a:schemeClr val="phClr"><a:tint val="60000"/></a:schemeClr></a:solidFill>"#,
        r#"<a:solidFill><a:schemeClr val="phClr"><a:shade val="80000"/></a:schemeClr></a:solidFill>"#,
        "</a:bgFillStyleLst>",
        "</a:fmtScheme>",
        "</a:themeElements>",
        "</a:theme>",
    )
    .as_bytes()
    .to_vec()
}

/// The bytes of `slideMaster1.xml`: a title and a body placeholder positioned for `size`, the
/// identity colour map, the layout list, and the three text styles a placeholder inherits from.
///
/// The placeholders carry explicit bounds because a master is the bottom of the position-inheritance
/// chain: nothing below it states where a title goes, so if the master does not, a title has no
/// position at all. Everything else about them is left unstated on purpose — no fill, no outline, no
/// run properties — so that the layout, the theme and `p:txStyles` remain the single source for how
/// they look.
///
/// `p:clrMap` is the identity mapping (`bg1`→`lt1`, `tx1`→`dk1`, …), which is what makes
/// "background" mean the theme's light colour and "text" its dark one on a light deck.
fn slide_master_bytes(size: SlideSize) -> Vec<u8> {
    let title = placeholder_bounds(size, 838_200, 365_125, 10_515_600, 1_325_563);
    let body = placeholder_bounds(size, 838_200, 1_825_625, 10_515_600, 4_351_338);
    format!(
        concat!(
            "{declaration}",
            "<p:sldMaster{namespaces}>",
            r#"<p:cSld name="Office Theme"><p:spTree>"#,
            r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>"#,
            "<p:grpSpPr/>",
            // Title placeholder.
            "<p:sp>",
            r#"<p:nvSpPr><p:cNvPr id="2" name="Title Placeholder 1"/>"#,
            r#"<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>"#,
            r#"<p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>"#,
            "<p:spPr>{title_xfrm}",
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>"#,
            r#"<p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/>"#,
            r#"<a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#,
            "</p:sp>",
            // Body placeholder.
            "<p:sp>",
            r#"<p:nvSpPr><p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>"#,
            r#"<p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>"#,
            "<p:spPr>{body_xfrm}",
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>"#,
            r#"<p:txBody><a:bodyPr/><a:lstStyle/>"#,
            r#"<a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#,
            "</p:sp>",
            "</p:spTree></p:cSld>",
            r#"<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2""#,
            r#" accent1="accent1" accent2="accent2" accent3="accent3""#,
            r#" accent4="accent4" accent5="accent5" accent6="accent6""#,
            r#" hlink="hlink" folHlink="folHlink"/>"#,
            r#"<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>"#,
            "<p:txStyles>",
            // A title: the theme's major typeface, large, on the mapped text colour.
            "<p:titleStyle>",
            r#"<a:lvl1pPr algn="l"><a:defRPr sz="4400" kern="1200">"#,
            r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
            r#"<a:latin typeface="+mj-lt"/></a:defRPr></a:lvl1pPr>"#,
            "</p:titleStyle>",
            // Body text: five bulleted, indented outline levels in the minor typeface.
            "<p:bodyStyle>",
            "{body_style}",
            "</p:bodyStyle>",
            // Everything that is neither: plain, unbulleted, one size smaller.
            "<p:otherStyle>",
            r#"<a:lvl1pPr><a:defRPr sz="1800">"#,
            r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
            r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr>"#,
            "</p:otherStyle>",
            "</p:txStyles>",
            "</p:sldMaster>",
        ),
        declaration = XML_DECLARATION,
        namespaces = PML_NAMESPACES,
        title_xfrm = title,
        body_xfrm = body,
        body_style = BODY_STYLE_LEVELS,
    )
    .into_bytes()
}

/// The five outline levels of the master's `p:bodyStyle`: each one indented further, bulleted, and a
/// little smaller than the last — the shape of every default PowerPoint body placeholder.
///
/// `marL` is the left margin of the wrapped text and `indent` the negative first-line indent that
/// hangs the bullet in front of it; the two cancel, which is why the bullet sits at the margin and
/// the text lines up under itself.
const BODY_STYLE_LEVELS: &str = concat!(
    r#"<a:lvl1pPr marL="228600" indent="-228600" algn="l"><a:buChar char="•"/>"#,
    r#"<a:defRPr sz="2800" kern="1200">"#,
    r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
    r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr>"#,
    r#"<a:lvl2pPr marL="685800" indent="-228600" algn="l"><a:buChar char="•"/>"#,
    r#"<a:defRPr sz="2400" kern="1200">"#,
    r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
    r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl2pPr>"#,
    r#"<a:lvl3pPr marL="1143000" indent="-228600" algn="l"><a:buChar char="•"/>"#,
    r#"<a:defRPr sz="2000" kern="1200">"#,
    r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
    r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl3pPr>"#,
    r#"<a:lvl4pPr marL="1600200" indent="-228600" algn="l"><a:buChar char="•"/>"#,
    r#"<a:defRPr sz="1800" kern="1200">"#,
    r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
    r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl4pPr>"#,
    r#"<a:lvl5pPr marL="2057400" indent="-228600" algn="l"><a:buChar char="•"/>"#,
    r#"<a:defRPr sz="1800" kern="1200">"#,
    r#"<a:solidFill><a:schemeClr val="tx1"/></a:solidFill>"#,
    r#"<a:latin typeface="+mn-lt"/></a:defRPr></a:lvl5pPr>"#,
);

/// An `a:xfrm` positioning a master placeholder, with the reference measurements rescaled to `size`.
fn placeholder_bounds(
    size: SlideSize,
    offset_x: i64,
    offset_y: i64,
    width: i64,
    height: i64,
) -> String {
    format!(
        r#"<a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>"#,
        x = scaled(offset_x, size.width_emu, REFERENCE_WIDTH_EMU),
        y = scaled(offset_y, size.height_emu, REFERENCE_HEIGHT_EMU),
        cx = scaled(width, size.width_emu, REFERENCE_WIDTH_EMU),
        cy = scaled(height, size.height_emu, REFERENCE_HEIGHT_EMU),
    )
}

/// The bytes of `slideLayout1.xml`: the same two slots as the master, declaring nothing of their own.
///
/// An empty `p:spPr` is the point. A layout placeholder that states no transform inherits the
/// master's, so moving the master's title moves it on every slide built from this layout — which is
/// what a layout is for. `type="tx"` is `ST_SlideLayoutType`'s "Title and Text".
///
/// `<a:masterClrMapping/>` says "use the master's colour map unchanged". The alternative,
/// `a:overrideClrMapping`, requires all twelve mapping attributes; an empty one is invalid, which is
/// a defect this repository actually shipped for 58 releases before the schema gate caught it.
fn slide_layout_bytes() -> Vec<u8> {
    format!(
        concat!(
            "{declaration}",
            r#"<p:sldLayout{namespaces} type="tx">"#,
            r#"<p:cSld name="Title and Text"><p:spTree>"#,
            r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>"#,
            "<p:grpSpPr/>",
            "<p:sp>",
            r#"<p:nvSpPr><p:cNvPr id="2" name="Title 1"/>"#,
            r#"<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>"#,
            r#"<p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>"#,
            "<p:spPr/>",
            r#"<p:txBody><a:bodyPr/><a:lstStyle/>"#,
            r#"<a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#,
            "</p:sp>",
            "<p:sp>",
            r#"<p:nvSpPr><p:cNvPr id="3" name="Text Placeholder 2"/>"#,
            r#"<p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>"#,
            r#"<p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr>"#,
            "<p:spPr/>",
            r#"<p:txBody><a:bodyPr/><a:lstStyle/>"#,
            r#"<a:p><a:endParaRPr lang="en-US"/></a:p></p:txBody>"#,
            "</p:sp>",
            "</p:spTree></p:cSld>",
            "<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>",
            "</p:sldLayout>",
        ),
        declaration = XML_DECLARATION,
        namespaces = PML_NAMESPACES,
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_types::presentationml::SlideSizeKind;

    fn widescreen() -> SlideSize {
        SlideSize {
            width_emu: REFERENCE_WIDTH_EMU,
            height_emu: REFERENCE_HEIGHT_EMU,
            kind: SlideSizeKind::Screen16X9,
        }
    }

    #[test]
    fn every_authored_part_is_well_formed_xml() {
        let size = widescreen();
        for (label, bytes) in [
            ("presentation.xml", presentation_bytes(size)),
            ("theme1.xml", theme_bytes()),
            ("slideMaster1.xml", slide_master_bytes(size)),
            ("slideLayout1.xml", slide_layout_bytes()),
        ] {
            mjx_xml::fidelity::parse(&bytes)
                .unwrap_or_else(|e| panic!("{label} is not well-formed: {e}"));
        }
    }

    #[test]
    fn placeholder_bounds_scale_with_the_slide() {
        // At the reference size the measurements pass through unchanged.
        let at_reference =
            placeholder_bounds(widescreen(), 838_200, 365_125, 10_515_600, 1_325_563);
        assert!(at_reference.contains(r#"<a:off x="838200" y="365125"/>"#));
        assert!(at_reference.contains(r#"<a:ext cx="10515600" cy="1325563"/>"#));

        // Halve the slide and every measurement halves with it — a title that stayed put would hang
        // off the edge of a 4:3 deck.
        let half = SlideSize {
            width_emu: REFERENCE_WIDTH_EMU / 2,
            height_emu: REFERENCE_HEIGHT_EMU / 2,
            kind: SlideSizeKind::Custom,
        };
        let scaled_down = placeholder_bounds(half, 838_200, 365_125, 10_515_600, 1_325_563);
        assert!(scaled_down.contains(r#"<a:off x="419100" y="182562"/>"#));
        assert!(scaled_down.contains(r#"<a:ext cx="5257800" cy="662781"/>"#));
    }

    #[test]
    fn scaling_never_overflows_or_panics() {
        // `i64::MAX * i64::MAX / i64::MAX` overflows an `i64` multiplication outright; the `i128`
        // intermediate has to carry it, and the answer is exact.
        assert_eq!(scaled(i64::MAX, i64::MAX, i64::MAX), i64::MAX);
        // A result that cannot be narrowed back falls through to the input rather than panicking.
        assert_eq!(scaled(i64::MAX, 4, 1), i64::MAX);
        // A zero reference cannot divide; the value passes through instead of panicking.
        assert_eq!(scaled(123, 456, 0), 123);
    }

    #[test]
    fn a_slide_size_outside_the_schema_range_is_refused() {
        let too_small = SlideSize {
            width_emu: MIN_SLIDE_EXTENT_EMU - 1,
            height_emu: REFERENCE_HEIGHT_EMU,
            kind: SlideSizeKind::Custom,
        };
        assert!(matches!(
            package(too_small),
            Err(PptxError::InvalidSlideSize { .. })
        ));
        let too_large = SlideSize {
            width_emu: REFERENCE_WIDTH_EMU,
            height_emu: MAX_SLIDE_EXTENT_EMU + 1,
            kind: SlideSizeKind::Custom,
        };
        assert!(matches!(
            package(too_large),
            Err(PptxError::InvalidSlideSize { .. })
        ));
        // The boundaries themselves are legal.
        let smallest = SlideSize {
            width_emu: MIN_SLIDE_EXTENT_EMU,
            height_emu: MIN_SLIDE_EXTENT_EMU,
            kind: SlideSizeKind::Custom,
        };
        assert!(package(smallest).is_ok());
    }
}
