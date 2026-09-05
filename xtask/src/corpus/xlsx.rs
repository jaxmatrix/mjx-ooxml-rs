//! The SpreadsheetML corpus file: a workbook with hundreds of thousands of populated cells
//! (MJXOFF-147) — the one file MJXOFF-95 halts without. `mjx-xlsx` has no model yet (Phase D), so
//! this writes `xl/workbook.xml` and `xl/worksheets/sheet1.xml` directly on [`mjx_opc::Package`],
//! exactly the "open / tree-parse / save" layer that exists today.

use anyhow::{Context, Result};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

use super::common::{REL_OFFICE_DOCUMENT, XML_DECLARATION};

/// Populated rows.
pub const ROW_COUNT: usize = 5_000;
/// Populated columns per row.
pub const COLUMN_COUNT: usize = 60;
/// Total populated cells — `ROW_COUNT * COLUMN_COUNT`, "hundreds of thousands" per MJXOFF-68/-147.
pub const CELL_COUNT: usize = ROW_COUNT * COLUMN_COUNT;

const SPREADSHEETML_NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const CONTENT_TYPE_WORKBOOK: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const CONTENT_TYPE_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

/// Builds a SpreadsheetML package with one worksheet of [`CELL_COUNT`] populated cells: column A
/// holds an inline-string row label, the other [`COLUMN_COUNT`]` - 1` columns hold a numeric value,
/// and every row carries the `spans` hint Excel writes.
///
/// Not a claim that real workbooks are 98% numeric — it is one deliberately simple, deterministic
/// mix wide enough to stress both a numeric cell (`<c><v>`) and a string cell (`<c t="inlineStr">
/// <is><t>`) shape, without a shared-strings table `mjx-xlsx` has no model to build yet.
///
/// # Errors
/// Returns an error if the package cannot be assembled or fails its own validation.
pub fn build_large_workbook() -> Result<Vec<u8>> {
    let workbook = PartName::new("/xl/workbook.xml").context("workbook part name")?;
    let worksheet = PartName::new("/xl/worksheets/sheet1.xml").context("worksheet part name")?;
    let mut package = Package::empty();
    package
        .insert_part(&workbook, CONTENT_TYPE_WORKBOOK, workbook_bytes())
        .context("inserting xl/workbook.xml")?;
    package
        .insert_part(&worksheet, CONTENT_TYPE_WORKSHEET, worksheet_bytes())
        .context("inserting xl/worksheets/sheet1.xml")?;
    package
        .add_relationship(
            None,
            Relationship {
                id: "rId1".to_owned(),
                rel_type: REL_OFFICE_DOCUMENT.to_owned(),
                target: "xl/workbook.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .context("wiring the officeDocument relationship")?;
    package
        .add_relationship(
            Some(&workbook),
            Relationship {
                id: "rId1".to_owned(),
                rel_type: REL_WORKSHEET.to_owned(),
                target: "worksheets/sheet1.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .context("wiring the worksheet relationship")?;
    package.save().context("saving the generated workbook")
}

fn workbook_bytes() -> Vec<u8> {
    format!(
        "{XML_DECLARATION}<workbook xmlns=\"{SPREADSHEETML_NAMESPACE}\" \
         xmlns:r=\"{RELATIONSHIPS_NAMESPACE}\"><sheets><sheet name=\"Sheet1\" sheetId=\"1\" \
         r:id=\"rId1\"/></sheets></workbook>\r\n"
    )
    .into_bytes()
}

/// The bytes of `xl/worksheets/sheet1.xml`. Built as a plain string for the same reason
/// `docx::document_bytes` is: at [`CELL_COUNT`] cells, a `RawElement` tree costs the
/// memory and time this corpus exists to let a *reader* measure.
fn worksheet_bytes() -> Vec<u8> {
    let mut xml = String::with_capacity(ROW_COUNT * COLUMN_COUNT * 28 + 256);
    xml.push_str(XML_DECLARATION);
    xml.push_str("<worksheet xmlns=\"");
    xml.push_str(SPREADSHEETML_NAMESPACE);
    xml.push_str("\"><sheetData>\r\n");
    for row in 1..=ROW_COUNT {
        // `spans` on every row, as Excel writes it: the corpus is the one large SpreadsheetML file
        // this workspace has, and MJXOFF-93 could not exercise `ST_CellSpans` against it because
        // nothing here wrote one. It is an advisory hint, so it changes nothing about the file's
        // meaning — and it adds about 0.7% to the part's bytes, which the figures in
        // `docs/BENCHMARKS.md` were re-taken against.
        xml.push_str(&format!("<row r=\"{row}\" spans=\"1:{COLUMN_COUNT}\">"));
        for col in 0..COLUMN_COUNT {
            let cell_ref = column_letters(col);
            if col == 0 {
                xml.push_str(&format!(
                    "<c r=\"{cell_ref}{row}\" t=\"inlineStr\"><is><t>Row {row}</t></is></c>"
                ));
            } else {
                // A small deterministic spread of values, not a monotone counter — closer to what
                // a real sheet's numbers look like without needing a formula engine.
                let value = (row * 7 + col * 13) % 100_000;
                xml.push_str(&format!("<c r=\"{cell_ref}{row}\"><v>{value}</v></c>"));
            }
        }
        xml.push_str("</row>\r\n");
    }
    xml.push_str("</sheetData></worksheet>\r\n");
    xml.into_bytes()
}

/// The `ST_CellRef` column letters for a zero-based column index (`0 -> A`, `25 -> Z`, `26 -> AA`),
/// the same base-26 (no zero digit) encoding `SpreadsheetML` itself uses.
fn column_letters(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.reverse();
    String::from_utf8(letters).expect("only ASCII letters were pushed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_letters_match_spreadsheetml() {
        assert_eq!(column_letters(0), "A");
        assert_eq!(column_letters(25), "Z");
        assert_eq!(column_letters(26), "AA");
        assert_eq!(column_letters(27), "AB");
        assert_eq!(column_letters(51), "AZ");
        assert_eq!(column_letters(701), "ZZ");
        assert_eq!(column_letters(702), "AAA");
    }
}
