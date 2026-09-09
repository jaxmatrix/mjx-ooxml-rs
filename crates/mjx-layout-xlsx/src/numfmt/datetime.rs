//! Date serials, both epochs, and the 1900 leap-year bug — reproduced rather than corrected.
//!
//! # Two epochs
//!
//! `workbookPr@date1904` chooses between them and the difference is 1,462 days, so reading a
//! workbook under the wrong one shifts every date in it by just over four years.
//! [`mjx_xlsx::DateSystem`] is the two-valued type that carries the answer, and nothing here guesses
//! it.
//!
//! # ⚠ The 1900 leap-year bug is a requirement
//!
//! Excel's 1900 system contains a day that never existed: serial 60 is **1900-02-29**, and 1900 was
//! not a leap year. Lotus 1-2-3 had the defect, Excel copied it for file compatibility and has
//! carried it ever since, and a renderer that prints the arithmetically right answer for serial 60
//! prints something no Excel user has ever seen.
//!
//! So the epoch is *not* one constant:
//!
//! | serial | date |
//! |---|---|
//! | 0 | 1900-01-00 — Excel's own non-date, printed with a zero day |
//! | 1 ..= 59 | 1899-12-31 + serial |
//! | 60 | **1900-02-29** |
//! | 61 .. | 1899-12-30 + serial |
//!
//! `tests/the_excel_quirks_are_reproduced.rs` asserts serial 60 and the day either side of it, and
//! a "fix" fails it deliberately.
//!
//! The weekday is *not* derived from that table. It is a function of the serial alone —
//! `(serial + 6) mod 7` is Sunday-based for the 1900 system — which is both simpler and what Excel
//! answers: `TEXT(1,"dddd")` is `Sunday` even though 1900-01-01 was really a Monday.
//!
//! # What is deliberately not here
//!
//! Month and weekday names are **English only**. The engine takes no locale database, because one
//! would have to be linked into every target of the cross-build matrix to answer a question a
//! `[$-409]` prefix does not actually ask: ECMA-376 gives no localised name table and Excel's comes
//! from the operating system. A `[$-40C]` code therefore renders English names, and that is a
//! reported limitation rather than a silent one.

use mjx_xlsx::DateSystem;

/// How many days apart the two epochs are — 1904-01-01 minus 1900-01-01, plus the phantom day.
///
/// Stated for the documentation rather than used: both epochs are computed from
/// [`days_from_civil`], so there is no constant here to get wrong.
pub const EPOCH_DIFFERENCE_DAYS: i64 = 1462;

/// The largest serial this engine will turn into a date.
///
/// Excel's own ceiling is 2958465 — 9999-12-31 — and a serial past it has no date to show. Beyond
/// this the value renders as a number instead, which is what Excel does with `=TEXT(1e9,"yyyy")`
/// too: it refuses rather than inventing a year.
pub const MAX_DATE_SERIAL: f64 = 2_958_465.0;

/// One date or time token of a format code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateToken {
    /// `yy` — the last two digits of the year.
    Year2,
    /// `yyyy` — the whole year.
    Year4,
    /// `m` — the month, unpadded.
    MonthNumber,
    /// `mm` — the month, two digits.
    MonthNumberPadded,
    /// `mmm` — `Jan`.
    MonthAbbreviation,
    /// `mmmm` — `January`.
    MonthName,
    /// `mmmmm` — `J`.
    MonthLetter,
    /// `d` — the day of the month, unpadded.
    Day,
    /// `dd` — the day of the month, two digits.
    DayPadded,
    /// `ddd` — `Mon`.
    WeekdayAbbreviation,
    /// `dddd` — `Monday`.
    WeekdayName,
    /// `h` — the hour, unpadded, on twelve hours when the section carries an `AM/PM`.
    Hour,
    /// `hh` — the hour, two digits.
    HourPadded,
    /// `m` resolved as a minute — see [`super::parse`]'s disambiguation.
    Minute,
    /// `mm` resolved as a minute.
    MinutePadded,
    /// `s` — the second, unpadded.
    Second,
    /// `ss` — the second, two digits.
    SecondPadded,
    /// `.0`, `.00`, `.000` — the fraction of a second, to that many digits.
    SubSecond(u8),
    /// `AM/PM`, `A/P`, and their lower-case spellings.
    Meridiem(MeridiemStyle),
    /// `[h]`, `[hh]` — hours elapsed, not wrapped at 24, padded to that width.
    ElapsedHours(u8),
    /// `[m]`, `[mm]` — minutes elapsed.
    ElapsedMinutes(u8),
    /// `[s]`, `[ss]` — seconds elapsed.
    ElapsedSeconds(u8),
}

