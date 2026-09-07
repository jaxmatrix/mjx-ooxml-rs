//! MJXOFF-200 — the reference-resolution gate, held to its own standard.
//!
//! # Why this suite exists at all
//!
//! [`mjx_schema_gate::references`] is a gate, and a gate that cannot go red is a decoration. Every
//! case below is a **pair**: a package that resolves and the same package with one thing taken away,
//! so each rule is shown to fire rather than assumed to. The packages are hand-written here rather
//! than authored by a format crate, for the same reason `mjx-schema-gate` exists at all — this crate
//! sits below `mjx-pptx`, `mjx-docx` and `mjx-xlsx`, and a rule proved against markup one of them
//! happens to write today is a rule that stops being proved when that writer changes.
//!
//! The three format crates each assert the gate over the packages they really author; that is the
//! *use*. This file is the *proof*.

use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_schema_gate::{audit_package_references, ReferenceAudit};

const THEME_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.theme+xml";
const WORD_DOCUMENT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const WORD_STYLES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
const SHEET_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const SHEET_STYLES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";
const SLIDE_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
const CHART_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const X: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

const THEME_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";
const STYLES_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
const IMAGE_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

/// A theme with every colour slot and both font collections — what a resolving package carries.
fn full_theme() -> Vec<u8> {
    format!(
        r#"<a:theme xmlns:a="{A}" name="T"><a:themeElements>
             <a:clrScheme name="C">
               <a:dk1><a:srgbClr val="000000"/></a:dk1>
               <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
               <a:dk2><a:srgbClr val="111111"/></a:dk2>
               <a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
               <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
               <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
               <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
               <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
               <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
               <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
               <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
               <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
             </a:clrScheme>
             <a:fontScheme name="F">
               <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
               <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
             </a:fontScheme>
           </a:themeElements></a:theme>"#
    )
    .into_bytes()
}

/// The same theme with an **empty** `a:clrScheme` and no font scheme: the part exists, and resolves
/// nothing. This is the shape MJXOFF-200 warns a "the theme part is present" test would let through.
fn empty_theme() -> Vec<u8> {
    format!(
        r#"<a:theme xmlns:a="{A}" name="T"><a:themeElements>
             <a:clrScheme name="C"/><a:fontScheme name="F"/>
           </a:themeElements></a:theme>"#
    )
    .into_bytes()
}

/// A package holding one content part, optionally a theme, and the relationships between them.
fn package_with(
    part_name: &str,
    content_type: &str,
    body: &[u8],
    theme: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut package = Package::empty();
    let part = PartName::new(part_name).expect("a literal part name");
    package
        .insert_part(&part, content_type, body.to_vec())
        .expect("the content part");
    if let Some(theme) = theme {
        let theme_part = PartName::new("/theme/theme1.xml").expect("a literal part name");
        package
            .insert_part(&theme_part, THEME_CONTENT_TYPE, theme)
            .expect("the theme part");
        package
            .add_relationship(
                Some(&part),
                Relationship {
                    id: "rId9".to_owned(),
                    rel_type: THEME_REL_TYPE.to_owned(),
                    target: part.relative_target(&theme_part),
                    mode: TargetMode::Internal,
                },
            )
            .expect("the theme relationship");
    }
    package.save().expect("the package saves")
}

/// The audit of such a package.
fn audit(part: &str, content_type: &str, body: &str, theme: Option<Vec<u8>>) -> ReferenceAudit {
    audit_package_references(&package_with(part, content_type, body.as_bytes(), theme))
}

/// Every reference reported, as `site names "reference"` lines, for readable assertions.
fn sites(audit: &ReferenceAudit) -> Vec<String> {
    audit
        .dangling
        .iter()
        .map(|dangling| format!("{} {}", dangling.site, dangling.reference))
        .collect()
}

// =================================================================================================
// The defect this gate was written for
// =================================================================================================

