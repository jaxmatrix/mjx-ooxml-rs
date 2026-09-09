//! The format-code parser: `numFmt@formatCode` — a string — into a [`CompiledFormat`].
//!
//! # Two phases, because one is not enough
//!
//! A format code cannot be classified while it is being read. `,` is a thousands separator when a
//! digit placeholder follows it and a division by a thousand when none does; `/` is a fraction bar
//! between two digit runs and a literal everywhere else; `.` introduces a decimal fraction in a
//! number and a sub-second in a time; and `m` is a month or a minute depending on what stands next
//! to it. Every one of those answers needs the tokens that come *after* the one being read.
//!
//! So the parser reads a section into a flat token list first and classifies it second. The
//! classification pass is where a [`SectionKind`] is decided and where each digit placeholder learns
//! whether it is an integer, a decimal, a numerator, a denominator or an exponent digit.
//!
//! # Nothing here fails
//!
//! A malformed format code is common in real files — a truncated bracket, an unterminated quote, a
//! section that is only a colour — and refusing to draw a sheet because one cell's style is odd is
//! never the right answer. Every branch that cannot make sense of what it is reading falls back to a
//! literal or to `General`, so [`CompiledFormat::compile`] is total: it returns a format, always.

use super::datetime::{DateToken, MeridiemStyle};

/// How wide a section list is allowed to get before the tail is ignored.
///
/// ECMA-376 Part 1 §18.8.31 defines four sections and Excel writes at most four. A file that writes
/// forty is malformed; keeping the first four and dropping the rest is the degradation this module
/// exists to do, and the bound is what stops a pathological code costing an unbounded parse.
const MAX_SECTIONS: usize = 4;

/// How long a format code may be before the parser stops reading it.
///
/// Excel's own limit on a format code is 255 characters. This is far past that and exists only so
/// that a hostile file cannot make the parser walk a megabyte.
const MAX_FORMAT_CODE_CHARS: usize = 4096;

/// One digit position in a format code — `0`, `#` or `?`.
///
/// The three differ only in what they do when the value has no digit for them: `0` writes a zero,
/// `?` writes a space (which is what aligns a column of fractions), and `#` writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placeholder {
    /// `0` — a digit, or a literal zero when the value has none.
    Zero,
    /// `#` — a digit, or nothing when the value has none.
    Hash,
    /// `?` — a digit, or a space when the value has none.
    Space,
}

impl Placeholder {
    /// What this placeholder writes when the value supplies no digit for it.
    #[must_use]
    pub fn padding(self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::Hash => "",
            Self::Space => " ",
        }
    }
}

/// How a fraction section states its denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denominator {
    /// `?/?`, `??/??` — as many placeholders as the code writes, so the denominator is chosen to be
    /// the best one of at most that many digits.
    Placeholders(u32),
    /// `?/8`, `??/100` — the code names the denominator outright and the numerator is rounded to it.
    Fixed(u32),
}

/// The comparison a bracketed condition makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// `[<n]`.
    Less,
    /// `[<=n]`.
    LessOrEqual,
    /// `[>n]`.
    Greater,
    /// `[>=n]`.
    GreaterOrEqual,
    /// `[=n]`.
    Equal,
    /// `[<>n]`.
    NotEqual,
}

/// A bracketed condition — `[>=100]` — which replaces the positional section semantics entirely.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Condition {
    /// Which comparison.
    pub comparison: Comparison,
    /// What it compares against.
    pub threshold: f64,
}

impl Condition {
    /// Whether `value` satisfies this condition.
    #[must_use]
    pub fn holds(self, value: f64) -> bool {
        match self.comparison {
            Comparison::Less => value < self.threshold,
            Comparison::LessOrEqual => value <= self.threshold,
            Comparison::Greater => value > self.threshold,
            Comparison::GreaterOrEqual => value >= self.threshold,
            #[allow(clippy::float_cmp)]
            Comparison::Equal => value == self.threshold,
            #[allow(clippy::float_cmp)]
            Comparison::NotEqual => value != self.threshold,
        }
    }
}

