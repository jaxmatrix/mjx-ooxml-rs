//! The fifteen rule kinds that **decide** rather than interpolate, and the one thing they cannot
//! decide.
//!
//! # There is no calculation engine, and this does not pretend otherwise
//!
//! `PLAN.md` settles the absence of a formula evaluator as scope. Two of the eighteen kinds are
//! stated as formulas — `expression` always, and `cellIs` whenever its operand is anything but a
//! literal — so those cannot be answered here. They are reported as
//! [`Decision::Unevaluated`] with the reason and the text of the formula, they appear in
//! [`PageCatalogue::unevaluated_rules`](crate::PageCatalogue::unevaluated_rules), and
//! they are **neither faked nor dropped**. A rule that quietly did not fire and a rule that could
//! not be evaluated look identical on a screen, which is precisely the failure this ticket exists
//! to avoid.
//!
//! A literal operand *is* answerable, and most `cellIs` rules a person writes have one: `>5`,
//! `="Overdue"`, `between 10 and 20`. Those are evaluated in full.
//!
//! # Where the answers come from
//!
//! Four of the kinds have an **external, checkable definition**, because Excel writes the formula it
//! means into the file beside the rule:
//!
//! | Kind | The formula Excel writes | Read as |
//! |---|---|---|
//! | `containsBlanks` | `LEN(TRIM(A1))=0` | a cell of nothing but spaces **is** blank |
//! | `notContainsBlanks` | `LEN(TRIM(A1))>0` | its negation |
//! | `containsErrors` | `ISERROR(A1)` | any error code |
//! | `timePeriod last7Days` | `AND(TODAY()-FLOOR(A1,1)<=6,FLOOR(A1,1)<=TODAY())` | `today-6 ..= today` |
//!
//! Those are DocumentedBehaviour. The rest are readings, and each is marked `GUESS:` at its site.

use mjx_ooxml_types::spreadsheetml::{
    ConditionalFormatType, ConditionalFormattingOperator, TimePeriod,
};
use mjx_xlsx::DateSystem;

use crate::condfmt::rules::CompiledRule;
use crate::condfmt::stats::RangeStatistics;
use crate::condfmt::value::RuleValue;
use crate::numfmt::datetime;

/// Why a rule could not be answered.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnevaluatedReason {
    /// The rule is an `expression`, whose condition is a formula. There is no calculation engine in
    /// this workspace and `PLAN.md` settles that as scope.
    NoFormulaEngine,
    /// The rule is a `cellIs` whose operand is not a literal — a cell reference, a name or a call.
    /// The same absence, reached from the other side.
    OperandIsNotALiteral,
    /// The rule states a kind this build does not decide. Nothing reaches this today; it exists so
    /// that a kind added to `ST_CfType` after this was written degrades visibly rather than
    /// silently answering *false*.
    UnknownKind,
    /// The rule states `@type` and nothing that kind needs — a `cellIs` with no operator, a
    /// `timePeriod` with no `@timePeriod`, a `top10` with no `@rank`.
    Incomplete,
}

impl UnevaluatedReason {
    /// A short sentence for a report or a ledger.
    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Self::NoFormulaEngine => {
                "the condition is a formula and there is no calculation engine"
            }
            Self::OperandIsNotALiteral => {
                "the operand is not a literal, so answering it needs a calculation engine"
            }
            Self::UnknownKind => "the rule states a kind this build does not decide",
            Self::Incomplete => "the rule omits an attribute its own kind requires",
        }
    }
}

/// What one rule said about one cell.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// The condition held.
    Fired,
    /// It did not.
    DidNotFire,
    /// It could not be decided, for this reason.
    Unevaluated(UnevaluatedReason),
}

/// Everything a predicate needs beyond the rule itself.
#[derive(Debug)]
pub struct Context<'a> {
    /// The value in the cell being asked about.
    pub value: &'a RuleValue,
    /// The statistics of the rule's own range, for the kinds that are not local.
    pub statistics: &'a RangeStatistics,
    /// The serial number of today, in the workbook's epoch — what `TODAY()` answers.
    pub today: f64,
    /// Which epoch that is.
    pub dates: DateSystem,
}