/// How an `AM`/`PM` marker is spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeridiemStyle {
    /// `AM/PM`.
    UpperLong,
    /// `am/pm`.
    LowerLong,
    /// `A/P`.
    UpperShort,
    /// `a/p`.
    LowerShort,
}

impl MeridiemStyle {
    /// What this style writes for a time before noon or from noon.
    #[must_use]
    pub fn marker(self, afternoon: bool) -> &'static str {
        match (self, afternoon) {
            (Self::UpperLong, false) => "AM",
            (Self::UpperLong, true) => "PM",
            (Self::LowerLong, false) => "am",
            (Self::LowerLong, true) => "pm",
            (Self::UpperShort, false) => "A",
            (Self::UpperShort, true) => "P",
            (Self::LowerShort, false) => "a",
            (Self::LowerShort, true) => "p",
        }
    }
}

/// A serial number, split into the fields a format code's tokens read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CivilDateTime {
    /// The year.
    pub year: i32,
    /// The month, `1..=12`.
    pub month: u32,
    /// The day of the month, `1..=31` — or **`0`**, for the 1900 system's serial zero.
    pub day: u32,
    /// The day of the week, `0` for Sunday.
    pub weekday: u32,
    /// The hour, `0..=23`.
    pub hour: u32,
    /// The minute, `0..=59`.
    pub minute: u32,
    /// The second, `0..=59`.
    pub second: u32,
    /// The fraction of a second, `0.0..1.0`.
    pub fraction: f64,
    /// The whole serial, kept so an elapsed-time token can read it.
    pub serial: f64,
}

impl CivilDateTime {
    /// The hour on a twelve-hour clock, `1..=12`.
    #[must_use]
    pub fn hour12(self) -> u32 {
        match self.hour % 12 {
            0 => 12,
            hour => hour,
        }
    }

    /// Whether the time is at or past noon.
    #[must_use]
    pub fn is_afternoon(self) -> bool {
        self.hour >= 12
    }
}

/// Turns `serial` into civil fields under `system`, rounding the time to `subsecond_digits`.
///
/// The rounding is done here rather than in the renderer because a time of `23:59:59.6` shown to
/// whole seconds is `00:00:00` **of the next day**, and a renderer that rounded its own seconds
/// field would print `24:00:00` or the wrong date. Carrying the rounding into the day is the only
/// place that answer can be got right.
///
/// `None` for a negative serial — which has no date in either system, and which Excel shows as a row
/// of `#` rather than as a date — and for one past [`MAX_DATE_SERIAL`].
#[must_use]
pub fn civil_from_serial(
    serial: f64,
    system: DateSystem,
    subsecond_digits: u8,
) -> Option<CivilDateTime> {
    if !serial.is_finite() || serial < 0.0 || serial > MAX_DATE_SERIAL {
        return None;
    }
    let (days, hour, minute, second, fraction) = split_serial(serial, subsecond_digits)?;
    let (year, month, day) = civil_fields(days, system)?;
    let weekday = weekday_of(days, system);
    Some(CivilDateTime {
        year,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
        fraction,
        serial,
    })
}

