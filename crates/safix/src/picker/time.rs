//! The two stamp columns, as a person reads a date.
//!
//! A stamp is unix seconds, UTC, because that is what a clock reads without a
//! timezone database and without a rendering decision. Turning one into
//! `dd/mm/yyyy hh:mmam` is that rendering decision, and it lives here rather
//! than in the record.
//!
//! # Why the offset is asked of `date` and not computed
//!
//! Local time is a function of the zone database, and reading that database is
//! what a date library is for. This needs one number — the offset the machine
//! is at right now — and `date +%z` is where every unix states it, so the
//! offset is read once per run from there and applied as seconds. The cost is
//! stated rather than hidden: a picker left open across a daylight-saving
//! transition keeps the offset it started with, and a stamp from the other side
//! of a transition is rendered at today's offset rather than at the one in
//! force when it was written. Both are wrong by an hour on a column whose
//! purpose is "roughly when"; a zone-database dependency to fix them is not a
//! trade this repository makes for that.
//!
//! No `date` on the machine, or an answer this cannot parse, is UTC. A picker
//! that refused to draw because it could not establish a timezone would be a
//! picker that refused to draw.

use std::process::Command;
use std::sync::LazyLock;

/// Seconds in one day.
const DAY: i64 = 86_400;

/// Seconds in one hour.
const HOUR: i64 = 3_600;

/// Seconds in one minute.
const MINUTE: i64 = 60;

/// What a cell with no stamp behind it shows.
///
/// A missing record is not a zero and not an epoch date; see
/// [`safix_core::stamps`] for why no date is invented for one.
pub(crate) const ABSENT: &str = "-";

/// The offset from UTC this run renders at, in seconds east.
///
/// Read once. Every row of every frame renders through it, and forking `date`
/// per cell per keystroke would be a subprocess per cell per keystroke.
static OFFSET: LazyLock<i64> = LazyLock::new(|| probe().unwrap_or(0));

/// Ask the platform what it is offset by, as `date` states it.
fn probe() -> Option<i64> {
    let asked = Command::new("date").arg("+%z").output().ok()?;
    if !asked.status.success() {
        return None;
    }
    parse_offset(std::str::from_utf8(&asked.stdout).ok()?)
}

/// The seconds east of UTC a `+HHMM` or `-HHMM` field names.
///
/// Anything else is nothing: a `date` that answered something other than the
/// one field asked for has not told this what the offset is, and guessing from
/// a prefix of it would be worse than UTC.
fn parse_offset(text: &str) -> Option<i64> {
    let field = text.trim();
    let (sign, digits) = match field.split_at_checked(1)? {
        ("+", digits) => (1_i64, digits),
        ("-", digits) => (-1_i64, digits),
        _ => return None,
    };
    if digits.len() != 4 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let (hours, minutes) = digits.split_at_checked(2)?;
    let hours: i64 = hours.parse().ok()?;
    let minutes: i64 = minutes.parse().ok()?;
    Some(
        sign.saturating_mul(
            hours
                .saturating_mul(HOUR)
                .saturating_add(minutes.saturating_mul(MINUTE)),
        ),
    )
}

/// One stamp as a person reads it, or [`ABSENT`] when there is no stamp.
pub(crate) fn render(stamp: Option<u64>) -> String {
    stamp.map_or_else(|| ABSENT.to_owned(), |seconds| local(seconds, *OFFSET))
}

/// One stamp at one offset, `dd/mm/yyyy hh:mmam`.
///
/// Separated from [`render`] by the offset so the arithmetic is testable
/// without a timezone: the machine's own answer is [`offset`]'s business.
fn local(seconds: u64, offset: i64) -> String {
    let at = i64::try_from(seconds)
        .unwrap_or(i64::MAX)
        .saturating_add(offset);
    let days = div(at, DAY);
    // Euclidean rather than truncating: a stamp before the epoch at a negative
    // offset is still a time of day rather than a negative number of seconds.
    let rest = at.saturating_sub(days.saturating_mul(DAY));
    let (year, month, day) = civil_from_days(days);
    let hour = div(rest, HOUR);
    let minute = div(rem(rest, HOUR), MINUTE);
    let (clock, suffix) = twelve_hour(hour);
    format!("{day:02}/{month:02}/{year:04} {clock:02}:{minute:02}{suffix}")
}

/// One hour of the day on a twelve-hour clock, and which half it is in.
fn twelve_hour(hour: i64) -> (i64, &'static str) {
    let suffix = if hour < 12 { "am" } else { "pm" };
    let clock = match rem(hour, 12) {
        0 => 12,
        other => other,
    };
    (clock, suffix)
}