/// Decides one rule against one cell.
#[must_use]
pub fn decide(rule: &CompiledRule, context: &Context<'_>) -> Decision {
    let Some(kind) = rule.kind else {
        // A `cfRule` with no `@type` states no condition. It is not an error and it is not a
        // failure to evaluate: there is nothing to evaluate.
        return Decision::DidNotFire;
    };
    match kind {
        ConditionalFormatType::CellIs => cell_is(rule, context),
        ConditionalFormatType::Expression => expression(rule),
        ConditionalFormatType::Top10 => top_or_bottom(rule, context),
        ConditionalFormatType::AboveAverage => above_average(rule, context),
        ConditionalFormatType::DuplicateValues => occurrences(context, |count| count > 1),
        ConditionalFormatType::UniqueValues => occurrences(context, |count| count == 1),
        ConditionalFormatType::ContainsText => text_test(rule, context, TextTest::Contains, false),
        ConditionalFormatType::NotContainsText => {
            text_test(rule, context, TextTest::Contains, true)
        }
        ConditionalFormatType::BeginsWith => text_test(rule, context, TextTest::Begins, false),
        ConditionalFormatType::EndsWith => text_test(rule, context, TextTest::Ends, false),
        ConditionalFormatType::ContainsBlanks => fired(is_blank_by_excels_rule(context.value)),
        ConditionalFormatType::NotContainsBlanks => fired(!is_blank_by_excels_rule(context.value)),
        ConditionalFormatType::ContainsErrors => fired(context.value.is_error()),
        ConditionalFormatType::NotContainsErrors => fired(!context.value.is_error()),
        ConditionalFormatType::TimePeriod => time_period(rule, context),
        // The graded kinds have no condition at all: a colour scale applies to every cell of its
        // range. `super::evaluate` never routes one here, and answering `Fired` rather than
        // `Unevaluated` is what makes that true in both places.
        ConditionalFormatType::ColorScale
        | ConditionalFormatType::DataBar
        | ConditionalFormatType::IconSet => Decision::Fired,
    }
}

/// `Fired` or `DidNotFire`.
fn fired(held: bool) -> Decision {
    if held {
        Decision::Fired
    } else {
        Decision::DidNotFire
    }
}

/// `containsBlanks`, by the formula Excel writes beside it: `LEN(TRIM(A1))=0`.
///
/// DocumentedBehaviour. A cell holding three spaces is blank for this rule and populated for
/// everything else on the sheet, which is a genuine asymmetry rather than an inconsistency here.
fn is_blank_by_excels_rule(value: &RuleValue) -> bool {
    match value {
        RuleValue::Blank => true,
        RuleValue::Text(text) => text.trim().is_empty(),
        RuleValue::Number(_) | RuleValue::Boolean(_) | RuleValue::Error(_) => false,
    }
}

/// `cellIs` — twelve operators over one or two literal operands.
fn cell_is(rule: &CompiledRule, context: &Context<'_>) -> Decision {
    let Some(operator) = rule.operator else {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    };
    if context.value.is_blank() || context.value.is_error() {
        // See `RuleValue::compare`: neither takes a position in Excel's order, so no comparison
        // against either holds.
        return Decision::DidNotFire;
    }
    let first = rule.formulas.first().map(String::as_str);
    let second = rule.formulas.get(1).map(String::as_str);
    match operator {
        ConditionalFormattingOperator::Between | ConditionalFormattingOperator::NotBetween => {
            let (Some(low), Some(high)) = (first.and_then(literal), second.and_then(literal))
            else {
                return unevaluated_operand(first, second);
            };
            let (Some(above), Some(below)) =
                (context.value.compare(&low), context.value.compare(&high))
            else {
                return Decision::DidNotFire;
            };
            // Inclusive at both ends: §18.3.1.10 says `between` is *"between the two values"* and
            // Excel's own dialogue calls it *"between … and …"*, which includes the bounds.
            let inside = above.is_ge() && below.is_le();
            fired(inside == matches!(operator, ConditionalFormattingOperator::Between))
        }
        ConditionalFormattingOperator::ContainsText
        | ConditionalFormattingOperator::NotContains
        | ConditionalFormattingOperator::BeginsWith
        | ConditionalFormattingOperator::EndsWith => {
            // §18.3.1.10 lists these under `ST_ConditionalFormattingOperator`, so a `cellIs` may
            // spell a text test the four dedicated `ST_CfType` members also spell. The operand is
            // the formula rather than `@text` here.
            let Some(RuleValue::Text(needle)) = first.and_then(literal) else {
                return unevaluated_operand(first, None);
            };
            let Some(haystack) = context.value.as_text() else {
                return Decision::DidNotFire;
            };
            let (haystack, needle) = (haystack.to_lowercase(), needle.to_lowercase());
            let held = match operator {
                ConditionalFormattingOperator::ContainsText => haystack.contains(&needle),
                ConditionalFormattingOperator::NotContains => !haystack.contains(&needle),
                ConditionalFormattingOperator::BeginsWith => haystack.starts_with(&needle),
                _ => haystack.ends_with(&needle),
            };
            fired(held)
        }
        _ => {
            let Some(operand) = first.and_then(literal) else {
                return unevaluated_operand(first, None);
            };
            let Some(ordering) = context.value.compare(&operand) else {
                return Decision::DidNotFire;
            };
            fired(match operator {
                ConditionalFormattingOperator::LessThan => ordering.is_lt(),
                ConditionalFormattingOperator::LessThanOrEqual => ordering.is_le(),
                ConditionalFormattingOperator::Equal => ordering.is_eq(),
                ConditionalFormattingOperator::NotEqual => !ordering.is_eq(),
                ConditionalFormattingOperator::GreaterThanOrEqual => ordering.is_ge(),
                ConditionalFormattingOperator::GreaterThan => ordering.is_gt(),
                _ => false,
            })
        }
    }
}