/// Splits a serial into whole days and a clock, rounding to `subsecond_digits` and carrying.
fn split_serial(serial: f64, subsecond_digits: u8) -> Option<(i64, u32, u32, u32, f64)> {
    let whole = serial.floor();
    let mut days = whole as i64;
    let scale = 10_f64.powi(i32::from(subsecond_digits));
    // Rounding in seconds rather than in days keeps the precision where the digits are: a day is
    // 86,400 seconds, so a `f64` serial has far more resolution in seconds than the format code can
    // ever ask for.
    let seconds_in_day = 86_400.0;
    // The fifteen-digit clamp again, and for the same reason as everywhere else in this engine:
    // `0.5 + 0.125/86400` times 86,400 is `43200.124999999996` in binary, and rounding *that* to two
    // sub-second digits writes `.12` where the decimal `43200.125` writes `.13`.
    let exact = super::general::to_display_precision((serial - whole) * seconds_in_day);
    let mut seconds = (exact * scale).round() / scale;
    if seconds >= seconds_in_day {
        seconds -= seconds_in_day;
        days = days.checked_add(1)?;
    }
    let hour = (seconds / 3600.0).floor();
    let minute = ((seconds - hour * 3600.0) / 60.0).floor();
    let second = (seconds - hour * 3600.0 - minute * 60.0).floor();
    let fraction = seconds - hour * 3600.0 - minute * 60.0 - second;
    Some((
        days,
        hour as u32,
        minute as u32,
        second as u32,
        fraction.clamp(0.0, 1.0),
    ))
}

/// The civil date `days` names under `system` — **including the day that never existed**.
fn civil_fields(days: i64, system: DateSystem) -> Option<(i32, u32, u32)> {
    if system.is_1904() {
        // No leap-year defect here: the 1904 system counts from a real day and 1904 really was a
        // leap year, so one epoch answers for every serial.
        return Some(civil_from_days(days.checked_add(epoch_1904())?));
    }
    match days {
        // Excel's own "January 0, 1900", which `=TEXT(0,"m/d/yyyy")` prints as `1/0/1900`. It is not
        // a date and it is not an error; it is the zero of the serial line and it has a spelling.
        0 => Some((1900, 1, 0)),
        // Before the phantom day the epoch is 1899-12-31: serial 1 is 1900-01-01.
        1..=59 => Some(civil_from_days(
            days.checked_add(epoch_1900_before_the_bug())?,
        )),
        // ⚠ The phantom day. 1900 was not a leap year — it is divisible by 100 and not by 400 — and
        // Excel prints 1900-02-29 for this serial anyway. See the module documentation.
        60 => Some((1900, 2, 29)),
        // After it, every serial is one greater than the real elapsed day count, so the epoch moves
        // back a day to 1899-12-30 and stays there.
        _ => Some(civil_from_days(
            days.checked_add(epoch_1900_after_the_bug())?,
        )),
    }
}

/// Days from 1970-01-01 to 1899-12-31 — the epoch for a 1900-system serial below the phantom day.
fn epoch_1900_before_the_bug() -> i64 {
    days_from_civil(1899, 12, 31)
}

/// Days from 1970-01-01 to 1899-12-30 — the epoch for a 1900-system serial above the phantom day.
fn epoch_1900_after_the_bug() -> i64 {
    days_from_civil(1899, 12, 30)
}

/// Days from 1970-01-01 to 1904-01-01 — the 1904 system's epoch.
fn epoch_1904() -> i64 {
    days_from_civil(1904, 1, 1)
}

/// The day of the week, `0` for Sunday.
///
/// Taken from the serial rather than from the civil date, which is both simpler and what Excel
/// answers. In the 1900 system serial 1 is a Sunday — Excel's own answer, and one day off the real
/// 1900-01-01, which was a Monday; the phantom day is what puts the two back in step from serial 61
/// onward. In the 1904 system serial 0 is 1904-01-01, a Friday.
fn weekday_of(days: i64, system: DateSystem) -> u32 {
    let offset = if system.is_1904() { 5 } else { 6 };
    u32::try_from((days + offset).rem_euclid(7)).unwrap_or(0)
}

