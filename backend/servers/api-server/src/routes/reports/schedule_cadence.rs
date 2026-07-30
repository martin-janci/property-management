//! Schedule cadence, cron matching, and next-run computation for report
//! schedules (Epic 55 reporting).
//!
//! Pure, DB-free helpers extracted verbatim from `routes::reports` to isolate
//! the schedule-timing concern - cadence resolution, 5-field UNIX cron
//! matching, `HH:MM` parsing, and DST-aware next-run computation - which
//! changes independently of the report request handlers. Behavior-preserving
//! move; see issues #2198, #2242, #2303.

use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use db::models::ReportSchedule;

/// Allowed schedule frequencies (matches the `report_schedules.frequency`
/// CHECK constraint in migration 00162).
pub(super) const VALID_SCHEDULE_FREQUENCIES: &[&str] = &["daily", "weekly", "monthly"];

/// Allowed export formats for a schedule.
pub(super) const VALID_SCHEDULE_FORMATS: &[&str] = &["pdf", "excel", "csv"];

/// Default delivery time (HH:MM) when the caller omits `time`.
pub(super) const DEFAULT_SCHEDULE_TIME: &str = "08:00";

/// Parse an `HH:MM` 24-hour time-of-day string into `(hour, minute)`.
///
/// Rejects empty components and non-numeric input; enforces 24h/60m bounds.
pub(super) fn parse_time_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    if h.is_empty() || m.is_empty() {
        return None;
    }
    let hour = h.parse::<u32>().ok()?;
    let minute = m.parse::<u32>().ok()?;
    (hour <= 23 && minute <= 59).then_some((hour, minute))
}

/// Validate an `HH:MM` 24-hour time-of-day string.
fn validate_time_hhmm(s: &str) -> bool {
    parse_time_hhmm(s).is_some()
}

/// Compute the first `next_run_at` (UTC) for a freshly created schedule
/// (issue #2198).
///
/// Walks forward from "now" in the schedule's `tz` to the first calendar day
/// that satisfies the cadence, then anchors the `HH:MM` delivery time on that
/// day and converts back to UTC. Returns `None` only if no matching day is
/// found within the search horizon (should not happen for valid input).
///
/// Cadence rules (mirrors the create-schedule validation):
/// - `daily`   — today if the time is still in the future, else tomorrow.
/// - `weekly`  — the next date whose weekday matches `day_of_week`
///   (0=Sunday … 6=Saturday), today included only if the time has not passed.
/// - `monthly` — the next date whose day-of-month equals `day_of_month`. Months
///   that lack that day (e.g. day 31 in April, or 29–31 in February) are
///   skipped to the next month that has it, rather than clamped.
///
/// DST handling mirrors `services::quiet_hours::local_to_utc`: ambiguous local
/// times (fall-back) take the earliest instant; a skipped local time
/// (spring-forward gap) nudges forward one hour.
pub(super) fn compute_first_next_run(
    frequency: &str,
    day_of_week: Option<i32>,
    day_of_month: Option<i32>,
    hour: u32,
    minute: u32,
    tz: Tz,
) -> Option<DateTime<Utc>> {
    compute_next_run_after(
        Utc::now().with_timezone(&tz),
        frequency,
        day_of_week,
        day_of_month,
        hour,
        minute,
    )
}