/// **A scheme colour with no theme in the package is reported, and the same markup with a theme is
/// not.** This is MJXOFF-200 reduced to two packages.
#[test]
fn a_scheme_colour_needs_a_theme_and_the_gate_says_so() {
    let markup = format!(
        r#"<p:sld xmlns:p="{P}" xmlns:a="{A}"><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></p:sld>"#
    );

    let without = audit("/ppt/slides/slide1.xml", SLIDE_CONTENT_TYPE, &markup, None);
    assert_eq!(sites(&without), ["a:schemeClr@val accent1"]);
    assert!(
        without.dangling[0].reason.contains("no theme part"),
        "{}",
        without.dangling[0].reason
    );

    let with = audit(
        "/ppt/slides/slide1.xml",
        SLIDE_CONTENT_TYPE,
        &markup,
        Some(full_theme()),
    );
    assert!(with.is_clean(), "{}", with.report());
    assert!(with.resolved >= 1);
}

/// **A theme part that is present but empty does not save it.**
///
/// The whole reason this gate resolves the *slot* rather than asserting the part exists: an empty
/// `a:clrScheme` is a valid theme, passes the schema, passes the child-order audit, passes byte
/// identity — and paints nothing.
#[test]
fn an_empty_theme_resolves_nothing_and_is_reported() {
    let markup = format!(
        r#"<p:sld xmlns:p="{P}" xmlns:a="{A}"><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></p:sld>"#
    );
    let audit = audit(
        "/ppt/slides/slide1.xml",
        SLIDE_CONTENT_TYPE,
        &markup,
        Some(empty_theme()),
    );
    assert_eq!(sites(&audit), ["a:schemeClr@val accent1"]);
    assert!(
        audit.dangling[0].reason.contains("no such slot"),
        "{}",
        audit.dangling[0].reason
    );
}

/// `phClr` is the placeholder a style matrix substitutes, not a slot, so a theme's own
/// `a:fillStyleLst` does not report itself.
#[test]
fn the_style_matrix_placeholder_is_not_a_scheme_colour() {
    let markup = format!(
        r#"<p:sld xmlns:p="{P}" xmlns:a="{A}"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></p:sld>"#
    );
    let audit = audit("/ppt/slides/slide1.xml", SLIDE_CONTENT_TYPE, &markup, None);
    assert!(audit.is_clean(), "{}", audit.report());
}

/// The four colour-map spellings (`bg1`, `tx1`, `bg2`, `tx2`) resolve through the pair a
/// `p:clrMap` maps them onto, rather than being reported as unknown slots.
#[test]
fn the_colour_map_spellings_resolve_against_the_pair_they_map_onto() {
    let markup = format!(
        r#"<p:sld xmlns:p="{P}" xmlns:a="{A}"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill>
           <a:solidFill><a:schemeClr val="bg2"/></a:solidFill></p:sld>"#
    );
    let audit = audit(
        "/ppt/slides/slide1.xml",
        SLIDE_CONTENT_TYPE,
        &markup,
        Some(full_theme()),
    );
    assert!(audit.is_clean(), "{}", audit.report());
    // Two scheme colours, plus the theme relationship whose target resolves to the theme part.
    assert_eq!(audit.resolved, 3);
}

// =================================================================================================
// Theme fonts, in all three spellings
// =================================================================================================