/// What a section renders, once its digit placeholders have been given their roles.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// Characters that pass through unchanged — quoted text, an escaped character, a currency sign,
    /// a separator, or anything the parser could not read as a directive.
    Literal(String),
    /// A digit position left of the decimal point.
    IntegerDigit(Placeholder),
    /// The decimal point itself.
    DecimalPoint,
    /// A digit position right of the decimal point.
    DecimalDigit(Placeholder),
    /// A digit position in a fraction's numerator.
    NumeratorDigit(Placeholder),
    /// The `/` of a fraction.
    FractionBar,
    /// A digit position in a fraction's denominator, for [`Denominator::Placeholders`].
    DenominatorDigit(Placeholder),
    /// The denominator a `?/8` states outright.
    ///
    /// Lifted out of the literal that carried its digits, so that every renderer writes a fraction's
    /// three parts the same way and the blank a whole number leaves is one branch rather than two.
    FixedDenominator(String),
    /// `E+` or `E-` — with `positive_sign` recording whether a positive exponent is written with a
    /// `+` (`E+`) or with nothing (`E-`).
    Exponent {
        /// Whether a positive exponent carries a `+`.
        positive_sign: bool,
    },
    /// A digit position in the exponent.
    ExponentDigit(Placeholder),
    /// `%` — which both scales the value by a hundred and writes itself.
    Percent,
    /// A date or time token.
    Date(DateToken),
    /// `*c` — repeat `c` until the cell is full. See [`super::FormattedValue::repeat`] for why the
    /// character is reported rather than expanded.
    Repeat(char),
    /// `_c` — a gap the width of `c`.
    Skip(char),
    /// `@` — the cell's text.
    TextValue,
    /// The literal word `General`.
    General,
}

/// What kind of thing a section renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    /// Digits, a decimal point, grouping and scaling.
    Number,
    /// A mantissa and an exponent.
    Scientific,
    /// A whole part and a vulgar fraction.
    Fraction,
    /// Date and time tokens.
    DateTime,
    /// Nothing but literals and `@` — a section with no digit position anywhere in it.
    Literal,
}

/// One `;`-separated section of a format code, compiled.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// `[>100]` — the condition that selects this section, when the code states one.
    pub condition: Option<Condition>,
    /// `[Red]`, `[Color12]` — the row of the legacy indexed palette this section's text is drawn in,
    /// **zero-based**, or `None` when the code names no colour.
    pub colour: Option<u32>,
    /// What it renders, in order.
    pub elements: Vec<Element>,
    /// Which renderer runs.
    pub kind: SectionKind,
    /// Whether the integer part is grouped in thousands.
    pub grouped: bool,
    /// How many times the value is divided by a thousand, from trailing commas.
    pub scale_by_thousands: u32,
    /// How many times the value is multiplied by a hundred, from `%` signs.
    pub percent_multiplier: u32,
    /// How many integer digit positions there are.
    pub integer_places: usize,
    /// How many decimal digit positions there are.
    pub decimal_places: usize,
    /// How many numerator digit positions there are.
    pub numerator_places: u32,
    /// How the denominator is stated, for a [`SectionKind::Fraction`].
    pub denominator: Denominator,
    /// How many exponent digit positions there are.
    pub exponent_places: usize,
    /// Whether the section carries an `AM/PM`, which is what puts the clock on twelve hours.
    pub has_meridiem: bool,
    /// The `*c` fill character, when the section names one.
    pub repeat: Option<char>,
}

impl Section {
    /// The section a code that states nothing at all means: `General`.
    #[must_use]
    pub fn general() -> Self {
        Self {
            condition: None,
            colour: None,
            elements: vec![Element::General],
            kind: SectionKind::Literal,
            grouped: false,
            scale_by_thousands: 0,
            percent_multiplier: 0,
            integer_places: 0,
            decimal_places: 0,
            numerator_places: 0,
            denominator: Denominator::Placeholders(1),
            exponent_places: 0,
            has_meridiem: false,
            repeat: None,
        }
    }

    /// Whether the section renders nothing whatsoever — the `;;` idiom that hides a value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// A whole `formatCode`, compiled.
///
/// Holds between one and four [`Section`]s. Section *selection* — which one a given value renders
/// through, and when the sign is stripped — is [`select`](super::select)'s subject rather than this
/// module's, because it depends on the value and this depends only on the code.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledFormat {
    sections: Vec<Section>,
}

impl CompiledFormat {
    /// The compiled form of `code`.
    ///
    /// Never fails: see the [module documentation](self).
    #[must_use]
    pub fn compile(code: &str) -> Self {
        parse(code)
    }

    /// What `General` compiles to.
    #[must_use]
    pub fn general() -> Self {
        Self {
            sections: vec![Section::general()],
        }
    }

