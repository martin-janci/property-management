//! Quiet-hours (Do Not Disturb) evaluation — Story 8B.3 (issue #980).
//!
//! Pure, timezone-aware logic shared by the notification pipeline's Push gate
//! and the `/schedule` "is currently quiet" readout. Kept free of DB/IO so it
//! can be unit-tested exhaustively (see `tests` below).
//!
//! The previous `check_quiet_hours` helper compared `Utc::now().time()` against
//! the configured window and explicitly ignored the user's stored timezone and
//! weekend settings — so a 22:00–07:00 "Europe/Prague" window was evaluated in
//! UTC, silently shifting the quiet period by 1–2 hours. This module evaluates
//! the window in the user's own timezone and honours the separate weekend
//! window, and also reports *when* the active window ends so held push
//! notifications can be scheduled for release.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use db::models::NotificationSchedule;

/// Result of evaluating a user's quiet-hours window at an instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuietHoursState {
    /// Whether the user is currently within a quiet-hours window.
    pub active: bool,
    /// When the current window ends, in UTC. `Some` only when `active`.
    /// Used as the `release_at` for notifications held during quiet hours.
    pub ends_at: Option<DateTime<Utc>>,
}

impl QuietHoursState {
    const INACTIVE: Self = Self {
        active: false,
        ends_at: None,
    };
}

/// Parse the schedule's IANA timezone, falling back to UTC on an unknown value
/// (matching the column default and never panicking on bad data).
fn resolve_tz(schedule: &NotificationSchedule) -> Tz {
    schedule.timezone.parse::<Tz>().unwrap_or(Tz::UTC)
}

/// Pick the applicable window for `local_weekday`: the dedicated weekend window
/// on Sat/Sun when weekend quiet hours are enabled, otherwise the weekday
/// window when quiet hours are enabled. Returns `None` when no window applies.
fn window_for(
    schedule: &NotificationSchedule,
    local_weekday: Weekday,
) -> Option<(NaiveTime, NaiveTime)> {
    let is_weekend = matches!(local_weekday, Weekday::Sat | Weekday::Sun);

    if is_weekend && schedule.weekend_quiet_hours_enabled {
        return match (
            schedule.weekend_quiet_hours_start,
            schedule.weekend_quiet_hours_end,
        ) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };
    }

    if schedule.quiet_hours_enabled {
        return match (schedule.quiet_hours_start, schedule.quiet_hours_end) {
            (Some(start), Some(end)) => Some((start, end)),
            _ => None,
        };
    }

    None
}

/// Convert a naive local date+time in `tz` to UTC, tolerating DST gaps/folds by
/// taking the earliest valid instant (falls back to the next day's same time if
/// the local time is entirely skipped, which cannot happen for the whole-minute
/// boundaries we use in practice).
fn local_to_utc(tz: Tz, date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    use chrono::TimeZone;
    let naive = date.and_time(time);
    match tz.from_local_datetime(&naive).earliest() {
        Some(dt) => dt.with_timezone(&Utc),
        // Spring-forward gap: nudge forward an hour and retry; worst case use UTC.
        None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc.from_utc_datetime(&naive)),
    }
}