/// A DrawingML `+mj-lt` typeface, a Word `minorHAnsi` and an Excel `<scheme val="minor"/>` all name
/// the same two collections, and all three are reported when the theme is absent.
#[test]
fn a_theme_font_reference_needs_a_font_scheme_in_every_spelling() {
    let drawing =
        format!(r#"<p:sld xmlns:p="{P}" xmlns:a="{A}"><a:latin typeface="+mj-lt"/></p:sld>"#);
    let word = format!(
        r#"<w:document xmlns:w="{W}"><w:rPr><w:rFonts w:asciiTheme="minorHAnsi"/></w:rPr></w:document>"#
    );
    let sheet = format!(
        r#"<styleSheet xmlns="{X}"><fonts><font><scheme val="minor"/></font></fonts></styleSheet>"#
    );

    for (part, content_type, markup, site) in [
        (
            "/ppt/slides/slide1.xml",
            SLIDE_CONTENT_TYPE,
            drawing,
            "a:latin@typeface +mj-lt",
        ),
        (
            "/word/document.xml",
            WORD_DOCUMENT_CONTENT_TYPE,
            word,
            "w:rFonts@w:asciiTheme minorHAnsi",
        ),
        (
            "/xl/styles.xml",
            SHEET_STYLES_CONTENT_TYPE,
            sheet,
            "scheme@val minor",
        ),
    ] {
        let without = audit(part, content_type, &markup, None);
        assert_eq!(sites(&without), [site], "{part} with no theme");
        let with = audit(part, content_type, &markup, Some(full_theme()));
        assert!(with.is_clean(), "{part} with a theme: {}", with.report());
    }
}

// =================================================================================================
// SpreadsheetML: a colour by index, and the style tables
// =================================================================================================

/// `<color theme="N"/>` is an index into the colour scheme, so an index past its end is reported
/// even though the theme is there.
#[test]
fn a_spreadsheet_theme_colour_index_is_checked_against_the_slots() {
    let inside = format!(
        r#"<styleSheet xmlns="{X}"><fonts><font><color theme="1"/></font></fonts></styleSheet>"#
    );
    let outside = format!(
        r#"<styleSheet xmlns="{X}"><fonts><font><color theme="12"/></font></fonts></styleSheet>"#
    );

    let ok = audit(
        "/xl/styles.xml",
        SHEET_STYLES_CONTENT_TYPE,
        &inside,
        Some(full_theme()),
    );
    assert!(ok.is_clean(), "{}", ok.report());

    let past = audit(
        "/xl/styles.xml",
        SHEET_STYLES_CONTENT_TYPE,
        &outside,
        Some(full_theme()),
    );
    assert_eq!(sites(&past), ["color@theme 12"]);

    let none = audit("/xl/styles.xml", SHEET_STYLES_CONTENT_TYPE, &inside, None);
    assert_eq!(sites(&none), ["color@theme 1"]);
}

/// A cell's `@s` indexes into `cellXfs`, and an `xf`'s `@fontId` into `fonts`. Both are reported
/// when the index is past the end of the table, and neither when it is not.
#[test]
fn a_style_index_is_checked_against_the_table_it_indexes() {
    let styles = format!(
        r#"<styleSheet xmlns="{X}"><fonts><font/></fonts><fills><fill/></fills>
           <borders><border/></borders><cellStyleXfs><xf/></cellStyleXfs>
           <cellXfs><xf fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs></styleSheet>"#
    );
    let sheet_ok = format!(
        r#"<worksheet xmlns="{X}"><sheetData><row r="1"><c r="A1" s="0"/></row></sheetData></worksheet>"#
    );
    let sheet_past = format!(
        r#"<worksheet xmlns="{X}"><sheetData><row r="1"><c r="A1" s="4"/></row></sheetData></worksheet>"#
    );

    for (body, expected) in [
        (sheet_ok, Vec::new()),
        (sheet_past, vec!["c@s 4".to_owned()]),
    ] {
        let mut package = Package::empty();
        let sheet = PartName::new("/xl/worksheets/sheet1.xml").expect("a name");
        let styles_part = PartName::new("/xl/styles.xml").expect("a name");
        package
            .insert_part(&sheet, SHEET_CONTENT_TYPE, body.into_bytes())
            .expect("the sheet");
        package
            .insert_part(
                &styles_part,
                SHEET_STYLES_CONTENT_TYPE,
                styles.clone().into_bytes(),
            )
            .expect("the styles part");
        let audit = audit_package_references(&package.save().expect("saves"));
        assert_eq!(sites(&audit), expected, "{}", audit.report());
    }
}

/// A `numFmtId` below 164 is one of ECMA-376 §18.8.30's built-ins and needs no record; one at or
/// above it does.
#[test]
fn a_built_in_number_format_needs_no_record_and_a_custom_one_does() {
    let built_in =
        format!(r#"<styleSheet xmlns="{X}"><cellXfs><xf numFmtId="14"/></cellXfs></styleSheet>"#);
    let custom =
        format!(r#"<styleSheet xmlns="{X}"><cellXfs><xf numFmtId="180"/></cellXfs></styleSheet>"#);
    let declared = format!(
        r#"<styleSheet xmlns="{X}"><numFmts><numFmt numFmtId="180" formatCode="0.0"/></numFmts>
           <cellXfs><xf numFmtId="180"/></cellXfs></styleSheet>"#
    );

    assert!(audit("/xl/styles.xml", SHEET_STYLES_CONTENT_TYPE, &built_in, None).is_clean());
    assert_eq!(
        sites(&audit(
            "/xl/styles.xml",
            SHEET_STYLES_CONTENT_TYPE,
            &custom,
            None
        )),
        ["xf@numFmtId 180"]
    );
    assert!(audit("/xl/styles.xml", SHEET_STYLES_CONTENT_TYPE, &declared, None).is_clean());
}

// =================================================================================================
// WordprocessingML style ids
// =================================================================================================

/// A `w:pStyle` names a style id, and the two ways it can fail read differently: there is no
/// `styles.xml` at all, or there is one and it does not define the id.
#[test]
fn a_word_style_id_is_checked_and_the_two_failures_read_differently() {
    let body = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr></w:p></w:body></w:document>"#
    );

    // No styles part.
    let missing = audit(
        "/word/document.xml",
        WORD_DOCUMENT_CONTENT_TYPE,
        &body,
        None,
    );
    assert_eq!(sites(&missing), ["w:pStyle@w:val Heading1"]);
    assert!(
        missing.dangling[0].reason.contains("no styles.xml"),
        "{}",
        missing.dangling[0].reason
    );

    // A styles part that defines a different id, then one that defines this one.
    for (styles, expected) in [
        (
            r#"<w:style w:type="paragraph" w:styleId="Normal"/>"#,
            vec!["w:pStyle@w:val Heading1".to_owned()],
        ),
        (
            r#"<w:style w:type="paragraph" w:styleId="Heading1"/>"#,
            Vec::new(),
        ),
    ] {
        let mut package = Package::empty();
        let document = PartName::new("/word/document.xml").expect("a name");
        let styles_part = PartName::new("/word/styles.xml").expect("a name");
        package
            .insert_part(
                &document,
                WORD_DOCUMENT_CONTENT_TYPE,
                body.clone().into_bytes(),
            )
            .expect("the document");
        package
            .insert_part(
                &styles_part,
                WORD_STYLES_CONTENT_TYPE,
                format!(r#"<w:styles xmlns:w="{W}">{styles}</w:styles>"#).into_bytes(),
            )
            .expect("the styles part");
        package
            .add_relationship(
                Some(&document),
                Relationship {
                    id: "rId1".to_owned(),
                    rel_type: STYLES_REL_TYPE.to_owned(),
                    target: "styles.xml".to_owned(),
                    mode: TargetMode::Internal,
                },
            )
            .expect("the styles relationship");
        let audit = audit_package_references(&package.save().expect("saves"));
        assert_eq!(sites(&audit), expected, "{}", audit.report());
    }
}

/// `w:numId` `0` is §17.9.18's documented "no numbering" and names no `w:num`, so it is not
/// reported; any other value with no numbering part is.
#[test]
fn numbering_id_zero_means_none_and_is_not_a_dangling_reference() {
    let none = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:numPr><w:numId w:val="0"/></w:numPr></w:pPr></w:p></w:body></w:document>"#
    );
    let real = format!(
        r#"<w:document xmlns:w="{W}"><w:body><w:p><w:pPr><w:numPr><w:numId w:val="3"/></w:numPr></w:pPr></w:p></w:body></w:document>"#
    );
    assert!(audit(
        "/word/document.xml",
        WORD_DOCUMENT_CONTENT_TYPE,
        &none,
        None
    )
    .is_clean());
    assert_eq!(
        sites(&audit(
            "/word/document.xml",
            WORD_DOCUMENT_CONTENT_TYPE,
            &real,
            None
        )),
        ["w:numId@w:val 3"]
    );
}

// =================================================================================================
// Relationships — the coarsest reference in the format
// =================================================================================================

/// An `r:embed` that names no relationship of its own part is reported; the same markup with the
/// relationship declared is not. An **empty** value is neither, because several schemas spell
/// "explicitly none" that way.
///
/// The dangling package is written with [`Package::save_unchecked`] on purpose: `Package::save`
/// already refuses this exact shape (`UndeclaredRelationshipReference`), which is why the gate's own
/// documentation says this rule adds no new coverage for a package **we** write. It is here so the
/// audit states the whole class, and so it holds for bytes that never went through our writer.
#[test]
fn a_relationship_id_is_checked_against_the_parts_own_rels() {
    let markup = format!(
        r#"<p:sld xmlns:p="{P}" xmlns:a="{A}" xmlns:r="{R}"><a:blip r:embed="rId7"/></p:sld>"#
    );
    let empty =
        format!(r#"<p:sld xmlns:p="{P}" xmlns:a="{A}" xmlns:r="{R}"><a:blip r:embed=""/></p:sld>"#);

    let mut broken = Package::empty();
    let slide_part = PartName::new("/ppt/slides/slide1.xml").expect("a name");
    broken
        .insert_part(&slide_part, SLIDE_CONTENT_TYPE, markup.clone().into_bytes())
        .expect("the slide");
    assert!(
        broken.save().is_err(),
        "mjx-opc's own validation already refuses an undeclared relationship reference"
    );
    let dangling = audit_package_references(&broken.save_unchecked().expect("saves unchecked"));
    assert_eq!(sites(&dangling), ["a:blip@r:embed rId7"]);

    assert!(audit("/ppt/slides/slide1.xml", SLIDE_CONTENT_TYPE, &empty, None).is_clean());

    let mut package = Package::empty();
    let slide = PartName::new("/ppt/slides/slide1.xml").expect("a name");
    let image = PartName::new("/ppt/media/image1.png").expect("a name");
    package
        .insert_part(&slide, SLIDE_CONTENT_TYPE, markup.into_bytes())
        .expect("the slide");
    package
        .insert_part(&image, "image/png", b"\x89PNG".to_vec())
        .expect("the image");
    package
        .add_relationship(
            Some(&slide),
            Relationship {
                id: "rId7".to_owned(),
                rel_type: IMAGE_REL_TYPE.to_owned(),
                target: "../media/image1.png".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the image relationship");
    let audit = audit_package_references(&package.save().expect("saves"));
    assert!(audit.is_clean(), "{}", audit.report());
}

/// An internal relationship whose target is not a part of the package is reported — the coarsest
/// dangling reference there is, and the one that removes a whole part rather than a colour.
///
/// As above, `Package::save` refuses this too, so the package is written unchecked.
#[test]
fn an_internal_relationship_target_must_name_a_part_that_exists() {
    let mut package = Package::empty();
    let slide = PartName::new("/ppt/slides/slide1.xml").expect("a name");
    package
        .insert_part(
            &slide,
            SLIDE_CONTENT_TYPE,
            format!(r#"<p:sld xmlns:p="{P}"/>"#).into_bytes(),
        )
        .expect("the slide");
    package
        .add_relationship(
            Some(&slide),
            Relationship {
                id: "rId1".to_owned(),
                rel_type: IMAGE_REL_TYPE.to_owned(),
                target: "../media/image1.png".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("the relationship");
    assert!(
        package.save().is_err(),
        "mjx-opc's own validation already refuses a relationship target with no part"
    );
    let audit = audit_package_references(&package.save_unchecked().expect("saves unchecked"));
    assert_eq!(
        sites(&audit),
        ["Relationship@Target (rId1) ../media/image1.png"]
    );

    // An `External` target is a URL by definition and is left alone.
    let mut external = Package::empty();
    external
        .insert_part(
            &slide,
            SLIDE_CONTENT_TYPE,
            format!(r#"<p:sld xmlns:p="{P}"/>"#).into_bytes(),
        )
        .expect("the slide");
    external
        .add_relationship(
            Some(&slide),
            Relationship {
                id: "rId1".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
                        .to_owned(),
                target: "https://example.invalid/".to_owned(),
                mode: TargetMode::External,
            },
        )
        .expect("the relationship");
    assert!(audit_package_references(&external.save().expect("saves")).is_clean());
}

// =================================================================================================
// The assertion's own guards
// =================================================================================================

/// An audit that resolved nothing proves nothing, and
/// [`assert_authored_package_resolves_every_reference`] says so rather than passing.
///
/// This is MJXOFF-88 §7 applied to this gate itself: *"a gate phrased 'X is covered and green' is
/// green precisely when X is skipped."* A package with no references at all is exactly that state.
///
/// [`assert_authored_package_resolves_every_reference`]:
///     mjx_schema_gate::assert_authored_package_resolves_every_reference
#[test]
#[should_panic(expected = "resolved nothing at all")]
fn a_package_making_no_reference_at_all_is_not_a_pass() {
    let mut package = Package::empty();
    let slide = PartName::new("/ppt/slides/slide1.xml").expect("a name");
    package
        .insert_part(
            &slide,
            SLIDE_CONTENT_TYPE,
            format!(r#"<p:sld xmlns:p="{P}"/>"#).into_bytes(),
        )
        .expect("the slide");
    mjx_schema_gate::assert_authored_package_resolves_every_reference(
        "a package with nothing in it",
        &package.save().expect("saves"),
    );
}

/// A package that will not open is an error rather than a silent pass.
#[test]
fn bytes_that_are_not_a_package_are_reported_rather_than_skipped() {
    let audit = audit_package_references(b"not a zip at all");
    assert!(!audit.errors.is_empty(), "{audit:?}");
    assert!(!audit.is_clean());
}

// =================================================================================================
// The implicit reference — the one with nothing in the markup to look for
// =================================================================================================

/// **A chart series that states no `c:spPr` references the theme without naming it.**
///
/// This is the rule G4 actually needed, and the reason a gate that only looked for
/// `a:schemeClr` would have stayed green through the whole defect: a series with no shape
/// properties contains no scheme colour to find. It *defers* to `accent1…accent6`, cycled by series
/// order — so the reference is real, the file says nothing about it, and with no theme in the
/// package it paints nothing.
///
/// The pair: the same two-series chart with a theme resolves, and each series that states its own
/// `c:spPr` defers to nothing and is not reported.
#[test]
fn a_series_with_no_shape_properties_references_the_theme_without_naming_it() {
    let deferring = format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}"><c:chart><c:plotArea>
             <c:barChart>
               <c:ser><c:idx val="0"/><c:order val="0"/></c:ser>
               <c:ser><c:idx val="1"/><c:order val="1"/></c:ser>
             </c:barChart>
           </c:plotArea></c:chart></c:chartSpace>"#
    );
    let explicit = format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}"><c:chart><c:plotArea>
             <c:barChart>
               <c:ser><c:idx val="0"/><c:order val="0"/>
                 <c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr></c:ser>
             </c:barChart>
           </c:plotArea></c:chart></c:chartSpace>"#
    );

    // No theme: one dangling implicit reference per series, cycling the accent slots.
    let without = audit(
        "/ppt/charts/chart1.xml",
        CHART_CONTENT_TYPE,
        &deferring,
        None,
    );
    assert_eq!(
        without
            .dangling
            .iter()
            .map(|dangling| dangling.reference.clone())
            .collect::<Vec<_>>(),
        ["accent1", "accent2"]
    );
    assert!(
        without.dangling[0].site.contains("no c:spPr"),
        "the site says why the reference exists: {}",
        without.dangling[0].site
    );

    // With a theme: both resolve.
    let with = audit(
        "/ppt/charts/chart1.xml",
        CHART_CONTENT_TYPE,
        &deferring,
        Some(full_theme()),
    );
    assert!(with.is_clean(), "{}", with.report());

    // A series that states its own fill defers to nothing, so no theme is needed for it.
    let stated = audit(
        "/ppt/charts/chart1.xml",
        CHART_CONTENT_TYPE,
        &explicit,
        None,
    );
    assert!(stated.is_clean(), "{}", stated.report());
}

/// The accent slots cycle past six, because a chart with more series than accents reuses them.
#[test]
fn the_seventh_series_returns_to_the_first_accent() {
    let mut series = String::new();
    for order in 0..7 {
        series.push_str(&format!(
            r#"<c:ser><c:idx val="{order}"/><c:order val="{order}"/></c:ser>"#
        ));
    }
    let markup = format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}"><c:chart><c:plotArea>
             <c:barChart>{series}</c:barChart>
           </c:plotArea></c:chart></c:chartSpace>"#
    );
    let audit = audit("/ppt/charts/chart1.xml", CHART_CONTENT_TYPE, &markup, None);
    assert_eq!(
        audit
            .dangling
            .iter()
            .map(|dangling| dangling.reference.clone())
            .collect::<Vec<_>>(),
        ["accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "accent1"]
    );
}