    /// Its sections, in the order the code wrote them.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Whether any section carries a bracketed condition, which is what replaces the positional
    /// *positive; negative; zero* semantics with *first match wins*.
    #[must_use]
    pub fn is_conditional(&self) -> bool {
        self.sections
            .iter()
            .any(|section| section.condition.is_some())
    }
}

/// Splits `code` into sections and compiles each.
fn parse(code: &str) -> CompiledFormat {
    let mut sections = Vec::new();
    for piece in split_sections(code) {
        if sections.len() == MAX_SECTIONS {
            break;
        }
        sections.push(compile_section(&piece));
    }
    if sections.is_empty() {
        sections.push(Section::general());
    }
    CompiledFormat { sections }
}

/// Splits on the `;` that are not inside a quoted string, a bracket, or escaped by a backslash.
fn split_sections(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut bracketed = false;
    let mut escaped = false;
    for ch in code.chars().take(MAX_FORMAT_CODE_CHARS) {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => {
                current.push(ch);
                escaped = true;
            }
            '"' => {
                quoted = !quoted;
                current.push(ch);
            }
            '[' if !quoted => {
                bracketed = true;
                current.push(ch);
            }
            ']' if !quoted => {
                bracketed = false;
                current.push(ch);
            }
            ';' if !quoted && !bracketed => {
                out.push(std::mem::take(&mut current));
                if out.len() == MAX_SECTIONS {
                    return out;
                }
            }
            _ => current.push(ch),
        }
    }
    out.push(current);
    out
}

/// A token before its role is known.
#[derive(Debug, Clone, PartialEq)]
enum Raw {
    Literal(String),
    Digit(Placeholder),
    Point,
    Comma,
    Percent,
    Slash,
    Exponent { positive_sign: bool },
    Date(DateToken),
    Repeat(char),
    Skip(char),
    TextValue,
    General,
}

/// Reads one section's characters into [`Raw`] tokens, lifting the condition and the colour out of
/// their brackets as it goes.
#[allow(clippy::too_many_lines)]
fn tokenise(text: &str) -> (Vec<Raw>, Option<Condition>, Option<u32>) {
    let chars: Vec<char> = text.chars().collect();
    let mut raw = Vec::new();
    let mut condition = None;
    let mut colour = None;
    let mut at = 0;
    while at < chars.len() {
        let ch = chars[at];
        match ch {
            '\\' => {
                at += 1;
                if let Some(next) = chars.get(at) {
                    push_literal(&mut raw, *next);
                    at += 1;
                }
            }
            '"' => {
                at += 1;
                let mut literal = String::new();
                while at < chars.len() && chars[at] != '"' {
                    literal.push(chars[at]);
                    at += 1;
                }
                // An unterminated quote is a malformed code; taking what is there rather than
                // dropping it keeps the currency sign a truncated `"$` was reaching for.
                at += usize::from(at < chars.len());
                raw.push(Raw::Literal(literal));
            }
            '_' => {
                at += 1;
                let skipped = chars.get(at).copied().unwrap_or(' ');
                at += usize::from(at < chars.len());
                raw.push(Raw::Skip(skipped));
            }
            '*' => {
                at += 1;
                let repeated = chars.get(at).copied().unwrap_or(' ');
                at += usize::from(at < chars.len());
                raw.push(Raw::Repeat(repeated));
            }
            '0' => {
                raw.push(Raw::Digit(Placeholder::Zero));
                at += 1;
            }
            '#' => {
                raw.push(Raw::Digit(Placeholder::Hash));
                at += 1;
            }
            '?' => {
                raw.push(Raw::Digit(Placeholder::Space));
                at += 1;
            }
            '.' => {
                raw.push(Raw::Point);
                at += 1;
            }
            ',' => {
                raw.push(Raw::Comma);
                at += 1;
            }
            '%' => {
                raw.push(Raw::Percent);
                at += 1;
            }
            '/' => {
                raw.push(Raw::Slash);
                at += 1;
            }
            '@' => {
                raw.push(Raw::TextValue);
                at += 1;
            }
            '[' => {
                let close = chars[at..].iter().position(|c| *c == ']');
                let Some(close) = close else {
                    // No closing bracket. Turning the rest into literal text would **hide the
                    // value**: `[Red0.00` would render as the six characters `[Red0.00` and the
                    // reader's number would be nowhere on the screen. So the remainder is dropped,
                    // and a section that is left with nothing at all falls back to `General` —
                    // which is the degradation the whole engine is written around.
                    if raw.is_empty() {
                        raw.push(Raw::General);
                    }
                    at = chars.len();
                    continue;
                };
                let inner: String = chars[at + 1..at + close].iter().collect();
                at += close + 1;
                apply_bracket(&inner, &mut raw, &mut condition, &mut colour);
            }
            'E' | 'e' => {
                match chars.get(at + 1) {
                    Some('+') => {
                        raw.push(Raw::Exponent {
                            positive_sign: true,
                        });
                        at += 2;
                    }
                    Some('-') => {
                        raw.push(Raw::Exponent {
                            positive_sign: false,
                        });
                        at += 2;
                    }
                    // A bare `e` is the Japanese era year, which this engine does not implement;
                    // see `super`'s documentation for the list.
                    _ => {
                        push_literal(&mut raw, ch);
                        at += 1;
                    }
                }
            }
            'G' | 'g' => {
                if matches_ignoring_case(&chars, at, "general") {
                    raw.push(Raw::General);
                    at += "general".len();
                } else {
                    // `g`, `gg`, `ggg` are the Japanese era name, not implemented; it passes
                    // through as a literal rather than silently vanishing.
                    push_literal(&mut raw, ch);
                    at += 1;
                }
            }
            'A' | 'a' => {
                if let Some((token, width)) = read_meridiem(&chars, at) {
                    raw.push(Raw::Date(token));
                    at += width;
                } else {
                    push_literal(&mut raw, ch);
                    at += 1;
                }
            }
            'y' | 'Y' | 'm' | 'M' | 'd' | 'D' | 'h' | 'H' | 's' | 'S' => {
                let run = run_length(&chars, at, ch);
                raw.push(Raw::Date(date_token(ch, run)));
                at += run;
            }
            _ => {
                push_literal(&mut raw, ch);
                at += 1;
            }
        }
    }
    (raw, condition, colour)
}