/// Evaluate the user's quiet-hours window at `now`.
///
/// A window `[start, end)` that does not cross midnight (`start <= end`) is
/// active when `start <= local_time < end`. A window that crosses midnight
/// (`start > end`, e.g. 22:00–07:00) is active when `local_time >= start` OR
/// `local_time < end`.
pub fn evaluate(schedule: &NotificationSchedule, now: DateTime<Utc>) -> QuietHoursState {
    let tz = resolve_tz(schedule);
    let local = now.with_timezone(&tz);
    let local_date = local.date_naive();
    let local_time = local.time();

    let Some((start, end)) = window_for(schedule, local.weekday()) else {
        return QuietHoursState::INACTIVE;
    };

    // A degenerate start == end window is treated as "never quiet".
    if start == end {
        return QuietHoursState::INACTIVE;
    }

    if start < end {
        // Same-day window, e.g. 09:00–17:00.
        if local_time >= start && local_time < end {
            QuietHoursState {
                active: true,
                ends_at: Some(local_to_utc(tz, local_date, end)),
            }
        } else {
            QuietHoursState::INACTIVE
        }
    } else {
        // Crosses midnight, e.g. 22:00–07:00.
        if local_time >= start {
            // Evening portion: window ends tomorrow morning at `end`.
            QuietHoursState {
                active: true,
                ends_at: Some(local_to_utc(tz, local_date + Duration::days(1), end)),
            }
        } else if local_time < end {
            // Early-morning portion: window ends today at `end`.
            QuietHoursState {
                active: true,
                ends_at: Some(local_to_utc(tz, local_date, end)),
            }
        } else {
            QuietHoursState::INACTIVE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn schedule(tz: &str) -> NotificationSchedule {
        NotificationSchedule {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            quiet_hours_enabled: true,
            quiet_hours_start: NaiveTime::from_hms_opt(22, 0, 0),
            quiet_hours_end: NaiveTime::from_hms_opt(7, 0, 0),
            timezone: tz.to_string(),
            weekend_quiet_hours_enabled: false,
            weekend_quiet_hours_start: None,
            weekend_quiet_hours_end: None,
            digest_enabled: false,
            digest_frequency: None,
            digest_time: None,
            digest_day_of_week: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // 2026-06-04 is a Thursday (weekday).
    fn utc(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 4, h, m, 0).unwrap()
    }

    #[test]
    fn disabled_is_never_active() {
        let mut s = schedule("Europe/Prague");
        s.quiet_hours_enabled = false;
        assert!(!evaluate(&s, utc(23, 0)).active);
    }

    #[test]
    fn utc_crossing_midnight_evening_active() {
        let s = schedule("UTC");
        // 23:00 UTC is inside 22:00–07:00.
        let r = evaluate(&s, utc(23, 0));
        assert!(r.active);
        // Ends at 07:00 UTC the next day.
        assert_eq!(r.ends_at, Some(utc(7, 0) + Duration::days(1)));
    }

    #[test]
    fn utc_crossing_midnight_early_morning_active() {
        let s = schedule("UTC");
        let r = evaluate(&s, utc(3, 0));
        assert!(r.active);
        assert_eq!(r.ends_at, Some(utc(7, 0)));
    }

    #[test]
    fn utc_outside_window_inactive() {
        let s = schedule("UTC");
        assert!(!evaluate(&s, utc(12, 0)).active);
    }

    #[test]
    fn timezone_shifts_the_window() {
        // Window 22:00–07:00 Europe/Prague (summer = UTC+2).
        let s = schedule("Europe/Prague");
        // 19:00 UTC == 21:00 Prague -> NOT yet quiet.
        assert!(!evaluate(&s, utc(19, 0)).active);
        // 20:00 UTC == 22:00 Prague -> quiet begins.
        assert!(evaluate(&s, utc(20, 0)).active);
        // The buggy UTC-only impl would have called 22:00 UTC quiet and 20:00
        // UTC not-quiet — this asserts we evaluate in the user's timezone.
    }

    #[test]
    fn same_day_window() {
        let mut s = schedule("UTC");
        s.quiet_hours_start = NaiveTime::from_hms_opt(9, 0, 0);
        s.quiet_hours_end = NaiveTime::from_hms_opt(17, 0, 0);
        assert!(evaluate(&s, utc(12, 0)).active);
        assert_eq!(evaluate(&s, utc(12, 0)).ends_at, Some(utc(17, 0)));
        assert!(!evaluate(&s, utc(8, 0)).active);
        assert!(!evaluate(&s, utc(18, 0)).active);
    }

    #[test]
    fn weekend_window_used_on_saturday() {
        // 2026-06-06 is a Saturday.
        let mut s = schedule("UTC");
        s.quiet_hours_enabled = false; // weekday window off
        s.weekend_quiet_hours_enabled = true;
        s.weekend_quiet_hours_start = NaiveTime::from_hms_opt(10, 0, 0);
        s.weekend_quiet_hours_end = NaiveTime::from_hms_opt(14, 0, 0);
        let sat_noon = Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap();
        assert!(evaluate(&s, sat_noon).active);
        // On a weekday the (disabled) weekday window means not active.
        assert!(!evaluate(&s, utc(12, 0)).active);
    }

    #[test]
    fn unknown_timezone_falls_back_to_utc() {
        let s = schedule("Not/AZone");
        // Treated as UTC: 23:00 is inside 22:00–07:00.
        assert!(evaluate(&s, utc(23, 0)).active);
    }

    #[test]
    fn degenerate_equal_window_inactive() {
        let mut s = schedule("UTC");
        s.quiet_hours_start = NaiveTime::from_hms_opt(8, 0, 0);
        s.quiet_hours_end = NaiveTime::from_hms_opt(8, 0, 0);
        assert!(!evaluate(&s, utc(8, 0)).active);
    }
}