/// The proleptic Gregorian year, month and day a count of days since the epoch
/// falls on.
///
/// Howard Hinnant's `civil_from_days`, whose derivation is in "chrono-Compatible
/// Low-Level Date Algorithms" and whose correctness rests on shifting the era
/// to start on 1st March: a leap day is then the last day of a year rather than
/// a day in the middle of one, which is what removes the special cases. The
/// shift is the `+ 719_468` — the days from 1st January 1970 back to 1st March
/// 0000 — and the `+ 2`/`- 9` on the month is the shift back.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days.saturating_add(719_468);
    let era = div(
        if shifted >= 0 {
            shifted
        } else {
            shifted.saturating_sub(146_096)
        },
        146_097,
    );
    let day_of_era = shifted.saturating_sub(era.saturating_mul(146_097));
    let year_of_era = div(
        day_of_era
            .saturating_sub(div(day_of_era, 1_460))
            .saturating_add(div(day_of_era, 36_524))
            .saturating_sub(div(day_of_era, 146_096)),
        365,
    );
    let year = year_of_era.saturating_add(era.saturating_mul(400));
    let day_of_year = day_of_era.saturating_sub(
        year_of_era
            .saturating_mul(365)
            .saturating_add(div(year_of_era, 4))
            .saturating_sub(div(year_of_era, 100)),
    );
    let shifted_month = div(day_of_year.saturating_mul(5).saturating_add(2), 153);
    let day = day_of_year
        .saturating_sub(div(shifted_month.saturating_mul(153).saturating_add(2), 5))
        .saturating_add(1);
    let month = shifted_month.saturating_add(if shifted_month < 10 { 3 } else { -9 });
    (
        if month <= 2 {
            year.saturating_add(1)
        } else {
            year
        },
        month,
        day,
    )
}

/// Integer division that cannot end the process.
///
/// The workspace denies `arithmetic_side_effects`, and a division is the one
/// arithmetic operation here whose operands are both derived: every divisor
/// below is a non-zero literal, so the fallback is unreachable and is written
/// as a value rather than as a panic.
fn div(value: i64, by: i64) -> i64 {
    value.checked_div_euclid(by).unwrap_or(0)
}

/// The remainder of [`div`], on the same terms.
fn rem(value: i64, by: i64) -> i64 {
    value.checked_rem_euclid(by).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{ABSENT, local, parse_offset, render};

    #[test]
    fn the_epoch_renders_as_the_first_of_january_nineteen_seventy() {
        assert_eq!(local(0, 0), "01/01/1970 12:00am");
    }

    #[test]
    fn noon_is_pm_and_midnight_is_am() {
        assert_eq!(local(12 * 3600, 0), "01/01/1970 12:00pm");
        assert_eq!(local(13 * 3600 + 5 * 60, 0), "01/01/1970 01:05pm");
        assert_eq!(local(11 * 3600 + 59 * 60, 0), "01/01/1970 11:59am");
    }

    /// A leap day, which is the day the era shift exists to make ordinary.
    #[test]
    fn a_leap_day_is_the_twenty_ninth_of_february() {
        // 2024-02-29T06:07:00Z.
        assert_eq!(local(1_709_186_820, 0), "29/02/2024 06:07am");
    }

    /// The offset moves the calendar day, not only the clock.
    #[test]
    fn an_offset_can_carry_a_stamp_into_the_next_day() {
        // 2024-02-29T23:30:00Z, read on a machine two hours east.
        assert_eq!(local(1_709_249_400, 0), "29/02/2024 11:30pm");
        assert_eq!(local(1_709_249_400, 2 * 3600), "01/03/2024 01:30am");
        assert_eq!(local(1_709_249_400, -3600), "29/02/2024 10:30pm");
    }

    #[test]
    fn a_stamp_before_the_epoch_is_still_a_time_of_day() {
        // 1969-12-31T23:00:00Z is one hour before the epoch.
        assert_eq!(local(0, -3600), "31/12/1969 11:00pm");
    }

    #[test]
    fn an_offset_field_reads_in_both_directions() {
        assert_eq!(parse_offset("+0000\n"), Some(0));
        assert_eq!(parse_offset("+0530"), Some(5 * 3600 + 30 * 60));
        assert_eq!(parse_offset("-0800\n"), Some(-8 * 3600));
    }

    /// Anything that is not the one field asked for is UTC rather than a guess.
    #[test]
    fn an_unparsable_offset_is_nothing() {
        for answered in ["", "+", "0530", "+53", "+05300", "+05:30", "abcde", "+05a0"] {
            assert_eq!(parse_offset(answered), None, "{answered:?} was parsed");
        }
    }

    #[test]
    fn no_stamp_renders_as_the_absent_cell() {
        assert_eq!(render(None), ABSENT);
    }
}