/// Appends one character to the trailing literal, starting a new one when the last token is not a
/// literal. Keeping runs of literal characters in one token is what makes the classification pass's
/// lookaheads cheap and its output readable.
fn push_literal(raw: &mut Vec<Raw>, ch: char) {
    if let Some(Raw::Literal(last)) = raw.last_mut() {
        last.push(ch);
    } else {
        raw.push(Raw::Literal(ch.to_string()));
    }
}

/// Whether `chars[at..]` begins with `needle`, ignoring ASCII case.
fn matches_ignoring_case(chars: &[char], at: usize, needle: &str) -> bool {
    let mut expected = needle.chars();
    let mut index = at;
    loop {
        let Some(want) = expected.next() else {
            return true;
        };
        let Some(found) = chars.get(index) else {
            return false;
        };
        if !found.eq_ignore_ascii_case(&want) {
            return false;
        }
        index += 1;
    }
}

/// How many times `ch` repeats from `at`, case-insensitively.
fn run_length(chars: &[char], at: usize, ch: char) -> usize {
    let mut length = 0;
    while chars
        .get(at + length)
        .is_some_and(|found| found.eq_ignore_ascii_case(&ch))
    {
        length += 1;
    }
    length.max(1)
}

/// The `AM/PM`, `A/P` token at `at`, and how many characters it occupies.
fn read_meridiem(chars: &[char], at: usize) -> Option<(DateToken, usize)> {
    let upper = chars.get(at).is_some_and(char::is_ascii_uppercase);
    if matches_ignoring_case(chars, at, "am/pm") {
        let style = if upper {
            MeridiemStyle::UpperLong
        } else {
            MeridiemStyle::LowerLong
        };
        return Some((DateToken::Meridiem(style), 5));
    }
    if matches_ignoring_case(chars, at, "a/p") {
        let style = if upper {
            MeridiemStyle::UpperShort
        } else {
            MeridiemStyle::LowerShort
        };
        return Some((DateToken::Meridiem(style), 3));
    }
    None
}

