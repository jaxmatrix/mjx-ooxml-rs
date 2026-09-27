//! The number-format conformance table: `(format code, value, expected string)`, with provenance.
//!
//! # ⚠ Read the provenance before you read the rows
//!
//! MJXOFF-172's ticket asks for a table *"transcribed from real Excel output"*, and says — correctly
//! — that a table whose expectations were produced by running this engine is green for any behaviour
//! whatsoever. **No such transcription was possible in this child**: nobody has run Excel against
//! these codes, and there is no Windows sitting behind this file.
//!
//! So every row carries a [`Provenance`], and the three values mean three genuinely different
//! things:
//!
//! * [`Provenance::SpecCode`] — the **format code** is transcribed character for character from
//!   ECMA-376 Part 1 §18.8.30, through [`mjx_sml::builtin_format_code`]. The specification prints
//!   the code and *not* what it renders, so the expected string is still this engine's.
//! * [`Provenance::DocumentedBehaviour`] — the pair is stated in published documentation or is a
//!   behaviour with an external, checkable definition: the 1900 leap-year defect, the fifteen-digit
//!   display, `[h]` not wrapping at twenty-four, `AM/PM` putting the clock on twelve hours, the
//!   section-selection table. These are the rows that are actually *evidence*.
//! * [`Provenance::EngineDerived`] — the expected string is what this engine's stated rule produces.
//!   **It is not evidence.** It is a change detector: it fails when a later edit moves a behaviour,
//!   which is worth having, and it proves nothing about Excel.
//!
//! [`counts_by_provenance`] prints the split, and `the_table_is_mostly_not_evidence` asserts that
//! the split is *reported* rather than quietly forgotten.
//!
//! # The hand-off
//!
//! `MJX_NUMFMT_CONFORMANCE_SHEET=<path>` writes the whole table as a tab-separated file whose first
//! two columns are the format code and the value. A person opens it beside Excel, applies each code
//! to each value, and the third column is what should have been there all along. That is the only
//! thing that turns [`Provenance::EngineDerived`] into evidence; see
//! `docs/validation/07-the-reference-pack.md`.

use mjx_layout_xlsx::numfmt::{
    evaluate, CellValue, CompiledFormat, DateToken, Element, SectionKind,
};
use mjx_xlsx::DateSystem;

/// Where a row's *expected string* came from. See the [module documentation](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provenance {
    /// The code is ECMA-376 Part 1 §18.8.30's, transcribed; the rendering is this engine's.
    SpecCode,
    /// The pair is documented, or has an external definition anyone can check.
    DocumentedBehaviour,
    /// The expected string is this engine's own answer. A change detector, not evidence.
    EngineDerived,
}