/// Compute the first cadence-matching run strictly after `now` (UTC result).
///
/// This is the injectable core of [`compute_first_next_run`]: it takes the
/// reference instant explicitly (in the schedule's timezone) instead of reading
/// the wall clock, which makes the DST edge cases (spring-forward gap /
/// fall-back ambiguity) unit-testable and provides the reusable primitive a
/// post-fire `next_run_at` advance would call with `now = last fire` (issue
/// #2242, finding 1). The schedule timezone is taken from `now.timezone()`.
///
/// Cadence and DST semantics are identical to [`compute_first_next_run`].
fn compute_next_run_after(
    now: DateTime<Tz>,
    frequency: &str,
    day_of_week: Option<i32>,
    day_of_month: Option<i32>,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Utc>> {
    let tz = now.timezone();
    let today = now.date_naive();
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;

    // Horizon of ~2 years bounds the monthly day-of-month edge cases.
    for add in 0..=800u64 {
        let date = today.checked_add_days(chrono::Days::new(add))?;
        let matches = match frequency {
            "daily" => true,
            "weekly" => {
                day_of_week.is_some_and(|d| date.weekday().num_days_from_sunday() as i32 == d)
            }
            "monthly" => day_of_month.is_some_and(|d| date.day() as i32 == d),
            _ => false,
        };
        if !matches {
            continue;
        }

        let naive = date.and_time(time);
        let candidate = match tz.from_local_datetime(&naive).earliest() {
            Some(dt) => dt,
            // Spring-forward gap: nudge forward an hour; worst case anchor in UTC.
            None => tz
                .from_local_datetime(&(naive + chrono::Duration::hours(1)))
                .earliest()
                .unwrap_or_else(|| Utc.from_utc_datetime(&naive).with_timezone(&tz)),
        };

        if candidate > now {
            return Some(candidate.with_timezone(&Utc));
        }
    }
    None
}

/// Test whether a single already-validated cron field matches `value`.
///
/// Handles the four forms accepted by [`validate_cron_expression`]: `*`,
/// comma-separated lists, `lo-hi` ranges, and `base/step` (where `base` is
/// `*`, a single number, or a range). Each comma part is reduced to an
/// inclusive `(lo, hi, step)` window and matched with `lo <= value <= hi`
/// and `(value - lo) % step == 0` — the same semantics Vixie cron uses.
/// `field_min` / `field_max` supply the range bounds for `*` and open-ended
/// `base/step` forms (e.g. `5/10` on the minute field means 5,15,25,…,55).
fn cron_field_matches(field: &str, value: u32, field_min: u32, field_max: u32) -> bool {
    for part in field.split(',') {
        let (base, step) = match part.split_once('/') {
            Some((b, s)) => (b, s.parse::<u32>().unwrap_or(1).max(1)),
            None => (part, 1),
        };
        let (lo, hi) = if base == "*" {
            (field_min, field_max)
        } else if let Some((l, h)) = base.split_once('-') {
            match (l.parse::<u32>(), h.parse::<u32>()) {
                (Ok(l), Ok(h)) => (l, h),
                _ => continue,
            }
        } else {
            match base.parse::<u32>() {
                // A bare `base/step` (single number + step) runs from that
                // number up to the field maximum, matching Vixie semantics.
                Ok(v) if step > 1 => (v, field_max),
                Ok(v) => (v, v),
                _ => continue,
            }
        };
        if value >= lo && value <= hi && (value - lo).is_multiple_of(step) {
            return true;
        }
    }
    false
}

/// Compute the next fire time (UTC) strictly after `now` for a 5-field UNIX
/// cron expression, evaluated in `now`'s timezone (issue #2242).
///
/// This is the cron analogue of [`compute_next_run_after`]: when a schedule's
/// `cron_expression` is edited, the handler calls this to recompute
/// `next_run_at` so the schedule fires at the *new* cadence instead of the
/// stale time carried over from the previous expression. It walks forward one
/// minute at a time over real UTC instants (up to a ~366-day horizon that
/// bounds sparse expressions like `0 0 29 2 *`), converting each instant into
/// the schedule timezone before matching the five fields — so DST transitions
/// are handled naturally without constructing ambiguous local times.
///
/// The day-of-month / day-of-week combination follows Vixie cron: when both
/// fields are restricted (neither is `*`) a day matches if *either* matches;
/// otherwise the restricted field is ANDed with the other. Cron day-of-week 7
/// is treated as Sunday (0), matching `validate_cron_expression`.
///
/// Returns `None` only for a malformed expression or one with no match inside
/// the horizon — callers leave `next_run_at` unchanged in that case.
pub(super) fn compute_next_run_from_cron(now: DateTime<Tz>, cron: &str) -> Option<DateTime<Utc>> {
    let fields: Vec<&str> = cron.split_whitespace().collect();
    if fields.len() != 5 {
        return None;
    }
    let (minute_f, hour_f, dom_f, month_f, dow_f) =
        (fields[0], fields[1], fields[2], fields[3], fields[4]);
    let dom_restricted = dom_f != "*";
    let dow_restricted = dow_f != "*";

    let tz = now.timezone();
    // Start from the top of the next minute so we never return `now` itself.
    let start =
        now.with_timezone(&Utc).with_second(0)?.with_nanosecond(0)? + chrono::Duration::minutes(1);

    // 366 days of minute steps bounds the sparsest valid expression.
    for step in 0..(366 * 24 * 60) {
        let instant = start + chrono::Duration::minutes(step);
        let local = instant.with_timezone(&tz);

        if !cron_field_matches(minute_f, local.minute(), 0, 59) {
            continue;
        }
        if !cron_field_matches(hour_f, local.hour(), 0, 23) {
            continue;
        }
        if !cron_field_matches(month_f, local.month(), 1, 12) {
            continue;
        }

        let dom = local.day();
        let dow = local.weekday().num_days_from_sunday();
        let dom_match = cron_field_matches(dom_f, dom, 1, 31);
        // Sunday is 0 in chrono; cron also accepts 7 for Sunday.
        let dow_match = cron_field_matches(dow_f, dow, 0, 7)
            || (dow == 0 && cron_field_matches(dow_f, 7, 0, 7));
        let day_match = if dom_restricted && dow_restricted {
            dom_match || dow_match
        } else {
            dom_match && dow_match
        };
        if day_match {
            return Some(instant);
        }
    }
    None
}

/// Resolve a schedule's next fire time from its canonical cadence (issue #2303).
///
/// `cron_expression` is the single source of truth when present; otherwise the
/// legacy `frequency`/`day_of_week`/`day_of_month`/`time` columns are used. This
/// one resolver is shared by the background scheduler's post-fire advance (see
/// `services::scheduler::Scheduler::fire_due_report_schedules`) so the
/// recomputed `next_run_at` can never disagree with the cadence model a schedule
/// was created or edited with — the dual-cadence reconciliation for finding 2.
/// Evaluated in the schedule's stored timezone; falls back to UTC if that
/// somehow fails to parse (create/edit reject invalid zones at the edge).
///
/// Returns `None` when the cadence yields no future run inside the walker's
/// horizon (or the cron expression is malformed) — the caller parks the
/// schedule by storing a NULL `next_run_at`.
pub(crate) fn compute_next_run_for_schedule(
    schedule: &ReportSchedule,
    after: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let tz: Tz = schedule.timezone.parse().unwrap_or(chrono_tz::UTC);
    let now = after.with_timezone(&tz);
    match schedule.cron_expression.as_deref() {
        Some(cron) => compute_next_run_from_cron(now, cron),
        None => {
            let (hour, minute) = parse_time_hhmm(&schedule.time).unwrap_or((8, 0));
            compute_next_run_after(
                now,
                &schedule.frequency,
                schedule.day_of_week,
                schedule.day_of_month,
                hour,
                minute,
            )
        }
    }
}

/// Validate a 5-field UNIX cron expression (minute hour dom month dow).
pub(super) fn validate_cron_expression(expr: &str) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    fn valid_field(field: &str, min: u32, max: u32) -> bool {
        for part in field.split(',') {
            let (base, step) = if let Some((b, s)) = part.split_once('/') {
                (b, Some(s))
            } else {
                (part, None)
            };
            if let Some(s) = step {
                if s.parse::<u32>().map_or(true, |v| v == 0) {
                    return false;
                }
            }
            if base != "*" {
                if let Some((lo, hi)) = base.split_once('-') {
                    match (lo.parse::<u32>(), hi.parse::<u32>()) {
                        (Ok(l), Ok(h)) if l >= min && h <= max && l <= h => {}
                        _ => return false,
                    }
                } else {
                    match base.parse::<u32>() {
                        Ok(v) if (min..=max).contains(&v) => {}
                        _ => return false,
                    }
                }
            }
        }
        true
    }
    // minute(0-59) hour(0-23) dom(1-31) month(1-12) dow(0-7)
    valid_field(fields[0], 0, 59)
        && valid_field(fields[1], 0, 23)
        && valid_field(fields[2], 1, 31)
        && valid_field(fields[3], 1, 12)
        && valid_field(fields[4], 0, 7)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_first_next_run, compute_next_run_after, compute_next_run_for_schedule,
        compute_next_run_from_cron, cron_field_matches, parse_time_hhmm, validate_cron_expression,
        validate_time_hhmm, ReportSchedule,
    };
    use chrono::{Datelike, LocalResult, TimeZone, Timelike, Utc};
    use chrono_tz::Tz;

    // --- compute_next_run_for_schedule: canonical cadence resolver (issue #2303) ---
    //
    // This resolver is the single source of truth the background scheduler's
    // post-fire advance uses. `cron_expression` wins when set; otherwise the
    // legacy frequency/day/time columns are used. These no-DB tests pin that
    // reconciliation so a schedule's re-computed next_run_at can never disagree
    // with the cadence model it was created/edited with (finding 2).

    fn schedule_fixture(
        frequency: &str,
        day_of_week: Option<i32>,
        day_of_month: Option<i32>,
        time: &str,
        timezone: &str,
        cron_expression: Option<&str>,
    ) -> ReportSchedule {
        let now = Utc::now();
        ReportSchedule {
            id: uuid::Uuid::new_v4(),
            report_id: uuid::Uuid::new_v4(),
            organization_id: uuid::Uuid::new_v4(),
            name: "fixture".to_string(),
            frequency: frequency.to_string(),
            day_of_week,
            day_of_month,
            time: time.to_string(),
            timezone: timezone.to_string(),
            format: "pdf".to_string(),
            recipients: vec![],
            is_active: true,
            status: "active".to_string(),
            last_run_at: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
            cron_expression: cron_expression.map(str::to_string),
        }
    }

    #[test]
    fn resolver_uses_cron_expression_when_present() {
        // cron says "every Friday 17:00" while the legacy columns say "weekly
        // Monday 08:00" — the resolver must follow cron, never the stale legacy
        // cadence (the exact dual-model split the issue flags).
        let sched = schedule_fixture(
            "weekly",
            Some(1), // Monday, legacy
            None,
            "08:00", // legacy
            "UTC",
            Some("0 17 * * 5"), // cron: Friday 17:00
        );
        let after = Utc.with_ymd_and_hms(2026, 7, 13, 6, 0, 0).unwrap(); // a Monday
        let next = compute_next_run_for_schedule(&sched, after).expect("cron cadence resolves");
        assert!(next > after);
        assert_eq!((next.hour(), next.minute()), (17, 0));
        assert_eq!(
            next.weekday().num_days_from_sunday(),
            5,
            "cron_expression must win over the legacy Monday/08:00 columns"
        );
    }

    #[test]
    fn resolver_falls_back_to_legacy_columns_without_cron() {
        // No cron_expression → the legacy weekly/Wednesday/09:00 cadence drives it.
        let sched = schedule_fixture("weekly", Some(3), None, "09:00", "UTC", None);
        let after = Utc.with_ymd_and_hms(2026, 7, 13, 6, 0, 0).unwrap();
        let next = compute_next_run_for_schedule(&sched, after).expect("legacy cadence resolves");
        assert!(next > after);
        assert_eq!((next.hour(), next.minute()), (9, 0));
        assert_eq!(
            next.weekday().num_days_from_sunday(),
            3,
            "legacy weekly cadence must land on Wednesday"
        );
    }

    #[test]
    fn resolver_returns_none_for_unsatisfiable_cadence() {
        // A malformed cron expression yields no run → None, which the scheduler
        // turns into a parked (NULL next_run_at) schedule instead of a re-fire loop.
        let sched = schedule_fixture("weekly", None, None, "08:00", "UTC", Some("not a cron"));
        let after = Utc.with_ymd_and_hms(2026, 7, 13, 6, 0, 0).unwrap();
        assert!(compute_next_run_for_schedule(&sched, after).is_none());
    }

    // --- parse_time_hhmm (issue #2198) ---

    #[test]
    fn parse_time_returns_components() {
        assert_eq!(parse_time_hhmm("08:30"), Some((8, 30)));
        assert_eq!(parse_time_hhmm("0:0"), Some((0, 0)));
        assert_eq!(parse_time_hhmm("23:59"), Some((23, 59)));
        assert_eq!(parse_time_hhmm("24:00"), None);
        assert_eq!(parse_time_hhmm("aa:bb"), None);
    }

    // --- compute_first_next_run (issue #2198) ---

    #[test]
    fn next_run_daily_is_in_the_future() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();
        let next = compute_first_next_run("daily", None, None, 8, 30, tz)
            .expect("daily schedule must yield a next run");
        assert!(
            next > chrono::Utc::now(),
            "next_run_at must be in the future"
        );
        // Anchored at 08:30 local time.
        let local = next.with_timezone(&tz);
        assert_eq!((local.hour(), local.minute()), (8, 30));
    }

    #[test]
    fn next_run_weekly_lands_on_requested_weekday() {
        let tz: Tz = "UTC".parse().unwrap();
        // day_of_week = 3 (Wednesday, 0=Sunday).
        let next = compute_first_next_run("weekly", Some(3), None, 9, 0, tz)
            .expect("weekly schedule must yield a next run");
        let local = next.with_timezone(&tz);
        assert_eq!(
            local.weekday().num_days_from_sunday(),
            3,
            "weekly next run must fall on the requested weekday"
        );
        assert!(next > chrono::Utc::now());
    }

    #[test]
    fn next_run_monthly_lands_on_requested_day() {
        let tz: Tz = "UTC".parse().unwrap();
        // day_of_month = 15.
        let next = compute_first_next_run("monthly", None, Some(15), 6, 0, tz)
            .expect("monthly schedule must yield a next run");
        let local = next.with_timezone(&tz);
        assert_eq!(local.day(), 15, "monthly next run must fall on day 15");
        assert!(next > chrono::Utc::now());
    }

    #[test]
    fn next_run_monthly_day_31_skips_short_months() {
        let tz: Tz = "UTC".parse().unwrap();
        // Day 31 only exists in some months; the walker must still find one.
        let next = compute_first_next_run("monthly", None, Some(31), 0, 0, tz)
            .expect("day-31 schedule must resolve to a 31-day month");
        assert_eq!(next.with_timezone(&tz).day(), 31);
        assert!(next > chrono::Utc::now());
    }

    // --- compute_next_run_from_cron: recompute on cron edit (issue #2242) ---
    //
    // Regression guard: `update_schedule` must recompute `next_run_at` from the
    // NEW cron expression, otherwise an edited schedule keeps firing at the
    // stale time. These no-DB unit tests pin the pure computation the handler
    // feeds into the repository UPDATE.

    #[test]
    fn cron_next_run_is_in_the_future_and_matches_time() {
        let tz: Tz = "UTC".parse().unwrap();
        // Every Monday at 08:00.
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 12, 0, 0).unwrap();
        let next = compute_next_run_from_cron(now.with_timezone(&tz), "0 8 * * 1")
            .expect("weekly cron must yield a next run");
        assert!(next > now, "recomputed next_run_at must be in the future");
        let local = next.with_timezone(&tz);
        assert_eq!((local.hour(), local.minute()), (8, 0));
        assert_eq!(
            local.weekday().num_days_from_sunday(),
            1,
            "cron `0 8 * * 1` must land on a Monday"
        );
    }

    #[test]
    fn cron_edit_moves_next_run_to_the_new_cadence() {
        // Simulate the edit: schedule was "every Monday 08:00", user changes it
        // to "every Friday 17:00". The recomputed next_run_at must reflect the
        // NEW expression (a Friday at 17:00), never the old Monday time.
        let tz: Tz = "Europe/Bratislava".parse().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 6, 0, 0).unwrap(); // a Monday
        let old = compute_next_run_from_cron(now.with_timezone(&tz), "0 8 * * 1").unwrap();
        let new = compute_next_run_from_cron(now.with_timezone(&tz), "0 17 * * 5").unwrap();
        assert_ne!(old, new, "cron change must recompute next_run_at");
        let local = new.with_timezone(&tz);
        assert_eq!((local.hour(), local.minute()), (17, 0));
        assert_eq!(
            local.weekday().num_days_from_sunday(),
            5,
            "cron `0 17 * * 5` must land on a Friday"
        );
        assert!(new > now);
    }

    #[test]
    fn cron_next_run_handles_step_and_list_and_sunday_seven() {
        let tz: Tz = "UTC".parse().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 13, 10, 7, 0).unwrap();
        // `*/15` minutes → next quarter-hour boundary after 10:07 is 10:15.
        let next = compute_next_run_from_cron(now.with_timezone(&tz), "*/15 * * * *").unwrap();
        assert_eq!(next.with_timezone(&tz).minute() % 15, 0);
        assert!(next > now);
        // dow 7 must be accepted as Sunday.
        let sun = compute_next_run_from_cron(now.with_timezone(&tz), "0 9 * * 7").unwrap();
        assert_eq!(sun.with_timezone(&tz).weekday().num_days_from_sunday(), 0);
    }

    #[test]
    fn cron_next_run_dom_dow_union_when_both_restricted() {
        // Vixie semantics: when BOTH day-of-month and day-of-week are set, a day
        // matches if EITHER matches. `0 0 13 * 5` → the 13th OR any Friday.
        let tz: Tz = "UTC".parse().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let next = compute_next_run_from_cron(now.with_timezone(&tz), "0 0 13 * 5").unwrap();
        let local = next.with_timezone(&tz);
        let is_13th = local.day() == 13;
        let is_friday = local.weekday().num_days_from_sunday() == 5;
        assert!(
            is_13th || is_friday,
            "union match: day must be the 13th or a Friday, got {local}"
        );
    }

    #[test]
    fn cron_next_run_rejects_malformed_expression() {
        let tz: Tz = "UTC".parse().unwrap();
        let now = Utc::now().with_timezone(&tz);
        assert_eq!(compute_next_run_from_cron(now, "* * * *"), None);
        assert_eq!(compute_next_run_from_cron(now, ""), None);
    }

    #[test]
    fn cron_field_matches_covers_forms() {
        // wildcard
        assert!(cron_field_matches("*", 5, 0, 59));
        // single
        assert!(cron_field_matches("8", 8, 0, 23));
        assert!(!cron_field_matches("8", 9, 0, 23));
        // range
        assert!(cron_field_matches("9-17", 12, 0, 23));
        assert!(!cron_field_matches("9-17", 8, 0, 23));
        // list
        assert!(cron_field_matches("0,30", 30, 0, 59));
        assert!(!cron_field_matches("0,30", 15, 0, 59));
        // step over wildcard
        assert!(cron_field_matches("*/15", 45, 0, 59));
        assert!(!cron_field_matches("*/15", 20, 0, 59));
        // bare base with step (5/10 → 5,15,25,…)
        assert!(cron_field_matches("5/10", 25, 0, 59));
        assert!(!cron_field_matches("5/10", 20, 0, 59));
        // range with step
        assert!(cron_field_matches("5-15/5", 15, 0, 59));
        assert!(!cron_field_matches("5-15/5", 12, 0, 59));
    }

    // --- compute_next_run_after: DST edge cases (issue #2242, finding 2) ---
    //
    // These exercise the branches in the walker that plain daily/weekly/monthly
    // tests can't reach, because the real-clock `compute_first_next_run` never
    // lands its search on a specific DST-transition date. `compute_next_run_after`
    // injects the reference instant so we can anchor a delivery time inside the
    // spring-forward gap / fall-back ambiguous hour.
    //
    // Europe/Bratislava (CET/CEST) transitions, verified against chrono-tz:
    //  - Spring forward: 2027-03-28, 02:00 -> 03:00 (02:00..03:00 does not exist).
    //  - Fall back:      2026-10-25, 03:00 -> 02:00 (02:00..03:00 occurs twice).

    #[test]
    fn next_run_spring_forward_gap_nudges_into_valid_instant() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();

        // Precondition: 02:30 on the transition date is a genuine gap (no local
        // instant). If chrono-tz ever changes its rules this asserts loudly
        // rather than silently testing the wrong branch.
        assert!(
            matches!(
                tz.with_ymd_and_hms(2027, 3, 28, 2, 30, 0),
                LocalResult::None
            ),
            "02:30 on 2027-03-28 must be a spring-forward gap"
        );

        // Reference instant: 00:30 local on the transition day, before the gap.
        let now = tz.with_ymd_and_hms(2027, 3, 28, 0, 30, 0).unwrap();

        // Daily schedule at 02:30 -> today's candidate falls in the gap.
        let next = compute_next_run_after(now, "daily", None, None, 2, 30)
            .expect("gap-anchored schedule must still yield a next run");

        // Must not panic, must be strictly after `now`, and must be a real
        // instant nudged forward out of the gap (03:30 local == 01:30 UTC).
        assert!(
            next > now.with_timezone(&Utc),
            "next run must be in the future"
        );
        let local = next.with_timezone(&tz);
        assert_eq!(
            (local.year(), local.month(), local.day()),
            (2027, 3, 28),
            "next run stays on the transition date"
        );
        assert_eq!(
            (local.hour(), local.minute()),
            (3, 30),
            "gap time 02:30 nudges forward one hour to 03:30 CEST"
        );
        assert_eq!(
            next,
            Utc.with_ymd_and_hms(2027, 3, 28, 1, 30, 0).unwrap(),
            "03:30 CEST is 01:30 UTC"
        );
    }

    #[test]
    fn next_run_fall_back_ambiguous_picks_earliest() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();

        // Precondition: 02:30 on the fall-back date is ambiguous (two instants).
        assert!(
            matches!(
                tz.with_ymd_and_hms(2026, 10, 25, 2, 30, 0),
                LocalResult::Ambiguous(_, _)
            ),
            "02:30 on 2026-10-25 must be a fall-back ambiguous time"
        );

        // Reference instant: 00:30 local, before the ambiguous hour.
        let now = tz.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap();

        let next = compute_next_run_after(now, "daily", None, None, 2, 30)
            .expect("ambiguous-anchored schedule must still yield a next run");

        assert!(
            next > now.with_timezone(&Utc),
            "next run must be in the future"
        );
        let local = next.with_timezone(&tz);
        assert_eq!(
            (local.hour(), local.minute()),
            (2, 30),
            "fall-back time is kept at 02:30 local"
        );
        // The earliest of the two 02:30 instants is the CEST one (UTC+2),
        // i.e. 00:30 UTC — not the later CET (UTC+1) 01:30 UTC.
        assert_eq!(
            next,
            Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap(),
            "the earliest (CEST, +02:00) instant must be chosen"
        );
    }

    // --- validate_time_hhmm (gap-81-1: create schedule) ---

    #[test]
    fn time_valid_values() {
        for t in ["00:00", "08:00", "08:30", "23:59", "9:05", "0:0"] {
            assert!(validate_time_hhmm(t), "{t:?} should be a valid HH:MM time");
        }
    }

    #[test]
    fn time_invalid_values() {
        for t in [
            "", ":", "24:00", "08:60", "08", "08:", ":30", "aa:bb", "-1:00", "12:-5", "8:0:0",
        ] {
            assert!(!validate_time_hhmm(t), "{t:?} should be rejected");
        }
    }

    // --- valid expressions ---

    #[test]
    fn valid_every_minute() {
        assert!(validate_cron_expression("* * * * *"));
    }

    #[test]
    fn valid_specific_time() {
        // Every Monday at 08:00
        assert!(validate_cron_expression("0 8 * * 1"));
    }

    #[test]
    fn valid_ranges_and_lists() {
        assert!(validate_cron_expression("0,30 9-17 1-31 1-12 0-7"));
    }

    #[test]
    fn valid_step_syntax() {
        // Every 5 minutes
        assert!(validate_cron_expression("*/5 * * * *"));
    }

    #[test]
    fn valid_boundary_values() {
        assert!(validate_cron_expression("59 23 31 12 7"));
    }

    // --- invalid expressions ---

    #[test]
    fn invalid_too_few_fields() {
        assert!(!validate_cron_expression("* * * *"));
    }

    #[test]
    fn invalid_too_many_fields() {
        assert!(!validate_cron_expression("* * * * * *"));
    }

    #[test]
    fn invalid_empty_string() {
        assert!(!validate_cron_expression(""));
    }

    #[test]
    fn invalid_minute_out_of_range() {
        // minute must be 0-59
        assert!(!validate_cron_expression("60 * * * *"));
    }

    #[test]
    fn invalid_hour_out_of_range() {
        // hour must be 0-23
        assert!(!validate_cron_expression("* 24 * * *"));
    }

    #[test]
    fn invalid_dom_zero() {
        // day-of-month must be 1-31
        assert!(!validate_cron_expression("* * 0 * *"));
    }

    #[test]
    fn invalid_month_too_large() {
        // month must be 1-12
        assert!(!validate_cron_expression("* * * 13 *"));
    }

    #[test]
    fn invalid_dow_out_of_range() {
        // day-of-week must be 0-7
        assert!(!validate_cron_expression("* * * * 8"));
    }

    #[test]
    fn invalid_step_zero() {
        // step of 0 is meaningless
        assert!(!validate_cron_expression("*/0 * * * *"));
    }

    #[test]
    fn invalid_non_numeric_field() {
        assert!(!validate_cron_expression("foo * * * *"));
    }

    #[test]
    fn invalid_reversed_range() {
        // lo > hi in a range
        assert!(!validate_cron_expression("* * 31-1 * *"));
    }

    // --- macros / shorthand (gap-81-1: PR #531 review) ---
    //
    // The validator implements *only* the 5-field UNIX syntax. The `@reboot`,
    // `@daily`, `@hourly`, etc. macros are NOT supported: each macro is a single
    // whitespace-delimited token, so the field-count guard rejects them. These
    // tests pin that contract so a future refactor can't silently start
    // accepting (or mis-parsing) macros without updating the handler docs.

    #[test]
    fn invalid_reboot_macro_rejected() {
        // `@reboot` is one token -> field count is 1, not 5 -> rejected.
        assert!(!validate_cron_expression("@reboot"));
    }

    #[test]
    fn invalid_named_macros_rejected() {
        for macro_expr in [
            "@daily",
            "@hourly",
            "@weekly",
            "@monthly",
            "@yearly",
            "@annually",
        ] {
            assert!(
                !validate_cron_expression(macro_expr),
                "macro {macro_expr:?} must be rejected (macros are unsupported)"
            );
        }
    }

    #[test]
    fn invalid_macro_padded_to_five_tokens_rejected() {
        // Even if a macro string happens to contain five whitespace-separated
        // tokens, each token must still be a valid numeric/`*` field.
        assert!(!validate_cron_expression(
            "@hourly @daily @weekly @monthly @yearly"
        ));
    }

    // --- additional edge cases ---

    #[test]
    fn valid_range_with_step() {
        // A step applied to an explicit range: minutes 5..15 every 3.
        assert!(validate_cron_expression("5-15/3 * * * *"));
    }

    #[test]
    fn valid_step_on_wildcard_dow() {
        // `*/2` over the day-of-week wildcard.
        assert!(validate_cron_expression("* * * * */2"));
    }

    #[test]
    fn valid_explicit_value_list() {
        // Comma list of discrete minute values.
        assert!(validate_cron_expression("0,15,30,45 * * * *"));
    }

    #[test]
    fn valid_full_range_every_field() {
        // The inclusive bounds of every field expressed as ranges.
        assert!(validate_cron_expression("0-59 0-23 1-31 1-12 0-7"));
    }

    #[test]
    fn valid_tab_separated_fields() {
        // `split_whitespace` collapses any run of whitespace, so tabs work too.
        assert!(validate_cron_expression("0\t8\t*\t*\t1"));
    }

    #[test]
    fn invalid_whitespace_only() {
        // Only whitespace -> zero fields -> rejected.
        assert!(!validate_cron_expression("     "));
    }

    #[test]
    fn invalid_empty_list_element() {
        // A trailing/empty comma element (`1,,`) is not a valid sub-field.
        assert!(!validate_cron_expression("1,, * * * *"));
    }

    #[test]
    fn invalid_dangling_range_bound() {
        // A range missing its upper bound (`1-`) fails to parse.
        assert!(!validate_cron_expression("1- * * * *"));
    }

    #[test]
    fn invalid_negative_value() {
        // Leading `-` makes the field parse as a malformed range.
        assert!(!validate_cron_expression("-1 * * * *"));
    }

    #[test]
    fn invalid_non_numeric_step() {
        // Step value must be a positive integer.
        assert!(!validate_cron_expression("*/x * * * *"));
    }
}