/// The date token a run of `run` copies of `ch` names.
///
/// `m` is resolved to a month here and corrected to a minute by [`disambiguate_minutes`], which is
/// the only pass that can see what stands beside it.
fn date_token(ch: char, run: usize) -> DateToken {
    match ch.to_ascii_lowercase() {
        'y' => {
            if run >= 3 {
                DateToken::Year4
            } else {
                DateToken::Year2
            }
        }
        'm' => match run {
            1 => DateToken::MonthNumber,
            2 => DateToken::MonthNumberPadded,
            3 => DateToken::MonthAbbreviation,
            4 => DateToken::MonthName,
            _ => DateToken::MonthLetter,
        },
        'd' => match run {
            1 => DateToken::Day,
            2 => DateToken::DayPadded,
            3 => DateToken::WeekdayAbbreviation,
            _ => DateToken::WeekdayName,
        },
        'h' => {
            if run >= 2 {
                DateToken::HourPadded
            } else {
                DateToken::Hour
            }
        }
        _ => {
            if run >= 2 {
                DateToken::SecondPadded
            } else {
                DateToken::Second
            }
        }
    }
}

/// Reads one `[...]` — a condition, a colour, an elapsed-time token, a locale or a currency.
fn apply_bracket(
    inner: &str,
    raw: &mut Vec<Raw>,
    condition: &mut Option<Condition>,
    colour: &mut Option<u32>,
) {
    if inner.is_empty() {
        return;
    }
    if let Some(found) = parse_condition(inner) {
        // A code with two conditions in one section is malformed; the first wins, so the answer does
        // not depend on which branch of a search happens to run.
        if condition.is_none() {
            *condition = Some(found);
        }
        return;
    }
    if let Some(found) = parse_colour(inner) {
        if colour.is_none() {
            *colour = Some(found);
        }
        return;
    }
    if let Some(token) = parse_elapsed(inner) {
        raw.push(Raw::Date(token));
        return;
    }
    // `[$-409]` is a locale and writes nothing; `[$USD-409]` and `[$€-x-euro2]` carry a currency
    // sign before the `-`, which is the whole reason the construct exists.
    //
    // Anything that reaches neither branch — a locale id with no `$`, a producer extension, a typo —
    // writes nothing at all. That is the degradation; writing the brackets would put `[foo]` on a
    // person's screen.
    if let Some(rest) = inner.strip_prefix('$') {
        let symbol = rest.split('-').next().unwrap_or("");
        if !symbol.is_empty() {
            raw.push(Raw::Literal(symbol.to_owned()));
        }
    }
}

/// The condition `inner` states, or `None` when it states none.
fn parse_condition(inner: &str) -> Option<Condition> {
    let (comparison, rest) = if let Some(rest) = inner.strip_prefix("<=") {
        (Comparison::LessOrEqual, rest)
    } else if let Some(rest) = inner.strip_prefix(">=") {
        (Comparison::GreaterOrEqual, rest)
    } else if let Some(rest) = inner.strip_prefix("<>") {
        (Comparison::NotEqual, rest)
    } else if let Some(rest) = inner.strip_prefix('<') {
        (Comparison::Less, rest)
    } else if let Some(rest) = inner.strip_prefix('>') {
        (Comparison::Greater, rest)
    } else {
        // `?` rather than `else { return None }`, which clippy reads as the same thing written
        // longer: a bracket beginning with none of the six operators is not a condition at all.
        (Comparison::Equal, inner.strip_prefix('=')?)
    };
    let threshold = rest.trim().parse::<f64>().ok()?;
    if !threshold.is_finite() {
        return None;
    }
    Some(Condition {
        comparison,
        threshold,
    })
}

/// The zero-based indexed-palette row `inner` names, or `None` when it names no colour.
///
/// The eight names and `[ColorN]` address the same table: ECMA-376 Part 1 §18.8.27's `indexedColors`
/// runs black, white, red, green, blue, yellow, magenta, cyan from row zero, and `[Color1]` is that
/// first row. So `[Red]` and `[Color3]` are the same answer and there is one table rather than two.
fn parse_colour(inner: &str) -> Option<u32> {
    let trimmed = inner.trim();
    let named = |name: &str| trimmed.eq_ignore_ascii_case(name);
    if named("black") {
        return Some(0);
    }
    if named("white") {
        return Some(1);
    }
    if named("red") {
        return Some(2);
    }
    if named("green") {
        return Some(3);
    }
    if named("blue") {
        return Some(4);
    }
    if named("yellow") {
        return Some(5);
    }
    if named("magenta") {
        return Some(6);
    }
    if named("cyan") {
        return Some(7);
    }
    let rest = if trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("color") {
        trimmed[5..].trim()
    } else {
        return None;
    };
    let index = rest.parse::<u32>().ok()?;
    // `[Color0]` is not a colour: the construct is one-based, and the palette's fifty-sixth row is
    // the last one a `numFmt` may name.
    (1..=56).contains(&index).then(|| index - 1)
}