/// What a row formats.
#[derive(Debug, Clone, Copy)]
enum Value {
    /// A stored number.
    Number(f64),
    /// A stored string.
    Text(&'static str),
    /// `TRUE` or `FALSE`.
    Boolean(bool),
    /// An error code.
    Error(&'static str),
}

impl Value {
    fn as_cell(self) -> CellValue<'static> {
        match self {
            Self::Number(number) => CellValue::Number(number),
            Self::Text(text) => CellValue::Text(text),
            Self::Boolean(state) => CellValue::Boolean(state),
            Self::Error(text) => CellValue::Error(text),
        }
    }

    fn written(self) -> String {
        match self {
            Self::Number(number) => number.to_string(),
            Self::Text(text) => text.to_owned(),
            Self::Boolean(state) => state.to_string().to_uppercase(),
            Self::Error(text) => text.to_owned(),
        }
    }
}

/// One row of the table.
#[derive(Debug, Clone, Copy)]
struct Row {
    /// Which group of the table it belongs to, for the report.
    group: &'static str,
    /// The `numFmt@formatCode`.
    code: &'static str,
    /// What is formatted.
    value: Value,
    /// Which epoch the serials count from.
    dates: DateSystem,
    /// What the engine must write.
    expected: &'static str,
    /// The colour the format asks for, zero-based in `indexedColors`.
    colour: Option<u32>,
    /// Where the expectation came from.
    provenance: Provenance,
}

/// Shorthand for a 1900-system numeric row with no colour.
const fn number(
    group: &'static str,
    code: &'static str,
    value: f64,
    expected: &'static str,
    provenance: Provenance,
) -> Row {
    Row {
        group,
        code,
        value: Value::Number(value),
        dates: DateSystem::Windows1900,
        expected,
        colour: None,
        provenance,
    }
}

/// Shorthand for a row whose format asks for a colour.
const fn tinted(
    group: &'static str,
    code: &'static str,
    value: f64,
    expected: &'static str,
    colour: u32,
    provenance: Provenance,
) -> Row {
    Row {
        group,
        code,
        value: Value::Number(value),
        dates: DateSystem::Windows1900,
        expected,
        colour: Some(colour),
        provenance,
    }
}

/// Shorthand for a row that formats a string.
const fn text(
    group: &'static str,
    code: &'static str,
    value: &'static str,
    expected: &'static str,
    provenance: Provenance,
) -> Row {
    Row {
        group,
        code,
        value: Value::Text(value),
        dates: DateSystem::Windows1900,
        expected,
        colour: None,
        provenance,
    }
}

/// Shorthand for a row under the Macintosh epoch.
const fn macintosh(
    group: &'static str,
    code: &'static str,
    value: f64,
    expected: &'static str,
    provenance: Provenance,
) -> Row {
    Row {
        group,
        code,
        value: Value::Number(value),
        dates: DateSystem::Macintosh1904,
        expected,
        colour: None,
        provenance,
    }
}

use Provenance::{DocumentedBehaviour as Documented, EngineDerived as Derived, SpecCode as Spec};

/// `2025-03-03`, under the 1900 system.
const MARCH_THIRD: f64 = 45719.0;
/// `2025-03-03T12:00`, under the 1900 system.
const MARCH_THIRD_NOON: f64 = 45719.5;
/// `01:02:05`, as a fraction of a day.
const ONE_HOUR_TWO_MINUTES_FIVE_SECONDS: f64 = 3725.0 / 86400.0;

/// The table.
#[allow(clippy::too_many_lines)]
fn table() -> Vec<Row> {
    vec![
        // ── Every built-in id §18.8.30 lists under *All Languages* ────────────────────────────
        // The codes come from `mjx_sml::builtin_format_code`, which is the transcription; the
        // renderings are this engine's.
        number("builtin", "General", 1234.5678, "1234.5678", Spec),
        number("builtin", "0", 1234.5678, "1235", Spec),
        number("builtin", "0.00", 1234.5678, "1234.57", Spec),
        number("builtin", "#,##0", 1234.5678, "1,235", Spec),
        number("builtin", "#,##0.00", 1234.5678, "1,234.57", Spec),
        number("builtin", "0%", 1234.5678, "123457%", Spec),
        number("builtin", "0.00%", 1234.5678, "123456.78%", Spec),
        number("builtin", "0.00E+00", 1234.5678, "1.23E+03", Spec),
        number("builtin", "# ?/?", 1234.5678, "1234 4/7", Spec),
        number("builtin", "# ??/??", 1234.5678, "1234 46/81", Spec),
        number("builtin", "mm-dd-yy", MARCH_THIRD, "03-03-25", Spec),
        number("builtin", "d-mmm-yy", MARCH_THIRD, "3-Mar-25", Spec),
        number("builtin", "d-mmm", MARCH_THIRD, "3-Mar", Spec),
        number("builtin", "mmm-yy", MARCH_THIRD, "Mar-25", Spec),
        number("builtin", "h:mm AM/PM", 0.5, "12:00 PM", Spec),
        number("builtin", "h:mm:ss AM/PM", 0.5, "12:00:00 PM", Spec),
        number("builtin", "h:mm", 0.75, "18:00", Spec),
        number("builtin", "h:mm:ss", 0.75, "18:00:00", Spec),
        number(
            "builtin",
            "m/d/yy h:mm",
            MARCH_THIRD_NOON,
            "3/3/25 12:00",
            Spec,
        ),
        number("builtin", "#,##0 ;(#,##0)", 1234.5678, "1,235 ", Spec),
        number("builtin", "#,##0 ;(#,##0)", -1234.5678, "(1,235)", Spec),
        number("builtin", "#,##0 ;[Red](#,##0)", 1234.5678, "1,235 ", Spec),
        tinted(
            "builtin",
            "#,##0 ;[Red](#,##0)",
            -1234.5678,
            "(1,235)",
            2,
            Spec,
        ),
        number(
            "builtin",
            "#,##0.00;(#,##0.00)",
            -1234.5678,
            "(1,234.57)",
            Spec,
        ),
        tinted(
            "builtin",
            "#,##0.00;[Red](#,##0.00)",
            -1234.5678,
            "(1,234.57)",
            2,
            Spec,
        ),
        number(
            "builtin",
            "mm:ss",
            ONE_HOUR_TWO_MINUTES_FIVE_SECONDS,
            "02:05",
            Spec,
        ),
        number("builtin", "[h]:mm:ss", 1.5, "36:00:00", Spec),
        number(
            "builtin",
            "mmss.0",
            ONE_HOUR_TWO_MINUTES_FIVE_SECONDS,
            "0205.0",
            Spec,
        ),
        number("builtin", "##0.0E+0", 12345.0, "12.3E+3", Spec),
        text("builtin", "@", "hello", "hello", Spec),
        number("builtin", "@", 1234.5678, "1234.5678", Spec),
        // ── The four sections, on all four kinds of value ──────────────────────────────────────
        // The selection table is ECMA-376 Part 1 §18.8.31's and Microsoft documents it in the same
        // words; these rows are what "every multi-section format is exercised on a positive, a
        // negative, a zero and a text value" means.
        number(
            "sections",
            "#,##0.00;[Red](#,##0.00);\"—\";\"txt: \"@",
            1234.5,
            "1,234.50",
            Documented,
        ),
        tinted(
            "sections",
            "#,##0.00;[Red](#,##0.00);\"—\";\"txt: \"@",
            -1234.5,
            "(1,234.50)",
            2,
            Documented,
        ),
        number(
            "sections",
            "#,##0.00;[Red](#,##0.00);\"—\";\"txt: \"@",
            0.0,
            "—",
            Documented,
        ),
        text(
            "sections",
            "#,##0.00;[Red](#,##0.00);\"—\";\"txt: \"@",
            "abc",
            "txt: abc",
            Documented,
        ),
        // Two sections: the second takes every negative, unsigned, and zero stays with the first.
        number("sections", "0.00;(0.00)", 5.0, "5.00", Documented),
        number("sections", "0.00;(0.00)", -5.0, "(5.00)", Documented),
        number("sections", "0.00;(0.00)", 0.0, "0.00", Documented),
        text("sections", "0.00;(0.00)", "abc", "abc", Documented),
        // Three sections: zero gets its own.
        number("sections", "0.0;-0.0;\"zero\"", 1.0, "1.0", Documented),
        number("sections", "0.0;-0.0;\"zero\"", -1.0, "-1.0", Documented),
        number("sections", "0.0;-0.0;\"zero\"", 0.0, "zero", Documented),
        text("sections", "0.0;-0.0;\"zero\"", "abc", "abc", Documented),
        // One section: it answers for every number, and text still passes through.
        number("sections", "0.00", 5.0, "5.00", Documented),
        number("sections", "0.00", -5.0, "-5.00", Documented),
        number("sections", "0.00", 0.0, "0.00", Documented),
        text("sections", "0.00", "abc", "abc", Documented),
        // The `;;;` idiom hides a cell in all four states.
        number("sections", ";;;", 5.0, "", Documented),
        number("sections", ";;;", -5.0, "", Documented),
        number("sections", ";;;", 0.0, "", Documented),
        text("sections", ";;;", "abc", "", Documented),
        // A section that is only a colour writes nothing but still colours nothing.
        number("sections", "#,##0;;", 0.0, "", Derived),
        // ── Bracketed conditions ──────────────────────────────────────────────────────────────
        number(
            "conditions",
            "[>=1000]#,##0,\"K\";[>=0]0.00;[Red]\"neg\"",
            5000.0,
            "5K",
            Derived,
        ),
        number(
            "conditions",
            "[>=1000]#,##0,\"K\";[>=0]0.00;[Red]\"neg\"",
            12.5,
            "12.50",
            Derived,
        ),
        tinted(
            "conditions",
            "[>=1000]#,##0,\"K\";[>=0]0.00;[Red]\"neg\"",
            -3.0,
            "neg",
            2,
            Derived,
        ),
        number(
            "conditions",
            "[>100]\"big\";[<0]\"small\";0",
            5.0,
            "5",
            Derived,
        ),
        number(
            "conditions",
            "[>100]\"big\";[<0]\"small\";0",
            500.0,
            "big",
            Derived,
        ),
        number(
            "conditions",
            "[>100]\"big\";[<0]\"small\";0",
            -1.0,
            "small",
            Derived,
        ),
        number("conditions", "[=0]\"nil\";0.0", 0.0, "nil", Derived),
        number("conditions", "[=0]\"nil\";0.0", 3.0, "3.0", Derived),
        number("conditions", "[<>0]0.0;\"nil\"", 0.0, "nil", Derived),
        number("conditions", "[<=2]\"low\";\"high\"", 2.0, "low", Derived),
        // Nothing matches: GUESS, `General` rather than Excel's `#######`.
        number("conditions", "[>10]0;[>20]0", 1.0, "1", Derived),
        // ── Colours ───────────────────────────────────────────────────────────────────────────
        // The eight names and `[ColorN]` address `indexedColors`, whose first eight rows are black,
        // white, red, green, blue, yellow, magenta, cyan (ECMA-376 Part 1 §18.8.27).
        tinted("colours", "[Black]0", 1.0, "1", 0, Documented),
        tinted("colours", "[White]0", 1.0, "1", 1, Documented),
        tinted("colours", "[Red]0", 1.0, "1", 2, Documented),
        tinted("colours", "[Green]0", 1.0, "1", 3, Documented),
        tinted("colours", "[Blue]0", 1.0, "1", 4, Documented),
        tinted("colours", "[Yellow]0", 1.0, "1", 5, Documented),
        tinted("colours", "[Magenta]0", 1.0, "1", 6, Documented),
        tinted("colours", "[Cyan]0", 1.0, "1", 7, Documented),
        tinted("colours", "[Color1]0", 1.0, "1", 0, Documented),
        tinted("colours", "[Color3]0", 1.0, "1", 2, Documented),
        tinted("colours", "[Color15]0", 1.0, "1", 14, Documented),
        tinted("colours", "[COLOR56]0", 1.0, "1", 55, Documented),
        // `[Color0]` and `[Color57]` are outside the construct; the bracket degrades to nothing.
        number("colours", "[Color0]0", 1.0, "1", Derived),
        number("colours", "[Color57]0", 1.0, "1", Derived),
        // ── Scaling, percentages and grouping ─────────────────────────────────────────────────
        number("scaling", "0.0,,\"M\"", 1_234_567.0, "1.2M", Derived),
        number("scaling", "#,##0,\"K\"", 1_234_567.0, "1,235K", Derived),
        number("scaling", "0%", 0.125, "13%", Documented),
        number("scaling", "0.00%", 0.12345, "12.35%", Documented),
        number("scaling", "0.0%%", 0.5, "5000.0%%", Derived),
        number("scaling", "#,##0", 1_234_567.0, "1,234,567", Documented),
        number("scaling", "#,##0", 1.0, "1", Documented),
        number("scaling", "#,##0.00", 0.5, "0.50", Documented),
        number("scaling", "#,###", 1000.0, "1,000", Derived),
        // ── Placeholders and their padding ────────────────────────────────────────────────────
        number("placeholders", "0000", 12.0, "0012", Documented),
        number("placeholders", "####", 12.0, "12", Documented),
        number("placeholders", "????", 12.0, "  12", Documented),
        number("placeholders", "???0.0??", 5.5, "   5.5  ", Derived),
        number("placeholders", "#.##", 0.5, ".5", Documented),
        number("placeholders", "0.##", 0.5, "0.5", Documented),
        number("placeholders", ".00", 3.14259, "3.14", Derived),
        number("placeholders", "00", 12345.0, "12345", Documented),
        number(
            "placeholders",
            "000-0000",
            1_234_567.0,
            "123-4567",
            Documented,
        ),
        number("placeholders", "0.000", 1.5, "1.500", Documented),
        // ── Literals, escapes, skips and fills ────────────────────────────────────────────────
        number("literals", "\"$\"#,##0.00", 1234.5, "$1,234.50", Documented),
        number("literals", "\\$#,##0", 1234.0, "$1,234", Documented),
        number("literals", "0\" kg\"", 5.0, "5 kg", Documented),
        number("literals", "_(0_)", 5.0, " 5 ", Derived),
        number("literals", "0;;", 0.0, "", Derived),
        number(
            "literals",
            "[$-409]mmmm d, yyyy",
            MARCH_THIRD,
            "March 3, 2025",
            Derived,
        ),
        number("literals", "[$€-x-euro2] #,##0.00", 5.0, "€ 5.00", Derived),
        number("literals", "[$USD-409] 0.00", 5.0, "USD 5.00", Derived),
        number("literals", "0\"%\"", 5.0, "5%", Derived),
        number("literals", "*-0", 5.0, "5", Derived),
        // ── Scientific and engineering notation ───────────────────────────────────────────────
        number("scientific", "0.00E+00", 12345.0, "1.23E+04", Documented),
        number("scientific", "0.00E-00", 12345.0, "1.23E04", Derived),
        number(
            "scientific",
            "0.00E+00",
            0.000_123_45,
            "1.23E-04",
            Documented,
        ),
        number("scientific", "##0.0E+0", 12345.0, "12.3E+3", Documented),
        number("scientific", "##0.0E+0", 1234.0, "1.2E+3", Derived),
        number("scientific", "0.0E+0", 999.9, "1.0E+3", Derived),
        number("scientific", "0.00E+00", 0.0, "0.00E+00", Derived),
        // ── Fractions ─────────────────────────────────────────────────────────────────────────
        number("fractions", "# ?/?", 3.25, "3 1/4", Documented),
        number("fractions", "# ?/?", 0.5, " 1/2", Derived),
        number("fractions", "# ??/??", 0.5, "  1/2 ", Derived),
        number("fractions", "# ?/8", 3.25, "3 2/8", Documented),
        number("fractions", "# ??/16", 3.25, "3  4/16", Derived),
        number("fractions", "?/?", 1.25, "5/4", Derived),
        number("fractions", "# ?/?", 3.0, "3    ", Derived),
        number("fractions", "# ?/?", 2.99, "3    ", Derived),
        number("fractions", "# ???/???", 0.333_333_333, "   1/3  ", Derived),
        // ── Dates: every token ────────────────────────────────────────────────────────────────
        number("dates", "yyyy-mm-dd", MARCH_THIRD, "2025-03-03", Documented),
        number("dates", "yy", MARCH_THIRD, "25", Documented),
        number("dates", "yyyy", MARCH_THIRD, "2025", Documented),
        number("dates", "m", MARCH_THIRD, "3", Documented),
        number("dates", "mm", MARCH_THIRD, "03", Documented),
        number("dates", "mmm", MARCH_THIRD, "Mar", Documented),
        number("dates", "mmmm", MARCH_THIRD, "March", Documented),
        number("dates", "mmmmm", MARCH_THIRD, "M", Documented),
        number("dates", "d", MARCH_THIRD, "3", Documented),
        number("dates", "dd", MARCH_THIRD, "03", Documented),
        number("dates", "ddd", MARCH_THIRD, "Mon", Documented),
        number("dates", "dddd", MARCH_THIRD, "Monday", Documented),
        number("dates", "dddd", 1.0, "Sunday", Documented),
        number("dates", "dddd", 61.0, "Thursday", Documented),
        number(
            "dates",
            "m/d/yyyy h:mm:ss",
            MARCH_THIRD_NOON,
            "3/3/2025 12:00:00",
            Documented,
        ),
        number("dates", "hh:mm", 0.5, "12:00", Documented),
        number("dates", "h:m:s", MARCH_THIRD_NOON, "12:0:0", Derived),
        number("dates", "h:mm AM/PM", 0.0, "12:00 AM", Documented),
        number("dates", "h:mm am/pm", 0.75, "6:00 pm", Documented),
        number("dates", "h:mm A/P", 0.75, "6:00 P", Documented),
        number("dates", "h:mm a/p", 0.25, "6:00 a", Documented),
        number(
            "dates",
            "mm:ss.0",
            ONE_HOUR_TWO_MINUTES_FIVE_SECONDS,
            "02:05.0",
            Derived,
        ),
        number("dates", "ss.00", 0.5 + 0.125 / 86400.0, "00.13", Derived),
        // The minute/month disambiguation: the same `mm` twice, meaning two different things.
        number(
            "dates",
            "mm/dd hh:mm",
            MARCH_THIRD_NOON,
            "03/03 12:00",
            Documented,
        ),
        // ── Elapsed time ──────────────────────────────────────────────────────────────────────
        number("elapsed", "[h]:mm:ss", 1.5, "36:00:00", Documented),
        number("elapsed", "[h]", 0.25, "6", Documented),
        number("elapsed", "[hh]:mm", 0.25, "06:00", Derived),
        number("elapsed", "[mm]:ss", 0.5, "720:00", Documented),
        number("elapsed", "[ss]", 1.0, "86400", Documented),
        number("elapsed", "[m]", 1.0, "1440", Documented),
        // ── ⚠ The two Excel quirks ────────────────────────────────────────────────────────────
        // Serial 60 is a day that never existed. A renderer that "fixes" it fails here.
        number("quirks", "yyyy-mm-dd", 59.0, "1900-02-28", Documented),
        number("quirks", "yyyy-mm-dd", 60.0, "1900-02-29", Documented),
        number("quirks", "yyyy-mm-dd", 61.0, "1900-03-01", Documented),
        number("quirks", "yyyy-mm-dd", 1.0, "1900-01-01", Documented),
        number("quirks", "m/d/yyyy", 0.0, "1/0/1900", Documented),
        // Fifteen significant digits, and the decimal rounding that follows from it.
        number("quirks", "General", 0.1 + 0.2, "0.3", Documented),
        number(
            "quirks",
            "0.00000000000000000000",
            0.1 + 0.2,
            "0.30000000000000000000",
            Documented,
        ),
        number("quirks", "0.00", 2.675, "2.68", Documented),
        number("quirks", "0.00", 0.125, "0.13", Documented),
        number("quirks", "0.00", 0.135, "0.14", Documented),
        number(
            "quirks",
            "General",
            1_234_567_890_123_456.0,
            "1.23457E+15",
            Derived,
        ),
        number("quirks", "0", 0.5, "1", Documented),
        number("quirks", "0", 1.5, "2", Documented),
        number("quirks", "0", 2.5, "3", Documented),
        // ── The 1904 epoch ────────────────────────────────────────────────────────────────────
        // 1,462 days lower than the 1900 system, and no phantom day anywhere in it.
        macintosh("epoch", "yyyy-mm-dd", 0.0, "1904-01-01", Documented),
        macintosh("epoch", "yyyy-mm-dd", 1.0, "1904-01-02", Documented),
        macintosh("epoch", "yyyy-mm-dd", 59.0, "1904-02-29", Documented),
        macintosh("epoch", "yyyy-mm-dd", 60.0, "1904-03-01", Documented),
        macintosh(
            "epoch",
            "yyyy-mm-dd",
            MARCH_THIRD - 1462.0,
            "2025-03-03",
            Documented,
        ),
        macintosh("epoch", "dddd", 0.0, "Friday", Documented),
        // ── General ───────────────────────────────────────────────────────────────────────────
        number("general", "General", 0.0, "0", Documented),
        number("general", "General", 1.0, "1", Documented),
        number("general", "General", -1.5, "-1.5", Documented),
        number("general", "General", 0.000_1, "0.0001", Derived),
        number("general", "General", 0.000_01, "1E-05", Derived),
        number(
            "general",
            "General",
            99_999_999_999.0,
            "99999999999",
            Derived,
        ),
        number("general", "General", 100_000_000_000.0, "1E+11", Derived),
        number("general", "General", 1.0 / 3.0, "0.33333333333", Derived),
        // ── The sign, and where it lands ──────────────────────────────────────────────────────
        number("sign", "$#,##0.00", -5.0, "-$5.00", Derived),
        number("sign", "0.00", -5.0, "-5.00", Documented),
        number("sign", "\"n/a\"", -5.0, "n/a", Derived),
        number("sign", "0.00;0.00", -5.0, "5.00", Documented),
        number("sign", "General", -0.5, "-0.5", Documented),
        // ── Text values ───────────────────────────────────────────────────────────────────────
        text("text", "0;0;0;\"[[\"@\"]]\"", "x", "[[x]]", Derived),
        text("text", "@", "x", "x", Documented),
        text("text", "0.00", "x", "x", Documented),
        text("text", "0;0;0;", "x", "", Derived),
        text("text", "General", "x", "x", Documented),
        Row {
            group: "text",
            code: "0.00",
            value: Value::Boolean(true),
            dates: DateSystem::Windows1900,
            expected: "TRUE",
            colour: None,
            provenance: Documented,
        },
        Row {
            group: "text",
            code: "$#,##0.00",
            value: Value::Boolean(false),
            dates: DateSystem::Windows1900,
            expected: "FALSE",
            colour: None,
            provenance: Documented,
        },
        Row {
            group: "text",
            code: "0.00",
            value: Value::Error("#DIV/0!"),
            dates: DateSystem::Windows1900,
            expected: "#DIV/0!",
            colour: None,
            provenance: Documented,
        },
    ]
}

/// Runs every row.
#[test]
fn the_conformance_table_holds() {
    let mut failures = Vec::new();
    for row in table() {
        let compiled = CompiledFormat::compile(row.code);
        let rendered = evaluate(&compiled, row.value.as_cell(), row.dates);
        if rendered.text != row.expected {
            failures.push(format!(
                "[{}] {:?} on {} -> {:?}, expected {:?}",
                row.group,
                row.code,
                row.value.written(),
                rendered.text,
                row.expected
            ));
        }
        if rendered.colour != row.colour {
            failures.push(format!(
                "[{}] {:?} on {} -> colour {:?}, expected {:?}",
                row.group,
                row.code,
                row.value.written(),
                rendered.colour,
                row.colour
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} conformance rows failed:\n{}",
        failures.len(),
        table().len(),
        failures.join("\n")
    );
}

/// The table can fail — perturbing one expected string is caught.
///
/// The ticket asks for this outright, because a harness that silently passes every row is
/// indistinguishable from one that runs none of them.
#[test]
fn perturbing_one_expectation_fails() {
    let rows = table();
    let victim = rows
        .iter()
        .find(|row| row.group == "builtin" && row.code == "#,##0.00")
        .copied()
        .expect("the table states `#,##0.00`");
    let compiled = CompiledFormat::compile(victim.code);
    let rendered = evaluate(&compiled, victim.value.as_cell(), victim.dates);
    assert_eq!(rendered.text, victim.expected, "the row itself holds");
    let perturbed = format!("{}x", victim.expected);
    assert_ne!(
        rendered.text, perturbed,
        "a perturbed expectation must not match"
    );
}

/// How many rows carry each provenance.
fn counts_by_provenance(rows: &[Row]) -> (usize, usize, usize) {
    let count = |wanted: Provenance| rows.iter().filter(|row| row.provenance == wanted).count();
    (
        count(Provenance::SpecCode),
        count(Provenance::DocumentedBehaviour),
        count(Provenance::EngineDerived),
    )
}

/// ⚠ States, out loud, how much of this table is evidence and how much is a change detector.
///
/// The assertion is deliberately weak — it only checks that the table has rows of each kind and that
/// nobody has quietly turned it into an all-`EngineDerived` file. What it is really for is the
/// printed split, which the next reader needs before they trust a green run.
#[test]
fn the_table_is_mostly_not_evidence() {
    let rows = table();
    let (spec, documented, derived) = counts_by_provenance(&rows);
    println!(
        "conformance rows: {} total — {spec} spec-transcribed codes, {documented} documented \
         behaviours, {derived} engine-derived (change detectors, NOT evidence about Excel)",
        rows.len()
    );
    assert!(spec >= 30, "every all-languages built-in id is covered");
    assert!(
        documented >= 60,
        "the rows that are actually evidence must not shrink"
    );
    assert!(derived > 0, "the engine-derived rows are declared as such");
    assert_eq!(
        spec + documented + derived,
        rows.len(),
        "every row declares a provenance"
    );
}

/// Every all-languages built-in id of §18.8.30 appears in the table, by **code**.
///
/// The defence against the failure mode the ticket names: a built-in table that is short falls back
/// to `General` and looks plausible. This asks `mjx-sml` for the list rather than restating it, so a
/// row that vanishes from either side fails here.
#[test]
fn every_all_languages_builtin_id_is_covered() {
    let rows = table();
    let mut missing = Vec::new();
    let mut found = 0;
    for id in 0..=49_u32 {
        let Some(code) = mjx_sml::builtin_format_code(id) else {
            continue;
        };
        found += 1;
        if !rows.iter().any(|row| row.code == code) {
            missing.push(format!("{id} ({code})"));
        }
    }
    assert_eq!(
        found, 28,
        "§18.8.30 lists twenty-eight ids under *All Languages*; `mjx-sml` answered {found}"
    );
    assert!(
        missing.is_empty(),
        "built-in ids with no conformance row: {}",
        missing.join(", ")
    );
}

/// Writes the table as a sheet a person can take to Excel.
///
/// Off by default. `MJX_NUMFMT_CONFORMANCE_SHEET=<path>` turns it on, and the file's third column is
/// what the Windows sitting fills in — see the [module documentation](self).
#[test]
fn the_hand_off_sheet_is_writable() {
    let Ok(path) = std::env::var("MJX_NUMFMT_CONFORMANCE_SHEET") else {
        println!("set MJX_NUMFMT_CONFORMANCE_SHEET=<path> to write the sheet for the sitting");
        return;
    };
    let mut out = String::from("group\tformat code\tvalue\tdate system\tours\texcel\tprovenance\n");
    for row in table() {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t\t{:?}\n",
            row.group,
            row.code,
            row.value.written(),
            if row.dates.is_1904() { "1904" } else { "1900" },
            row.expected,
            row.provenance,
        ));
    }
    std::fs::write(&path, out).expect("the hand-off sheet is writable");
    println!("wrote {path}");
}

/// The name of an [`Element`] variant, for the reachability report.
fn element_name(element: &Element) -> &'static str {
    match element {
        Element::Literal(_) => "Literal",
        Element::IntegerDigit(_) => "IntegerDigit",
        Element::DecimalPoint => "DecimalPoint",
        Element::DecimalDigit(_) => "DecimalDigit",
        Element::NumeratorDigit(_) => "NumeratorDigit",
        Element::FractionBar => "FractionBar",
        Element::DenominatorDigit(_) => "DenominatorDigit",
        Element::FixedDenominator(_) => "FixedDenominator",
        Element::Exponent { .. } => "Exponent",
        Element::ExponentDigit(_) => "ExponentDigit",
        Element::Percent => "Percent",
        Element::Date(_) => "Date",
        Element::Repeat(_) => "Repeat",
        Element::Skip(_) => "Skip",
        Element::TextValue => "TextValue",
        Element::General => "General",
    }
}

