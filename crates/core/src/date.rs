//! Civil dates (proleptic Gregorian) for the journal: day arithmetic, weekdays,
//! ISO weeks and month grids, with no time-zone database.
//!
//! The days-to-civil conversion is Howard Hinnant's algorithm; it is also what
//! [`crate::log::timestamp`] uses.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unix seconds of 2020-01-01 00:00:00 UTC. Earlier clocks are taken to be unset.
const CLOCK_SET: u64 = 1_577_836_800;

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year (astronomical numbering).
    pub y: i32,
    /// Month, 1 to 12.
    pub m: u8,
    /// Day of the month, 1 to 31.
    pub d: u8,
}

/// Which day a calendar week starts on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WeekStart {
    /// Weeks start on Monday (ISO).
    #[default]
    Monday,
    /// Weeks start on Sunday.
    Sunday,
}

/// Whether `y` is a leap year.
#[must_use]
pub fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Number of days in month `m` (1 to 12) of year `y`; 0 for an invalid month.
#[must_use]
pub fn days_in_month(y: i32, m: u8) -> u8 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Days since 1970-01-01 to `(year, month, day)` (Hinnant's `civil_from_days`).
#[must_use]
pub fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y as i32, m as u8, d as u8)
}

/// `(year, month, day)` to days since 1970-01-01 (Hinnant's `days_from_civil`).
#[must_use]
pub fn days_from_civil(y: i32, m: u8, d: u8) -> i64 {
    let y = i64::from(y) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

impl Date {
    /// A validated date, or `None` if the month or day does not exist.
    #[must_use]
    pub fn new(y: i32, m: u8, d: u8) -> Option<Date> {
        (d >= 1 && d <= days_in_month(y, m)).then_some(Date { y, m, d })
    }

    /// The date `days` days after 1970-01-01 (negative for before).
    #[must_use]
    pub fn from_days(days: i64) -> Date {
        let (y, m, d) = civil_from_days(days);
        Date { y, m, d }
    }

    /// Days since 1970-01-01.
    #[must_use]
    pub fn to_days(self) -> i64 {
        days_from_civil(self.y, self.m, self.d)
    }

    /// Day of the week, 0 = Monday to 6 = Sunday.
    #[must_use]
    pub fn weekday(self) -> u8 {
        // 1970-01-01 was a Thursday (3).
        (self.to_days() + 3).rem_euclid(7) as u8
    }

    /// ISO 8601 `(year, week)`; weeks start on Monday and week 1 holds the
    /// year's first Thursday.
    #[must_use]
    pub fn iso_week(self) -> (i32, u8) {
        // The Thursday of this date's week decides the ISO year.
        let thursday = self.add_days(3 - i64::from(self.weekday()));
        let jan1 = Date {
            y: thursday.y,
            m: 1,
            d: 1,
        };
        let week = (thursday.to_days() - jan1.to_days()) / 7 + 1;
        (thursday.y, week as u8)
    }

    /// This date shifted by `n` days (negative for earlier).
    #[must_use]
    pub fn add_days(self, n: i64) -> Date {
        Date::from_days(self.to_days() + n)
    }

    /// Strict `yyyy-mm-dd`: four digits, dash, two digits, dash, two digits.
    #[must_use]
    pub fn parse(s: &str) -> Option<Date> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let digits = |r: std::ops::Range<usize>| -> Option<u32> {
            let mut n = 0;
            for &c in &b[r] {
                if !c.is_ascii_digit() {
                    return None;
                }
                n = n * 10 + u32::from(c - b'0');
            }
            Some(n)
        };
        Date::new(
            digits(0..4)? as i32,
            digits(5..7)? as u8,
            digits(8..10)? as u8,
        )
    }

    /// The first day of this date's month.
    #[must_use]
    pub fn first_of_month(self) -> Date {
        Date { d: 1, ..self }
    }

    /// The first day of the previous month.
    #[must_use]
    pub fn prev_month(self) -> Date {
        if self.m == 1 {
            Date {
                y: self.y - 1,
                m: 12,
                d: 1,
            }
        } else {
            Date {
                m: self.m - 1,
                d: 1,
                ..self
            }
        }
    }

    /// The first day of the next month.
    #[must_use]
    pub fn next_month(self) -> Date {
        if self.m == 12 {
            Date {
                y: self.y + 1,
                m: 1,
                d: 1,
            }
        } else {
            Date {
                m: self.m + 1,
                d: 1,
                ..self
            }
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

/// The weeks of a month as six rows of seven days; days outside the month
/// (and unused trailing rows) are `None`. An invalid month gives an empty grid.
#[must_use]
pub fn month_grid(y: i32, m: u8, start: WeekStart) -> [[Option<Date>; 7]; 6] {
    let mut grid = [[None; 7]; 6];
    let Some(first) = Date::new(y, m, 1) else {
        return grid;
    };
    let lead = match start {
        WeekStart::Monday => first.weekday(),
        WeekStart::Sunday => (first.weekday() + 1) % 7,
    } as usize;
    for d in 1..=days_in_month(y, m) {
        let i = lead + usize::from(d) - 1;
        grid[i / 7][i % 7] = Some(Date { y, m, d });
    }
    grid
}

/// The local date for Unix seconds and a whole-hour offset from UTC, or `None`
/// if the clock is before 2020 (taken to be unset).
#[must_use]
pub fn today_at(unix_secs: u64, offset_hours: i32) -> Option<Date> {
    if unix_secs < CLOCK_SET {
        return None;
    }
    let local = unix_secs as i64 + i64::from(offset_hours) * 3_600;
    Some(Date::from_days(local.div_euclid(86_400)))
}

/// Today's date. The `SIGMA_TODAY` environment variable (`yyyy-mm-dd`)
/// overrides the clock, for tests and screenshots; otherwise the system clock
/// plus `offset_hours`. `None` if the clock looks unset.
#[must_use]
pub fn today(offset_hours: i32) -> Option<Date> {
    if let Some(d) = std::env::var("SIGMA_TODAY")
        .ok()
        .and_then(|s| Date::parse(&s))
    {
        return Some(d);
    }
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    today_at(secs, offset_hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u8, d: u8) -> Date {
        Date::new(y, m, d).unwrap()
    }

    #[test]
    fn days_round_trip() {
        assert_eq!(dt(1970, 1, 1).to_days(), 0);
        for n in (-800_000..800_000).step_by(7) {
            let d = Date::from_days(n);
            assert_eq!(d.to_days(), n, "{d}");
            assert_eq!(Date::new(d.y, d.m, d.d), Some(d));
        }
        assert_eq!(Date::from_days(1).to_string(), "1970-01-02");
        assert_eq!(Date::from_days(-1).to_string(), "1969-12-31");
    }

    #[test]
    fn leap_years() {
        assert!(is_leap(2000) && is_leap(2024));
        assert!(!is_leap(1900) && !is_leap(2023));
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 13), 0);
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(1900, 2, 29).is_none());
        assert!(Date::new(2026, 4, 31).is_none());
        assert!(Date::new(2026, 0, 1).is_none());
        assert!(Date::new(2026, 1, 0).is_none());
    }

    #[test]
    fn weekdays() {
        assert_eq!(dt(2026, 11, 2).weekday(), 0);
        assert_eq!(dt(2026, 11, 8).weekday(), 6);
        assert_eq!(dt(1970, 1, 1).weekday(), 3);
        assert_eq!(dt(1969, 12, 31).weekday(), 2);
    }

    #[test]
    fn iso_weeks() {
        assert_eq!(dt(2020, 12, 31).iso_week(), (2020, 53));
        assert_eq!(dt(2021, 1, 3).iso_week(), (2020, 53));
        assert_eq!(dt(2021, 1, 4).iso_week(), (2021, 1));
        assert_eq!(dt(2025, 12, 29).iso_week(), (2026, 1));
        assert_eq!(dt(2026, 11, 2).iso_week(), (2026, 45));
    }

    #[test]
    fn add_days_and_months() {
        assert_eq!(dt(2026, 11, 2).add_days(30), dt(2026, 12, 2));
        assert_eq!(dt(2024, 3, 1).add_days(-1), dt(2024, 2, 29));
        assert_eq!(dt(2026, 11, 20).first_of_month(), dt(2026, 11, 1));
        assert_eq!(dt(2026, 1, 20).prev_month(), dt(2025, 12, 1));
        assert_eq!(dt(2026, 12, 20).next_month(), dt(2027, 1, 1));
        assert_eq!(dt(2026, 3, 31).prev_month(), dt(2026, 2, 1));
        assert_eq!(dt(2026, 11, 20).next_month(), dt(2026, 12, 1));
    }

    #[test]
    fn parse_is_strict() {
        assert_eq!(Date::parse("2026-11-02"), Some(dt(2026, 11, 2)));
        assert_eq!(dt(2026, 11, 2).to_string(), "2026-11-02");
        for bad in [
            "",
            "2026-1-02",
            "2026-11-2",
            "2026/11/02",
            "2026-11-02 ",
            " 2026-11-02",
            "2026-13-01",
            "2026-02-30",
            "2026-00-10",
            "2026-11-00",
            "+026-11-02",
            "2026-11-0a",
            "20261102",
            "2026-11-022",
        ] {
            assert_eq!(Date::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn grid_november_2026() {
        // 2026-11-01 is a Sunday.
        let g = month_grid(2026, 11, WeekStart::Monday);
        assert_eq!(g[0][5], None);
        assert_eq!(g[0][6], Some(dt(2026, 11, 1)));
        assert_eq!(g[1][0], Some(dt(2026, 11, 2)));
        assert_eq!(g[5][0], Some(dt(2026, 11, 30)));
        assert_eq!(g[5][1], None);

        let g = month_grid(2026, 11, WeekStart::Sunday);
        assert_eq!(g[0][0], Some(dt(2026, 11, 1)));
        assert_eq!(g[0][1], Some(dt(2026, 11, 2)));
        assert_eq!(g[4][1], Some(dt(2026, 11, 30)));
        assert_eq!(g[4][2], None);
        assert!(g[5].iter().all(Option::is_none));
        let n = g.iter().flatten().flatten().count();
        assert_eq!(n, 30);
    }

    #[test]
    fn grid_needs_six_rows() {
        // 2026-08-01 is a Saturday: 31 days from column 5 (Monday start).
        let g = month_grid(2026, 8, WeekStart::Monday);
        assert_eq!(g[5][0], Some(dt(2026, 8, 31)));
        assert!(month_grid(2026, 13, WeekStart::Monday)
            .iter()
            .flatten()
            .all(Option::is_none));
    }

    #[test]
    fn today_at_offsets_and_unset_clock() {
        // 2026-11-02 23:30:00Z
        let t = 1_793_662_200;
        assert_eq!(today_at(t, 0), Some(dt(2026, 11, 2)));
        assert_eq!(today_at(t, 1), Some(dt(2026, 11, 3)));
        assert_eq!(today_at(t, -23), Some(dt(2026, 11, 2)));
        assert_eq!(today_at(t, -24), Some(dt(2026, 11, 1)));
        assert_eq!(today_at(CLOCK_SET, -1), Some(dt(2019, 12, 31)));
        assert_eq!(today_at(CLOCK_SET, 0), Some(dt(2020, 1, 1)));
        assert_eq!(today_at(CLOCK_SET - 1, 0), None);
        assert_eq!(today_at(0, 5), None);
    }
}