/// The elapsed-time token `inner` names — `[h]`, `[mm]`, `[ss]` — or `None`.
fn parse_elapsed(inner: &str) -> Option<DateToken> {
    let mut chars = inner.chars();
    let first = chars.next()?;
    let width = u8::try_from(inner.chars().count()).ok()?;
    if !inner.chars().all(|ch| ch.eq_ignore_ascii_case(&first)) {
        return None;
    }
    match first.to_ascii_lowercase() {
        'h' => Some(DateToken::ElapsedHours(width)),
        'm' => Some(DateToken::ElapsedMinutes(width)),
        's' => Some(DateToken::ElapsedSeconds(width)),
        _ => None,
    }
}

/// Turns one section's [`Raw`] tokens into a [`Section`].
#[allow(clippy::too_many_lines)]
fn compile_section(text: &str) -> Section {
    let (mut raw, condition, colour) = tokenise(text);
    let has_date = raw.iter().any(|token| matches!(token, Raw::Date(_)));
    if has_date {
        disambiguate_minutes(&mut raw);
        return compile_datetime(raw, condition, colour);
    }
    let fraction_bar = fraction_bar_at(&raw);
    let point_at = raw.iter().position(|token| matches!(token, Raw::Point));
    let exponent_at = raw
        .iter()
        .position(|token| matches!(token, Raw::Exponent { .. }));

    let mut section = Section {
        condition,
        colour,
        elements: Vec::new(),
        kind: SectionKind::Literal,
        grouped: false,
        scale_by_thousands: 0,
        percent_multiplier: 0,
        integer_places: 0,
        decimal_places: 0,
        numerator_places: 0,
        denominator: Denominator::Placeholders(1),
        exponent_places: 0,
        has_meridiem: false,
        repeat: None,
    };

    // The numerator run is the digits immediately before the fraction bar; everything before that
    // run is the whole part.
    let numerator_from = fraction_bar.map(|bar| digit_run_start(&raw, bar));

    for (index, token) in raw.iter().enumerate() {
        match token {
            Raw::Literal(text) => section.elements.push(Element::Literal(text.clone())),
            Raw::Digit(placeholder) => {
                let element = if let Some(bar) = fraction_bar {
                    if index > bar {
                        section
                            .elements
                            .push(Element::DenominatorDigit(*placeholder));
                        continue;
                    } else if numerator_from.is_some_and(|from| index >= from) {
                        section.numerator_places += 1;
                        Element::NumeratorDigit(*placeholder)
                    } else {
                        section.integer_places += 1;
                        Element::IntegerDigit(*placeholder)
                    }
                } else if exponent_at.is_some_and(|exponent| index > exponent) {
                    section.exponent_places += 1;
                    Element::ExponentDigit(*placeholder)
                } else if point_at.is_some_and(|point| index > point) {
                    section.decimal_places += 1;
                    Element::DecimalDigit(*placeholder)
                } else {
                    section.integer_places += 1;
                    Element::IntegerDigit(*placeholder)
                };
                section.elements.push(element);
            }
            Raw::Point => section.elements.push(Element::DecimalPoint),
            Raw::Comma => {
                // A comma with a digit still to come groups; one with none divides by a thousand.
                if raw[index + 1..]
                    .iter()
                    .any(|later| matches!(later, Raw::Digit(_)))
                {
                    section.grouped = true;
                } else {
                    section.scale_by_thousands += 1;
                }
            }
            Raw::Percent => {
                section.percent_multiplier += 1;
                section.elements.push(Element::Percent);
            }
            Raw::Slash => {
                if fraction_bar == Some(index) {
                    section.elements.push(Element::FractionBar);
                } else {
                    section.elements.push(Element::Literal("/".to_owned()));
                }
            }
            Raw::Exponent { positive_sign } => section.elements.push(Element::Exponent {
                positive_sign: *positive_sign,
            }),
            Raw::Date(token) => section.elements.push(Element::Date(*token)),
            Raw::Repeat(ch) => {
                section.repeat.get_or_insert(*ch);
                section.elements.push(Element::Repeat(*ch));
            }
            Raw::Skip(ch) => section.elements.push(Element::Skip(*ch)),
            Raw::TextValue => section.elements.push(Element::TextValue),
            Raw::General => section.elements.push(Element::General),
        }
    }

    if let Some(bar) = fraction_bar {
        section.kind = SectionKind::Fraction;
        section.denominator = read_denominator(&raw, bar);
        if let Denominator::Fixed(fixed) = section.denominator {
            lift_fixed_denominator(&mut section.elements, fixed);
        }
    } else if exponent_at.is_some() {
        section.kind = SectionKind::Scientific;
    } else if section.integer_places > 0 || section.decimal_places > 0 {
        section.kind = SectionKind::Number;
    }
    section
}