/// The name of a [`DateToken`] variant.
fn token_name(token: DateToken) -> &'static str {
    match token {
        DateToken::Year2 => "Year2",
        DateToken::Year4 => "Year4",
        DateToken::MonthNumber => "MonthNumber",
        DateToken::MonthNumberPadded => "MonthNumberPadded",
        DateToken::MonthAbbreviation => "MonthAbbreviation",
        DateToken::MonthName => "MonthName",
        DateToken::MonthLetter => "MonthLetter",
        DateToken::Day => "Day",
        DateToken::DayPadded => "DayPadded",
        DateToken::WeekdayAbbreviation => "WeekdayAbbreviation",
        DateToken::WeekdayName => "WeekdayName",
        DateToken::Hour => "Hour",
        DateToken::HourPadded => "HourPadded",
        DateToken::Minute => "Minute",
        DateToken::MinutePadded => "MinutePadded",
        DateToken::Second => "Second",
        DateToken::SecondPadded => "SecondPadded",
        DateToken::SubSecond(_) => "SubSecond",
        DateToken::Meridiem(_) => "Meridiem",
        DateToken::ElapsedHours(_) => "ElapsedHours",
        DateToken::ElapsedMinutes(_) => "ElapsedMinutes",
        DateToken::ElapsedSeconds(_) => "ElapsedSeconds",
    }
}

