use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Duration, Utc, Weekday};

/// The time span a retro covers, plus the label used in the rendered title
/// and the feed post. `since`/`until` are always resolved to concrete
/// instants before any collector runs, so every source filters against the
/// same window regardless of how it was requested on the CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetroWindow {
    pub label: String,
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
}

impl RetroWindow {
    pub fn daily(now: DateTime<Utc>) -> Self {
        Self {
            label: "daily".to_string(),
            since: now - Duration::hours(24),
            until: now,
        }
    }

    pub fn weekly(now: DateTime<Utc>) -> Self {
        Self {
            label: "weekly".to_string(),
            since: now - Duration::days(7),
            until: now,
        }
    }

    pub fn custom(since: DateTime<Utc>, until: DateTime<Utc>) -> Result<Self> {
        if since >= until {
            bail!("--since must be before --until (got {since} .. {until})");
        }
        Ok(Self {
            label: "custom".to_string(),
            since,
            until,
        })
    }

    /// True on Sundays local-naively treated as UTC-Sunday for the LaunchAgent
    /// wrapper's "also run weekly today" check. The daily job always runs;
    /// this only decides whether the weekly companion run fires alongside it.
    pub fn is_weekly_day(now: DateTime<Utc>) -> bool {
        now.weekday() == Weekday::Sun
    }

    pub fn contains(&self, ts: DateTime<Utc>) -> bool {
        ts >= self.since && ts < self.until
    }

    pub fn duration_hours(&self) -> i64 {
        (self.until - self.since).num_hours()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn daily_window_spans_24_hours_ending_now() {
        let now = Utc.with_ymd_and_hms(2026, 7, 5, 21, 0, 0).unwrap();
        let window = RetroWindow::daily(now);
        assert_eq!(window.duration_hours(), 24);
        assert_eq!(window.until, now);
    }

    #[test]
    fn weekly_window_spans_7_days() {
        let now = Utc.with_ymd_and_hms(2026, 7, 5, 21, 0, 0).unwrap();
        let window = RetroWindow::weekly(now);
        assert_eq!(window.duration_hours(), 24 * 7);
    }

    #[test]
    fn custom_window_rejects_inverted_range() {
        let a = Utc.with_ymd_and_hms(2026, 7, 5, 0, 0, 0).unwrap();
        let b = Utc.with_ymd_and_hms(2026, 7, 4, 0, 0, 0).unwrap();
        assert!(RetroWindow::custom(a, b).is_err());
    }

    #[test]
    fn contains_is_half_open_since_inclusive_until_exclusive() {
        let since = Utc.with_ymd_and_hms(2026, 7, 4, 21, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 7, 5, 21, 0, 0).unwrap();
        let window = RetroWindow::custom(since, until).unwrap();
        assert!(window.contains(since));
        assert!(!window.contains(until));
        assert!(window.contains(since + Duration::hours(1)));
    }

    #[test]
    fn is_weekly_day_matches_sunday() {
        let sunday = Utc.with_ymd_and_hms(2026, 7, 5, 21, 0, 0).unwrap();
        let monday = Utc.with_ymd_and_hms(2026, 7, 6, 21, 0, 0).unwrap();
        assert!(RetroWindow::is_weekly_day(sunday));
        assert!(!RetroWindow::is_weekly_day(monday));
    }
}