/// Replaces the digits a fixed denominator wrote as a literal with an [`Element::FixedDenominator`].
///
/// `?/8` tokenises the `8` as an ordinary literal, and leaving it there would make a whole number's
/// blanked fraction impossible to write: the denominator has to be recognisable as a denominator.
fn lift_fixed_denominator(elements: &mut Vec<Element>, fixed: u32) {
    let Some(bar) = elements
        .iter()
        .position(|element| matches!(element, Element::FractionBar))
    else {
        return;
    };
    let _ = bar;
    elements.retain(|element| !matches!(element, Element::DenominatorDigit(_)));
    let Some(bar) = elements
        .iter()
        .position(|element| matches!(element, Element::FractionBar))
    else {
        return;
    };
    let digits = fixed.to_string();
    match elements.get_mut(bar + 1) {
        Some(Element::Literal(text)) => {
            let rest: String = text.chars().skip_while(|ch| ch.is_ascii_digit()).collect();
            if rest.is_empty() {
                elements[bar + 1] = Element::FixedDenominator(digits);
            } else {
                elements[bar + 1] = Element::Literal(rest);
                elements.insert(bar + 1, Element::FixedDenominator(digits));
            }
        }
        _ => elements.insert(bar + 1, Element::FixedDenominator(digits)),
    }
}

/// The index of the `/` that is a fraction bar, or `None` when the section holds none.
///
/// A fraction bar has a digit placeholder immediately before it and a digit placeholder or a literal
/// digit immediately after. `mm/dd` never reaches here — a section with a date token is compiled by
/// [`compile_datetime`] — and `0/0` is a fraction, which is what Excel makes of it too.
fn fraction_bar_at(raw: &[Raw]) -> Option<usize> {
    raw.iter().enumerate().find_map(|(index, token)| {
        if !matches!(token, Raw::Slash) {
            return None;
        }
        let before = index.checked_sub(1).and_then(|before| raw.get(before))?;
        if !matches!(before, Raw::Digit(_)) {
            return None;
        }
        match raw.get(index + 1)? {
            Raw::Digit(_) => Some(index),
            Raw::Literal(text) if text.starts_with(|ch: char| ch.is_ascii_digit()) => Some(index),
            _ => None,
        }
    })
}

/// Where the run of digit placeholders that ends at `bar - 1` begins.
fn digit_run_start(raw: &[Raw], bar: usize) -> usize {
    let mut start = bar;
    while start > 0 && matches!(raw.get(start - 1), Some(Raw::Digit(_))) {
        start -= 1;
    }
    start
}

/// How the denominator after `bar` is stated.
fn read_denominator(raw: &[Raw], bar: usize) -> Denominator {
    if let Some(Raw::Literal(text)) = raw.get(bar + 1) {
        let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(fixed) = digits.parse::<u32>() {
            if fixed > 0 {
                return Denominator::Fixed(fixed);
            }
        }
    }
    let places = raw[bar + 1..]
        .iter()
        .take_while(|token| matches!(token, Raw::Digit(_)))
        .count();
    Denominator::Placeholders(u32::try_from(places).unwrap_or(1).max(1))
}