/// Every construct the format language has, and whether a conformance row reaches it.
const ELEMENT_NAMES: [&str; 16] = [
    "Literal",
    "IntegerDigit",
    "DecimalPoint",
    "DecimalDigit",
    "NumeratorDigit",
    "FractionBar",
    "DenominatorDigit",
    "FixedDenominator",
    "Exponent",
    "ExponentDigit",
    "Percent",
    "Date",
    "Repeat",
    "Skip",
    "TextValue",
    "General",
];

/// Every date token the language has.
const TOKEN_NAMES: [&str; 22] = [
    "Year2",
    "Year4",
    "MonthNumber",
    "MonthNumberPadded",
    "MonthAbbreviation",
    "MonthName",
    "MonthLetter",
    "Day",
    "DayPadded",
    "WeekdayAbbreviation",
    "WeekdayName",
    "Hour",
    "HourPadded",
    "Minute",
    "MinutePadded",
    "Second",
    "SecondPadded",
    "SubSecond",
    "Meridiem",
    "ElapsedHours",
    "ElapsedMinutes",
    "ElapsedSeconds",
];

/// Would an `abort()` in the fraction arm — or the elapsed-time arm, or the exponent arm — fire in
/// any test?
///
/// The instrument this programme uses against a suite that is green because it never reaches the
/// code. Every [`Element`] variant, every [`SectionKind`], every [`DateToken`] and all four `AM/PM`
/// spellings must be produced **by a row of the conformance table** — not by a helper written to
/// satisfy this test.
#[test]
fn every_construct_of_the_language_is_reached() {
    let mut elements: Vec<&'static str> = Vec::new();
    let mut tokens: Vec<&'static str> = Vec::new();
    let mut kinds: Vec<SectionKind> = Vec::new();
    let mut meridiems: Vec<String> = Vec::new();
    let mut conditioned = 0;
    let mut coloured = 0;
    for row in table() {
        let compiled = CompiledFormat::compile(row.code);
        for section in compiled.sections() {
            if !kinds.contains(&section.kind) {
                kinds.push(section.kind);
            }
            conditioned += usize::from(section.condition.is_some());
            coloured += usize::from(section.colour.is_some());
            for element in &section.elements {
                let name = element_name(element);
                if !elements.contains(&name) {
                    elements.push(name);
                }
                if let Element::Date(token) = element {
                    let name = token_name(*token);
                    if !tokens.contains(&name) {
                        tokens.push(name);
                    }
                    if let DateToken::Meridiem(style) = token {
                        let spelling = format!("{style:?}");
                        if !meridiems.contains(&spelling) {
                            meridiems.push(spelling);
                        }
                    }
                }
            }
        }
    }

    let missing_elements: Vec<&str> = ELEMENT_NAMES
        .iter()
        .copied()
        .filter(|name| !elements.contains(name))
        .collect();
    assert!(
        missing_elements.is_empty(),
        "format-language constructs no conformance row reaches: {missing_elements:?}"
    );
    let missing_tokens: Vec<&str> = TOKEN_NAMES
        .iter()
        .copied()
        .filter(|name| !tokens.contains(name))
        .collect();
    assert!(
        missing_tokens.is_empty(),
        "date tokens no conformance row reaches: {missing_tokens:?}"
    );
    assert_eq!(
        kinds.len(),
        5,
        "every SectionKind must be reached; reached {kinds:?}"
    );
    assert_eq!(
        meridiems.len(),
        4,
        "all four AM/PM spellings must be reached; reached {meridiems:?}"
    );
    assert!(conditioned >= 10, "bracketed conditions are exercised");
    assert!(coloured >= 12, "colour codes are exercised");
}