/// The reason a `cellIs` could not be answered: a missing operand is incomplete markup, a present
/// one that is not a literal is the absent calculation engine.
fn unevaluated_operand(first: Option<&str>, second: Option<&str>) -> Decision {
    if first.is_none() {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    }
    if second.is_some_and(str::is_empty) {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    }
    Decision::Unevaluated(UnevaluatedReason::OperandIsNotALiteral)
}

/// `expression` — always a formula, and answerable only when that formula is a literal truth value.
///
/// `<formula>TRUE</formula>` is not a hypothetical: it is what a rule reduces to when a person
/// disables a condition without deleting it, and answering it costs nothing. Everything else is
/// reported unevaluated.
fn expression(rule: &CompiledRule) -> Decision {
    match rule.formulas.first().map(|text| text.trim()) {
        Some(text) if text.eq_ignore_ascii_case("TRUE") => Decision::Fired,
        Some(text) if text.eq_ignore_ascii_case("FALSE") => Decision::DidNotFire,
        Some(_) => Decision::Unevaluated(UnevaluatedReason::NoFormulaEngine),
        None => Decision::Unevaluated(UnevaluatedReason::Incomplete),
    }
}

/// `top10` — the top or bottom `@rank` values, or `@rank` per cent of them.
fn top_or_bottom(rule: &CompiledRule, context: &Context<'_>) -> Decision {
    let Some(rank) = rule.rank else {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    };
    let Some(value) = context.value.as_number() else {
        return Decision::DidNotFire;
    };
    let count = if rule.ranks_by_percent {
        context.statistics.rank_count_for_percent(rank)
    } else {
        usize::try_from(rank).unwrap_or(usize::MAX)
    };
    let Some(threshold) = context
        .statistics
        .rank_threshold(count, rule.ranks_from_bottom)
    else {
        return Decision::DidNotFire;
    };
    fired(if rule.ranks_from_bottom {
        value <= threshold
    } else {
        value >= threshold
    })
}

/// `aboveAverage` — with or without a standard-deviation offset, and with or without the average
/// itself.
fn above_average(rule: &CompiledRule, context: &Context<'_>) -> Decision {
    let Some(value) = context.value.as_number() else {
        return Decision::DidNotFire;
    };
    let Some(mean) = context.statistics.mean() else {
        return Decision::DidNotFire;
    };
    let offset = match rule.standard_deviations {
        None | Some(0) => 0.0,
        Some(count) => {
            let Some(deviation) = context.statistics.standard_deviation() else {
                return Decision::DidNotFire;
            };
            f64::from(count) * deviation
        }
    };
    // GUESS: that `@stdDev` moves the threshold *away* from the mean in the direction the rule
    // points — up for above, down for below — and that a negative `@stdDev` therefore moves it
    // back across. §18.3.1.10 calls it *"the number of standard deviations to include above or
    // below the average"* and states no sign convention.
    let threshold = if rule.is_above_average {
        mean + offset
    } else {
        mean - offset
    };
    fired(match (rule.is_above_average, rule.includes_the_average) {
        (true, false) => value > threshold,
        (true, true) => value >= threshold,
        (false, false) => value < threshold,
        (false, true) => value <= threshold,
    })
}

/// `duplicateValues` and `uniqueValues`, which are the same count read two ways.
fn occurrences(context: &Context<'_>, holds: impl Fn(u32) -> bool) -> Decision {
    if context.value.duplicate_key().is_none() {
        // Blanks and errors take part in neither: a range of forty empty cells is not forty
        // duplicates, and it is not forty unique values either.
        return Decision::DidNotFire;
    }
    fired(holds(context.statistics.occurrences(context.value)))
}

/// Which of the three shapes a text test takes.
#[derive(Clone, Copy)]
enum TextTest {
    Contains,
    Begins,
    Ends,
}

/// `containsText`, `notContainsText`, `beginsWith` and `endsWith`, whose operand is `@text`.
fn text_test(
    rule: &CompiledRule,
    context: &Context<'_>,
    test: TextTest,
    negated: bool,
) -> Decision {
    let Some(needle) = rule.text.as_deref() else {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    };
    let Some(haystack) = context.value.as_text() else {
        // A number is searched as no characters — see `RuleValue::as_text` for the `GUESS:` that
        // carries. `notContainsText` over a numeric cell therefore fires, which is the same answer
        // Excel gives whenever the number's own spelling does not contain the needle.
        return fired(negated);
    };
    let (haystack, needle) = (haystack.to_lowercase(), needle.to_lowercase());
    let held = match test {
        TextTest::Contains => haystack.contains(&needle),
        TextTest::Begins => haystack.starts_with(&needle),
        TextTest::Ends => haystack.ends_with(&needle),
    };
    fired(held != negated)
}