/// Compiles a section that holds at least one date or time token.
fn compile_datetime(raw: Vec<Raw>, condition: Option<Condition>, colour: Option<u32>) -> Section {
    let has_meridiem = raw
        .iter()
        .any(|token| matches!(token, Raw::Date(DateToken::Meridiem(_))));
    let mut elements: Vec<Element> = Vec::new();
    let mut at = 0;
    while at < raw.len() {
        match &raw[at] {
            // `.0`, `.00`, `.000` after a second is a sub-second, and is the one place a digit
            // placeholder means something inside a time.
            Raw::Point => {
                let zeros = raw[at + 1..]
                    .iter()
                    .take_while(|token| matches!(token, Raw::Digit(Placeholder::Zero)))
                    .count();
                if zeros > 0 && follows_a_second(&elements) {
                    let digits = u8::try_from(zeros).unwrap_or(u8::MAX);
                    elements.push(Element::Date(DateToken::SubSecond(digits)));
                    at += zeros + 1;
                    continue;
                }
                elements.push(Element::Literal(".".to_owned()));
                at += 1;
            }
            Raw::Digit(placeholder) => {
                elements.push(Element::Literal(
                    match placeholder {
                        Placeholder::Zero => "0",
                        Placeholder::Hash => "#",
                        Placeholder::Space => "?",
                    }
                    .to_owned(),
                ));
                at += 1;
            }
            Raw::Literal(text) => {
                elements.push(Element::Literal(text.clone()));
                at += 1;
            }
            Raw::Comma => {
                elements.push(Element::Literal(",".to_owned()));
                at += 1;
            }
            Raw::Slash => {
                elements.push(Element::Literal("/".to_owned()));
                at += 1;
            }
            Raw::Percent => {
                elements.push(Element::Literal("%".to_owned()));
                at += 1;
            }
            Raw::Exponent { positive_sign } => {
                elements.push(Element::Literal(
                    if *positive_sign { "E+" } else { "E-" }.to_owned(),
                ));
                at += 1;
            }
            Raw::Date(token) => {
                elements.push(Element::Date(*token));
                at += 1;
            }
            Raw::Repeat(ch) => {
                elements.push(Element::Repeat(*ch));
                at += 1;
            }
            Raw::Skip(ch) => {
                elements.push(Element::Skip(*ch));
                at += 1;
            }
            Raw::TextValue => {
                elements.push(Element::TextValue);
                at += 1;
            }
            Raw::General => {
                elements.push(Element::General);
                at += 1;
            }
        }
    }
    let repeat = elements.iter().find_map(|element| match element {
        Element::Repeat(ch) => Some(*ch),
        _ => None,
    });
    Section {
        condition,
        colour,
        elements,
        kind: SectionKind::DateTime,
        grouped: false,
        scale_by_thousands: 0,
        percent_multiplier: 0,
        integer_places: 0,
        decimal_places: 0,
        numerator_places: 0,
        denominator: Denominator::Placeholders(1),
        exponent_places: 0,
        has_meridiem,
        repeat,
    }
}

/// Whether the last non-literal element emitted was a seconds token.
fn follows_a_second(elements: &[Element]) -> bool {
    matches!(
        elements.last(),
        Some(Element::Date(DateToken::Second | DateToken::SecondPadded))
    ) || matches!(
        elements.last(),
        Some(Element::Date(DateToken::ElapsedSeconds(_)))
    )
}

/// Turns every `m` that means a *minute* into one.
///
/// The rule Excel documents: an `m` is a minute when it directly follows an hour token or directly
/// precedes a seconds token. "Directly" allows the separators between them — `h:mm:ss` has a colon
/// on each side of the `mm` and both of its neighbours are still the hour and the second — so the
/// scan skips literals while looking, and stops at anything else.
fn disambiguate_minutes(raw: &mut [Raw]) {
    let is_month = |token: &Raw| {
        matches!(
            token,
            Raw::Date(DateToken::MonthNumber | DateToken::MonthNumberPadded)
        )
    };
    for index in 0..raw.len() {
        if !is_month(&raw[index]) {
            continue;
        }
        let padded = matches!(raw[index], Raw::Date(DateToken::MonthNumberPadded));
        let after_hour = neighbour(raw, index, false).is_some_and(|token| {
            matches!(
                token,
                Raw::Date(DateToken::Hour | DateToken::HourPadded | DateToken::ElapsedHours(_))
            )
        });
        let before_second = neighbour(raw, index, true).is_some_and(|token| {
            matches!(
                token,
                Raw::Date(
                    DateToken::Second | DateToken::SecondPadded | DateToken::ElapsedSeconds(_)
                )
            )
        });
        if after_hour || before_second {
            raw[index] = Raw::Date(if padded {
                DateToken::MinutePadded
            } else {
                DateToken::Minute
            });
        }
    }
}

/// The nearest token on one side of `index` that is not a literal separator.
fn neighbour(raw: &[Raw], index: usize, forward: bool) -> Option<&Raw> {
    let mut at = index;
    loop {
        at = if forward {
            at.checked_add(1)?
        } else {
            at.checked_sub(1)?
        };
        match raw.get(at)? {
            Raw::Literal(_) | Raw::Skip(_) => {}
            found => return Some(found),
        }
    }
}
