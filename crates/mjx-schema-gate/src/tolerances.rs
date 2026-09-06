//! Deviations carried by inputs this project preserves verbatim.
//!
//! Fidelity forbids "fixing" a defect on the way out, so the gate records it here instead of
//! failing — but every error the validator reports for that part must match, so a *new* defect in
//! the same part is still a failure. A tolerance never applies to a deck this library authors: the
//! authoring paths are handed an empty tolerance list.
//!
//! Every entry names a producer and says why the markup is not ours to correct. There are five:
//! three are in fixtures written by LibreOffice or python-pptx, one reproduces a producer-wide
//! divergence in a fixture MJXOFF-97 authored on purpose, and the fifth (MJXOFF-133's
//! `xl/xmlMaps.xml`) records a place where **the specification contradicts its own schema** — see
//! that entry's reason.

/// A schema deviation carried by an *input* rather than by markup this project writes.
#[derive(Debug, Clone, Copy)]
pub struct ToleratedDeviation {
    /// The fixture file name.
    pub fixture: &'static str,
    /// The absolute part name, e.g. `/ppt/charts/chart1.xml`.
    pub part: &'static str,
    /// Substring every tolerated validator error line contains.
    pub error_contains: &'static str,
    /// Why this is not ours to fix.
    pub reason: &'static str,
}

/// The complete set of deviations this gate tolerates. Deliberately tiny: each entry is a claim
/// that the markup came from somewhere else and that fidelity requires re-emitting it unchanged.
pub const TOLERATED_DEVIATIONS: &[ToleratedDeviation] = &[
    ToleratedDeviation {
        fixture: "charts.pptx",
        part: "/ppt/charts/chart1.xml",
        error_contains: "is not a valid value of the atomic type 'xs:unsignedInt'",
        reason: "charts.pptx is python-pptx's template and python-pptx derives c:axId/c:crossAx \
                 from a signed hash; the schema says xs:unsignedInt. An input we preserve verbatim, \
                 not markup we emit — `authored_charts_are_schema_valid` proves we never write a \
                 negative axis id ourselves",
    },
    ToleratedDeviation {
        fixture: "sample.xlsx",
        part: "/xl/sharedStrings.xml",
        error_contains: "The attribute '{http://www.w3.org/XML/1998/namespace}space' is not allowed",
        reason: "LibreOffice writes xml:space=\"preserve\" on every `s:t`, as Excel does; \
                 `sml.xsd` types `t` as the simple type `ST_Xstring`, which can carry no attribute \
                 at all, and does not import the XML namespace. A producer-wide divergence from the \
                 Transitional schema in an input we preserve — `mjx-chart`'s authored workbook \
                 writes no xml:space, so nothing here excuses markup we emit",
    },
    ToleratedDeviation {
        fixture: "shared_strings_rich_text.xlsx",
        part: "/xl/sharedStrings.xml",
        error_contains: "The attribute '{http://www.w3.org/XML/1998/namespace}space' is not allowed",
        reason: "The same producer-wide divergence `sample.xlsx` carries, in a fixture MJXOFF-97 \
                 authored on purpose: `xml:space=\"preserve\"` is what Excel and LibreOffice write \
                 on a `s:t` whose text has leading or trailing whitespace, and without it a consumer \
                 may collapse that whitespace and change the string. `sml.xsd` types `t` as the \
                 simple type `ST_Xstring`, which can carry no attribute at all. This fixture exists \
                 to hold a `preserve` entry, an unreferenced entry and a `uniqueCount` that \
                 disagrees with its own entry count, so that `mjx-sml`'s table is tested against a \
                 file that disagrees with the naive answer — and `mjx-sml` writes the attribute \
                 only where its absence would change the value, so an ordinary authored table is \
                 schema-valid",
    },
    ToleratedDeviation {
        fixture: "sample.xlsx",
        part: "/xl/workbook.xml",
        error_contains: "The attribute 'dateCompatibility' is not allowed",
        reason: "LibreOffice writes `dateCompatibility` on `s:workbookPr`; the attribute is not in \
                 the ECMA-376 5th-edition Transitional `sml.xsd`. An input we preserve verbatim",
    },
    ToleratedDeviation {
        fixture: "preserved_parts.xlsx",
        part: "/xl/xmlMaps.xml",
        error_contains: "demanded by the strict wildcard",
        reason: "`CT_Schema` (sml.xsd:340) is `<xsd:sequence><xsd:any/></xsd:sequence>` — one \
                 required child, and `processContents` defaults to `strict`, so the child must be a \
                 globally declared element of a schema the validator has loaded. What actually goes \
                 there is an **XML Schema document**, in the `http://www.w3.org/2001/XMLSchema` \
                 namespace, which `sml.xsd` neither imports nor could import. ECMA-376 Part 1 \
                 §12.3.6's own example inlines exactly such an `xsd:schema` and so does every file \
                 Excel writes, which makes `CT_Schema` a type no conformant instance can satisfy: \
                 omitting the child breaks `minOccurs`, and supplying the one the specification \
                 shows breaks the wildcard. MJXOFF-133 authored this fixture to reproduce the \
                 §12.3.6 example rather than to dodge it — `xl/xmlMaps.xml` is a part this library \
                 preserves verbatim and never writes, and a gap the specification leaves is not a \
                 gap a fixture should paper over. The same shape as the `CT_Extension` \
                 `xsd:any minOccurs=\"1\"` trap MJXOFF-120 recorded against `mc:Ignorable`",
    },
];

/// The tolerances registered for one fixture.
#[must_use]
pub fn tolerances_for(fixture: &str) -> Vec<&'static ToleratedDeviation> {
    TOLERATED_DEVIATIONS
        .iter()
        .filter(|tolerance| tolerance.fixture == fixture)
        .collect()
}