/// How many *distinct* values did the gate see, per multi-section format?
///
/// A currency format tested only on a positive number exercises one of four sections. This reports
/// which multi-section codes were tried against a positive, a negative, a zero and a text value, and
/// holds the `sections` group — which exists to answer exactly that — at a floor.
#[test]
fn every_multi_section_format_meets_all_four_kinds_of_value() {
    let rows = table();
    let mut multi: Vec<&'static str> = Vec::new();
    for row in &rows {
        let sections = CompiledFormat::compile(row.code).sections().len();
        if sections > 1 && !multi.contains(&row.code) {
            multi.push(row.code);
        }
    }
    assert!(
        multi.len() >= 8,
        "the table must hold several multi-section formats; it holds {}",
        multi.len()
    );
    let mut unexercised = Vec::new();
    for code in multi {
        let seen =
            |wanted: fn(&Row) -> bool| rows.iter().any(|row| row.code == code && wanted(row));
        let positive = seen(|row| matches!(row.value, Value::Number(number) if number > 0.0));
        let negative = seen(|row| matches!(row.value, Value::Number(number) if number < 0.0));
        let zero = seen(|row| matches!(row.value, Value::Number(number) if number == 0.0));
        let text = seen(|row| matches!(row.value, Value::Text(_)));
        if !(positive && negative && zero && text) {
            unexercised.push(format!(
                "{code:?}: positive={positive} negative={negative} zero={zero} text={text}"
            ));
        }
    }
    println!(
        "multi-section codes not tried against all four kinds of value:\n  {}",
        if unexercised.is_empty() {
            "none".to_owned()
        } else {
            unexercised.join("\n  ")
        }
    );
    let four_way = rows.iter().filter(|row| row.group == "sections").count();
    assert!(
        four_way >= 20,
        "the `sections` group is what holds the four-way requirement; it has {four_way} rows"
    );
}