/// Howard Hinnant's `days_from_civil`: days from 1970-01-01 to `year-month-day`, proleptic Gregorian.
///
/// Written out rather than taken from a dependency because a date library is a dependency the
/// cross-build matrix would have to carry for two functions, and because this one is exact for every
/// year a spreadsheet can hold.
#[must_use]
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Howard Hinnant's `civil_from_days`, the inverse of [`days_from_civil`].
#[must_use]
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    (
        i32::try_from(year + i64::from(month <= 2)).unwrap_or(0),
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

/// `Jan` … `Dec`, indexed from one.
const MONTH_ABBREVIATIONS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `January` … `December`, indexed from one.
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `Sun` … `Sat`, indexed from Sunday.
const WEEKDAY_ABBREVIATIONS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// `Sunday` … `Saturday`, indexed from Sunday.
const WEEKDAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Writes one date token into `out`.
///
/// `has_meridiem` is the section's, not the token's: `h` is a twelve-hour clock only when an
/// `AM/PM` stands somewhere in the same section.
pub fn render_token(out: &mut String, token: DateToken, at: CivilDateTime, has_meridiem: bool) {
    match token {
        DateToken::Year2 => {
            let year = at.year.rem_euclid(100);
            out.push_str(&format!("{year:02}"));
        }
        DateToken::Year4 => out.push_str(&at.year.to_string()),
        DateToken::MonthNumber => out.push_str(&at.month.to_string()),
        DateToken::MonthNumberPadded => out.push_str(&format!("{:02}", at.month)),
        DateToken::MonthAbbreviation => {
            out.push_str(month_of(&MONTH_ABBREVIATIONS, at.month));
        }
        DateToken::MonthName => out.push_str(month_of(&MONTH_NAMES, at.month)),
        DateToken::MonthLetter => {
            let name = month_of(&MONTH_NAMES, at.month);
            out.extend(name.chars().take(1));
        }
        DateToken::Day => out.push_str(&at.day.to_string()),
        DateToken::DayPadded => out.push_str(&format!("{:02}", at.day)),
        DateToken::WeekdayAbbreviation => {
            out.push_str(weekday_of_table(&WEEKDAY_ABBREVIATIONS, at.weekday));
        }
        DateToken::WeekdayName => out.push_str(weekday_of_table(&WEEKDAY_NAMES, at.weekday)),
        DateToken::Hour => {
            let hour = if has_meridiem { at.hour12() } else { at.hour };
            out.push_str(&hour.to_string());
        }
        DateToken::HourPadded => {
            let hour = if has_meridiem { at.hour12() } else { at.hour };
            out.push_str(&format!("{hour:02}"));
        }
        DateToken::Minute => out.push_str(&at.minute.to_string()),
        DateToken::MinutePadded => out.push_str(&format!("{:02}", at.minute)),
        DateToken::Second => out.push_str(&at.second.to_string()),
        DateToken::SecondPadded => out.push_str(&format!("{:02}", at.second)),
        DateToken::SubSecond(digits) => {
            let digits = usize::from(digits);
            let scaled = at.fraction * 10_f64.powi(i32::try_from(digits).unwrap_or(0));
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let value = scaled.round().max(0.0) as u64;
            out.push('.');
            out.push_str(&format!("{value:0digits$}"));
        }
        DateToken::Meridiem(style) => out.push_str(style.marker(at.is_afternoon())),
        DateToken::ElapsedHours(width) => {
            elapsed(out, at.serial * 24.0, width);
        }
        DateToken::ElapsedMinutes(width) => {
            elapsed(out, at.serial * 24.0 * 60.0, width);
        }
        DateToken::ElapsedSeconds(width) => {
            elapsed(out, at.serial * 24.0 * 60.0 * 60.0, width);
        }
    }
}

/// The month's name, guarding an out-of-range month rather than indexing past the table.
fn month_of(table: &[&'static str; 12], month: u32) -> &'static str {
    let index = usize::try_from(month.saturating_sub(1)).unwrap_or(0);
    table.get(index).copied().unwrap_or("")
}

/// The weekday's name, guarding an out-of-range weekday.
fn weekday_of_table(table: &[&'static str; 7], weekday: u32) -> &'static str {
    let index = usize::try_from(weekday).unwrap_or(0);
    table.get(index).copied().unwrap_or("")
}

/// Writes an elapsed count, floored and padded to `width`.
///
/// `[h]:mm` on `1.5` is `36:00`: the hours are not wrapped at twenty-four, which is the whole reason
/// the bracketed form exists.
fn elapsed(out: &mut String, total: f64, width: u8) {
    let width = usize::from(width.max(1));
    #[allow(clippy::cast_possible_truncation)]
    let whole = total.floor().max(0.0) as i64;
    out.push_str(&format!("{whole:0width$}"));
}