/// `timePeriod` — ten windows around today, in the workbook's own epoch.
fn time_period(rule: &CompiledRule, context: &Context<'_>) -> Decision {
    let Some(period) = rule.time_period else {
        return Decision::Unevaluated(UnevaluatedReason::Incomplete);
    };
    let Some(serial) = context.value.as_number() else {
        return Decision::DidNotFire;
    };
    // `FLOOR(A1,1)` in Excel's own formula: a timestamp is compared by its day.
    let day = serial.floor();
    let today = context.today.floor();
    let held = match period {
        TimePeriod::Today => day == today,
        TimePeriod::Yesterday => day == today - 1.0,
        TimePeriod::Tomorrow => day == today + 1.0,
        // DocumentedBehaviour: `AND(TODAY()-FLOOR(A1,1)<=6,FLOOR(A1,1)<=TODAY())`, which Excel
        // writes into the file next to the rule. Seven days ending today, tomorrow excluded.
        TimePeriod::Last7Days => (today - day) <= 6.0 && day <= today,
        TimePeriod::ThisMonth => same_month(day, today, 0, context.dates),
        TimePeriod::LastMonth => same_month(day, today, -1, context.dates),
        TimePeriod::NextMonth => same_month(day, today, 1, context.dates),
        TimePeriod::ThisWeek => week_start(day, context.dates) == week_start(today, context.dates),
        TimePeriod::LastWeek => week_start(day, context.dates)
            .zip(week_start(today, context.dates))
            .is_some_and(|(value, now)| value + 7.0 == now),
        TimePeriod::NextWeek => week_start(day, context.dates)
            .zip(week_start(today, context.dates))
            .is_some_and(|(value, now)| value == now + 7.0),
    };
    fired(held)
}

/// Whether `day` falls in the month `offset` months from the one `today` is in.
fn same_month(day: f64, today: f64, offset: i32, dates: DateSystem) -> bool {
    let (Some(value), Some(now)) = (
        datetime::civil_from_serial(day, dates, 0),
        datetime::civil_from_serial(today, dates, 0),
    ) else {
        return false;
    };
    let months = i64::from(now.year) * 12 + i64::from(now.month) + i64::from(offset);
    let target_year = months.div_euclid(12);
    let target_month = months.rem_euclid(12);
    // `rem_euclid` puts December at 0, so December belongs to the previous year's count.
    let (target_year, target_month) = if target_month == 0 {
        (target_year - 1, 12)
    } else {
        (target_year, target_month)
    };
    i64::from(value.year) == target_year && i64::from(value.month) == target_month
}

/// The serial of the Sunday that starts `day`'s week.
///
/// DocumentedBehaviour, from the formula Excel writes for `thisWeek`:
/// `ROUNDDOWN(A1,0)-WEEKDAY(A1)+1 = TODAY()-WEEKDAY(TODAY())+1`. `WEEKDAY`'s default numbering puts
/// Sunday at 1, so the week starts on a Sunday — which is not a choice this crate made and not one
/// it can localise, because nothing in the file says otherwise.
fn week_start(day: f64, dates: DateSystem) -> Option<f64> {
    let at = datetime::civil_from_serial(day, dates, 0)?;
    Some(day - f64::from(at.weekday))
}

/// Reads a formula that is a literal, or `None` for one that needs an engine.
///
/// Three shapes are literals and nothing else is: a quoted string (with `""` for an embedded
/// quote), a number, and `TRUE`/`FALSE`. A leading `=` is tolerated because a person writing a rule
/// by hand types one, even though Excel does not store it.
#[must_use]
pub fn literal(formula: &str) -> Option<RuleValue> {
    let text = formula.trim().trim_start_matches('=').trim();
    if text.is_empty() {
        return None;
    }
    if let Some(inner) = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        if inner.contains('"') && !inner.contains("\"\"") {
            // An unbalanced quote in the middle is not a string literal; it is markup this cannot
            // read, and guessing at it would be worse than reporting it.
            return None;
        }
        return Some(RuleValue::Text(inner.replace("\"\"", "\"")));
    }
    if let Ok(number) = text.parse::<f64>() {
        if number.is_finite() {
            return Some(RuleValue::Number(number));
        }
    }
    if text.eq_ignore_ascii_case("TRUE") {
        return Some(RuleValue::Boolean(true));
    }
    if text.eq_ignore_ascii_case("FALSE") {
        return Some(RuleValue::Boolean(false));
    }
    None
}
